//! Host input sampling and native Model 1 / Model 2 cabinet inputs.
//!
//! Daytona is a dedicated cabinet: a 270-degree wheel, two pedals, a 4-speed
//! H-pattern shifter and four coloured view buttons. None of that maps onto a
//! pad one-for-one, so what follows is the usual console-conversion compromise:
//! the wheel becomes the left stick, the pedals become the triggers, and the
//! H-pattern becomes a sequential shifter on the shoulder buttons.
//!
//! The analog ranges are the hardware's, not 0..255: the reference port definitions
//! give steering, throttle and brake a travel of 0x20..0xe0, centred at 0x80 for
//! the wheel and resting at 0x20 for both pedals.

mod cabinet;
mod model1_rumble;
mod model2_rumble;
#[cfg(test)]
mod player_tests;
pub mod players;
mod sampling;
#[cfg(target_os = "macos")]
mod sdl_pads;
pub mod signals;
use model1_rumble::Model1PadRumble;
use model2_rumble::Model2PadRumble;
use players::{PadAssignments, PadDevice, Player};
use signals::Signal;
use std::collections::{HashMap, HashSet};

use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Repeat, Replay, Ticks};
use winit::keyboard::KeyCode;

use tgpulse_core::config::Inputs;
use tgpulse_core::model2_drive::Protocol as Model2DriveProtocol;

use crate::bindings::{Bindings, GamepadBackend, Sign, Source};

// --- Legacy fallback for unaudited Model 1 outputs --------------------------
//
// Recovered by disassembling Daytona's drive board Z80 ROM (epr-16488a): the
// dispatch at 0x0520 debounces the byte, then 0x0328 selects the effect from
// `cmd & 0xf8` while 0x04c0 takes `cmd & 0x07` as the magnitude and scales the
// motor target with it. `0x10` lands on 0x04d4, which zeroes the target -- no
// force.
//
// This old intensity-only path remains for Model 1 titles other than VR until
// their output protocol has been verified. Model 2 uses model2_drive instead.
/// Mask selecting the effect kind.
const DRIVE_KIND: u8 = 0xf8;
/// Mask selecting the force magnitude, 0..7.
const DRIVE_MAGNITUDE: u8 = 0x07;
/// Effect kind that means "no force".
const DRIVE_KIND_OFF: u8 = 0x10;
/// Only this range reaches the motor. The dispatch peels off `0x0x`, `0x7x`,
/// `0x8x` and `0xfx` before that -- they are init and mode traffic, not force,
/// and reading a magnitude out of them would rumble on handshakes.
const DRIVE_FORCE_FIRST: u8 = 0x10;
const DRIVE_FORCE_LAST: u8 = 0x6f;

/// Travel limits of the I/O board's ADC channels.
const ANALOG_MIN: i32 = 0x20;
const ANALOG_MAX: i32 = 0xe0;
/// Wheel centre.
const STEER_CENTRE: i32 = 0x80;

/// Per-frame travel when an axis is driven from a key rather than a stick.
/// These are the reference values for this game.
const STEER_KEYDELTA: i32 = 10;
const PEDAL_KEYDELTA: i32 = 20;

/// Stick deflection below which the wheel is treated as centred.
const STICK_DEADZONE: f32 = 0.15;
/// Trigger travel below which a pedal is treated as released.
const TRIGGER_DEADZONE: f32 = 0.05;

// Star Wars' analog channels are two-axis flight sticks rather than a wheel with an 0x80 centre. The I/O
// board reads them on the same ADC channels 0/1/2 the racers use for
// wheel/accel/brake, so they travel out through `steer`/`accel`/`brake`:
// channel 0 = stick X, 1 = stick Y, 2 = throttle.
const SWA_CENTRE: i32 = 0x7f;
const SWA_STICK_RANGE: i32 = 100; // half-travel -> 0x1b..0xe3

// Virtua Cop lightgun. The gun trigger and the on-board start pump IN.1 bit 0
// (P1) / bit 1 (P2). The ADC coordinates span the cabinet's travel limits for P1_X and
// P1_Y; the front end maps the mouse across the window into these.
const IN1_VCOP_TRIGGER: u8 = 0x01;
const GUN_X_MIN: i32 = 0x083;
const GUN_X_MAX: i32 = 0x276;
const GUN_Y_MIN: i32 = 0x024;
const GUN_Y_MAX: i32 = 0x1a9;

/// Highest selectable gear; 0 is neutral.
const TOP_GEAR: usize = 4;

pub struct InputState {
    gilrs: Option<gilrs::Gilrs>,
    #[cfg(target_os = "macos")]
    sdl: Option<sdl_pads::SdlPads>,
    // Cache logical events, not native codes: gilrs' unmapped-button fallback
    // can alias an SDL-mapped button (e.g. Xbox R3 and D-pad Right on macOS).
    pad_buttons: HashMap<gilrs::GamepadId, HashSet<gilrs::Button>>,
    assignments: PadAssignments,
    rumble_pad: Option<usize>,
    /// A single always-running rumble effect whose gain we scale, rather than
    /// rebuilding an effect every time the game changes force.
    rumble: Option<Effect>,
    rumble_gain: f32,
    rumble_enabled: bool,
    model1_rumble: Model1PadRumble,
    model2_rumble: Model2PadRumble,
    keys: HashSet<KeyCode>,
    /// 0 = neutral, 1..4 = gears.
    gear: usize,
    /// Shoulder buttons shift once per press, not once per frame held.
    shift_up_held: bool,
    shift_down_held: bool,
    /// Current analog positions, retained between frames so key-driven axes can
    /// travel gradually the way a real pedal or wheel does.
    steer: i32,
    accel: i32,
    brake: i32,
    /// Mouse cursor as a fraction of the render area, [0, 1] left-to-right and
    /// top-to-bottom. Drives the lightgun aim.
    cursor: (f32, f32),
    /// P2 gun cursor, independent of the P1 mouse and controller.
    cursor_p2: (f32, f32),
    cursor_p2_active: bool,
    /// Left mouse button (fire) and right (reload / point off-screen).
    mouse_fire: bool,
    mouse_reload: bool,
    /// Which game's control layout `poll` emits. The IN.1 bits and analog
    /// channels mean entirely different things between a racer, Virtua
    /// the joystick games' stick + buttons, Star Wars' flight sticks, and Virtua
    /// Cop's lightgun.
    scheme: ControlScheme,
    game: String,
    special_shift: bool,
    special_shift_held: bool,
    return_to_menu_held: bool,
    /// The ADC channel wiring of the loaded cabinet.
    analog_roles: [AnalogRole; 8],
    /// What the player has bound each control to.
    bindings: Bindings,
    /// On-screen signals: 0..1 for buttons, -1..1 for signed axes. These
    /// sit alongside the bound sources rather than inside them: a thumb is not
    /// a key or a pad button, and the player never bound it to anything.
    touch: Vec<(Signal, f32)>,
    /// A pad the platform reports for itself.
    external: ExternalPad,
    external_p2: ExternalPad,
}

