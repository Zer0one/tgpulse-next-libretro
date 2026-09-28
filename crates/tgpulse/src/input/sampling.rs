//! Physical sources resolve directly to the sole public signal catalogue.
use super::*;
use signals::{expression::Atom, Signal as S};
impl InputState {
    pub(super) fn signal(&self, signal: S) -> f32 {
        let value = self.sample_signal(signal, false, false);
        // Single-view cabinets may use either gamepad view position. Reuse the
        // assignable View4 pad binding (default D-pad Up), not a hidden button
        // check. Do not alias keyboard View4 or multi-view cabinets.
        if signal == S::View1
            && (self.game.starts_with("srally")
                || matches!(self.game.as_str(), "sgt24h" | "swa" | "swaj"))
        {
            value.max(self.sample_player_signal(Player::One, S::View4, false, true))
        } else {
            value
        }
    }
    pub(super) fn sample_signal(&self, signal: S, keyboard_only: bool, pad_only: bool) -> f32 {
        let bound = self.sample_player_signal(Player::One, signal, keyboard_only, pad_only);
        let touch = if keyboard_only {
            0.0
        } else {
            self.touch_amount(signal)
        };
        if touch.abs() > bound.abs() {
            touch
        } else {
            bound
        }
    }
    pub(super) fn signal_p2(&self, signal: S) -> f32 {
        if !signal.supports_p2() {
            return 0.0;
        }
        self.sample_player_signal(Player::Two, signal, false, false)
    }
    fn sample_player_signal(
        &self,
        player: Player,
        signal: S,
        keyboard_only: bool,
        pad_only: bool,
    ) -> f32 {
        self.bindings
            .player_binding(player, signal)
            .value(|atom| match atom {
                Atom::Source(Source::Key(k)) if !pad_only => f32::from(u8::from(self.held(*k))),
                Atom::Source(Source::Key(_)) => 0.0,
                Atom::Source(s) if !keyboard_only => self.source_amount_for(player, *s),
                Atom::Axis(axis, sign) if !keyboard_only => {
                    let v = self.axis_value_for(player, *axis) * sign;
                    if v.abs() > 0.15 {
                        v
                    } else {
                        0.0
                    }
                }
                Atom::Keys(negative, positive) if !pad_only => {
                    f32::from(u8::from(self.held(*positive)))
                        - f32::from(u8::from(self.held(*negative)))
                }
                _ => 0.0,
            })
    }
}
