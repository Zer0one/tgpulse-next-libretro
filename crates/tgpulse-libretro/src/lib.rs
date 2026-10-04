//! Experimental Sega Model 1 Libretro frontend. Host callbacks and content
//! paths stay here; `tgpulse-core` remains independent of Libretro.

mod ffi;
mod diagnostic_overlay;
mod gpu;
mod model1_controls;
mod mvd;
mod sensors;
mod sensor_trace;
#[allow(dead_code)] // Shared neutral helper also exposes standalone diagnostics.
#[path = "../../tgpulse/src/input/motion.rs"]
mod motion;
mod netpacket;
mod nvram;
mod persistence;
mod steering_response;
mod timing_overlay;
// This frontend-only pad policy is shared with the standalone; the emulated
// drive command decoder remains in tgpulse-core.
#[path = "../../tgpulse/src/input/model1_rumble.rs"]
mod model1_rumble;

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use model1_controls::{FlightCabinet, FlightKind, VfCabinet, VfPlayer, VrCabinet};
use model1_rumble::Model1PadRumble;
use tgpulse_core::config::{AudioGains, AudioMutes, Config, Inputs, NetmercAudioDonor, System};
use tgpulse_core::loader;
use tgpulse_core::model1::{Model1System, CPU_HZ, CYCLES_PER_FRAME};
use tgpulse_core::sound::AudioSource;
use tgpulse_core::{model1_video, roms_db, tilemap};

const WIDTH: usize = tilemap::SCREEN_W;
const HEIGHT: usize = tilemap::SCREEN_H;
const FPS: f64 = CPU_HZ as f64 / CYCLES_PER_FRAME as f64;
const DEFAULT_AUDIO_RATE: f64 = 10_000_000.0 / 224.0;
const FAST_AUDIO_DENOMINATOR: u64 = 224 * 60;
const ASPECT_4_3: f32 = 4.0 / 3.0;
const ASPECT_16_9: f32 = 16.0 / 9.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AspectRatio {
    #[default]
    Auto,
    FourThree,
    SixteenNine,
}

#[derive(Clone, Copy)]
struct Settings {
    smooth_shadows: bool,
    netmerc_city_workaround: bool,
    volume: u32,
    audio_gains: AudioGains,
    audio_mutes: AudioMutes,
    apply_known_rom_repairs: bool,
    fast_60hz: bool,
    aspect_ratio: AspectRatio,
    rumble: bool,
    overlay_font: u32,
    diagnostic: diagnostic_overlay::Settings,
    driving_ranges: [u32; 3],
    steering_response: steering_response::Response,
    mvd: mvd::Settings,
    automatic_nvram: bool,
    nvram_settings: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let config = Config::default();
        Self {
            smooth_shadows: config.smooth_shadows,
            netmerc_city_workaround: config.netmerc_city_workaround,
            volume: config.volume,
            audio_gains: config.audio_gains,
            audio_mutes: config.audio_mutes,
            apply_known_rom_repairs: true,
            fast_60hz: false,
            aspect_ratio: AspectRatio::Auto,
            rumble: true,
            overlay_font: 0,
            diagnostic: diagnostic_overlay::Settings::default(),
            driving_ranges: [100; 3],
            steering_response: steering_response::Response::Linear,
            mvd: mvd::Settings::default(),
            automatic_nvram: true,
            nvram_settings: false,
        }
    }
}

#[derive(Default)]
struct Core {
    environment: Option<ffi::Environment>,
    video: Option<ffi::Video>,
    audio_sample: Option<ffi::AudioSample>,
    audio_batch: Option<ffi::AudioBatch>,
    input_poll: Option<ffi::InputPoll>,
    input_state: Option<ffi::InputState>,
    settings: Settings,
    game: Option<Game>,
    devices: [u32; 2],
    gpu: Option<gpu::Renderer>,
    hw_callback: Option<Box<gpu::HwCallback>>,
    hardware_requested: bool,
    hardware_error: Option<String>,
    save_ram: Box<[u8]>,
    save_imported: bool,
    nvram_display_enabled: bool,
}

struct Game {
    graphics: gpu::Settings,
    hardware: bool,
    set_name: String,
    profile: Profile,
    controllers: Box<ControllerStorage>,
    machine: Box<Model1System>,
    initial_state: Vec<u8>,
    linked_reset: Option<(loader::Model1Roms, Config)>,
    link_status: Option<[u8; 4]>,
    linked_online_frames: u32,
    factory_nvram_loaded: bool,
    audio_notice: Option<String>,
    nvram_pending: bool,
    nvram_wait_frames: u32,
    pixels: Vec<u32>,
    foreground: Vec<u32>,
    panel_foreground: Vec<u32>,
    audio: Vec<i16>,
    steering: i32,
    accelerator: i32,
    brake: i32,
    fast_60hz: bool,
    audio_remainder: u64,
    output_audio: Vec<i16>,
    reported_aspect_ratio: Option<f32>,
    reported_width: u32,
    rumble: PadRumble,
    timing: timing_overlay::Timing,
    panel: timing_overlay::Panel,
    diagnostic: diagnostic_overlay::Display,
    mvd_commands: mvd::Commands,
    mvd_sensors: sensors::Sensors,
    mvd_holder_notice: mvd::HolderNotice,
}

impl Game {
    fn publish_mvd_holder(&mut self, environment: Option<ffi::Environment>) {
        if let Some(message) = self.mvd_holder_notice.update(self.machine.mvd_holder_latched()) {
            notify_frames(environment, message, 180);
        }
    }
}

struct PadRumble {
    set_state: Option<ffi::SetRumbleState>,
    policy: Model1PadRumble,
}

impl PadRumble {
    fn new(environment: ffi::Environment) -> Self {
        let mut interface = ffi::RumbleInterface {
            set_rumble_state: None,
        };
        let available = unsafe {
            environment(
                ffi::GET_RUMBLE_INTERFACE,
                (&mut interface as *mut ffi::RumbleInterface).cast(),
            )
        };
        Self {
            set_state: available.then_some(interface.set_rumble_state).flatten(),
            policy: Model1PadRumble::default(),
        }
    }

    fn stop(&mut self) {
        self.policy.reset();
        if let Some(set_state) = self.set_state {
            unsafe {
                set_state(0, ffi::RUMBLE_STRONG, 0);
                set_state(0, ffi::RUMBLE_WEAK, 0);
            }
        }
    }

    fn frame(&mut self, command: u8, steer: u8) {
        if self.set_state.is_none() { return; }
        let (strong, weak) = self.policy.frame(command, steer);
        self.apply(strong, weak);
    }

    fn netmerc(&mut self, on: bool) {
        let (strong, weak) = model1_rumble::binary_motor_levels(on);
        self.apply(strong, weak);
    }

    fn apply(&mut self, strong: f32, weak: f32) {
        let Some(set_state) = self.set_state else { return; };
        let strength = |value: f32| (value.clamp(0.0, 1.0) * u16::MAX as f32).round() as u16;
        unsafe {
            set_state(0, ffi::RUMBLE_STRONG, strength(strong));
            set_state(0, ffi::RUMBLE_WEAK, strength(weak));
        }
    }
}

struct ControllerStorage {
    _full_names: [CString; 2],
    types: [[ffi::ControllerDescription; 2]; 2],
    ports: [ffi::ControllerInfo; 3],
}

// Same subclass convention as SM2 and Supermodel: the base RetroPad includes
// Test/Service; subclass 0 keeps gameplay but removes those cabinet slots.
const NO_SERVICE_DEVICE: u32 = ffi::DEVICE_JOYPAD | (1 << 8);
fn joypad_enabled(device: u32) -> bool {
    matches!(device, ffi::DEVICE_JOYPAD | NO_SERVICE_DEVICE)
}
fn cabinet_button_enabled(device: u32, id: u32) -> bool {
    joypad_enabled(device)
        && (device == ffi::DEVICE_JOYPAD || !matches!(id, ffi::JOY_L3 | ffi::JOY_R3))
}

// Pointers refer only to static C strings or fields inside this boxed object;
// access to the box is serialized by CORE's mutex.
unsafe impl Send for ControllerStorage {}

impl ControllerStorage {
    fn new(profile: Profile) -> Box<Self> {
        let name = match profile {
            Profile::VirtuaRacing | Profile::VirtuaFormula => c"Driving: Sequential + VR4",
            Profile::VirtuaFighter => c"Joystick (Standard): Fighting",
            Profile::WingWar => c"Flight: Wing War + VR4",
            Profile::WingWar360 => c"Flight: Stick + Throttle",
            Profile::StarWarsArcade => c"Flight: Star Wars Arcade (Pilot) + VR1",
            Profile::NetMerc => c"Special: Sega NetMerc",
        };
        let names = (
            name,
            if profile == Profile::StarWarsArcade {
                c"Flight: Star Wars Arcade (Gunner) + VR1"
            } else {
                name
            },
        );
        let full_names = [names.0, names.1].map(|name|
            CString::new(format!("{} + Test/Service Slots", name.to_str().unwrap())).unwrap());
        let mut storage = Box::new(Self {
            types: std::array::from_fn(|port| [
                ffi::ControllerDescription { desc: full_names[port].as_ptr(), id: ffi::DEVICE_JOYPAD },
                ffi::ControllerDescription { desc: [names.0, names.1][port].as_ptr(), id: NO_SERVICE_DEVICE },
            ]),
            _full_names: full_names,
            ports: std::array::from_fn(|_| ffi::ControllerInfo {
                types: ptr::null(),
                num_types: 0,
            }),
        });
        storage.ports[0] = ffi::ControllerInfo {
            types: storage.types[0].as_ptr(),
            num_types: 2,
        };
        storage.ports[1] = ffi::ControllerInfo {
            types: storage.types[1].as_ptr(),
            num_types: 2,
        };
        storage
    }

    fn publish(&mut self, environment: ffi::Environment) {
        unsafe { environment(ffi::SET_CONTROLLER_INFO, self.ports.as_mut_ptr().cast()) };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    VirtuaRacing,
    VirtuaFormula,
    VirtuaFighter,
    WingWar,
    WingWar360,
    StarWarsArcade,
    NetMerc,
}

impl Profile {
    fn for_game(game: &roms_db::GameDef) -> Option<Self> {
        use roms_db::{AnalogRole as A, Scheme};
        if !game.board.is_model1() {
            return None;
        }
        let roles = &game.analog_roles;
        let signature = |scheme, expected: &[A]| {
            game.scheme == scheme
                && roles[..expected.len()] == *expected
                && roles[expected.len()..].iter().all(|role| *role == A::None)
        };
        match game.name.as_str() {
            "vr" if signature(Scheme::Racing, &[A::Steer, A::Accel, A::Brake]) => {
                Some(Self::VirtuaRacing)
            }
            "vformula" if signature(Scheme::Racing, &[A::Steer, A::Accel, A::Brake]) => {
                Some(Self::VirtuaFormula)
            }
            "vf" if signature(Scheme::Joystick, &[]) => Some(Self::VirtuaFighter),
            "wingwar" | "wingwarj" | "wingwaru"
                if signature(Scheme::Flight, &[A::StickX, A::StickY, A::Throttle]) =>
            {
                Some(Self::WingWar)
            }
            "wingwar360" if signature(Scheme::Flight, &[A::StickX, A::StickY, A::Throttle]) => {
                Some(Self::WingWar360)
            }
            "swa" | "swaj"
                if signature(
                    Scheme::Flight,
                    &[
                        A::StickX,
                        A::StickY,
                        A::Throttle,
                        A::None,
                        A::Stick2X,
                        A::Stick2Y,
                    ],
                ) =>
            {
                Some(Self::StarWarsArcade)
            }
            "netmerc" if signature(Scheme::Flight, &[A::StickX, A::None, A::StickY]) => {
                Some(Self::NetMerc)
            }
            _ => None,
        }
    }
}

static CORE: OnceLock<Mutex<Core>> = OnceLock::new();

fn core() -> &'static Mutex<Core> {
    CORE.get_or_init(|| Mutex::new(Core::default()))
}

fn with_core<T>(f: impl FnOnce(&mut Core) -> T) -> T {
    let mut guard = core().lock().unwrap_or_else(|error| error.into_inner());
    f(&mut guard)
}

fn notify(environment: Option<ffi::Environment>, error: &str) {
    notify_frames(environment, error, 300);
}

fn notify_frames(environment: Option<ffi::Environment>, error: &str, frames: u32) {
    notify_priority(environment, error, frames, 1, 1);
}

fn notify_priority(environment: Option<ffi::Environment>, error: &str,
                   frames: u32, priority: u32, level: u32) {
    eprintln!("[TGPulse-Next Libretro] {error}");
    if let Some(env) = environment {
        let safe = error.replace('\0', " ");
        if let Ok(message) = CString::new(safe) {
            // Unlike SET_MESSAGE, queued notifications do not replace each
            // other in RetroArch. Keep the legacy path for older frontends.
            let mut version = 0u32;
            if unsafe { env(ffi::GET_MESSAGE_INTERFACE_VERSION, (&mut version as *mut u32).cast()) }
                && version >= 1 {
                let mut data = ffi::MessageExt {
                    msg: message.as_ptr(),
                    duration: (f64::from(frames) * 1000.0 / FPS).round() as u32,
                    priority, level, target: 0, message_type: 0, progress: -1,
                };
                if unsafe { env(ffi::SET_MESSAGE_EXT, (&mut data as *mut ffi::MessageExt).cast()) } {
                    return;
                }
            }
            let mut data = ffi::Message {
                msg: message.as_ptr(),
                frames,
            };
            // The frontend consumes the message during the call.
            unsafe { env(ffi::SET_MESSAGE, (&mut data as *mut ffi::Message).cast()) };
        }
    }
}

fn option_value(environment: ffi::Environment, key: &'static CStr) -> Option<String> {
    let mut option = ffi::Variable {
        key: key.as_ptr(),
        value: ptr::null(),
    };
    let found = unsafe {
        environment(
            ffi::GET_VARIABLE,
            (&mut option as *mut ffi::Variable).cast(),
        )
    };
    if !found || option.value.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(option.value) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

fn gain_option(environment: ffi::Environment, key: &'static CStr, default: u32) -> u32 {
    option_value(environment, key)
        .and_then(|value| {
            if value == "Mute" || value == "mute" {
                Some(0)
            } else {
                value.parse::<u32>().ok()
            }
        })
        .filter(|value| *value <= AudioGains::MAX && value % 10 == 0)
        .unwrap_or(default)
}

fn update_audio_settings(settings: &mut Settings, environment: ffi::Environment) {
    let reference = AudioGains::REFERENCE;
    settings.audio_gains = AudioGains {
        multipcm1: gain_option(environment, AUDIO_GAIN_KEYS[0], reference.multipcm1),
        multipcm2: gain_option(environment, AUDIO_GAIN_KEYS[1], reference.multipcm2),
        ym3438: gain_option(environment, AUDIO_GAIN_KEYS[2], reference.ym3438),
        dsb: gain_option(environment, AUDIO_GAIN_KEYS[3], reference.dsb),
        scsp: reference.scsp,
    };
    settings.audio_mutes = AudioMutes::default();
}

const AUDIO_DONOR_KEY: &CStr = c"tgpulse_next_netmerc_audio_donor";
const AUDIO_DONOR_VALUES: &[(&CStr, &CStr)] = &[
    (c"vf", c"Virtua Fighter"), (c"vr", c"Virtua Racing"),
    (c"swa", c"Star Wars Arcade"), (c"wingwar", c"Wing War"), (c"off", c"Off"),
];

fn audio_donor_option(environment: ffi::Environment) -> NetmercAudioDonor {
    option_value(environment, AUDIO_DONOR_KEY)
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

// Resource policy matches the standalone: donor game ZIPs are adjacent to the
// loaded content. The system directory is reserved for optional board BIOSes.
fn apply_audio_donor(roms: &mut loader::Model1Roms, path: &str,
                     donor: NetmercAudioDonor) -> Option<String> {
    if roms.ioboard_kind != tgpulse_core::model1board::Kind::NetMerc {
        return None;
    }
    match loader::audio_donor::apply_adjacent(roms, std::path::Path::new(path), donor) {
        Ok(true) => None,
        result => {
            let mode = if roms.netmerc_procedural_audio {
                "Procedural Fallback"
            } else {
                "Original Audio Incomplete (Bad ROM Dumps)"
            };
            if let Err(error) = result {
                eprintln!("[TGPulse-Next Libretro] NetMerc donor unavailable: {error}");
                Some(format!("NetMerc Audio Donor Unavailable - {mode}"))
            } else {
                Some(format!("NetMerc Audio: {mode}"))
            }
        }
    }
}

const DRIVING_RANGE_KEYS: [&CStr; 3] = [
    c"tgpulse_next_steering_output_range",
    c"tgpulse_next_accelerator_output_range",
    c"tgpulse_next_brake_output_range",
];

fn driving_ranges(environment: ffi::Environment) -> [u32; 3] {
    DRIVING_RANGE_KEYS.map(|key| {
        option_value(environment, key)
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| (50..=150).contains(v) && v % 10 == 0)
            .unwrap_or(100)
    })
}

// Keep ramp state unscaled so live changes do not feed back into sampling.
// Model 1 defaults use 20..E0, centered on 80 for steering, rest 20 for pedals.
fn apply_driving_ranges(mut inputs: Inputs, ranges: [u32; 3]) -> Inputs {
    let scale = |value: u8, origin: i32, percent: u32| {
        (origin + (i32::from(value) - origin) * percent.clamp(50, 150) as i32 / 100)
            .clamp(0x20, 0xe0) as u8
    };
    inputs.steer = scale(inputs.steer, 0x80, ranges[0]);
    inputs.accel = scale(inputs.accel, 0x20, ranges[1]);
    inputs.brake = scale(inputs.brake, 0x20, ranges[2]);
    inputs.analog[..3].copy_from_slice(&[inputs.steer, inputs.accel, inputs.brake]);
    inputs
}

// Curve first, range second, exactly as in the reference cores. Sampling ramp
// state stays native/unmodified so live response changes cannot feed back.
fn apply_driving_tuning(
    mut inputs: Inputs,
    ranges: [u32; 3],
    response: steering_response::Response,
) -> Inputs {
    inputs.steer = response.apply(inputs.steer);
    apply_driving_ranges(inputs, ranges)
}

fn publish_nvram_option_visibility(environment: ffi::Environment, set: &str, enabled: bool) {
    for (game, key, _) in netpacket::GAMES {
        let mut display = ffi::OptionDisplay {
            key: key.as_ptr(),
            visible: game == set,
        };
        unsafe {
            environment(
                ffi::SET_CORE_OPTIONS_DISPLAY,
                (&mut display as *mut ffi::OptionDisplay).cast(),
            );
        }
    }

    for (key, visible) in std::iter::once((c"tgpulse_next_nvram_settings", true))
        .chain(
            nvram::FIELDS
                .iter()
                .map(|f| (f.key, f.set == set && enabled)),
        )
    {
        let mut display = ffi::OptionDisplay {
            key: key.as_ptr(),
            visible,
        };
        unsafe {
            environment(
                ffi::SET_CORE_OPTIONS_DISPLAY,
                (&mut display as *mut ffi::OptionDisplay).cast(),
            );
        }
    }
}

unsafe extern "C" fn update_nvram_option_display() -> bool {
    with_core(|core| {
        let Some(env) = core.environment else {
            return false;
        };
        let set = core.game.as_ref().map_or("", |g| g.set_name.as_str());
        let enabled = nvram::supported(set)
            && option_value(env, c"tgpulse_next_nvram_settings").is_some_and(|v| v == "enabled");
        publish_nvram_option_visibility(env, set, enabled);
        let sources = core.game.as_ref().map_or(&[][..], |game| game.machine.sound.sources());
        publish_global_option_visibility(env, sources);
        let changed = core.nvram_display_enabled != enabled;
        core.nvram_display_enabled = enabled;
        changed
    })
}

// General Core Options remain visible. Source gains follow the fitted Model 1
// sound hardware; their global values remain available across games.
fn publish_global_option_visibility(environment: ffi::Environment, sources: &[AudioSource]) {
    for definition in &options().definitions {
        if definition.key.is_null() { continue; }
        let key = unsafe { CStr::from_ptr(definition.key) };
        let bytes = key.to_bytes();
        if (bytes.starts_with(b"tgpulse_next_nvram_") && key != c"tgpulse_next_nvram_settings")
            || bytes.starts_with(b"tgpulse_next_linked_cabinets_") { continue; }
        let visible = match AUDIO_GAIN_KEYS.iter().position(|gain| *gain == key) {
            Some(index) => sources.contains(&AUDIO_GAIN_SOURCES[index]),
            None => true,
        };
        let mut display = ffi::OptionDisplay { key: definition.key, visible };
        unsafe { environment(ffi::SET_CORE_OPTIONS_DISPLAY, (&mut display as *mut ffi::OptionDisplay).cast()); }
    }
}

fn city_workaround_option(environment: ffi::Environment) -> bool {
    option_value(environment, c"tgpulse_next_netmerc_city_workaround")
        .is_none_or(|value| value != "disabled")
}

fn mvd_settings(environment: ffi::Environment) -> mvd::Settings {
    mvd::Settings {
        mode: mvd::Mode::parse(option_value(environment, mvd::KEYS[0]).as_deref()),
        range: [
            mvd::range(option_value(environment, mvd::KEYS[1]).as_deref(), 30),
            mvd::range(option_value(environment, mvd::KEYS[2]).as_deref(), 20),
        ],
        gravity: option_value(environment, mvd::KEYS[3]).is_none_or(|v| v != "disabled"),
        drift_compensation: mvd::drift_compensation(option_value(environment, mvd::KEYS[4]).as_deref()),
        sensor_diagnostics: option_value(environment, mvd::KEYS[5]).is_some_and(|v| v == "enabled"),
    }
}

fn update_settings(core: &mut Core) {
    let Some(env) = core.environment else { return };
    core.settings.smooth_shadows = option_value(env, c"tgpulse_next_smooth_shadows")
        .map_or(Config::default().smooth_shadows, |value| value == "enabled");
    core.settings.volume = option_value(env, c"tgpulse_next_volume")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value <= 800)
        .unwrap_or(100);
    core.settings.apply_known_rom_repairs =
        option_value(env, c"tgpulse_next_rom_repairs").is_none_or(|value| value != "disabled");
    core.settings.fast_60hz =
        option_value(env, c"tgpulse_next_av_timing").is_some_and(|value| value == "60hz");
    core.settings.aspect_ratio = aspect_ratio_option(env);
    core.settings.rumble =
        option_value(env, c"tgpulse_next_gamepad_rumble").is_none_or(|value| value != "disabled");
    core.settings.overlay_font = overlay_font_option(env);
    core.settings.diagnostic = diagnostic_settings(env);
    core.settings.netmerc_city_workaround = city_workaround_option(env);
    core.settings.driving_ranges = driving_ranges(env);
    core.settings.mvd = mvd_settings(env);
    core.settings.steering_response = steering_response::Response::parse(
        option_value(env, c"tgpulse_next_steering_response").as_deref(),
    );
    core.settings.automatic_nvram =
        option_value(env, c"tgpulse_next_initial_nvram_setup").is_none_or(|v| v != "disabled");
    core.settings.nvram_settings =
        option_value(env, c"tgpulse_next_nvram_settings").is_some_and(|v| v == "enabled");
    update_audio_settings(&mut core.settings, env);
}