/// A gamepad the platform hands over directly.
///
/// gilrs has no Android backend, so on a handset a controller arrives as
/// activity key events instead of through the library. Those are translated
/// into the same `gilrs::Button` and `gilrs::Axis` values a desktop pad
/// produces, which means the bindings -- including anything the player has
/// rebound -- resolve identically on both.
#[derive(Default)]
struct ExternalPad {
    buttons: HashSet<gilrs::Button>,
    left_x: f32,
    left_y: f32,
    /// Set once anything has been heard from the pad. Without it an absent
    /// controller would look like one resting perfectly at centre, and the
    /// analog paths would believe it.
    present: bool,
}

impl ExternalPad {
    fn axis(&self, axis: gilrs::Axis) -> f32 {
        match axis {
            gilrs::Axis::LeftStickX => self.left_x,
            gilrs::Axis::LeftStickY => self.left_y,
            _ => 0.0,
        }
    }
}

pub use tgpulse_core::roms_db::{AnalogRole, Scheme as ControlScheme};

/// Every physical axis a Model 1/2 cabinet can present, in the I/O chip's
/// 0..0xff ADC units. A scheme fills the ones its cabinet has; `scatter` then
/// places them on the channels that game's board actually wires them to.
#[derive(Clone, Copy)]
pub struct Axes {
    pub steer: u8,
    pub accel: u8,
    pub brake: u8,
    pub throttle: u8,
    pub stickx: u8,
    pub sticky: u8,
    pub stick2x: u8,
    pub stick2y: u8,
    pub gun1x: u8,
    pub gun1y: u8,
    pub gun2x: u8,
    pub gun2y: u8,
    pub roll: u8,
    pub pitch: u8,
    pub slide: u8,
    pub curving: u8,
    pub swing: u8,
    pub incline: u8,
    pub bat1: u8,
    pub bat2: u8,
    pub p1r: u8,
    pub p1l: u8,
    pub p2r: u8,
    pub p2l: u8,
}

impl Default for Axes {
    /// Everything centred: an untouched cabinet reads mid-scale on every
    /// channel, which is also what the cabinet's resting positions give.
    fn default() -> Self {
        Self {
            steer: 0x80,
            accel: 0x00,
            brake: 0x00,
            throttle: 0x00,
            stickx: 0x80,
            sticky: 0x80,
            stick2x: 0x80,
            stick2y: 0x80,
            gun1x: 0x80,
            gun1y: 0x80,
            gun2x: 0x80,
            gun2y: 0x80,
            roll: 0x80,
            pitch: 0x80,
            slide: 0x80,
            curving: 0x80,
            swing: 0x80,
            incline: 0x80,
            bat1: 0x00,
            bat2: 0x00,
            p1r: 0x00,
            p1l: 0x00,
            p2r: 0x00,
            p2l: 0x00,
        }
    }
}

impl Axes {
    fn by_role(&self, role: AnalogRole) -> u8 {
        use AnalogRole::*;
        match role {
            None => 0xff,
            Steer => self.steer,
            Accel => self.accel,
            Brake => self.brake,
            Throttle => self.throttle,
            StickX => self.stickx,
            StickY => self.sticky,
            Stick2X => self.stick2x,
            Stick2Y => self.stick2y,
            Gun1X => self.gun1x,
            Gun1Y => self.gun1y,
            Gun2X => self.gun2x,
            Gun2Y => self.gun2y,
            Roll => self.roll,
            Pitch => self.pitch,
            Slide => self.slide,
            Curving => self.curving,
            Swing => self.swing,
            Incline => self.incline,
            Bat1 => self.bat1,
            Bat2 => self.bat2,
            P1R => self.p1r,
            P1L => self.p1l,
            P2R => self.p2r,
            P2L => self.p2l,
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => {
                for (_id, pad) in g.gamepads() {
                    log::info!(target: "input", "gamepad: {}", pad.name());
                }
                Some(g)
            }
            Err(e) => {
                log::warn!(target: "input", "no gamepad support: {e}");
                None
            }
        };
        Self::with_gilrs(gilrs)
    }

    fn with_gilrs(gilrs: Option<gilrs::Gilrs>) -> Self {
        let mut state = Self {
            gilrs,
            #[cfg(target_os = "macos")]
            sdl: None,
            pad_buttons: HashMap::new(),
            assignments: PadAssignments::default(),
            rumble_pad: None,
            rumble: None,
            rumble_gain: -1.0,
            rumble_enabled: false,
            model1_rumble: Model1PadRumble::default(),
            model2_rumble: Model2PadRumble::default(),
            keys: HashSet::new(),
            gear: 0,
            shift_up_held: false,
            shift_down_held: false,
            steer: STEER_CENTRE,
            accel: ANALOG_MIN,
            brake: ANALOG_MIN,
            cursor: (0.5, 0.5),
            cursor_p2: (0.5, 0.5),
            cursor_p2_active: false,
            mouse_fire: false,
            mouse_reload: false,
            scheme: ControlScheme::Racing,
            game: String::new(),
            special_shift: false,
            special_shift_held: false,
            return_to_menu_held: false,
            analog_roles: [AnalogRole::None; 8],
            bindings: Bindings::default(),
            touch: Vec::new(),
            external: ExternalPad::default(),
            external_p2: ExternalPad::default(),
        };
        state.refresh_controllers();
        state
    }

