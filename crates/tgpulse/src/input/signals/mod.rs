//! The single native cabinet signal catalogue, shared by bindings and input.
#[cfg(test)]
mod audit;
pub mod expression;
use expression::Binding;
use std::collections::BTreeMap;
macro_rules! signals {
    ($( $id:ident, $label:literal, $key:literal, $axis:literal, $default:literal; )*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Signal { $( $id, )* }
        impl Signal {
            pub const ALL: &'static [Self] = &[$( Self::$id, )*];
            pub fn label(self) -> &'static str { match self { $( Self::$id => $label, )* } }
            pub fn key(self) -> &'static str { match self { $( Self::$id => $key, )* } }
            pub fn signed(self) -> bool { match self { $( Self::$id => $axis, )* } }
            pub fn default_text(self) -> &'static str { match self { $( Self::$id => $default, )* } }
            pub fn from_key(key: &str) -> Option<Self> {
                // Preserve existing customized Wing War bindings on load;
                // the GUI and saved files expose only the shared signals.
                let key = match key {
                    "wingwar_throttle_up" => "throttle_up",
                    "wingwar_throttle_down" => "throttle_down",
                    other => other,
                };
                Self::ALL.iter().copied().find(|s| s.key()==key)
            }
        }
    }
}
signals! {
    Coin, "Coin", "coin", false, "Digit5, pad:Select";
    Start, "Start", "start", false, "Enter, NumpadEnter, pad:Start";
    Test, "Test", "test", false, "F2, pad:LeftThumb";
    Service, "Service", "service", false, "F8, pad:RightThumb";
    Up, "Joystick Up", "up", false, "ArrowUp, KeyW, pad:DPadUp";
    Down, "Joystick Down", "down", false, "ArrowDown, KeyS, pad:DPadDown";
    Left, "Joystick Left", "left", false, "ArrowLeft, KeyA, pad:DPadLeft";
    Right, "Joystick Right", "right", false, "ArrowRight, KeyD, pad:DPadRight";
    Action1, "Button 1 / Kick", "action1", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    Action2, "Button 2 / Punch", "action2", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    Action3, "Button 3 / Guard / Jump / Hold / Barrier", "action3", false, "KeyL, pad:West";
    View1, "View / Select 1", "view1", false, "KeyZ, pad:DPadDown";
    View2, "View / Select 2", "view2", false, "KeyX, pad:DPadLeft";
    View3, "View / Select 3", "view3", false, "KeyC, pad:DPadRight";
    View4, "View / Select 4", "view4", false, "KeyV, pad:DPadUp";
    Steering, "Steering / Bank", "steering", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    Accelerator, "Accelerator", "accelerator", false, "KeyW, ArrowUp, pad:RightZ+";
    Brake, "Brake", "brake", false, "KeyS, ArrowDown, pad:LeftZ+";
    Gear1, "H-Gate: Gear 1", "gear1", false, "Digit1, pad:RightStickX- & pad:RightStickY+";
    Gear2, "H-Gate: Gear 2", "gear2", false, "Digit2, pad:RightStickX- & pad:RightStickY-";
    Gear3, "H-Gate: Gear 3", "gear3", false, "Digit3, pad:RightStickX+ & pad:RightStickY+";
    Gear4, "H-Gate: Gear 4", "gear4", false, "Digit4, pad:RightStickX+ & pad:RightStickY-";
    Neutral, "H-Gate: Neutral", "neutral", false, "Digit0, pad:West";
    GearDown, "Gear Down", "gear_down", false, "KeyE, pad:LeftTrigger";
    GearUp, "Gear Up", "gear_up", false, "KeyQ, pad:RightTrigger";
    SkyX, "Analog Joystick X", "analog_x", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    SkyY, "Analog Joystick Y", "analog_y", true, "keys:KeyG/KeyT, pad:LeftStickY";
    ThrottleUp, "Throttle Up", "throttle_up", false, "KeyW, ArrowUp, pad:LeftZ+, pad:RightStickY+";
    ThrottleDown, "Throttle Down", "throttle_down", false, "KeyS, ArrowDown, pad:RightZ+, pad:RightStickY-";
    GunYaw, "Gun Yaw", "gun_yaw", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    GunPitch, "Gun Pitch", "gun_pitch", true, "keys:ArrowDown/ArrowUp, keys:KeyS/KeyW, pad:LeftStickY";
    PrimaryFire, "Gun Primary Fire", "primary_fire", false, "KeyJ, KeyE, Space, pad:South, pad:RightTrigger";
    SecondaryFire, "Gun Secondary Fire", "secondary_fire", false, "KeyK, KeyQ, KeyR, pad:East, pad:LeftTrigger";
    Elevation, "Desert Tank: Elevation", "elevation", true, "keys:KeyG/KeyT, pad:LeftStickY, pad:RightStickY";
    DesertGun, "Desert Tank: Machine Gun", "desert_gun", false, "KeyJ, KeyE, Space, pad:South, pad:RightTrigger";
    DesertCannon, "Desert Tank: Cannon", "desert_cannon", false, "KeyK, KeyQ, KeyR, pad:East, pad:LeftTrigger";
    DesertShift, "Desert Tank: Shift", "desert_shift", false, "KeyL, pad:West";
    BatSwing, "Dynamite Baseball: Bat Swing", "bat_swing", false, "KeyI, pad:RightStickY-";
    NetmercButton1, "NetMerc: Button 1", "netmerc_button1", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    NetmercButton2, "NetMerc: Button 2", "netmerc_button2", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    NetmercMvdHolder, "NetMerc: MVD Holder", "netmerc_mvd_holder", false, "KeyL, pad:West";
    SledEntry, "Power Sled: Entry", "sled_entry", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    SledCall, "Power Sled: Call", "sled_call", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    Action4, "Power Sled: Cancel Error", "action4", false, "KeyI, pad:North";
    Handbrake, "Sega Rally: Handbrake", "handbrake", false, "KeyI, pad:South, pad:North";
    Swing, "Ski Super G: Swing", "swing", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    Inclining, "Ski Super G: Inclining", "inclining", true, "keys:KeyU/KeyO, pad:RightStickX";
    SkiFootLeft, "Ski Super G: Foot Sensor Left", "ski_foot_left", false, "KeyK, pad:LeftTrigger";
    SkiFootRight, "Ski Super G: Foot Sensor Right", "ski_foot_right", false, "KeyJ, pad:RightTrigger";
    SkiSelect1, "Ski Super G: Select 1", "ski_select1", false, "KeyL, pad:West";
    SkiSelect2, "Ski Super G: Select 2", "ski_select2", false, "KeyI, pad:South, pad:North";
    SkiSelect3, "Ski Super G: Select 3", "ski_select3", false, "KeyK, pad:East";
    SkyMachineGun, "Sky Target: Machine Gun", "sky_machine_gun", false, "KeyJ, KeyE, Space, pad:South, pad:RightTrigger";
    SkyMissile, "Sky Target: Missile", "sky_missile", false, "KeyK, KeyQ, KeyR, pad:East, pad:LeftTrigger";
    SwaLaser, "Star Wars Arcade: Laser", "swa_laser", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    SwaTorpedo, "Star Wars Arcade: Torpedo", "swa_torpedo", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    SkaterSlide, "Top Skater: Slide", "top_skater_slide", true, "keys:KeyU/KeyO, pad:RightStickX";
    Curving, "Top Skater: Curving", "curving", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    SkaterJumpTail, "Top Skater: Jump Tail", "skater_jump_tail", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    SkaterJumpFront, "Top Skater: Jump Front", "skater_jump_front", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    StrikerShortPass, "Virtua Striker: Short Pass", "striker_short_pass", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    StrikerLongPass, "Virtua Striker: Long Pass", "striker_long_pass", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    StrikerShoot, "Virtua Striker: Shoot", "striker_shoot", false, "KeyL, pad:West";
    TwinLeftX, "Virtual On: Left Joystick X", "twin_left_x", true, "keys:KeyA/KeyD, pad:LeftStickX";
    TwinLeftY, "Virtual On: Left Joystick Y", "twin_left_y", true, "keys:KeyS/KeyW, pad:LeftStickY";
    TwinRightX, "Virtual On: Right Joystick X", "twin_right_x", true, "keys:ArrowLeft/ArrowRight, pad:RightStickX";
    TwinRightY, "Virtual On: Right Joystick Y", "twin_right_y", true, "keys:ArrowDown/ArrowUp, pad:RightStickY";
    TwinLeftShot, "Virtual On: Left Shot Trigger", "twin_left_shot", false, "KeyJ, pad:LeftZ+";
    TwinRightShot, "Virtual On: Right Shot Trigger", "twin_right_shot", false, "KeyK, pad:RightZ+";
    TwinLeftDash, "Virtual On: Left Dash (Turbo)", "twin_left_dash", false, "KeyL, pad:LeftTrigger";
    TwinRightDash, "Virtual On: Right Dash (Turbo)", "twin_right_dash", false, "KeyI, pad:RightTrigger";
    WaterSlide, "Water Ski: Slide", "water_ski_slide", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    WaterSet, "Water Ski: Set", "water_set", false, "KeyL, pad:South";
    WaterPitchLeft, "Water Ski: Pitch Left", "water_pitch_left", false, "KeyK, pad:West, pad:LeftTrigger";
    WaterPitchRight, "Water Ski: Pitch Right", "water_pitch_right", false, "KeyJ, pad:East, pad:RightTrigger";
    Handle, "Wave Runner: Handle", "handle", true, "keys:ArrowLeft/ArrowRight, keys:KeyA/KeyD, pad:LeftStickX";
    Pitch, "Wave Runner: Pitch", "pitch", true, "keys:KeyG/KeyT, pad:LeftStickY";
    Roll, "Wave Runner: Roll", "roll", true, "keys:KeyU/KeyO, pad:RightStickX";
    WingMachineGun, "Wing War: Machine Gun", "wing_machine_gun", false, "KeyJ, KeyE, Space, pad:South, pad:LeftTrigger";
    WingMissile, "Wing War: Missile", "wing_missile", false, "KeyK, KeyQ, KeyR, pad:East, pad:RightTrigger";
    WingSmoke, "Wing War: Smoke", "wing_smoke", false, "KeyL, pad:West";
}
pub fn defaults() -> BTreeMap<Signal, Binding> {
    Signal::ALL
        .iter()
        .map(|&s| {
            (
                s,
                Binding::parse(s.default_text(), s.signed()).expect("valid default"),
            )
        })
        .collect()
}