fn diagnostic_settings(environment: ffi::Environment) -> diagnostic_overlay::Settings {
    diagnostic_overlay::Settings {
        enabled: option_value(environment, diagnostic_overlay::KEYS[0]).is_some_and(|v| matches!(v.as_str(), "overlay" | "overlay_half")),
        half: option_value(environment, diagnostic_overlay::KEYS[0]).is_some_and(|v| v == "overlay_half"),
        corner: match option_value(environment, diagnostic_overlay::KEYS[1]).as_deref() {
            Some("top_left") => 0, Some("bottom_right") => 2, Some("bottom_left") => 3, _ => 1,
        },
        opacity: option_value(environment, diagnostic_overlay::KEYS[2])
            .and_then(|v| v.parse::<u32>().ok()).filter(|v| *v <= 100 && v % 10 == 0).unwrap_or(80),
    }
}

fn bios_directory(environment: ffi::Environment) -> Option<std::path::PathBuf> {
    let mut path: *const c_char = ptr::null();
    if unsafe { environment(ffi::GET_SYSTEM_DIRECTORY, (&mut path as *mut *const c_char).cast()) }
        && !path.is_null() {
        let value = unsafe { CStr::from_ptr(path) }.to_string_lossy();
        if !value.is_empty() { return Some(std::path::PathBuf::from(value.as_ref()).join("tgpulse-next")); }
    }
    None
}

fn aspect_ratio_option(environment: ffi::Environment) -> AspectRatio {
    match option_value(environment, c"tgpulse_next_aspect_ratio").as_deref() {
        Some("4_3") => AspectRatio::FourThree,
        Some("16_9") => AspectRatio::SixteenNine,
        _ => AspectRatio::Auto,
    }
}

fn overlay_font_option(environment: ffi::Environment) -> u32 {
    match option_value(environment, c"tgpulse_next_timing_overlay").as_deref() {
        Some("auto") => 13,
        Some("11") => 11,
        Some("12") => 12,
        Some("13") => 13,
        Some("14") => 14,
        _ => 0,
    }
}

fn has_native_widescreen(set_name: &str) -> bool {
    // Confirmed standalone/service catalogue; do not infer VFormula from VR.
    set_name == "vr"
}

fn widescreen_hack_mode(set_name: &str, selected: u32) -> u32 {
    if has_native_widescreen(set_name) { 0 } else { selected }
}

// VR's Game System Monitor setting is byte 0x0a in its little-endian EEPROM.
// Match the standalone interpretation; other Model 1 sets have no verified
// native widescreen monitor setting.
fn selected_aspect_ratio(mode: AspectRatio, set_name: &str, eeprom: &[u16]) -> f32 {
    match mode {
        AspectRatio::FourThree => ASPECT_4_3,
        AspectRatio::SixteenNine => ASPECT_16_9,
        AspectRatio::Auto => {
            if has_native_widescreen(set_name) && eeprom.get(5).is_some_and(|word| (*word & 0xff) == 1) {
                ASPECT_16_9
            } else {
                ASPECT_4_3
            }
        }
    }
}

fn expanded_render_width(hardware: bool, wide: u32, aspect: f32) -> u32 {
    if hardware && wide > 0 && aspect > ASPECT_4_3 + 0.01 {
        683
    } else {
        WIDTH as u32
    }
}
fn render_width(game: &Game, aspect: AspectRatio) -> u32 {
    expanded_render_width(
        game.hardware,
        game.graphics.wide,
        game_aspect_ratio(game, aspect),
    )
}

unsafe extern "C" fn gpu_context_reset() {
    let (environment, callback) =
        with_core(|core| (core.environment, core.hw_callback.as_deref().copied()));
    let (Some(environment), Some(callback)) = (environment, callback) else {
        return;
    };
    let result = if callback.context_type == 6 {
        let mut interface: *const gpu::vulkan::Interface = ptr::null();
        let found = environment(
            gpu::GET_HW_RENDER_INTERFACE,
            (&mut interface as *mut *const gpu::vulkan::Interface).cast(),
        );
        if found && !interface.is_null() {
            gpu::vulkan::Renderer::new(*interface).map(gpu::Renderer::Vulkan)
        } else {
            Err("Frontend Vulkan interface unavailable".into())
        }
    } else {
        gpu::opengl::Renderer::new(callback).map(gpu::Renderer::OpenGl)
    };
    with_core(|core| {
        if let Some(old) = core.gpu.take() {
            old.abandon()
        }
        match result {
            Ok(renderer) => {
                core.gpu = Some(renderer);
                core.hardware_error = None;
                notify(
                    core.environment,
                    if callback.context_type == 6 {
                        "Renderer: Vulkan"
                    } else {
                        "Renderer: OpenGL"
                    },
                );
            }
            Err(error) => {
                notify(core.environment, &error);
                core.hardware_error = Some(error);
            }
        }
    });
}
unsafe extern "C" fn gpu_context_destroy() {
    with_core(|core| {
        core.gpu = None;
        core.hardware_error = None;
    });
}
fn graphics_settings(environment: ffi::Environment) -> gpu::Settings {
    gpu::Settings {
        backend: match option_value(environment, c"tgpulse_next_renderer").as_deref() {
            Some("vulkan") => gpu::Backend::Vulkan,
            Some("opengl") => gpu::Backend::OpenGl,
            Some("software") => gpu::Backend::Software,
            _ => gpu::Backend::Auto,
        },
        wide: match option_value(environment, c"tgpulse_next_widescreen_mode").as_deref() {
            Some("stretch") => 0,
            Some("expand_3d_2d") => 2,
            _ => 1,
        },
        ss: option_value(environment, c"tgpulse_next_supersampling")
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| (1..=4).contains(v))
            .unwrap_or(1),
        srgb: false,
    }
}
fn request_graphics(
    core: &mut Core,
    settings: gpu::Settings,
    environment: ffi::Environment,
) -> Result<bool, String> {
    let mut preferred = 0u32;
    let has_preference = unsafe { environment(56, (&mut preferred as *mut u32).cast()) };
    let backend = match settings.backend {
        gpu::Backend::Auto => {
            if has_preference && preferred == 6 {
                gpu::Backend::Vulkan
            } else if has_preference && matches!(preferred, 1..=5) && !cfg!(target_os = "macos") {
                gpu::Backend::OpenGl
            } else {
                gpu::Backend::Software
            }
        }
        backend => backend,
    };
    if backend == gpu::Backend::Software {
        core.hardware_requested = false;
        notify(Some(environment), "Renderer: Software");
        return Ok(false);
    }
    let callback = core.hw_callback.get_or_insert_with(|| {
        Box::new(gpu::HwCallback::vulkan(
            gpu_context_reset,
            gpu_context_destroy,
        ))
    });
    **callback = gpu::HwCallback::vulkan(gpu_context_reset, gpu_context_destroy);
    if backend == gpu::Backend::OpenGl {
        let es = matches!(preferred, 2 | 4 | 5);
        callback.context_type = if es { 5 } else { 3 };
        callback.version_major = if es { 3 } else { 4 };
        callback.version_minor = if es { 1 } else { 3 };
        callback.bottom_left_origin = true;
    }
    let accepted = unsafe {
        environment(
            gpu::SET_HW_RENDER,
            (&mut **callback as *mut gpu::HwCallback).cast(),
        )
    };
    core.hardware_requested = accepted;
    core.hardware_error = None;
    if !accepted && settings.backend != gpu::Backend::Auto {
        return Err("Frontend rejected the requested GPU context".into());
    }
    if !accepted {
        notify(
            Some(environment),
            "Renderer: Software (GPU context unavailable)",
        )
    }
    Ok(accepted)
}

fn game_aspect_ratio(game: &Game, mode: AspectRatio) -> f32 {
    selected_aspect_ratio(mode, &game.set_name, &game.machine.ioboard.eeprom().data)
}

fn publish_aspect_ratio(game: &mut Game, mode: AspectRatio, environment: ffi::Environment) {
    let aspect_ratio = game_aspect_ratio(game, mode);
    if game.reported_aspect_ratio == Some(aspect_ratio)
        && game.reported_width == render_width(game, mode)
    {
        return;
    }
    let mut geometry = ffi::Geometry {
        base_width: render_width(game, mode),
        base_height: HEIGHT as u32,
        max_width: 683,
        max_height: HEIGHT as u32,
        aspect_ratio,
    };
    unsafe {
        environment(
            ffi::SET_GEOMETRY,
            (&mut geometry as *mut ffi::Geometry).cast(),
        )
    };
    game.reported_aspect_ratio = Some(aspect_ratio);
    game.reported_width = render_width(game, mode);
}

struct OptionStorage {
    categories: [ffi::OptionCategory; 5],
    definitions: Vec<ffi::OptionDefinition>,
}

// Global frontend-owned source gains, shared by all Model 1 ROM sets.
const AUDIO_GAIN_KEYS: [&CStr; 4] = [
    c"tgpulse_next_multipcm1_gain",
    c"tgpulse_next_multipcm2_gain",
    c"tgpulse_next_ym3438_gain",
    c"tgpulse_next_dsb_gain",
];
const AUDIO_GAIN_SOURCES: [AudioSource; 4] = [
    AudioSource::MultiPcm1,
    AudioSource::MultiPcm2,
    AudioSource::Ym3438,
    AudioSource::Dsb,
];

const GAIN_VALUES: &[(&CStr, &CStr)] = &[
    (c"mute", c"Mute"),
    (c"auto", c"Auto"),
    (c"0", c"0%"),
    (c"10", c"10%"),
    (c"20", c"20%"),
    (c"30", c"30%"),
    (c"40", c"40%"),
    (c"50", c"50%"),
    (c"60", c"60%"),
    (c"70", c"70%"),
    (c"80", c"80%"),
    (c"90", c"90%"),
    (c"100", c"100%"),
];
const RUMBLE_VALUES: &[(&CStr, &CStr)] = &[(c"enabled", c"Enabled"), (c"disabled", c"Disabled")];

// Option storage contains only immutable pointers to static C strings and is
// never modified after OnceLock initialization.
unsafe impl Send for OptionStorage {}
unsafe impl Sync for OptionStorage {}
static OPTIONS: OnceLock<OptionStorage> = OnceLock::new();

fn option_definition(
    key: &'static CStr,
    label: &'static CStr,
    info: &'static CStr,
    category: &'static CStr,
    values: &[(&'static CStr, &'static CStr)],
    default: &'static CStr,
) -> ffi::OptionDefinition {
    let mut options = [ffi::OptionValue {
        value: ptr::null(),
        label: ptr::null(),
    }; 128];
    for (slot, (value, description)) in options.iter_mut().zip(values) {
        *slot = ffi::OptionValue {
            value: value.as_ptr(),
            label: description.as_ptr(),
        };
    }
    ffi::OptionDefinition {
        key: key.as_ptr(),
        desc: label.as_ptr(),
        desc_categorized: ptr::null(),
        info: info.as_ptr(),
        info_categorized: ptr::null(),
        category_key: category.as_ptr(),
        values: options,
        default_value: default.as_ptr(),
    }
}

// Keep Master Volume numeric, with the standalone range and reference-core step.
fn master_volume_values() -> &'static [(CString, CString)] {
    static VALUES: OnceLock<Vec<(CString, CString)>> = OnceLock::new();
    VALUES.get_or_init(|| {
        (0..=800)
            .step_by(10)
            .map(|value| {
                (
                    CString::new(value.to_string()).unwrap(),
                    CString::new(if value == 0 {
                        "OFF".to_owned()
                    } else {
                        format!("{value}%")
                    })
                    .unwrap(),
                )
            })
            .collect()
    })
}
fn master_volume_legacy() -> &'static CStr {
    static VALUE: OnceLock<CString> = OnceLock::new();
    VALUE
        .get_or_init(|| {
            let mut values = vec!["100".to_owned()];
            values.extend(
                (0..=800)
                    .step_by(10)
                    .filter(|v| *v != 100)
                    .map(|v| v.to_string()),
            );
            CString::new(format!("Master Volume; {}", values.join("|"))).unwrap()
        })
        .as_c_str()
}

// SM2/Supermodel: System, Video, Audio, Input; related controls stay adjacent.
fn menu_order(key: *const c_char) -> (u8, u8) {
    if key.is_null() { return (255, 255); }
    let key = unsafe { CStr::from_ptr(key) }.to_bytes();
    let key = key.strip_prefix(b"tgpulse_next_").unwrap_or(key);
    match key {
        b"rom_repairs" => (0, 0),
        b"initial_nvram_setup" => (0, 1),
        _ if key.starts_with(b"linked_cabinets_") => (0, 2),
        b"nvram_settings" => (0, 3),
        _ if key.starts_with(b"nvram_") => (0, 4),
        b"renderer" => (1, 0),
        b"supersampling" => (1, 1),
        b"smooth_shadows" => (1, 2),
        b"netmerc_city_workaround" => (1, 3),
        b"aspect_ratio" => (1, 4),
        b"widescreen_mode" => (1, 5),
        b"av_timing" => (1, 6),
        b"timing_overlay" => (1, 7),
        b"netmerc_diagnostic_display" => (1, 8),
        b"netmerc_diagnostic_position" => (1, 9),
        b"netmerc_diagnostic_opacity" => (1, 10),
        b"volume" => (2, 0),
        b"multipcm1_gain" => (2, 1),
        b"multipcm2_gain" => (2, 2),
        b"ym3438_gain" => (2, 3),
        b"dsb_gain" => (2, 4),
        b"netmerc_audio_donor" => (2, 5),
        b"gamepad_rumble" => (3, 0),
        b"steering_response" => (3, 1),
        b"steering_output_range" => (3, 2),
        b"accelerator_output_range" => (3, 3),
        b"brake_output_range" => (3, 4),
        b"netmerc_mvd_input" => (3, 5),
        b"netmerc_mvd_horizontal_range" => (3, 6),
        b"netmerc_mvd_vertical_range" => (3, 7),
        b"netmerc_mvd_gravity_stabilization" => (3, 8),
        b"netmerc_mvd_drift_compensation" => (3, 9),
        b"netmerc_mvd_sensor_diagnostics" => (3, 10),
        _ => (254, 0),
    }
}

const MVD_RANGE_VALUES: &[(&CStr, &CStr)] = &[
    (c"0", c"Off"), (c"10", c"10 Degrees"), (c"20", c"20 Degrees"),
    (c"30", c"30 Degrees"), (c"40", c"40 Degrees"), (c"50", c"50 Degrees"),
    (c"60", c"60 Degrees"), (c"70", c"70 Degrees"), (c"80", c"80 Degrees"),
    (c"90", c"90 Degrees"),
];

