use super::*;
use crate::dsbz80::{AudioSample, Board, FIRMWARE_SIZE};
use z80::Z80_io;

fn stream() -> Vec<u8> {
    crate::mpeg::tests::stream(6) // stereo, 32 kHz, five nonzero synthetic frames
}
fn setup() -> (Audio, Playback) {
    let data = stream();
    let p = Playback {
        end: data.len() as u32,
        mode: 1,
        ..Playback::default()
    };
    (Audio::new(data).unwrap(), p)
}
fn address(b: &mut Board, port: u16, value: u32) {
    for i in 0..3 {
        b.cpu.io.port_out(port + i, (value >> ((2 - i) * 8)) as u8);
    }
}
fn board(looping: bool) -> Board {
    let mut firmware = vec![0; FIRMWARE_SIZE];
    firmware[0] = 0x76; // HALT; device clocks continue
    let data = stream();
    let size = data.len() as u32;
    let mut b = Board::with_mpeg(&firmware, data).unwrap();
    address(&mut b, 0xe2, 0);
    address(&mut b, 0xe5, size);
    b.cpu.io.port_out(0xe0, if looping { 2 } else { 1 });
    b
}
fn capture(b: &mut Board, clocks: u32) -> Vec<AudioSample> {
    let mut out = vec![];
    b.run_with_audio(clocks, |_| {}, |s| out.push(s)).unwrap();
    out
}
fn bytes(b: &Board) -> Vec<u8> {
    bincode::serialize(&b.snapshot()).unwrap()
}

#[test]
fn buffered_pcm_matches_decoder_and_reports_prefetched_bit_position() {
    let data = stream();
    let mut decoder = mpeg::Decoder::default();
    let (mut audio, mut p) = setup();
    let mut bit = 0;
    for _ in 0..5 {
        let frame = decoder.decode(&data, bit, data.len() * 8).unwrap().unwrap();
        for pair in frame.samples.chunks_exact(2) {
            let actual = audio.sample(&mut p).unwrap();
            assert_eq!(
                actual,
                [pair[0], pair[1]].map(|x| x as f32 * 127.0 / 4194304.0)
            );
            // Position changes on frame decode, not on every DAC sample.
            assert_eq!(p.bit_position as usize, frame.next_bit);
        }
        bit = frame.next_bit;
    }
    assert_eq!(audio.sample(&mut p).unwrap(), [0.0; 2]);
    assert_eq!(p.mode, 0);
}

#[test]
fn live_volume_pan_stop_and_retrigger_follow_reference_buffer_semantics() {
    let (mut a, mut p) = setup();
    a.sample(&mut p).unwrap();
    let decoder_before = a.decoder.snapshot();
    p.write(0xe0, 1).unwrap(); // must keep the current decoded frame
    assert_eq!(p.bit_position, 0);
    for pan in 0..=2 {
        p.write(0xe9, pan).unwrap();
        p.write(0xe8, !43).unwrap();
        let l = a.pcm[a.cursor];
        let r = a.pcm[a.cursor + 1];
        let pair = match pan {
            0 => [l, r],
            1 => [l, l],
            _ => [r, r],
        };
        assert_eq!(
            a.sample(&mut p).unwrap(),
            pair.map(|x| x as f32 * 43.0 / 4194304.0)
        );
        assert_eq!(a.decoder.snapshot(), decoder_before);
    }
    a.stop();
    p.write(0xe0, 0).unwrap();
    assert_eq!(a.sample(&mut p).unwrap(), [0.0; 2]);
    assert!(a.pcm.is_empty());
    p.write(0xe0, 1).unwrap();
    a.sample(&mut p).unwrap();
    assert_ne!(a.decoder.snapshot(), decoder_before);
}

#[test]
fn loop_uses_latched_end_and_retains_synthesis_history() {
    let (mut a, mut p) = setup();
    let original_end = p.end;
    p.mode = 2;
    // A one-frame loop with a nonzero end latch, entered after five frames.
    let first = mpeg::Decoder::default()
        .decode(&stream(), 0, stream().len() * 8)
        .unwrap()
        .unwrap();
    p.loop_start = 0;
    p.loop_end = first.next_bit.div_ceil(8) as u32;
    for _ in 0..5760 {
        a.sample(&mut p).unwrap();
    }
    let history = a.decoder.snapshot();
    let mut reference = mpeg::Decoder::default();
    reference.restore(&history).unwrap();
    let expected = reference
        .decode(&stream(), 0, p.loop_end as usize * 8)
        .unwrap()
        .unwrap();
    let out = a.sample(&mut p).unwrap();
    assert_eq!(
        out,
        [expected.samples[0], expected.samples[1]].map(|x| x as f32 * 127.0 / 4194304.0)
    );
    assert_eq!(p.end, p.loop_end);
    assert_ne!(p.end, original_end);
    assert_eq!(a.decoder.snapshot(), reference.snapshot());
    // Zero loop end retains the active end marker.
    p.loop_end = 0;
    for _ in 1..1152 {
        a.sample(&mut p).unwrap();
    }
    a.sample(&mut p).unwrap();
    assert_eq!(p.end, first.next_bit.div_ceil(8) as u32);
    assert_eq!(p.mode, 2);
}

