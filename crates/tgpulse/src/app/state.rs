//! Desktop storage/output boundary. Model 1 core snapshots stay memory-owned.
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};

impl Machine {
    pub(super) fn save_slot(&self, set: &str, slot: u32) -> Result<PathBuf, String> {
        match self {
            Self::Model2(sys) => savestate::save_to_file(sys, set, slot),
            Self::Model1(sys) => {
                let path = savestate::path_for(set, slot);
                save_model1(sys, &path)?;
                Ok(path)
            }
        }
    }

    fn load_slot(&mut self, set: &str, slot: u32) -> Result<PathBuf, String> {
        match self {
            Self::Model2(sys) => savestate::load_from_file(sys, set, slot),
            Self::Model1(sys) => {
                let path = savestate::path_for(set, slot);
                load_model1(sys, &path)?;
                Ok(path)
            }
        }
    }
}

impl Session {
    pub(super) fn load_slot(&mut self, slot: u32) -> Result<PathBuf, String> {
        let path = self.machine.load_slot(&self.set, slot)?;
        if let Machine::Model1(sys) = &self.machine {
            // Only after a successful atomic core load. The next normal redraw
            // regenerates software layers/GPU quads, including while paused.
            self.audio.clear();
            self.background.fill(0);
            self.foreground.fill(0);
            self.input.reset_model1_rumble();
            // A paused load must not re-arm the pad from a stale saved latch.
            // VR-family motor boards resume from the restored command on the
            // next emulated frame, not while the frontend remains paused.
            if tgpulse_core::model1_drive::DriveFamily::for_set(&self.set).is_none() {
                self.input.set_legacy_model1_rumble(sys.drive_cmd);
            }
            // Loading NVRAM into memory must not cause an immediate disk flush.
            // Normal periodic/close persistence remains the existing policy.
            self.nvram_countdown = NVRAM_FLUSH_INTERVAL;
        }
        Ok(path)
    }
}

fn load_model1(sys: &mut Model1System, path: &Path) -> Result<(), String> {
    let limit = tgpulse_core::model1::MAX_STATE_BYTES;
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err("Model 1 state must be a regular file no larger than 64 MiB".into());
    }
    // The file could grow after metadata: enforce the limit on the read too.
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Model 1 save state exceeds 64 MiB".into());
    }
    sys.load_state(&bytes)
}

fn save_model1(sys: &Model1System, path: &Path) -> Result<(), String> {
    // Serialize/check COMM before creating anything or replacing a valid slot.
    let bytes = sys.save_state()?;
    atomic_write(path, &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    // Own only a create_new sibling; never truncate a previous slot/temp file.
    let (tmp, mut file) = loop {
        let name = format!(
            ".tgpulse-state-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let tmp = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(file) => break (tmp, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    };
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|e: std::io::Error| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "tgpulse-state-test-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn system() -> Model1System {
        Model1System::new(&loader::Model1Roms {
            maincpu: vec![0],
            tgp: vec![],
            copro_tables: vec![],
            polygons: vec![],
            copro_data: vec![],
            iocpu: vec![0x76; 0x10000],
            sndcpu: vec![],
            mpcm1: vec![],
            mpcm2: vec![],
            ioboard_config: vec![],
            ioboard_kind: tgpulse_core::model1board::Kind::Original,
            comm_board: false,
            dsb: None,
            nvram_default: vec![],
        })
        .unwrap()
    }

    #[test]
    fn model1_slot_round_trip_overwrites_atomically_and_rejects_bad_files() {
        let tmp = Temp::new();
        let path = tmp.0.join("test.0.state");
        let mut sys = system();
        sys.nvram[0] = 19;
        save_model1(&sys, &path).unwrap();
        sys.nvram[0] = 20;
        save_model1(&sys, &path).unwrap();
        sys.nvram[0] = 33;
        load_model1(&mut sys, &path).unwrap();
        assert_eq!(sys.nvram[0], 20);
        let before = sys.save_state().unwrap();
        let oversized = tmp.0.join("large.state");
        File::create(&oversized)
            .unwrap()
            .set_len(tgpulse_core::model1::MAX_STATE_BYTES as u64 + 1)
            .unwrap();
        let bad = tmp.0.join("bad.state");
        fs::write(&bad, b"not a state").unwrap();
        for file in [&oversized, &bad, &tmp.0.join("missing.state"), &tmp.0] {
            assert!(load_model1(&mut sys, file).is_err());
            assert!(sys.save_state().unwrap() == before);
        }
        // Failure at rename keeps the existing destination, and cleans only
        // this operation's temporary file. No truncated/half-written slot.
        let directory = tmp.0.join("directory.state");
        fs::create_dir(&directory).unwrap();
        assert!(atomic_write(&directory, b"new data").is_err());
        assert!(directory.is_dir());
        assert!(fs::read_dir(&tmp.0).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
        assert!(fs::read(&path).unwrap() == before);
    }

    #[test]
    fn session_load_invalidates_outputs_only_after_success_and_leaves_nvram_disk_alone() {
        let tmp = Temp::new();
        // An absolute test set keeps the existing slot helper isolated without
        // changing process cwd or the user's states/NVRAM directories.
        let set = tmp.0.join("test").to_string_lossy().into_owned();
        let nvram_path = tmp.0.join("personal.nv");
        fs::write(&nvram_path, b"untouched").unwrap();
        let mut session = Session {
            nvram_file: Some(crate::settings::NvramFile(nvram_path.clone())),
            network: None,
            machine: Machine::Model1(Box::new(system())),
            set,
            title: "test".into(),
            input: InputState::new(),
            audio: Audio::silent(),
            scheme: ControlScheme::Joystick,
            nvram_countdown: 1,
            background: vec![123; SCREEN_W * SCREEN_H],
            foreground: vec![456; SCREEN_W * SCREEN_H],
        };
        let path = session.machine.save_slot(&session.set, 4).unwrap();
        assert_eq!(path, tmp.0.join("test.4.state"));
        session.audio.push([(123, 456)].into_iter());
        assert!(session.load_slot(5).is_err());
        assert_eq!(session.audio.queued_samples(), 1);
        assert_eq!(session.background[0], 123);
        assert_eq!(session.foreground[0], 456);
        assert_eq!(session.nvram_countdown, 1);
        session.load_slot(4).unwrap();
        assert_eq!(session.audio.queued_samples(), 0);
        assert!(session.background.iter().all(|&v| v == 0));
        assert!(session.foreground.iter().all(|&v| v == 0));
        assert_eq!(session.nvram_countdown, NVRAM_FLUSH_INTERVAL);
        assert_eq!(fs::read(nvram_path).unwrap(), b"untouched");
    }
}
