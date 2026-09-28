//! Sega Model 1 sound board (`segam1audio`), as fitted to Daytona.
//!
//! A 68000 at 10MHz with its own program ROM, two MultiPCM samplers, a YM3438
//! for FM, and an i8251 UART that is the only wire back to the i960. The main
//! board sends it one byte at a time and it does the rest on its own.
//!
//! This is emulated rather than reimplemented because the program is the game's
//! own: `epr-16720`/`epr-16721` are Daytona's sound driver, so an HLE would
//! mean rewriting that driver by hand for every Model 2 title. Same reasoning as
//! the TGP.
//!
//! Layout:
//!
//! ```text
//!   000000-03ffff program ROM
//!   080000-09ffff mirror of the upper ROM socket (sndcpu + 0x20000)
//!   c20000-c20003 i8251 UART        (odd bytes only)
//!   c40000-c40007 MultiPCM 1        (odd bytes only)
//!   c50000-c50001 MultiPCM 1 bank
//!   c60000-c60007 MultiPCM 2        (odd bytes only)
//!   c70000-c70001 MultiPCM 2 bank
//!   d00000-d00007 YM3438            (odd bytes only)
//!   f00000-f0ffff work RAM
//! ```

use crate::config::{AudioGains, AudioMutes};
use crate::multipcm::MultiPcm;
use m68000::cpu_details::Mc68000;
use m68000::exception::{Exception, Vector};
use m68000::memory_access::MemoryAccess;
use m68000::M68000;
mod dsb;
mod fm;
mod serial;
mod state;
pub use crate::i8251::Error as SerialError;
pub use dsb::DsbPathState;
pub use serial::SerialState;
pub use state::SoundState;
#[cfg(test)]
mod dsb_tests;
use fm::FmPath;
pub use fm::FmPathState;

/// 20MHz crystal divided by two.
pub const SND_CPU_HZ: u32 = 10_000_000;

/// Implemented audio outputs, not hardware enable/disable switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioSource {
    MultiPcm1,
    MultiPcm2,
    Ym3438,
    Scsp,
    Dsb,
}

pub const MULTIPCM_SOURCES: &[AudioSource] = &[
    AudioSource::MultiPcm1,
    AudioSource::MultiPcm2,
    AudioSource::Ym3438,
];
pub const SCSP_SOURCES: &[AudioSource] = &[AudioSource::Scsp];
const DSB_SOURCES: &[AudioSource] = &[
    AudioSource::MultiPcm1,
    AudioSource::MultiPcm2,
    AudioSource::Ym3438,
    AudioSource::Dsb,
];

#[cfg(test)]
mod mute_tests {
    use super::*;

    #[test]
    fn absolute_gains_replace_reference_and_defaults_preserve_old_rounding() {
        for a in [-100000, -32768, -101, -1, 0, 1, 101, 32767, 100000] {
            for b in [-32001, -1, 0, 1, 32001] {
                for fm in [-32896, -3, 0, 3, 32768] {
                    for dsb in [-900, 0, 900] {
                        let old = (((a.clamp(-32768, 32767) + b) * 5 + fm * 3) / 10 + dsb)
                            .clamp(-32768, 32767) as i16;
                        assert_eq!(
                            mix_with_gains(
                                (a, a),
                                (b, b),
                                [fm; 2],
                                [false; 3],
                                [dsb; 2],
                                AudioGains::default()
                            ),
                            (old, old)
                        );
                    }
                }
            }
        }
        let gains = AudioGains {
            multipcm1: 80,
            multipcm2: 20,
            ym3438: 40,
            dsb: 25,
            scsp: 100,
        };
        for (pcm1, pcm2, fm, dsb, expected) in [
            ((1000, -1000), (0, 0), [0; 2], [0; 2], 800),
            ((0, 0), (1000, -1000), [0; 2], [0; 2], 200),
            ((0, 0), (0, 0), [1000, -1000], [0; 2], 400),
            ((0, 0), (0, 0), [0; 2], [1000, -1000], 250),
        ] {
            assert_eq!(
                mix_with_gains(pcm1, pcm2, fm, [false; 3], dsb, gains),
                (expected, -expected)
            );
        }
        let max = AudioGains {
            multipcm1: u32::MAX,
            multipcm2: u32::MAX,
            ym3438: u32::MAX,
            dsb: u32::MAX,
            scsp: u32::MAX,
        }
        .clamped();
        assert_eq!(max.multipcm1, 100);
        assert_eq!(
            mix_with_gains(
                (32767, -32768),
                (32767, -32768),
                [32767, -32768],
                [false; 3],
                [32767, -32768],
                max
            ),
            (32767, -32768)
        );
    }

    #[test]
    fn zero_gain_preserves_hardware_and_mute_retains_custom_gain() {
        for chip in 0..2 {
            let mut reference = sounding_board(chip);
            let mut adjusted = sounding_board(chip);
            let mut gains = AudioGains::default();
            gains.multipcm1 = 0;
            gains.multipcm2 = 0;
            adjusted.set_gains(gains);
            reference.run(70_013, SND_CPU_HZ);
            adjusted.run(70_013, SND_CPU_HZ);
            assert!(adjusted.samples.iter().all(|s| *s == (0, 0)));
            assert_eq!(reference.cpu.regs.pc, adjusted.cpu.regs.pc);
            assert_eq!(reference.snapshot_fm_path(), adjusted.snapshot_fm_path());
            gains.multipcm1 = 75;
            gains.multipcm2 = 25;
            reference.set_gains(gains);
            adjusted.set_gains(gains);
            adjusted.set_mutes(AudioMutes {
                multipcm1: true,
                multipcm2: true,
                ..AudioMutes::default()
            });
            reference.run(10_007, SND_CPU_HZ);
            adjusted.run(10_007, SND_CPU_HZ);
            adjusted.set_mutes(AudioMutes::default());
            assert_eq!(adjusted.gains, gains);
            assert!(adjusted.samples.is_empty());
            reference.samples.clear();
            reference.run(10_001, SND_CPU_HZ);
            adjusted.run(10_001, SND_CPU_HZ);
            assert_eq!(reference.samples, adjusted.samples);
        }
    }

