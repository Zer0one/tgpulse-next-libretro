//! User-facing signal bindings and emulator hotkeys.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::input::players::Player;
use crate::input::signals::{self, expression::Binding, Signal};
use gilrs::{Axis, Button};
use winit::keyboard::KeyCode;

// Upgrade only exact shipped defaults; keep custom/empty expressions intact.
// Loading never rewrites the user's file. Newly split signals inherit defaults.
fn refreshed_stock_binding(signal: Signal, value: &str) -> &str {
    match (signal, value) {
        (Signal::Test, "F2, pad:RightThumb")
        | (Signal::Service, "F8, pad:LeftThumb")
        | (Signal::Elevation, "keys:KeyG/KeyT, pad:LeftStickY") => signal.default_text(),
        _ => value,
    }
}

/// Something the emulator itself does, rather than the machine.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Hotkey {
    ToggleMenu,
    Fullscreen,
    SaveState,
    LoadState,
    NextSlot,
    PreviousSlot,
    Reset,
    Pause,
    FastForward,
}

impl Hotkey {
    pub const ALL: &'static [Hotkey] = &[
        Hotkey::ToggleMenu,
        Hotkey::Fullscreen,
        Hotkey::SaveState,
        Hotkey::LoadState,
        Hotkey::NextSlot,
        Hotkey::PreviousSlot,
        Hotkey::Reset,
        Hotkey::Pause,
        Hotkey::FastForward,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Hotkey::ToggleMenu => "Show/hide menu",
            Hotkey::Fullscreen => "Fullscreen",
            Hotkey::SaveState => "Save state",
            Hotkey::LoadState => "Load state",
            Hotkey::NextSlot => "Next state slot",
            Hotkey::PreviousSlot => "Previous state slot",
            Hotkey::Reset => "Reset machine",
            Hotkey::Pause => "Pause",
            Hotkey::FastForward => "Fast forward (hold)",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Hotkey::ToggleMenu => "toggle_menu",
            Hotkey::Fullscreen => "fullscreen",
            Hotkey::SaveState => "save_state",
            Hotkey::LoadState => "load_state",
            Hotkey::NextSlot => "next_slot",
            Hotkey::PreviousSlot => "previous_slot",
            Hotkey::Reset => "reset",
            Hotkey::Pause => "pause",
            Hotkey::FastForward => "fast_forward",
        }
    }

    fn from_key(s: &str) -> Option<Hotkey> {
        Hotkey::ALL.iter().copied().find(|h| h.key() == s)
    }
}

/// One physical thing that can drive a control.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Key(KeyCode),
    Pad(Button),
    /// A stick or trigger past the deadzone in one direction.
    PadAxis(Axis, Sign),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sign {
    Positive,
    Negative,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Key(k) => write!(f, "{}", key_name(*k)),
            Source::Pad(b) => write!(f, "Pad {}", pad_button_name(*b)),
            Source::PadAxis(a, s) => write!(
                f,
                "Pad {}{}",
                pad_axis_name(*a),
                match s {
                    Sign::Positive => "+",
                    Sign::Negative => "-",
                }
            ),
        }
    }
}

/// Everything the player has bound.
#[derive(Clone, Debug)]
pub struct Bindings {
    pub return_to_menu: Binding,
    pub controls: BTreeMap<Signal, Binding>,
    pub controls_p2: BTreeMap<Signal, Binding>,
    /// "auto", "none", or a persistent device UUID:ordinal from the frontend.
    pub controllers: [String; 2],
    /// Desktop gamepad provider; exactly one is active at a time.
    pub gamepad_backend: GamepadBackend,
    pub hotkeys: BTreeMap<Hotkey, KeyCode>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GamepadBackend {
    #[default]
    Gilrs,
    Sdl3,
}

impl GamepadBackend {
    pub fn name(self) -> &'static str {
        match self {
            Self::Gilrs => "gilrs",
            Self::Sdl3 => "sdl3",
        }
    }
}

impl Default for Bindings {
    /// Match SM2-Emu's RetroPad positions where a cabinet control has the
    /// same meaning. Keyboard bindings retain their original layout.
    fn default() -> Self {
        let controls = signals::defaults();

        let hotkeys = BTreeMap::from([
            (Hotkey::ToggleMenu, KeyCode::F1),
            (Hotkey::Fullscreen, KeyCode::F11),
            (Hotkey::SaveState, KeyCode::F5),
            (Hotkey::LoadState, KeyCode::F7),
            (Hotkey::NextSlot, KeyCode::F6),
            (Hotkey::PreviousSlot, KeyCode::F4),
            (Hotkey::Reset, KeyCode::F3),
            (Hotkey::Pause, KeyCode::F9),
            (Hotkey::FastForward, KeyCode::Tab),
        ]);

        Self {
            controls,
            controls_p2: Signal::ALL
                .iter()
                .map(|&s| {
                    (
                        s,
                        Binding::parse(&s.p2_default_text(), s.signed()).expect("valid P2 default"),
                    )
                })
                .collect(),
            controllers: ["auto".into(), "auto".into()],
            gamepad_backend: GamepadBackend::default(),
            hotkeys,
            return_to_menu: Binding::parse("Escape, pad:Select & pad:Start", false)
                .expect("valid return-to-menu default"),
        }
    }
}

