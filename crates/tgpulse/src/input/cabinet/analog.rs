//! Hardware channel calibration and stateful cabinet controls, no host I/O.
use super::*;
use AnalogRole as A;
use ControlScheme as Scheme;
impl InputState {
    pub fn set_game(&mut self, game: &str) {
        if self.game != game {
            self.special_shift = false;
            self.special_shift_held = false;
            self.cursor_p2_active = false;
            self.game = game.to_owned();
            let rest_fraction = |role| {
                let (min, max, rest, reverse) = self.positional_gun_range(role);
                let fraction = (rest - min) as f32 / (max - min) as f32;
                if reverse {
                    1.0 - fraction
                } else {
                    fraction
                }
            };
            self.cursor_p2 = (rest_fraction(A::Gun2X), rest_fraction(A::Gun2Y));
        }
    }

    fn positional_gun_range(&self, role: A) -> (u8, u8, u8, bool) {
        match (self.game.as_str(), role) {
            ("gunblade" | "bel", A::Gun1X) => (0x69, 0xff, 0xb1, false),
            ("gunblade" | "bel", A::Gun2X) => (0x00, 0x96, 0x50, false),
            ("gunblade" | "bel", _) => (0x11, 0xae, 0x5f, false),
            ("rchase2", A::Gun1X) => (0x3a, 0xca, 0x82, true),
            ("rchase2", A::Gun2X) => (0x34, 0xc7, 0x7d, true),
            ("rchase2", _) => (0x1c, 0xcb, 0x73, true),
            _ => (0x00, 0xff, 0x80, false),
        }
    }

    /// Produce the cabinet's calibrated ADC channel from native signals and
    /// the existing ramped controls; preserve channel order from the ROM DB.
    pub(in crate::input) fn cabinet_axis(&self, role: A, axes: &Axes) -> u8 {
        let axis = |s| centered(self.signal(s), 128, 0, 255);
        // The legacy racing sampler ramps within Daytona/VR's 20..e0 range.
        // Translate that travel, not the bindings, for full-range cabinets.
        let full_range_car = matches!(self.scheme, Scheme::Racing | Scheme::Bike)
            && !self.game.is_empty()
            && !matches!(self.game.as_str(), "vr" | "vformula");
        if full_range_car {
            let pedal = |v: u8| {
                ((v.saturating_sub(0x20) as f32 / 192.0) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            match role {
                A::Steer => {
                    return centered(
                        (axes.steer as f32 - 128.0) / 96.0
                            * if self.scheme == Scheme::Bike {
                                -1.0
                            } else {
                                1.0
                            },
                        128,
                        0,
                        255,
                    )
                }
                A::Brake if self.game == "desert" => {
                    return centered(-self.signal(S::Elevation), 128, 0, 255)
                }
                A::Accel | A::Brake | A::Throttle => {
                    let v = pedal(axes.by_role(role));
                    return if self.game.starts_with("overrev") || self.game == "sgt24h" {
                        255 - v
                    } else {
                        v
                    };
                }
                _ => {}
            }
        }
        if self.scheme == Scheme::Flight && !self.game.is_empty() {
            let swa = self.game.starts_with("swa");
            let (mid, lo, hi) = if swa { (127, 27, 227) } else { (128, 0, 255) };
            match role {
                A::StickX => {
                    return centered(
                        self.signal(S::SkyX)
                            * if self.game == "netmerc" || self.game == "wingwar360" {
                                1.0
                            } else {
                                -1.0
                            },
                        if self.game == "netmerc" { 127 } else { mid },
                        lo,
                        hi,
                    )
                }
                // gilrs Y is positive UP; MAME's non-reversed ADC Y is positive DOWN.
                A::StickY => {
                    return centered(
                        self.signal(S::SkyY)
                            * if self.game == "netmerc" || self.game == "wingwar360" {
                                1.0
                            } else {
                                -1.0
                            },
                        if self.game == "netmerc" { 127 } else { mid },
                        lo,
                        hi,
                    )
                }
                A::Stick2X | A::Stick2Y if swa => {
                    return centered(
                        -self.signal_p2(if role == A::Stick2X { S::SkyX } else { S::SkyY }),
                        127,
                        27,
                        227,
                    );
                }
                A::Throttle if swa || self.game.starts_with("wingwar") => {
                    // Two assignable half-axes drive one cabinet ADC. Centre
                    // at release is a gamepad adaptation, not MAME's idle value.
                    // User-selected polarity: Up raises the ADC, Down lowers it.
                    // Keep signal names and right-stick assignments unchanged.
                    return centered(
                        self.signal(S::ThrottleUp) - self.signal(S::ThrottleDown),
                        128,
                        if swa { 28 } else { 1 },
                        if swa { 228 } else { 255 },
                    );
                }
                _ => {}
            }
        }
        match role {
            // Positional gun cabinets have calibrated ADC travel, not the
            // serial lightgun coordinates. rchase2a really differs from rchase2.
            A::Gun1X | A::Gun1Y | A::Gun2X | A::Gun2Y if !self.serial_gun() => {
                let (min, max, _rest, reverse) = self.positional_gun_range(role);
                // Do not quantize to an intermediate 0..255 axis before
                // applying the calibrated range (SM2 scales the cursor itself).
                let raw = match role {
                    A::Gun1X => self.cursor.0,
                    A::Gun1Y => self.cursor.1,
                    A::Gun2X => self.cursor_p2.0,
                    _ => self.cursor_p2.1,
                };
                let fraction = if reverse { 1.0 - raw } else { raw };
                (min as f32 + fraction * (max - min) as f32).round() as u8
            }
            A::Roll => axis(S::Roll),
            A::Pitch => axis(S::Pitch),
            A::Slide if self.scheme == Scheme::Skate => axis(S::SkaterSlide),
            A::Slide => centered(-self.signal(S::WaterSlide), 128, 0, 255),
            A::Curving => centered(-self.signal(S::Curving), 128, 0, 255),
            A::Swing => centered(-self.signal(S::Swing), 128, 0, 255),
            A::Incline => axis(S::Inclining),
            A::Bat1 => (255.0 * self.signal(S::BatSwing)).round() as u8,
            A::Bat2 => (255.0 * self.signal_p2(S::BatSwing)).round() as u8,
            A::P2R => (255.0 * self.signal_p2(S::Accelerator)).round() as u8,
            A::P2L => (255.0 * self.signal_p2(S::Brake)).round() as u8,
            _ => axes.by_role(role),
        }
    }

    pub(in crate::input) fn serial_gun(&self) -> bool {
        self.game.is_empty() || self.game.starts_with("vcop") || self.game.starts_with("hotd")
    }

    pub(in crate::input) fn h_gate(&self) -> bool {
        self.game.is_empty() || self.game.starts_with("daytona") || self.game.starts_with("srally")
    }

    pub(in crate::input) fn apply_direct_gear(&mut self) {
        if !self.h_gate() {
            return;
        }
        let mut active = [S::Neutral, S::Gear1, S::Gear2, S::Gear3, S::Gear4]
            .into_iter()
            .enumerate()
            .filter(|(_, s)| self.signal(*s) > 0.5)
            .map(|(i, _)| i);
        // A released stick or conflicting choices leave the current gear alone.
        let gear = active.next();
        if active.next().is_none() {
            if let Some(gear) = gear {
                self.gear = gear;
            }
        }
    }
}

fn centered(value: f32, rest: u8, min: u8, max: u8) -> u8 {
    let v = value.clamp(-1.0, 1.0);
    let span = if v < 0.0 { rest - min } else { max - rest };
    (rest as f32 + v * span as f32).round() as u8
}