    pub(super) fn sounding_board(chip: usize) -> SoundSystem {
        let mut program = vec![0; 16];
        program[..4].copy_from_slice(&0x00f0fff0u32.to_be_bytes());
        program[4..8].copy_from_slice(&8u32.to_be_bytes());
        program[8..12].copy_from_slice(&[0x4e, 0x71, 0x60, 0xfc]); // NOP; BRA
        let mut samples = vec![0; 0x140];
        // Instrument 0: signed 8-bit waveform at 0x100, loop 0..64,
        // instant attack, sustained envelope, no LFO.
        samples[..12].copy_from_slice(&[0, 1, 0, 0, 0, 0xff, 0xc0, 0, 0xf0, 0, 0xff, 0]);
        for i in 0..64 {
            samples[0x100 + i] = (i as i8 * 2 - 64) as u8;
        }
        let mut sound = SoundSystem::new(program, samples.clone(), samples);
        for (reg, value) in [(0, 0), (1, 0), (2, 0), (3, 0x10), (5, 1), (4, 0x80)] {
            sound.board.pcm[chip].write(2, reg);
            sound.board.pcm[chip].write(0, value);
        }
        for (address, value) in [(0x24, 255), (0x25, 3), (0x27, 5)] {
            sound.board.ym.write(0, address);
            sound.board.ym.write(1, value);
        }
        sound
    }

    #[test]
    fn each_multipcm_mute_changes_only_output_and_preserves_continuation() {
        for chip in 0..2 {
            let mut reference = sounding_board(chip);
            let mut muted = sounding_board(chip);
            muted.set_mutes(AudioMutes {
                multipcm1: chip == 0,
                multipcm2: chip == 1,
                ..AudioMutes::default()
            });
            reference.run(70_013, SND_CPU_HZ);
            muted.run(70_013, SND_CPU_HZ);
            assert!(!reference.samples.is_empty());
            assert!(reference.samples.iter().any(|&s| s != (0, 0)));
            assert_eq!(reference.samples.len(), muted.samples.len());
            assert!(muted.samples.iter().all(|&s| s == (0, 0)));
            assert_eq!(reference.cpu.regs.pc, muted.cpu.regs.pc);
            assert_eq!(
                reference.board.pcm[chip].active_samples(),
                muted.board.pcm[chip].active_samples()
            );
            assert_eq!(reference.board.ym.read(0), muted.board.ym.read(0));
            assert_ne!(muted.board.ym.read(0) & 1, 0, "YM timer still runs");
            reference.samples.clear();
            muted.set_mutes(AudioMutes::default());
            assert!(muted.samples.is_empty());
            reference.run(50_003, SND_CPU_HZ);
            muted.run(50_003, SND_CPU_HZ);
            assert_eq!(
                reference.samples, muted.samples,
                "unmute must not restart/freeze the chip"
            );
        }
    }

    #[test]
    fn muting_the_other_multipcm_does_not_change_gain() {
        for chip in 0..2 {
            let mut reference = sounding_board(chip);
            let mut muted = sounding_board(chip);
            muted.set_mutes(AudioMutes {
                multipcm1: chip != 0,
                multipcm2: chip != 1,
                ..AudioMutes::default()
            });
            reference.run(70_013, SND_CPU_HZ);
            muted.run(70_013, SND_CPU_HZ);
            assert_eq!(reference.samples, muted.samples);
        }
    }

    fn fm_dac(sound: &mut SoundSystem) {
        // Use the actual odd-byte bus map, not a second control path.
        for (address, data) in [(0x2b, 0x80), (0x2a, 255)] {
            sound.board.write8(0xd00001, address);
            sound.board.write8(0xd00003, data);
        }
    }

    #[test]
    fn fm_mute_is_output_only_and_unmute_continues_identically() {
        let mut reference = sounding_board(0);
        let mut muted = sounding_board(0);
        for board in [&mut reference, &mut muted] {
            fm_dac(board);
            board.set_mutes(AudioMutes {
                multipcm1: true,
                multipcm2: true,
                ..AudioMutes::default()
            });
        }
        muted.set_mutes(AudioMutes {
            multipcm1: true,
            multipcm2: true,
            ym3438: true,
            ..AudioMutes::default()
        });
        reference.run(70_013, SND_CPU_HZ);
        muted.run(70_013, SND_CPU_HZ);
        assert!(reference.samples.iter().any(|s| *s != (0, 0)));
        assert!(muted.samples.iter().all(|s| *s == (0, 0)));
        assert_eq!(reference.snapshot_fm_path(), muted.snapshot_fm_path());
        reference.samples.clear();
        muted.set_mutes(AudioMutes {
            multipcm1: true,
            multipcm2: true,
            ..AudioMutes::default()
        });
        reference.run(50_003, SND_CPU_HZ);
        muted.run(50_003, SND_CPU_HZ);
        assert_eq!(reference.samples, muted.samples);
    }

    #[test]
    fn stopped_sound_cpu_keeps_exact_model1_audio_clock_and_output_count() {
        let total = 100_003u64;
        let expected_clocks = total * u64::from(SND_CPU_HZ) / 16_000_000;
        let mut reference = sounding_board(0);
        reference.cpu.stop = true;
        fm_dac(&mut reference);
        reference.run(total as i32, 16_000_000);
        assert_eq!(reference.samples.len() as u64, expected_clocks / 224);
        assert_eq!(reference.remainder, 0);
        assert_eq!(
            reference.main_fraction,
            total * u64::from(SND_CPU_HZ) % 16_000_000
        );
        assert!(reference.samples.iter().any(|s| *s != (0, 0)));
        for chunk in [1, 3, 64, 4097] {
            let mut sliced = sounding_board(0);
            sliced.cpu.stop = true;
            fm_dac(&mut sliced);
            let mut left = total as i32;
            while left > 0 {
                let step = left.min(chunk);
                sliced.run(step, 16_000_000);
                left -= step;
            }
            assert_eq!(sliced.samples, reference.samples, "chunk {chunk}");
            assert_eq!(sliced.snapshot_fm_path(), reference.snapshot_fm_path());
            assert_eq!(sliced.main_fraction, reference.main_fraction);
            assert_eq!(sliced.remainder, 0);
        }
    }

