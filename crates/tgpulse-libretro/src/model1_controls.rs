//! Libretro cabinet actions converted to the imported Model 1 machine I/O.
//! RetroPad sampling stays in the adapter; the standalone input path is untouched.

use tgpulse_core::config::Inputs;

/// Virtua Racing and Virtua Formula use the original Model 1 driving ports.
#[derive(Clone, Copy, Debug, Default)]
pub struct VrCabinet {
    pub coin1: bool,
    pub coin2: bool,
    pub test: bool,
    pub service: bool,
    pub start: bool,
    pub views: [bool; 4],
    pub gear_up: bool,
    pub gear_down: bool,
    pub steer: u8,
    pub accel: u8,
    pub brake: u8,
}

impl VrCabinet {
    pub fn into_native(self) -> Inputs {
        let mut out = Inputs {
            in0: 0xff,
            in1: 0xff,
            in2: 0xff,
            ..Inputs::default()
        };
        for (bit, active) in [
            (0x01, self.coin1),
            (0x02, self.coin2),
            (0x04, self.test),
            (0x08, self.service),
            (0x10, self.start),
            (0x20, self.views[0]),
            (0x40, self.views[1]),
            (0x80, self.views[2]),
        ] {
            if active {
                out.in0 &= !bit;
            }
        }
        if self.views[3] {
            out.in1 &= !0x01;
        }
        if self.gear_up && !self.gear_down {
            out.in1 &= !0x20;
        }
        if self.gear_down && !self.gear_up {
            out.in1 &= !0x10;
        }
        out.steer = self.steer;
        out.accel = self.accel;
        out.brake = self.brake;
        out.analog = [0xff; 8];
        out.analog[..3].copy_from_slice(&[self.steer, self.accel, self.brake]);
        out
    }
}

