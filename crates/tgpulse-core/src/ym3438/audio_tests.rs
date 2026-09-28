use super::tests::{write, Hash};
use super::*;

// Shared synthetic program with tools/ym3438-reference.cpp (no game ROMs).
fn tone(chip: &mut Ym3438, pattern: u32, channel: usize) {
    let ch = (channel % 3 + 0x100 * (channel / 3)) as u16;
    write(chip, 0xb0 + ch, (pattern & 63) as u8);
    write(
        chip,
        0xb4 + ch,
        [0xc0, 0x80, 0x40][channel % 3]
            | (((pattern >> 14) & 3) << 4) as u8
            | ((pattern >> 16) & 7) as u8,
    );
    write(chip, 0x22, ((pattern >> 10) & 15) as u8);
    for (slot, offset) in [0, 8, 4, 12].into_iter().enumerate() {
        let op = ch + offset;
        write(chip, 0x30 + op, [0x01, 0x32, 0x73, 0x40][slot]);
        write(chip, 0x40 + op, [12, 24, 32, 16][slot]);
        write(chip, 0x50 + op, [0x1f, 0x54, 0x98, 0xdf][slot]);
        write(chip, 0x60 + op, 0x80 | [18, 12, 24, 16][slot]);
        write(chip, 0x70 + op, [6, 10, 8, 4][slot]);
        write(chip, 0x80 + op, [0x28, 0x49, 0x3a, 0x6f][slot]);
        write(chip, 0x90 + op, ((pattern >> 6) & 15) as u8);
    }
    write(chip, 0xa4 + ch, 0x22);
    write(chip, 0xa0 + ch, 0x69 + 7 * channel as u8);
    for i in 0..3 {
        write(chip, 0xac + i, 0x18 + 4 * i as u8);
        write(chip, 0xa8 + i, 0x47 + 37 * i as u8);
    }
    let mode = (pattern >> 19) & 3;
    write(chip, 0x24, 240);
    write(chip, 0x25, 2);
    write(chip, 0x27, ((mode << 6) | u32::from(mode == 2)) as u8);
    write(
        chip,
        0x28,
        (channel % 3) as u8 | ((channel / 3) as u8) << 2 | if mode == 2 { 0 } else { 0xf0 },
    );
}

fn render(chip: &mut Ym3438, clocks: u32) -> Vec<[i32; 2]> {
    let mut samples = Vec::new();
    chip.advance_with_output(clocks, |frame| samples.push(frame));
    samples
}

#[test]
fn pinned_ymfm_sample_streams() {
    let mut chip = Ym3438::new();
    let mut compared_frames = 0;
    for (number, line) in include_str!("audio.trace").lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (command, expected) = line.split_once('#').expect("missing oracle audio hash");
        let fields: Vec<_> = command.split_whitespace().collect();
        let arg = |index: usize| fields[index].parse::<u32>().unwrap();
        let samples = match fields[0] {
            "T" => {
                chip = Ym3438::new();
                tone(&mut chip, arg(1), arg(2) as usize);
                Vec::new()
            }
            "U" => {
                tone(&mut chip, arg(1), arg(2) as usize);
                Vec::new()
            }
            "P" => {
                chip = Ym3438::new();
                Vec::new()
            }
            "R" => {
                chip.reset();
                Vec::new()
            }
            "W" => {
                chip.write(arg(1) as u8, arg(2) as u8);
                Vec::new()
            }
            "A" => render(&mut chip, arg(1)),
            "N" => render(&mut chip, arg(1) * 144),
            _ => panic!("unknown audio trace command"),
        };
        compared_frames += samples.len();
        let mut digest = Hash::new();
        digest.add(samples.len() as u64, 4);
        for sample in samples {
            for value in sample {
                digest.add(value as u32 as u64, 4);
            }
        }
        assert_eq!(
            digest.0,
            u64::from_str_radix(expected.trim(), 16).unwrap(),
            "YMFM sample trace line {}: {command}",
            number + 1
        );
    }
    assert!(compared_frames > 1_000_000);
}

#[test]
fn mid_note_serialized_continuation_is_sample_identical_with_different_slices() {
    // Feedback + slow attack/decay, SSG-EG, LFO and CSM; never restore into
    // an already matching chip, which could conceal missing mutable state.
    for pattern in [
        63,
        63 | (14 << 6),
        63 | (15 << 10) | (7 << 16),
        63 | (2 << 19),
    ] {
        let mut chip = Ym3438::new();
        tone(&mut chip, pattern, 2);
        chip.advance(144 * 1001 + 37);
        let bytes = bincode::serialize(&chip.snapshot()).unwrap();
        let snapshot: State = bincode::deserialize(&bytes).unwrap();
        let mut restored = Ym3438::new();
        restored.restore(&snapshot).unwrap();
        let expected = render(&mut chip, 144 * 5000 + 103);
        assert!(
            expected.iter().any(|&s| s != [0, 0]),
            "vacuous silent round-trip"
        );
        let mut actual = Vec::new();
        let mut remaining = 144 * 5000 + 103;
        for slice in [1, 13, 127, 288, 11].into_iter().cycle() {
            let take = slice.min(remaining);
            restored.advance_with_output(take, |sample| actual.push(sample));
            remaining -= take;
            if remaining == 0 {
                break;
            }
        }
        assert_eq!(actual, expected);
        assert_eq!(restored.snapshot(), chip.snapshot());
        for target in [&mut restored, &mut chip] {
            write(target, 0x28, 2);
        }
        assert_eq!(
            render(&mut restored, 144 * 1024),
            render(&mut chip, 144 * 1024)
        );
    }
}

#[test]
fn discard_output_never_freezes_chip_and_reset_retains_clock_phase() {
    let mut audible = Ym3438::new();
    let mut discarded = Ym3438::new();
    tone(&mut audible, 63, 0);
    tone(&mut discarded, 63, 0);
    assert!(render(&mut audible, 144 * 800 + 17)
        .iter()
        .any(|&s| s != [0, 0]));
    discarded.advance(144 * 800 + 17);
    assert_eq!(audible.snapshot(), discarded.snapshot());
    assert_eq!(
        render(&mut audible, 144 * 200),
        render(&mut discarded, 144 * 200)
    );
    audible.reset();
    assert!(render(&mut audible, 144 * 200).iter().all(|&s| s == [0, 0]));
    assert_eq!(audible.state.sample_remaining, 127);
}

#[test]
fn dac_has_signed_nine_bit_data_and_independent_stereo_routing() {
    let mut chip = Ym3438::new();
    write(&mut chip, 0x1b6, 0x80);
    write(&mut chip, 0x2b, 0x80);
    write(&mut chip, 0x2a, 0xff);
    assert_eq!(render(&mut chip, 144), [[5418, 0]]);
    write(&mut chip, 0x2c, 8);
    assert_eq!(render(&mut chip, 144), [[5440, 0]]);
    write(&mut chip, 0x1b6, 0x40);
    write(&mut chip, 0x2a, 0);
    assert_eq!(render(&mut chip, 144), [[0, -5440]]);
}