    #[test]
    fn timed_serial_runs_during_stop_without_queuing_masked_irq() {
        for partition in [1, 3, 64, 10003] {
            let mut s = sounding_board(0);
            s.enable_model1_serial();
            s.board.uart_control(0x4e);
            s.board.uart_control(0x37);
            s.board.write8(0xc20003, 0x4e);
            s.board.write8(0xc20003, 0x37);
            s.cpu.stop = true;
            s.cpu.regs.sr.interrupt_mask = 7;
            s.send(0x96);
            let mut left = 10003;
            while left > 0 {
                let n = left.min(partition);
                s.run(n, 16_000_000);
                left -= n;
            }
            assert!(s.cpu.stop, "masked UART must not wake STOP");
            assert_eq!(s.board.uart_status() & 2, 2);
            assert_eq!(s.board.read8(0xc20001), 0x96);
            s.cpu.regs.sr.interrupt_mask = 0;
            s.run(64, 16_000_000);
            assert!(s.cpu.stop, "polling must not leave a phantom IRQ queued");
            assert_eq!(s.serial_fault(), None);
        }
    }

    #[test]
    fn timed_receive_wakes_stopped_cpu_through_live_irq2() {
        let mut s = sounding_board(0);
        s.enable_model1_serial();
        s.board.rom.resize(0x200, 0);
        s.board.rom[0x68..0x6c].copy_from_slice(&0x100u32.to_be_bytes());
        // IRQ2 handler: MOVE.B UART data,D0; STOP #$2700.
        s.board.rom[0x100..0x10a]
            .copy_from_slice(&[0x10, 0x39, 0x00, 0xc2, 0x00, 0x01, 0x4e, 0x72, 0x27, 0x00]);
        s.board.uart_control(0x4e);
        s.board.uart_control(0x37);
        s.board.write8(0xc20003, 0x4e);
        s.board.write8(0xc20003, 0x37);
        s.cpu.stop = true;
        s.cpu.regs.sr.interrupt_mask = 0;
        s.send(0x96);
        s.run(3000, SND_CPU_HZ);
        assert_eq!(s.board.rx_read_count, 0);
        s.run(2000, SND_CPU_HZ);
        assert_eq!(s.board.rx_read_count, 1);
        assert_eq!(s.cpu.regs.d[0].0 & 0xff, 0x96);
        assert!(s.cpu.stop);
        assert!(!s.board.uart_rx_full());
    }

    #[test]
    fn model1_clock_reaches_dsb_while_68000_is_stopped() {
        let machine = || {
            let firmware = vec![0; crate::dsbz80::FIRMWARE_SIZE]; // Z80 NOPs
            let mut sound = SoundSystem::with_dsb(vec![0; 16], vec![], vec![], &firmware).unwrap();
            sound.cpu.stop = true;
            sound
        };
        let mut whole = machine();
        let mut sliced = machine();
        let total = 10_007;
        whole.run(total, 16_000_000);
        for _ in 0..total {
            sliced.run(1, 16_000_000);
        }
        let expected_sound_clocks = total as u64 * 5 / 8;
        let state = whole.snapshot_dsb_path().unwrap();
        assert_eq!(state.board.sound_ticks(), Some(expected_sound_clocks * 2));
        assert_eq!(state.conversion.time(), expected_sound_clocks * 2);
        assert_eq!(whole.samples.len() as u64, expected_sound_clocks / 224);
        assert_eq!(whole.samples, sliced.samples);
        assert_eq!(
            bincode::serialize(&state).unwrap(),
            bincode::serialize(&sliced.snapshot_dsb_path().unwrap()).unwrap()
        );
        assert_eq!(whole.snapshot_fm_path(), sliced.snapshot_fm_path());
        assert!(whole.dsb_fault().is_none());
    }

    #[test]
    fn mixed_stream_and_cpu_are_independent_of_main_slice_size() {
        for hz in [16_000_000, 25_000_000] {
            let mut whole = sounding_board(0);
            let mut sliced = sounding_board(0);
            fm_dac(&mut whole);
            fm_dac(&mut sliced);
            whole.run(100_003, hz);
            for _ in 0..100_003 {
                sliced.run(1, hz);
            }
            assert_eq!(whole.cpu.regs.pc, sliced.cpu.regs.pc);
            assert_eq!(whole.remainder, sliced.remainder);
            assert_eq!(whole.main_fraction, sliced.main_fraction);
            assert_eq!(whole.samples, sliced.samples);
            assert_eq!(whole.snapshot_fm_path(), sliced.snapshot_fm_path());
            assert!(!whole.samples.is_empty());
        }
    }

    #[test]
    fn bus_ports_follow_ymfm_and_do_not_wire_fm_irq_to_uart() {
        let mut sound = sounding_board(0);
        sound.board.write8(0xd00000, 0x2a); // unused high byte
        assert_eq!(sound.board.ym_writes, 0);
        fm_dac(&mut sound);
        assert_eq!(sound.board.ym_writes, 4);
        assert_eq!(sound.board.read8(0xd00000), 0);
        assert_ne!(sound.board.read8(0xd00001) & 0x80, 0);
        assert_eq!(sound.board.read8(0xd00005), 0);
        sound.render_cycles(500);
        assert_eq!(sound.board.read8(0xd00001) & 0x80, 0);
        assert_ne!(sound.board.read8(0xd00001) & 1, 0);
        assert!(!sound.irq_pending);
    }

