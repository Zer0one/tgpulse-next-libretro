use super::*;

pub(super) fn write(chip: &mut Ym3438, register: u16, value: u8) {
    let port = ((register >> 8) * 2) as u8;
    chip.write(port, register as u8);
    chip.write(port + 1, value);
}

#[test]
fn address_banks_busy_and_port_mirrors() {
    let mut chip = Ym3438::new();
    chip.write(0, 0x24);
    chip.write(3, 255);
    assert_eq!(chip.read(0), 0);
    assert_eq!(chip.state.registers[0x24], 0);
    chip.write(1, 255);
    chip.advance(191);
    assert_eq!(chip.read(0), 0x80);
    for port in 1..4 {
        assert_eq!(chip.read(port), 0);
    }
    chip.write(2, 0x24);
    chip.write(1, 42); // must not extend Busy or use old bank-0 address
    chip.advance(1);
    assert_eq!(chip.read(0), 0);
    assert_eq!(chip.state.registers[0x24], 255);
    chip.write(7, 99); // port 3 mirror
    assert_eq!(chip.state.registers[0x124], 99);
    assert_eq!(chip.read(4), 0x80);
}

#[test]
fn frequency_latches_are_shared_across_channels_and_banks() {
    let mut chip = Ym3438::new();
    write(&mut chip, 0xa4, 0xff);
    assert_eq!(chip.state.registers[0xa4], 0);
    write(&mut chip, 0x1a1, 0x12);
    assert_eq!(chip.state.registers[0x1a5], 0x3f);
    write(&mut chip, 0xad, 0x25); // independent special-frequency latch
    write(&mut chip, 0xaa, 0x45);
    assert_eq!(chip.state.registers[0xae], 0x25);
    write(&mut chip, 0xa0, 0x23);
    assert_eq!(chip.state.registers[0xa4], 0x3f);
    for reg in [0xb8, 0xb9, 0x1b8, 0x1b9, 0xa7, 0xaf] {
        write(&mut chip, reg, 0);
    }
    assert_eq!(chip.state.registers[0xb8], 0x3f);
    assert_eq!(chip.state.registers[0xb9], 0x25);
}

#[test]
fn timer_a_reloads_updated_period_without_restarting_on_write() {
    let mut chip = Ym3438::new();
    write(&mut chip, 0x24, 255);
    write(&mut chip, 0x25, 3);
    write(&mut chip, 0x27, 5);
    chip.advance(100);
    write(&mut chip, 0x25, 2); // next period 288, current still 44 remaining
    write(&mut chip, 0x27, 5); // load bit already set: do not restart
    chip.advance(43);
    assert!(!chip.irq());
    chip.advance(1);
    assert!(chip.irq());
    assert_eq!(chip.state.timer_remaining[0], Some(288));
    write(&mut chip, 0x27, 0x11); // clear flag, continue without flag enable
    chip.advance(288);
    assert!(!chip.irq());
    write(&mut chip, 0x27, 5);
    chip.advance(288);
    assert!(chip.irq());
    write(&mut chip, 0x27, 0); // stop does not clear latched status
    assert!(chip.irq());
    assert_eq!(chip.state.timer_remaining, [None; 2]);
    write(&mut chip, 0x27, 0x10);
    assert!(!chip.irq());
}

#[test]
fn timer_b_first_period_uses_free_running_phase_even_after_reset() {
    for phase in 0..16 {
        let mut chip = Ym3438::new();
        chip.advance(144 * (256 + phase) + 17); // counter wraps; fractional tick
        chip.reset();
        write(&mut chip, 0x26, 255);
        write(&mut chip, 0x27, 10);
        let first = (16 - phase) * 144;
        assert_eq!(chip.state.timer_remaining[1], Some(first));
        chip.advance(first - 1);
        assert!(!chip.irq());
        chip.advance(1);
        assert!(chip.irq());
        assert_eq!(chip.state.timer_remaining[1], Some(2304));
    }
}

