//! Versioned, memory-owned Model 1 save states. No filesystem or host transport.
use super::*;
use bincode::Options;
use sha1::{Digest, Sha1};

const MAGIC: &[u8; 8] = b"TGP1STAT";
// v2 adds the board identity and write-only diagnostic LCD state.
// v3 adds the clocked NetMerc tracking peer, including in-flight serial state.
// v5 preserves NetMerc's first-decoded-record startup publication gate.
const VERSION: u32 = 5;
// magic + version + resource hash + payload length + payload checksum
const HEADER_BYTES: usize = 8 + 4 + 20 + 8 + 20;
/// Maximum complete encoded state. Frontends should enforce this before reading
/// a file; the core also checks it before any decoding/allocation from payload.
pub const MAX_STATE_BYTES: usize = 64 * 1024 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
struct State {
    motherboard: MotherboardState,
    io: crate::model1board::BoardState,
    video: crate::model1_video::VideoState,
    sound: crate::sound::SoundState,
}

fn codec() -> impl Options {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_little_endian()
        .with_limit((MAX_STATE_BYTES - HEADER_BYTES) as u64)
        .reject_trailing_bytes()
}

/// Hash length-delimited, normalized loaded resources, never filenames or
/// operator defaults. SHA-1 reuses the loader dependency; it identifies ROMs
/// and detects accidental corruption, not authentication of untrusted states.
pub(super) fn resource_identity(roms: &Model1Roms) -> [u8; 20] {
    fn bytes(h: &mut Sha1, data: &[u8]) {
        h.update((data.len() as u64).to_le_bytes());
        h.update(data);
    }
    fn words(h: &mut Sha1, data: &[u32]) {
        h.update((data.len() as u64).to_le_bytes());
        for value in data {
            h.update(value.to_le_bytes());
        }
    }
    let mut hash = Sha1::new();
    hash.update(b"TGPulse Model 1 resources v1");
    hash.update([
        match roms.ioboard_kind {
            crate::model1board::Kind::Original => 0,
            crate::model1board::Kind::WingWar => 1,
            crate::model1board::Kind::WingWarR360 => 2,
            crate::model1board::Kind::NetMerc => 3,
        },
        u8::from(roms.comm_board),
        u8::from(roms.dsb.is_some()),
        u8::from(roms.netmerc_procedural_audio),
    ]);
    for data in [
        &roms.maincpu,
        &roms.tgp,
        &roms.iocpu,
        &roms.sndcpu,
        &roms.mpcm1,
        &roms.mpcm2,
    ] {
        bytes(&mut hash, data);
    }
    for data in [&roms.copro_tables, &roms.polygons, &roms.copro_data] {
        words(&mut hash, data);
    }
    if let Some(dsb) = &roms.dsb {
        bytes(&mut hash, &dsb.firmware);
        bytes(&mut hash, &dsb.mpeg);
    }
    hash.finalize().into()
}

fn encode(state: &State, identity: &[u8; 20]) -> Result<Vec<u8>, String> {
    let payload = codec().serialize(state).map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(identity);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&Sha1::digest(&payload));
    bytes.extend(payload);
    Ok(bytes)
}

fn decode(bytes: &[u8], identity: &[u8; 20]) -> Result<State, String> {
    if bytes.len() < HEADER_BYTES || bytes.len() > MAX_STATE_BYTES {
        return Err("invalid Model 1 save-state size".into());
    }
    if &bytes[..8] != MAGIC {
        return Err("not a Model 1 save state".into());
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(format!(
            "Model 1 save-state version {version} is unsupported (expected {VERSION})"
        ));
    }
    if &bytes[12..32] != identity {
        return Err(
            "Model 1 save state belongs to different ROM resources or board variant".into(),
        );
    }
    let size = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
    let payload = &bytes[HEADER_BYTES..];
    if size != payload.len() as u64 {
        return Err("Model 1 save-state length mismatch".into());
    }
    if Sha1::digest(payload)[..] != bytes[40..60] {
        return Err("Model 1 save-state checksum mismatch".into());
    }
    codec()
        .deserialize(payload)
        .map_err(|e| format!("invalid Model 1 save-state payload: {e}"))
}

