//! Runtime configuration for the machine. Nothing game-specific should be
//! hardcoded in the emulation itself; it comes through here.

/// Optional substitute sample banks for NetMerc; never a recovered original ROM.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetmercAudioDonor {
    #[default]
    Vf,
    Vr,
    Swa,
    Wingwar,
    Off,
}

impl NetmercAudioDonor {
    pub const ALL: [Self; 5] = [Self::Vf, Self::Vr, Self::Swa, Self::Wingwar, Self::Off];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vf => "vf",
            Self::Vr => "vr",
            Self::Swa => "swa",
            Self::Wingwar => "wingwar",
            Self::Off => "off",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Vf => "Virtua Fighter",
            Self::Vr => "Virtua Racing",
            Self::Swa => "Star Wars Arcade",
            Self::Wingwar => "Wing War",
            Self::Off => "Off",
        }
    }
}

impl std::str::FromStr for NetmercAudioDonor {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| {
                format!("invalid NetMerc audio donor '{value}' (want vf, vr, swa, wingwar or off)")
            })
    }
}

/// Manual widescreen override, or the game's persisted cabinet/monitor choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Widescreen {
    #[default]
    Off,
    On,
    Auto,
}

impl Widescreen {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Auto => "auto",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Auto,
            Self::Auto => Self::Off,
        }
    }
}

impl std::str::FromStr for Widescreen {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "off" | "false" | "0" | "no" => Ok(Self::Off),
            "on" | "true" | "1" | "yes" => Ok(Self::On),
            "auto" => Ok(Self::Auto),
            _ => Err(format!(
                "invalid widescreen mode '{value}' (expected off, on or auto)"
            )),
        }
    }
}

/// Whether a supported Model 1/2 COMM network board is fitted. This is a physical
/// property of the cabinet, not a game setting, so it cannot be derived from
/// the ROMs and has to be told to us.
///
/// Model 1 uses this only for board presence; operator roles stay in NVRAM
/// and the desktop frontend starts TCP when fitted. The following legacy
/// boot/role behavior describes Model 2, not a Model 1 role override.
///
/// The game's boot code copes with either, and takes a visibly different branch
/// for each:
///
/// * fitted -- it clears `cn`, reads back a 0 in bit 0, and runs the ring
///   handshake, showing NETWORK CHECKING until the link comes up.
/// * absent -- the slot floats, bit 0 reads back 1, and it prints
///   NETWORK BOARD NOT PRESENT / CANCELLED, waits its usual 320 frames and
///   carries on as a lone node.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cabinet {
    /// Board fitted. Model 1 retains the game's NVRAM role. Model 2 sets the
    /// game to run as the link master. With one
    /// cabinet the ring cable closes back on this board, so it links to itself
    /// as a single-node ring: link id 1 of 1.
    Twin,
    /// No board fitted. On Model 2 the game is also set to standalone, as a single cabinet
    /// ships. The boot code then skips the network check entirely.
    Single,
}

impl Cabinet {
    /// The link role the game keeps in its battery-backed settings, which is
    /// what actually decides whether the network check runs at all. Recovered
    /// from the boot code at 0x1200, which reads it and skips straight past the
    /// check when it is zero, and from the string table at 0x1b948.
    pub fn link_role(self) -> u8 {
        match self {
            Cabinet::Single => 0, // standalone: no check
            Cabinet::Twin => 1,   // master (2 would be slave)
        }
    }
}

impl std::str::FromStr for Cabinet {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "twin" | "link" | "linked" => Ok(Cabinet::Twin),
            "single" | "standalone" => Ok(Cabinet::Single),
            other => Err(format!(
                "unknown cabinet mode '{}' (want twin|single)",
                other
            )),
        }
    }
}