#[test]
fn key_requests_csm_mode_and_dac_state_are_separate() {
    let mut chip = Ym3438::new();
    for (channel, code) in [0, 1, 2, 4, 5, 6].into_iter().enumerate() {
        write(&mut chip, 0x28, 0x50 | code);
        assert_eq!(chip.state.key_on[channel], 5);
    }
    write(&mut chip, 0x28, 3); // invalid channel
    write(&mut chip, 0x128, 0); // high bank does not address key-on
    assert_eq!(chip.state.key_on, [5; 6]);
    write(&mut chip, 0x2a, 0xff);
    write(&mut chip, 0x2b, 0x80);
    write(&mut chip, 0x2c, 8);
    assert_eq!(chip.state.dac_data, 255);
    assert!(chip.state.dac_enabled);
    assert_eq!(chip.state.registers[0x2a], 0);
    chip.advance(13);
    write(&mut chip, 0x24, 255);
    write(&mut chip, 0x25, 3);
    write(&mut chip, 0x27, 0x81); // CSM, flag disabled: key request still occurs
    chip.advance(144);
    assert!(chip.state.csm_pending);
    assert!(!chip.irq());
    chip.advance(130);
    assert!(chip.state.csm_pending);
    chip.advance(1);
    assert!(!chip.state.csm_pending);
    write(&mut chip, 0x27, 0xc1); // both mode bits != CSM
    chip.advance(13);
    assert!(!chip.state.csm_pending);
    assert_eq!(chip.state.key_on, [5; 6]);
}

#[test]
fn reset_is_not_power_cycle() {
    let mut chip = Ym3438::new();
    chip.advance(901);
    write(&mut chip, 0x2a, 0x42);
    write(&mut chip, 0x2b, 0x80);
    write(&mut chip, 0x27, 15);
    chip.write(2, 0x99);
    let before = chip.snapshot();
    chip.reset();
    assert_eq!(chip.state.address, before.address);
    assert_eq!(chip.state.dac_data, before.dac_data);
    assert_eq!(chip.state.dac_enabled, before.dac_enabled);
    assert_eq!(chip.state.busy_remaining, before.busy_remaining);
    assert_eq!(chip.state.sample_remaining, before.sample_remaining);
    assert_eq!(chip.state.total_samples, before.total_samples);
    assert_eq!(chip.state.timer_remaining, [None; 2]);
    assert_eq!(chip.state.registers[0x27], 0);
    for reg in [0xb4, 0xb5, 0xb6, 0x1b4, 0x1b5, 0x1b6] {
        assert_eq!(chip.state.registers[reg], 0xc0);
    }
    assert_ne!(chip.snapshot(), Ym3438::new().snapshot());
}

#[test]
fn serialized_mid_timer_csm_busy_state_continues_identically_with_different_slices() {
    let mut chip = Ym3438::new();
    chip.advance(29);
    for (reg, data) in [(0x24, 255), (0x25, 3), (0x26, 255), (0x27, 0x8f)] {
        write(&mut chip, reg, data);
    }
    chip.advance(144);
    assert!(chip.state.csm_pending);
    assert!(chip.state.busy_remaining > 0);
    let bytes = bincode::serialize(&chip.snapshot()).unwrap();
    let snapshot: State = bincode::deserialize(&bytes).unwrap();
    let mut restored = Ym3438::new();
    restored.restore(&snapshot).unwrap();
    for count in [1, 17, 114, 2304, 100_001, 590_000] {
        chip.advance(count);
        for _ in 0..count {
            restored.advance(1);
        }
        assert_eq!(chip.snapshot(), restored.snapshot());
        assert_eq!(chip.read(0), restored.read(0));
        for device in [&mut chip, &mut restored] {
            write(device, 0x27, 0xbf); // clear status without restarting
        }
    }
}

#[test]
fn invalid_restore_is_rejected_atomically() {
    let mut chip = Ym3438::new();
    write(&mut chip, 0x27, 15);
    let valid = chip.snapshot();
    let corruptions: &[fn(&mut State)] = &[
        |s| s.address = 512,
        |s| s.dac_data = 512,
        |s| s.status = 4,
        |s| s.busy_remaining = 193,
        |s| s.sample_remaining = 0,
        |s| s.sample_remaining = 145,
        |s| s.key_on[3] = 16,
        |s| s.registers[0xb9] = 64,
        |s| s.timer_remaining[0] = Some(0),
        |s| s.timer_remaining[1] = Some(589825),
        |s| s.timer_remaining[0] = None,
        |s| s.registers[0x27] = 0,
    ];
    for corrupt in corruptions {
        let mut invalid = valid.clone();
        corrupt(&mut invalid);
        assert!(chip.restore(&invalid).is_err());
        assert_eq!(chip.snapshot(), valid);
    }
    // A changed period register is legitimately different from the live period.
    write(&mut chip, 0x24, 255);
    write(&mut chip, 0x25, 3);
    chip.restore(&chip.snapshot()).unwrap();
}