    #[test]
    fn mixer_keeps_reference_gains_and_clips_final_sum() {
        assert_eq!(
            mix((1000, -1000), (2000, -2000), [1000, -1000], [false; 3]),
            (1800, -1800)
        );
        assert_eq!(
            mix(
                (1000, -1000),
                (2000, -2000),
                [1000, -1000],
                [false, false, true]
            ),
            (1500, -1500)
        );
        assert_eq!(
            mix(
                (1000, -1000),
                (2000, -2000),
                [1000, -1000],
                [true, false, false]
            ),
            (1300, -1300)
        );
        assert_eq!(
            mix(
                (100000, -100000),
                (100000, -100000),
                [32768, -32896],
                [false; 3]
            ),
            (32767, -32768)
        );
    }

    #[test]
    fn actual_68000_writes_produce_fm_through_the_board_mixer() {
        let mut rom = vec![0; 8];
        rom[..4].copy_from_slice(&0x00f0fff0u32.to_be_bytes());
        rom[4..8].copy_from_slice(&8u32.to_be_bytes());
        let mut registers = vec![(0xb0, 7), (0xb4, 0xc0)];
        for slot in [0, 4, 8, 12] {
            registers.extend([
                (0x30 + slot, 1),
                (0x40 + slot, 32),
                (0x50 + slot, 31),
                (0x80 + slot, 15),
            ]);
        }
        registers.extend([(0xa4, 0x22), (0xa0, 0x69), (0x28, 0xf0)]);
        let writes = registers.len() * 2;
        for (reg, data) in registers {
            for (address, value) in [(0x00d00001u32, reg), (0x00d00003, data)] {
                rom.extend(0x13fcu16.to_be_bytes()); // MOVE.B #imm,abs.l
                rom.extend((value as u16).to_be_bytes());
                rom.extend(address.to_be_bytes());
            }
        }
        rom.extend(0x60feu16.to_be_bytes()); // BRA.S self
        let mut sound = SoundSystem::new(rom, vec![], vec![]);
        sound.run(100_000, SND_CPU_HZ);
        assert_eq!(sound.board.ym_writes, writes as u64);
        assert_eq!(sound.exception_counts.iter().sum::<u64>(), 0);
        assert!(sound.samples.iter().any(|&(l, r)| l > 0 && r > 0));
        assert!(sound.samples.iter().any(|&(l, r)| l < 0 && r < 0));
    }
}

/// The board's RAM: the reference maps 0xf00000-0xf0ffff, noting the real PCB carries
/// two 8Kx8 SRAMs.
const SND_RAM_SIZE: usize = 0x10000;

/// i8251 status bits the driver polls. TxRDY and TxEMPTY are always true here:
/// we consume a byte the instant it is written, so the transmitter is never
/// busy. RxRDY is raised only when the main board has actually sent something.
const UART_TX_RDY: u8 = 0x01;
const UART_RX_RDY: u8 = 0x02;
const UART_TX_EMPTY: u8 = 0x04;

/// The board's memory map and devices, everything except the 68000 itself.
pub struct SoundBoard {
    serial: Option<serial::SerialState>,
    /// Optional DSB link, selected by Model 1 ROM resources. The Model 1 owner
    /// propagates its sticky errors through the normal machine error path.
    pub dsb: Option<crate::dsbz80::Board>,
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    /// The two samplers. Between them they carry Daytona's entire mix bar the
    /// YM3438's FM (see `ym_writes`).
    pub pcm: [MultiPcm; 2],

    // --- i8251, main board -> sound board ---
    /// Bytes received from the i960 and not yet read by the driver. A queue,
    /// not a single slot: back-to-back command bytes (VF's sound handshake
    /// sends three in a row) must not overwrite each other.
    pub rx: std::collections::VecDeque<u8>,
    /// How many bytes the main board has sent us.
    pub rx_count: u64,
    /// How many of those the driver actually collected from the data register.
    pub rx_read_count: u64,
    /// Byte the driver sent back, waiting for the i960 to collect it.
    pub tx: Option<u8>,

    /// YM3438 bus traffic for diagnostics (including address writes).
    pub ym_writes: u64,
    ym: FmPath,
}

impl SoundBoard {
    pub fn new(rom: Vec<u8>, pcm1: Vec<u8>, pcm2: Vec<u8>) -> Self {
        Self {
            serial: None,
            dsb: None,
            rom,
            ram: vec![0; SND_RAM_SIZE],
            pcm: [
                MultiPcm::new(pcm1, SND_CPU_HZ as f32),
                MultiPcm::new(pcm2, SND_CPU_HZ as f32),
            ],
            rx: std::collections::VecDeque::new(),
            rx_count: 0,
            rx_read_count: 0,
            tx: None,
            ym_writes: 0,
            ym: FmPath::new(),
        }
    }

    /// The i960 has put a byte on the wire.
    pub fn uart_send(&mut self, data: u8) {
        if let Some(s) = &mut self.serial {
            let result = s.main.write(data);
            s.latch(result);
            self.rx_count += 1;
            return;
        }
        self.rx.push_back(data);
        if self.rx.len() > 8 {
            self.rx.pop_front();
        }
        self.rx_count += 1;
        log::trace!(target: "sound", "main -> driver: {:02X}", data);
    }

    /// Main-side i8251 mode/command register. Model 1 uses real framing;
    /// the retained Model 2 HLE path only observes the reset bit.
    pub fn uart_control(&mut self, val: u8) {
        log::trace!(target: "sound", "main uart ctl: {:02X}", val);
        if let Some(s) = &mut self.serial {
            let result = s.main.control(val);
            s.latch(result);
            return;
        }
        // Command register bit 6 = internal reset.
        if val & 0x40 != 0 {
            self.rx.clear();
            self.tx = None;
        }
    }

    /// True while the driver has a byte waiting to be read back.
    pub fn uart_rx_ready(&self) -> bool {
        if let Some(s) = &self.serial {
            return s.main.irq();
        }
        self.tx.is_some()
    }

    /// True while the main board has a byte waiting for the driver.
    pub fn uart_rx_full(&self) -> bool {
        if let Some(s) = &self.serial {
            return s.driver.irq();
        }
        !self.rx.is_empty()
    }

