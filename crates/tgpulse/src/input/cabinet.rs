//! Cabinet wiring consumes the public signal catalogue directly.
//! No physical keys/buttons belong here. Port bytes are built once per frame.
mod analog;
use super::*;
use Signal as S;

impl InputState {
    pub(super) fn poll_native_body(&mut self, out: &mut Inputs) {
        let wave = self.scheme == ControlScheme::Jetski;
        let water = self.game == "segawski";
        let ski = self.game == "skisuprg";
        let skate = self.game.starts_with("topskatr");
        let sled = self.scheme == ControlScheme::Sled;
        self.cabinet_common(
            out,
            if wave || water || skate { 0x40 } else { 0x10 },
            !water && self.game != "waverunr",
            sled,
        );
        let buttons: &[(usize, u8, S, f32)] = if wave {
            &[(1, 1, S::View4, 0.0)]
        } else if water {
            &[
                (1, 2, S::View4, 0.5),
                (0, 0x40, S::View1, 0.5),
                (1, 1, S::WaterSet, 0.5),
                (1, 4, S::WaterPitchLeft, 0.5),
                (1, 8, S::WaterPitchRight, 0.5),
            ]
        } else if ski {
            &[
                (0, 0x20, S::View4, 0.5),
                (1, 1, S::View1, 0.5),
                (0, 0x80, S::SkiSelect2, 0.5),
                (0, 0x10, S::SkiSelect3, 0.5),
                (0, 0x40, S::SkiSelect1, 0.5),
            ]
        } else if skate {
            &[
                (0, 0x80, S::View2, 0.5),
                (0, 0x10, S::View3, 0.5),
                (0, 0x20, S::SkaterJumpFront, 0.5),
                (1, 1, S::SkaterJumpTail, 0.5),
            ]
        } else if sled {
            &[
                (1, 1, S::SledEntry, 0.0),
                (1, 2, S::SledCall, 0.0),
                (0, 0x80, S::Action4, 0.5),
            ]
        } else {
            &[
                (1, 1, S::Action1, 0.0),
                (1, 2, S::Action2, 0.0),
                (1, 4, S::Action3, 0.0),
            ]
        };
        self.cabinet_buttons(out, buttons);
        if wave {
            out.in2 = 0xf7;
        }
        if ski {
            out.in2 = if self.signal(S::SkiFootLeft) > 0.5 {
                0xf0
            } else {
                0
            } | if self.signal(S::SkiFootRight) > 0.5 {
                0x0f
            } else {
                0
            };
        }
        if sled {
            if self.signal_p2(S::SledEntry) > 0.5 {
                out.in1 &= !4;
            }
            if self.signal_p2(S::SledCall) > 0.5 {
                out.in1 &= !8;
            }
        }
        let steering = match self.scheme {
            ControlScheme::Jetski => S::Handle,
            ControlScheme::Skate => S::Curving,
            ControlScheme::Ski if ski => S::Swing,
            ControlScheme::Ski => S::WaterSlide,
            _ => S::Steering,
        };
        let axis = |v: f32| (128 + (v * 127.0) as i32).clamp(0, 255) as u8;
        let x = axis(self.signal(steering));
        let y = axis(self.signal(S::Up) - self.signal(S::Down));
        let right = self.signal(S::Accelerator).clamp(0.0, 1.0);
        let left = self.signal(S::Brake).clamp(0.0, 1.0);
        let axes = if wave {
            Axes {
                steer: x,
                throttle: (128.0 * (1.0 - right)).round() as u8,
                ..Default::default()
            }
        } else {
            Axes {
                slide: x,
                swing: x,
                curving: x,
                incline: y,
                p1r: (right * 255.0) as u8,
                p1l: (left * 255.0) as u8,
                steer: x,
                ..Default::default()
            }
        };
        self.scatter(&axes, out);
    }