fn options() -> &'static OptionStorage {
    OPTIONS.get_or_init(|| {
    let mut storage = OptionStorage {
        categories: [
            ffi::OptionCategory {
                key: c"system".as_ptr(),
                desc: c"System".as_ptr(),
                info: c"Model 1 ROM compatibility.".as_ptr(),
            },
            ffi::OptionCategory {
                key: c"video".as_ptr(),
                desc: c"Video".as_ptr(),
                info: c"Model 1 image settings.".as_ptr(),
            },
            ffi::OptionCategory {
                key: c"audio".as_ptr(),
                desc: c"Audio".as_ptr(),
                info: c"Model 1 output level.".as_ptr(),
            },
            ffi::OptionCategory {
                key: c"input".as_ptr(),
                desc: c"Input".as_ptr(),
                info: c"Model 1 cabinet feedback.".as_ptr(),
            },
            ffi::OptionCategory {
                key: ptr::null(),
                desc: ptr::null(),
                info: ptr::null(),
            },
        ],
        definitions: vec![
            option_definition(
                c"tgpulse_next_rom_repairs",
                c"Known Bad Dump ROM Repairs (Restart Required)",
                c"Repair a recognized legacy 315-5711 dump in memory after SHA-1 verification. ZIP files are never changed. Reload content after changing this option.",
                c"system",
                &[(c"enabled", c"Enabled"), (c"disabled", c"Disabled")],
                c"enabled",
            ),
            option_definition(
                c"tgpulse_next_smooth_shadows",
                c"Smooth Shadows (Restart Required)",
                c"Blend Model 1 stippled shadows instead of preserving the hardware dither. Reload content after changing this option.",
                c"video",
                &[(c"enabled", c"Enabled"), (c"disabled", c"Disabled")],
                c"enabled",
            ),
            option_definition(
                c"tgpulse_next_av_timing",
                c"A/V Timing (Restart Required)",
                c"Native follows the Model 1 clock. 60 Hz runs one new machine frame per callback, making game and sound about 4.3% faster. Audio is resampled to the normal output rate. Reload content after changing this option.",
                c"video",
                &[(c"native", c"Native (57.524 Hz)"), (c"60hz", c"60 Hz (Faster Emulation)")],
                c"native",
            ),
            option_definition(
                c"tgpulse_next_aspect_ratio",
                c"Aspect Ratio",
                c"Auto follows Virtua Racing's saved 4:3 or 16:9 monitor setting; other Model 1 games default to 4:3. Explicit values override presentation aspect only. The 496x384 image is not widened. Changes take effect immediately.",
                c"video",
                &[(c"auto", c"Auto"), (c"4_3", c"4:3"), (c"16_9", c"16:9")],
                c"auto",
            ),
            option_definition(
                c"tgpulse_next_volume",
                c"Master Volume",
                c"Adjust the mixed Model 1 output level. Changes take effect immediately.",
                c"audio",
                &master_volume_values().iter().map(|(value, label)| (value.as_c_str(), label.as_c_str())).collect::<Vec<_>>(),
                c"100",
            ),
            option_definition(
                c"tgpulse_next_multipcm1_gain", c"MultiPCM 1 Gain",
                c"Set the absolute output gain for MultiPCM 1. 50% is selected by Auto and matches the standalone reference mix. Changes take effect immediately without stopping the chip.",
                c"audio", GAIN_VALUES, c"auto",
            ),
            option_definition(
                c"tgpulse_next_multipcm2_gain", c"MultiPCM 2 Gain",
                c"Set the absolute output gain for MultiPCM 2. 50% is selected by Auto and matches the standalone reference mix. Changes take effect immediately without stopping the chip.",
                c"audio", GAIN_VALUES, c"auto",
            ),
            option_definition(
                c"tgpulse_next_ym3438_gain", c"FM (YM3438) Gain",
                c"Set the absolute FM output gain. 30% is selected by Auto and matches the standalone reference mix. Changes take effect immediately without stopping the chip.",
                c"audio", GAIN_VALUES, c"auto",
            ),
            option_definition(
                c"tgpulse_next_dsb_gain", c"DSB (MPEG) Gain",
                c"Set the absolute output gain for a fitted DSB music board. 100% is selected by Auto and matches the standalone reference mix. Changes take effect immediately.",
                c"audio", GAIN_VALUES, c"auto",
            ),
            option_definition(
                AUDIO_DONOR_KEY, c"Sega NetMerc Audio Donor (Restart Required)",
                c"Use substitute MultiPCM samples from Virtua Fighter, Virtua Racing, Star Wars Arcade or Wing War. Donor ZIPs must be beside netmerc.zip. Off disables the donor, not the procedural fallback for missing original samples. Missing or invalid donors retain the available audio path. Applies only to Sega NetMerc. Reload content after changing this option.",
                c"audio", AUDIO_DONOR_VALUES, c"vf",
            ),
            option_definition(
                c"tgpulse_next_gamepad_rumble", c"Gamepad Rumble",
                c"Forward Virtua Racing/Virtua Formula drive-board feedback and Sega NetMerc's cabinet motor to player one's gamepad. Enabled by default. Adjust overall intensity through the frontend's rumble gain. Requires a frontend and controller with rumble support. Changes take effect immediately.",
                c"input", RUMBLE_VALUES, c"enabled",
            ),
            option_definition(
                c"tgpulse_next_timing_overlay", c"Timing / FPS Overlay",
                c"Show 61-frame averages for machine, video and audio work, total retro_run time, worst frame, actual frontend cadence and estimated processing capacity. Auto uses a 13 px font. Drawn into the game frame; adds rendering cost. Changes take effect immediately.",
                c"video",
                &[(c"disabled", c"Off"), (c"auto", c"Auto"), (c"11", c"On (11 px)"), (c"12", c"On (12 px)"), (c"13", c"On (13 px)"), (c"14", c"On (14 px)")],
                c"disabled",
            ),
            ffi::OptionDefinition {
                key: ptr::null(),
                desc: ptr::null(),
                desc_categorized: ptr::null(),
                info: ptr::null(),
                info_categorized: ptr::null(),
                category_key: ptr::null(),
                values: [ffi::OptionValue {
                    value: ptr::null(),
                    label: ptr::null(),
                }; 128],
                default_value: ptr::null(),
            },
        ],
    };
    let end=storage.definitions.len()-1;
    storage.definitions.splice(end..end,[
        option_definition(c"tgpulse_next_renderer",c"Renderer (Restart Required)",c"Select frontend-owned GPU rendering or Software. Auto follows supported frontend context preference.",c"video",&[(c"auto",c"Auto"),(c"vulkan",c"Vulkan"),(c"opengl",c"OpenGL / GLES"),(c"software",c"Software")],c"auto"),
        option_definition(c"tgpulse_next_widescreen_mode",c"Widescreen Hack (Restart Required)",c"Applies only to games without native widescreen support. Expand the 3D view while preserving or stretching 2D layers, or stretch the complete image. Requires a widescreen Aspect Ratio and hardware rendering. Native widescreen, including Service Menu settings, is preserved.",c"video",&[(c"expand_3d",c"Expand 3D View Only"),(c"expand_3d_2d",c"Expand 3D View + Stretch 2D"),(c"stretch",c"Stretch 3D and 2D View")],c"expand_3d"),
        option_definition(c"tgpulse_next_supersampling",c"Supersampling (Restart Required)",c"Requires Vulkan or OpenGL/GLES rendering. Use the standalone Model 1 supersampling pass. Higher scales average more 3D samples per output pixel and increase GPU time and memory. Tile detail is unchanged.",c"video",&[(c"1",c"1x (Disabled)"),(c"2",c"2x (4 Samples)"),(c"3",c"3x (9 Samples)"),(c"4",c"4x (16 Samples)")],c"1")
    ]);
    let values = [(c"50", c"50%"), (c"60", c"60%"), (c"70", c"70%"),
        (c"80", c"80%"), (c"90", c"90%"), (c"100", c"100%"),
        (c"110", c"110%"), (c"120", c"120%"), (c"130", c"130%"),
        (c"140", c"140%"), (c"150", c"150%")];
    let end = storage.definitions.len() - 1;
    for (index, label, info) in [
        (0, c"Driving Steering Output Range", c"Applied only to VR/VFormula. Scale steering around its native center. Below 100% reduces the emulated wheel range; above 100% reaches full lock with less stick travel. Changes take effect immediately."),
        (1, c"Driving Accelerator Output Range", c"Applied only to VR/VFormula. Scale the accelerator from its native released position. Below 100% reduces maximum pedal output; above 100% reaches full output with less trigger travel. Changes take effect immediately."),
        (2, c"Driving Brake Output Range", c"Applied only to VR/VFormula. Scale the brake from its native released position. Below 100% reduces maximum pedal output; above 100% reaches full output with less trigger travel. Changes take effect immediately."),
    ].into_iter().rev() {
        storage.definitions.insert(end, option_definition(DRIVING_RANGE_KEYS[index], label, info, c"input", &values, c"100"));
    }
    let end = storage.definitions.len() - 1;
    let mut definitions = vec![
        option_definition(diagnostic_overlay::KEYS[0], c"Sega NetMerc Diagnostic Display", c"Show the cabinet's HD44780 diagnostic LCD in the game frame. Applied only to Sega NetMerc. The optional validated hd44780.zip BIOS is searched in system/tgpulse-next/, beside the game, then inside the game ZIP. Select full size (100%) or half size (50%). If unavailable while ON is selected, a notification is shown and the LCD is not drawn. Changes take effect immediately.", c"video", &[(c"off", c"OFF"), (c"overlay", c"ON (100%)"), (c"overlay_half", c"ON (50%)")], c"off"),
        option_definition(diagnostic_overlay::KEYS[1], c"Sega NetMerc Diagnostic Position", c"Select the diagnostic LCD corner in final output coordinates. The Timing / FPS panel is drawn over the LCD when their areas overlap. Changes take effect immediately.", c"video", &[(c"top_left", c"Top Left"), (c"top_right", c"Top Right"), (c"bottom_right", c"Bottom Right"), (c"bottom_left", c"Bottom Left")], c"top_right"),
        option_definition(diagnostic_overlay::KEYS[2], c"Sega NetMerc Diagnostic Background Opacity", c"Set diagnostic LCD background opacity. Active glyphs remain opaque. Changes take effect immediately.", c"video", &[(c"0", c"0%"), (c"10", c"10%"), (c"20", c"20%"), (c"30", c"30%"), (c"40", c"40%"), (c"50", c"50%"), (c"60", c"60%"), (c"70", c"70%"), (c"80", c"80%"), (c"90", c"90%"), (c"100", c"100%")], c"80"),
        option_definition(c"tgpulse_next_netmerc_city_workaround", c"Sega NetMerc City Workaround", c"Apply the Sega NetMerc City TGP conversion workaround for correct road geometry. Applies only to Sega NetMerc, with any renderer. Enabled matches the standalone default. Changes take effect on subsequent matching conversions without restarting content.", c"video", &[(c"enabled", c"Enabled"), (c"disabled", c"Disabled")], c"enabled"),
        option_definition(mvd::KEYS[0], c"Sega NetMerc MVD Input", c"Applied only to Sega NetMerc. Select head tracking. Auto prefers calibrated P1 sensors, then the right stick, otherwise a fixed camera. Sensors uses calibrated P1 motion sensors or a fixed camera while unavailable. Keep P1 still for three seconds to calibrate. Off fixes the camera; Right Stick uses absolute stick deflection and returns to forward view on release. The frontend owns remapping and dead zones. Changes take effect immediately.", c"input", &[(c"auto", c"Auto"), (c"off", c"Off (Fixed Camera)"), (c"right_stick", c"Right Stick"), (c"sensors", c"Sensors (e.g. DualSense)")], c"auto"),
        option_definition(mvd::KEYS[1], c"Sega NetMerc MVD Horizontal Range", c"Set the right-stick horizontal look range on each side of forward, in degrees. Off locks this axis. Applied only to Sega NetMerc stick MVD input; changes take effect immediately.", c"input", MVD_RANGE_VALUES, c"30"),
        option_definition(mvd::KEYS[2], c"Sega NetMerc MVD Vertical Range", c"Set the right-stick vertical look range on each side of forward, in degrees. Off locks this axis. Applied only to Sega NetMerc stick MVD input; changes take effect immediately.", c"input", MVD_RANGE_VALUES, c"20"),
        option_definition(mvd::KEYS[3], c"Sega NetMerc MVD Gravity Stabilization", c"Applied only to Sega NetMerc sensor input. Use accelerometer gravity to stabilize pitch and roll; yaw remains gyroscope-based. Without an accelerometer, tracking uses the gyroscope alone. Changes take effect immediately.", c"input", &[(c"enabled", c"Enabled"), (c"disabled", c"Disabled")], c"enabled"),
        option_definition(mvd::KEYS[4], c"Sega NetMerc MVD Drift Compensation", c"Applied only to Sega NetMerc sensor input. Refine the calibrated gyroscope bias after three seconds of detected stillness. Default 50% uses gradual correction; higher percentages learn faster. Very slow turns may be mistaken for drift. Keeps the current view without automatic recentering. Off uses the original fixed calibration. Changes take effect immediately.", c"input", &[(c"off", c"Off"), (c"10", c"10%"), (c"20", c"20%"), (c"30", c"30%"), (c"40", c"40%"), (c"50", c"50%"), (c"60", c"60%"), (c"70", c"70%"), (c"80", c"80%"), (c"90", c"90%"), (c"100", c"100%")], c"50"),
        option_definition(mvd::KEYS[5], c"Sega NetMerc MVD Sensor Diagnostics", c"Record P1 motion callbacks, calibration, drift correction and orientation to a CSV in the frontend save directory under tgpulse-next/diagnostics. Applied only to Sega NetMerc while sensors are active. A notification shows the path. Disable to close the file; reset or state load closes it and starts a new file when sensors resume. A recording stops after 30 minutes; toggle Disabled then Enabled to start another. Default Disabled. Changes take effect immediately.", c"input", &[(c"disabled", c"Disabled"), (c"enabled", c"Enabled")], c"disabled"),
        option_definition(c"tgpulse_next_initial_nvram_setup", c"Automatic Initial NVRAM Setup (Restart Required)", c"When no valid frontend save exists, initialize the loaded set from its validated complete sample before the first frame. Apply approved country, offline network and VR Special cabinet settings, or Sega NetMerc controller endpoint calibration; keep other native fields. Never replaces an existing valid save. Delete save data to regenerate.", c"system", &[(c"enabled",c"Enabled"),(c"disabled",c"Disabled")], c"enabled"),
        option_definition(c"tgpulse_next_nvram_settings", c"NVRAM Settings", c"For games with reviewed operator fields, apply the displayed settings at startup and when Core Options change, overriding saved values for these fields. Disabled preserves service-menu settings. Applying changed values resets the machine.", c"system", &[(c"disabled",c"Disabled"),(c"enabled",c"Enabled")], c"disabled"),
    ];
    for field in nvram::FIELDS {
        let values: Vec<_> = field.values.iter().map(|v| (v.key, v.label)).collect();
        definitions.push(option_definition(field.key, field.label, c"Reviewed operator setting for the loaded ROM set. Applied only while NVRAM Settings is enabled. Other EEPROM fields and calibration are preserved. A changed persisted value resets the machine.", c"system", &values, field.values[field.default].key));
    }
    definitions.push(option_definition(c"tgpulse_next_steering_response", c"Driving Steering Response", c"Applied only to VR/VFormula. Select the steering response curve; Progressive and FBNeo Logarithmic reduce sensitivity around the center while retaining the available output range. Changes take effect immediately.", c"input", &[(c"linear",c"Linear"),(c"progressive",c"Progressive (Fine Center)"),(c"fbneo",c"FBNeo Logarithmic (Fine Center)")], c"linear"));
    for (_,key,max) in netpacket::GAMES {
        let mut values=vec![(c"disabled",c"Disabled"),(c"2",c"2 Cabinets")];
        if max>2 {values.extend([(c"3",c"3 Cabinets"),(c"4",c"4 Cabinets"),(c"5",c"5 Cabinets"),(c"6",c"6 Cabinets"),(c"7",c"7 Cabinets"),(c"8",c"8 Cabinets"),(c"9",c"9 Cabinets")]);}
        let info = if max==2 {c"Use one Master and one Slave through RetroArch Netplay. NETWORK roles remain game NVRAM settings. Requires Netpacket support. Save States are unavailable while the COMM board is fitted. Reload content after changing this option."} else {c"Use one MASTER, remaining SLAVEs and at most one optional LIVE relay through RetroArch Netplay. Select the total including LIVE; at most eight playable cabinets. LINK ID and unique CAR COLOR/NUMBER remain game NVRAM settings. Requires Netpacket support. Save States are unavailable while COMM is fitted. Reload content after changing this option."};
        definitions.push(option_definition(key,c"Linked Cabinets (Restart Required)",info,c"system",&values,c"disabled"));
    }
    storage.definitions.splice(end..end, definitions);
    // Stable order keeps per-set NVRAM fields and linked-cabinet entries together.
    storage.definitions.sort_by_key(|definition| menu_order(definition.key));
    storage
    })
}

fn register_options(environment: ffi::Environment) {
    let mut version = 0u32;
    let supported = unsafe {
        environment(
            ffi::GET_CORE_OPTIONS_VERSION,
            (&mut version as *mut u32).cast(),
        )
    };
    if supported && version >= 2 {
        let storage = options();
        let mut data = ffi::OptionsV2 {
            categories: storage.categories.as_ptr(),
            definitions: storage.definitions.as_ptr(),
        };
        if unsafe {
            environment(
                ffi::SET_CORE_OPTIONS_V2,
                (&mut data as *mut ffi::OptionsV2).cast(),
            )
        } {
            return;
        }
    }
    let mut legacy = vec![
        ffi::Variable {
            key: c"tgpulse_next_rom_repairs".as_ptr(),
            value: c"Known Bad Dump ROM Repairs (Restart Required); enabled|disabled".as_ptr(),
        },
        ffi::Variable {
            key: c"tgpulse_next_smooth_shadows".as_ptr(),
            value: c"Smooth Shadows (Restart Required); enabled|disabled".as_ptr(),
        },
        ffi::Variable {
            key: c"tgpulse_next_av_timing".as_ptr(),
            value: c"A/V Timing (Restart Required); native|60hz".as_ptr(),
        },
        ffi::Variable {
            key: c"tgpulse_next_aspect_ratio".as_ptr(),
            value: c"Aspect Ratio; auto|4_3|16_9".as_ptr(),
        },
        ffi::Variable {
            key: c"tgpulse_next_volume".as_ptr(),
            value: master_volume_legacy().as_ptr(),
        },
        ffi::Variable {
            key: c"tgpulse_next_gamepad_rumble".as_ptr(),
            value: c"Gamepad Rumble; enabled|disabled".as_ptr(),
        },
        ffi::Variable {
            key: c"tgpulse_next_timing_overlay".as_ptr(),
            value: c"Timing / FPS Overlay; disabled|auto|11|12|13|14".as_ptr(),
        },
        ffi::Variable {
            key: ptr::null(),
            value: ptr::null(),
        },
    ];
    let terminator = legacy.pop().unwrap();
    legacy.push(ffi::Variable {
        key: AUDIO_DONOR_KEY.as_ptr(),
        value: c"Sega NetMerc Audio Donor (Restart Required); vf|vr|swa|wingwar|off".as_ptr(),
    });
    legacy.push(ffi::Variable {
        key: c"tgpulse_next_netmerc_city_workaround".as_ptr(),
        value: c"Sega NetMerc City Workaround; enabled|disabled".as_ptr(),
    });
    for (key, value) in [
        (
            c"tgpulse_next_renderer",
            c"Renderer (Restart Required); auto|vulkan|opengl|software",
        ),
        (
            c"tgpulse_next_widescreen_mode",
            c"Widescreen Hack (Restart Required); expand_3d|expand_3d_2d|stretch",
        ),
        (
            c"tgpulse_next_supersampling",
            c"Supersampling (Restart Required); 1|2|3|4",
        ),
    ] {
        legacy.push(ffi::Variable {
            key: key.as_ptr(),
            value: value.as_ptr(),
        });
    }

    legacy.push(ffi::Variable {
        key: c"tgpulse_next_steering_response".as_ptr(),
        value: c"Driving Steering Response; linear|progressive|fbneo".as_ptr(),
    });
    for (key, value) in DRIVING_RANGE_KEYS.into_iter().zip([
        c"Driving Steering Output Range; 100|50|60|70|80|90|110|120|130|140|150",
        c"Driving Accelerator Output Range; 100|50|60|70|80|90|110|120|130|140|150",
        c"Driving Brake Output Range; 100|50|60|70|80|90|110|120|130|140|150",
    ]) {
        legacy.push(ffi::Variable {
            key: key.as_ptr(),
            value: value.as_ptr(),
        });
    }
    for (key, value) in mvd::KEYS.into_iter().zip([
        c"Sega NetMerc MVD Input; auto|off|right_stick|sensors",
        c"Sega NetMerc MVD Horizontal Range; 30|0|10|20|40|50|60|70|80|90",
        c"Sega NetMerc MVD Vertical Range; 20|0|10|30|40|50|60|70|80|90",
        c"Sega NetMerc MVD Gravity Stabilization; enabled|disabled",
        c"Sega NetMerc MVD Drift Compensation; 50|off|10|20|30|40|60|70|80|90|100",
        c"Sega NetMerc MVD Sensor Diagnostics; disabled|enabled",
    ]) {
        legacy.push(ffi::Variable { key: key.as_ptr(), value: value.as_ptr() });
    }
    for (key, value) in diagnostic_overlay::KEYS.into_iter().zip([
        c"Sega NetMerc Diagnostic Display; off|overlay|overlay_half",
        c"Sega NetMerc Diagnostic Position; top_right|top_left|bottom_right|bottom_left",
        c"Sega NetMerc Diagnostic Background Opacity; 80|0|10|20|30|40|50|60|70|90|100",
    ]) {
        legacy.push(ffi::Variable { key: key.as_ptr(), value: value.as_ptr() });
    }
    for (_, key, max) in netpacket::GAMES {
        let value = if max == 2 {
            c"Linked Cabinets (Restart Required); disabled|2"
        } else {
            c"Linked Cabinets (Restart Required); disabled|2|3|4|5|6|7|8|9"
        };
        legacy.push(ffi::Variable {
            key: key.as_ptr(),
            value: value.as_ptr(),
        });
    }
    for (key, value) in AUDIO_GAIN_KEYS.into_iter().zip([
        c"MultiPCM 1 Gain; auto|0|10|20|30|40|50|60|70|80|90|100|Mute",
        c"MultiPCM 2 Gain; auto|0|10|20|30|40|50|60|70|80|90|100|Mute",
        c"FM (YM3438) Gain; auto|0|10|20|30|40|50|60|70|80|90|100|Mute",
        c"DSB (MPEG) Gain; auto|0|10|20|30|40|50|60|70|80|90|100|Mute",
    ]) {
        legacy.push(ffi::Variable {
            key: key.as_ptr(),
            value: value.as_ptr(),
        });
    }
    legacy.push(ffi::Variable {
        key: c"tgpulse_next_initial_nvram_setup".as_ptr(),
        value: c"Automatic Initial NVRAM Setup (Restart Required); enabled|disabled".as_ptr(),
    });
    legacy.push(ffi::Variable {
        key: c"tgpulse_next_nvram_settings".as_ptr(),
        value: c"NVRAM Settings; disabled|enabled".as_ptr(),
    });
    static NVRAM_LEGACY: OnceLock<Vec<CString>> = OnceLock::new();
    let values = NVRAM_LEGACY.get_or_init(|| {
        nvram::FIELDS
            .iter()
            .map(|field| {
                let mut values = vec![field.values[field.default].key.to_str().unwrap()];
                values.extend(
                    field
                        .values
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != field.default)
                        .map(|(_, v)| v.key.to_str().unwrap()),
                );
                CString::new(format!(
                    "{}; {}",
                    field.label.to_str().unwrap(),
                    values.join("|")
                ))
                .unwrap()
            })
            .collect()
    });
    for (field, value) in nvram::FIELDS.iter().zip(values) {
        legacy.push(ffi::Variable {
            key: field.key.as_ptr(),
            value: value.as_ptr(),
        });
    }
    legacy.push(terminator);
    legacy.sort_by_key(|variable| menu_order(variable.key));
    unsafe { environment(ffi::SET_VARIABLES, legacy.as_mut_ptr().cast()) };
}

