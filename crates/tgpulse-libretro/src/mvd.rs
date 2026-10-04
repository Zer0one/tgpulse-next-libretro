//! Frontend-only NetMerc MVD policy, adapted from standalone input/mvd.rs.
//! The frontend owns stick dead zones and remapping. Sensor commands are
//! distinct from cabinet switches. Sensors are owned by the frontend.
use tgpulse_core::model1io2::HmdPose;

pub const KEYS: [&std::ffi::CStr; 6] = [
    c"tgpulse_next_netmerc_mvd_input",
    c"tgpulse_next_netmerc_mvd_horizontal_range",
    c"tgpulse_next_netmerc_mvd_vertical_range",
    c"tgpulse_next_netmerc_mvd_gravity_stabilization",
    c"tgpulse_next_netmerc_mvd_drift_compensation",
    c"tgpulse_next_netmerc_mvd_sensor_diagnostics",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Auto,
    Off,
    RightStick,
    Sensors,
}

impl Mode {
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("off") => Self::Off,
            Some("right_stick") => Self::RightStick,
            Some("sensors") => Self::Sensors,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub mode: Mode,
    pub range: [u32; 2],
    pub gravity: bool,
    pub drift_compensation: u32,
    pub sensor_diagnostics: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Auto,
            range: [30, 20],
            gravity: true,
            drift_compensation: 50,
            sensor_diagnostics: false,
        }
    }
}

pub fn range(value: Option<&str>, default: u32) -> u32 {
    value
        .and_then(|s| s.parse().ok())
        .filter(|v: &u32| *v <= 90 && *v % 10 == 0)
        .unwrap_or(default)
}

pub fn drift_compensation(value: Option<&str>) -> u32 {
    if matches!(value, Some("off" | "0")) { return 0; }
    value.and_then(|s| s.parse().ok())
        .filter(|v: &u32| (10..=100).contains(v) && *v % 10 == 0)
        .unwrap_or(50)
}

/// Libretro X is positive right and Y positive down. Upstream's +XANG
/// looks right and +ZANG looks down; keep its neutral pose and raw scale.
/// This is absolute deflection: releasing the stick restores forward view.
pub fn pose(settings: Settings, pad_available: bool, axes: [f32; 2]) -> HmdPose {
    let mut pose = HmdPose::default();
    if matches!(settings.mode, Mode::Off | Mode::Sensors) || !pad_available {
        return pose;
    }
    for (channel, value, degrees) in [
        (0, axes[0], settings.range[0]),
        (2, axes[1], settings.range[1]),
    ] {
        pose.orientation[channel] +=
            (value.clamp(-1.0, 1.0) * degrees.min(90) as f32 * 25736.0 / 180.0).round() as i16;
    }
    pose
}

#[derive(Default)]
pub struct HolderNotice {
    last: Option<bool>,
}

impl HolderNotice {
    /// Match the standalone GUI: observe the game-acknowledged latch, not
    /// button level. Repeated frames and unchanged restores do not renew it.
    pub fn update(&mut self, state: Option<bool>) -> Option<&'static str> {
        if state == self.last {
            return None;
        }
        self.last = state;
        state.map(|latched| {
            if latched {
                "MVD Holder: Latched"
            } else {
                "MVD Holder: Cleared"
            }
        })
    }
}

#[derive(Default)]
pub struct Commands {
    held: Option<[bool; 2]>,
}

impl Commands {
    /// Resynchronize after load/reset/device changes. Already-held commands
    /// must be released before they can fire again; no synthetic press.
    pub fn resync(&mut self) {
        self.held = None;
    }

