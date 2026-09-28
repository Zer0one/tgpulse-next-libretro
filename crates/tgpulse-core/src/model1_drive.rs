//! The original Model 1 motor-board command, before a host chooses how to
//! present it. No controller, SDL or wall-clock state belongs here.
//!
//! Virtua Racing sends a command family in the high nibble and a 0..15 power
//! step in the low nibble. The motor turns even at step zero; 0x1x releases it.
//! Source: SailorSat's first-hand Virtua Racing drive-board protocol notes
//! (forum.arcadecontrols.com/index.php?topic=145454.0, reply #4).

/// Drive-board protocols are selected by hardware family, not by the
/// availability of a cabinet-type entry in a game's service menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveFamily {
    VrMotor,
}

impl DriveFamily {
    pub fn for_set(set: &str) -> Option<Self> {
        match set {
            "vr" | "vformula" => Some(Self::VrMotor),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveEffect {
    Other,
    Release,
    Clutch,
    Center,
    Uncenter,
    PushLeft,
    PushRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DriveCommand {
    pub effect: DriveEffect,
    /// Hardware power step, 0..15. A zero step is still active except for
    /// Release and Other; consumers may normalize it as (step + 1) / 16.
    pub step: u8,
}

pub fn decode(command: u8) -> DriveCommand {
    let effect = match command & 0xf0 {
        0x10 => DriveEffect::Release,
        0x20 => DriveEffect::Clutch,
        0x30 => DriveEffect::Center,
        0x40 => DriveEffect::Uncenter,
        0x50 => DriveEffect::PushLeft,
        0x60 => DriveEffect::PushRight,
        _ => DriveEffect::Other,
    };
    DriveCommand {
        effect,
        step: command & 0x0f,
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, DriveEffect as E, DriveFamily};

    #[test]
    fn vr_motor_family_includes_vformula_but_not_other_model1_boards() {
        assert_eq!(DriveFamily::for_set("vr"), Some(DriveFamily::VrMotor));
        assert_eq!(DriveFamily::for_set("vformula"), Some(DriveFamily::VrMotor));
        for set in ["vf", "swa", "wingwar", "wingwar360", "netmerc"] {
            assert_eq!(DriveFamily::for_set(set), None, "{set}");
        }
    }

    #[test]
    fn decodes_every_vr_family_without_truncating_the_fourth_power_bit() {
        for (byte, effect, step) in [
            (0x00, E::Other, 0),
            (0x10, E::Release, 0),
            (0x2f, E::Clutch, 15),
            (0x30, E::Center, 0),
            (0x3a, E::Center, 10),
            (0x4f, E::Uncenter, 15),
            (0x50, E::PushLeft, 0),
            (0x5f, E::PushLeft, 15),
            (0x6f, E::PushRight, 15),
            (0x80, E::Other, 0),
        ] {
            assert_eq!(decode(byte).effect, effect, "{byte:02X}");
            assert_eq!(decode(byte).step, step, "{byte:02X}");
        }
    }
}