    /// True while the UART can accept another byte for the board.
    ///
    /// Always in the Model 2 HLE path: `uart_send` hands the byte over on the spot, so the
    /// transmitter is never busy. This matters more than it looks -- the main
    /// board's sound interrupt is asserted on TxRDY *or* RxRDY, so this line is
    /// what keeps the game's sound task running at all.
    pub fn uart_tx_ready(&self) -> bool {
        if let Some(s) = &self.serial {
            return s.main.command & 1 != 0 && s.main.status() & 1 != 0;
        }
        true
    }

    pub fn main_uart_read(&mut self) -> u8 {
        self.serial
            .as_mut()
            .map_or_else(|| self.tx.take().unwrap_or(0), |s| s.main.read())
    }
    pub fn main_uart_status(&self) -> u8 {
        self.serial
            .as_ref()
            .map_or(5 | if self.tx.is_some() { 2 } else { 0 }, |s| {
                s.main.status()
            })
    }

    fn uart_status(&self) -> u8 {
        if let Some(s) = &self.serial {
            return s.driver.status();
        }
        let mut s = self
            .dsb
            .as_ref()
            .map_or(UART_TX_RDY | UART_TX_EMPTY, |d| d.sender_status());
        if !self.rx.is_empty() {
            s |= UART_RX_RDY;
        }
        s
    }

    /// Reads one byte of the board's address space. The 68000's byte lanes are
    /// handled by the caller; this takes a flat address.
    fn read8(&mut self, addr: u32) -> u8 {
        let a = addr & 0xffffff;
        match a {
            0x000000..=0x03ffff => self.rom.get(a as usize).copied().unwrap_or(0xff),
            // Mirror of the upper ROM socket.
            0x080000..=0x09ffff => self
                .rom
                .get((a - 0x080000 + 0x20000) as usize)
                .copied()
                .unwrap_or(0xff),
            0xf00000..=0xf0ffff => self.ram[(a - 0xf00000) as usize],

            // i8251: even register = data, odd = status (odd bytes of the word).
            0xc20001 => {
                if let Some(s) = &mut self.serial {
                    if s.driver.irq() {
                        self.rx_read_count += 1;
                    }
                    return s.driver.read();
                }
                if !self.rx.is_empty() {
                    self.rx_read_count += 1;
                }
                let v = self.rx.pop_front().unwrap_or(0);
                log::trace!(target: "sound", "driver reads cmd: {:02X}", v);
                v
            }
            0xc20003 => self.uart_status(),

            0xc40001..=0xc40007 if a & 1 == 1 => self.pcm[0].read(),
            0xc60001..=0xc60007 if a & 1 == 1 => self.pcm[1].read(),
            // YM3438: address/status A, data A, address/status B, data B,
            // all on the low byte lane of successive 16-bit words.
            0xd00001..=0xd00007 if a & 1 == 1 => self.ym.read(((a - 0xd00001) >> 1) as u8),
            _ => 0,
        }
    }

    fn write8(&mut self, addr: u32, val: u8) {
        let a = addr & 0xffffff;
        match a {
            0xf00000..=0xf0ffff => self.ram[(a - 0xf00000) as usize] = val,

            0xc20001 => {
                log::trace!(target: "sound", "driver -> main: {:02X}", val);
                if let Some(s) = &mut self.serial {
                    let result = s.driver.write(val);
                    if result.is_ok() {
                        if let Some(dsb) = &mut self.dsb {
                            dsb.note_external_transmit();
                        }
                    }
                    s.latch(result);
                    return;
                }
                self.tx = Some(val);
                if let Some(dsb) = &mut self.dsb {
                    dsb.sender_write(val);
                }
            }
            // Model 1 configures its physical endpoint; the legacy fixture
            // path configures only the optional DSB-owned sender.
            0xc20003 => {
                log::trace!(target: "sound", "driver uart ctl: {:02X}", val);
                if let Some(s) = &mut self.serial {
                    let result = s.driver.control(val);
                    s.latch(result);
                    return;
                }
                if let Some(dsb) = &mut self.dsb {
                    dsb.sender_control(val);
                }
            }

            0xc50000..=0xc50001 => self.pcm[0].set_bank(val as u32),
            0xc70000..=0xc70001 => self.pcm[1].set_bank(val as u32),
            0xc40012..=0xc40013 => {} // the reference: nopw

            // The chips sit on the odd byte lanes (the reference: umask16(0x00ff)), so
            // 68000 address 0xc40001 is chip offset 0, 0xc40003 offset 1,...
            0xc40001..=0xc40007 if a & 1 == 1 => self.pcm[0].write((a - 0xc40001) >> 1, val),
            0xc60001..=0xc60007 if a & 1 == 1 => self.pcm[1].write((a - 0xc60001) >> 1, val),
            0xd00001..=0xd00007 if a & 1 == 1 => {
                self.ym_writes += 1;
                self.ym.write(((a - 0xd00001) >> 1) as u8, val);
            }
            0x000000..=0x09ffff => {} // ROM
            _ => {}
        }
    }
}

impl MemoryAccess for SoundBoard {
    fn get_byte(&mut self, addr: u32) -> Option<u8> {
        Some(self.read8(addr))
    }
    fn get_word(&mut self, addr: u32) -> Option<u16> {
        // The 68000 is big endian.
        Some(((self.read8(addr) as u16) << 8) | self.read8(addr.wrapping_add(1)) as u16)
    }
    fn set_byte(&mut self, addr: u32, value: u8) -> Option<()> {
        self.write8(addr, value);
        Some(())
    }
    fn set_word(&mut self, addr: u32, value: u16) -> Option<()> {
        self.write8(addr, (value >> 8) as u8);
        self.write8(addr.wrapping_add(1), value as u8);
        Some(())
    }
    fn reset_instruction(&mut self) {}
}

