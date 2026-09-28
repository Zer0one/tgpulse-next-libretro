use super::*;

#[derive(Default)]
struct Writer {
    data: Vec<u8>,
    bits: usize,
}
impl Writer {
    fn put(&mut self, value: usize, count: usize) {
        for bit in (0..count).rev() {
            if self.bits % 8 == 0 {
                self.data.push(0);
            }
            self.data[self.bits / 8] |= (((value >> bit) & 1) as u8) << (7 - self.bits % 8);
            self.bits += 1;
        }
    }
}
// All input is synthetic, constructed from the public frame grammar, no ROMs.
fn frame(w: &mut Writer, case: usize, sequence: usize) {
    let mode = [0, 2, 3][case % 3];
    let rate = case / 3 % 3;
    let channels = if mode == 3 { 1 } else { 2 };
    let bitrate = if case / 9 % 2 == 0 {
        if channels == 1 {
            1
        } else {
            4
        }
    } else {
        9
    };
    let param = PARAM[channels - 1][rate][bitrate] as usize;
    let crc = case % 2 == 0;
    w.put(0xfff, 12);
    w.put(6, 3);
    w.put((!crc) as usize, 1);
    w.put(bitrate, 4);
    w.put(rate, 2);
    w.put(0, 2);
    w.put(mode, 2);
    w.put(0, 6);
    if crc {
        w.put(0x1234, 16);
    }
    let mut alloc = [[0; 32]; 2];
    for band in 0..BANDS[param] {
        for channel in 0..channels {
            let n = ALLOC_BITS[param][band];
            let index = (band + channel * 3 + sequence * 5 + case) % (1 << n);
            alloc[channel][band] = ALLOC[param][band][index] as usize;
            w.put(index, n);
        }
    }
    for band in 0..BANDS[param] {
        for channel in 0..channels {
            if alloc[channel][band] != 0 {
                w.put((band + channel + sequence) % 4, 2);
            }
        }
    }
    for band in 0..BANDS[param] {
        for channel in 0..channels {
            if alloc[channel][band] == 0 {
                continue;
            }
            let scfsi = (band + channel + sequence) % 4;
            let count = [3, 2, 1, 2][scfsi];
            for step in 0..count {
                w.put((band + channel * 5 + step + sequence) % 30 + 3, 6);
            }
        }
    }
    for group in 0..12 {
        for band in 0..BANDS[param] {
            for channel in 0..channels {
                let q = alloc[channel][band];
                if q == 0 {
                    continue;
                }
                let (bits, modulus, cube) = quantization(q);
                let seed = case * 173 + sequence * 93 + group * 123 + band * 7 + channel * 43;
                if modulus != 0 {
                    w.put(seed % (modulus * modulus * modulus), cube);
                } else {
                    for step in 0..3 {
                        w.put((seed + step * 57) % (1 << bits), bits);
                    }
                }
            }
        }
    }
}
fn fixture(case: usize) -> Writer {
    let mut w = Writer::default();
    w.put(0, case % 8); // non-byte-aligned sync
    for sequence in 0..5 {
        frame(&mut w, case, sequence);
        w.put(0, 7);
    }
    w
}
pub(crate) fn stream(case: usize) -> Vec<u8> {
    fixture(case).data
}
fn digest(samples: &[i16]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for s in samples {
        for byte in s.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    hash
}
fn trace(data: &[u8], limit: usize) -> String {
    let mut decoder = Decoder::default();
    let mut pos = 0;
    let mut output = String::new();
    while let Some(frame) = decoder.decode(data, pos, limit).unwrap() {
        output += &format!(
            "{} {} {} {} {:016x}\n",
            frame.next_bit,
            frame.channels,
            frame.sample_rate,
            frame.samples.len() / frame.channels,
            digest(&frame.samples)
        );
        pos = frame.next_bit;
    }
    output
}

#[test]
fn synthetic_reference_vectors() {
    let expected = include_str!("reference.txt");
    let mut combined = String::new();
    for case in 0..18 {
        let w = fixture(case);
        let actual = trace(&w.data, w.bits);
        combined += &format!("case {case}\n{actual}");
        assert_eq!(actual.lines().count(), 5);
        // Optional export for the standalone MAME oracle, not a runtime path.
        if let Some(dir) = std::env::var_os("TGPULSE_MPEG_AUDIT_DIR") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::write(dir.join(format!("case-{case}.mp2")), &w.data).unwrap();
            std::fs::write(dir.join(format!("case-{case}.rust.txt")), &actual).unwrap();
        }
    }
    assert_eq!(combined, expected.replace("\r\n", "\n"));
}