    /// Publishes what the on-screen controls are asking for. Called once a
    /// frame with the overlay's current state.
    pub fn set_touch(&mut self, amounts: &[(Signal, f32)]) {
        self.touch.clear();
        self.touch.extend_from_slice(amounts);
    }

    /// A button on a pad the platform reports rather than gilrs. Only Android
    /// has such a pad; elsewhere gilrs sees every controller itself.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub fn set_pad_button(&mut self, button: gilrs::Button, pressed: bool) {
        self.external.present = true;
        if pressed {
            self.external.buttons.insert(button);
        } else {
            self.external.buttons.remove(&button);
        }
    }

    /// The left stick of such a pad, each axis in -1..1 and positive up.
    pub fn set_pad_stick(&mut self, x: f32, y: f32) {
        self.external.present = true;
        self.external.left_x = x.clamp(-1.0, 1.0);
        self.external.left_y = y.clamp(-1.0, 1.0);
    }

    /// Which physical axis each of the eight ADC channels carries, taken from
    /// the ROM database so a cabinet's wiring is data, not code.
    pub fn set_analog_roles(&mut self, mut roles: [AnalogRole; 8]) {
        // The tested SM2 layout takes precedence over the MAME-generated DB.
        // Guard the old order so regenerating corrected metadata is harmless.
        if self.game == "skisuprg" && roles[..2] == [AnalogRole::Swing, AnalogRole::Incline] {
            roles.swap(0, 1);
        }
        self.analog_roles = roles;
    }

    /// Places the axes a scheme produced onto the channels this game reads.
    fn scatter(&self, axes: &Axes, out: &mut Inputs) {
        for (ch, role) in self.analog_roles.iter().enumerate() {
            out.analog[ch] = self.cabinet_axis(*role, axes);
        }
        // The original I/O board reads these mirrors rather than the mux.
        // Mirror the *routed channels*, including NetMerc's Y on channel 2.
        if self.analog_roles.iter().any(|r| *r != AnalogRole::None) {
            [out.steer, out.accel, out.brake] = out.analog[..3].try_into().unwrap();
        } else {
            out.steer = axes.steer;
            out.accel = axes.accel;
            out.brake = axes.brake;
        }
    }

    /// Records the mouse position as a fraction of the render area (0..1).
    pub fn on_cursor(&mut self, x: f32, y: f32) {
        self.cursor = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
    }

    /// The lightgun aim as a fraction of the render area (0..1), for drawing
    /// the on-screen crosshair. Follows the mouse or the bound gun axes.
    pub fn aim(&self) -> (f32, f32) {
        self.cursor
    }

    pub fn aim_p2(&self) -> Option<(f32, f32)> {
        // Keep the idle single-player screen unchanged. A selected P2 pad or
        // a moved/rebound P2 cursor makes the second reticle useful.
        (self.has_pad(Player::Two) || self.cursor_p2_active).then_some(self.cursor_p2)
    }

    /// Records the mouse buttons: left fires, right reloads (points off-screen).
    pub fn on_mouse_button(&mut self, left: Option<bool>, right: Option<bool>) {
        if let Some(l) = left {
            self.mouse_fire = l;
        }
        if let Some(r) = right {
            self.mouse_reload = r;
        }
    }

    /// Selects the control layout `poll` emits for the running game.
    pub fn set_scheme(&mut self, scheme: ControlScheme) {
        self.scheme = scheme;
    }

    /// Builds the one rumble effect we keep alive, at full magnitude. Force is
    /// applied by scaling its gain, which is cheap enough to do every frame.
    fn build_rumble(g: &mut gilrs::Gilrs, selected: usize) -> Option<Effect> {
        let pad = g
            .gamepads()
            .find(|(id, _)| usize::from(*id) == selected)
            .map(|(id, _)| id)?;
        let eff = EffectBuilder::new()
            .add_effect(BaseEffect {
                kind: BaseEffectType::Strong {
                    magnitude: u16::MAX,
                },
                scheduling: Replay {
                    play_for: Ticks::from_ms(1_000),
                    ..Default::default()
                },
                envelope: Default::default(),
            })
            .add_effect(BaseEffect {
                kind: BaseEffectType::Weak {
                    magnitude: u16::MAX,
                },
                scheduling: Replay {
                    play_for: Ticks::from_ms(1_000),
                    ..Default::default()
                },
                envelope: Default::default(),
            })
            .repeat(Repeat::Infinitely)
            .gamepads(&[pad])
            .finish(g)
            .ok()?;
        // Start silent; the game decides from here.
        eff.set_gain(0.0).ok()?;
        eff.play().ok()?;
        log::info!(target: "input", "force feedback available");
        Some(eff)
    }

    fn send_rumble(&mut self, low: f32, high: f32) {
        // Gilrs exposes one effect gain here; SDL3 can retain the two motors.
        let gain = low.max(high);
        if (gain - self.rumble_gain).abs() > f32::EPSILON {
            if let Some(eff) = self.rumble.as_ref() {
                let _ = eff.set_gain(gain);
            }
            self.rumble_gain = gain;
        }
        // SDL rumble expires automatically. Renew it while emulation runs;
        // a paused or stopped machine cannot leave a motor running.
        #[cfg(target_os = "macos")]
        if let (Some(sdl), Some(id)) = (&mut self.sdl, self.rumble_pad) {
            sdl.rumble(id, low, high);
        }
    }

    /// Applies an original Model 1 VR-family motor command to the pad adapter.
    pub fn set_model1_rumble(&mut self, cmd: u8, steer: u8) {
        if !self.rumble_enabled {
            return;
        }
        let (low, high) = self.model1_rumble.frame(cmd, steer);
        self.send_rumble(low, high);
    }

    pub fn reset_model1_rumble(&mut self) {
        self.model1_rumble.reset();
        self.send_rumble(0.0, 0.0);
    }

    pub fn set_model2_rumble(&mut self, set: &str, cmd: u8, steer: u8) {
        if !self.rumble_enabled {
            return;
        }
        let levels = if let Some(protocol) = Model2DriveProtocol::for_set(set) {
            self.model2_rumble.frame(protocol, cmd, steer)
        } else {
            self.model2_rumble.reset();
            (0.0, 0.0)
        };
        self.send_rumble(levels.0, levels.1);
    }

    /// Legacy intensity approximation for unaudited Model 1 boards only.
    pub fn set_legacy_model1_rumble(&mut self, cmd: u8) {
        if !self.rumble_enabled {
            return;
        }
        if !(DRIVE_FORCE_FIRST..=DRIVE_FORCE_LAST).contains(&cmd) {
            return; // not a force command; leave the motors as they were
        }
        let gain = if cmd & DRIVE_KIND == DRIVE_KIND_OFF {
            0.0
        } else {
            (cmd & DRIVE_MAGNITUDE) as f32 / DRIVE_MAGNITUDE as f32
        };
        self.send_rumble(gain, gain);
    }

    pub fn enable_rumble(&mut self, on: bool) {
        self.rumble_enabled = on;
        if !on {
            self.reset_model1_rumble();
            self.model2_rumble.reset();
        }
    }

    pub fn on_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    fn held(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    pub fn set_bindings(&mut self, bindings: Bindings) {
        #[cfg(target_os = "macos")]
        if bindings.gamepad_backend != self.bindings.gamepad_backend {
            let rumble_was_enabled = self.rumble_enabled;
            self.enable_rumble(false);
            self.rumble = None;
            self.rumble_pad = None;
            self.pad_buttons.clear();
            self.assignments = PadAssignments::default();
            match bindings.gamepad_backend {
                GamepadBackend::Sdl3 => {
                    self.gilrs = None;
                    self.sdl = match sdl_pads::SdlPads::new() {
                        Ok(sdl) => Some(sdl),
                        Err(e) => {
                            log::error!(target: "input", "SDL3 gamepad backend unavailable: {e}");
                            None
                        }
                    };
                }
                GamepadBackend::Gilrs => {
                    self.sdl = None;
                    self.gilrs = gilrs::Gilrs::new()
                        .map_err(|e| {
                            log::error!(target: "input", "gilrs gamepad backend unavailable: {e}");
                        })
                        .ok();
                }
            }
            self.rumble_enabled = rumble_was_enabled;
        }
        self.bindings = bindings;
        self.refresh_controllers();
    }

    pub fn controller_devices(&self) -> Vec<PadDevice> {
        self.assignments.devices.clone()
    }
    pub fn controller_labels(&self) -> [String; 2] {
        Player::ALL.map(|p| self.assignments.label(p))
    }
    /// Keep device identity/held state across library -> game, without carrying
    /// the previous cabinet's gear, analog ramps or game routing into the new one.
    pub fn retain_controllers_from(&mut self, previous: &mut Self) {
        // The transferred SDL event pump is unique to the active input state.
        // Preserve its backend selection so set_bindings does not try to make
        // a second event pump during the menu-to-game transition.
        self.bindings.gamepad_backend = previous.bindings.gamepad_backend;
        std::mem::swap(&mut self.gilrs, &mut previous.gilrs);
        #[cfg(target_os = "macos")]
        std::mem::swap(&mut self.sdl, &mut previous.sdl);
        std::mem::swap(&mut self.assignments, &mut previous.assignments);
        std::mem::swap(&mut self.pad_buttons, &mut previous.pad_buttons);
        std::mem::swap(&mut self.rumble, &mut previous.rumble);
        std::mem::swap(&mut self.rumble_pad, &mut previous.rumble_pad);
        std::mem::swap(&mut self.external, &mut previous.external);
        std::mem::swap(&mut self.external_p2, &mut previous.external_p2);
        self.return_to_menu_held = previous.return_to_menu_held;
        self.rumble_gain = -1.0;
        self.model1_rumble.reset();
        self.model2_rumble.reset();
    }
    fn refresh_controllers(&mut self) {
        let mut connected = self
            .gilrs
            .as_ref()
            .map(|g| {
                g.gamepads()
                    .map(|(id, pad)| {
                        let uuid = pad
                            .uuid()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>();
                        (usize::from(id), uuid, pad.name().to_owned())
                    })
                    .collect()
            })
            .unwrap_or_default();
        #[cfg(target_os = "macos")]
        if let Some(sdl) = &self.sdl {
            connected = sdl.devices();
        }
        self.assignments.observe(connected);
        self.assignments.resolve(&self.bindings.controllers);
        let selected = self.assignments.id(Player::One);
        if selected != self.rumble_pad {
            if let Some(effect) = self.rumble.take() {
                let _ = effect.stop();
            }
            self.rumble =
                selected.and_then(|id| self.gilrs.as_mut().and_then(|g| Self::build_rumble(g, id)));
            self.rumble_pad = selected;
            self.rumble_gain = -1.0;
            self.model1_rumble.reset();
            self.model2_rumble.reset();
        }
    }

    /// Native P1 signal supplied by the touch overlay, independent of bindings.
    fn touch_amount(&self, control: Signal) -> f32 {
        self.touch
            .iter()
            .find(|(c, _)| *c == control)
            .map_or(0.0, |(_, amount)| *amount)
    }

    /// Whether something with real travel is driving the axes.
    ///
    /// A keyboard is not: its controls are on or off, so the schemes ramp them
    /// toward the ends of the range instead of reading them as positions. A
    /// stick, a trigger or a thumb on the screen has a position of its own and
    /// is read directly.
    fn has_analog(&self) -> bool {
        self.has_pad(Player::One) || self.external.present || !self.touch.is_empty()
    }

    /// The travel of one pad axis, from whichever device is reporting it.
    fn external_for(&self, player: Player) -> &ExternalPad {
        if player == Player::One {
            &self.external
        } else {
            &self.external_p2
        }
    }
    fn axis_value_for(&self, player: Player, axis: gilrs::Axis) -> f32 {
        let mut hardware = self.pad_for(player).map_or(0.0, |pad| {
            mapped_axis_value(
                axis,
                pad.axis_code(axis).map(|_| pad.value(axis)),
                |button| pad.button_data(button).map_or(0.0, |data| data.value()),
            )
        });
        #[cfg(target_os = "macos")]
        if let (Some(sdl), Some(id)) = (&self.sdl, self.assignments.id(player)) {
            hardware = sdl.axis(id, axis);
        }
        let external = self.external_for(player).axis(axis);
        if external.abs() > hardware.abs() {
            external
        } else {
            hardware
        }
    }

    fn source_amount(&self, source: Source) -> f32 {
        self.source_amount_for(Player::One, source)
    }
    fn source_amount_for(&self, player: Player, source: Source) -> f32 {
        match source {
            Source::Key(k) => f32::from(u8::from(self.held(k))),
            Source::Pad(b) => {
                let mut hardware = self.pad_for(player).is_some_and(|pad| {
                    self.pad_buttons
                        .get(&pad.id())
                        .is_some_and(|buttons| buttons.contains(&b))
                });
                #[cfg(target_os = "macos")]
                if let (Some(sdl), Some(id)) = (&self.sdl, self.assignments.id(player)) {
                    hardware = sdl.button(id, b);
                }
                f32::from(u8::from(
                    hardware || self.external_for(player).buttons.contains(&b),
                ))
            }
            Source::PadAxis(a, sign) => {
                let value = self.axis_value_for(player, a);
                // Triggers rest at zero and only travel positive; sticks rest
                // centred and travel both ways. The deadzone differs to match.
                let value = match sign {
                    Sign::Positive => value,
                    Sign::Negative => -value,
                };
                let deadzone = match a {
                    gilrs::Axis::LeftZ | gilrs::Axis::RightZ => TRIGGER_DEADZONE,
                    _ => STICK_DEADZONE,
                };
                if value > deadzone {
                    value.min(1.0)
                } else {
                    0.0
                }
            }
        }
    }

    fn has_pad(&self, player: Player) -> bool {
        let Some(id) = self.assignments.id(player) else {
            return false;
        };
        #[cfg(target_os = "macos")]
        if let Some(sdl) = &self.sdl {
            return sdl.has(id);
        }
        self.pad_for(player).is_some()
    }
    fn pad_for(&self, player: Player) -> Option<gilrs::Gamepad<'_>> {
        let selected = self.assignments.id(player)?;
        self.gilrs.as_ref().and_then(|g| {
            g.gamepads()
                .find(|(id, _)| usize::from(*id) == selected)
                .map(|(_, pad)| pad)
        })
    }

    /// Moves `axis` toward `target` by at most `delta`.
    fn approach(axis: i32, target: i32, delta: i32) -> i32 {
        if axis < target {
            (axis + delta).min(target)
        } else {
            (axis - delta).max(target)
        }
    }

    fn shift(&mut self, up: bool) {
        if up {
            self.gear = (self.gear + 1).min(TOP_GEAR);
        } else {
            self.gear = self.gear.saturating_sub(1);
        }
    }

    /// Samples every device and publishes the result the way the I/O board sees
    /// it. Call once per emulated frame, before the board's input command runs.
    pub fn poll(&mut self, out: &mut Inputs) {
        match self.scheme {
            ControlScheme::Racing | ControlScheme::Bike => self.poll_native_racing(out),
            ControlScheme::Joystick => self.poll_native_joystick(out),
            ControlScheme::Flight => self.poll_native_flight(out),
            ControlScheme::Gun => self.poll_native_gun(out),
            ControlScheme::Jetski
            | ControlScheme::Skate
            | ControlScheme::Ski
            | ControlScheme::Sled => self.poll_native_body(out),
        }
    }

    /// Sample devices before advancing the machine, even while paused.
    /// Consume a complete exit chord before its Start reaches the cabinet.
    pub fn return_to_menu_requested(&mut self) -> bool {
        // Drain the event queue so gilrs keeps its button/axis state current.
        if let Some(g) = self.gilrs.as_mut() {
            while let Some(event) = g.next_event() {
                if matches!(event.event, gilrs::EventType::Disconnected) {
                    self.pad_buttons.remove(&event.id);
                } else {
                    update_pad_buttons(self.pad_buttons.entry(event.id).or_default(), event.event);
                }
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(sdl) = self.sdl.as_mut() {
            sdl.poll();
        }
        self.refresh_controllers();

        let active = self.bindings.return_to_menu.value(|atom| match atom {
            signals::expression::Atom::Source(source) => self.source_amount(*source),
            _ => 0.0,
        }) > 0.5;
        let pressed = active && !self.return_to_menu_held;
        self.return_to_menu_held = active;
        pressed
    }
}

// SDL mappings may expose triggers as analog buttons rather than Z axes.
// Keep their full travel and only fall back when the requested axis is absent.
fn mapped_axis_value(
    axis: gilrs::Axis,
    mapped: Option<f32>,
    button_value: impl FnOnce(gilrs::Button) -> f32,
) -> f32 {
    mapped.unwrap_or_else(|| match axis {
        gilrs::Axis::LeftZ => button_value(gilrs::Button::LeftTrigger2),
        gilrs::Axis::RightZ => button_value(gilrs::Button::RightTrigger2),
        _ => 0.0,
    })
}

fn update_pad_buttons(buttons: &mut HashSet<gilrs::Button>, event: gilrs::EventType) {
    match event {
        gilrs::EventType::ButtonPressed(button, _) if button != gilrs::Button::Unknown => {
            buttons.insert(button);
        }
        gilrs::EventType::ButtonReleased(button, _) => {
            buttons.remove(&button);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IN0_COIN1: u8 = 0x01;
    const IN0_COIN2: u8 = 0x02;
    const IN0_TEST: u8 = 0x04;
    const IN0_SERVICE: u8 = 0x08;
    const IN0_START1: u8 = 0x10;
    const IN1_JOY_BTN1: u8 = 0x01;

    #[test]
    fn analog_trigger_buttons_supply_missing_z_axes() {
        use gilrs::{Axis as A, Button as B};
        for value in [0.0, 0.03, 0.25, 0.5, 1.0, 0.0] {
            for (axis, expected) in [(A::LeftZ, B::LeftTrigger2), (A::RightZ, B::RightTrigger2)] {
                assert_eq!(
                    mapped_axis_value(axis, None, |button| {
                        assert_eq!(button, expected);
                        value
                    }),
                    value
                );
                assert_eq!(
                    mapped_axis_value(axis, Some(value), |_| panic!(
                        "mapped axis must take priority"
                    )),
                    value
                );
            }
        }
        for axis in [A::LeftStickX, A::LeftStickY, A::RightStickX, A::RightStickY] {
            assert_eq!(
                mapped_axis_value(axis, None, |_| panic!("sticks must not read triggers")),
                0.0
            );
            assert_eq!(
                mapped_axis_value(axis, Some(-0.5), |_| panic!(
                    "stick mapping must be preserved"
                )),
                -0.5
            );
        }
    }

    #[test]
    fn logical_pad_buttons_do_not_alias_native_codes() {
        use gilrs::{Button as B, EventType as E};
        let mut buttons = HashSet::new();
        // A hat-generated D-pad event can share a native code with R3.
        let code = B::RightThumb.to_nec().unwrap();
        update_pad_buttons(&mut buttons, E::ButtonPressed(B::RightThumb, code));
        assert_eq!(buttons, HashSet::from([B::RightThumb]));
        update_pad_buttons(&mut buttons, E::ButtonPressed(B::DPadRight, code));
        update_pad_buttons(&mut buttons, E::ButtonReleased(B::RightThumb, code));
        assert_eq!(buttons, HashSet::from([B::DPadRight]));
        update_pad_buttons(&mut buttons, E::ButtonReleased(B::DPadRight, code));
        assert!(buttons.is_empty());
    }

    /// Every scheme, so a new one cannot be added without being covered.
    const SCHEMES: [ControlScheme; 8] = [
        ControlScheme::Racing,
        ControlScheme::Bike,
        ControlScheme::Joystick,
        ControlScheme::Gun,
        ControlScheme::Flight,
        ControlScheme::Jetski,
        ControlScheme::Skate,
        ControlScheme::Ski,
    ];

    /// The cabinet furniture every machine has, and the IN0 bit each pulls low.
    /// Wave Runner puts start on a different bit, which the check allows for.
    const FURNITURE: [(Player, Signal, u8); 4] = [
        (Player::One, Signal::Coin, IN0_COIN1),
        (Player::Two, Signal::Coin, IN0_COIN2),
        (Player::One, Signal::Test, IN0_TEST),
        (Player::One, Signal::Service, IN0_SERVICE),
    ];

    fn state(scheme: ControlScheme) -> InputState {
        let mut input = InputState::with_gilrs(None); // No physical device or FF worker in tests.
        input.set_scheme(scheme);
        input
    }

    #[test]
    fn exit_chord_remains_held_when_input_moves_back_to_menu() {
        let mut input = state(ControlScheme::Racing);
        input.set_pad_button(gilrs::Button::Select, true);
        input.set_pad_button(gilrs::Button::Start, true);
        assert!(input.return_to_menu_requested());
        // App transfers the session's input to the library instead of resetting it.
        let mut menu_input = Some(input);
        let input = menu_input.as_mut().unwrap();
        assert!(!input.return_to_menu_requested());
        input.set_pad_button(gilrs::Button::Start, false);
        assert!(!input.return_to_menu_requested());
        input.set_pad_button(gilrs::Button::Start, true);
        assert!(input.return_to_menu_requested());
    }

    #[test]
    fn return_to_menu_is_configurable_and_edge_triggered() {
        use gilrs::Button;
        let mut input = state(ControlScheme::Racing);
        assert!(!input.return_to_menu_requested());
        input.set_pad_button(Button::Select, true);
        assert!(!input.return_to_menu_requested());
        input.set_pad_button(Button::Start, true);
        // The app checks this before polling cabinet ports, even when paused.
        assert!(input.return_to_menu_requested());
        assert!(!input.return_to_menu_requested());
        input.set_pad_button(Button::Start, false);
        input.set_pad_button(Button::Select, false);
        assert!(!input.return_to_menu_requested());
        input.on_key(KeyCode::Escape, true);
        assert!(input.return_to_menu_requested());
        assert!(!input.return_to_menu_requested());
        input.on_key(KeyCode::Escape, false);
        assert!(!input.return_to_menu_requested());

        let mut bindings = Bindings::default();
        bindings.return_to_menu = signals::expression::Binding::parse("KeyQ", false).unwrap();
        input.set_bindings(bindings);
        input.on_key(KeyCode::Escape, true);
        assert!(!input.return_to_menu_requested());
        input.on_key(KeyCode::KeyQ, true);
        assert!(input.return_to_menu_requested());
    }

    /// Coin, test and service must reach the machine through the *binding*,
    /// not through whatever key the binding happens to hold.
    ///
    /// Rebinding to a key nothing is hardcoded to is the point: a scheme that
    /// reads `KeyCode::Digit5` directly passes any test that presses Digit5,
    /// and fails this one -- which is exactly how the pad came to be ignored
    /// on some schemes while the keyboard worked.
    #[test]
    fn every_scheme_reads_the_binding_not_the_key() {
        for scheme in SCHEMES {
            for (player, control, bit) in FURNITURE {
                let mut input = state(scheme);
                let mut bindings = Bindings::default();
                bindings
                    .set_player_expression(player, control, "F12")
                    .unwrap();
                input.set_bindings(bindings);

                let mut out = Inputs::default();
                input.on_key(KeyCode::F12, true);
                input.poll(&mut out);
                assert_eq!(
                    out.in0 & bit,
                    0,
                    "{scheme:?} ignores {control:?} when it is bound elsewhere",
                );

                input.on_key(KeyCode::F12, false);
                input.poll(&mut out);
                assert_ne!(out.in0 & bit, 0, "{scheme:?} leaves {control:?} stuck on");
            }
        }
    }

    /// The on-screen controls have to reach every scheme, for the same reason
    /// the bindings do: a scheme that reads a key or a pad directly ignores the
    /// screen, and on a handset the screen is all there is.
    #[test]
    fn every_scheme_reads_the_on_screen_controls() {
        for scheme in SCHEMES {
            for (player, control, bit) in FURNITURE {
                let mut input = state(scheme);
                let mut out = Inputs::default();

                if player == Player::Two {
                    continue;
                } // touch is P1-only
                input.set_touch(&[(control, 1.0)]);
                input.poll(&mut out);
                assert_eq!(
                    out.in0 & bit,
                    0,
                    "{scheme:?} ignores {control:?} pressed on the screen",
                );

                input.set_touch(&[]);
                input.poll(&mut out);
                assert_ne!(out.in0 & bit, 0, "{scheme:?} leaves {control:?} stuck on");
            }
        }
    }

    /// A thumb has travel, so it drives the wheel to a position. Reading it as
    /// a key would slam the wheel to full lock the moment it moved at all.
    #[test]
    fn a_partly_turned_wheel_lands_between_the_stops() {
        let mut input = state(ControlScheme::Racing);
        let mut out = Inputs::default();
        input.set_touch(&[(Signal::Steering, 0.5)]);
        input.poll(&mut out);
        assert!(
            out.steer > STEER_CENTRE as u8 && out.steer < ANALOG_MAX as u8,
            "half a turn read as {:#04x}",
            out.steer,
        );
    }

    /// A key has no travel, so it must keep ramping instead. This is the case
    /// the analog path is not allowed to swallow.
    #[test]
    fn a_key_still_ramps_the_wheel() {
        let mut input = state(ControlScheme::Racing);
        let mut bindings = Bindings::default();
        bindings
            .set_expression(Signal::Steering, "keys:F12/KeyD")
            .unwrap();
        input.set_bindings(bindings);

        let mut out = Inputs::default();
        input.on_key(KeyCode::F12, true);
        input.poll(&mut out);
        assert_eq!(
            out.steer,
            (STEER_CENTRE - STEER_KEYDELTA) as u8,
            "one frame of a held key should move the wheel one step",
        );
    }

    /// A pad the platform reports itself -- which is how Android delivers one
    /// -- resolves through the same bindings a gilrs pad does.
    #[test]
    fn a_platform_pad_reaches_the_machine() {
        let mut input = state(ControlScheme::Joystick);
        let mut out = Inputs::default();

        // Button 1 uses the shared Action 1 South face button.
        input.set_pad_button(gilrs::Button::South, true);
        input.poll(&mut out);
        assert_eq!(
            out.in1 & IN1_JOY_BTN1,
            0,
            "a platform pad button is ignored"
        );

        input.set_pad_button(gilrs::Button::South, false);
        input.poll(&mut out);
        assert_ne!(out.in1 & IN1_JOY_BTN1, 0, "it stayed pressed");
    }

    /// Start has to work the same way. Its bit differs on Wave Runner, whose
    /// cabinet wires it to 0x40 rather than 0x10.
    #[test]
    fn every_scheme_reads_the_start_binding() {
        for scheme in SCHEMES {
            let mut input = state(scheme);
            let mut bindings = Bindings::default();
            bindings.set_expression(Signal::Start, "F12").unwrap();
            input.set_bindings(bindings);

            let mut out = Inputs::default();
            input.on_key(KeyCode::F12, true);
            input.poll(&mut out);
            let start = if matches!(scheme, ControlScheme::Jetski | ControlScheme::Bike) {
                0x40
            } else {
                IN0_START1
            };
            assert_eq!(out.in0 & start, 0, "{scheme:?} ignores a rebound start");
        }
    }

    #[test]
    fn dpad_views_do_not_drive_the_car() {
        let mut input = state(ControlScheme::Racing);
        input.set_game("daytona");
        input.set_pad_button(gilrs::Button::DPadDown, true);
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!(out.in0 & 0x20, 0);
        assert_eq!((out.steer, out.accel, out.brake), (0x80, 0x20, 0x20));
    }

    #[test]
    fn cars_and_bikes_share_the_rebound_steering_signal() {
        for (scheme, game) in [
            (ControlScheme::Racing, "daytona"),
            (ControlScheme::Bike, "manxtt"),
            (ControlScheme::Bike, "motoraid"),
        ] {
            let mut input = state(scheme);
            input.set_game(game);
            input
                .bindings
                .set_expression(Signal::Steering, "keys:F12/KeyD")
                .unwrap();
            input.on_key(KeyCode::F12, true);
            let mut out = Inputs::default();
            input.poll(&mut out);
            assert_eq!(out.steer, (STEER_CENTRE - STEER_KEYDELTA) as u8, "{game}");
        }
    }

    #[test]
    fn face_and_shoulder_aliases_are_or_not_chords() {
        use gilrs::Button as B;
        let mut input = state(ControlScheme::Joystick);
        for (signal, face, shoulder, other) in [
            (Signal::Action1, B::South, B::LeftTrigger, Signal::Action2),
            (Signal::Action2, B::East, B::RightTrigger, Signal::Action1),
        ] {
            for buttons in [vec![face], vec![shoulder], vec![face, shoulder]] {
                input.external.buttons.clear();
                for button in buttons {
                    input.set_pad_button(button, true);
                }
                assert_eq!(input.signal(signal), 1.0);
                assert_eq!(input.signal(other), 0.0);
                assert_eq!(input.signal(Signal::Accelerator), 0.0);
                assert_eq!(input.signal(Signal::Brake), 0.0);
            }
        }
        input.external.buttons.clear();
        input.set_pad_button(B::RightTrigger2, true);
        assert_eq!(input.signal(Signal::Action2), 0.0);
    }

    #[test]
    fn direct_gears_require_chords_latch_and_ignore_conflicts() {
        let mut input = state(ControlScheme::Racing);
        input.set_game("daytona");
        input
            .bindings
            .set_expression(Signal::Gear1, "KeyA & KeyB")
            .unwrap();
        input
            .bindings
            .set_expression(Signal::Gear2, "KeyC")
            .unwrap();
        let mut out = Inputs::default();
        input.on_key(KeyCode::KeyA, true);
        input.poll(&mut out);
        assert_eq!(input.gear, 0);
        input.on_key(KeyCode::KeyB, true);
        input.poll(&mut out);
        assert_eq!(input.gear, 1);
        input.on_key(KeyCode::KeyC, true);
        input.poll(&mut out);
        assert_eq!(input.gear, 1);
        input.keys.clear();
        input.poll(&mut out);
        assert_eq!(input.gear, 1);
        input.on_key(KeyCode::Digit0, true);
        input.poll(&mut out);
        assert_eq!(input.gear, 0);
    }

    #[test]
    fn sequential_shifts_keep_their_physical_positions() {
        use gilrs::Button as B;
        for game in ["indy500", "stcc", "overrev", "sgt24h", "manxtt"] {
            let mut input = state(if game == "manxtt" {
                ControlScheme::Bike
            } else {
                ControlScheme::Racing
            });
            input.set_game(game);
            let mut out = Inputs::default();
            for (button, expected) in [
                (B::East, 0xff),
                (B::RightTrigger, 0xef),
                (B::South, 0xff),
                (B::LeftTrigger, 0xdf),
            ] {
                input.set_pad_button(button, true);
                input.poll(&mut out);
                assert_eq!(out.in1, expected, "{game} {button:?}");
                input.set_pad_button(button, false);
                input.poll(&mut out);
                assert_eq!(out.in1, 0xff, "{game} release");
            }
            input.set_pad_button(B::LeftTrigger, true);
            input.set_pad_button(B::RightTrigger, true);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xff, "{game} conflict");
        }
    }

    #[test]
    fn vr_service_is_isolated_from_momentary_shift_inputs() {
        use gilrs::Button as B;
        for game in ["vr", "vformula"] {
            let mut input = state(ControlScheme::Racing);
            input.set_game(game);
            let mut out = Inputs::default();
            input.poll(&mut out);
            assert_eq!((out.in0, out.in1), (0xff, 0xff), "{game}: idle");
            input.set_pad_button(B::RightThumb, true);
            input.poll(&mut out);
            assert_eq!((out.in0, out.in1), (0xf7, 0xff), "{game}: service");
            input.set_pad_button(B::RightThumb, false);
            input.set_pad_button(B::RightTrigger, true);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xdf, "{game}: shift up");
            input.set_pad_button(B::RightTrigger, false);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xff, "{game}: release must not latch");
            input.set_pad_button(B::LeftTrigger, true);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xef, "{game}: shift down");
            input.set_pad_button(B::RightTrigger, true);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xff, "{game}: conflicting shifts");
            input.external.buttons.clear();
            input.on_key(KeyCode::Digit1, true);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xff, "{game}: no H-gate encoding");
            input.on_key(KeyCode::F8, true);
            input.poll(&mut out);
            assert_eq!((out.in0, out.in1), (0xf7, 0xff), "{game}: keyboard service");
            input.set_pad_button(B::DPadUp, true);
            input.poll(&mut out);
            assert_eq!(out.in1, 0xfe, "{game}: VR4 preserved");
        }
    }

    #[test]
    fn flight_axis_keeps_partial_travel_and_rebinding() {
        let mut input = state(ControlScheme::Flight);
        input.set_game("skytargt");
        input.set_analog_roles([AnalogRole::StickX; 8]);
        input.set_pad_stick(0.5, 0.0);
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!(out.analog[0], 64); // Sky Target: full-range reversed X
        input.bindings.set_expression(Signal::SkyX, "").unwrap();
        input.poll(&mut out);
        assert_eq!(out.analog[0], 128);
    }

    #[test]
    fn game_context_routes_start_actions_and_independent_axes() {
        let mut input = state(ControlScheme::Joystick);
        input.set_game("vf");
        input.set_pad_button(gilrs::Button::South, true);
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!(out.in1 & 7, 5); // VF: South is Kick, mask 0x02.
        input.set_game("vf2");
        input.poll(&mut out);
        assert_eq!(out.in1 & 7, 5); // VF2: same requested SM2 convention.
        input.set_scheme(ControlScheme::Ski);
        input.set_game("segawski");
        input.set_pad_button(gilrs::Button::Start, true);
        input.poll(&mut out);
        assert_eq!(out.in0 & 0x40, 0);
        assert_ne!(out.in0 & 0x10, 0);
        input.set_scheme(ControlScheme::Skate);
        input.set_game("topskatr");
        input.set_analog_roles([
            AnalogRole::Curving,
            AnalogRole::Slide,
            AnalogRole::None,
            AnalogRole::None,
            AnalogRole::None,
            AnalogRole::None,
            AnalogRole::None,
            AnalogRole::None,
        ]);
        input.set_pad_stick(0.5, 0.0);
        input.poll(&mut out);
        assert_eq!(out.analog[0], 64); // Top Skater: reversed Curving
        assert_eq!(out.analog[1], 128);
    }

    #[test]
    fn virtual_on_has_two_independent_sticks() {
        let mut input = state(ControlScheme::Joystick);
        input.set_game("von");
        input.on_key(KeyCode::KeyW, true);
        input.on_key(KeyCode::ArrowDown, true);
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!(out.in1 & 0x30, 0x10);
        assert_eq!(out.in2 & 0x30, 0x20);
    }

    #[test]
    fn slide_bindings_are_independent_per_game() {
        let mut input = state(ControlScheme::Ski);
        input.set_game("segawski");
        input.set_analog_roles([AnalogRole::Slide; 8]);
        input
            .bindings
            .set_expression(Signal::WaterSlide, "keys:KeyA/KeyD")
            .unwrap();
        input
            .bindings
            .set_expression(Signal::SkaterSlide, "keys:KeyU/KeyO")
            .unwrap();
        let mut out = Inputs::default();
        input.on_key(KeyCode::KeyD, true);
        input.poll(&mut out);
        assert_eq!(out.analog[0], 0); // Water Ski: reversed Slide
        input.set_game("topskatr");
        input.set_scheme(ControlScheme::Skate);
        input.poll(&mut out);
        assert_eq!(out.analog[0], 128);
        input.on_key(KeyCode::KeyO, true);
        input.poll(&mut out);
        assert_eq!(out.analog[0], 255);
    }
}