/// Virtua Fighter's two independent digital panels. Actions are Kick, Punch,
/// Guard in that order; the host chooses physical button assignments.
#[derive(Clone, Copy, Debug, Default)]
pub struct VfPlayer {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub actions: [bool; 3],
    pub coin: bool,
    pub start: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VfCabinet {
    pub players: [VfPlayer; 2],
    pub test: bool,
    pub service: bool,
}

impl VfCabinet {
    pub fn into_native(self) -> Inputs {
        let mut out = released();
        // The standalone joystick sampler leaves unused mirrored pedal axes
        // at zero; keep its established cabinet trace unchanged.
        out.accel = 0;
        out.brake = 0;
        common(
            &mut out,
            self.players[0].coin,
            self.players[1].coin,
            self.test,
            self.service,
            self.players[0].start,
            self.players[1].start,
            true,
        );
        for (player, port) in self.players.iter().zip([&mut out.in1, &mut out.in2]) {
            for (bit, active) in [
                (0x02, player.actions[0]),
                (0x01, player.actions[1]),
                (0x04, player.actions[2]),
                (0x10, player.down),
                (0x20, player.up),
                (0x40, player.right),
                (0x80, player.left),
            ] {
                clear(port, bit, active);
            }
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightKind {
    WingWar,
    WingWar360,
    StarWars,
    NetMerc,
}

/// Signed axes range from -1 to 1. Positive X points right and positive Y
/// points up as seen by the player. Throttle Up raises the cabinet ADC.
#[derive(Clone, Copy, Debug)]
pub struct FlightCabinet {
    pub kind: FlightKind,
    pub coin1: bool,
    pub coin2: bool,
    pub test: bool,
    pub service: bool,
    pub start: bool,
    pub weapons: [bool; 3],
    pub views: [bool; 4],
    pub pilot: [f32; 2],
    pub throttle: f32,
    pub gunner: [f32; 2],
    pub gunner_weapons: [bool; 2],
}

impl FlightCabinet {
    pub fn into_native(self) -> Inputs {
        let mut out = released();
        let wing = matches!(self.kind, FlightKind::WingWar | FlightKind::WingWar360);
        let swa = self.kind == FlightKind::StarWars;
        common(
            &mut out,
            self.coin1,
            self.coin2,
            self.test,
            self.service,
            self.start && self.kind != FlightKind::NetMerc,
            false,
            self.kind == FlightKind::WingWar360 || swa,
        );
        for (bit, active) in if wing {
            [
                (0x10, self.weapons[0]),
                (0x20, self.weapons[1]),
                (0x40, self.weapons[2]),
            ]
        } else {
            [
                (0x01, self.weapons[0]),
                (0x02, self.weapons[1]),
                (0x04, self.weapons[2] && self.kind == FlightKind::NetMerc),
            ]
        } {
            clear(&mut out.in1, bit, active);
        }
        if self.kind == FlightKind::WingWar {
            for (bit, active) in [
                (0x20, self.views[0]),
                (0x40, self.views[1]),
                (0x80, self.views[2]),
            ] {
                clear(&mut out.in0, bit, active);
            }
            clear(&mut out.in1, 0x01, self.views[3]);
        } else if swa {
            clear(&mut out.in1, 0x10, self.views[0]);
            clear(&mut out.in1, 0x04, self.gunner_weapons[0]);
            clear(&mut out.in1, 0x08, self.gunner_weapons[1]);
        }
        let (rest, lo, hi) = if swa {
            (127, 27, 227)
        } else if self.kind == FlightKind::NetMerc {
            (127, 0, 255)
        } else {
            (128, 0, 255)
        };
        let polarity = if matches!(self.kind, FlightKind::WingWar360 | FlightKind::NetMerc) {
            1.0
        } else {
            -1.0
        };
        out.analog = [0xff; 8];
        out.analog[0] = centered(self.pilot[0] * polarity, rest, lo, hi);
        out.analog[1] = if self.kind == FlightKind::NetMerc {
            0xff
        } else {
            centered(self.pilot[1] * polarity, rest, lo, hi)
        };
        match self.kind {
            FlightKind::NetMerc => out.analog[2] = centered(-self.pilot[1], 127, 0, 255),
            FlightKind::WingWar | FlightKind::WingWar360 => {
                out.analog[2] = centered(self.throttle, 128, 1, 255);
            }
            FlightKind::StarWars => {
                out.analog[2] = centered(self.throttle, 128, 28, 228);
                out.analog[4] = centered(-self.gunner[0], 127, 27, 227);
                out.analog[5] = centered(-self.gunner[1], 127, 27, 227);
            }
        }
        [out.steer, out.accel, out.brake] = out.analog[..3].try_into().unwrap();
        out
    }
}

fn released() -> Inputs {
    Inputs {
        in0: 0xff,
        in1: 0xff,
        in2: 0xff,
        analog: [0xff; 8],
        ..Inputs::default()
    }
}

fn clear(port: &mut u8, bit: u8, active: bool) {
    if active {
        *port &= !bit;
    }
}

fn common(
    out: &mut Inputs,
    coin1: bool,
    coin2: bool,
    test: bool,
    service: bool,
    start: bool,
    start2: bool,
    allow_coin2: bool,
) {
    for (bit, active) in [
        (1, coin1),
        (2, allow_coin2 && coin2),
        (4, test),
        (8, service),
        (0x10, start),
        (0x20, start2),
    ] {
        clear(&mut out.in0, bit, active);
    }
}

fn centered(value: f32, rest: u8, min: u8, max: u8) -> u8 {
    let value = value.clamp(-1.0, 1.0);
    let span = if value < 0.0 { rest - min } else { max - rest };
    (rest as f32 + value * span as f32).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vr_actions_drive_original_ports_and_adc_mirrors() {
        let input = VrCabinet {
            coin1: true,
            test: true,
            start: true,
            views: [true, false, false, true],
            gear_up: true,
            steer: 0x36,
            accel: 0xa4,
            brake: 0x20,
            ..VrCabinet::default()
        }
        .into_native();
        assert_eq!(input.in0, 0xff & !(0x01 | 0x04 | 0x10 | 0x20));
        assert_eq!(input.in1, 0xff & !(0x01 | 0x20));
        assert_eq!(input.analog[..3], [0x36, 0xa4, 0x20]);
        assert_eq!((input.steer, input.accel, input.brake), (0x36, 0xa4, 0x20));
    }

    #[test]
    fn opposing_gear_buttons_do_not_select_a_direction() {
        let input = VrCabinet {
            gear_up: true,
            gear_down: true,
            ..VrCabinet::default()
        }
        .into_native();
        assert_eq!(input.in1, 0xff);
    }

    #[test]
    fn vf_player_two_has_independent_start_directions_and_actions() {
        let mut cabinet = VfCabinet::default();
        cabinet.players[1] = VfPlayer {
            coin: true,
            start: true,
            left: true,
            actions: [true, false, true],
            ..VfPlayer::default()
        };
        let input = cabinet.into_native();
        assert_eq!(input.in0, 0xff & !(0x02 | 0x20));
        assert_eq!(input.in1, 0xff);
        assert_eq!(input.in2, 0xff & !(0x80 | 0x02 | 0x04));
    }

    fn idle_flight(kind: FlightKind) -> FlightCabinet {
        FlightCabinet {
            kind,
            coin1: false,
            coin2: false,
            test: false,
            service: false,
            start: false,
            weapons: [false; 3],
            views: [false; 4],
            pilot: [0.0; 2],
            throttle: 0.0,
            gunner: [0.0; 2],
            gunner_weapons: [false; 2],
        }
    }

    #[test]
    fn swa_gunner_uses_own_fire_and_adc_without_start_or_throttle() {
        let mut cabinet = idle_flight(FlightKind::StarWars);
        cabinet.coin2 = true;
        cabinet.gunner = [1.0, -1.0];
        cabinet.gunner_weapons = [true, true];
        let input = cabinet.into_native();
        assert_eq!(input.in0, 0xff & !0x02);
        assert_eq!(input.in1, 0xff & !(0x04 | 0x08));
        assert_eq!(input.analog[..6], [127, 127, 128, 255, 27, 227]);
    }

    #[test]
    fn wingwar_r360_and_netmerc_keep_distinct_wiring() {
        let mut wing = idle_flight(FlightKind::WingWar);
        wing.pilot = [1.0, 1.0];
        wing.throttle = 1.0;
        wing.views = [true, false, false, true];
        wing.weapons = [true, true, true];
        let normal = wing.into_native();
        assert_eq!(normal.in0, 0xff & !0x20);
        assert_eq!(normal.in1, 0xff & !(0x10 | 0x20 | 0x40 | 0x01));
        assert_eq!(normal.analog[..3], [0, 0, 255]);
        wing.kind = FlightKind::WingWar360;
        let r360 = wing.into_native();
        assert_eq!(r360.in0, 0xff);
        assert_eq!(r360.in1, 0xff & !(0x10 | 0x20 | 0x40));
        assert_eq!(r360.analog[..3], [255, 255, 255]);
        let mut netmerc = idle_flight(FlightKind::NetMerc);
        netmerc.start = true;
        netmerc.pilot[1] = 1.0;
        netmerc.weapons[2] = true;
        let net = netmerc.into_native();
        assert_eq!(net.in0, 0xff);
        assert_eq!(net.in1, 0xff & !0x04);
        assert_eq!(net.analog[..3], [127, 255, 0]);
    }
}
