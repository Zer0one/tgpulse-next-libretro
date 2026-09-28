//! The player's adjustments, remembered between runs.
//!
//! Everything the settings window touches is written to `config/settings.conf`
//! the moment it changes and read back at startup, so the emulator comes back
//! the way it was left. A command-line flag overrides the file for that run
//! without rewriting it, and the debugger never reads it, so scripts and
//! captures stay reproducible.
//!
//! The file is plain `key = value` lines, written out in full on first run so
//! it is discoverable and hand-editable. A line the parser does not understand
//! is warned about and skipped; a setting the file does not mention keeps its
//! shipped value, so a file written by an older build still works.

use std::path::{Path, PathBuf};

use tgpulse_core::config::{AudioGains, AudioMutes, Cabinet, Config, Widescreen};

/// The adjustable subset of `Config` that is worth remembering between runs.
///
/// Fullscreen is persisted along with the other user preferences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub nvram: Option<PathBuf>,
    pub network: crate::network::Config,
    pub ssaa: u32,
    pub fullscreen: bool,
    pub srgb: bool,
    pub widescreen: Widescreen,
    pub widescreen_stretch_2d: bool,
    pub smooth_shadows: bool,
    pub volume: u32,
    pub audio_mutes: AudioMutes,
    pub audio_gains: AudioGains,
    pub rumble: bool,
    pub cabinet: Cabinet,
    pub reverse_landscape: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