/// The board with its CPU attached.
pub struct SoundSystem {
    pub cpu: M68000<Mc68000>,
    pub board: SoundBoard,
    /// Cycle budget carried between slices, since the 68000 runs at its own
    /// clock rather than the i960's.
    remainder: i64,
    /// The UART's rxrdy line is wired to the 68000's IRQ 2.
    irq_pending: bool,
    /// Main-CPU to sound-CPU fractional conversion; callers keep main Hz fixed.
    main_fraction: u64,
    muted: [bool; 3],
    dsb_muted: bool,
    gains: AudioGains,
    dsb_conversion: dsb::Conversion,
    /// Rendered stereo output at the chip rate, drained by the front end.
    /// Headless tools never drain it, so it is capped rather than unbounded.
    pub samples: std::collections::VecDeque<(i16, i16)>,
    /// Exceptions raised by executed opcodes, indexed by vector. IRQ2 is
    /// injected separately and therefore does not appear here.
    pub exception_counts: [u64; 256],
}

/// Upper bound on buffered audio (~2s) so headless runs don't accumulate it.
const MAX_BUFFERED_SAMPLES: usize = 90_000;

#[cfg(test)]
fn mix(pcm1: (i32, i32), pcm2: (i32, i32), fm: [i32; 2], muted: [bool; 3]) -> (i16, i16) {
    mix_dsb(pcm1, pcm2, fm, muted, [0; 2])
}

#[cfg(test)]
fn mix_dsb(
    pcm1: (i32, i32),
    pcm2: (i32, i32),
    fm: [i32; 2],
    muted: [bool; 3],
    dsb: [i32; 2],
) -> (i16, i16) {
    mix_with_gains(pcm1, pcm2, fm, muted, dsb, AudioGains::default())
}

fn mix_with_gains(
    pcm1: (i32, i32),
    pcm2: (i32, i32),
    fm: [i32; 2],
    muted: [bool; 3],
    dsb: [i32; 2],
    gains: AudioGains,
) -> (i16, i16) {
    let side = |a: i32, b: i32, fm: i32, dsb: i32| {
        let a = if muted[0] { 0 } else { a.clamp(-32768, 32767) };
        let b = if muted[1] { 0 } else { b.clamp(-32768, 32767) };
        let fm = if muted[2] { 0 } else { fm };
        // Absolute route gains replace the reference 0.5/0.5/0.3/1.0.
        // Keep the old rounding boundary before DSB for bit-identical defaults.
        let main = (i64::from(a) * i64::from(gains.multipcm1)
            + i64::from(b) * i64::from(gains.multipcm2)
            + i64::from(fm) * i64::from(gains.ym3438))
            / 100;
        (main + i64::from(dsb) * i64::from(gains.dsb) / 100).clamp(-32768, 32767) as i16
    };
    (
        side(pcm1.0, pcm2.0, fm[0], dsb[0]),
        side(pcm1.1, pcm2.1, fm[1], dsb[1]),
    )
}