    pub fn pressed(&mut self, active: [bool; 2]) -> [bool; 2] {
        let edges = self.held.map_or([false; 2], |held| {
            std::array::from_fn(|i| active[i] && !held[i])
        });
        self.held = Some(active);
        edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holder_notice_tracks_game_latch_without_repeating_held_or_restored_state() {
        let mut notice = HolderNotice::default();
        assert_eq!(notice.update(None), None);
        assert_eq!(notice.update(Some(false)), Some("MVD Holder: Cleared"));
        assert_eq!(notice.update(Some(false)), None);
        assert_eq!(notice.update(Some(true)), Some("MVD Holder: Latched"));
        assert_eq!(notice.update(Some(true)), None);
        assert_eq!(notice.update(Some(false)), Some("MVD Holder: Cleared"));
        assert_eq!(notice.update(None), None);
        assert_eq!(notice.update(Some(true)), Some("MVD Holder: Latched"));
    }

    #[test]
    fn pose_preserves_neutral_and_matches_upstream_axis_directions() {
        let settings = Settings::default();
        let neutral = HmdPose::default();
        assert_eq!(pose(settings, true, [0.0; 2]), neutral);
        for mode in [Mode::Auto, Mode::RightStick] {
            let settings = Settings { mode, ..settings };
            assert_eq!(pose(settings, false, [1.0; 2]), neutral);
            let down_right = pose(settings, true, [1.0; 2]);
            let up_left = pose(settings, true, [-1.0; 2]);
            assert_eq!(down_right.orientation, [17157, 25736, 15728]);
            assert_eq!(up_left.orientation, [8579, 25736, 10008]);
            assert_eq!(down_right.position, neutral.position);
            assert!(pose(settings, true, [0.01, 0.01]).orientation[0] > neutral.orientation[0]);
        }
        assert_eq!(
            pose(
                Settings {
                    mode: Mode::Off,
                    ..settings
                },
                true,
                [1.0; 2]
            ),
            neutral
        );
        assert_eq!(
            pose(
                Settings {
                    range: [0, 0],
                    ..settings
                },
                true,
                [1.0; 2]
            ),
            neutral
        );
        assert_eq!(
            pose(
                Settings {
                    range: [90, 90],
                    ..settings
                },
                true,
                [1.0; 2]
            )
            .orientation,
            [25736, 25736, 25736]
        );
    }

    #[test]
    fn commands_fire_once_per_press_and_require_release_after_resync() {
        let mut commands = Commands::default();
        assert_eq!(commands.pressed([true; 2]), [false; 2]);
        assert_eq!(commands.pressed([false; 2]), [false; 2]);
        assert_eq!(commands.pressed([true, false]), [true, false]);
        assert_eq!(commands.pressed([true, true]), [false, true]);
        assert_eq!(commands.pressed([true; 2]), [false; 2]);
        commands.resync();
        assert_eq!(commands.pressed([true; 2]), [false; 2]);
        commands.pressed([false; 2]);
        assert_eq!(commands.pressed([true; 2]), [true; 2]);
    }

    #[test]
    fn invalid_settings_fall_back_to_approved_defaults() {
        for value in [None, Some("101"), Some("15"), Some("-10"), Some("bad")] {
            assert_eq!(drift_compensation(value), 50);
        }
        assert_eq!(drift_compensation(Some("off")), 0);
        assert_eq!(drift_compensation(Some("0")), 0);
        for value in (10..=100).step_by(10) {
            assert_eq!(drift_compensation(Some(&value.to_string())), value);
        }
        assert_eq!(Mode::parse(Some("sensors")), Mode::Sensors);
        for value in [None, Some("91"), Some("-10"), Some("15"), Some("bad")] {
            assert_eq!(range(value, 30), 30);
        }
        assert_eq!(range(Some("0"), 30), 0);
        assert_eq!(range(Some("90"), 20), 90);
    }
}

/// Upstream SDL orientation to native NetMerc angles, including neutral offsets.
pub fn sensor_pose([pitch, yaw, roll]: [f64; 3]) -> HmdPose {
    let mut pose = HmdPose::default();
    let raw = |r: f64| (r.to_degrees().clamp(-90.0, 90.0) * 25736.0 / 180.0).round() as i32;
    let wrap = |angle: i32| {
        let angle = if angle > 25736 { angle - 51472 }
            else if angle < -25736 { angle + 51472 } else { angle };
        angle as i16
    };
    pose.orientation[0] = wrap(i32::from(pose.orientation[0]) - raw(yaw));
    pose.orientation[1] = wrap(i32::from(pose.orientation[1]) + raw(roll));
    pose.orientation[2] = wrap(i32::from(pose.orientation[2]) - raw(pitch));
    pose
}