/// Live control state, in the same encoding the Model 1 I/O board reports to
/// the game. All the digital lines are active-low, so a set bit means "not
/// pressed"; the gearbox bits inside IN1 are the exception and are active-high.
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Inputs {
    /// IN0: b0 coin1, b1 coin2, b2 test, b3 service, b4 start1, b5-7 VR1-3.
    pub in0: u8,
    /// IN1: b0 VR4, b4-6 gearbox (active high), rest unused.
    pub in1: u8,
    /// IN2: unused on Daytona.
    pub in2: u8,
    pub steer: u8,
    pub accel: u8,
    pub brake: u8,
    /// Player-1 lightgun position, 10-bit each, for gun games (Virtua Cop).
    /// Centre is the game's default aim; off-screen is signalled separately.
    pub gun_x: u16,
    pub gun_y: u16,
    /// Player-1 gun pointed off-screen (trigger reload). The I/O board reports
    /// this in its own byte rather than as an extreme coordinate.
    pub gun_offscreen: bool,
    /// Independent P2 coordinates and off-screen detector, in cabinet ADC units.
    pub gun2_x: u16,
    pub gun2_y: u16,
    pub gun2_offscreen: bool,
    /// The 315-5649's eight auto-incrementing analog channels (2A/2B/2C).
    /// Racing cabinets wire the first three to wheel/accelerator/brake, but
    /// Wave Runner's jet-ski uses four: handle, roll, throttle, pitch.
    pub analog: [u8; 8],
    /// DIP switches on the I/O board. Daytona leaves all of them unused.
    pub dsw: [u8; 3],
}

/// Gearbox encoding from the reference: neutral, 1st..4th.
pub const GEAR_VALUES: [u8; 5] = [0, 2, 1, 6, 5];

impl Default for Inputs {
    fn default() -> Self {
        Self {
            // Nothing pressed.
            in0: 0xFF,
            // Active-low bits idle high, gearbox (0x70) idle at neutral (0).
            in1: !0x70 | (GEAR_VALUES[0] << 4),
            in2: 0xFF,
            // The reference port default defaults for daytona.
            steer: 0x80,
            accel: 0x20,
            brake: 0x20,
            // Virtua Cop's port default lightgun centres (P1_X 0x17c, P1_Y 0x0e6).
            gun_x: 0x17c,
            gun_y: 0x0e6,
            gun_offscreen: false,
            gun2_x: 0x179,
            gun2_y: 0x0e8,
            gun2_offscreen: false,
            // Unwired channels read back as all-ones on a real I/O board.
            analog: [0x80, 0x20, 0x20, 0x80, 0xFF, 0xFF, 0xFF, 0xFF],
            dsw: [0xFF; 3],
        }
    }
}

impl Inputs {
    /// Selects a gear, 0 = neutral through 4 = 4th.
    pub fn set_gear(&mut self, gear: usize) {
        let g = GEAR_VALUES[gear.min(4)];
        self.in1 = (self.in1 & !0x70) | (g << 4);
    }
}

/// Which Sega arcade board a ROM set is for. They share the TGP coprocessor,
/// the Model 1 sound board and the segas24 tilemap, but the main CPU differs:
/// Model 1 is a NEC V60, Model 2 an i960.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum System {
    Model1,
    Model2,
}

impl System {
    /// Guesses the board from the ROM archive's contents. Model 1 sets carry
    /// the TGP program in `*-tgp.bin` / `315-557x.bin`; Model 2 sets do not,
    /// and their main program interleaves `epr-16722a`-style chips. The
    /// filenames are the cheapest reliable signal.
    pub fn detect(rom_path: &str) -> System {
        let names = crate::loader::archive_names(rom_path).unwrap_or_default();
        // The ROM database identifies the game from its files and knows which
        // board it is; fall back to Model 2 for an unrecognised set.
        match crate::roms_db::identify(&names) {
            Some(def) if def.board.is_model1() => System::Model1,
            _ => System::Model2,
        }
    }
}

/// Output-only preferences: never stop chip clocks, voices or sound CPUs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioMutes {
    pub multipcm1: bool,
    pub multipcm2: bool,
    pub ym3438: bool,
    pub dsb: bool,
    pub scsp: bool,
}