impl SoundSystem {
    /// Select at construction before running slices; Model 2 retains its HLE interface.
    pub(crate) fn enable_model1_serial(&mut self) {
        self.board.serial = Some(serial::SerialState::default());
        if let Some(dsb) = &mut self.board.dsb {
            dsb.disconnect_sender();
        }
    }
    /// The wire/endpoints only; a full restore also needs CPU budgets and DSB state.
    pub fn snapshot_serial(&self) -> Option<serial::SerialState> {
        self.board.serial.clone()
    }
    pub fn restore_serial(&mut self, state: &serial::SerialState) -> Result<(), &'static str> {
        if self.board.serial.is_none() || !state.valid() {
            return Err("invalid serial state");
        }
        self.board.serial = Some(state.clone());
        Ok(())
    }
    pub fn serial_fault(&self) -> Option<SerialError> {
        self.board.serial.as_ref().and_then(|s| s.fault)
    }
    pub fn new(rom: Vec<u8>, pcm1: Vec<u8>, pcm2: Vec<u8>) -> Self {
        Self::from_board(SoundBoard::new(rom, pcm1, pcm2))
    }
    /// Opt-in resource-owned pair, connected before the first instruction
    /// executes. Existing game constructors do not select this path yet.
    pub fn with_dsb(
        rom: Vec<u8>,
        pcm1: Vec<u8>,
        pcm2: Vec<u8>,
        firmware: &[u8],
    ) -> Result<Self, crate::dsbz80::Error> {
        let mut board = SoundBoard::new(rom, pcm1, pcm2);
        let mut dsb = crate::dsbz80::Board::new(firmware)?;
        dsb.connect_sender();
        board.dsb = Some(dsb);
        Ok(Self::from_board(board))
    }
    /// Full resource-owned pair, connected before the first 68000 instruction.
    pub fn with_dsb_audio(
        rom: Vec<u8>,
        pcm1: Vec<u8>,
        pcm2: Vec<u8>,
        firmware: &[u8],
        mpeg: Vec<u8>,
    ) -> Result<Self, crate::dsbz80::Error> {
        let mut board = SoundBoard::new(rom, pcm1, pcm2);
        let mut dsb = crate::dsbz80::Board::with_mpeg(firmware, mpeg)?;
        dsb.connect_sender();
        board.dsb = Some(dsb);
        Ok(Self::from_board(board))
    }
    fn from_board(mut board: SoundBoard) -> Self {
        // M68000::new() resets, which reads the vectors through the bus.
        let cpu = {
            let mut c: M68000<Mc68000> = M68000::new();
            // Force the reset to happen against our map straight away, so the
            // vectors are reported now rather than on the first instruction.
            c.interpreter(&mut board);
            c
        };
        log::debug!(
            target: "sound",
            "68000 reset: ssp={:08X} pc={:08X}",
            cpu.regs.ssp.0,
            cpu.regs.pc.0
        );
        Self {
            cpu,
            board,
            remainder: 0,
            irq_pending: false,
            main_fraction: 0,
            muted: [false; 3],
            dsb_muted: false,
            gains: AudioGains::default(),
            dsb_conversion: dsb::Conversion::default(),
            samples: std::collections::VecDeque::new(),
            exception_counts: [0; 256],
        }
    }

    /// Output sample rate: the MultiPCMs' native clock (10MHz / 224).
    pub fn sample_rate(&self) -> f32 {
        self.board.pcm[0].sample_rate()
    }

    pub fn set_mutes(&mut self, mutes: AudioMutes) {
        let muted = [mutes.multipcm1, mutes.multipcm2, mutes.ym3438];
        if self.muted != muted || self.dsb_muted != mutes.dsb {
            self.muted = muted;
            self.dsb_muted = mutes.dsb;
            self.samples.clear();
        }
    }

    pub fn set_gains(&mut self, gains: AudioGains) {
        let gains = gains.clamped();
        if self.gains != gains {
            self.gains = gains;
            self.samples.clear();
        }
    }

    pub fn sources(&self) -> &'static [AudioSource] {
        if self.board.dsb.is_some() {
            DSB_SOURCES
        } else {
            MULTIPCM_SOURCES
        }
    }

    pub fn dsb_fault(&self) -> Option<crate::dsbz80::Error> {
        self.board.dsb.as_ref().and_then(|d| d.fault())
    }

    pub fn snapshot_dsb_path(&self) -> Option<DsbPathState> {
        self.board.dsb.as_ref().map(|b| DsbPathState {
            board: b.snapshot(),
            conversion: self.dsb_conversion.clone(),
        })
    }

    pub fn restore_dsb_path(&mut self, state: &DsbPathState) -> Result<(), crate::dsbz80::Error> {
        if !state.conversion.valid()
            || state.board.sound_ticks() != Some(state.conversion.time())
            || state
                .board
                .executed_ticks()
                .is_none_or(|limit| state.conversion.latest_sample_time() > limit)
        {
            return Err(crate::dsbz80::Error::InvalidSnapshot);
        }
        let board = self
            .board
            .dsb
            .as_mut()
            .ok_or(crate::dsbz80::Error::InvalidSnapshot)?;
        board.restore(&state.board)?;
        self.dsb_conversion = state.conversion.clone();
        self.samples.clear();
        Ok(())
    }

    /// Submit a main-board byte. The timed Model 1 path raises RXRDY only
    /// when reception finishes; the Model 2 HLE path keeps immediate delivery.
    pub fn send(&mut self, data: u8) {
        self.board.uart_send(data);
        self.irq_pending = self.board.serial.is_none();
    }

    /// Only the FM chip/converter, not a complete board or machine snapshot.
    /// A future full restore must also restore PCM, 68000, UART and run budgets.
    pub fn snapshot_fm_path(&self) -> FmPathState {
        self.board.ym.snapshot()
    }

    pub fn restore_fm_path(&mut self, state: &FmPathState) -> Result<(), &'static str> {
        self.board.ym.restore(state)?;
        self.samples.clear(); // host output is not emulated state
        Ok(())
    }

    /// Advance all sound devices on the 68000 instruction grid. The CPU core
    /// exposes instruction totals, not timed bus micro-operations: reads/writes
    /// occur at that instruction's start, then we render its elapsed interval.
    /// This preserves write order without pretending to be bus-cycle accurate.
    fn render_cycles(&mut self, cycles: usize) {
        if self.board.serial.is_some() {
            let mut remaining = cycles;
            while remaining > 0 {
                let s = self.board.serial.as_ref().unwrap();
                let step = remaining.min(s.to_edge());
                if let Some(dsb) = &mut self.board.dsb {
                    dsb.set_rx(s.driver.tx);
                }
                self.render_devices(step);
                let s = self.board.serial.as_mut().unwrap();
                for _ in 0..step {
                    s.tick_sound();
                }
                remaining -= step;
            }
        } else {
            self.render_devices(cycles);
        }
    }
    fn render_devices(&mut self, cycles: usize) {
        if self.board.dsb.is_some() {
            let mut remaining = cycles;
            while remaining > 0 {
                // At most one destination sample per chunk. Both converters
                // start at phase zero and retain their 224-clock phase.
                let step = remaining.min(224);
                let conversion = &mut self.dsb_conversion;
                let board = self.board.dsb.as_mut().unwrap();
                if board
                    .run_sound_cycles_with_audio(step as u32, |s| conversion.push(s))
                    .is_err()
                {
                    return; // sticky fault propagated by Model1System
                }
                let mut dsb_sample = [0; 2];
                conversion.advance(step, |s| dsb_sample = s);
                if self.dsb_muted {
                    dsb_sample = [0; 2];
                }
                let SoundBoard { ym, pcm, .. } = &mut self.board;
                let muted = self.muted;
                let gains = self.gains;
                let samples = &mut self.samples;
                ym.advance(step, |fm| {
                    let mixed = mix_with_gains(
                        pcm[0].generate(),
                        pcm[1].generate(),
                        fm,
                        muted,
                        dsb_sample,
                        gains,
                    );
                    if samples.len() < MAX_BUFFERED_SAMPLES {
                        samples.push_back(mixed);
                    }
                });
                remaining -= step;
            }
            return;
        }
        let SoundBoard { ym, pcm, .. } = &mut self.board;
        let muted = self.muted;
        let samples = &mut self.samples;
        ym.advance(cycles, |fm| {
            // All chips run even if muted or if the bounded output queue is full.
            let pcm1 = pcm[0].generate();
            let pcm2 = pcm[1].generate();
            let mixed = mix_with_gains(pcm1, pcm2, fm, muted, [0; 2], self.gains);
            if samples.len() < MAX_BUFFERED_SAMPLES {
                samples.push_back(mixed);
            }
        });
    }

    /// Runs the board for `i960_cycles` of main-board time.
    pub fn run(&mut self, i960_cycles: i32, i960_hz: u32) {
        if self.serial_fault().is_some() {
            return;
        }
        if self.board.dsb.as_ref().is_some_and(|d| d.fault().is_some()) {
            return;
        }
        // Convert the main board's budget into this board's clock.
        let scaled = i960_cycles as i64 * SND_CPU_HZ as i64 + self.main_fraction as i64;
        self.remainder += scaled.div_euclid(i960_hz as i64);
        self.main_fraction = scaled.rem_euclid(i960_hz as i64) as u64;
        // The 8251's rxrdy line drives the driver's level-2 interrupt. Assert
        // it when a byte arrives; then re-assert it (edge per byte) each time
        // the driver actually consumes one while more remain, so a burst -- VF
        // sends three command bytes back to back -- drains one interrupt at a
        // time. Asserting unconditionally while the FIFO is non-empty instead
        // leaves a stale level-2 pending that fires again after the byte is
        // read, and the driver then services a phantom 0 and stops making sound.
        if self.irq_pending {
            self.irq_pending = false;
            self.cpu
                .exception(Exception::from(Vector::Level2Interrupt as u8));
        }
        while self.remainder > 0 {
            // The CPU library queues exceptions, not physical IRQ levels. Only
            // inject a live RXRDY when it can be accepted by this instruction;
            // otherwise polling while masked would leave a phantom pending IRQ.
            if self.board.serial.as_ref().is_some_and(|s| s.driver.irq())
                && self.cpu.regs.sr.interrupt_mask < 2
            {
                self.cpu
                    .exception(Exception::from(Vector::Level2Interrupt as u8));
            }
            let reads_before = self.board.rx_read_count;
            let (used, exception) = self.cpu.interpreter_exception(&mut self.board);
            if self.board.rx_read_count > reads_before && !self.board.rx.is_empty() {
                self.cpu
                    .exception(Exception::from(Vector::Level2Interrupt as u8));
            }
            if let Some(vector) = exception {
                self.exception_counts[vector as usize] += 1;
                self.cpu.exception(Exception::from(vector));
            }
            // A stopped CPU reports no cycles; do not spin on it.
            if used == 0 {
                let rest = if self.board.serial.is_some() {
                    (self.remainder as usize).min(20)
                } else {
                    self.remainder as usize
                };
                self.remainder -= rest as i64;
                self.render_cycles(rest);
            } else {
                self.remainder -= used as i64;
                self.render_cycles(used);
            }
            if self.serial_fault().is_some()
                || self.board.dsb.as_ref().is_some_and(|d| d.fault().is_some())
            {
                break;
            }
        }
    }
}

