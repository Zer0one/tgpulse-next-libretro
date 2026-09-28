//! Device-neutral interpretation of Model 2 drive-board output bytes.
//! Games with drive boards use three distinct command protocols. The desktop
//! pad and any future frontend can consume these effects independently of SDL.
//! Behavioral reference: SM2-Emu's per-game drive protocols and output tables.

pub const FULL_STRENGTH: u16 = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Daytona,
    Rally,
    TouringCar,
}

impl Protocol {
    pub fn for_set(set: &str) -> Option<Self> {
        if set.starts_with("daytona") || set.starts_with("indy500") {
            Some(Self::Daytona)
        } else if set.starts_with("srallyc") {
            Some(Self::Rally)
        } else if set.starts_with("stcc") {
            Some(Self::TouringCar)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Other,
    Release,
    Spring,
    Friction,
    Vibrate,
    PushLeft,
    PushRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub effect: Effect,
    pub strength: u16,
    /// The game streams this force each frame; it is not an impact.
    pub held: bool,
}

impl Command {
    fn new(effect: Effect, step: u16, full_steps: u16, held: bool) -> Self {
        Self {
            effect,
            strength: step * FULL_STRENGTH / full_steps,
            held,
        }
    }

    fn other() -> Self {
        Self::new(Effect::Other, 0, 1, false)
    }

    fn release() -> Self {
        Self::new(Effect::Release, 0, 1, false)
    }

    pub fn is_push(self) -> bool {
        matches!(self.effect, Effect::PushLeft | Effect::PushRight)
    }
}

pub fn decode(protocol: Protocol, byte: u8) -> Command {
    match protocol {
        Protocol::Daytona | Protocol::TouringCar => {
            let step = u16::from(byte & 0x0f);
            let mut command = match byte & 0xf0 {
                0x10 if step <= 7 => Command::new(Effect::Spring, step + 1, 8, false),
                0x20 if step <= 7 => Command::new(Effect::Friction, step + 1, 8, false),
                0x30 if step <= 12 => Command::new(Effect::Spring, step + 1, 13, false),
                0x40 if step <= 7 => Command::new(Effect::Vibrate, step + 1, 8, false),
                0x50 if step <= 7 => Command::new(Effect::PushLeft, step, 7, false),
                0x60 if step <= 7 => Command::new(Effect::PushRight, step, 7, false),
                _ => Command::other(),
            };
            if protocol == Protocol::TouringCar && command.is_push() {
                command.effect = Effect::Spring;
                command.strength = step.saturating_sub(1) * FULL_STRENGTH / 6;
                command.held = true;
            }
            command
        }
        Protocol::Rally => {
            let step = u16::from(byte & 0x1f) + 1;
            match byte & 0xe0 {
                0x80 => Command::new(Effect::PushRight, step, 32, true),
                0xc0 => Command::new(Effect::PushLeft, step, 32, true),
                _ if byte == 0x00 => Command::release(),
                _ => Command::other(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, Effect, Protocol, FULL_STRENGTH};

    #[test]
    fn selects_only_drive_board_families() {
        assert_eq!(Protocol::for_set("daytonas"), Some(Protocol::Daytona));
        assert_eq!(Protocol::for_set("indy500d"), Some(Protocol::Daytona));
        assert_eq!(Protocol::for_set("srallycdx"), Some(Protocol::Rally));
        assert_eq!(Protocol::for_set("stcco"), Some(Protocol::TouringCar));
        assert_eq!(Protocol::for_set("vf2"), None);
    }

    #[test]
    fn daytona_distinguishes_effects_strength_and_handshakes() {
        let p = Protocol::Daytona;
        assert_eq!(decode(p, 0x10).effect, Effect::Spring);
        assert_eq!(decode(p, 0x27).effect, Effect::Friction);
        assert_eq!(decode(p, 0x3c).strength, FULL_STRENGTH);
        assert_eq!(decode(p, 0x40).effect, Effect::Vibrate);
        assert_eq!(decode(p, 0x50).strength, 0);
        assert_eq!(decode(p, 0x57).strength, FULL_STRENGTH);
        assert_eq!(decode(p, 0x67).effect, Effect::PushRight);
        for invalid in [0x00, 0x07, 0x3d, 0x48, 0x58, 0x6f, 0xb1, 0xa1] {
            assert_eq!(decode(p, invalid).effect, Effect::Other);
        }
    }

    #[test]
    fn rally_uses_five_bit_streamed_torque_and_explicit_release() {
        let p = Protocol::Rally;
        assert_eq!(decode(p, 0x80).strength, FULL_STRENGTH / 32);
        assert_eq!(decode(p, 0x9f).strength, FULL_STRENGTH);
        assert_eq!(decode(p, 0x9f).effect, Effect::PushRight);
        assert!(decode(p, 0xdf).held);
        assert_eq!(decode(p, 0xdf).effect, Effect::PushLeft);
        assert_eq!(decode(p, 0x00).effect, Effect::Release);
        assert_eq!(decode(p, 0x10).effect, Effect::Other);
    }

    #[test]
    fn touring_car_push_is_streamed_spring_with_idle_step() {
        let p = Protocol::TouringCar;
        let idle = decode(p, 0x51);
        assert_eq!(idle.effect, Effect::Spring);
        assert_eq!(idle.strength, 0);
        assert!(idle.held);
        assert_eq!(decode(p, 0x57).strength, FULL_STRENGTH);
        assert_eq!(decode(p, 0x67).effect, Effect::Spring);
        assert_eq!(decode(p, 0x58).effect, Effect::Other);
    }
}