impl Bindings {
    pub fn player_binding(&self, player: Player, signal: Signal) -> &Binding {
        if player == Player::One {
            self.binding(signal)
        } else {
            &self.controls_p2[&signal]
        }
    }
    pub fn set_player_expression(
        &mut self,
        player: Player,
        signal: Signal,
        text: &str,
    ) -> Result<(), String> {
        if player == Player::One {
            return self.set_expression(signal, text);
        }
        if !signal.supports_p2() {
            return Err("No Player 2 counterpart for this signal".into());
        }
        self.controls_p2
            .insert(signal, Binding::parse(text, signal.signed())?);
        Ok(())
    }
    pub fn set_controller(&mut self, player: Player, key: String) {
        let i = player.index();
        if key != "auto" && key != "none" && self.controllers[1 - i] == key {
            self.controllers[1 - i] = "none".into();
        }
        self.controllers[i] = key;
    }
    pub fn binding(&self, signal: Signal) -> &Binding {
        &self.controls[&signal]
    }
    pub fn set_expression(&mut self, signal: Signal, text: &str) -> Result<(), String> {
        self.controls
            .insert(signal, Binding::parse(text, signal.signed())?);
        Ok(())
    }

    pub fn hotkey(&self, hotkey: Hotkey) -> Option<KeyCode> {
        self.hotkeys.get(&hotkey).copied()
    }

    /// The hotkey a key press triggers, if any.
    pub fn hotkey_for(&self, key: KeyCode) -> Option<Hotkey> {
        self.hotkeys
            .iter()
            .find(|(_, bound)| **bound == key)
            .map(|(hotkey, _)| *hotkey)
    }

    pub fn bind_hotkey(&mut self, hotkey: Hotkey, key: KeyCode) {
        // A key drives one hotkey; taking it from another is the intent.
        self.hotkeys.retain(|_, bound| *bound != key);
        self.hotkeys.insert(hotkey, key);
    }

