//! Pad-only rendering of Model 2 drive effects. Cabinet protocol decoding is
//! in tgpulse-core; this approximation never enters machine state or saves.

use tgpulse_core::model2_drive::{decode, Command, Effect, Protocol, FULL_STRENGTH};

const PAD_CEILING: f32 = 0.6;
const HOLD_FRAMES: u8 = 12;
const STEER_CENTRE: f32 = 0x80 as f32;
const STEER_TRAVEL: f32 = 0x60 as f32;
const CORNER_DEADZONE: f32 = 0.2;

#[derive(Default)]
pub struct Model2PadRumble {
    current: Option<Command>,
    last_push: Option<Effect>,
    level: f32,
    hold_remaining: u8,
}

impl Model2PadRumble {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn frame(&mut self, protocol: Protocol, raw: u8, steer: u8) -> (f32, f32) {
        let incoming = decode(protocol, raw);
        if incoming.effect != Effect::Other {
            // A transient Daytona-style spring byte must not overwrite a
            // streamed Touring Car spring. Other/parameter bytes also retain
            // the prior effect.
            let transient_spring = incoming.effect == Effect::Spring && !incoming.held;
            let held_effect = self.current.is_some_and(|command| command.held);
            if !(transient_spring && held_effect) {
                self.current = Some(incoming);
            }
        }

        let deflection = ((f32::from(steer) - STEER_CENTRE).abs() / STEER_TRAVEL).min(1.0);
        // A centring wheel force cannot be reproduced by eccentric motors.
        // This quiet load cue is deliberately lower than a board impact.
        let cornering = ((deflection - CORNER_DEADZONE) / (1.0 - CORNER_DEADZONE)).clamp(0.0, 1.0)
            * PAD_CEILING
            * 0.4;

        let mut impact = 0.0;
        if let Some(command) = self.current {
            if !command.held
                && command.strength > 0
                && (command.is_push() || command.effect == Effect::Vibrate)
            {
                impact = PAD_CEILING / 4.0
                    + PAD_CEILING * 3.0 / 4.0 * f32::from(command.strength)
                        / f32::from(FULL_STRENGTH);
                if command.is_push() {
                    if self
                        .last_push
                        .is_some_and(|direction| direction != command.effect)
                    {
                        impact = (impact * 1.5).min(1.0);
                    }
                    self.last_push = Some(command.effect);
                }
            }
        }

        let target = impact.max(cornering);
        if target > 0.0 {
            self.level = target;
            self.hold_remaining = HOLD_FRAMES;
        } else if self.hold_remaining > 0 {
            self.hold_remaining -= 1;
            if self.hold_remaining == 0 {
                self.level = 0.0;
                self.last_push = None;
            }
        }
        (self.level, self.level / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Model2PadRumble, HOLD_FRAMES};
    use tgpulse_core::model2_drive::Protocol;

    #[test]
    fn daytona_handshake_retains_effect_and_spring_is_not_a_stop() {
        let mut pad = Model2PadRumble::default();
        let p = Protocol::Daytona;
        assert_eq!(pad.frame(p, 0x00, 0x80), (0.0, 0.0));
        let hit = pad.frame(p, 0x57, 0x80).0;
        assert!(hit > 0.0);
        assert_eq!(pad.frame(p, 0xb1, 0x80).0, hit);
        assert!((pad.frame(p, 0x10, 0xe0).0 - 0.24).abs() < 0.0001);
    }

    #[test]
    fn rally_and_touring_car_streamed_forces_are_not_pad_impacts() {
        let mut pad = Model2PadRumble::default();
        assert_eq!(pad.frame(Protocol::Rally, 0xdf, 0x80), (0.0, 0.0));
        assert_eq!(pad.frame(Protocol::Rally, 0x00, 0x80), (0.0, 0.0));
        assert_eq!(pad.frame(Protocol::TouringCar, 0x57, 0x80), (0.0, 0.0));
        assert_eq!(pad.frame(Protocol::TouringCar, 0x10, 0x80), (0.0, 0.0));
        assert!(pad.frame(Protocol::TouringCar, 0x57, 0xe0).0 > 0.0);
    }

    #[test]
    fn impact_has_short_tail_and_reset_clears_it() {
        let mut pad = Model2PadRumble::default();
        let p = Protocol::Daytona;
        assert!(pad.frame(p, 0x47, 0x80).0 > 0.0);
        for _ in 0..HOLD_FRAMES - 1 {
            assert!(pad.frame(p, 0x50, 0x80).0 > 0.0);
        }
        assert_eq!(pad.frame(p, 0x50, 0x80), (0.0, 0.0));
        assert!(pad.frame(p, 0x67, 0x80).0 > 0.0);
        pad.reset();
        assert_eq!(pad.frame(p, 0x00, 0x80), (0.0, 0.0));
    }
}