impl Model1System {
    fn state_allowed(&self) -> Result<(), String> {
        // Transport is frontend-owned. A disconnected socket is insufficient:
        // the peer may still be running or reconnect later. No unilateral rewind
        // whenever COMM is fitted, even before CN/link establishment.
        if self.comm.is_some() {
            return Err("Model 1 save states require cabinet = single (COMM/network state cannot be restored independently)".into());
        }
        if self.ioboard.fault().is_some()
            || self.sound.serial_fault().is_some()
            || self.sound.dsb_fault().is_some()
        {
            return Err("Model 1 save/load is unavailable while a device fault is latched".into());
        }
        Ok(())
    }

    /// Capture a standalone machine between run/render calls. The returned
    /// bytes are frontend-owned; ROM resources must remain immutable after new.
    pub fn save_state(&self) -> Result<Vec<u8>, String> {
        self.state_allowed()?;
        let state = State {
            motherboard: self.snapshot_motherboard()?,
            io: self.ioboard.snapshot(),
            video: self.snapshot_video(),
            sound: self.sound.snapshot_model1()?,
        };
        encode(&state, &self.resource_identity)
    }

    /// All-or-nothing in-memory restore. Errors preserve devices, NVRAM and
    /// pending audio. Success keeps frontend preferences/debug trace policy,
    /// clears stale core audio, and requires the frontend to redraw/flush its
    /// own output queues. This never writes NVRAM to disk or resets a CPU.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.state_allowed()?;
        let state = decode(bytes, &self.resource_identity)?;
        self.validate_motherboard_restore(&state.motherboard)?;
        state.video.validate()?;
        self.ioboard
            .validate_state(&state.io)
            .map_err(|e| e.to_string())?;
        // The sound restore is itself atomic and is the final fallible step.
        // No live state or output queue has been changed before it succeeds.
        self.sound.restore_model1(&state.sound)?;
        // Validated against this same machine above; no callbacks or concurrent
        // execution can change these invariants during an exclusive &mut call.
        self.ioboard
            .restore(&state.io)
            .expect("validated Model 1 I/O state");
        self.restore_video(&state.video)
            .expect("validated Model 1 video state");
        self.restore_motherboard(&state.motherboard)
            .expect("validated Model 1 motherboard state");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roms(kind: crate::model1board::Kind, dsb: bool) -> Model1Roms {
        Model1Roms {
            netmerc_procedural_audio: false,
            dsb: dsb.then(|| crate::loader::DsbRoms {
                firmware: vec![0x76; crate::dsbz80::FIRMWARE_SIZE],
                mpeg: vec![0; 16],
            }),
            comm_board: true,
            ioboard_kind: kind,
            nvram_default: vec![],
            maincpu: vec![0xcd, 0],
            tgp: vec![],
            copro_tables: vec![],
            polygons: vec![],
            copro_data: vec![],
            iocpu: vec![0x76; 0x10000],
            sndcpu: vec![],
            mpcm1: vec![1; 16],
            mpcm2: vec![2; 16],
            ioboard_config: vec![],
        }
    }

    fn machine(roms: &Model1Roms) -> Model1System {
        let mut s = Model1System::new(roms).unwrap();
        s.main_cpu.reg[v60::cpu::PC] = 0;
        s.nvram[0] = 0x5a;
        s.work_ram[4] = 0xa5;
        s.run_slice(65).unwrap();
        s
    }

    fn reseal(bytes: &mut [u8]) {
        let size = (bytes.len() - HEADER_BYTES) as u64;
        bytes[32..40].copy_from_slice(&size.to_le_bytes());
        let hash = Sha1::digest(&bytes[HEADER_BYTES..]);
        bytes[40..60].copy_from_slice(&hash);
    }