/// Absolute output-route gains in percent: 50 means gain 0.5, not half of
/// the reference gain. Frontend preferences, not emulated chip state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioGains {
    pub multipcm1: u32,
    pub multipcm2: u32,
    pub ym3438: u32,
    pub dsb: u32,
    pub scsp: u32,
}

impl AudioGains {
    pub const MAX: u32 = 100;
    pub const REFERENCE: Self = Self {
        multipcm1: 50,
        multipcm2: 50,
        ym3438: 30,
        dsb: 100,
        scsp: 100,
    };
    /// Alternative donor mixer, not original NetMerc hardware levels.
    pub const NETMERC_DONOR_ALTERNATIVE: Self = Self {
        multipcm1: 38,
        multipcm2: 38,
        ..Self::REFERENCE
    };
    pub fn clamped(self) -> Self {
        Self {
            multipcm1: self.multipcm1.min(Self::MAX),
            multipcm2: self.multipcm2.min(Self::MAX),
            ym3438: self.ym3438.min(Self::MAX),
            dsb: self.dsb.min(Self::MAX),
            scsp: self.scsp.min(Self::MAX),
        }
    }
}

impl Default for AudioGains {
    fn default() -> Self {
        Self::REFERENCE
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub rom_path: String,
    /// Directory the ROM library is read from.
    pub rom_dir: std::path::PathBuf,
    /// Start with the window filling the screen.
    pub fullscreen: bool,
    /// Frontend-only correction of display RGB sampling into an sRGB surface.
    /// Off preserves the previous presentation; never changes emulated pixels.
    pub srgb: bool,
    pub system: System,
    pub cabinet: Cabinet,
    /// Supersampling factor for the 3D rasterizer.
    ///
    /// The board draws 3D with no antialiasing at all, so polygon edges are
    /// hard steps. Rasterizing on an NxN denser grid and averaging each block
    /// back to one board pixel resolves those edges while leaving the tile
    /// layers -- HUD text, the whole 2D image -- untouched at native
    /// resolution. 1 is the hardware's own output, pixel for pixel.
    pub ssaa: u32,

    /// Forward the drive board's force commands to the pad's rumble motors.
    ///
    /// Off by default: the cabinet's wheel motor applies force continuously
    /// while driving, which is correct for a torque motor you are holding, but
    /// translated to a pad it means near-constant buzzing.
    pub rumble: bool,

    /// Host pad output intensity, 0..100%; 100 preserves the decoder's levels.
    /// Does not change emulated drive-board commands or machine snapshots.
    pub rumble_intensity: u32,

    /// Blend the stippled-transparency quads instead of dithering them.
    ///
    /// The board has no alpha channel: "transparent" shadows are drawn as a
    /// checkerboard stipple that covers every second pixel (Model 1's MOIRE
    /// quad flag, Model 2's stipple mode), and translucent *textures* (car
    /// windows, water) as a per-texel coverage that thresholds to keep or
    /// discard. With this on, stippled quads blend 50/50 with the pixels
    /// behind them and texture coverage becomes a blend weight -- the
    /// perceptual equivalent of the hardware dither, without the dithering.
    /// Off keeps the hardware-exact behaviour.
    pub smooth_shadows: bool,

    /// Output volume as a percentage (100 = the board's own level).
    ///
    /// Some boards mix quiet: the SCSP games (Sega Rally) peak at ~4% of
    /// full scale because the cabinet's amplifier did the rest. This is a
    /// plain digital gain on the mixed output, clamped against clipping.
    pub volume: u32,
    pub audio_mutes: AudioMutes,
    pub audio_gains: AudioGains,
    /// Resource selection applied by the frontend before constructing NetMerc.
    pub netmerc_audio_donor: NetmercAudioDonor,
    /// Frontend output preset; the caller supplies actual donor availability.
    pub netmerc_alternative_gains: bool,
    /// Optional firmware-specific rounding override; live, NetMerc only.
    pub netmerc_city_workaround: bool,

    /// Off: legacy native framing. On: widen the 3D field of view.
    /// Auto: present the native image at the saved cabinet's 4:3/16:9 aspect.
    ///
    /// The render target widens from
    /// 496x384 to 683x384 and the polygon viewport/frustum widens around its
    /// center by the same factor, so more of the scene is visible on the
    /// sides instead of the 4:3 image being stretched. The 2D tile layers
    /// (sky, HUD) stretch to fill, Off is
    /// the hardware's own framing.
    pub widescreen: Widescreen,

    /// Stretch the 2D tile layers to fill the widened frame.
    ///
    /// On, the sky/background tilemap stretches
    /// across the extra width; off, the 2D layers stay at native width
    /// centered in the frame (only the 3D widens). Only meaningful with
    /// --widescreen on.
    pub widescreen_stretch_2d: bool,

    /// Rotate the display 180 degrees from the usual landscape.
    ///
    /// A controller cradle (GameSir and the like) can hold the phone with its
    /// charging port on the other side, which leaves the ordinary landscape
    /// upside down in the hand. Android only: a desktop window is never
    /// rotated, so the value is carried but ignored there.
    pub reverse_landscape: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rom_path: String::new(),
            rom_dir: std::path::PathBuf::from(crate::library::DEFAULT_DIR),
            fullscreen: false,
            srgb: false,
            system: System::Model2,
            // A standalone cabinet is what anyone running one machine wants,
            // and it is the only setting that reaches the game without a link
            // partner on the other end of the ring.
            cabinet: Cabinet::Single,
            ssaa: 2,
            rumble: false,
            rumble_intensity: 100,
            // The blended stipple reads the way the hardware's checkerboard
            // did on a CRT; the exact dither is one flag away for purists.
            smooth_shadows: true,
            volume: 100,
            audio_mutes: AudioMutes::default(),
            audio_gains: AudioGains::default(),
            netmerc_audio_donor: NetmercAudioDonor::default(),
            netmerc_alternative_gains: false,
            netmerc_city_workaround: true,
            widescreen: Widescreen::Off,
            widescreen_stretch_2d: true,
            reverse_landscape: false,
        }
    }
}