    /// Reads the bindings, writing the defaults out first if there is no file
    /// yet -- so the format is discoverable and hand-editable without having to
    /// change something in the interface to make one appear.
    pub fn load_or_create(path: &Path) -> Self {
        if !path.exists() {
            let defaults = Self::default();
            if let Err(e) = defaults.save(path) {
                log::warn!(target: "input", "cannot write {}: {e}", path.display());
            }
            return defaults;
        }
        let bindings = Self::load(path);
        if let Ok(text) = std::fs::read_to_string(path) {
            if !text.lines().any(|l| l.trim() == "format = signals-v3") {
                let backup = path.with_extension(
                    if text.lines().any(|l| l.trim() == "format = signals-v2") {
                        "conf.pre-model2-players"
                    } else if text.lines().any(|l| l.trim() == "format = signals-v1") {
                        "conf.pre-players"
                    } else {
                        "conf.pre-signals"
                    },
                );
                // Never overwrite an earlier backup.
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&backup)
                {
                    Ok(mut file) => {
                        use std::io::Write;
                        if file.write_all(text.as_bytes()).is_ok() {
                            if let Err(e) = bindings.save(path) {
                                log::warn!("Cannot save migrated bindings: {e}");
                            }
                        }
                    }
                    Err(e) => log::warn!(
                        "Cannot back up {}; migration remains in memory: {e}",
                        path.display()
                    ),
                }
            }
        }
        bindings
    }

    pub fn path() -> PathBuf {
        PathBuf::from("config").join("input.conf")
    }

    /// Reads the file, falling back to the defaults for anything it does not
    /// mention -- so a file written by an older build still works, and so a
    /// control the player has not touched keeps its shipped binding.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut bindings = Self::default();
        let modern = text.lines().any(|l| {
            matches!(
                l.trim(),
                "format = signals-v1" | "format = signals-v2" | "format = signals-v3"
            )
        });
        let model1_players = text.lines().any(|l| l.trim() == "format = signals-v2");
        let mut explicit_p1 = BTreeSet::new();
        let mut explicit_p2 = BTreeSet::new();
        if !modern {
            bindings.migrate_keyboard(&text);
        }
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                log::warn!(target: "input", "{}:{}: not a binding", path.display(), number + 1);
                continue;
            };
            let (name, value) = (name.trim(), value.trim());
            if name == "format" {
                continue;
            }
            if name == "gamepad_backend" {
                bindings.gamepad_backend = match value {
                    "gilrs" => GamepadBackend::Gilrs,
                    "sdl3" if cfg!(target_os = "macos") => GamepadBackend::Sdl3,
                    _ => {
                        log::warn!(target: "input", "{}:{}: unsupported gamepad backend '{value}'", path.display(), number + 1);
                        GamepadBackend::Gilrs
                    }
                };
                continue;
            }
            if name == "controller_p1" || name == "controller_p2" {
                bindings.controllers[usize::from(name == "controller_p2")] = value.to_owned();
            } else if modern && (name.starts_with("p2.") || name == "coin2" || name == "start2") {
                let signal = match name {
                    "coin2" => Some(Signal::Coin),
                    "start2" => Some(Signal::Start),
                    _ => Signal::from_key(&name[3..]),
                };
                match signal {
                    Some(s) => {
                        // v2 persisted disabled rows as empty. These signals
                        // could not be edited there; enable their new defaults
                        // once, without replacing any nonempty custom value.
                        if model1_players
                            && value.is_empty()
                            && matches!(
                                s,
                                Signal::GunYaw
                                    | Signal::GunPitch
                                    | Signal::BatSwing
                                    | Signal::Accelerator
                                    | Signal::Brake
                            )
                        {
                            continue;
                        }
                        let value = match (name, value) {
                            ("coin2", "Digit6") => "Digit6, pad:Select",
                            ("start2", "Digit2") => "Digit2, pad:Start",
                            _ => value,
                        };
                        let value = refreshed_stock_binding(s, value);
                        if let Err(e) = bindings.set_player_expression(Player::Two, s, value) {
                            // Empty disabled rows are persisted for a complete catalogue.
                            if !value.is_empty() {
                                log::warn!("Invalid binding {name}: {e}");
                            }
                        } else {
                            explicit_p2.insert(s);
                        }
                    }
                    None => log::warn!("Unknown P2 signal {name}"),
                }
            } else if name == "return_to_menu" {
                match Binding::parse(value, false) {
                    Ok(binding) => bindings.return_to_menu = binding,
                    Err(e) => log::warn!("Invalid return_to_menu binding: {e}; keeping default"),
                }
            } else if let Some(signal) = Signal::from_key(name).filter(|_| modern) {
                if let Err(e) =
                    bindings.set_expression(signal, refreshed_stock_binding(signal, value))
                {
                    log::warn!("Invalid binding {name}: {e}; keeping default");
                } else {
                    explicit_p1.insert(signal);
                }
            } else if let Some(hotkey) = Hotkey::from_key(name) {
                if let Some(key) = parse_key(value) {
                    bindings.hotkeys.insert(hotkey, key);
                } else if value.is_empty() {
                    bindings.hotkeys.remove(&hotkey);
                }
            } else if modern {
                log::warn!(target: "input", "{}:{}: unknown control '{name}'", path.display(), number + 1);
            }
        }
        // A pre-split file drove these cabinets through Action 1/2/3. Carry
        // any customized expressions forward once; explicit per-game rows win.
        // Saving writes independent keys, so later edits cannot recouple them.
        for (dedicated, previous) in [
            (Signal::DesertShift, Signal::Action3),
            (Signal::NetmercButton1, Signal::Action1),
            (Signal::NetmercButton2, Signal::Action2),
            (Signal::NetmercMvdHolder, Signal::Action3),
            (Signal::SledEntry, Signal::Action1),
            (Signal::SledCall, Signal::Action2),
            (Signal::SkiSelect1, Signal::Action3),
            (Signal::SwaLaser, Signal::Action1),
            (Signal::SwaTorpedo, Signal::Action2),
            (Signal::SkaterJumpTail, Signal::Action1),
            (Signal::SkaterJumpFront, Signal::Action2),
            (Signal::StrikerShortPass, Signal::Action1),
            (Signal::StrikerLongPass, Signal::Action2),
            (Signal::StrikerShoot, Signal::Action3),
            (Signal::WingMachineGun, Signal::Action1),
            (Signal::WingMissile, Signal::Action2),
            (Signal::WingSmoke, Signal::Action3),
        ] {
            if !explicit_p1.contains(&dedicated) && explicit_p1.contains(&previous) {
                let value = bindings.binding(previous).text.clone();
                bindings.set_expression(dedicated, &value).unwrap();
            }
            if dedicated.supports_p2()
                && !explicit_p2.contains(&dedicated)
                && explicit_p2.contains(&previous)
            {
                let value = bindings.player_binding(Player::Two, previous).text.clone();
                bindings
                    .set_player_expression(Player::Two, dedicated, &value)
                    .unwrap();
            }
        }
        log::info!(target: "input", "bindings from {}", path.display());
        bindings
    }

    fn migrate_keyboard(&mut self, text: &str) {
        // Import keyboard assignments once, keeping the new controller defaults.
        let old: BTreeMap<&str, Vec<&str>> = text
            .lines()
            .filter_map(|l| l.split('#').next()?.split_once('='))
            .map(|(k, v)| {
                (
                    k.trim(),
                    v.split(',')
                        .map(str::trim)
                        .filter(|s| parse_key(s).is_some())
                        .collect(),
                )
            })
            .collect();
        let mut merged: BTreeMap<Signal, Vec<&str>> = BTreeMap::new();
        for (old_name, signal) in [
            ("coin1", Signal::Coin),
            ("start1", Signal::Start),
            ("test", Signal::Test),
            ("service", Signal::Service),
            ("up", Signal::Up),
            ("down", Signal::Down),
            ("left", Signal::Left),
            ("right", Signal::Right),
            ("button1", Signal::Action1),
            ("button2", Signal::Action2),
            ("button3", Signal::Action3),
            ("button4", Signal::Action4),
            ("view_red", Signal::View1),
            ("view_blue", Signal::View2),
            ("view_yellow", Signal::View3),
            ("view_green", Signal::View4),
            ("gear_up", Signal::GearUp),
            ("gear_down", Signal::GearDown),
            ("fire", Signal::Action1),
            ("reload", Signal::Action2),
        ] {
            if let Some(keys) = old.get(old_name) {
                merged
                    .entry(signal)
                    .or_default()
                    .extend(keys.iter().copied());
            }
        }
        for (name, signal) in [("coin2", Signal::Coin), ("start2", Signal::Start)] {
            if let Some(keys) = old.get(name) {
                self.set_player_expression(Player::Two, signal, &keys.join(", "))
                    .unwrap();
            }
        }
        for (signal, mut keys) in merged {
            keys.sort_unstable();
            keys.dedup();
            let pad: Vec<_> = signal
                .default_text()
                .split(',')
                .map(str::trim)
                .filter(|s| s.starts_with("pad:"))
                .collect();
            self.set_expression(
                signal,
                &keys.into_iter().chain(pad).collect::<Vec<_>>().join(", "),
            )
            .expect("valid migrated keys");
        }
        for (negative, positive, signals) in [(
            "left",
            "right",
            &[
                Signal::Steering,
                Signal::Handle,
                Signal::SkyX,
                Signal::Curving,
                Signal::Swing,
            ][..],
        )] {
            if let (Some(neg), Some(pos)) = (old.get(negative), old.get(positive)) {
                if !neg.is_empty() && !pos.is_empty() {
                    let pairs: Vec<_> = (0..neg.len().max(pos.len()))
                        .map(|i| format!("keys:{}/{}", neg[i % neg.len()], pos[i % pos.len()]))
                        .collect();
                    for &signal in signals {
                        let pad: Vec<_> = signal
                            .default_text()
                            .split(',')
                            .map(str::trim)
                            .filter(|s| s.starts_with("pad:"))
                            .collect();
                        self.set_expression(
                            signal,
                            &pairs
                                .iter()
                                .map(String::as_str)
                                .chain(pad)
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                        .unwrap();
                    }
                }
            }
        }
        for (direction, pedal, signal) in [
            ("up", "throttle", Signal::Accelerator),
            ("down", "brake", Signal::Brake),
        ] {
            if old.contains_key(direction) || old.contains_key(pedal) {
                let mut keys = old.get(direction).cloned().unwrap_or_default();
                keys.extend(old.get(pedal).cloned().unwrap_or_default());
                keys.sort_unstable();
                keys.dedup();
                let pad: Vec<_> = signal
                    .default_text()
                    .split(',')
                    .map(str::trim)
                    .filter(|s| s.starts_with("pad:"))
                    .collect();
                self.set_expression(
                    signal,
                    &keys.into_iter().chain(pad).collect::<Vec<_>>().join(", "),
                )
                .unwrap();
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut out = String::from("# TGPulse: one signal list, independent player bindings.\n# Comma = alternatives; & = simultaneous chord.\n# pad:Axis = signed axis, pad:Axis~ = inverted, +/- = half axis.\n# keys:Negative/Positive = keyboard axis. Empty = unbound.\nformat = signals-v3\n\n");
        out += &format!(
            "gamepad_backend = {}\ncontroller_p1 = {}\ncontroller_p2 = {}\n\n# Cabinet P1\n",
            self.gamepad_backend.name(),
            self.controllers[0],
            self.controllers[1]
        );
        for signal in Signal::ALL {
            out += &format!(
                "# {}\n{} = {}\n",
                signal.label(),
                signal.key(),
                self.binding(*signal).text
            );
        }
        out += "\n# Cabinet P2 (unsupported signals are unbound)\n";
        for &signal in Signal::ALL {
            out += &format!(
                "# {}\np2.{} = {}\n",
                signal.label(),
                signal.key(),
                self.player_binding(Player::Two, signal).text
            );
        }
        out += "\n# Emulator hotkeys\n";
        out += &format!("return_to_menu = {}\n", self.return_to_menu.text);
        for hotkey in Hotkey::ALL {
            let key = self.hotkey(*hotkey).map(key_token).unwrap_or_default();
            out += &format!("{} = {}\n", hotkey.key(), key);
        }
        std::fs::write(path, out).map_err(|e| e.to_string())
    }
}

pub(crate) fn parse_source(token: &str) -> Option<Source> {
    if let Some(rest) = token.strip_prefix("pad:") {
        if let Some(name) = rest.strip_suffix('+') {
            return pad_axis_from_token(name).map(|a| Source::PadAxis(a, Sign::Positive));
        }
        if let Some(name) = rest.strip_suffix('-') {
            return pad_axis_from_token(name).map(|a| Source::PadAxis(a, Sign::Negative));
        }
        return pad_button_from_token(rest).map(Source::Pad);
    }
    parse_key(token).map(Source::Key)
}

/// Keys are written as winit names it it, which are stable and unambiguous.
fn key_token(key: KeyCode) -> String {
    format!("{key:?}")
}

pub(crate) fn parse_key(token: &str) -> Option<KeyCode> {
    KEYS.iter()
        .copied()
        .find(|k| format!("{k:?}").eq_ignore_ascii_case(token))
}

/// A friendlier spelling for the interface; the file keeps the exact name.
fn key_name(key: KeyCode) -> String {
    let raw = format!("{key:?}");
    for prefix in ["Key", "Digit", "Numpad", "Arrow"] {
        if let Some(rest) = raw.strip_prefix(prefix) {
            return match prefix {
                "Numpad" => format!("Num {rest}"),
                "Arrow" => rest.to_string(),
                _ => rest.to_string(),
            };
        }
    }
    raw
}

fn pad_button_from_token(token: &str) -> Option<Button> {
    PAD_BUTTONS
        .iter()
        .copied()
        .find(|b| format!("{b:?}").eq_ignore_ascii_case(token))
}

/// Pad buttons named as they are printed on the two common layouts, since
/// "South" means nothing to anyone holding the controller.
fn pad_button_name(button: Button) -> &'static str {
    match button {
        Button::South => "A / Cross",
        Button::East => "B / Circle",
        Button::West => "X / Square",
        Button::North => "Y / Triangle",
        Button::LeftTrigger => "L1 / LB",
        Button::RightTrigger => "R1 / RB",
        Button::LeftTrigger2 => "L2 / LT",
        Button::RightTrigger2 => "R2 / RT",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::LeftThumb => "L3",
        Button::RightThumb => "R3",
        Button::DPadUp => "D-pad up",
        Button::DPadDown => "D-pad down",
        Button::DPadLeft => "D-pad left",
        Button::DPadRight => "D-pad right",
        _ => "button",
    }
}

fn pad_axis_from_token(token: &str) -> Option<Axis> {
    PAD_AXES
        .iter()
        .copied()
        .find(|a| format!("{a:?}").eq_ignore_ascii_case(token))
}

fn pad_axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::LeftStickX => "left stick X",
        Axis::LeftStickY => "left stick Y",
        Axis::RightStickX => "right stick X",
        Axis::RightStickY => "right stick Y",
        Axis::LeftZ => "left trigger",
        Axis::RightZ => "right trigger",
        _ => "axis",
    }
}