impl Settings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            nvram: None,
            network: crate::network::Config::default(),
            ssaa: config.ssaa,
            fullscreen: config.fullscreen,
            srgb: config.srgb,
            widescreen: config.widescreen,
            widescreen_stretch_2d: config.widescreen_stretch_2d,
            smooth_shadows: config.smooth_shadows,
            volume: config.volume,
            audio_mutes: config.audio_mutes,
            audio_gains: config.audio_gains,
            rumble: config.rumble,
            cabinet: config.cabinet,
            reverse_landscape: config.reverse_landscape,
        }
    }

    pub fn apply_to(&self, config: &mut Config) {
        config.ssaa = self.ssaa;
        config.fullscreen = self.fullscreen;
        config.srgb = self.srgb;
        config.widescreen = self.widescreen;
        config.widescreen_stretch_2d = self.widescreen_stretch_2d;
        config.smooth_shadows = self.smooth_shadows;
        config.volume = self.volume;
        config.audio_mutes = self.audio_mutes;
        config.audio_gains = self.audio_gains.clamped();
        config.rumble = self.rumble;
        config.cabinet = self.cabinet;
        config.reverse_landscape = self.reverse_landscape;
    }

    pub fn path() -> PathBuf {
        PathBuf::from("config").join("settings.conf")
    }

    /// Reads the file, writing the defaults out first if there is none yet, so
    /// the format is discoverable without having to change something in the
    /// interface to make one appear.
    pub fn load_or_create(path: &Path) -> Self {
        if !path.exists() {
            let defaults = Self::default();
            if let Err(e) = defaults.save(path) {
                log::warn!(target: "settings", "cannot write {}: {e}", path.display());
            }
            return defaults;
        }
        Self::load(path)
    }

    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut settings = Self::default();
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                log::warn!(target: "settings", "{}:{}: not a setting", path.display(), number + 1);
                continue;
            };
            let (name, value) = (name.trim(), value.trim());
            let boolean = |v: &str| match v {
                "on" | "true" | "1" => Some(true),
                "off" | "false" | "0" => Some(false),
                _ => None,
            };
            match name {
                "nvram" => settings.nvram = (!value.is_empty()).then(|| PathBuf::from(value)),
                "model1_address_in" => settings.network.address_in = value.into(),
                "model1_address_out" => settings.network.address_out = value.into(),
                "model1_port_in" | "model1_port_out" => match value.parse::<u16>() {
                    Ok(port) if port != 0 => {
                        if name == "model1_port_in" {
                            settings.network.port_in = port;
                        } else {
                            settings.network.port_out = port;
                        }
                    }
                    _ => log::warn!(target: "settings", "bad {name} '{value}' (want 1..65535)"),
                },
                "ssaa" => match value.parse::<u32>().ok().filter(|n| (1..=4).contains(n)) {
                    Some(n) => settings.ssaa = n,
                    None => {
                        log::warn!(target: "settings", "{}:{}: bad ssaa '{value}' (want 1..4)", path.display(), number + 1)
                    }
                },
                "widescreen" => settings.widescreen = value.parse().unwrap_or(settings.widescreen),
                "widescreen_stretch_2d" => {
                    settings.widescreen_stretch_2d =
                        boolean(value).unwrap_or(settings.widescreen_stretch_2d)
                }
                "smooth_shadows" => {
                    settings.smooth_shadows = boolean(value).unwrap_or(settings.smooth_shadows)
                }
                "fullscreen" => settings.fullscreen = boolean(value).unwrap_or(settings.fullscreen),
                "srgb" => settings.srgb = boolean(value).unwrap_or(settings.srgb),
                "volume" => match value.parse::<u32>() {
                    Ok(n) => settings.volume = n,
                    Err(_) => {
                        log::warn!(target: "settings", "{}:{}: bad volume '{value}'", path.display(), number + 1)
                    }
                },
                "rumble" => settings.rumble = boolean(value).unwrap_or(settings.rumble),
                "gain_multipcm1" | "gain_multipcm2" | "gain_ym3438" | "gain_dsb" | "gain_scsp" => {
                    if let Some(gain) = value.parse::<u32>().ok().filter(|v| *v <= AudioGains::MAX)
                    {
                        let target = match name {
                            "gain_multipcm1" => &mut settings.audio_gains.multipcm1,
                            "gain_multipcm2" => &mut settings.audio_gains.multipcm2,
                            "gain_ym3438" => &mut settings.audio_gains.ym3438,
                            "gain_dsb" => &mut settings.audio_gains.dsb,
                            _ => &mut settings.audio_gains.scsp,
                        };
                        *target = gain;
                    } else {
                        log::warn!(target: "settings", "{}:{}: bad {name} '{value}' (want 0..{})", path.display(), number + 1, AudioGains::MAX);
                    }
                }
                "mute_multipcm1" => {
                    settings.audio_mutes.multipcm1 =
                        boolean(value).unwrap_or(settings.audio_mutes.multipcm1)
                }
                "mute_multipcm2" => {
                    settings.audio_mutes.multipcm2 =
                        boolean(value).unwrap_or(settings.audio_mutes.multipcm2)
                }
                "mute_scsp" => {
                    settings.audio_mutes.scsp = boolean(value).unwrap_or(settings.audio_mutes.scsp)
                }
                "mute_ym3438" => {
                    settings.audio_mutes.ym3438 =
                        boolean(value).unwrap_or(settings.audio_mutes.ym3438)
                }
                "mute_dsb" => {
                    settings.audio_mutes.dsb = boolean(value).unwrap_or(settings.audio_mutes.dsb)
                }
                "reverse_landscape" => {
                    settings.reverse_landscape =
                        boolean(value).unwrap_or(settings.reverse_landscape)
                }
                "cabinet" => match value.parse::<Cabinet>() {
                    Ok(c) => settings.cabinet = c,
                    Err(_) => {
                        log::warn!(target: "settings", "{}:{}: unknown cabinet '{value}'", path.display(), number + 1)
                    }
                },
                other => {
                    log::warn!(target: "settings", "{}:{}: unknown setting '{other}'", path.display(), number + 1)
                }
            }
        }
        log::info!(target: "settings", "settings from {}", path.display());
        settings
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let on_off = |b: bool| if b { "on" } else { "off" };
        let cabinet = match self.cabinet {
            Cabinet::Twin => "twin",
            Cabinet::Single => "single",
        };
        let out = format!(
            "# TGPulse settings.\n\
             #\n\
             # Written when a setting changes in the interface; the command line\n\
             # overrides any of these for one run without rewriting the file.\n\
             # Delete a line to go back to the shipped value.\n\
             \n\
             # Optional explicit NVRAM file, read and written; relative to launch cwd.\n\
             nvram = {}\n\
             ssaa = {}\n\
             fullscreen = {}\n\
             # Correct framebuffer sRGB presentation; off preserves the legacy look.\n\
             srgb = {}\n\
             widescreen = {}\n\
             widescreen_stretch_2d = {}\n\
             smooth_shadows = {}\n\
             volume = {}\n\
             # Absolute route gain in percent (50 = 0.5), range 0..100.\n\
             gain_multipcm1 = {}\n\
             gain_multipcm2 = {}\n\
             gain_ym3438 = {}\n\
             gain_dsb = {}\n\
             gain_scsp = {}\n\
             # Output mutes only; chip emulation continues.\n\
             mute_multipcm1 = {}\n\
             mute_multipcm2 = {}\n\
             mute_ym3438 = {}\n\
             mute_dsb = {}\n\
             mute_scsp = {}\n\
             rumble = {}\n\
             # Model 1/2 COMM presence: single=absent, twin=fitted where supported.\n\
             # Model 1 twin also enables TCP; roles remain in NVRAM.\n\
             cabinet = {}\n\
             reverse_landscape = {}\n\
             # Model 1 TCP ring; applies on game load/reset. Numeric IP addresses.\n\
             # Configure cabinet roles in the game's own test menu.\n\
             model1_address_in = {}\n\
             model1_port_in = {}\n\
             model1_address_out = {}\n\
             model1_port_out = {}\n",
            self.nvram
                .as_ref()
                .map_or_else(String::new, |p| p.to_string_lossy().into_owned()),
            self.ssaa,
            on_off(self.fullscreen),
            on_off(self.srgb),
            self.widescreen.as_str(),
            on_off(self.widescreen_stretch_2d),
            on_off(self.smooth_shadows),
            self.volume,
            self.audio_gains.multipcm1,
            self.audio_gains.multipcm2,
            self.audio_gains.ym3438,
            self.audio_gains.dsb,
            self.audio_gains.scsp,
            on_off(self.audio_mutes.multipcm1),
            on_off(self.audio_mutes.multipcm2),
            on_off(self.audio_mutes.ym3438),
            on_off(self.audio_mutes.dsb),
            on_off(self.audio_mutes.scsp),
            on_off(self.rumble),
            cabinet,
            on_off(self.reverse_landscape),
            self.network.address_in,
            self.network.port_in,
            self.network.address_out,
            self.network.port_out,
        );
        std::fs::write(path, out).map_err(|e| e.to_string())
    }
}