/// Either of the two sound boards a Model 2 game can ship with: the
/// segam1audio MultiPCM board (original Model 2, e.g. Daytona) or the
/// SCSP board (Model 2A/2B/2C, e.g. Sega Rally). The main board talks to both
/// through the same i8251 UART, so this exposes the union of what the memory
/// map and front end use.
pub enum Sound {
    // Boxed because the two boards differ by several hundred kilobytes of
    // sample RAM and voice state, and every `Sound` would otherwise be as big
    // as the larger of them.
    MultiPcm(Box<SoundSystem>),
    Scsp(Box<crate::sound2a::SoundSystem2A>),
}

impl Sound {
    pub fn set_gains(&mut self, gains: AudioGains) {
        match self {
            Self::MultiPcm(s) => s.set_gains(gains),
            Self::Scsp(s) => s.set_gain(gains.scsp),
        }
    }
    pub fn sources(&self) -> &'static [AudioSource] {
        match self {
            Self::MultiPcm(_) => MULTIPCM_SOURCES,
            Self::Scsp(_) => SCSP_SOURCES,
        }
    }

    pub fn set_mutes(&mut self, mutes: AudioMutes) {
        match self {
            Self::MultiPcm(s) => s.set_mutes(mutes),
            Self::Scsp(s) => s.set_muted(mutes.scsp),
        }
    }

    /// The i960 has put a byte on the wire.
    pub fn send(&mut self, data: u8) {
        match self {
            Sound::MultiPcm(s) => s.send(data),
            Sound::Scsp(s) => s.send(data),
        }
    }

    /// i8251 mode/command register write from the i960 side.
    pub fn control(&mut self, val: u8) {
        match self {
            Sound::MultiPcm(s) => s.board.uart_control(val),
            Sound::Scsp(s) => s.board.uart_control(val),
        }
    }

    /// Takes the byte the driver sent back, if any (a read of the UART's data
    /// register consumes it).
    pub fn take_reply(&mut self) -> u8 {
        match self {
            Sound::MultiPcm(s) => s.board.tx.take().unwrap_or(0),
            Sound::Scsp(s) => s.board.tx.take().unwrap_or(0),
        }
    }

    /// True while the driver has a byte waiting to be read back.
    pub fn reply_ready(&self) -> bool {
        match self {
            Sound::MultiPcm(s) => s.board.uart_rx_ready(),
            Sound::Scsp(s) => s.board.uart_rx_ready(),
        }
    }

    /// True while the UART can accept another byte for the board.
    pub fn tx_ready(&self) -> bool {
        match self {
            Sound::MultiPcm(s) => s.board.uart_tx_ready(),
            Sound::Scsp(s) => s.board.uart_tx_ready(),
        }
    }

    /// Runs the board for `i960_cycles` of main-board time.
    pub fn run(&mut self, i960_cycles: i32, i960_hz: u32) {
        match self {
            Sound::MultiPcm(s) => s.run(i960_cycles, i960_hz),
            Sound::Scsp(s) => s.run(i960_cycles, i960_hz),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        match self {
            Sound::MultiPcm(s) => s.sample_rate(),
            Sound::Scsp(s) => s.sample_rate(),
        }
    }

    /// Drains the rendered stereo output.
    pub fn drain_samples(&mut self) -> std::collections::vec_deque::Drain<'_, (i16, i16)> {
        match self {
            Sound::MultiPcm(s) => s.samples.drain(..),
            Sound::Scsp(s) => s.samples.drain(..),
        }
    }

    /// The segam1audio board, for tools that report MultiPCM-specific stats.
    pub fn as_multi_pcm(&self) -> Option<&SoundSystem> {
        match self {
            Sound::MultiPcm(s) => Some(s),
            Sound::Scsp(_) => None,
        }
    }

    /// The SCSP board, for tools that report 2A-specific stats.
    pub fn as_scsp(&self) -> Option<&crate::sound2a::SoundSystem2A> {
        match self {
            Sound::MultiPcm(_) => None,
            Sound::Scsp(s) => Some(s),
        }
    }
}