impl Signal {
    /// One catalogue for both seats. Capability is not filtered by loaded game.
    /// Enabled when at least one supported Model 1/2 cabinet has that P2 input.
    pub fn supports_p2(self) -> bool {
        matches!(
            self,
            Self::Coin
                | Self::Start
                | Self::Test
                | Self::Service
                | Self::Up
                | Self::Down
                | Self::Left
                | Self::Right
                | Self::Action1
                | Self::Action2
                | Self::Action3
                | Self::SledEntry
                | Self::SledCall
                | Self::SwaLaser
                | Self::SwaTorpedo
                | Self::StrikerShortPass
                | Self::StrikerLongPass
                | Self::StrikerShoot
                | Self::SkyX
                | Self::SkyY
                | Self::GunYaw
                | Self::GunPitch
                | Self::BatSwing
                | Self::PrimaryFire
                | Self::SecondaryFire
                | Self::Accelerator
                | Self::Brake
        )
    }

    pub fn p2_default_text(self) -> String {
        if !self.supports_p2() {
            return String::new();
        }
        match self {
            Self::Coin => "Digit6, pad:Select".into(),
            Self::Start => "Digit2, pad:Start".into(),
            Self::Test | Self::Service => self.default_text().into(),
            _ => self
                .default_text()
                .split(',')
                .map(str::trim)
                .filter(|s| s.starts_with("pad:"))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    /// Game families, including their revisions. Kept with the public labels.
    pub fn usage(self) -> Option<&'static str> {
        match self {
            Self::SkyX | Self::SkyY => {
                Some("(Sky Target, Star Wars Arcade (Pilot), Wing War, NetMerc)")
            }
            Self::ThrottleUp | Self::ThrottleDown => Some("(Star Wars Arcade (Pilot), Wing War)"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn driving_signals_are_consecutive() {
        let start = Signal::ALL
            .iter()
            .position(|s| *s == Signal::Steering)
            .unwrap();
        assert_eq!(
            &Signal::ALL[start..start + 10],
            &[
                Signal::Steering,
                Signal::Accelerator,
                Signal::Brake,
                Signal::Gear1,
                Signal::Gear2,
                Signal::Gear3,
                Signal::Gear4,
                Signal::Neutral,
                Signal::GearDown,
                Signal::GearUp,
            ]
        );
    }

    #[test]
    fn gun_signals_precede_lexical_game_groups() {
        use Signal as S;
        let gun = S::ALL.iter().position(|s| *s == S::GunYaw).unwrap();
        assert_eq!(
            &S::ALL[gun..gun + 4],
            &[S::GunYaw, S::GunPitch, S::PrimaryFire, S::SecondaryFire]
        );
        assert_eq!(S::ALL[gun + 4], S::Elevation);
        let sled = S::ALL.iter().position(|s| *s == S::SledEntry).unwrap();
        assert_eq!(
            &S::ALL[sled..sled + 3],
            &[S::SledEntry, S::SledCall, S::Action4]
        );
        let mut previous_game = "";
        for signal in &S::ALL[gun + 4..] {
            let (game, _) = signal.label().split_once(':').expect("game-specific label");
            assert!(game >= previous_game, "game groups are not lexicographic");
            previous_game = game;
        }
        assert_eq!(
            S::ALL
                .iter()
                .filter(|s| s.usage().is_some() && **s == S::View1)
                .count(),
            0
        );
        assert_eq!(S::Action4.label(), "Power Sled: Cancel Error");
        assert_eq!(S::Action4.key(), "action4");
        assert_eq!(S::BatSwing.key(), "bat_swing");
        assert_eq!(S::PrimaryFire.label(), "Gun Primary Fire");
        assert_eq!(S::SecondaryFire.label(), "Gun Secondary Fire");
        assert_eq!(S::SecondaryFire.usage(), None);
        assert_eq!(S::SkyMachineGun.label(), "Sky Target: Machine Gun");
        assert_eq!(S::SkyMissile.label(), "Sky Target: Missile");
        assert_ne!(S::PrimaryFire.key(), S::SkyMachineGun.key());
        assert_ne!(S::SecondaryFire.key(), S::SkyMissile.key());
    }
}