/// Desktop profile provenance. Explicit paths are relative to the invocation
/// directory, while default paths stay relative to the existing runtime cwd.
pub struct Profile {
    pub path: PathBuf,
    pub settings: Settings,
    pub launch_dir: PathBuf,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            path: Settings::path(),
            settings: Settings::default(),
            launch_dir: PathBuf::from("."),
        }
    }
}
impl Profile {
    pub fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_owned()
        } else {
            self.launch_dir.join(path)
        }
    }
    pub fn nvram_file(&self) -> Option<NvramFile> {
        self.settings
            .nvram
            .as_ref()
            .map(|p| NvramFile(self.resolve(p)))
    }
}

#[derive(Clone)]
pub struct NvramFile(pub PathBuf);
impl NvramFile {
    pub fn load(&self, backup_len: usize, eeprom_len: usize) -> Result<(Vec<u8>, Vec<u8>), String> {
        let blob = std::fs::read(&self.0)
            .map_err(|e| format!("cannot read NVRAM {}: {e}", self.0.display()))?;
        tgpulse_core::nvram::decode(&blob, backup_len, eeprom_len)
            .ok_or_else(|| format!("invalid/incompatible NVRAM: {}", self.0.display()))
    }
    pub fn save(&self, backup: &[u8], eeprom: &[u8]) -> Result<(), String> {
        std::fs::write(&self.0, tgpulse_core::nvram::encode(backup, eeprom))
            .map_err(|e| format!("cannot write NVRAM {}: {e}", self.0.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_nvram_reads_and_writes_only_its_selected_file() {
        let dir =
            std::env::temp_dir().join(format!("tgpulse-nvram-profile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let master = NvramFile(dir.join("master.nv"));
        let slave = NvramFile(dir.join("slave.nv"));
        assert!(master.load(4, 2).is_err());
        master.save(&[1; 4], &[2; 2]).unwrap();
        slave.save(&[3; 4], &[4; 2]).unwrap();
        assert_eq!(master.load(4, 2).unwrap(), (vec![1; 4], vec![2; 2]));
        master.save(&[5; 4], &[6; 2]).unwrap();
        assert_eq!(master.load(4, 2).unwrap(), (vec![5; 4], vec![6; 2]));
        assert_eq!(slave.load(4, 2).unwrap(), (vec![3; 4], vec![4; 2]));
        assert!(master.load(8, 2).is_err());
        std::fs::write(&master.0, b"invalid").unwrap();
        assert!(master.load(4, 2).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saved_file_loads_back_identically() {
        let dir = std::env::temp_dir().join(format!("tgpulse-settings-{}", std::process::id()));
        let path = dir.join("settings.conf");
        let settings = Settings {
            nvram: Some(PathBuf::from("nvram/master.nv")),
            network: crate::network::Config {
                address_in: "0.0.0.0".into(),
                address_out: "192.0.2.1".into(),
                port_in: 25000,
                port_out: 25001,
            },
            ssaa: 4,
            fullscreen: true,
            srgb: true,
            widescreen: Widescreen::Auto,
            smooth_shadows: false,
            volume: 400,
            audio_gains: AudioGains {
                multipcm1: 80,
                multipcm2: 0,
                ym3438: 45,
                dsb: 25,
                scsp: 100,
            },
            audio_mutes: AudioMutes {
                multipcm1: true,
                multipcm2: true,
                ym3438: true,
                dsb: true,
                scsp: true,
            },
            cabinet: Cabinet::Twin,
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert!(!std::fs::read_to_string(&path)
            .unwrap()
            .contains("model1_network"));
        assert_eq!(Settings::load(&path), settings);
        for mode in [Widescreen::Off, Widescreen::On, Widescreen::Auto] {
            let mut settings = settings.clone();
            settings.widescreen = mode;
            settings.save(&path).unwrap();
            assert_eq!(Settings::load(&path), settings);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_partial_file_keeps_shipped_values_for_the_rest() {
        let dir =
            std::env::temp_dir().join(format!("tgpulse-settings-partial-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.conf");
        std::fs::write(&path, "volume = 250\nunknown_thing = yes\n").unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.volume, 250);
        assert_eq!(settings.ssaa, Settings::default().ssaa);
        assert_eq!(settings.cabinet, Settings::default().cabinet);
        assert_eq!(settings.audio_mutes, AudioMutes::default());
        assert_eq!(settings.audio_gains, AudioGains::REFERENCE);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_gains_keep_reference_and_mute_does_not_reset_gain() {
        let path = std::env::temp_dir().join(format!("tgpulse-gains-{}.conf", std::process::id()));
        std::fs::write(&path, "gain_multipcm1 = -1\ngain_multipcm2 = 101\ngain_ym3438 = NaN\ngain_dsb = 25\nmute_dsb = on\ngain_scsp = 0\n").unwrap();
        let settings = Settings::load(&path);
        assert_eq!(
            settings.audio_gains,
            AudioGains {
                dsb: 25,
                scsp: 0,
                ..AudioGains::REFERENCE
            }
        );
        assert!(settings.audio_mutes.dsb);
        let mut config = Config::default();
        settings.apply_to(&mut config);
        assert_eq!(Settings::from_config(&config), settings);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn apply_and_from_config_round_trip() {
        let mut config = Config::default();
        Settings {
            volume: 300,
            srgb: true,
            audio_mutes: AudioMutes {
                multipcm1: true,
                multipcm2: false,
                ym3438: true,
                dsb: true,
                scsp: true,
            },
            ..Settings::default()
        }
        .apply_to(&mut config);
        assert_eq!(config.volume, 300);
        assert_eq!(
            Settings::from_config(&config),
            Settings {
                volume: 300,
                srgb: true,
                audio_mutes: AudioMutes {
                    multipcm1: true,
                    multipcm2: false,
                    ym3438: true,
                    dsb: true,
                    scsp: true
                },
                ..Settings::default()
            }
        );
    }
}