/// The keys offered for binding. Anything a cabinet button might reasonably
/// live on; the exotic ones are left out so the picker stays readable.
pub const KEYS: &[KeyCode] = &[
    KeyCode::Escape,
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::Space,
    KeyCode::Enter,
    KeyCode::NumpadEnter,
    KeyCode::Tab,
    KeyCode::Backspace,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::AltLeft,
    KeyCode::AltRight,
    KeyCode::Comma,
    KeyCode::Period,
    KeyCode::Slash,
    KeyCode::Semicolon,
    KeyCode::Quote,
    KeyCode::BracketLeft,
    KeyCode::BracketRight,
    KeyCode::Minus,
    KeyCode::Equal,
    KeyCode::Backquote,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
];

pub const PAD_BUTTONS: &[Button] = &[
    Button::South,
    Button::East,
    Button::West,
    Button::North,
    Button::LeftTrigger,
    Button::RightTrigger,
    Button::LeftTrigger2,
    Button::RightTrigger2,
    Button::Select,
    Button::Start,
    Button::LeftThumb,
    Button::RightThumb,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
];

pub const PAD_AXES: &[Axis] = &[
    Axis::LeftStickX,
    Axis::LeftStickY,
    Axis::RightStickX,
    Axis::RightStickY,
    Axis::LeftZ,
    Axis::RightZ,
];