impl Config {
    /// Resolve output preferences without changing stored manual gains or state.
    /// Recovery retains its internal 0.105 normalization and 50% route scaling.
    pub fn effective_audio_gains(&self, game: &str, donor_active: bool) -> AudioGains {
        if self.netmerc_alternative_gains && game == "netmerc" {
            if donor_active {
                AudioGains::NETMERC_DONOR_ALTERNATIVE
            } else {
                AudioGains::REFERENCE
            }
        } else {
            self.audio_gains
        }
    }

    /// A short label for the loaded game, used in the window title.
    pub fn title(&self) -> String {
        let stem = std::path::Path::new(&self.rom_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let board = match self.system {
            System::Model1 => "Model 1",
            System::Model2 => "Model 2",
        };
        format!("{stem} ({board})")
    }
}

#[cfg(test)]
mod audio_gain_tests {
    use super::*;

    #[test]
    fn netmerc_alternative_preset_uses_actual_donor_without_changing_manual_gains() {
        let mut config = Config::default();
        config.audio_gains.multipcm1 = 15;
        config.audio_gains.multipcm2 = 100;
        let manual = config.audio_gains;
        assert_eq!(config.effective_audio_gains("netmerc", true), manual);
        config.netmerc_alternative_gains = true;
        assert_eq!(
            config.effective_audio_gains("netmerc", true),
            AudioGains::NETMERC_DONOR_ALTERNATIVE
        );
        // A selected donor may be unavailable: use the running fallback path.
        assert_eq!(
            config.effective_audio_gains("netmerc", false),
            AudioGains::REFERENCE
        );
        for game in ["vr", "swa", "vf", "daytona"] {
            assert_eq!(config.effective_audio_gains(game, true), manual);
        }
        config.netmerc_alternative_gains = false;
        assert_eq!(config.effective_audio_gains("netmerc", true), manual);
        assert_eq!(config.audio_gains, manual);
    }
}