#[test]
fn history_roundtrip_clear_and_repeated_segment_continuation() {
    let w = fixture(17);
    let mut a = Decoder::default();
    let first = a.decode(&w.data, 0, w.bits).unwrap().unwrap();
    assert!(first.samples.iter().any(|&x| x != 0));
    let saved = bincode::serialize(&a.snapshot()).unwrap();
    let mut b = Decoder::default();
    b.restore(&bincode::deserialize(&saved).unwrap()).unwrap();
    let mut pos = first.next_bit;
    for _ in 0..3 {
        let x = a.decode(&w.data, pos, w.bits).unwrap().unwrap();
        let y = b.decode(&w.data, pos, w.bits).unwrap().unwrap();
        assert_eq!(x, y);
        assert_eq!(a.snapshot(), b.snapshot());
        pos = x.next_bit;
    }
    // A DSB loop seeks input without clearing synthesis history.
    assert_eq!(a.decode(&w.data, 0, w.bits), b.decode(&w.data, 0, w.bits));
    a.clear();
    assert_eq!(a.decode(&w.data, 0, w.bits).unwrap().unwrap(), first);
    let before = a.snapshot();
    let mut invalid = before.clone();
    invalid.history[0] = f64::NAN;
    assert_eq!(a.restore(&invalid), Err(Error::InvalidState));
    assert_eq!(a.snapshot(), before);
}

#[test]
fn every_truncation_is_bounded_and_does_not_mutate_history() {
    let mut w = Writer::default();
    frame(&mut w, 0, 1);
    let mut d = Decoder::default();
    d.decode(&w.data, 0, w.bits).unwrap().unwrap();
    let before = d.snapshot();
    for limit in 0..w.bits {
        assert!(!matches!(d.decode(&w.data, 0, limit), Ok(Some(_))));
        assert_eq!(d.snapshot(), before);
    }
    assert_eq!(d.decode(&w.data, 1, 0), Err(Error::Bounds));
    assert_eq!(
        d.decode(&w.data, 0, w.data.len() * 8 + 1),
        Err(Error::Bounds)
    );
}

#[test]
fn malformed_headers_and_joint_stereo_are_explicit_errors() {
    let mut w = Writer::default();
    frame(&mut w, 1, 0); // no CRC
    w.data[2] |= 0x0c; // invalid sampling-rate index
    assert_eq!(
        Decoder::default().decode(&w.data, 0, w.bits),
        Err(Error::InvalidHeader)
    );
    let mut w = Writer::default();
    frame(&mut w, 1, 0);
    w.data[3] = 0x40; // joint stereo
    assert_eq!(
        Decoder::default().decode(&w.data, 0, w.bits),
        Err(Error::JointStereo)
    );
    assert_eq!(Decoder::default().decode(&[0; 32], 0, 256), Ok(None));
}

#[test]
fn seeded_malformed_data_never_panics_or_commits_failed_frames() {
    let mut seed = 0x243f6a88u32;
    let mut decoder = Decoder::default();
    for n in 0..500 {
        let mut data = vec![0; n % 257 + 4];
        for byte in &mut data {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *byte = seed as u8;
        }
        if n % 2 == 0 {
            data[0] = 0xff;
            data[1] = 0xfd;
        }
        let before = decoder.snapshot();
        let result = decoder.decode(&data, 0, data.len() * 8);
        if !matches!(result, Ok(Some(_))) {
            assert_eq!(decoder.snapshot(), before);
        }
    }
}