    pub(super) fn poll_native_flight(&mut self, out: &mut Inputs) {
        let wing = self.game.starts_with("wingwar");
        let swa = self.game.starts_with("swa");
        let netmerc = self.game == "netmerc";
        let sky = self.game == "skytargt";
        self.cabinet_common(
            out,
            if netmerc {
                0
            } else if sky {
                0x40
            } else {
                0x10
            },
            !netmerc && (!wing || self.game == "wingwar360"),
            false,
        );
        let buttons: &[(usize, u8, S, f32)] = if wing {
            if self.game == "wingwar360" {
                &[
                    (1, 0x10, S::WingMachineGun, 0.5),
                    (1, 0x20, S::WingMissile, 0.5),
                    (1, 0x40, S::WingSmoke, 0.5),
                ]
            } else {
                &[
                    (1, 0x10, S::WingMachineGun, 0.5),
                    (1, 0x20, S::WingMissile, 0.5),
                    (1, 0x40, S::WingSmoke, 0.5),
                    (0, 0x20, S::View1, 0.5),
                    (0, 0x40, S::View2, 0.5),
                    (0, 0x80, S::View3, 0.5),
                    (1, 1, S::View4, 0.5),
                ]
            }
        } else if sky {
            &[
                (1, 0x10, S::SkyMachineGun, 0.5),
                (1, 0x20, S::SkyMissile, 0.5),
                (0, 0x20, S::View4, 0.5),
            ]
        } else if netmerc {
            &[
                (1, 1, S::NetmercButton1, 0.5),
                (1, 2, S::NetmercButton2, 0.5),
                (1, 4, S::NetmercMvdHolder, 0.5),
            ]
        } else if swa {
            &[
                (1, 1, S::SwaLaser, 0.0),
                (1, 2, S::SwaTorpedo, 0.0),
                (1, 0x10, S::View1, 0.0),
            ]
        } else {
            &[
                (1, 1, S::Action1, 0.0),
                (1, 2, S::Action2, 0.0),
                (1, 0x10, S::Action3, 0.0),
            ]
        };
        self.cabinet_buttons(out, buttons);
        if swa {
            if self.signal_p2(S::SwaLaser) > 0.5 {
                out.in1 &= !4;
            }
            if self.signal_p2(S::SwaTorpedo) > 0.5 {
                out.in1 &= !8;
            }
        }
        // Fallback mirrors for uncatalogued flight cabinets, retaining the
        // original quantization. Catalogued ADC roles are calibrated below.
        let axis = |v: f32| (SWA_CENTRE + (v * SWA_STICK_RANGE as f32) as i32).clamp(0, 255) as u8;
        let x = axis(-self.signal(S::SkyX));
        let y = axis(self.signal(S::SkyY));
        let throttle = if swa || wing {
            self.signal(S::ThrottleUp) - self.signal(S::ThrottleDown)
        } else {
            self.signal(S::Accelerator) - self.signal(S::Brake)
        };
        self.scatter(
            &Axes {
                stickx: x,
                sticky: y,
                stick2x: x,
                stick2y: y,
                throttle: axis(-throttle),
                steer: x,
                accel: y,
                brake: axis(-throttle),
                ..Default::default()
            },
            out,
        );
    }

    fn cabinet_buttons(&self, out: &mut Inputs, buttons: &[(usize, u8, S, f32)]) {
        let ports = [&mut out.in0, &mut out.in1, &mut out.in2];
        for &(port, bit, s, threshold) in buttons {
            if self.signal(s) > threshold {
                *ports[port] &= !bit;
            }
        }
    }

    fn cabinet_common(&self, out: &mut Inputs, start: u8, coin2: bool, start2: bool) {
        out.in0 = 0xff;
        out.in1 = 0xff;
        out.in2 = 0xff;
        let test = self.signal(S::Test).max(self.signal_p2(S::Test));
        let service = self.signal(S::Service).max(self.signal_p2(S::Service));
        let (test_bit, service_bit) = if self.game == "bel" { (8, 4) } else { (4, 8) };
        for (bit, pressed) in [
            (1, self.signal(S::Coin) > 0.0),
            (2, coin2 && self.signal_p2(S::Coin) > 0.0),
            (test_bit, test > 0.0),
            (service_bit, service > 0.0),
            (
                start,
                self.signal(S::Start)
                    > if (start == 0x40 && self.scheme != ControlScheme::Jetski)
                        || self.game == "skisuprg"
                    {
                        0.5
                    } else {
                        0.0
                    },
            ),
            (0x20, start2 && self.signal_p2(S::Start) > 0.5),
        ] {
            if pressed {
                out.in0 &= !bit;
            }
        }
    }