    #[test]
    fn netmerc_donor_preserves_program_and_active_audio_continuation() {
        use crate::{
            config::NetmercAudioDonor, loader::audio_donor::SampleBanks, model1board::Kind,
        };
        let mut resources = roms(Kind::NetMerc, false);
        resources.netmerc_procedural_audio = true;
        resources.sndcpu = vec![0; 16];
        resources.sndcpu[..4].copy_from_slice(&0x00f0fff0u32.to_be_bytes());
        resources.sndcpu[4..8].copy_from_slice(&8u32.to_be_bytes());
        resources.sndcpu[8..12].copy_from_slice(&[0x4e, 0x71, 0x60, 0xfc]);
        let original_program = resources.sndcpu.clone();
        let original_identity = resource_identity(&resources);
        let bank = || {
            let mut data = vec![0; 0x400000];
            data[..12].copy_from_slice(&[0, 1, 0, 0, 0, 0xff, 0xc0, 0, 0xf0, 0, 0xff, 0]);
            for i in 0..64 {
                data[0x100 + i] = (i as i8 * 2 - 64) as u8;
            }
            data
        };
        SampleBanks {
            pcm1: bank(),
            pcm2: bank(),
        }
        .apply(&mut resources)
        .unwrap();
        assert_eq!(resources.sndcpu, original_program);
        assert!(
            !resources.netmerc_procedural_audio,
            "valid donor replaces the fallback"
        );
        assert_ne!(resource_identity(&resources), original_identity);
        let identity = resource_identity(&resources);
        // Failed replacement is atomic; Off does not search the filesystem.
        assert!(SampleBanks {
            pcm1: bank(),
            pcm2: vec![]
        }
        .apply(&mut resources)
        .is_err());
        assert_eq!(resource_identity(&resources), identity);
        assert!(!crate::loader::audio_donor::apply_adjacent(
            &mut resources,
            std::path::Path::new("/absent/netmerc.zip"),
            NetmercAudioDonor::Off
        )
        .unwrap());
        let mut a = machine(&resources);
        for chip in &mut a.sound.board.pcm {
            for (reg, value) in [(0, 0), (1, 0), (2, 0), (3, 0x10), (5, 1), (4, 0x80)] {
                chip.write(2, reg);
                chip.write(0, value);
            }
        }
        a.run_slice(1237).unwrap();
        let saved = a.save_state().unwrap();
        let mut b = machine(&resources);
        b.load_state(&saved).unwrap();
        a.sound.samples.clear();
        for cycles in [127, 8119, 40003] {
            a.run_slice(cycles).unwrap();
            b.run_slice(cycles).unwrap();
            assert_eq!(a.sound.samples, b.sound.samples);
            assert_eq!(a.save_state().unwrap(), b.save_state().unwrap());
        }
        assert!(a.sound.samples.iter().any(|&(l, r)| l != 0 || r != 0));
        resources.mpcm1[0x110] ^= 1;
        assert!(machine(&resources)
            .load_state(&saved)
            .unwrap_err()
            .contains("different ROM resources"));
        resources.ioboard_kind = Kind::Original;
        assert!(SampleBanks {
            pcm1: bank(),
            pcm2: bank()
        }
        .apply(&mut resources)
        .is_err());
    }