impl FromStr for Source {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        parse_source(s).ok_or_else(|| format!("unknown input source '{s}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_signals_round_trip() {
        let path =
            std::env::temp_dir().join(format!("tgpulse-signals-{}.conf", std::process::id()));
        let mut written = Bindings::default();
        written
            .set_expression(
                Signal::Gear1,
                "KeyJ & KeyK, pad:RightStickX- & pad:RightStickY+",
            )
            .unwrap();
        written
            .set_player_expression(Player::Two, Signal::Coin, "")
            .unwrap();
        written
            .set_player_expression(Player::Two, Signal::SkyX, "pad:RightStickX~")
            .unwrap();
        written.set_controller(Player::Two, "012345:1".into());
        written.bind_hotkey(Hotkey::Reset, KeyCode::F5);
        written.return_to_menu = Binding::parse("KeyQ, pad:Select & pad:North", false).unwrap();
        written.save(&path).unwrap();
        let read = Bindings::load(&path);
        for signal in Signal::ALL {
            assert_eq!(read.binding(*signal), written.binding(*signal));
        }
        assert_eq!(read.hotkeys, written.hotkeys);
        assert_eq!(read.controls_p2, written.controls_p2);
        assert_eq!(read.controllers, written.controllers);
        assert_eq!(read.gamepad_backend, written.gamepad_backend);
        assert_eq!(read.return_to_menu, written.return_to_menu);
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sdl3_selection_round_trips_without_changing_cabinet_bindings() {
        let path =
            std::env::temp_dir().join(format!("tgpulse-sdl3-bindings-{}.conf", std::process::id()));
        let mut bindings = Bindings::default();
        let original = bindings.controls.clone();
        bindings.gamepad_backend = GamepadBackend::Sdl3;
        bindings.save(&path).unwrap();
        let loaded = Bindings::load(&path);
        assert_eq!(loaded.gamepad_backend, GamepadBackend::Sdl3);
        assert_eq!(loaded.controls, original);
        assert_eq!(loaded.controls_p2, bindings.controls_p2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn preferred_defaults_refresh_without_overwriting_custom_bindings() {
        for (signal, old) in [
            (Signal::Test, "F2, pad:RightThumb"),
            (Signal::Service, "F8, pad:LeftThumb"),
            (Signal::Elevation, "keys:KeyG/KeyT, pad:LeftStickY"),
        ] {
            assert_eq!(refreshed_stock_binding(signal, old), signal.default_text());
            assert_eq!(
                refreshed_stock_binding(signal, "F12, pad:South"),
                "F12, pad:South"
            );
            assert_eq!(refreshed_stock_binding(signal, ""), "");
        }
    }

    #[test]
    fn former_stock_test_service_bindings_refresh_for_both_players() {
        let path =
            std::env::temp_dir().join(format!("tgpulse-test-service-{}.conf", std::process::id()));
        std::fs::write(
            &path,
            "format = signals-v3\ntest = F2, pad:RightThumb\nservice = F8, pad:LeftThumb\np2.test = F2, pad:RightThumb\np2.service = F8, pad:LeftThumb\n",
        )
        .unwrap();
        let loaded = Bindings::load(&path);
        for player in Player::ALL {
            assert_eq!(
                loaded.player_binding(player, Signal::Test).text,
                "F2, pad:LeftThumb"
            );
            assert_eq!(
                loaded.player_binding(player, Signal::Service).text,
                "F8, pad:RightThumb"
            );
        }
        // Loading is read-only; the current preference is written only on save.
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("test = F2, pad:RightThumb"));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn invalid_binding_is_not_applied() {
        let mut b = Bindings::default();
        assert!(b.set_expression(Signal::Gear1, "KeyJ & nonsense").is_err());
        assert_eq!(b.binding(Signal::Gear1).text, Signal::Gear1.default_text());
    }

    #[test]
    fn split_actions_inherit_only_missing_rows_and_then_save_independently() {
        let path =
            std::env::temp_dir().join(format!("tgpulse-split-actions-{}.conf", std::process::id()));
        let original = "format = signals-v3\naction1 = F11\naction2 = \naction3 = F12\nswa_laser = KeyU\np2.action1 = pad:North\np2.swa_laser = pad:West\n";
        std::fs::write(&path, original).unwrap();
        let bindings = Bindings::load(&path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(bindings.binding(Signal::SwaLaser).text, "KeyU");
        assert_eq!(bindings.binding(Signal::WingMachineGun).text, "F11");
        assert_eq!(bindings.binding(Signal::WingMissile).text, "");
        assert_eq!(bindings.binding(Signal::DesertShift).text, "F12");
        assert_eq!(
            bindings.player_binding(Player::Two, Signal::SwaLaser).text,
            "pad:West"
        );
        assert_eq!(
            bindings.player_binding(Player::Two, Signal::SledEntry).text,
            "pad:North"
        );
        bindings.save(&path).unwrap();
        let loaded = Bindings::load(&path);
        assert_eq!(loaded.binding(Signal::WingMachineGun).text, "F11");
        assert_eq!(loaded.binding(Signal::SwaLaser).text, "KeyU");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn dedicated_gears_default_load_save_and_legacy_keys_are_independent() {
        let path = std::env::temp_dir().join(format!(
            "tgpulse-dedicated-gears-{}.conf",
            std::process::id()
        ));
        let original = "format = signals-v3\naction1 = F11\naction2 = F12\n";
        std::fs::write(&path, original).unwrap();
        let mut bindings = Bindings::load_or_create(&path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(
            bindings.binding(Signal::GearDown).text,
            "KeyE, pad:LeftTrigger"
        );
        assert_eq!(
            bindings.binding(Signal::GearUp).text,
            "KeyQ, pad:RightTrigger"
        );
        assert_eq!(bindings.binding(Signal::Action1).text, "F11");
        assert_eq!(bindings.binding(Signal::Action2).text, "F12");
        assert!(!Signal::GearUp.supports_p2());
        assert!(!Signal::GearDown.supports_p2());
        bindings
            .set_expression(Signal::GearUp, "pad:North")
            .unwrap();
        bindings.set_expression(Signal::GearDown, "").unwrap();
        bindings.save(&path).unwrap();
        let loaded = Bindings::load(&path);
        assert_eq!(loaded.binding(Signal::GearUp).text, "pad:North");
        assert_eq!(loaded.binding(Signal::GearDown).text, "");
        assert_eq!(loaded.binding(Signal::Action1).text, "F11");
        assert_eq!(loaded.binding(Signal::Action2).text, "F12");
        std::fs::remove_file(&path).unwrap();

        let mut legacy = Bindings::default();
        legacy.migrate_keyboard("gear_up = F11\ngear_down = F12\nbutton1 = KeyJ\nbutton2 = KeyK\n");
        assert_eq!(legacy.binding(Signal::GearUp).text, "F11, pad:RightTrigger");
        assert_eq!(
            legacy.binding(Signal::GearDown).text,
            "F12, pad:LeftTrigger"
        );
        assert!(!legacy.binding(Signal::Action1).text.contains("F12"));
        assert!(!legacy.binding(Signal::Action2).text.contains("F11"));
    }
    #[test]
    fn p2_defaults_share_pad_conventions_but_not_gameplay_keys() {
        use crate::input::signals::expression::Atom;
        let b = Bindings::default();
        for &s in Signal::ALL {
            let binding = b.player_binding(Player::Two, s);
            if matches!(
                s,
                Signal::Coin | Signal::Start | Signal::Test | Signal::Service
            ) {
                continue;
            }
            assert_eq!(
                binding.text,
                if s.supports_p2() {
                    s.default_text()
                        .split(',')
                        .map(str::trim)
                        .filter(|p| p.starts_with("pad:"))
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    String::new()
                }
            );
            assert_eq!(
                binding.value(|a| match a {
                    Atom::Source(Source::Key(_)) | Atom::Keys(_, _) => 1.0,
                    _ => 0.0,
                }),
                0.0
            );
        }
        let mut b = b;
        assert!(b
            .set_player_expression(Player::Two, Signal::ThrottleUp, "KeyW")
            .is_err());
        b.set_controller(Player::One, "uuid:1".into());
        b.set_controller(Player::Two, "uuid:1".into());
        assert_eq!(b.controllers, ["none", "uuid:1"]);
    }
    #[test]
    fn old_coin_start_move_to_p2_without_losing_customizations() {
        let path = std::env::temp_dir().join(format!(
            "tgpulse-player-migration-{}.conf",
            std::process::id()
        ));
        let original = "format = signals-v1\ncoin = F11\ncoin2 = F12, pad:North\nstart2 = \n";
        std::fs::write(&path, original).unwrap();
        let b = Bindings::load_or_create(&path);
        assert_eq!(b.binding(Signal::Coin).text, "F11");
        assert_eq!(
            b.player_binding(Player::Two, Signal::Coin).text,
            "F12, pad:North"
        );
        assert_eq!(b.player_binding(Player::Two, Signal::Start).text, "");
        assert_eq!(
            std::fs::read_to_string(path.with_extension("conf.pre-players")).unwrap(),
            original
        );
        assert_eq!(Bindings::load(&path).controls_p2, b.controls_p2);
        std::fs::write(
            &path,
            "format = signals-v1\ncoin2 = Digit6\nstart2 = Digit2\n",
        )
        .unwrap();
        let b = Bindings::load(&path);
        assert_eq!(
            b.player_binding(Player::Two, Signal::Coin).text,
            "Digit6, pad:Select"
        );
        assert_eq!(
            b.player_binding(Player::Two, Signal::Start).text,
            "Digit2, pad:Start"
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(path.with_extension("conf.pre-players")).unwrap();
    }
    #[test]
    fn model2_p2_migration_only_fills_previously_disabled_rows_once() {
        let path =
            std::env::temp_dir().join(format!("tgpulse-model2-p2-{}.conf", std::process::id()));
        let original = "format = signals-v2\naction1 = KeyU\np2.action1 = \np2.gun_yaw = \np2.gun_pitch = keys:KeyT/KeyG\np2.accelerator = \ncontroller_p2 = custom:1\n";
        std::fs::write(&path, original).unwrap();
        let mut b = Bindings::load_or_create(&path);
        assert_eq!(b.binding(Signal::Action1).text, "KeyU");
        assert_eq!(b.player_binding(Player::Two, Signal::Action1).text, "");
        assert_eq!(
            b.player_binding(Player::Two, Signal::GunYaw).text,
            "pad:LeftStickX"
        );
        assert_eq!(
            b.player_binding(Player::Two, Signal::GunPitch).text,
            "keys:KeyT/KeyG"
        );
        assert_eq!(
            b.player_binding(Player::Two, Signal::Accelerator).text,
            "pad:RightZ+"
        );
        assert_eq!(b.controllers[1], "custom:1");
        assert_eq!(
            std::fs::read_to_string(path.with_extension("conf.pre-model2-players")).unwrap(),
            original
        );
        b.set_player_expression(Player::Two, Signal::GunYaw, "")
            .unwrap();
        b.save(&path).unwrap();
        assert_eq!(
            Bindings::load_or_create(&path)
                .player_binding(Player::Two, Signal::GunYaw)
                .text,
            ""
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(path.with_extension("conf.pre-model2-players")).unwrap();
    }
    #[test]
    fn existing_signals_file_inherits_shared_throttle_defaults() {
        let path = std::env::temp_dir().join(format!(
            "tgpulse-wingwar-bindings-{}.conf",
            std::process::id()
        ));
        let original = "format = signals-v1\naction1 = F12\naccelerator = KeyU\nbrake = KeyO\n";
        std::fs::write(&path, original).unwrap();
        let loaded = Bindings::load_or_create(&path);
        assert_eq!(loaded.binding(Signal::Action1).text, "F12");
        assert_eq!(loaded.binding(Signal::Accelerator).text, "KeyU");
        assert_eq!(loaded.binding(Signal::Brake).text, "KeyO");
        assert_eq!(
            loaded.binding(Signal::ThrottleUp).text,
            "KeyW, ArrowUp, pad:LeftZ+, pad:RightStickY+"
        );
        assert_eq!(
            loaded.binding(Signal::ThrottleDown).text,
            "KeyS, ArrowDown, pad:RightZ+, pad:RightStickY-"
        );
        assert_eq!(
            std::fs::read_to_string(path.with_extension("conf.pre-players")).unwrap(),
            original
        );
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("format = signals-v3"));
        std::fs::remove_file(path.with_extension("conf.pre-players")).unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn wingwar_throttle_bindings_load_and_save_as_shared_signals() {
        let path = std::env::temp_dir().join(format!(
            "tgpulse-throttle-rename-{}.conf",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "format = signals-v1\nwingwar_throttle_up = F11\nwingwar_throttle_down = F12\n",
        )
        .unwrap();
        let loaded = Bindings::load(&path);
        assert_eq!(loaded.binding(Signal::ThrottleUp).text, "F11");
        assert_eq!(loaded.binding(Signal::ThrottleDown).text, "F12");
        loaded.save(&path).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("\nthrottle_up = F11\n"));
        assert!(saved.contains("\nthrottle_down = F12\n"));
        assert!(!saved.contains("wingwar_throttle"));
        let reloaded = Bindings::load(&path);
        assert_eq!(
            reloaded.binding(Signal::ThrottleUp),
            loaded.binding(Signal::ThrottleUp)
        );
        assert_eq!(
            reloaded.binding(Signal::ThrottleDown),
            loaded.binding(Signal::ThrottleDown)
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn migration_keeps_custom_keyboard_and_hotkey_uniqueness() {
        let mut b = Bindings::default();
        b.migrate_keyboard("coin1 = F12, pad:North");
        assert_eq!(b.binding(Signal::Coin).text, "F12, pad:Select");
        b.bind_hotkey(Hotkey::Reset, KeyCode::F5);
        assert_eq!(b.hotkey(Hotkey::SaveState), None);
    }

    #[test]
    fn migration_backs_up_original_once() {
        let path =
            std::env::temp_dir().join(format!("tgpulse-migrate-{}.conf", std::process::id()));
        let backup = path.with_extension("conf.pre-signals");
        let original = "coin1 = F12\nleft = KeyA\nright = KeyD\nfullscreen = F10\n";
        std::fs::write(&path, original).unwrap();
        let migrated = Bindings::load_or_create(&path);
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);
        assert!(migrated
            .binding(Signal::Steering)
            .text
            .contains("keys:KeyA/KeyD"));
        assert_eq!(migrated.hotkey(Hotkey::Fullscreen), Some(KeyCode::F10));
        assert_eq!(Bindings::load_or_create(&path).controls, migrated.controls);
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(backup).unwrap();
    }
}