pub(super) struct Hash(pub(super) u64);
impl Hash {
    pub(super) fn new() -> Self {
        Self(14_695_981_039_346_656_037)
    }
    pub(super) fn add(&mut self, mut value: u64, bytes: usize) {
        for _ in 0..bytes {
            self.0 = (self.0 ^ (value & 255)).wrapping_mul(1_099_511_628_211);
            value >>= 8;
        }
    }
}

// Deliberately explicit canonical byte order, shared with the reference tool.
// Includes ALL register bytes plus externally hosted timing state, not just
// the CPU-visible status bits. This is a test digest, not a save-state format.
fn hash(chip: &Ym3438) -> u64 {
    let mut hash = Hash::new();
    let s = &chip.state;
    for value in s.registers {
        hash.add(u64::from(value), 1);
    }
    hash.add(u64::from(s.address), 2);
    hash.add(u64::from(s.dac_data), 2);
    hash.add(u64::from(s.dac_enabled), 1);
    for port in 0..4 {
        hash.add(u64::from(chip.read(port)), 1);
    }
    hash.add(u64::from(chip.irq()), 1);
    hash.add(u64::from(s.busy_remaining), 2);
    for timer in s.timer_remaining {
        hash.add(u64::from(timer.unwrap_or(0)), 4);
    }
    hash.add(u64::from(s.sample_remaining), 2);
    hash.add(u64::from(s.total_samples), 1);
    for key in s.key_on {
        hash.add(u64::from(key), 1);
    }
    hash.add(u64::from(s.csm_pending), 1);
    hash.0
}

fn fuzz(chip: &mut Ym3438, mut seed: u32, count: u32) -> u64 {
    let mut history = Hash::new();
    let registers = [
        0x24, 0x25, 0x26, 0x27, 0x28, 0x2a, 0x2b, 0x2c, 0xa0, 0xa4, 0xa8, 0xac, 0xb8, 0x124, 0x128,
        0x1a1, 0x1a5, 0x1b6,
    ];
    for _ in 0..count {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        match seed & 7 {
            0..=2 => chip.advance(1 + ((seed >> 8) & 4095)),
            3..=4 => chip.write(((seed >> 8) & 3) as u8, (seed >> 16) as u8),
            _ => write(
                chip,
                registers[((seed >> 8) % 18) as usize],
                (seed >> 24) as u8,
            ),
        }
        history.add(hash(chip), 8);
    }
    history.0
}

#[test]
fn pinned_ymfm_differential_trace() {
    let mut chip = Ym3438::new();
    for (line_number, line) in include_str!("reference.trace").lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (command, expected) = line.split_once('#').expect("missing oracle hash");
        let fields: Vec<_> = command.split_whitespace().collect();
        let arg = |index: usize| fields[index].parse::<u32>().unwrap();
        let actual = match fields[0] {
            "P" => {
                chip = Ym3438::new();
                hash(&chip)
            }
            "R" => {
                chip.reset();
                hash(&chip)
            }
            "A" => {
                chip.advance(arg(1));
                hash(&chip)
            }
            "W" => {
                chip.write(arg(1) as u8, arg(2) as u8);
                hash(&chip)
            }
            "F" => fuzz(&mut chip, arg(1), arg(2)),
            _ => panic!("unknown trace command"),
        };
        let expected = u64::from_str_radix(expected.trim(), 16).unwrap();
        assert_eq!(
            actual,
            expected,
            "YMFM trace line {}: {command}",
            line_number + 1
        );
        // Every observed valid state must also survive the public restore gate.
        chip.restore(&chip.snapshot()).unwrap();
    }
}