    fn joystick_buttons(&self) -> &'static [(u8, S)] {
        if self.game == "vf"
            || self.game.starts_with("vf2")
            || self.game.starts_with("fvipers")
            || self.game.starts_with("lastbrnx")
            || self.game.starts_with("dynamcop")
            || self.game.starts_with("dyndeka2")
            || matches!(self.game.as_str(), "schamp" | "sfight")
        {
            // Requested SM2 convention: South/L1 Kick, East/R1 Punch,
            // West Guard. The global Action bindings stay unchanged.
            &[(1, S::Action2), (2, S::Action1), (4, S::Action3)]
        } else if self.game.starts_with("doa") {
            &[(1, S::Action3), (2, S::Action2), (4, S::Action1)]
        } else if self.game.starts_with("vstriker") {
            &[
                (1, S::StrikerLongPass),
                (2, S::StrikerShoot),
                (4, S::StrikerShortPass),
            ]
        } else if self.game.starts_with("dynabb") {
            &[(1, S::Action1), (2, S::Action2)]
        } else if matches!(self.game.as_str(), "hpyagu98" | "rascot2" | "airwlkrs") {
            &[(1, S::Action1), (2, S::Action2), (4, S::Action3)]
        } else if self.game.starts_with("pltkids") || self.game.starts_with("zerogun") {
            &[(1, S::Action1), (2, S::Action2)]
        } else {
            &[(1, S::Action1), (2, S::Action2), (4, S::Action3)]
        }
    }

    pub(super) fn poll_native_joystick(&mut self, out: &mut Inputs) {
        let twin = self.game.starts_with("von");
        let p2 = !twin && self.game != "rascot2";
        self.cabinet_common(out, 0x10, true, p2);
        if twin {
            for (port, x, y, shot, dash) in [
                (
                    &mut out.in1,
                    S::TwinLeftX,
                    S::TwinLeftY,
                    S::TwinLeftShot,
                    S::TwinLeftDash,
                ),
                (
                    &mut out.in2,
                    S::TwinRightX,
                    S::TwinRightY,
                    S::TwinRightShot,
                    S::TwinRightDash,
                ),
            ] {
                for (bit, on) in [
                    (0x80, self.signal(x) < -0.5),
                    (0x40, self.signal(x) > 0.5),
                    (0x20, self.signal(y) > 0.5),
                    (0x10, self.signal(y) < -0.5),
                    (1, self.signal(shot) > 0.5),
                    (2, self.signal(dash) > 0.5),
                ] {
                    if on {
                        *port &= !bit;
                    }
                }
            }
        } else {
            for (bit, s) in self.joystick_buttons().iter().copied().chain([
                (0x10, S::Down),
                (0x20, S::Up),
                (0x40, S::Right),
                (0x80, S::Left),
            ]) {
                if self.signal(s) > 0.0 {
                    out.in1 &= !bit;
                }
                if p2 && self.signal_p2(s) > 0.5 {
                    out.in2 &= !bit;
                }
            }
        }
        self.scatter(&Axes::default(), out);
    }
    pub(super) fn poll_native_racing(&mut self, out: &mut Inputs) {
        let pad = self.has_analog();
        let x = if pad {
            self.sample_signal(S::Steering, false, true)
        } else {
            0.0
        };
        let pad_steer = pad.then_some(if x.abs() > STICK_DEADZONE {
            x.signum() * (x.abs() - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)
        } else {
            0.0
        });
        let pad_accel = if pad {
            self.sample_signal(S::Accelerator, false, true)
        } else {
            0.0
        };
        let pad_brake = if pad {
            self.sample_signal(S::Brake, false, true)
        } else {
            0.0
        };
        let key_steer = self.sample_signal(S::Steering, true, false);
        let k_left = key_steer < 0.0;
        let k_right = key_steer > 0.0;
        let k_accel = self.sample_signal(S::Accelerator, true, false) > 0.0;
        let k_brake = self.sample_signal(S::Brake, true, false) > 0.0;
        let sequential = self.game != "desert" && !self.game.starts_with("motoraid");
        let want_up = sequential && self.signal(S::GearUp) > 0.0;
        let want_down = sequential && self.signal(S::GearDown) > 0.0;
        if want_up && !self.shift_up_held {
            self.shift(true);
        }
        if want_down && !self.shift_down_held {
            self.shift(false);
        }
        self.shift_up_held = want_up;
        self.shift_down_held = want_down;
        self.apply_direct_gear();
        let start40 = self.scheme == ControlScheme::Bike
            || self.game.starts_with("srally")
            || self.game.starts_with("indy500")
            || self.game.starts_with("stcc")
            || self.game.starts_with("overrev")
            || self.game == "sgt24h";
        self.cabinet_common(out, if start40 { 0x40 } else { 0x10 }, true, false);
        let buttons: &[(usize, u8, S, f32)];
        if self.h_gate() {
            out.set_gear(self.gear);
            if self.game.starts_with("srally") {
                buttons = &[(0, 0x20, S::View1, 0.5)];
                out.in2 = (255.0 * self.signal(S::Handbrake).clamp(0.0, 1.0)).round() as u8;
            } else {
                buttons = &[
                    (0, 0x20, S::View1, 0.0),
                    (0, 0x40, S::View2, 0.0),
                    (0, 0x80, S::View3, 0.0),
                    (1, 1, S::View4, 0.0),
                ];
            }
        } else if matches!(self.game.as_str(), "vr" | "vformula") {
            buttons = &[
                (0, 0x20, S::View1, 0.0),
                (0, 0x40, S::View2, 0.0),
                (0, 0x80, S::View3, 0.0),
                (1, 1, S::View4, 0.0),
            ];
            if want_up && !want_down {
                out.in1 &= !0x20;
            }
            if want_down && !want_up {
                out.in1 &= !0x10;
            }
        } else if self.game == "desert" {
            let shift = self.signal(S::DesertShift) > 0.5;
            if shift && !self.special_shift_held {
                self.special_shift = !self.special_shift;
            }
            self.special_shift_held = shift;
            if self.special_shift {
                out.in1 &= !1;
            }
            buttons = &[
                (0, 0x20, S::View1, 0.0),
                (0, 0x40, S::View2, 0.0),
                (0, 0x80, S::View4, 0.5),
                (1, 0x10, S::DesertGun, 0.5),
                (1, 0x20, S::DesertCannon, 0.5),
            ];
        } else {
            let independent = self.game.starts_with("motoraid");
            let up = self.signal(if independent { S::Action2 } else { S::GearUp }) > 0.5;
            let down = self.signal(if independent { S::Action1 } else { S::GearDown }) > 0.5;
            if up && (independent || !down) {
                out.in1 &= !0x10;
            }
            if down && (independent || !up) {
                out.in1 &= !0x20;
            }
            if self.game.starts_with("overrev")
                || self.game.starts_with("indy500")
                || self.game.starts_with("stcc")
            {
                buttons = &[(1, 1, S::View4, 0.5), (1, 2, S::View1, 0.5)];
            } else if self.game == "sgt24h" {
                buttons = &[(1, 1, S::View1, 0.5)];
            } else if self.game.starts_with("manxtt") {
                buttons = &[(0, 0x40, S::View1, 0.5)];
            } else {
                buttons = &[];
            }
        }
        self.cabinet_buttons(out, buttons);
        // --- analog ----------------------------------------------------------
        let half = (ANALOG_MAX - STEER_CENTRE) as f32;
        match pad_steer {
            // A stick has a position of its own, so it drives the wheel
            // directly; only fall back to key travel when it is centred.
            Some(s) if s.abs() > 0.0 => {
                self.steer = (STEER_CENTRE as f32 + s * half).round() as i32;
            }
            _ => {
                let target = if k_left {
                    ANALOG_MIN
                } else if k_right {
                    ANALOG_MAX
                } else {
                    STEER_CENTRE
                };
                self.steer = Self::approach(self.steer, target, STEER_KEYDELTA);
            }
        }

        let pedal = |cur: i32, pad: f32, key: bool| -> i32 {
            if pad > TRIGGER_DEADZONE {
                (ANALOG_MIN as f32 + pad * (ANALOG_MAX - ANALOG_MIN) as f32).round() as i32
            } else {
                let target = if key { ANALOG_MAX } else { ANALOG_MIN };
                Self::approach(cur, target, PEDAL_KEYDELTA)
            }
        };
        self.accel = pedal(self.accel, pad_accel, k_accel);
        self.brake = pedal(self.brake, pad_brake, k_brake);

        self.steer = self.steer.clamp(ANALOG_MIN, ANALOG_MAX);
        self.accel = self.accel.clamp(ANALOG_MIN, ANALOG_MAX);
        self.brake = self.brake.clamp(ANALOG_MIN, ANALOG_MAX);

        let axes = Axes {
            steer: self.steer as u8,
            accel: self.accel as u8,
            brake: self.brake as u8,
            // A bike's throttle grip is the same pedal input on another channel.
            throttle: self.accel as u8,
            ..Default::default()
        };
        self.scatter(&axes, out);
    }

    pub(super) fn poll_native_gun(&mut self, out: &mut Inputs) {
        self.cabinet_common(out, 0x10, true, true);
        let mut in1: u8 = 0xff;

        let mut fire = self.mouse_fire;
        let mut reload = self.mouse_reload;

        // SM2's Analog Stick gun mode moves a persistent cursor. Releasing
        // the stick must not jump back to the last mouse position/centre.
        let step = |value: f32| {
            const DEADZONE: f32 = 6000.0 / 32767.0;
            if value.abs() <= DEADZONE {
                return 0.0;
            }
            value.signum() * (1.0 + ((value.abs() - DEADZONE) / (1.0 - DEADZONE) * 11.0).round())
        };
        self.cursor.0 = (self.cursor.0 + step(self.signal(Signal::GunYaw)) / 495.0).clamp(0.0, 1.0);
        self.cursor.1 =
            (self.cursor.1 - step(self.signal(Signal::GunPitch)) / 383.0).clamp(0.0, 1.0);
        self.cursor_p2_active |= step(self.signal_p2(Signal::GunYaw)) != 0.0
            || step(self.signal_p2(Signal::GunPitch)) != 0.0
            || self.signal_p2(Signal::PrimaryFire) > 0.5
            || self.signal_p2(Signal::SecondaryFire) > 0.5;
        self.cursor_p2.0 =
            (self.cursor_p2.0 + step(self.signal_p2(Signal::GunYaw)) / 495.0).clamp(0.0, 1.0);
        self.cursor_p2.1 =
            (self.cursor_p2.1 - step(self.signal_p2(Signal::GunPitch)) / 383.0).clamp(0.0, 1.0);
        let (mut nx, mut ny) = self.cursor;

        fire |= self.signal(S::PrimaryFire) > if self.game == "bel" { 0.5 } else { 0.0 };
        reload |= self.signal(S::SecondaryFire) > 0.0;
        // Reloading is done by shooting off-screen: report the gun off-screen
        // and pull the trigger, which is exactly what the cabinet's gun does.
        reload &= self.serial_gun();
        if reload {
            fire = true;
            nx = 0.0;
            ny = 0.0;
        }
        if fire {
            in1 &= !IN1_VCOP_TRIGGER;
        }

        let lerp = |t: f32, lo: i32, hi: i32| (lo as f32 + t * (hi - lo) as f32).round() as u16;
        out.in1 = in1;
        out.in2 = 0xff;
        let (xmin, xmax, ymin, ymax) = if self.game.starts_with("hotd") {
            (173, 596, 87, 380)
        } else if self.game == "vcop2" {
            (137, 630, 36, 425)
        } else {
            (GUN_X_MIN, GUN_X_MAX, GUN_Y_MIN, GUN_Y_MAX)
        };
        out.gun_x = lerp(nx, xmin, xmax);
        out.gun_y = lerp(ny, ymin, ymax);
        out.gun_offscreen = reload;
        let reload2 = self.serial_gun() && self.signal_p2(Signal::SecondaryFire) > 0.5;
        let fire2 = reload2 || self.signal_p2(Signal::PrimaryFire) > 0.5;
        let (nx2, ny2) = if reload2 { (0.0, 0.0) } else { self.cursor_p2 };
        // SM2 games.xml: hotd uses IN2:01. hotdo/hotdp explicitly declare
        // their own lightgun spec (no trigger override), hence use IN1:02.
        if fire2 {
            if self.game == "hotd" {
                out.in2 &= !1;
            } else {
                out.in1 &= !2;
            }
        }
        let (xmin2, xmax2, ymin2, ymax2) = if self.game.starts_with("hotd") {
            (0x0a3, 0x254, 0x057, 0x17c)
        } else if self.game == "vcop2" {
            (0x086, 0x273, 0x024, 0x1a9)
        } else {
            (0x080, 0x273, 0x027, 0x1a9)
        };
        out.gun2_x = lerp(nx2, xmin2, xmax2);
        out.gun2_y = lerp(ny2, ymin2, ymax2);
        out.gun2_offscreen = reload2;
        // The mounted-gun cabinets (Gunblade, Rail Chase 2, Behind Enemy Lines)
        // read aim straight off the ADC instead of the gun interface board, so
        // publish both independent cursors there as 8-bit values.
        let byte = |t: f32| (t.clamp(0.0, 1.0) * 255.0) as u8;
        let axes = Axes {
            gun1x: byte(nx),
            gun1y: byte(ny),
            gun2x: byte(nx2),
            gun2y: byte(ny2),
            ..Default::default()
        };
        self.scatter(&axes, out);
        if self.game == "bel" {
            if self.signal(S::SecondaryFire) > 0.5 || self.mouse_reload {
                out.in1 &= !0x10;
            }
            if self.signal_p2(S::SecondaryFire) > 0.5 {
                out.in1 &= !0x20;
            }
        }
    }
}