    #[test]
    fn envelope_restores_all_variants_and_dsb_without_replacing_preferences() {
        use crate::model1board::Kind;
        for (kind, dsb) in [
            (Kind::Original, false),
            (Kind::Original, true),
            (Kind::WingWar, false),
            (Kind::WingWarR360, false),
            (Kind::NetMerc, false),
        ] {
            let roms = roms(kind, dsb);
            let mut a = machine(&roms);
            a.set_netmerc_city_workaround(false);
            a.frame_num = 17;
            a.palette_ram[7] = 0x5a;
            a.ioboard.eeprom_mut().data[3] = 0x1234;
            let saved = a.save_state().unwrap();
            assert!(saved.len() < MAX_STATE_BYTES);
            let mut b = machine(&roms);
            b.set_netmerc_city_workaround(true);
            b.run_slice(444).unwrap();
            b.nvram[0] = 0;
            b.config.smooth_shadows = true;
            b.video.smooth_shadows = true;
            b.main_cpu.trace_cap = 10;
            b.sound.samples.push_back((123, 456));
            b.load_state(&saved).unwrap();
            assert!(b.config.netmerc_city_workaround);
            assert_eq!(b.tgp_cpu.netmerc_city_conversion, kind == Kind::NetMerc);
            assert_eq!(
                b.tgp_cpu.float_mode,
                if kind == Kind::NetMerc {
                    mb86233::FloatMode::Finite
                } else {
                    mb86233::FloatMode::Ieee
                }
            );
            assert!(b.sound.samples.is_empty());
            assert_eq!(b.save_state().unwrap(), saved);
            assert_eq!(b.ioboard.eeprom().data[3], 0x1234);
            assert_eq!(b.nvram[0], 0x5a);
            assert_eq!(b.frame_num, 17);
            assert!(b.config.smooth_shadows && b.video.smooth_shadows);
            assert_eq!(b.main_cpu.trace_cap, 10);
            a.sound.samples.clear();
            a.set_netmerc_city_workaround(true);
            for cycles in [1, 3, 64, 127, 999] {
                a.run_slice(cycles).unwrap();
                b.run_slice(cycles).unwrap();
                assert_eq!(a.save_state().unwrap(), b.save_state().unwrap());
                assert_eq!(a.sound.samples, b.sound.samples);
            }
        }
    }

    #[test]
    fn bad_header_sizes_checksum_and_payload_do_not_mutate_machine_or_audio() {
        let mut s = machine(&roms(crate::model1board::Kind::Original, false));
        s.sound.samples.push_back((12, 34));
        let before = s.save_state().unwrap();
        let samples = s.sound.samples.clone();
        for case in 0..9 {
            let mut bad = before.clone();
            match case {
                0 => bad.truncate(HEADER_BYTES - 1),
                1 => bad[0] ^= 1,
                2 => bad[8..12].copy_from_slice(&1u32.to_le_bytes()), // legacy layout
                3 => bad[12] ^= 1,
                4 => bad[32..40].copy_from_slice(&u64::MAX.to_le_bytes()),
                5 => bad[HEADER_BYTES + 100] ^= 1,
                6 => {
                    bad.push(0);
                    reseal(&mut bad);
                } // valid checksum, trailing payload
                7 => {
                    bad.truncate(HEADER_BYTES + 7);
                    reseal(&mut bad);
                }
                _ => {
                    // V60 starts the payload. Its empty trace Vec precedes the
                    // final lo/hi/cap fields (16 bytes); claim an impossible length.
                    let cpu_len = bincode::serialized_size(&s.main_cpu).unwrap() as usize;
                    let trace_len = HEADER_BYTES + cpu_len - 24;
                    bad.truncate(trace_len + 8);
                    bad[trace_len..].copy_from_slice(&u64::MAX.to_le_bytes());
                    reseal(&mut bad);
                }
            }
            assert!(s.load_state(&bad).is_err(), "case {case}");
            assert_eq!(s.save_state().unwrap(), before);
            assert_eq!(s.sound.samples, samples);
        }
        let oversized = vec![0; MAX_STATE_BYTES + 1];
        assert!(s.load_state(&oversized).is_err());
        assert_eq!(s.save_state().unwrap(), before);
        assert_eq!(s.sound.samples, samples);
    }