#[test]
fn multipart_loop_switches_to_a_different_latched_segment() {
    let mut data = stream();
    let split = data.len();
    data.extend(crate::mpeg::tests::stream(7)); // different stereo content
    let mut p = Playback {
        mode: 2,
        end: split as u32,
        loop_start: split as u32,
        loop_end: data.len() as u32,
        ..Playback::default()
    };
    let mut a = Audio::new(data.clone()).unwrap();
    for _ in 0..5760 {
        a.sample(&mut p).unwrap();
    }
    let mut reference = mpeg::Decoder::default();
    reference.restore(&a.decoder.snapshot()).unwrap();
    let expected = reference
        .decode(&data, split * 8, data.len() * 8)
        .unwrap()
        .unwrap();
    assert_eq!(
        a.sample(&mut p).unwrap(),
        [expected.samples[0], expected.samples[1]].map(|x| x as f32 * 127.0 / 4194304.0)
    );
    assert_eq!(p.bit_position as usize, expected.next_bit);
    assert_eq!(p.end as usize, data.len());
    assert_eq!(a.decoder.snapshot(), reference.snapshot());
}

#[test]
fn mpeg_fault_stops_board_and_position_ports_expose_decode_progress() {
    let mut b = board(false);
    capture(&mut b, 125);
    let port_position = (0..3).fold(0, |value, i| {
        (value << 8) | u32::from(b.cpu.io.port_in(0xe2 + i))
    });
    assert_eq!(port_position, b.playback().bit_position >> 3);
    assert!(port_position > 0);
    b.cpu.io.port_out(0xe0, 0);
    address(&mut b, 0xe5, stream().len() as u32 + 1);
    b.cpu.io.port_out(0xe0, 1);
    let fault = Err(Error::Mpeg(mpeg::Error::Bounds));
    assert_eq!(b.run(1000, |_| {}), fault);
    let state = bytes(&b);
    assert_eq!(b.run(1000, |_| {}), fault);
    assert_eq!(bytes(&b), state);
}

#[test]
fn empty_truncated_loops_terminate_and_invalid_resources_fault_explicitly() {
    assert!(matches!(Audio::new(vec![]), Err(Error::MpegRegionSize)));
    for data in [vec![0; 8], vec![0xff, 0xfd]] {
        let mut a = Audio::new(data.clone()).unwrap();
        let mut p = Playback {
            mode: 2,
            end: data.len() as u32,
            bit_position: 8,
            ..Playback::default()
        };
        assert_eq!(a.sample(&mut p).unwrap(), [0.0; 2]);
        assert_eq!(p.mode, 0);
    }
    let (mut a, mut p) = setup();
    p.end += 1;
    assert_eq!(a.sample(&mut p), Err(Error::Mpeg(mpeg::Error::Bounds)));
    for case in [0, 8] {
        // 44.1 kHz stereo / 32 kHz mono, no guessed conversion
        let data = crate::mpeg::tests::stream(case);
        let mut p = Playback {
            mode: 1,
            end: data.len() as u32,
            ..Playback::default()
        };
        assert!(matches!(
            Audio::new(data).unwrap().sample(&mut p),
            Err(Error::MpegFormat { .. })
        ));
    }
}

#[test]
fn clock_slicing_discard_and_mid_buffer_or_loop_restore_are_identical() {
    let mut whole = board(true);
    let mut sliced = board(true);
    let expected = capture(&mut whole, 200_003);
    let mut actual = vec![];
    for _ in 0..200_003 {
        actual.extend(capture(&mut sliced, 1));
    }
    assert_eq!(actual, expected);
    assert_eq!(bytes(&whole), bytes(&sliced));
    assert_eq!(expected.len(), 1600);
    assert!(expected.iter().any(|s| s.channels != [0.0; 2]));
    assert!(expected
        .iter()
        .enumerate()
        .all(|(i, s)| s.clock == (i as u64 + 1) * 125));
    for prefix in [37_003, 720_000, 727_001] {
        let mut original = board(true);
        original.run(prefix, |_| {}).unwrap(); // output discarded, state advances
        let saved = bytes(&original);
        let mut restored = board(true);
        restored
            .restore(&bincode::deserialize(&saved).unwrap())
            .unwrap();
        assert_eq!(
            capture(&mut original, 800_000),
            capture(&mut restored, 800_000)
        );
        assert_eq!(bytes(&original), bytes(&restored));
    }
}

#[test]
fn invalid_audio_snapshot_is_rejected_atomically_and_roms_are_not_saved() {
    let mut b = board(false);
    capture(&mut b, 1000);
    let before = bytes(&b);
    let state = b.snapshot();
    for field in 0..4 {
        let mut bad = state.clone();
        let a = bad.audio.as_mut().unwrap();
        match field {
            0 => a.cursor = 1,
            1 => a.phase = 125,
            2 => a.rom_size += 1,
            _ => a.pcm.push(0),
        }
        assert_eq!(b.restore(&bad), Err(Error::InvalidSnapshot));
        assert_eq!(bytes(&b), before);
    }
    let mut no_audio = Board::new(&vec![0; FIRMWARE_SIZE]).unwrap();
    assert_eq!(no_audio.restore(&state), Err(Error::InvalidSnapshot));
    assert_eq!(b.restore(&no_audio.snapshot()), Err(Error::InvalidSnapshot));
    assert_eq!(bytes(&b), before);
}

#[test]
fn stop_and_soft_reset_clear_buffer_but_keep_decoder_history() {
    let mut b = board(false);
    capture(&mut b, 1000);
    let history = b.cpu.io.audio.as_ref().unwrap().decoder.snapshot();
    b.cpu.io.port_out(0xe0, 0);
    assert!(capture(&mut b, 1000).iter().all(|s| s.channels == [0.0; 2]));
    b.cpu.io.port_out(0xe0, 1);
    capture(&mut b, 1000);
    let history2 = b.cpu.io.audio.as_ref().unwrap().decoder.snapshot();
    assert_ne!(history, history2);
    b.reset();
    assert_eq!(
        b.cpu.io.audio.as_ref().unwrap().decoder.snapshot(),
        history2
    );
    assert!(b.cpu.io.audio.as_ref().unwrap().pcm.is_empty());
    assert!(capture(&mut b, 1000).iter().all(|s| s.channels == [0.0; 2]));
}
