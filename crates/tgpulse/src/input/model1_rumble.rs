//! Desktop gamepad approximation of the Model 1 motor board. The core only
//! decodes cabinet commands; this frontend decides what two pad motors can do.

use tgpulse_core::model1_drive::{decode, DriveCommand, DriveEffect};

const PAD_CEILING: f32 = 0.6;
const BURST_FRAMES: u8 = 12;
const STEER_CENTRE: f32 = 0x80 as f32;
const STEER_TRAVEL: f32 = 0x60 as f32;
const CORNER_DEADZONE: f32 = 0.2;

/// Upstream binary cabinet motor adaptation, separate from the VR protocol.
pub fn binary_motor_levels(on: bool) -> (f32, f32) {
    let gain = if on { PAD_CEILING } else { 0.0 };
    (gain, gain)
}

#[derive(Default)]
pub struct Model1PadRumble {
    board_active: bool,
    current: Option<DriveCommand>,
    last_push_direction: Option<DriveEffect>,
    flip_boost_remaining: u8,
}

impl Model1PadRumble {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Return low/high motor levels. Other/handshake bytes do not replace the
    /// current effect; the original board holds it until a new command arrives.
    pub fn frame(&mut self, raw: u8, steer: u8) -> (f32, f32) {
        let incoming = decode(raw);
        if incoming.effect != DriveEffect::Other {
            self.board_active = true;
            if matches!(
                incoming.effect,
                DriveEffect::PushLeft | DriveEffect::PushRight
            ) {
                if self
                    .last_push_direction
                    .is_some_and(|d| d != incoming.effect)
                {
                    self.flip_boost_remaining = BURST_FRAMES;
                }
                self.last_push_direction = Some(incoming.effect);
            } else {
                self.flip_boost_remaining = 0;
                self.last_push_direction = None;
            }
            self.current = Some(incoming);
        }
        if !self.board_active {
            return (0.0, 0.0); // Standard cabinet: no drive-board activity.
        }

        // The wheel's centring load has no literal pad equivalent. Following
        // SM2-Emu's pad approximation, a subtler buzz follows steering
        // deflection only under centring/friction. Release stays silent.
        let deflection = ((f32::from(steer) - STEER_CENTRE).abs() / STEER_TRAVEL).min(1.0);
        let cornering = if self.current.is_some_and(|command| {
            matches!(command.effect, DriveEffect::Clutch | DriveEffect::Center)
        }) {
            ((deflection - CORNER_DEADZONE) / (1.0 - CORNER_DEADZONE)).clamp(0.0, 1.0)
                * PAD_CEILING
                * 0.4
        } else {
            0.0
        };
        let vibration = self.current.map_or(0.0, |command| {
            if command.effect == DriveEffect::Uncenter {
                PAD_CEILING * (f32::from(command.step) + 1.0) / 16.0
            } else {
                0.0
            }
        });
        let push = self.current.map_or(0.0, |command| {
            if matches!(
                command.effect,
                DriveEffect::PushLeft | DriveEffect::PushRight
            ) {
                PAD_CEILING / 4.0 + PAD_CEILING * 3.0 / 4.0 * (f32::from(command.step) + 1.0) / 16.0
            } else {
                0.0
            }
        });
        let push = if self.flip_boost_remaining > 0 {
            self.flip_boost_remaining -= 1;
            (push * 1.5).min(1.0)
        } else {
            push
        };
        let low = cornering.max(vibration).max(push);
        (low, low / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Model1PadRumble, BURST_FRAMES};

    #[test]
    fn standard_cabinet_handshake_never_synthesizes_rumble() {
        let mut pad = Model1PadRumble::default();
        for _ in 0..60 {
            assert_eq!(pad.frame(0x00, 0xe0), (0.0, 0.0));
        }
    }

    #[test]
    fn clutch_and_center_are_not_impacts_but_allow_subtle_cornering() {
        let mut pad = Model1PadRumble::default();
        assert_eq!(pad.frame(0x20, 0x80), (0.0, 0.0));
        let (low, high) = pad.frame(0x3a, 0xe0);
        assert!(low > 0.0 && low < 0.3);
        assert_eq!(high, low / 2.0);
        assert_eq!(pad.frame(0x00, 0xe0), (low, high));
    }

    #[test]
    fn fourth_power_bit_and_zero_step_both_matter() {
        let mut pad = Model1PadRumble::default();
        let weak = pad.frame(0x40, 0x80).0;
        let strong = pad.frame(0x48, 0x80).0;
        assert!(weak > 0.0 && strong > weak);
        assert!(pad.frame(0x4f, 0x80).0 > strong);
        assert!(pad.frame(0x50, 0x80).0 > 0.0);
    }

    #[test]
    fn directional_push_is_held_with_a_bounded_flip_boost_and_release_stops_it() {
        let mut pad = Model1PadRumble::default();
        let normal = pad.frame(0x5f, 0x80).0;
        assert!(normal > 0.0);
        assert_eq!(pad.frame(0x5f, 0x80).0, normal);
        let boosted = pad.frame(0x6f, 0x80).0;
        assert!(boosted > normal);
        for _ in 1..BURST_FRAMES {
            assert_eq!(pad.frame(0x6f, 0x80).0, boosted);
        }
        assert_eq!(pad.frame(0x6f, 0x80).0, normal);
        assert_eq!(pad.frame(0x10, 0xe0), (0.0, 0.0));
        assert_eq!(pad.frame(0x10, 0xe0), (0.0, 0.0));
        pad.reset();
        assert_eq!(pad.frame(0x00, 0xe0), (0.0, 0.0));
    }
}