    #[test]
    fn late_audio_rejection_and_io_variant_rejection_are_machine_atomic() {
        let mut s = machine(&roms(crate::model1board::Kind::Original, false));
        s.sound.samples.push_back((12, 34));
        let before = s.save_state().unwrap();
        let samples = s.sound.samples.clone();
        // Early components are valid but deliberately different. Rejecting the
        // last component must not first install their changed RAM or EEPROM.
        let mut other = machine(&roms(crate::model1board::Kind::Original, false));
        other.nvram[0] = 0x99;
        other.ioboard.eeprom_mut().data[0] = 0x9876;
        let mut state = decode(&other.save_state().unwrap(), &s.resource_identity).unwrap();
        let mut sound = bincode::serialize(&state.sound).unwrap();
        sound[..4].copy_from_slice(&u32::MAX.to_le_bytes()); // unsupported sound version
        state.sound = bincode::deserialize(&sound).unwrap();
        let bad = encode(&state, &s.resource_identity).unwrap();
        assert!(s.load_state(&bad).unwrap_err().contains("sound"));
        assert_eq!(s.save_state().unwrap(), before);
        assert_eq!(s.sound.samples, samples);

        state.sound = other.sound.snapshot_model1().unwrap();
        let advanced = machine(&roms(crate::model1board::Kind::WingWar, false));
        state.io = advanced.ioboard.snapshot();
        let bad = encode(&state, &s.resource_identity).unwrap();
        assert!(s.load_state(&bad).is_err());
        assert_eq!(s.save_state().unwrap(), before);
        assert_eq!(s.sound.samples, samples);
    }

    #[test]
    fn identity_covers_resources_and_variants_but_not_operator_defaults() {
        use crate::model1board::Kind;
        let base = roms(Kind::Original, true);
        let identity = resource_identity(&base);
        for field in 0..15 {
            let mut changed = roms(Kind::Original, true);
            match field {
                0 => changed.maincpu.push(1),
                1 => changed.tgp.push(1),
                2 => changed.iocpu[0] ^= 1,
                3 => changed.sndcpu.push(1),
                4 => changed.mpcm1[0] ^= 1,
                5 => changed.mpcm2[0] ^= 1,
                6 => changed.copro_tables.push(1),
                7 => changed.polygons.push(1),
                8 => changed.copro_data.push(1),
                9 => changed.dsb.as_mut().unwrap().firmware[0] ^= 1,
                10 => changed.dsb.as_mut().unwrap().mpeg[0] ^= 1,
                11 => changed.dsb = None,
                12 => changed.ioboard_kind = Kind::WingWar,
                13 => changed.comm_board = false,
                _ => changed.netmerc_procedural_audio = true,
            }
            assert_ne!(resource_identity(&changed), identity, "resource {field}");
        }
        let mut defaults = roms(Kind::Original, true);
        defaults.ioboard_config = vec![0xff; 128];
        defaults.nvram_default = vec![0x5a; 0x10000];
        assert_eq!(resource_identity(&defaults), identity);
        let saved = machine(&base).save_state().unwrap();
        let mut other_roms = roms(Kind::Original, true);
        other_roms.maincpu[0] ^= 1;
        let mut other = Model1System::new(&other_roms).unwrap();
        assert!(other.load_state(&saved).unwrap_err().contains("ROM"));
    }

    #[test]
    fn comm_presence_refuses_save_and_load_even_before_link_connects() {
        let roms = roms(crate::model1board::Kind::Original, false);
        let saved = machine(&roms).save_state().unwrap();
        let mut linked = Model1System::with_config(
            &roms,
            Config {
                cabinet: crate::config::Cabinet::Twin,
                ..Config::default()
            },
        )
        .unwrap();
        linked.sound.samples.push_back((12, 34));
        let before = bincode::serialize(&linked.snapshot_motherboard().unwrap()).unwrap();
        let comm = linked.comm.as_ref().unwrap().snapshot();
        assert!(linked.save_state().unwrap_err().contains("COMM"));
        assert!(linked.load_state(&saved).unwrap_err().contains("COMM"));
        assert_eq!(linked.comm.as_ref().unwrap().snapshot(), comm);
        assert_eq!(
            bincode::serialize(&linked.snapshot_motherboard().unwrap()).unwrap(),
            before
        );
        assert_eq!(linked.sound.samples.back(), Some(&(12, 34)));
    }
}