type Descriptor = (u32, u32, u32, u32, &'static CStr);

fn descriptors(profile: Profile) -> Vec<Descriptor> {
    use ffi::*;
    let mut entries = vec![
        (0, DEVICE_JOYPAD, 0, JOY_SELECT, c"Coin"),
        (0, DEVICE_JOYPAD, 0, JOY_L3, c"Test"),
        (0, DEVICE_JOYPAD, 0, JOY_R3, c"Service"),
    ];
    if profile != Profile::NetMerc {
        entries.push((0, DEVICE_JOYPAD, 0, JOY_START, c"Start"));
    }
    entries.extend([
        (1, DEVICE_JOYPAD, 0, JOY_L3, c"Test"),
        (1, DEVICE_JOYPAD, 0, JOY_R3, c"Service"),
    ]);
    if !matches!(profile, Profile::WingWar | Profile::NetMerc) {
        entries.push((1, DEVICE_JOYPAD, 0, JOY_SELECT, c"Coin"));
    }
    match profile {
        Profile::VirtuaRacing | Profile::VirtuaFormula => entries.extend([
            (0, DEVICE_JOYPAD, 0, JOY_DOWN, c"VR1 (Red)"),
            (0, DEVICE_JOYPAD, 0, JOY_LEFT, c"VR2 (Blue)"),
            (0, DEVICE_JOYPAD, 0, JOY_RIGHT, c"VR3 (Yellow)"),
            (0, DEVICE_JOYPAD, 0, JOY_UP, c"VR4 (Green)"),
            (0, DEVICE_JOYPAD, 0, JOY_L, c"Gear Down"),
            (0, DEVICE_JOYPAD, 0, JOY_R, c"Gear Up"),
            (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_X, c"Steering"),
            (0, DEVICE_ANALOG, ANALOG_BUTTON, JOY_R2, c"Accelerator"),
            (0, DEVICE_ANALOG, ANALOG_BUTTON, JOY_L2, c"Brake"),
        ]),
        Profile::VirtuaFighter => {
            for port in 0..=1 {
                if port == 1 {
                    entries.extend([(port, DEVICE_JOYPAD, 0, JOY_START, c"Start")]);
                }
                entries.extend([
                    (port, DEVICE_JOYPAD, 0, JOY_UP, c"Up"),
                    (port, DEVICE_JOYPAD, 0, JOY_DOWN, c"Down"),
                    (port, DEVICE_JOYPAD, 0, JOY_LEFT, c"Left"),
                    (port, DEVICE_JOYPAD, 0, JOY_RIGHT, c"Right"),
                    (port, DEVICE_JOYPAD, 0, JOY_B, c"Kick"),
                    (port, DEVICE_JOYPAD, 0, JOY_A, c"Punch"),
                    (port, DEVICE_JOYPAD, 0, JOY_Y, c"Guard"),
                ]);
            }
        }
        Profile::WingWar | Profile::WingWar360 => {
            entries.extend([
                (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_X, c"Stick X"),
                (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_Y, c"Stick Y"),
                (
                    0,
                    DEVICE_ANALOG,
                    ANALOG_RIGHT,
                    ANALOG_Y,
                    c"Throttle Up / Down",
                ),
                (0, DEVICE_ANALOG, ANALOG_BUTTON, JOY_L2, c"Throttle Down"),
                (0, DEVICE_ANALOG, ANALOG_BUTTON, JOY_R2, c"Throttle Up"),
                (0, DEVICE_JOYPAD, 0, JOY_B, c"Machine Gun"),
                (0, DEVICE_JOYPAD, 0, JOY_L, c"Machine Gun"),
                (0, DEVICE_JOYPAD, 0, JOY_A, c"Missile"),
                (0, DEVICE_JOYPAD, 0, JOY_R, c"Missile"),
                (0, DEVICE_JOYPAD, 0, JOY_Y, c"Smoke"),
            ]);
            if profile == Profile::WingWar {
                entries.extend([
                    (0, DEVICE_JOYPAD, 0, JOY_DOWN, c"View 1"),
                    (0, DEVICE_JOYPAD, 0, JOY_LEFT, c"View 2"),
                    (0, DEVICE_JOYPAD, 0, JOY_RIGHT, c"View 3"),
                    (0, DEVICE_JOYPAD, 0, JOY_UP, c"View 4"),
                ]);
            }
        }
        Profile::StarWarsArcade => entries.extend([
            (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_X, c"Pilot Stick X"),
            (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_Y, c"Pilot Stick Y"),
            (
                0,
                DEVICE_ANALOG,
                ANALOG_RIGHT,
                ANALOG_Y,
                c"Throttle Up / Down",
            ),
            (
                0,
                DEVICE_ANALOG,
                ANALOG_BUTTON,
                JOY_L2,
                c"Pilot Throttle Up",
            ),
            (
                0,
                DEVICE_ANALOG,
                ANALOG_BUTTON,
                JOY_R2,
                c"Pilot Throttle Down",
            ),
            (0, DEVICE_JOYPAD, 0, JOY_B, c"Pilot Laser"),
            (0, DEVICE_JOYPAD, 0, JOY_L, c"Pilot Laser"),
            (0, DEVICE_JOYPAD, 0, JOY_A, c"Pilot Torpedo"),
            (0, DEVICE_JOYPAD, 0, JOY_R, c"Pilot Torpedo"),
            (0, DEVICE_JOYPAD, 0, JOY_DOWN, c"VR1"),
            (0, DEVICE_JOYPAD, 0, JOY_UP, c"VR1"),
            (1, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_X, c"Gunner Stick X"),
            (1, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_Y, c"Gunner Stick Y"),
            (1, DEVICE_JOYPAD, 0, JOY_B, c"Gunner Laser"),
            (1, DEVICE_JOYPAD, 0, JOY_L, c"Gunner Laser"),
            (1, DEVICE_JOYPAD, 0, JOY_A, c"Gunner Torpedo"),
            (1, DEVICE_JOYPAD, 0, JOY_R, c"Gunner Torpedo"),
        ]),
        Profile::NetMerc => entries.extend([
            (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_X, c"Stick X"),
            (0, DEVICE_ANALOG, ANALOG_LEFT, ANALOG_Y, c"Stick Y"),
            (0, DEVICE_ANALOG, ANALOG_RIGHT, ANALOG_X, c"MVD Look X"),
            (0, DEVICE_ANALOG, ANALOG_RIGHT, ANALOG_Y, c"MVD Look Y"),
            (0, DEVICE_JOYPAD, 0, JOY_B, c"Trigger Button"),
            (0, DEVICE_JOYPAD, 0, JOY_L2, c"Trigger Button"),
            (0, DEVICE_JOYPAD, 0, JOY_R2, c"Trigger Button"),
            (0, DEVICE_JOYPAD, 0, JOY_A, c"Thumb Button"),
            (0, DEVICE_JOYPAD, 0, JOY_L, c"Thumb Button"),
            (0, DEVICE_JOYPAD, 0, JOY_R, c"Thumb Button"),
            (0, DEVICE_JOYPAD, 0, JOY_START, c"MVD Holder"),
            (0, DEVICE_JOYPAD, 0, JOY_DOWN, c"MVD Holder"),
            (0, DEVICE_JOYPAD, 0, JOY_X, c"MVD Calibrate"),
            (0, DEVICE_JOYPAD, 0, JOY_Y, c"MVD Recenter"),
        ]),
    }
    entries
}

fn register_descriptors(environment: ffi::Environment, profile: Profile, devices: [u32; 2]) {
    let mut descriptions: Vec<ffi::InputDescriptor> = descriptors(profile)
        .iter()
        .filter(|(port, device, _, id, _)| {
            let selected = devices[*port as usize];
            joypad_enabled(selected) && (*device != ffi::DEVICE_JOYPAD || cabinet_button_enabled(selected, *id))
        })
        .map(
            |(port, device, index, id, description)| ffi::InputDescriptor {
                port: *port,
                device: *device,
                index: *index,
                id: *id,
                description: description.as_ptr(),
            },
        )
        .collect();
    descriptions.push(ffi::InputDescriptor {
        port: 0,
        device: 0,
        index: 0,
        id: 0,
        description: ptr::null(),
    });
    unsafe { environment(ffi::SET_INPUT_DESCRIPTORS, descriptions.as_mut_ptr().cast()) };
}

fn clear_controls(environment: Option<ffi::Environment>) {
    if let Some(environment) = environment {
        let mut descriptions = [ffi::InputDescriptor {
            port: 0,
            device: 0,
            index: 0,
            id: 0,
            description: ptr::null(),
        }];
        let mut controllers = [ffi::ControllerInfo {
            types: ptr::null(),
            num_types: 0,
        }];
        unsafe {
            environment(ffi::SET_INPUT_DESCRIPTORS, descriptions.as_mut_ptr().cast());
            environment(ffi::SET_CONTROLLER_INFO, controllers.as_mut_ptr().cast());
        }
    }
}

fn press(value: i16) -> bool {
    value != 0
}

fn approach(current: i32, target: i32, delta: i32) -> i32 {
    current + (target - current).clamp(-delta, delta)
}

fn vr_inputs(
    steering: &mut i32,
    accelerator: &mut i32,
    brake: &mut i32,
    input: Option<ffi::InputState>,
    devices: [u32; 2],
) -> Inputs {
    let Some(input) = input else {
        return Inputs::default();
    };
    let read = |device, index, id| {
        if joypad_enabled(devices[0])
            && (device != ffi::DEVICE_JOYPAD || cabinet_button_enabled(devices[0], id)) {
            unsafe { input(0, device, index, id) }
        } else {
            0
        }
    };
    let p2 = |id| {
        cabinet_button_enabled(devices[1], id) && press(unsafe { input(1, ffi::DEVICE_JOYPAD, 0, id) })
    };
    let button = |id| press(read(ffi::DEVICE_JOYPAD, 0, id));
    let raw_steer = read(ffi::DEVICE_ANALOG, ffi::ANALOG_LEFT, ffi::ANALOG_X) as f32 / 32768.0;
    let steer = if raw_steer.abs() > 0.15 {
        raw_steer.signum() * (raw_steer.abs() - 0.15) / 0.85
    } else {
        0.0
    };
    *steering = if steer != 0.0 {
        (128.0 + steer * 96.0).round() as i32
    } else {
        approach(*steering, 0x80, 10)
    };
    let pedal = |id, current| {
        let analog = read(ffi::DEVICE_ANALOG, ffi::ANALOG_BUTTON, id).max(0) as f32 / 32767.0;
        if analog > 0.05 {
            (0x20 as f32 + analog * 0xc0 as f32).round() as i32
        } else {
            approach(current, if button(id) { 0xe0 } else { 0x20 }, 20)
        }
    };
    *accelerator = pedal(ffi::JOY_R2, *accelerator).clamp(0x20, 0xe0);
    *brake = pedal(ffi::JOY_L2, *brake).clamp(0x20, 0xe0);
    VrCabinet {
        coin1: button(ffi::JOY_SELECT),
        coin2: p2(ffi::JOY_SELECT),
        test: button(ffi::JOY_L3) || p2(ffi::JOY_L3),
        service: button(ffi::JOY_R3) || p2(ffi::JOY_R3),
        start: button(ffi::JOY_START),
        views: [
            button(ffi::JOY_DOWN),
            button(ffi::JOY_LEFT),
            button(ffi::JOY_RIGHT),
            button(ffi::JOY_UP),
        ],
        gear_up: button(ffi::JOY_R),
        gear_down: button(ffi::JOY_L),
        steer: (*steering).clamp(0x20, 0xe0) as u8,
        accel: *accelerator as u8,
        brake: *brake as u8,
        ..VrCabinet::default()
    }
    .into_native()
}

fn button(input: ffi::InputState, port: u32, id: u32) -> bool {
    press(unsafe { input(port, ffi::DEVICE_JOYPAD, 0, id) })
}

// Normalized frontend input, without adding another dead zone.
fn normalized_axis(input: ffi::InputState, port: u32, index: u32, axis: u32) -> f32 {
    let raw = unsafe { input(port, ffi::DEVICE_ANALOG, index, axis) };
    raw as f32 / if raw < 0 { 32768.0 } else { 32767.0 }
}

// Preserve the previously approved policy for the other flight profiles.
fn flight_axis(input: ffi::InputState, port: u32, index: u32, axis: u32) -> f32 {
    let value = normalized_axis(input, port, index, axis);
    if value.abs() <= 0.15 {
        0.0
    } else {
        value
    }
}

fn trigger(input: ffi::InputState, port: u32, id: u32) -> f32 {
    let value =
        unsafe { input(port, ffi::DEVICE_ANALOG, ffi::ANALOG_BUTTON, id) }.max(0) as f32 / 32767.0;
    if value > 0.05 {
        value
    } else if button(input, port, id) {
        1.0
    } else {
        0.0
    }
}

fn vf_inputs(input: Option<ffi::InputState>, devices: [u32; 2]) -> Inputs {
    let Some(input) = input else {
        return VfCabinet::default().into_native();
    };
    let player = |port| {
        if !joypad_enabled(devices[port as usize]) {
            return VfPlayer::default();
        }
        VfPlayer {
            coin: button(input, port, ffi::JOY_SELECT),
            start: button(input, port, ffi::JOY_START),
            left: button(input, port, ffi::JOY_LEFT),
            right: button(input, port, ffi::JOY_RIGHT),
            up: button(input, port, ffi::JOY_UP),
            down: button(input, port, ffi::JOY_DOWN),
            actions: [
                button(input, port, ffi::JOY_B),
                button(input, port, ffi::JOY_A),
                button(input, port, ffi::JOY_Y),
            ],
        }
    };
    VfCabinet {
        players: [player(0), player(1)],
        test: (devices[0] == ffi::DEVICE_JOYPAD && button(input, 0, ffi::JOY_L3))
            || (devices[1] == ffi::DEVICE_JOYPAD && button(input, 1, ffi::JOY_L3)),
        service: (devices[0] == ffi::DEVICE_JOYPAD && button(input, 0, ffi::JOY_R3))
            || (devices[1] == ffi::DEVICE_JOYPAD && button(input, 1, ffi::JOY_R3)),
    }
    .into_native()
}

fn flight_inputs(profile: Profile, input: Option<ffi::InputState>, devices: [u32; 2]) -> Inputs {
    let kind = match profile {
        Profile::WingWar => FlightKind::WingWar,
        Profile::WingWar360 => FlightKind::WingWar360,
        Profile::StarWarsArcade => FlightKind::StarWars,
        Profile::NetMerc => FlightKind::NetMerc,
        _ => unreachable!(),
    };
    let b = |port: u32, id| {
        cabinet_button_enabled(devices[port as usize], id)
            && input.is_some_and(|input| button(input, port, id))
    };
    let a = |port: u32, axis| {
        if joypad_enabled(devices[port as usize]) {
            input.map_or(0.0, |input| {
                if profile == Profile::NetMerc {
                    normalized_axis(input, port, ffi::ANALOG_LEFT, axis)
                } else {
                    flight_axis(input, port, ffi::ANALOG_LEFT, axis)
                }
            })
        } else {
            0.0
        }
    };
    let t = |id| {
        if joypad_enabled(devices[0]) {
            input.map_or(0.0, |input| trigger(input, 0, id))
        } else {
            0.0
        }
    };
    let right_y = if joypad_enabled(devices[0]) {
        input.map_or(0.0, |input| {
            flight_axis(input, 0, ffi::ANALOG_RIGHT, ffi::ANALOG_Y)
        })
    } else {
        0.0
    };
    // Wing War keeps each trigger's previous ADC response while reversing the
    // right stick's response. SWA retains the standalone half-axis bindings.
    let throttle = if matches!(profile, Profile::WingWar | Profile::WingWar360) {
        t(ffi::JOY_R2).max((-right_y).max(0.0))
            - t(ffi::JOY_L2).max(right_y.max(0.0))
    } else {
        t(ffi::JOY_L2).max((-right_y).max(0.0))
            - t(ffi::JOY_R2).max(right_y.max(0.0))
    };
    FlightCabinet {
        kind,
        coin1: b(0, ffi::JOY_SELECT),
        coin2: b(1, ffi::JOY_SELECT),
        test: b(0, ffi::JOY_L3) || b(1, ffi::JOY_L3),
        service: b(0, ffi::JOY_R3) || b(1, ffi::JOY_R3),
        start: b(0, ffi::JOY_START),
        weapons: if profile == Profile::NetMerc {
            [b(0, ffi::JOY_B) || t(ffi::JOY_L2) > 0.5 || t(ffi::JOY_R2) > 0.5,
             b(0, ffi::JOY_A) || b(0, ffi::JOY_L) || b(0, ffi::JOY_R),
             b(0, ffi::JOY_START) || b(0, ffi::JOY_DOWN)]
        } else {
            [b(0, ffi::JOY_B) || b(0, ffi::JOY_L),
             b(0, ffi::JOY_A) || b(0, ffi::JOY_R),
             b(0, ffi::JOY_Y)]
        },
        views: if profile == Profile::StarWarsArcade {
            [
                b(0, ffi::JOY_DOWN) || b(0, ffi::JOY_UP),
                false,
                false,
                false,
            ]
        } else {
            [
                b(0, ffi::JOY_DOWN),
                b(0, ffi::JOY_LEFT),
                b(0, ffi::JOY_RIGHT),
                b(0, ffi::JOY_UP),
            ]
        },
        // Libretro stick Y is positive down; cabinet actions use positive up.
        pilot: [a(0, ffi::ANALOG_X), -a(0, ffi::ANALOG_Y)],
        throttle: throttle.clamp(-1.0, 1.0),
        gunner: [a(1, ffi::ANALOG_X), -a(1, ffi::ANALOG_Y)],
        gunner_weapons: [
            b(1, ffi::JOY_B) || b(1, ffi::JOY_L),
            b(1, ffi::JOY_A) || b(1, ffi::JOY_R),
        ],
    }
    .into_native()
}

fn mvd_input(input: Option<ffi::InputState>, device: u32, settings: mvd::Settings)
    -> (tgpulse_core::model1io2::HmdPose, [bool; 2]) {
    let available = joypad_enabled(device) && input.is_some();
    let axes = if available {
        let input = input.unwrap();
        [normalized_axis(input, 0, ffi::ANALOG_RIGHT, ffi::ANALOG_X),
         normalized_axis(input, 0, ffi::ANALOG_RIGHT, ffi::ANALOG_Y)]
    } else { [0.0; 2] };
    let commands = if available {
        let input = input.unwrap();
        [button(input, 0, ffi::JOY_X), button(input, 0, ffi::JOY_Y)]
    } else { [false; 2] };
    (mvd::pose(settings, available, axes), commands)
}


fn load_game(core: &mut Core, info: *const ffi::GameInfo) -> Result<Game, String> {
    let environment = core
        .environment
        .ok_or("Frontend environment callback is missing")?;
    if info.is_null() {
        return Err("A Model 1 ZIP path is required".into());
    }
    let info = unsafe { &*info };
    if info.path.is_null() {
        return Err("A Model 1 ZIP path is required".into());
    }
    let path = unsafe { CStr::from_ptr(info.path) }
        .to_str()
        .map_err(|_| "The ROM path is not valid UTF-8")?;
    if path.is_empty() {
        return Err("A Model 1 ZIP path is required".into());
    }
    let mut pixel_format = ffi::PIXEL_FORMAT_XRGB8888;
    if !unsafe {
        environment(
            ffi::SET_PIXEL_FORMAT,
            (&mut pixel_format as *mut u32).cast(),
        )
    } {
        return Err("Frontend does not support XRGB8888 video".into());
    }
    let names = loader::archive_names(path)?;
    let definition = roms_db::identify_complete_with_external_io(&names)
        .ok_or("ZIP does not contain a complete recognized ROM set")?;
    if !definition.board.is_model1() {
        return Err(format!(
            "{} is a Model 2 game; this core currently supports Model 1 only",
            definition.name
        ));
    }
    let profile = Profile::for_game(definition).ok_or_else(|| {
        format!(
            "{} has no verified Model 1 Libretro cabinet profile",
            definition.name
        )
    })?;
    let mut graphics = graphics_settings(environment);
    graphics.wide = widescreen_hack_mode(&definition.name, graphics.wide);
    let hardware = request_graphics(core, graphics, environment)?;
    let bios = bios_directory(environment);
    let mut roms = loader::load_model1_zip_with_bios(path, core.settings.apply_known_rom_repairs, bios.as_deref())?;
    let donor = audio_donor_option(environment);
    let audio_notice = apply_audio_donor(&mut roms, path, donor);
    let factory_nvram_loaded = roms.nvram_default.len() == 65536
        && names.iter().any(|name| name.rsplit('/').next().unwrap_or(name)
            .eq_ignore_ascii_case("netmerc_nvram.bin"));
    update_audio_settings(&mut core.settings, environment);
    let mut config = Config {
        rom_path: path.to_owned(),
        system: System::Model1,
        ..Config::default()
    };
    let linked = netpacket::GAMES
        .iter()
        .find(|(set, _, _)| *set == definition.name)
        .and_then(|(_, key, max)| {
            option_value(environment, key)
                .and_then(|v| v.parse::<u16>().ok())
                .filter(|n| (2..=*max).contains(n))
        })
        .unwrap_or(1);
    netpacket::configure(&definition.name, linked);
    if linked > 1 {
        config.cabinet = tgpulse_core::config::Cabinet::Twin;
        if !netpacket::supported() {
            notify(
                Some(environment),
                "Linked Cabinets enabled, but frontend has no Netpacket support",
            );
        }
    }
    config.smooth_shadows = core.settings.smooth_shadows;
    config.netmerc_city_workaround = core.settings.netmerc_city_workaround;
    config.audio_mutes = core.settings.audio_mutes;
    config.audio_gains = core.settings.audio_gains;
    config.netmerc_audio_donor = donor;
    let mut machine = Box::new(
        Model1System::with_config(&roms, config.clone()).map_err(|error| error.to_string())?,
    );
    machine.sound.set_gains(core.settings.audio_gains);
    let initial_state = if linked > 1 {
        Vec::new()
    } else {
        machine.save_state()?
    };
    let linked_reset = if linked > 1 {
        Some((roms, config))
    } else {
        None
    };
    let mut controllers = ControllerStorage::new(profile);
    controllers.publish(environment);
    register_descriptors(environment, profile, core.devices);
    publish_nvram_option_visibility(environment, &definition.name, core.settings.nvram_settings);
    let game = Game {
        graphics,
        hardware,
        set_name: definition.name.clone(),
        profile,
        controllers,
        machine,
        initial_state,
        linked_reset,
        link_status: None,
        linked_online_frames: 0,
        factory_nvram_loaded,
        audio_notice,
        nvram_pending: core.settings.nvram_settings,
        nvram_wait_frames: 0,
        pixels: vec![0; WIDTH * HEIGHT],
        foreground: vec![0; WIDTH * HEIGHT],
        panel_foreground: Vec::new(),
        audio: Vec::with_capacity(4096),
        steering: 0x80,
        accelerator: 0x20,
        brake: 0x20,
        fast_60hz: core.settings.fast_60hz,
        audio_remainder: 0,
        output_audio: Vec::with_capacity(4096),
        reported_aspect_ratio: None,
        reported_width: 0,
        rumble: PadRumble::new(environment),
        timing: timing_overlay::Timing::default(),
        panel: timing_overlay::Panel::default(),
        diagnostic: diagnostic_overlay::Display::new(std::path::PathBuf::from(path), bios),
        mvd_commands: mvd::Commands::default(),
        mvd_sensors: sensors::Sensors::new((profile == Profile::NetMerc).then_some(environment)),
        mvd_holder_notice: mvd::HolderNotice::default(),
    };
    publish_global_option_visibility(environment, game.machine.sound.sources());
    Ok(game)
}

fn render(game: &mut Game) {
    tilemap::render_background(&*game.machine, &mut game.pixels);
    model1_video::render_below_hud(&mut game.machine, &mut game.pixels);
    tilemap::render_foreground(&*game.machine, &mut game.foreground);
    for (pixel, &overlay) in game.pixels.iter_mut().zip(&game.foreground) {
        if overlay != 0 {
            *pixel = overlay;
        }
    }
}

fn collect_audio(game: &mut Game, volume: u32) {
    game.audio.clear();
    for (left, right) in game.machine.sound.samples.drain(..) {
        for sample in [left, right] {
            let scaled = i32::from(sample) * volume as i32 / 100;
            game.audio
                .push(scaled.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
        }
    }
}

fn resample_fast_audio(game: &mut Game) {
    resample_stereo_60hz(
        &game.audio,
        &mut game.output_audio,
        &mut game.audio_remainder,
    );
}

fn resample_stereo_60hz(input: &[i16], output: &mut Vec<i16>, remainder: &mut u64) {
    *remainder += 10_000_000;
    let output_frames = (*remainder / FAST_AUDIO_DENOMINATOR) as usize;
    *remainder %= FAST_AUDIO_DENOMINATOR;
    let input_frames = input.len() / 2;
    output.clear();
    output.resize(output_frames * 2, 0);
    if input_frames == 0 {
        return;
    }
    let denominator = output_frames.saturating_sub(1).max(1);
    for frame in 0..output_frames {
        let position = frame * input_frames.saturating_sub(1);
        let left = position / denominator;
        let right = (left + 1).min(input_frames - 1);
        let fraction = (position % denominator) as i64;
        for channel in 0..2 {
            let a = i64::from(input[left * 2 + channel]);
            let b = i64::from(input[right * 2 + channel]);
            output[frame * 2 + channel] =
                ((a * (denominator as i64 - fraction) + b * fraction) / denominator as i64) as i16;
        }
    }
}

fn reset_machine(game: &mut Game) -> Result<(), String> {
    game.mvd_sensors.stop();
    game.mvd_commands.resync();
    if let Some((roms, config)) = &game.linked_reset {
        let config = config.clone();
        game.machine =
            Box::new(Model1System::with_config(roms, config).map_err(|e| e.to_string())?);
        netpacket::clear_frames();
        game.link_status = None;
        game.linked_online_frames = 0;
        Ok(())
    } else {
        game.machine.load_state(&game.initial_state)
    }
}

fn frontend_timing(sample_rate: f64, fast_60hz: bool) -> ffi::Timing {
    ffi::Timing {
        fps: if fast_60hz { 60.0 } else { FPS },
        sample_rate,
    }
}

fn import_save_ram(core: &mut Core) {
    if core.save_imported {
        return;
    }
    let Some(game) = core.game.as_mut() else {
        return;
    };
    let loaded = match persistence::import_save(&game.set_name, &core.save_ram) {
        Ok(Some((backup, eeprom))) => {
            game.machine.set_nvram_blocks(backup, eeprom);
            true
        }
        Ok(None) => false,
        Err(error) => {
            notify(core.environment, error);
            false
        }
    };
    // Repair the optional seed only after save precedence is known; reset,
    // explicit operator overrides and state restore never invoke this repair.
    if !loaded && game.factory_nvram_loaded {
        let (mut backup, eeprom) = game.machine.nvram_blocks();
        if loader::initialize_netmerc_nvram_seed(&mut backup) {
            game.machine.set_nvram_blocks(&backup, &eeprom);
        }
    }
    if !loaded && core.settings.automatic_nvram {
        let (mut backup, eeprom) = game.machine.nvram_blocks();
        match nvram::initial_backup(&game.set_name, &mut backup, game.factory_nvram_loaded) {
            Ok(true) => {
                game.machine.set_nvram_blocks(&backup, &eeprom);
                notify(core.environment, "Applied automatic initial NVRAM setup");
            }
            Ok(false) => {}
            Err(error) => notify(core.environment, error),
        }
        match nvram::seed(&game.set_name) {
            Ok(Some((backup, eeprom))) => {
                game.machine.set_nvram_blocks(&backup, &eeprom);
                notify(core.environment, "Applied automatic initial NVRAM setup");
            }
            Ok(None) => {}
            Err(error) => notify(core.environment, error),
        }
    }
    core.save_imported = true;
    export_save_ram(core);
}

fn apply_pending_nvram(game: &mut Game, environment: Option<ffi::Environment>, enabled: bool) {
    if !game.nvram_pending {
        return;
    }
    if !enabled {
        game.nvram_pending = false;
        return;
    }
    let Some(env) = environment else {
        return;
    };
    let fields: Vec<_> = nvram::FIELDS
        .iter()
        .filter(|f| f.set == game.set_name)
        .collect();
    let choices: Vec<_> = fields
        .iter()
        .map(|field| {
            option_value(env, field.key)
                .and_then(|v| {
                    field
                        .values
                        .iter()
                        .position(|choice| choice.key.to_bytes() == v.as_bytes())
                })
                .unwrap_or(field.default)
        })
        .collect();
    let (mut backup, mut eeprom) = game.machine.nvram_blocks();
    match nvram::apply_settings(&game.set_name, &mut backup, &mut eeprom, &choices) {
        nvram::Apply::NotReady => {
            game.nvram_wait_frames += 1;
            if game.nvram_wait_frames >= 1200 {
                game.nvram_pending = false;
                notify(
                    environment,
                    "NVRAM layout not ready; operator options were not applied",
                );
            }
        }
        nvram::Apply::Unchanged => game.nvram_pending = false,
        nvram::Apply::Changed => {
            game.nvram_pending = false;
            // The reference ports reset after persisted changes so firmware
            // reloads its settings. Preserve backup, unrelated fields and calibration.
            if let Err(error) = reset_machine(game) {
                notify(
                    environment,
                    &format!("NVRAM settings reset failed: {error}"),
                );
                return;
            }
            game.mvd_commands.resync();
            game.machine.set_nvram_blocks(&backup, &eeprom);
            game.rumble.stop();
            game.timing.reset();
            game.machine.sound.samples.clear();
            game.audio.clear();
            game.output_audio.clear();
            game.audio_remainder = 0;
            game.steering = 0x80;
            game.accelerator = 0x20;
            game.brake = 0x20;
            notify(environment, "Applied NVRAM Settings; machine reset");
        }
    }
}

fn export_save_ram(core: &mut Core) {
    let Some(game) = core.game.as_ref() else {
        return;
    };
    let (backup, eeprom) = game.machine.nvram_blocks();
    if !persistence::export_save(&game.set_name, &backup, &eeprom, &mut core.save_ram) {
        notify(core.environment, "Model 1 Save RAM layout mismatch");
    }
}

#[no_mangle]
pub extern "C" fn retro_api_version() -> u32 {
    ffi::API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_set_environment(callback: Option<ffi::Environment>) {
    with_core(|core| core.environment = callback);
    if let Some(environment) = callback {
        register_options(environment);
        netpacket::register(environment);
        let mut display_callback = ffi::OptionUpdateDisplay {
            callback: Some(update_nvram_option_display),
        };
        unsafe {
            environment(
                ffi::SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK,
                (&mut display_callback as *mut ffi::OptionUpdateDisplay).cast(),
            );
        }

        publish_nvram_option_visibility(environment, "", false);
        publish_global_option_visibility(environment, &[]);
    }
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(callback: Option<ffi::Video>) {
    with_core(|core| core.video = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(callback: Option<ffi::AudioSample>) {
    with_core(|core| core.audio_sample = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(callback: Option<ffi::AudioBatch>) {
    with_core(|core| core.audio_batch = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(callback: Option<ffi::InputPoll>) {
    with_core(|core| core.input_poll = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(callback: Option<ffi::InputState>) {
    with_core(|core| core.input_state = callback);
}

#[no_mangle]
pub extern "C" fn retro_init() {
    eprintln!("[TGPulse-Next Libretro] {}", tgpulse_core::MACHINE_SCOPE);
    with_core(|core| {
        core.devices = [ffi::DEVICE_JOYPAD; 2];
        core.save_ram = vec![0; persistence::SAVE_BYTES].into_boxed_slice();
        core.save_imported = false;
        update_settings(core);
    });
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    netpacket::shutdown();
    with_core(|core| {
        if let Some(game) = core.game.as_mut() {
            game.rumble.stop();
        }
        clear_controls(core.environment);
        core.game = None;
    });
}

#[no_mangle]
pub extern "C" fn retro_get_system_info(info: *mut ffi::SystemInfo) {
    if info.is_null() {
        return;
    }
    unsafe {
        *info = ffi::SystemInfo {
            library_name: c"TGPulse-Next".as_ptr(),
            library_version: c"0.1.0.3".as_ptr(),
            valid_extensions: c"zip".as_ptr(),
            need_fullpath: true,
            block_extract: true,
        };
    }
}

#[no_mangle]
pub extern "C" fn retro_get_system_av_info(info: *mut ffi::AvInfo) {
    if info.is_null() {
        return;
    }
    let (rate, fast_60hz, aspect_ratio, base_width) = with_core(|core| {
        core.game.as_mut().map_or(
            (
                DEFAULT_AUDIO_RATE,
                core.settings.fast_60hz,
                ASPECT_4_3,
                WIDTH as u32,
            ),
            |game| {
                let aspect_ratio = game_aspect_ratio(game, core.settings.aspect_ratio);
                game.reported_aspect_ratio = Some(aspect_ratio);
                game.reported_width = render_width(game, core.settings.aspect_ratio);
                (
                    game.machine.sound.sample_rate() as f64,
                    game.fast_60hz,
                    aspect_ratio,
                    game.reported_width,
                )
            },
        )
    });
    unsafe {
        *info = ffi::AvInfo {
            geometry: ffi::Geometry {
                base_width,
                base_height: HEIGHT as u32,
                max_width: 683,
                max_height: HEIGHT as u32,
                aspect_ratio,
            },
            timing: frontend_timing(rate, fast_60hz),
        };
    }
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(port: u32, device: u32) {
    with_core(|core| {
        if let Some(slot) = core.devices.get_mut(port as usize) {
            *slot = if joypad_enabled(device) {
                device
            } else {
                ffi::DEVICE_NONE
            };
            if port == 0 {
                if let Some(game) = core.game.as_mut() {
                    game.mvd_commands.resync();
                    game.mvd_sensors.stop();
                    game.rumble.stop();
                }
            }
            if let (Some(game), Some(environment)) = (core.game.as_mut(), core.environment) {
                game.controllers.publish(environment);
                register_descriptors(environment, game.profile, core.devices);
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    with_core(|core| {
        import_save_ram(core);
        if let Some(game) = core.game.as_mut() {
            game.rumble.stop();
            game.timing.reset();
            let (backup, eeprom) = game.machine.nvram_blocks();
            if let Err(error) = reset_machine(game) {
                notify(core.environment, &format!("Reset failed: {error}"));
                return;
            }
            game.mvd_commands.resync();
            game.machine.set_nvram_blocks(&backup, &eeprom);
            game.machine.sound.samples.clear();
            game.steering = 0x80;
            game.accelerator = 0x20;
            game.brake = 0x20;
            game.audio_remainder = 0;
            game.audio.clear();
            game.output_audio.clear();
            game.nvram_pending = core.settings.nvram_settings;
            game.nvram_wait_frames = 0;
            game.publish_mvd_holder(core.environment);
        }
        export_save_ram(core);
    });
}

#[no_mangle]
pub extern "C" fn retro_load_game(info: *const ffi::GameInfo) -> bool {
    netpacket::shutdown();
    with_core(|core| {
        if let Some(game) = core.game.as_mut() {
            game.rumble.stop();
        }
        clear_controls(core.environment);
        if let Some(environment) = core.environment {
            publish_nvram_option_visibility(environment, "", false);
            publish_global_option_visibility(environment, &[]);
        }
        core.game = None;
        core.save_ram.fill(0);
        core.save_imported = false;
        update_settings(core);
        match load_game(core, info) {
            Ok(game) => {
                core.game = Some(game);
                true
            }
            Err(error) => {
                notify(core.environment, &error);
                false
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _kind: u32,
    _info: *const ffi::GameInfo,
    _count: usize,
) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    netpacket::shutdown();
    with_core(|core| {
        import_save_ram(core);
        export_save_ram(core);
        if let Some(game) = core.game.as_mut() {
            game.rumble.stop();
        }
        clear_controls(core.environment);
        if let Some(environment) = core.environment {
            publish_nvram_option_visibility(environment, "", false);
            publish_global_option_visibility(environment, &[]);
        }
        core.game = None;
    });
}

#[no_mangle]
pub extern "C" fn retro_run() {
    with_core(|core| {
        let run_start = (core.settings.overlay_font > 0).then(Instant::now);
        import_save_ram(core);
        let Some(game) = core.game.as_mut() else {
            return;
        };
        if let Some(env) = core.environment {
            let mut changed = false;
            if unsafe { env(ffi::GET_VARIABLE_UPDATE, (&mut changed as *mut bool).cast()) }
                && changed
            {
                core.settings.volume = option_value(env, c"tgpulse_next_volume")
                    .and_then(|value| value.parse::<u32>().ok())
                    .filter(|value| *value <= 800)
                    .unwrap_or(100);
                core.settings.aspect_ratio = aspect_ratio_option(env);
                core.settings.driving_ranges = driving_ranges(env);
                core.settings.mvd = mvd_settings(env);
                core.settings.diagnostic = diagnostic_settings(env);
                core.settings.netmerc_city_workaround = city_workaround_option(env);
                game.machine.set_netmerc_city_workaround(core.settings.netmerc_city_workaround);
                core.settings.steering_response = steering_response::Response::parse(
                    option_value(env, c"tgpulse_next_steering_response").as_deref(),
                );
                core.settings.nvram_settings = option_value(env, c"tgpulse_next_nvram_settings")
                    .is_some_and(|v| v == "enabled");
                game.nvram_pending = core.settings.nvram_settings;
                game.nvram_wait_frames = 0;
                publish_nvram_option_visibility(env, &game.set_name, core.settings.nvram_settings);
                core.settings.rumble = option_value(env, c"tgpulse_next_gamepad_rumble")
                    .is_none_or(|value| value != "disabled");
                if !core.settings.rumble {
                    game.rumble.stop();
                }
                let overlay_font = overlay_font_option(env);
                if overlay_font != core.settings.overlay_font {
                    game.timing.reset();
                    core.settings.overlay_font = overlay_font;
                }
                update_audio_settings(&mut core.settings, env);
                game.machine.sound.set_gains(core.settings.audio_gains);
                game.machine.sound.set_mutes(core.settings.audio_mutes);
            }
        }
        if let Some((_, config)) = game.linked_reset.as_mut() {
            config.audio_gains = core.settings.audio_gains;
            config.audio_mutes = core.settings.audio_mutes;
        }
        apply_pending_nvram(game, core.environment, core.settings.nvram_settings);
        if let Some(poll) = core.input_poll {
            unsafe { poll() };
        }
        match game.profile {
            Profile::VirtuaRacing | Profile::VirtuaFormula => {
                game.machine.inputs = apply_driving_tuning(
                    vr_inputs(
                        &mut game.steering,
                        &mut game.accelerator,
                        &mut game.brake,
                        core.input_state,
                        core.devices,
                    ),
                    core.settings.driving_ranges,
                    core.settings.steering_response,
                );
            }
            Profile::VirtuaFighter => {
                game.machine.inputs = vf_inputs(core.input_state, core.devices)
            }
            profile => game.machine.inputs = flight_inputs(profile, core.input_state, core.devices),
        }
        game.diagnostic.prepare(core.settings.diagnostic.enabled && game.profile == Profile::NetMerc);
        if game.profile == Profile::NetMerc {
            let (pose, active) = mvd_input(core.input_state, core.devices[0], core.settings.mvd);
            let mut notices = game.mvd_sensors.configure(core.settings.mvd, joypad_enabled(core.devices[0]));
            notices.extend(game.mvd_sensors.commands(game.mvd_commands.pressed(active)));
            notices.extend(game.mvd_sensors.poll());
            let pose = game.mvd_sensors.angles().map(mvd::sensor_pose).unwrap_or(pose);
            game.machine.ioboard.set_hmd_pose(pose);
            for notice in notices { notify(core.environment, &notice); }
        }
        if let Some(board) = game.machine.comm.as_mut() {
            if let Err(error) = netpacket::pump(board) {
                notify(core.environment, error);
            }
        }
        let machine_start = run_start.map(|_| Instant::now());
        if let Err(error) = game.machine.run_frame() {
            game.rumble.stop();
            notify(core.environment, &format!("Model 1 frame failed: {error}"));
            if let Some(env) = core.environment {
                unsafe { env(ffi::SHUTDOWN, ptr::null_mut()) };
            }
            return;
        }
        let game = core.game.as_mut().unwrap();
        game.publish_mvd_holder(core.environment);
        if let Some(board) = game.machine.comm.as_mut() {
            if let Err(error) = netpacket::pump(board) {
                notify(core.environment, error);
            }
            let status = [
                board.shared_read(0),
                board.shared_read(1),
                board.shared_read(2),
                board.shared_read(3),
            ];
            if game.link_status != Some(status) {
                eprintln!(
                    "[TGPulse-Next Libretro] [NetBoard] COMM status/role/ID/count: {status:?}"
                );
                game.link_status = Some(status);
            }
            if status[0] == 1 {
                game.linked_online_frames += 1;
                if game.linked_online_frames == 600 {
                    eprintln!("[TGPulse-Next Libretro] [NetBoard] Game link online for 600 consecutive frames");
                }
            } else {
                game.linked_online_frames = 0;
            }
        }
        // Match the standalone: sample the final port-D motor latch.
        // Within-frame On/Off pulses do not become extra frontend effects.
        let motor = game.machine.ioboard.netmerc_motor();
        if core.settings.rumble && joypad_enabled(core.devices[0]) {
            if let Some(on) = motor {
                game.rumble.netmerc(on);
            } else if tgpulse_core::model1_drive::DriveFamily::for_set(&game.set_name).is_some() {
                game.rumble.frame(game.machine.drive_cmd, game.machine.inputs.steer);
            }
        }
        if let Some(environment) = core.environment {
            publish_aspect_ratio(game, core.settings.aspect_ratio, environment);
        }
        if let Some((_, config)) = game.linked_reset.as_mut() {
            config.audio_gains = core.settings.audio_gains;
            config.audio_mutes = core.settings.audio_mutes;
        }
        apply_pending_nvram(game, core.environment, core.settings.nvram_settings);
        let machine_end = run_start.map(|_| Instant::now());
        if game.hardware {
            let width = render_width(game, core.settings.aspect_ratio);
            tilemap::render_background(&*game.machine, &mut game.pixels);
            tilemap::render_foreground(&*game.machine, &mut game.foreground);
            let quads = model1_video::gpu_quads_ws(&mut game.machine, width as f32);
            let mut frame = gpu::FrameData::new(
                width,
                game.graphics.ss,
                &quads,
                &game.pixels,
                &game.foreground,
                core.settings.smooth_shadows,
                game.graphics.wide == 2,
            );
            let lcd_pixels = if core.settings.diagnostic.enabled && game.profile == Profile::NetMerc {
                game.diagnostic.font().and_then(|font| game.machine.ioboard.diagnostic_pixels(font))
            } else { None };
            if core.settings.overlay_font > 0 || lcd_pixels.is_some() {
                // Map the game's 2D layer first. The panel uses final output
                // coordinates and never inherits native-layer centering/stretch.
                gpu::map_foreground_for_panel(
                    &game.foreground, width as usize, game.graphics.wide == 2,
                    &mut game.panel_foreground,
                );
                if let Some(pixels) = lcd_pixels {
                    diagnostic_overlay::draw(&mut game.panel_foreground, width as usize, HEIGHT,
                        &pixels, core.settings.diagnostic, true);
                }
                game.panel.draw_foreground(
                    &mut game.panel_foreground,
                    width as usize,
                    HEIGHT,
                    core.settings.overlay_font,
                    if game.fast_60hz { 60.0 } else { FPS },
                    game.timing.published,
                );
                frame.use_display_foreground(&game.panel_foreground);
            }
            let result = core
                .gpu
                .as_mut()
                .ok_or_else(|| {
                    core.hardware_error
                        .clone()
                        .unwrap_or_else(|| "GPU context not ready".into())
                })
                .and_then(|renderer| unsafe { renderer.render(&frame, game.graphics.srgb) });
            if let Err(error) = result {
                game.rumble.stop();
                notify(core.environment, &error);
                if let Some(env) = core.environment {
                    unsafe {
                        env(ffi::SHUTDOWN, ptr::null_mut());
                    }
                }
                return;
            }
            if let Some(video) = core.video {
                unsafe { video(gpu::HW_FRAME, width, HEIGHT as u32, 0) }
            }
        } else {
            render(game);
            if core.settings.diagnostic.enabled && game.profile == Profile::NetMerc {
                if let Some(pixels) = game.diagnostic.font()
                    .and_then(|font| game.machine.ioboard.diagnostic_pixels(font)) {
                    diagnostic_overlay::draw(&mut game.pixels, WIDTH, HEIGHT, &pixels,
                        core.settings.diagnostic, false);
                }
            }
            if core.settings.overlay_font > 0 {
                game.panel.draw(
                    &mut game.pixels,
                    WIDTH,
                    HEIGHT,
                    core.settings.overlay_font,
                    if game.fast_60hz { 60.0 } else { FPS },
                    game.timing.published,
                );
            }
            if let Some(video) = core.video {
                unsafe {
                    video(
                        game.pixels.as_ptr().cast(),
                        WIDTH as u32,
                        HEIGHT as u32,
                        WIDTH * 4,
                    )
                }
            }
        }
        let video_end = run_start.map(|_| Instant::now());
        collect_audio(game, core.settings.volume);
        if game.fast_60hz {
            resample_fast_audio(game);
        }
        let audio = if game.fast_60hz {
            &game.output_audio
        } else {
            &game.audio
        };
        if let Some(batch) = core.audio_batch {
            let mut sent = 0;
            let total = audio.len() / 2;
            while sent < total {
                let consumed = unsafe { batch(audio.as_ptr().add(sent * 2), total - sent) };
                if consumed == 0 || consumed > total - sent {
                    break;
                }
                sent += consumed;
            }
        } else if let Some(sample) = core.audio_sample {
            for pair in audio.chunks_exact(2) {
                unsafe { sample(pair[0], pair[1]) };
            }
        }
        // Publish after the first delivered frame, once startup NVRAM/Holder
        // messages and frontend initialization have completed.
        if game.diagnostic.take_notice() {
            notify_priority(core.environment, diagnostic_overlay::MISSING_BIOS, 300, 2, 2);
        }
        if let Some(message) = game.audio_notice.take() {
            notify_priority(core.environment, &message, 300, 3, 2);
        }
        if core.settings.overlay_font > 0 {
            if let (Some(start), Some(machine_start), Some(machine_end), Some(video_end)) =
                (run_start, machine_start, machine_end, video_end)
            {
                let end = Instant::now();
                let ms = |a: Instant, b: Instant| b.duration_since(a).as_secs_f64() * 1000.0;
                game.timing.record(
                    start,
                    ms(machine_start, machine_end),
                    ms(machine_end, video_end),
                    ms(video_end, end),
                    ms(start, end),
                );
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    ffi::REGION_NTSC
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    with_core(|core| {
        if core.game.as_ref().is_some_and(|g| g.machine.comm.is_some()) {
            0
        } else {
            persistence::STATE_BYTES
        }
    })
}

#[no_mangle]
pub extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    if data.is_null() || size != persistence::STATE_BYTES {
        return false;
    }
    with_core(|core| {
        import_save_ram(core);
        let Some(game) = core.game.as_ref() else {
            return false;
        };
        match game.machine.save_state() {
            Ok(state) => {
                let target = unsafe { std::slice::from_raw_parts_mut(data.cast::<u8>(), size) };
                persistence::export_state(
                    &state,
                    [game.steering, game.accelerator, game.brake],
                    game.audio_remainder,
                    target,
                )
            }
            Err(error) => {
                notify(core.environment, &error);
                false
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    if data.is_null() || size != persistence::STATE_BYTES {
        return false;
    }
    with_core(|core| {
        import_save_ram(core);
        let Some(game) = core.game.as_mut() else {
            return false;
        };
        let source = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
        let (state, controls, audio_remainder) = match persistence::import_state(source) {
            Ok(value) => value,
            Err(error) => {
                notify(core.environment, error);
                return false;
            }
        };
        if let Err(error) = game.machine.load_state(state) {
            notify(core.environment, &error);
            return false;
        }
        game.rumble.stop();
        game.timing.reset();
        game.nvram_pending = false;
        game.mvd_commands.resync();
        game.mvd_sensors.stop();
        [game.steering, game.accelerator, game.brake] = controls;
        game.audio_remainder = audio_remainder;
        game.audio.clear();
        game.output_audio.clear();
        game.machine.sound.samples.clear();
        game.publish_mvd_holder(core.environment);
        export_save_ram(core);
        true
    })
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}

#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: u32, _enabled: bool, _code: *const c_char) {}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(id: u32) -> *mut c_void {
    if id != ffi::MEMORY_SAVE_RAM {
        return ptr::null_mut();
    }
    with_core(|core| {
        if core.save_imported {
            export_save_ram(core);
        }
        core.save_ram.as_mut_ptr().cast()
    })
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(id: u32) -> usize {
    if id == ffi::MEMORY_SAVE_RAM {
        persistence::SAVE_BYTES
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static CALLS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    static RUMBLE_CALLS: Mutex<Vec<(u32, u32, u16)>> = Mutex::new(Vec::new());

    unsafe extern "C" fn capture_rumble(port: u32, effect: u32, strength: u16) -> bool {
        RUMBLE_CALLS.lock().unwrap().push((port, effect, strength));
        true
    }

    unsafe extern "C" fn rumble_environment(command: u32, data: *mut c_void) -> bool {
        if command != ffi::GET_RUMBLE_INTERFACE {
            return false;
        }
        unsafe { (*data.cast::<ffi::RumbleInterface>()).set_rumble_state = Some(capture_rumble) };
        true
    }

    unsafe extern "C" fn no_rumble_environment(_command: u32, _data: *mut c_void) -> bool {
        false
    }

    #[test]
    fn frontend_rumble_uses_two_p1_motors_and_stops_them() {
        RUMBLE_CALLS.lock().unwrap().clear();
        let mut rumble = PadRumble::new(rumble_environment);
        rumble.frame(0x5f, 0x80);
        rumble.stop();
        let calls = RUMBLE_CALLS.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].0, 0);
        assert_eq!(calls[0].1, ffi::RUMBLE_STRONG);
        assert!(calls[0].2 > 0);
        assert_eq!(calls[1].1, ffi::RUMBLE_WEAK);
        assert!(calls[1].2 > 0 && calls[1].2 < calls[0].2);
        assert_eq!(calls[2], (0, ffi::RUMBLE_STRONG, 0));
        assert_eq!(calls[3], (0, ffi::RUMBLE_WEAK, 0));
        drop(calls);
        RUMBLE_CALLS.lock().unwrap().clear();
        rumble.netmerc(true);
        rumble.netmerc(false);
        rumble.stop();
        assert_eq!(*RUMBLE_CALLS.lock().unwrap(), [
            (0, ffi::RUMBLE_STRONG, 39321), (0, ffi::RUMBLE_WEAK, 39321),
            (0, ffi::RUMBLE_STRONG, 0), (0, ffi::RUMBLE_WEAK, 0),
            (0, ffi::RUMBLE_STRONG, 0), (0, ffi::RUMBLE_WEAK, 0),
        ]);
        // Match standalone sampling: firmware On/Off inside one slice ends
        // silent, even if a native pulse observation is available.
        let mut firmware = vec![0; 0x10000];
        let program = [0x3e, 4, 0x32, 3, 0x80, 0xaf, 0x32, 3, 0x80, 0x76];
        firmware[..program.len()].copy_from_slice(&program);
        let mut board = tgpulse_core::model1board::IoBoard::new(
            tgpulse_core::model1board::Kind::NetMerc, &firmware,
            tgpulse_core::eeprom93c46::Eeprom93c46::new(),
        ).unwrap();
        board.run_main_cycles(200, Inputs::default()).unwrap();
        assert_eq!(board.netmerc_motor(), Some(false));
        RUMBLE_CALLS.lock().unwrap().clear();
        assert_eq!(board.take_netmerc_motor_activity(), Some(true));
        rumble.netmerc(board.netmerc_motor().unwrap());
        rumble.netmerc(board.netmerc_motor().unwrap());
        assert_eq!(*RUMBLE_CALLS.lock().unwrap(), [
            (0, ffi::RUMBLE_STRONG, 0), (0, ffi::RUMBLE_WEAK, 0),
            (0, ffi::RUMBLE_STRONG, 0), (0, ffi::RUMBLE_WEAK, 0),
        ]);
        RUMBLE_CALLS.lock().unwrap().clear();
        let mut unsupported = PadRumble::new(no_rumble_environment);
        unsupported.frame(0x5f, 0x80);
        unsupported.netmerc(true);
        unsupported.stop();
        assert!(RUMBLE_CALLS.lock().unwrap().is_empty());
    }

    unsafe extern "C" fn mock_environment(command: u32, data: *mut c_void) -> bool {
        CALLS.lock().unwrap().push(command);
        match command {
            ffi::GET_CORE_OPTIONS_VERSION => {
                unsafe { *data.cast::<u32>() = 2 };
                true
            }
            ffi::SET_CORE_OPTIONS_V2 => {
                let options = unsafe { &*data.cast::<ffi::OptionsV2>() };
                let find = |key: &CStr| self::options().definitions.iter()
                    .find(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == key).unwrap();
                assert_eq!(options.definitions, self::options().definitions.as_ptr());
                let first = find(c"tgpulse_next_rom_repairs");
                assert_eq!(
                    unsafe { CStr::from_ptr(first.key) },
                    c"tgpulse_next_rom_repairs"
                );
                assert_eq!(unsafe { CStr::from_ptr(first.default_value) }, c"enabled");
                let second = find(c"tgpulse_next_smooth_shadows");
                assert_eq!(
                    unsafe { CStr::from_ptr(second.key) },
                    c"tgpulse_next_smooth_shadows"
                );
                let third = find(c"tgpulse_next_av_timing");
                assert_eq!(
                    unsafe { CStr::from_ptr(third.key) },
                    c"tgpulse_next_av_timing"
                );
                assert_eq!(unsafe { CStr::from_ptr(third.default_value) }, c"native");
                let fourth = find(c"tgpulse_next_aspect_ratio");
                assert_eq!(
                    unsafe { CStr::from_ptr(fourth.key) },
                    c"tgpulse_next_aspect_ratio"
                );
                assert_eq!(unsafe { CStr::from_ptr(fourth.default_value) }, c"auto");
                let fifth = find(c"tgpulse_next_volume");
                assert_eq!(unsafe { CStr::from_ptr(fifth.key) }, c"tgpulse_next_volume");
                for (key, default) in [
                    (c"tgpulse_next_multipcm1_gain", c"auto"),
                    (c"tgpulse_next_multipcm2_gain", c"auto"),
                    (c"tgpulse_next_ym3438_gain", c"auto"),
                    (c"tgpulse_next_dsb_gain", c"auto"),
                ] {
                    let definition = find(key);
                    assert_eq!(unsafe { CStr::from_ptr(definition.key) }, key);
                    assert_eq!(unsafe { CStr::from_ptr(definition.default_value) }, default);
                    assert_eq!(
                        unsafe { CStr::from_ptr(definition.values[0].value) },
                        c"mute"
                    );
                    assert_eq!(
                        unsafe { CStr::from_ptr(definition.values[0].label) },
                        c"Mute"
                    );
                    assert_eq!(
                        unsafe { CStr::from_ptr(definition.values[1].value) },
                        c"auto"
                    );
                    assert_eq!(unsafe { CStr::from_ptr(definition.values[2].value) }, c"0");
                }
                let source_keys: Vec<_> = self::options()
                    .definitions
                    .iter()
                    .filter(|definition| !definition.key.is_null())
                    .map(|definition| unsafe { CStr::from_ptr(definition.key) })
                    .filter(|key| key.to_bytes().ends_with(b"_gain"))
                    .collect();
                assert_eq!(source_keys, AUDIO_GAIN_KEYS);
                let rumble = find(c"tgpulse_next_gamepad_rumble");
                assert_eq!(
                    unsafe { CStr::from_ptr(rumble.key) },
                    c"tgpulse_next_gamepad_rumble"
                );
                assert_eq!(unsafe { CStr::from_ptr(rumble.default_value) }, c"enabled");
                assert_eq!(unsafe { CStr::from_ptr(rumble.category_key) }, c"input");
                let overlay = find(c"tgpulse_next_timing_overlay");
                assert_eq!(
                    unsafe { CStr::from_ptr(overlay.key) },
                    c"tgpulse_next_timing_overlay"
                );
                assert_eq!(
                    unsafe { CStr::from_ptr(overlay.default_value) },
                    c"disabled"
                );
                true
            }
            ffi::GET_VARIABLE => false,
            ffi::SET_MESSAGE => {
                let message = unsafe { &*data.cast::<ffi::Message>() };
                assert!(unsafe { CStr::from_ptr(message.msg) }
                    .to_str()
                    .unwrap()
                    .contains("ZIP path"));
                true
            }
            _ => true,
        }
    }

    unsafe extern "C" fn sixty_hz_environment(command: u32, data: *mut c_void) -> bool {
        if command != ffi::GET_VARIABLE {
            return false;
        }
        let option = unsafe { &mut *data.cast::<ffi::Variable>() };
        if unsafe { CStr::from_ptr(option.key) } != c"tgpulse_next_av_timing" {
            return false;
        }
        option.value = c"60hz".as_ptr();
        true
    }

    unsafe extern "C" fn audio_environment(command: u32, data: *mut c_void) -> bool {
        if command != ffi::GET_VARIABLE {
            return false;
        }
        let option = unsafe { &mut *data.cast::<ffi::Variable>() };
        option.value = match unsafe { CStr::from_ptr(option.key) } {
            value if value == c"tgpulse_next_multipcm1_gain" => c"70".as_ptr(),
            value if value == c"tgpulse_next_multipcm2_gain" => c"Mute".as_ptr(),
            value if value == c"tgpulse_next_dsb_gain" => c"0".as_ptr(),
            value if value == c"tgpulse_next_ym3438_gain" => c"auto".as_ptr(),
            _ => panic!("unexpected source Gain key"),
        };
        true
    }

    unsafe extern "C" fn wide_aspect_environment(command: u32, data: *mut c_void) -> bool {
        if command != ffi::GET_VARIABLE {
            return false;
        }
        let option = unsafe { &mut *data.cast::<ffi::Variable>() };
        if unsafe { CStr::from_ptr(option.key) } != c"tgpulse_next_aspect_ratio" {
            return false;
        }
        option.value = c"16_9".as_ptr();
        true
    }

    #[test]
    fn aspect_ratio_uses_vr_monitor_setting_and_explicit_override() {
        let mut eeprom = [0xffff; 64];
        assert_eq!(
            selected_aspect_ratio(AspectRatio::Auto, "vr", &eeprom),
            ASPECT_4_3
        );
        eeprom[5] = 0xff01;
        assert_eq!(
            selected_aspect_ratio(AspectRatio::Auto, "vr", &eeprom),
            ASPECT_16_9
        );
        assert_eq!(
            selected_aspect_ratio(AspectRatio::Auto, "vformula", &eeprom),
            ASPECT_4_3
        );
        eeprom[5] = 0x0100;
        assert_eq!(
            selected_aspect_ratio(AspectRatio::Auto, "vr", &eeprom),
            ASPECT_4_3
        );
        assert_eq!(
            selected_aspect_ratio(AspectRatio::Auto, "vr", &[]),
            ASPECT_4_3
        );
        assert_eq!(
            selected_aspect_ratio(AspectRatio::SixteenNine, "vf", &[]),
            ASPECT_16_9
        );
        assert_eq!(
            selected_aspect_ratio(AspectRatio::FourThree, "vr", &[1; 64]),
            ASPECT_4_3
        );
        assert_eq!(
            aspect_ratio_option(wide_aspect_environment),
            AspectRatio::SixteenNine
        );
    }

    #[test]
    fn netmerc_audio_notices_identify_original_and_procedural_fallback() {
        let mut roms = loader::Model1Roms {
            netmerc_procedural_audio: false, dsb: None, comm_board: false,
            ioboard_kind: tgpulse_core::model1board::Kind::NetMerc,
            nvram_default: vec![], maincpu: vec![], tgp: vec![], copro_tables: vec![],
            polygons: vec![], copro_data: vec![], iocpu: vec![], sndcpu: vec![],
            mpcm1: vec![], mpcm2: vec![], ioboard_config: vec![],
        };
        assert_eq!(apply_audio_donor(&mut roms, "/absent/netmerc.zip", NetmercAudioDonor::Off).as_deref(),
                   Some("NetMerc Audio: Original Audio Incomplete (Bad ROM Dumps)"));
        assert_eq!(apply_audio_donor(&mut roms, "/absent/netmerc.zip", NetmercAudioDonor::Vf).as_deref(),
                   Some("NetMerc Audio Donor Unavailable - Original Audio Incomplete (Bad ROM Dumps)"));
        roms.netmerc_procedural_audio = true;
        assert_eq!(apply_audio_donor(&mut roms, "/absent/netmerc.zip", NetmercAudioDonor::Off).as_deref(),
                   Some("NetMerc Audio: Procedural Fallback"));
        roms.ioboard_kind = tgpulse_core::model1board::Kind::Original;
        assert!(apply_audio_donor(&mut roms, "/absent/vf.zip", NetmercAudioDonor::Vf).is_none());
    }

    #[test]
    fn audio_donor_defaults_and_legacy_registration_match_upstream() {
        assert_eq!(audio_donor_option(no_rumble_environment), NetmercAudioDonor::Vf);
        let definition = options().definitions.iter().find(|d| {
            !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == AUDIO_DONOR_KEY
        }).unwrap();
        assert_eq!(unsafe { CStr::from_ptr(definition.default_value) }, c"vf");
        assert_eq!(unsafe { CStr::from_ptr(definition.category_key) }, c"audio");
        for (value, (raw, label)) in definition.values.iter().zip(AUDIO_DONOR_VALUES) {
            assert_eq!(unsafe { CStr::from_ptr(value.value) }, *raw);
            assert_eq!(unsafe { CStr::from_ptr(value.label) }, *label);
        }
        unsafe extern "C" fn legacy(command: u32, data: *mut c_void) -> bool {
            if command == ffi::GET_CORE_OPTIONS_VERSION {
                *data.cast::<u32>() = 0;
                return true;
            }
            if command != ffi::SET_VARIABLES { return false; }
            let mut variables = data.cast::<ffi::Variable>();
            while !(*variables).key.is_null() {
                if CStr::from_ptr((*variables).key) == AUDIO_DONOR_KEY {
                    assert_eq!(CStr::from_ptr((*variables).value),
                               c"Sega NetMerc Audio Donor (Restart Required); vf|vr|swa|wingwar|off");
                    return true;
                }
                variables = variables.add(1);
            }
            panic!("Missing legacy donor option");
        }
        register_options(legacy);
    }

    #[test]
    fn source_audio_options_preserve_reference_defaults_and_read_changes() {
        let mut settings = Settings::default();
        assert_eq!(settings.audio_gains, AudioGains::REFERENCE);
        assert_eq!(settings.audio_mutes, AudioMutes::default());
        update_audio_settings(&mut settings, audio_environment);
        assert_eq!(settings.audio_gains.multipcm1, 70);
        assert_eq!(settings.audio_gains.multipcm2, 0);
        assert_eq!(settings.audio_gains.ym3438, 30);
        assert_eq!(settings.audio_gains.dsb, 0);
        assert_eq!(settings.audio_gains.scsp, AudioGains::REFERENCE.scsp);
        assert_eq!(settings.audio_mutes, AudioMutes::default());
        assert!(!settings.audio_mutes.multipcm1);
        assert!(!settings.audio_mutes.scsp);
        update_audio_settings(&mut settings, no_rumble_environment);
        assert_eq!(settings.audio_gains, AudioGains::REFERENCE);
        update_audio_settings(&mut settings, audio_environment);
        assert_eq!(settings.audio_gains.multipcm1, 70);
        assert_eq!(settings.audio_gains.multipcm2, 0);
    }

    static AUDIO_VISIBILITY: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

    unsafe extern "C" fn audio_visibility_environment(command: u32, data: *mut c_void) -> bool {
        if command == ffi::SET_CORE_OPTIONS_DISPLAY {
            let display = unsafe { &*data.cast::<ffi::OptionDisplay>() };
            AUDIO_VISIBILITY.lock().unwrap().push((
                unsafe { CStr::from_ptr(display.key) }
                    .to_str()
                    .unwrap()
                    .to_owned(),
                display.visible,
            ));
        }
        true
    }

    #[test]
    fn city_option_matches_standalone_defaults_and_legacy_values() {
        let definition = options().definitions.iter().find(|d| !d.key.is_null()
            && unsafe { CStr::from_ptr(d.key) } == c"tgpulse_next_netmerc_city_workaround").unwrap();
        assert_eq!(unsafe { CStr::from_ptr(definition.default_value) }, c"enabled");
        assert_eq!(unsafe { CStr::from_ptr(definition.category_key) }, c"video");
        assert!(Settings::default().netmerc_city_workaround);
        assert!(city_workaround_option(no_rumble_environment));
        unsafe extern "C" fn disabled(command: u32, data: *mut c_void) -> bool {
            if command != ffi::GET_VARIABLE { return false; }
            let variable = &mut *data.cast::<ffi::Variable>();
            if CStr::from_ptr(variable.key) != c"tgpulse_next_netmerc_city_workaround" { return false; }
            variable.value = c"disabled".as_ptr();
            true
        }
        assert!(!city_workaround_option(disabled));
        unsafe extern "C" fn legacy(command: u32, data: *mut c_void) -> bool {
            if command == ffi::GET_CORE_OPTIONS_VERSION {
                *data.cast::<u32>() = 0;
                return true;
            }
            if command != ffi::SET_VARIABLES { return false; }
            let mut variables = data.cast::<ffi::Variable>();
            let mut found = false;
            while !(*variables).key.is_null() {
                if CStr::from_ptr((*variables).key) == c"tgpulse_next_netmerc_city_workaround" {
                    assert_eq!(CStr::from_ptr((*variables).value), c"Sega NetMerc City Workaround; enabled|disabled");
                    found = true;
                }
                variables = variables.add(1);
            }
            assert!(found);
            true
        }
        register_options(legacy);
    }

    #[test]
    fn only_fitted_audio_sources_have_visible_gains() {
        let cases: &[(&[AudioSource], [bool; 4])] = &[
            (&[], [false; 4]),
            (tgpulse_core::sound::MULTIPCM_SOURCES, [true, true, true, false]),
            (&AUDIO_GAIN_SOURCES, [true; 4]),
            (&[AudioSource::MultiPcm2], [false, true, false, false]),
            (&[AudioSource::Scsp], [false; 4]),
        ];
        for &(sources, expected) in cases {
            AUDIO_VISIBILITY.lock().unwrap().clear();
            publish_global_option_visibility(audio_visibility_environment, sources);
            let calls = AUDIO_VISIBILITY.lock().unwrap();
            for (key, visible) in AUDIO_GAIN_KEYS.into_iter().zip(expected) {
                assert_eq!(calls.iter().find(|(name, _)| name == key.to_str().unwrap()).map(|(_, shown)| *shown),
                           Some(visible), "{key:?}, sources: {sources:?}");
            }
            assert!(calls.iter().filter(|(name, _)| !name.ends_with("_gain")).all(|(_, shown)| *shown));
        }
        let calls = AUDIO_VISIBILITY.lock().unwrap();
        for key in DRIVING_RANGE_KEYS.into_iter().chain(mvd::KEYS)
            .chain([c"tgpulse_next_netmerc_city_workaround", c"tgpulse_next_gamepad_rumble",
                    c"tgpulse_next_widescreen_mode", c"tgpulse_next_supersampling",
                    c"tgpulse_next_volume", c"tgpulse_next_nvram_settings"]) {
            assert!(calls.iter().any(|(k, _)| k == key.to_str().unwrap()));
        }
        assert!(!calls.iter().any(|(k, _)| k.starts_with("tgpulse_next_linked_cabinets_")
            || (k.starts_with("tgpulse_next_nvram_") && k != "tgpulse_next_nvram_settings")));
    }

    #[test]
    fn model1_timing_and_option_defaults_are_stable() {
        assert!((FPS - 57.52).abs() < 0.01);
        assert_eq!(WIDTH, 496);
        assert_eq!(HEIGHT, 384);
        assert!(Settings::default().smooth_shadows);
        assert_eq!(Settings::default().volume, 100);
        assert!(Settings::default().apply_known_rom_repairs);
        assert!(!Settings::default().fast_60hz);
        assert!(Settings::default().rumble);
        assert_eq!(Settings::default().overlay_font, 0);
        assert!((60.0 / FPS - 1.0 - 0.04304).abs() < 0.00001);
        assert_eq!(frontend_timing(DEFAULT_AUDIO_RATE, false).fps, FPS);
        assert_eq!(frontend_timing(DEFAULT_AUDIO_RATE, true).fps, 60.0);
        assert_eq!(
            frontend_timing(DEFAULT_AUDIO_RATE, true).sample_rate,
            DEFAULT_AUDIO_RATE
        );
    }

    #[test]
    fn frontend_selection_enables_faster_timing() {
        let mut core = Core {
            environment: Some(sixty_hz_environment),
            ..Core::default()
        };
        update_settings(&mut core);
        assert!(core.settings.fast_60hz);
        assert_eq!(
            frontend_timing(DEFAULT_AUDIO_RATE, core.settings.fast_60hz).fps,
            60.0
        );
    }

    #[test]
    fn faster_timing_delivers_steady_audio_at_native_output_rate() {
        let input = vec![100i16; 776 * 2];
        let mut output = Vec::new();
        let mut remainder = 0;
        let mut delivered = 0;
        for _ in 0..600 {
            resample_stereo_60hz(&input, &mut output, &mut remainder);
            assert!(matches!(output.len() / 2, 744 | 745));
            assert!(output.iter().all(|&sample| sample == 100));
            delivered += output.len() / 2;
        }
        assert_eq!(
            delivered,
            (600u64 * 10_000_000 / FAST_AUDIO_DENOMINATOR) as usize
        );
    }

    #[test]
    fn pedal_and_steering_steps_remain_bounded() {
        assert_eq!(approach(0x80, 0xe0, 10), 0x8a);
        assert_eq!(approach(0x20, 0xe0, 20), 0x34);
        assert_eq!(approach(0x80, 0x80, 10), 0x80);
    }

    #[test]
    fn abi_registers_options_and_rejects_missing_content() {
        CALLS.lock().unwrap().clear();
        retro_set_environment(Some(mock_environment));
        retro_init();
        let mut info = ffi::SystemInfo {
            library_name: ptr::null(),
            library_version: ptr::null(),
            valid_extensions: ptr::null(),
            need_fullpath: false,
            block_extract: false,
        };
        retro_get_system_info(&mut info);
        assert_eq!(unsafe { CStr::from_ptr(info.valid_extensions) }, c"zip");
        assert!(info.need_fullpath && info.block_extract);
        assert!(!retro_load_game(ptr::null()));
        retro_run();
        retro_deinit();
        let calls = CALLS.lock().unwrap();
        assert!(calls.contains(&ffi::SET_CORE_OPTIONS_V2));
        assert!(calls.contains(&ffi::SET_MESSAGE));
    }

    unsafe extern "C" fn p2_only(port: u32, device: u32, index: u32, id: u32) -> i16 {
        if port != 1 {
            return 0;
        }
        match (device, index, id) {
            (ffi::DEVICE_JOYPAD, 0, ffi::JOY_SELECT | ffi::JOY_B | ffi::JOY_A) => 1,
            (ffi::DEVICE_JOYPAD, 0, ffi::JOY_START) => 1,
            (ffi::DEVICE_ANALOG, ffi::ANALOG_LEFT, ffi::ANALOG_X) => 32767,
            _ => 0,
        }
    }

    #[test]
    fn swa_descriptors_and_p2_mapping_are_gunner_only() {
        let entries = descriptors(Profile::StarWarsArcade);
        assert!(entries
            .iter()
            .any(|(port, _, _, _, label)| *port == 1 && *label == c"Gunner Laser"));
        assert!(entries
            .iter()
            .any(|(port, _, _, _, label)| *port == 1 && *label == c"Gunner Stick X"));
        assert!(!entries.iter().any(|(port, _, _, _, label)| *port == 1
            && (*label == c"Start" || *label == c"Pilot View" || *label == c"Pilot Throttle Up")));
        let input = flight_inputs(
            Profile::StarWarsArcade,
            Some(p2_only),
            [ffi::DEVICE_JOYPAD; 2],
        );
        assert_eq!(input.in0 & 0x20, 0x20); // Gunner Start cannot reach Start2.
        assert_eq!(input.in1 & 0x0f, 0x03); // Only Gunner Laser/Torpedo.
        assert_eq!(input.analog[0], 127); // Pilot remains centred.
        assert!(input.analog[4] < 127); // Independent Gunner X.
    }

    #[test]
    fn vf_p2_and_r360_descriptors_are_specific() {
        let fighter = descriptors(Profile::VirtuaFighter);
        assert!(fighter
            .iter()
            .any(|(port, _, _, _, label)| *port == 1 && *label == c"Guard"));
        let inputs = vf_inputs(Some(p2_only), [ffi::DEVICE_JOYPAD; 2]);
        assert_eq!(inputs.in0 & 0x20, 0);
        assert_eq!(inputs.in1, 0xff);
        assert_eq!(inputs.in2 & 0x02, 0);
        let r360 = descriptors(Profile::WingWar360);
        assert!(!r360
            .iter()
            .any(|(_, _, _, _, label)| label.to_bytes().starts_with(b"View")));
        let netmerc = descriptors(Profile::NetMerc);
        assert!(!netmerc.iter().any(|(_, _, _, _, label)| *label == c"Start"));
    }

    unsafe extern "C" fn direction<const ID: u32>(port: u32, device: u32, _: u32, id: u32) -> i16 {
        i16::from(port == 0 && device == ffi::DEVICE_JOYPAD && id == ID)
    }

    unsafe extern "C" fn p2_operators(port: u32, device: u32, _: u32, id: u32) -> i16 {
        i16::from(
            port == 1
                && device == ffi::DEVICE_JOYPAD
                && matches!(id, ffi::JOY_SELECT | ffi::JOY_L3 | ffi::JOY_R3),
        )
    }

    #[test]
    fn reference_vr_views_reach_native_switches_without_steering() {
        use ffi::*;
        let cases: [(u32, ffi::InputState, u8, u8); 4] = [
            (JOY_DOWN, direction::<JOY_DOWN>, 0xdf, 0xff),
            (JOY_LEFT, direction::<JOY_LEFT>, 0xbf, 0xff),
            (JOY_RIGHT, direction::<JOY_RIGHT>, 0x7f, 0xff),
            (JOY_UP, direction::<JOY_UP>, 0xff, 0xfe),
        ];
        for (id, input, in0, in1) in cases {
            let controls = vr_inputs(&mut 128, &mut 32, &mut 32, Some(input), [DEVICE_JOYPAD; 2]);
            assert_eq!(
                (controls.in0, controls.in1, controls.steer),
                (in0, in1, 128)
            );
            for profile in [Profile::VirtuaRacing, Profile::VirtuaFormula] {
                assert!(descriptors(profile)
                    .iter()
                    .any(|(p, d, _, b, label)| *p == 0
                        && *d == DEVICE_JOYPAD
                        && *b == id
                        && label.to_bytes().starts_with(b"VR")));
                assert!(!descriptors(profile).iter().any(|(p, d, _, b, _)| *p == 0
                    && *d == DEVICE_JOYPAD
                    && matches!(*b, JOY_B | JOY_A | JOY_Y | JOY_X)));
            }
        }
    }

    unsafe extern "C" fn gunner_view(port: u32, device: u32, _: u32, id: u32) -> i16 {
        i16::from(
            port == 1 && device == ffi::DEVICE_JOYPAD && matches!(id, ffi::JOY_DOWN | ffi::JOY_UP),
        )
    }

    #[test]
    fn native_widescreen_bypasses_all_hack_modes_without_affecting_other_sets() {
        for selected in 0..=2 {
            assert_eq!(widescreen_hack_mode("vr", selected), 0);
            assert_eq!(expanded_render_width(true, widescreen_hack_mode("vr", selected), ASPECT_16_9), 496);
            for set in ["vformula", "vf", "wingwar", "swa"] {
                assert_eq!(widescreen_hack_mode(set, selected), selected);
            }
        }
        assert_eq!(graphics_settings(no_rumble_environment).wide, 1);
    }

    #[test]
    fn gpu_expansion_requires_a_wide_presentation_and_real_hardware_path() {
        for wide in 0..=2 {
            assert_eq!(expanded_render_width(false, wide, ASPECT_16_9), 496);
            assert_eq!(expanded_render_width(true, wide, ASPECT_4_3), 496);
            assert_eq!(
                expanded_render_width(true, wide, ASPECT_16_9),
                if wide == 0 { 496 } else { 683 }
            );
        }
    }

    #[test]
    fn reviewed_nvram_options_have_reference_defaults_and_readable_legacy_values() {
        for field in nvram::FIELDS {
            let option = options()
                .definitions
                .iter()
                .find(|o| !o.key.is_null() && unsafe { CStr::from_ptr(o.key) } == field.key)
                .unwrap();
            assert_eq!(
                unsafe { CStr::from_ptr(option.default_value) },
                field.values[field.default].key
            );
            assert_eq!(unsafe { CStr::from_ptr(option.category_key) }, c"system");
            assert_eq!(
                option
                    .values
                    .iter()
                    .take_while(|v| !v.value.is_null())
                    .count(),
                field.values.len()
            );
        }
        assert!(Settings::default().automatic_nvram);
        assert!(!Settings::default().nvram_settings);
        unsafe extern "C" fn legacy(cmd: u32, data: *mut c_void) -> bool {
            if cmd == ffi::GET_CORE_OPTIONS_VERSION {
                unsafe {
                    *data.cast::<u32>() = 0;
                }
                return true;
            }
            if cmd == ffi::SET_VARIABLES {
                let mut variables = data.cast::<ffi::Variable>();
                let mut fields = 0;
                let mut gains = Vec::new();
                let mut mvd_defaults = Vec::new();
                while !unsafe { (*variables).key }.is_null() {
                    let variable = unsafe { &*variables };
                    let key = unsafe { CStr::from_ptr(variable.key) };
                    if key.to_bytes().ends_with(b"_gain") {
                        gains.push(key);
                        let text = unsafe { CStr::from_ptr(variable.value) }.to_str().unwrap();
                        assert_eq!(
                            text.split("; ").nth(1).unwrap().split('|').next(),
                            Some("auto")
                        );
                    }
                    if mvd::KEYS.contains(&key) {
                        let text = CStr::from_ptr(variable.value).to_str().unwrap();
                        mvd_defaults.push(text.split("; ").nth(1).unwrap().split('|').next().unwrap());
                    }
                    if let Some(field) = nvram::FIELDS.iter().find(|f| f.key == key) {
                        let text = unsafe { CStr::from_ptr(variable.value) }.to_str().unwrap();
                        assert_eq!(
                            text.split("; ").nth(1).unwrap().split('|').next().unwrap(),
                            field.values[field.default].key.to_str().unwrap()
                        );
                        fields += 1;
                    }
                    variables = unsafe { variables.add(1) };
                }
                assert_eq!(fields, 43);
                assert_eq!(gains, AUDIO_GAIN_KEYS);
                assert_eq!(mvd_defaults, ["auto", "30", "20", "enabled", "50", "disabled"]);
                return true;
            }
            false
        }
        register_options(legacy);
    }

    #[test]
    fn driving_ranges_preserve_defaults_and_scale_only_native_analog_channels() {
        for value in 0x20..=0xe0u8 {
            let input = VrCabinet {
                steer: value,
                accel: value,
                brake: value,
                coin1: true,
                views: [true, false, true, false],
                ..VrCabinet::default()
            }
            .into_native();
            let normal = apply_driving_ranges(input, [100; 3]);
            assert_eq!([normal.steer, normal.accel, normal.brake], [value; 3]);
            assert_eq!(normal.analog, input.analog);
            for percent in (50..=150).step_by(10) {
                let scaled = apply_driving_ranges(input, [percent; 3]);
                assert_eq!(
                    [scaled.in0, scaled.in1, scaled.in2],
                    [input.in0, input.in1, input.in2]
                );
                assert_eq!(
                    scaled.analog[..3],
                    [scaled.steer, scaled.accel, scaled.brake]
                );
                assert_eq!(scaled.analog[3..], input.analog[3..]);
                assert!((0x20..=0xe0).contains(&scaled.steer));
            }
        }
        let input = VrCabinet {
            steer: 0xe0,
            accel: 0xe0,
            brake: 0x20,
            ..VrCabinet::default()
        }
        .into_native();
        let half = apply_driving_ranges(input, [50; 3]);
        assert_eq!([half.steer, half.accel, half.brake], [0xb0, 0x80, 0x20]);
        let full_early = apply_driving_ranges(
            VrCabinet {
                steer: 192,
                accel: 160,
                brake: 128,
                ..VrCabinet::default()
            }
            .into_native(),
            [150; 3],
        );
        assert_eq!(
            [full_early.steer, full_early.accel, full_early.brake],
            [224, 224, 176]
        );
        for percent in (50..=150).step_by(10) {
            let rest = apply_driving_ranges(
                VrCabinet {
                    steer: 128,
                    accel: 32,
                    brake: 32,
                    ..VrCabinet::default()
                }
                .into_native(),
                [percent; 3],
            );
            assert_eq!([rest.steer, rest.accel, rest.brake], [128, 32, 32]);
            let mut previous = [32; 3];
            for value in 32..=224u8 {
                let out = apply_driving_ranges(
                    VrCabinet {
                        steer: value,
                        accel: value,
                        brake: value,
                        ..VrCabinet::default()
                    }
                    .into_native(),
                    [percent; 3],
                );
                let channels = [out.steer, out.accel, out.brake];
                assert!(channels
                    .iter()
                    .zip(previous)
                    .all(|(current, old)| *current >= old));
                previous = channels;
            }
        }
    }

    #[test]
    fn driving_range_options_defaults_values_and_decode() {
        for key in DRIVING_RANGE_KEYS {
            let definition = options()
                .definitions
                .iter()
                .find(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == key)
                .unwrap();
            assert_eq!(unsafe { CStr::from_ptr(definition.default_value) }, c"100");
            assert_eq!(unsafe { CStr::from_ptr(definition.category_key) }, c"input");
            let values: Vec<_> = definition
                .values
                .iter()
                .take_while(|v| !v.value.is_null())
                .map(|v| {
                    unsafe { CStr::from_ptr(v.value) }
                        .to_str()
                        .unwrap()
                        .parse::<u32>()
                        .unwrap()
                })
                .collect();
            assert_eq!(values, (50..=150).step_by(10).collect::<Vec<_>>());
        }
        unsafe extern "C" fn env(cmd: u32, data: *mut c_void) -> bool {
            if cmd != ffi::GET_VARIABLE {
                return false;
            }
            let variable = unsafe { &mut *data.cast::<ffi::Variable>() };
            variable.value = match unsafe { CStr::from_ptr(variable.key) } {
                key if key == DRIVING_RANGE_KEYS[0] => c"50".as_ptr(),
                key if key == DRIVING_RANGE_KEYS[1] => c"150".as_ptr(),
                _ => c"75.3".as_ptr(),
            };
            true
        }
        assert_eq!(driving_ranges(env), [50, 150, 100]);
        assert_eq!(Settings::default().driving_ranges, [100; 3]);
    }

    #[test]
    fn linked_cabinet_options_follow_native_set_eligibility_and_reference_defaults() {
        let defs = &options().definitions;
        for (set, key, max) in netpacket::GAMES {
            assert!(tgpulse_core::model1comm::present_for_set(set));
            let d = defs
                .iter()
                .find(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == key)
                .unwrap();
            assert_eq!(unsafe { CStr::from_ptr(d.default_value) }, c"disabled");
            assert_eq!(unsafe { CStr::from_ptr(d.category_key) }, c"system");
            assert_eq!(
                d.values.iter().filter(|v| !v.value.is_null()).count(),
                max as usize
            );
        }
        for set in ["vf", "swa", "swaj", "netmerc"] {
            assert!(!netpacket::GAMES.iter().any(|(name, _, _)| *name == set));
        }
    }

    #[test]
    fn steering_response_matches_reference_options_and_fallback() {
        let key = c"tgpulse_next_steering_response";
        let option = options()
            .definitions
            .iter()
            .find(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == key)
            .unwrap();
        assert_eq!(unsafe { CStr::from_ptr(option.default_value) }, c"linear");
        assert_eq!(unsafe { CStr::from_ptr(option.category_key) }, c"input");
        let values: Vec<_> = option
            .values
            .iter()
            .take_while(|v| !v.value.is_null())
            .map(|v| unsafe { (CStr::from_ptr(v.value), CStr::from_ptr(v.label)) })
            .collect();
        assert_eq!(
            values,
            vec![
                (c"linear", c"Linear"),
                (c"progressive", c"Progressive (Fine Center)"),
                (c"fbneo", c"FBNeo Logarithmic (Fine Center)")
            ]
        );
        assert_eq!(
            Settings::default().steering_response,
            steering_response::Response::Linear
        );
    }

    #[test]
    fn steering_response_keeps_pedals_bits_and_native_mirrors_with_ranges() {
        for response in [
            steering_response::Response::Linear,
            steering_response::Response::Progressive,
            steering_response::Response::Fbneo,
        ] {
            for percent in (50..=150).step_by(10) {
                let mut previous = 0x20;
                for value in 0x20..=0xe0 {
                    let input = VrCabinet {
                        steer: value,
                        accel: 0xa0,
                        brake: 0x40,
                        ..VrCabinet::default()
                    }
                    .into_native();
                    let plain = apply_driving_ranges(input.clone(), [percent, 80, 120]);
                    let out = apply_driving_tuning(input, [percent, 80, 120], response);
                    assert!(out.steer >= previous);
                    assert_eq!(
                        (out.accel, out.brake, out.in0, out.in1),
                        (plain.accel, plain.brake, plain.in0, plain.in1)
                    );
                    assert_eq!(out.analog[..3], [out.steer, out.accel, out.brake]);
                    if response == steering_response::Response::Linear {
                        assert_eq!(out.steer, plain.steer);
                    }
                    previous = out.steer;
                }
            }
        }
    }

    #[test]
    fn renderer_defaults_expand_only_3d_and_no_srgb_is_exposed() {
        let definitions = &options().definitions;
        let find = |key: &CStr| {
            definitions
                .iter()
                .find(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == key)
                .unwrap()
        };
        assert_eq!(
            unsafe { CStr::from_ptr(find(c"tgpulse_next_widescreen_mode").default_value) },
            c"expand_3d"
        );
        let ss = find(c"tgpulse_next_supersampling");
        assert_eq!(unsafe { CStr::from_ptr(ss.default_value) }, c"1");
        let values: Vec<_> = ss
            .values
            .iter()
            .take_while(|v| !v.value.is_null())
            .map(|v| unsafe { CStr::from_ptr(v.value) })
            .collect();
        assert_eq!(values, vec![c"1", c"2", c"3", c"4"]);
        assert!(!definitions
            .iter()
            .any(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == c"tgpulse_next_srgb"));
    }

    #[test]
    fn swa_vr1_follows_standalone_down_and_up_alias_on_pilot_only() {
        use ffi::*;
        for input in [direction::<JOY_DOWN> as InputState, direction::<JOY_UP>] {
            let native = flight_inputs(Profile::StarWarsArcade, Some(input), [DEVICE_JOYPAD; 2]);
            assert_eq!(native.in1, 0xef);
        }
        for input in [direction::<JOY_X> as InputState, gunner_view] {
            let native = flight_inputs(Profile::StarWarsArcade, Some(input), [DEVICE_JOYPAD; 2]);
            assert_eq!(native.in1, 0xff);
        }
        assert_eq!(
            unsafe {
                CStr::from_ptr(ControllerStorage::new(Profile::WingWar).types[0][1].desc)
            },
            c"Flight: Wing War + VR4"
        );
        let entries = descriptors(Profile::StarWarsArcade);
        assert!(entries
            .iter()
            .any(|(p, _, _, id, label)| *p == 0 && *id == JOY_DOWN && *label == c"VR1"));
        assert!(entries
            .iter()
            .any(|(p, _, _, id, label)| *p == 0 && *id == JOY_UP && *label == c"VR1"));
        assert!(!entries
            .iter()
            .any(|(p, _, _, _, label)| *p == 1 && label.to_bytes().starts_with(b"VR1")));
    }

    unsafe extern "C" fn port_button<const PORT: u32, const ID: u32>(
        port: u32,
        device: u32,
        _: u32,
        id: u32,
    ) -> i16 {
        i16::from(port == PORT && device == ffi::DEVICE_JOYPAD && id == ID)
    }

    unsafe extern "C" fn axis_value<
        const PORT: u32,
        const INDEX: u32,
        const AXIS: u32,
        const VALUE: i16,
    >(
        port: u32,
        device: u32,
        index: u32,
        id: u32,
    ) -> i16 {
        if port == PORT && device == ffi::DEVICE_ANALOG && index == INDEX && id == AXIS {
            VALUE
        } else {
            0
        }
    }

    #[test]
    fn vf_face_actions_match_reference_bits_and_shoulders_are_removed() {
        use ffi::*;
        let cases: [(InputState, usize, u8); 10] = [
            (port_button::<0, JOY_B>, 0, 0xfd),
            (port_button::<0, JOY_A>, 0, 0xfe),
            (port_button::<0, JOY_Y>, 0, 0xfb),
            (port_button::<1, JOY_B>, 1, 0xfd),
            (port_button::<1, JOY_A>, 1, 0xfe),
            (port_button::<1, JOY_Y>, 1, 0xfb),
            (port_button::<0, JOY_L>, 0, 0xff),
            (port_button::<0, JOY_R>, 0, 0xff),
            (port_button::<1, JOY_L>, 1, 0xff),
            (port_button::<1, JOY_R>, 1, 0xff),
        ];
        for (callback, port, expected) in cases {
            let input = vf_inputs(Some(callback), [DEVICE_JOYPAD; 2]);
            let panels = [input.in1, input.in2];
            assert_eq!(panels[port], expected);
            assert_eq!(panels[1 - port], 0xff);
        }
        assert!(!descriptors(Profile::VirtuaFighter)
            .iter()
            .any(|(_, device, _, id, _)| *device == DEVICE_JOYPAD && matches!(*id, JOY_L | JOY_R)));
    }

    #[test]
    fn vf_exposes_only_native_digital_controls() {
        use ffi::*;
        for port in 0..2 {
            assert!(!descriptors(Profile::VirtuaFighter)
                .iter()
                .any(|(p, d, _, _, _)| *p == port && *d == DEVICE_ANALOG));
        }
        for input in [
            axis_value::<0, ANALOG_LEFT, ANALOG_X, -32768> as InputState,
            axis_value::<0, ANALOG_LEFT, ANALOG_Y, 32767>,
            axis_value::<1, ANALOG_LEFT, ANALOG_X, 32767>,
            axis_value::<1, ANALOG_LEFT, ANALOG_Y, -32768>,
        ] {
            let native = vf_inputs(Some(input), [DEVICE_JOYPAD; 2]);
            assert_eq!((native.in1, native.in2), (0xff, 0xff));
        }
        for (input, expected) in [
            (port_button::<0, JOY_LEFT> as InputState, (0x7f, 0xff)),
            (port_button::<1, JOY_RIGHT> as InputState, (0xff, 0xbf)),
            (port_button::<0, JOY_UP> as InputState, (0xdf, 0xff)),
            (port_button::<1, JOY_DOWN> as InputState, (0xff, 0xef)),
        ] {
            let native = vf_inputs(Some(input), [DEVICE_JOYPAD; 2]);
            assert_eq!((native.in1, native.in2), expected);
        }
    }

    #[test]
    fn netmerc_aliases_match_native_switches_and_virtual_commands_have_no_cabinet_pins() {
        use ffi::*;
        let devices = [DEVICE_JOYPAD; 2];
        for (callback, bit) in [
            (port_button::<0, JOY_B> as InputState, 1),
            (port_button::<0, JOY_L2>, 1),
            (port_button::<0, JOY_R2>, 1),
            (port_button::<0, JOY_A>, 2),
            (port_button::<0, JOY_L>, 2),
            (port_button::<0, JOY_R>, 2),
            (port_button::<0, JOY_START>, 4),
            (port_button::<0, JOY_DOWN>, 4),
        ] {
            let native = flight_inputs(Profile::NetMerc, Some(callback), devices);
            assert_eq!(native.in0, 0xff); // Start is only a Holder binding.
            assert_eq!(native.in1, 0xff ^ bit);
            assert_eq!(flight_inputs(Profile::NetMerc, Some(callback), [DEVICE_NONE, DEVICE_JOYPAD]).in1, 0xff);
        }
        for callback in [port_button::<0, JOY_X> as InputState, port_button::<0, JOY_Y>,
                         port_button::<1, JOY_L>, port_button::<1, JOY_R>,
                         port_button::<1, JOY_L2>, port_button::<1, JOY_R2>,
                         port_button::<1, JOY_B>, port_button::<1, JOY_A>,
                         port_button::<1, JOY_START>, port_button::<1, JOY_X>, port_button::<1, JOY_Y>] {
            let native = flight_inputs(Profile::NetMerc, Some(callback), devices);
            assert_eq!((native.in0, native.in1), (0xff, 0xff));
        }
        for (callback, expected) in [
            (axis_value::<0, ANALOG_BUTTON, JOY_L2, 16383> as InputState, 0xff),
            (axis_value::<0, ANALOG_BUTTON, JOY_L2, 16384>, 0xfe),
            (axis_value::<0, ANALOG_BUTTON, JOY_R2, 16383>, 0xff),
            (axis_value::<0, ANALOG_BUTTON, JOY_R2, 16384>, 0xfe),
            (axis_value::<0, ANALOG_BUTTON, JOY_R2, 32767>, 0xfe),
        ] {
            assert_eq!(flight_inputs(Profile::NetMerc, Some(callback), devices).in1, expected);
        }
        let entries = descriptors(Profile::NetMerc);
        for (id, label) in [(JOY_B, c"Trigger Button"), (JOY_L2, c"Trigger Button"), (JOY_R2, c"Trigger Button"),
            (JOY_A, c"Thumb Button"), (JOY_L, c"Thumb Button"), (JOY_R, c"Thumb Button"),
            (JOY_START, c"MVD Holder"), (JOY_DOWN, c"MVD Holder"),
            (JOY_X, c"MVD Calibrate"), (JOY_Y, c"MVD Recenter")] {
            assert!(entries.iter().any(|&(p, dev, _, b, l)| p == 0 && dev == DEVICE_JOYPAD && b == id && l == label));
        }
        assert_eq!(entries.iter().filter(|e| e.0 == 1).count(), 2);
        let profile = ControllerStorage::new(Profile::NetMerc);
        assert_eq!(unsafe { CStr::from_ptr(profile.types[0][1].desc) }, c"Special: Sega NetMerc");
    }

    #[test]
    fn netmerc_sticks_preserve_frontend_travel_and_native_axis_polarity() {
        use ffi::*;
        for (callback, channel, expected) in [
            (axis_value::<0, ANALOG_LEFT, ANALOG_X, -32768> as InputState, 0, 0),
            (axis_value::<0, ANALOG_LEFT, ANALOG_X, 32767>, 0, 255),
            (axis_value::<0, ANALOG_LEFT, ANALOG_Y, -32768>, 2, 0),
            (axis_value::<0, ANALOG_LEFT, ANALOG_Y, 32767>, 2, 255),
            (axis_value::<0, ANALOG_LEFT, ANALOG_Y, -16384>, 2, 64),
            (axis_value::<0, ANALOG_LEFT, ANALOG_Y, 0>, 2, 127),
            (axis_value::<0, ANALOG_LEFT, ANALOG_Y, 1000>, 2, 131),
        ] {
            assert_eq!(flight_inputs(Profile::NetMerc, Some(callback), [DEVICE_JOYPAD; 2]).analog[channel], expected);
        }
        let settings = mvd::Settings::default();
        let forward = tgpulse_core::model1io2::HmdPose::default();
        for (callback, channel, sign) in [
            (axis_value::<0, ANALOG_RIGHT, ANALOG_X, -32768> as InputState, 0, -1),
            (axis_value::<0, ANALOG_RIGHT, ANALOG_X, 32767>, 0, 1),
            (axis_value::<0, ANALOG_RIGHT, ANALOG_Y, -32768>, 2, -1),
            (axis_value::<0, ANALOG_RIGHT, ANALOG_Y, 32767>, 2, 1),
        ] {
            let (pose, commands) = mvd_input(Some(callback), DEVICE_JOYPAD, settings);
            assert_eq!((pose.orientation[channel] - forward.orientation[channel]).signum(), sign);
            assert_eq!(commands, [false; 2]);
            assert_eq!(mvd_input(Some(callback), DEVICE_NONE, settings), (forward, [false; 2]));
        }
        assert_eq!(mvd_input(Some(port_button::<0, JOY_X>), DEVICE_JOYPAD, settings).1, [true, false]);
        assert_eq!(mvd_input(Some(port_button::<0, JOY_Y>), DEVICE_JOYPAD, settings).1, [false, true]);
        assert_eq!(mvd_input(Some(port_button::<1, JOY_X>), DEVICE_JOYPAD, settings).1, [false; 2]);
    }

    #[test]
    fn netmerc_options_use_live_input_category_and_reference_defaults() {
        for (key, default) in mvd::KEYS.into_iter().zip([c"auto", c"30", c"20", c"enabled", c"50", c"disabled"]) {
            let d = options().definitions.iter().find(|d| !d.key.is_null() && unsafe { CStr::from_ptr(d.key) } == key).unwrap();
            assert_eq!(unsafe { CStr::from_ptr(d.default_value) }, default);
            assert_eq!(unsafe { CStr::from_ptr(d.category_key) }, c"input");
            assert!(!unsafe { CStr::from_ptr(d.desc) }.to_bytes().windows(7).any(|s| s == b"Restart"));
        }
        unsafe extern "C" fn selected(command: u32, data: *mut c_void) -> bool {
            if command != ffi::GET_VARIABLE { return false; }
            let variable = &mut *data.cast::<ffi::Variable>();
            let key = CStr::from_ptr(variable.key);
            variable.value = if key == mvd::KEYS[0] { c"right_stick".as_ptr() }
                else if key == mvd::KEYS[1] { c"90".as_ptr() }
                else if key == mvd::KEYS[2] { c"0".as_ptr() }
                else { return false; };
            true
        }
        assert_eq!(mvd_settings(selected), mvd::Settings { mode: mvd::Mode::RightStick, range: [90, 0], gravity: true, drift_compensation: 50, sensor_diagnostics: false });
        unsafe extern "C" fn visible(command: u32, data: *mut c_void) -> bool {
            assert_eq!(command, ffi::SET_CORE_OPTIONS_DISPLAY);
            assert!((*data.cast::<ffi::OptionDisplay>()).visible);
            true
        }
        publish_global_option_visibility(visible, &AUDIO_GAIN_SOURCES);
    }

    #[test]
    fn flight_shoulders_match_face_actions_and_preserve_port_isolation() {
        use ffi::*;
        for profile in [
            Profile::WingWar,
            Profile::WingWar360,
            Profile::StarWarsArcade,
        ] {
            for (face, shoulder) in [
                (
                    port_button::<0, JOY_B> as InputState,
                    port_button::<0, JOY_L> as InputState,
                ),
                (port_button::<0, JOY_A>, port_button::<0, JOY_R>),
            ] {
                let expected = flight_inputs(profile, Some(face), [DEVICE_JOYPAD; 2]);
                let actual = flight_inputs(profile, Some(shoulder), [DEVICE_JOYPAD; 2]);
                assert_eq!(actual.in1, expected.in1);
                assert_ne!(actual.in1, 0xff);
                assert_eq!(
                    flight_inputs(profile, Some(shoulder), [DEVICE_NONE, DEVICE_JOYPAD]).in1,
                    0xff
                );
            }
        }
        for (callback, bit) in [
            (port_button::<1, JOY_L> as InputState, 0x04),
            (port_button::<1, JOY_R>, 0x08),
        ] {
            let native = flight_inputs(Profile::StarWarsArcade, Some(callback), [DEVICE_JOYPAD; 2]);
            assert_eq!(native.in1, 0xff ^ bit);
            assert_eq!(
                flight_inputs(
                    Profile::StarWarsArcade,
                    Some(callback),
                    [DEVICE_JOYPAD, DEVICE_NONE]
                )
                .in1,
                0xff
            );
        }
    }

    #[test]
    fn flight_midpoint_and_throttle_follow_profile_polarity() {
        use ffi::*;
        for (profile, midpoint) in [
            (Profile::WingWar, 64),
            (Profile::WingWar360, 192),
            (Profile::StarWarsArcade, 77),
            (Profile::NetMerc, 191),
        ] {
            let input = flight_inputs(
                profile,
                Some(axis_value::<0, ANALOG_LEFT, ANALOG_X, 16384>),
                [DEVICE_JOYPAD; 2],
            );
            assert_eq!(input.analog[0], midpoint);
        }
        let gunner = flight_inputs(
            Profile::StarWarsArcade,
            Some(axis_value::<1, ANALOG_LEFT, ANALOG_X, 16384>),
            [DEVICE_JOYPAD; 2],
        );
        assert_eq!(gunner.analog[4], 77);
        assert_eq!(gunner.analog[0], 127);
        for profile in [
            Profile::WingWar,
            Profile::WingWar360,
            Profile::StarWarsArcade,
        ] {
            let up = flight_inputs(
                profile,
                Some(axis_value::<0, ANALOG_RIGHT, ANALOG_Y, -16384>),
                [DEVICE_JOYPAD; 2],
            );
            let down = flight_inputs(
                profile,
                Some(axis_value::<0, ANALOG_RIGHT, ANALOG_Y, 16384>),
                [DEVICE_JOYPAD; 2],
            );
            assert_eq!(
                up.analog[2],
                if profile == Profile::StarWarsArcade {
                    178
                } else {
                    65
                }
            );
            assert_eq!(
                down.analog[2],
                if profile == Profile::StarWarsArcade {
                    78
                } else {
                    192
                }
            );
            assert_eq!(
                flight_inputs(
                    profile,
                    Some(axis_value::<1, ANALOG_RIGHT, ANALOG_Y, -32768>),
                    [DEVICE_JOYPAD; 2]
                )
                .analog[2],
                128
            );
        }
    }

    #[test]
    fn wing_war_trigger_swap_preserves_adc_response_and_descriptors() {
        use ffi::*;
        for profile in [Profile::WingWar, Profile::WingWar360] {
            let entries = descriptors(profile);
            assert!(entries.iter().any(|entry| entry.3 == JOY_L2 && entry.4 == c"Throttle Down"));
            assert!(entries.iter().any(|entry| entry.3 == JOY_R2 && entry.4 == c"Throttle Up"));
            for (source, expected) in [
                (axis_value::<0, ANALOG_BUTTON, JOY_L2, 32767> as InputState, 255),
                (axis_value::<0, ANALOG_BUTTON, JOY_R2, 32767>, 1),
            ] {
                assert_eq!(
                    flight_inputs(profile, Some(source), [DEVICE_JOYPAD; 2]).analog[2],
                    expected
                );
            }
        }
        let swa = descriptors(Profile::StarWarsArcade);
        assert!(swa.iter().any(|entry| entry.3 == JOY_L2 && entry.4 == c"Pilot Throttle Up"));
        assert!(swa.iter().any(|entry| entry.3 == JOY_R2 && entry.4 == c"Pilot Throttle Down"));
    }

    #[test]
    fn both_ports_keep_the_family_profile_and_operator_controls() {
        use ffi::*;
        for profile in [
            Profile::VirtuaRacing,
            Profile::VirtuaFormula,
            Profile::VirtuaFighter,
            Profile::WingWar,
            Profile::WingWar360,
            Profile::StarWarsArcade,
            Profile::NetMerc,
        ] {
            let info = ControllerStorage::new(profile);
            assert_eq!(
                (
                    info.ports[0].num_types,
                    info.ports[1].num_types,
                    info.ports[2].num_types
                ),
                (2, 2, 0)
            );
            if profile != Profile::StarWarsArcade {
                assert_eq!(
                    unsafe { CStr::from_ptr(info.types[0][1].desc) },
                    unsafe { CStr::from_ptr(info.types[1][1].desc) }
                );
            }
            let entries = descriptors(profile);
            for port in 0..2 {
                assert_eq!(info.types[port][0].id, DEVICE_JOYPAD);
                assert_eq!(info.types[port][1].id, NO_SERVICE_DEVICE);
                assert_eq!(unsafe { CStr::from_ptr(info.types[port][0].desc) }.to_str().unwrap(),
                    format!("{} + Test/Service Slots", unsafe { CStr::from_ptr(info.types[port][1].desc) }.to_str().unwrap()));
                for (id, label) in [(JOY_L3, c"Test"), (JOY_R3, c"Service")] {
                    assert_eq!(
                        entries
                            .iter()
                            .filter(|(p, _, _, b, l)| *p == port as u32 && *b == id && *l == label)
                            .count(),
                        1
                    );
                }
            }
            let coin2 = !matches!(profile, Profile::WingWar | Profile::NetMerc);
            assert_eq!(
                entries
                    .iter()
                    .any(|(p, _, _, b, _)| *p == 1 && *b == JOY_SELECT),
                coin2
            );
            for device in [DEVICE_JOYPAD, NO_SERVICE_DEVICE, DEVICE_NONE] {
                let enabled = joypad_enabled(device);
                let devices = [
                    DEVICE_NONE,
                    device,
                ];
                let inputs = match profile {
                    Profile::VirtuaRacing | Profile::VirtuaFormula => {
                        vr_inputs(&mut 128, &mut 32, &mut 32, Some(p2_operators), devices)
                    }
                    Profile::VirtuaFighter => vf_inputs(Some(p2_operators), devices),
                    _ => flight_inputs(profile, Some(p2_operators), devices),
                };
                assert_eq!(inputs.in0 & 0x0c, if device == DEVICE_JOYPAD { 0 } else { 0x0c });
                assert_eq!(inputs.in0 & 0x02, if enabled && coin2 { 0 } else { 0x02 });
                assert_eq!(inputs.in0 & 0x01, 0x01);
            }
        }
    }

    #[test]
    fn reduced_profiles_preserve_gameplay_axes_and_mvd_commands() {
        use ffi::*;
        unsafe extern "C" fn held(_: u32, device: u32, _: u32, _: u32) -> i16 {
            if device == DEVICE_ANALOG { 12000 } else { 1 }
        }
        for profile in [Profile::VirtuaRacing, Profile::VirtuaFormula, Profile::VirtuaFighter,
            Profile::WingWar, Profile::WingWar360, Profile::StarWarsArcade, Profile::NetMerc] {
            let read = |devices| match profile {
                Profile::VirtuaRacing | Profile::VirtuaFormula =>
                    vr_inputs(&mut 128, &mut 32, &mut 32, Some(held), devices),
                Profile::VirtuaFighter => vf_inputs(Some(held), devices),
                _ => flight_inputs(profile, Some(held), devices),
            };
            let full = read([DEVICE_JOYPAD; 2]);
            let reduced = read([NO_SERVICE_DEVICE; 2]);
            assert_eq!(full.in0 & 0x0c, 0);
            assert_eq!(reduced.in0 & 0x0c, 0x0c);
            assert_eq!(full.in0 | 0x0c, reduced.in0);
            assert_eq!((full.in1, full.in2, full.analog, full.steer, full.accel, full.brake),
                (reduced.in1, reduced.in2, reduced.analog, reduced.steer, reduced.accel, reduced.brake));
        }
        let (full, full_commands) = mvd_input(Some(held), DEVICE_JOYPAD, mvd::Settings::default());
        let (reduced, reduced_commands) = mvd_input(Some(held), NO_SERVICE_DEVICE, mvd::Settings::default());
        assert_eq!(full, reduced);
        assert_eq!(full_commands, reduced_commands);
    }

    #[test]
    fn controller_profiles_name_each_cabinet_port_and_disable_input() {
        let swa = ControllerStorage::new(Profile::StarWarsArcade);
        assert_eq!(swa.ports[0].num_types, 2);
        assert_eq!(swa.ports[1].num_types, 2);
        assert_eq!(swa.ports[2].num_types, 0);
        assert_eq!(
            unsafe { CStr::from_ptr(swa.types[0][1].desc) },
            c"Flight: Star Wars Arcade (Pilot) + VR1"
        );
        assert_eq!(
            unsafe { CStr::from_ptr(swa.types[1][1].desc) },
            c"Flight: Star Wars Arcade (Gunner) + VR1"
        );
        let formula = ControllerStorage::new(Profile::VirtuaFormula);
        assert_eq!(
            unsafe { CStr::from_ptr(formula.types[0][1].desc) },
            c"Driving: Sequential + VR4"
        );
        assert_eq!(formula.ports[1].num_types, 2);
        let fighter = vf_inputs(Some(p2_only), [ffi::DEVICE_JOYPAD, ffi::DEVICE_NONE]);
        assert_eq!(fighter.in2, 0xff);
        assert_eq!(fighter.in0 & 0x22, 0x22);
        let star_wars = flight_inputs(
            Profile::StarWarsArcade,
            Some(p2_only),
            [ffi::DEVICE_JOYPAD, ffi::DEVICE_NONE],
        );
        assert_eq!(star_wars.in1 & 0x0c, 0x0c);
        assert_eq!(star_wars.analog[4..6], [127, 127]);
    }
}
