//! Independent expectations from the SM2 profile workbook and MAME port maps.
//! See docs/INPUT_AUDIT.md for sources, intentional layout differences and limits.
use super::Signal as S;
use crate::bindings::Bindings;
use crate::input::{AnalogRole as A, ControlScheme as Scheme, InputState};
use tgpulse_core::config::Inputs;
use winit::keyboard::KeyCode;

struct Cabinet {
    sets: &'static str,
    scheme: Scheme,
    idle: [u8; 3],
    start: u8,
    start2: bool,
    coin2: bool,
    buttons: &'static [(S, usize, u8)],
}

fn cabinets() -> Vec<Cabinet> {
    use Scheme::*;
    use S::*;
    let mut cases = Vec::new();
    let mut add = |sets, scheme, idle, start, start2, coin2, buttons| {
        cases.push(Cabinet {
            sets,
            scheme,
            idle,
            start,
            start2,
            coin2,
            buttons,
        });
    };
    add(
        "daytona daytona93 daytonagtx daytonam daytonas daytonase daytonat daytonata",
        Racing,
        [255, 143, 255],
        0x10,
        false,
        true,
        &[
            (View1, 0, 0x20),
            (View2, 0, 0x40),
            (View3, 0, 0x80),
            (View4, 1, 1),
            (GearUp, 1, 0x20),
            (Gear1, 1, 0x20),
            (Gear2, 1, 0x10),
            (Gear3, 1, 0x60),
            (Gear4, 1, 0x50),
        ],
    );
    add(
        "vr vformula",
        Racing,
        [255; 3],
        0x10,
        false,
        true,
        &[
            (View1, 0, 0x20),
            (View2, 0, 0x40),
            (View3, 0, 0x80),
            (View4, 1, 1),
            (GearUp, 1, 0x20),
            (GearDown, 1, 0x10),
        ],
    );
    add(
        "srallyc srallycb srallycc srallycdx srallycdxa",
        Racing,
        [255, 143, 0],
        0x40,
        false,
        true,
        &[
            (View1, 0, 0x20),
            (Handbrake, 2, 255),
            (GearUp, 1, 0x20),
            (Gear1, 1, 0x20),
            (Gear2, 1, 0x10),
            (Gear3, 1, 0x60),
            (Gear4, 1, 0x50),
        ],
    );
    add(
        "indy500 indy500d indy500to stcc stcca stccb stcco overrev overrevb overrevba",
        Racing,
        [255; 3],
        0x40,
        false,
        true,
        &[
            (View4, 1, 1),
            (View1, 1, 2),
            (GearUp, 1, 0x10),
            (GearDown, 1, 0x20),
        ],
    );
    add(
        "sgt24h",
        Racing,
        [255; 3],
        0x40,
        false,
        true,
        &[(View1, 1, 1), (GearUp, 1, 0x10), (GearDown, 1, 0x20)],
    );
    add(
        "manxtt manxttc manxttdx",
        Bike,
        [255; 3],
        0x40,
        false,
        true,
        &[(GearUp, 1, 0x10), (GearDown, 1, 0x20), (View1, 0, 0x40)],
    );
    add(
        "motoraid motoraiddx",
        Bike,
        [255; 3],
        0x40,
        false,
        true,
        &[(Action1, 1, 0x20), (Action2, 1, 0x10)],
    );
    add(
        "desert",
        Racing,
        [255; 3],
        0x10,
        false,
        true,
        &[
            (View1, 0, 0x20),
            (View2, 0, 0x40),
            (View4, 0, 0x80),
            (DesertGun, 1, 0x10),
            (DesertCannon, 1, 0x20),
            (DesertShift, 1, 1),
        ],
    );
    add(
        "doa doaa doaab doaae doab",
        Joystick,
        [255; 3],
        0x10,
        true,
        true,
        &[(Action1, 1, 4), (Action2, 1, 2), (Action3, 1, 1)],
    );
    add(
        "vf vf2 vf2a vf2b vf2o",
        Joystick,
        [255; 3],
        0x10,
        true,
        true,
        &[(Action1, 1, 2), (Action2, 1, 1), (Action3, 1, 4)],
    );
    add("fvipers fvipersa fvipersb lastbrnx lastbrnxj lastbrnxu dynamcop dynamcopb dynamcopc dyndeka2 dyndeka2b schamp sfight",
        Joystick, [255;3], 0x10, true, true,
        &[(Action1,1,2),(Action2,1,1),(Action3,1,4)]);
    add(
        "hpyagu98 airwlkrs",
        Joystick,
        [255; 3],
        0x10,
        true,
        true,
        &[(Action1, 1, 1), (Action2, 1, 2), (Action3, 1, 4)],
    );
    add(
        "rascot2",
        Joystick,
        [255; 3],
        0x10,
        false,
        true,
        &[(Action1, 1, 1), (Action2, 1, 2), (Action3, 1, 4)],
    );
    add(
        "vstriker vstrikero",
        Joystick,
        [255; 3],
        0x10,
        true,
        true,
        &[
            (StrikerShortPass, 1, 4),
            (StrikerLongPass, 1, 1),
            (StrikerShoot, 1, 2),
        ],
    );
    add(
        "dynabb dynabb97",
        Joystick,
        [255; 3],
        0x10,
        true,
        true,
        &[(Action1, 1, 1), (Action2, 1, 2)],
    );
    add(
        "zerogun zeroguna zerogunaj zerogunj pltkids pltkidsa",
        Joystick,
        [255; 3],
        0x10,
        true,
        true,
        &[(Action1, 1, 1), (Action2, 1, 2)],
    );
    add(
        "von vonj vonr vonu",
        Joystick,
        [255; 3],
        0x10,
        false,
        true,
        &[
            (TwinLeftShot, 1, 1),
            (TwinRightShot, 2, 1),
            (TwinLeftDash, 1, 2),
            (TwinRightDash, 2, 2),
            (TwinLeftX, 1, 0x40),
            (TwinLeftY, 1, 0x20),
            (TwinRightX, 2, 0x40),
            (TwinRightY, 2, 0x20),
        ],
    );
    add(
        "vcop vcopa vcop2 hotd hotdo hotdp",
        Gun,
        [255; 3],
        0x10,
        true,
        true,
        &[(PrimaryFire, 1, 1), (SecondaryFire, 1, 1)],
    );
    add(
        "gunblade rchase2 rchase2a",
        Gun,
        [255; 3],
        0x10,
        true,
        true,
        &[(PrimaryFire, 1, 1)],
    );
    add(
        "bel",
        Gun,
        [255; 3],
        0x10,
        true,
        true,
        &[(PrimaryFire, 1, 1), (SecondaryFire, 1, 0x10)],
    );
    add(
        "skytargt",
        Flight,
        [255; 3],
        0x40,
        false,
        true,
        &[
            (SkyMachineGun, 1, 0x10),
            (SkyMissile, 1, 0x20),
            (View4, 0, 0x20),
        ],
    );
    add(
        "swa swaj",
        Flight,
        [255; 3],
        0x10,
        false, // Gunner has no Start: user cabinet convention, unlike MAME's generic Start2 bit.
        true,
        &[(SwaLaser, 1, 1), (SwaTorpedo, 1, 2), (View1, 1, 0x10)],
    );
    add(
        "wingwar wingwarj wingwaru",
        Flight,
        [255; 3],
        0x10,
        false,
        false,
        &[
            (WingMachineGun, 1, 0x10),
            (WingMissile, 1, 0x20),
            (WingSmoke, 1, 0x40),
            (View1, 0, 0x20),
            (View2, 0, 0x40),
            (View3, 0, 0x80),
            (View4, 1, 1),
        ],
    );
    add(
        "wingwar360",
        Flight,
        [255; 3],
        0x10,
        false,
        true,
        &[
            (WingMachineGun, 1, 0x10),
            (WingMissile, 1, 0x20),
            (WingSmoke, 1, 0x40),
        ],
    );
    add(
        "netmerc",
        Flight,
        [255; 3],
        0,
        false,
        false,
        &[
            (NetmercButton1, 1, 1),
            (NetmercButton2, 1, 2),
            (NetmercMvdHolder, 1, 4),
        ],
    );
    add(
        "segawski",
        Ski,
        [255; 3],
        0x40,
        false,
        false,
        &[
            (View4, 1, 2),
            (View1, 0, 0x40),
            (WaterPitchRight, 1, 8),
            (WaterPitchLeft, 1, 4),
            (WaterSet, 1, 1),
        ],
    );
    add(
        "skisuprg",
        Ski,
        [255, 255, 0],
        0x10,
        false,
        true,
        &[
            (View4, 0, 0x20),
            (View1, 1, 1),
            (SkiFootRight, 2, 0x0f),
            (SkiFootLeft, 2, 0xf0),
            (SkiSelect1, 0, 0x40),
            (SkiSelect2, 0, 0x80),
            (SkiSelect3, 0, 0x10),
        ],
    );
    add(
        "topskatr topskatrj topskatru topskatruo",
        Skate,
        [255; 3],
        0x40,
        false,
        true,
        &[
            (View2, 0, 0x80),
            (View3, 0, 0x10),
            (SkaterJumpFront, 0, 0x20),
            (SkaterJumpTail, 1, 1),
        ],
    );
    add(
        "waverunr",
        Jetski,
        [255, 255, 0xf7],
        0x40,
        false,
        false,
        &[(View4, 1, 1)],
    );
    add(
        "powsled powsledm powsledr",
        Sled,
        [255; 3],
        0x10,
        true,
        true,
        &[(SledEntry, 1, 1), (SledCall, 1, 2), (Action4, 0, 0x80)],
    );
    cases
}

fn state(game: &str, scheme: Scheme) -> InputState {
    let mut input = InputState::with_gilrs(None);
    input.set_scheme(scheme);
    input.set_game(game);
    input
}

#[test]
fn split_game_actions_do_not_follow_shared_action_rebinds() {
    for (game, scheme, dedicated, port, mask) in [
        ("vstriker", Scheme::Joystick, S::StrikerShortPass, 1, 4),
        ("wingwar", Scheme::Flight, S::WingMachineGun, 1, 0x10),
        ("swa", Scheme::Flight, S::SwaLaser, 1, 1),
        ("topskatr", Scheme::Skate, S::SkaterJumpTail, 1, 1),
        ("powsled", Scheme::Sled, S::SledEntry, 1, 1),
        ("skisuprg", Scheme::Ski, S::SkiSelect1, 0, 0x40),
        ("desert", Scheme::Racing, S::DesertShift, 1, 1),
        ("netmerc", Scheme::Flight, S::NetmercButton1, 1, 1),
    ] {
        let mut input = state(game, scheme);
        for shared in [S::Action1, S::Action2, S::Action3] {
            input.bindings.set_expression(shared, "F11").unwrap();
        }
        input.bindings.set_expression(dedicated, "F12").unwrap();
        let mut out = Inputs::default();
        input.poll(&mut out);
        let idle = ports(&out);
        input.on_key(KeyCode::F11, true);
        input.poll(&mut out);
        assert_eq!(ports(&out), idle, "{game}: shared Action leaked");
        input.on_key(KeyCode::F11, false);
        input.on_key(KeyCode::F12, true);
        input.poll(&mut out);
        let mut expected = idle;
        expected[port] &= !mask;
        assert_eq!(ports(&out), expected, "{game}: dedicated action");
    }
}

// Frozen against main 1fedad9 before removing the compatibility route.
// This detects behavioral drift; the independent port tests above/below remain
// the wiring authority. Never regenerate these fingerprints to hide a mismatch.
#[test]
fn cabinet_trace_equivalence() {
    use crate::input::players::Player;
    use gilrs::Button as B;
    fn sample(input: &mut InputState, out: &mut Inputs, hash: &mut u64) {
        // The preferred Test/Service defaults match the historical trace.
        // Test-only adapter: replay the old logical shift requests through
        // their new channels. New defaults/independence have separate tests.
        for (gear, old) in [(S::GearDown, S::Action1), (S::GearUp, S::Action2)] {
            let text = input.bindings.binding(old).text.clone();
            input.bindings.set_expression(gear, &text).unwrap();
        }
        input.poll(out);
        let mut baseline = *out;
        // Explicit user-approved semantic correction after the refactor:
        // VF/VF2 now use South=Kick, East=Punch, West=Guard. Preserve the
        // ORIGINAL fingerprints, undoing only this documented bit permutation
        // on a copy. Separate raw-port assertions below verify the new mapping.
        let shift = match input.game.as_str() {
            "vf" => Some(2), // old VF incorrectly used DOA's 1=Guard, 4=Kick
            "vf2" | "vf2a" | "vf2b" | "vf2o" => Some(1),
            _ => None,
        };
        if let Some(shift) = shift {
            for port in [&mut baseline.in1, &mut baseline.in2] {
                let mask = 1 | (1 << shift);
                *port = (*port & !mask) | ((*port & 1) << shift) | ((*port >> shift) & 1);
            }
        }
        let row = format!("{baseline:?}:{:?}:{:?}", input.aim(), input.aim_p2());
        for byte in row.bytes() {
            *hash = (*hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    for cabinet in cabinets() {
        // These families intentionally changed semantics to match the workbook.
        // Do not regenerate their old fingerprints or pretend equivalence.
        // The all-set raw-port and physical-button tests cover the new contract.
        if [
            "srallyc",
            "manxtt",
            "motoraid",
            "desert",
            "doa",
            "fvipers",
            "hpyagu98",
            "rascot2",
            "vstriker",
            "dynabb",
            "von",
            "segawski",
            "skisuprg",
            "topskatr",
            "vcop",
            "gunblade",
            "bel",
            "skytargt",
            "swa",
            "wingwar",
            "wingwar360",
            "netmerc",
            "powsled",
        ]
        .contains(&cabinet.sets.split_whitespace().next().unwrap())
        {
            continue;
        }
        for game in cabinet.sets.split_whitespace() {
            let mut hash = 0xcbf29ce484222325;
            let mut input = state(game, cabinet.scheme);
            input.set_analog_roles(db_roles(game));
            let mut out = Inputs::default();
            for _ in 0..8 {
                sample(&mut input, &mut out, &mut hash);
            }
            for key in [
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                KeyCode::KeyG,
                KeyCode::KeyT,
                KeyCode::KeyJ,
                KeyCode::KeyK,
                KeyCode::KeyL,
                KeyCode::KeyI,
                KeyCode::KeyQ,
                KeyCode::KeyE,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::KeyZ,
                KeyCode::KeyX,
                KeyCode::KeyC,
                KeyCode::KeyV,
                KeyCode::Digit0,
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
                KeyCode::Digit5,
                KeyCode::Digit6,
                KeyCode::F2,
                KeyCode::F8,
            ] {
                input.on_key(key, true);
                for _ in 0..24 {
                    sample(&mut input, &mut out, &mut hash);
                }
                input.on_key(key, false);
                for _ in 0..24 {
                    sample(&mut input, &mut out, &mut hash);
                }
            }
            for player in Player::ALL {
                for button in [
                    B::South,
                    B::East,
                    B::West,
                    B::North,
                    B::LeftTrigger,
                    B::RightTrigger,
                    B::DPadUp,
                    B::DPadDown,
                    B::DPadLeft,
                    B::DPadRight,
                    B::Select,
                    B::Start,
                    B::LeftThumb,
                    B::RightThumb,
                ] {
                    let pad = if player == Player::One {
                        &mut input.external
                    } else {
                        &mut input.external_p2
                    };
                    pad.present = true;
                    pad.buttons.insert(button);
                    for _ in 0..3 {
                        sample(&mut input, &mut out, &mut hash);
                    }
                    let pad = if player == Player::One {
                        &mut input.external
                    } else {
                        &mut input.external_p2
                    };
                    pad.buttons.clear();
                    for _ in 0..3 {
                        sample(&mut input, &mut out, &mut hash);
                    }
                }
            }
            // Every assignable signal, including H-gate and half-axes, driven
            // independently of its defaults on both seats, then held/released.
            // Replay the original catalogue order. GUI-only regrouping must
            // not alter the frozen trace stimulus or require new fingerprints.
            let mut legacy_signals: Vec<S> = S::ALL
                .iter()
                .copied()
                .take_while(|s| *s != S::GunYaw)
                .collect();
            let extra_position = legacy_signals
                .iter()
                .position(|s| *s == S::Action3)
                .unwrap()
                + 1;
            legacy_signals.insert(extra_position, S::Action4);
            legacy_signals.extend([
                S::Handle,
                S::GunYaw,
                S::GunPitch,
                S::Elevation,
                S::TwinLeftX,
                S::TwinLeftY,
                S::TwinRightX,
                S::TwinRightY,
                S::Pitch,
                S::Roll,
                S::WaterSlide,
                S::SkaterSlide,
                S::Curving,
                S::Swing,
                S::Inclining,
                S::BatSwing,
            ]);
            for player in Player::ALL {
                for signal in legacy_signals
                    .iter()
                    .copied()
                    .filter(|s| !matches!(s, S::GearDown | S::GearUp))
                    .filter(|s| player == Player::One || s.supports_p2())
                {
                    input.bindings = Bindings::default();
                    input
                        .bindings
                        .set_player_expression(
                            player,
                            signal,
                            if signal.signed() {
                                "pad:LeftStickX"
                            } else {
                                "pad:LeftStickX+"
                            },
                        )
                        .unwrap();
                    for amount in [-1.0, -0.51, -0.15, 0.0, 0.15, 0.17, 0.49, 0.51, 1.0, 0.0] {
                        let pad = if player == Player::One {
                            &mut input.external
                        } else {
                            &mut input.external_p2
                        };
                        pad.left_x = amount;
                        pad.present = true;
                        for _ in 0..3 {
                            sample(&mut input, &mut out, &mut hash);
                        }
                    }
                }
            }
            input.bindings = Bindings::default();
            input.on_key(KeyCode::KeyE, true);
            input.on_key(KeyCode::KeyQ, true);
            for _ in 0..10 {
                sample(&mut input, &mut out, &mut hash);
            }
            input.on_key(KeyCode::KeyE, false);
            input.on_key(KeyCode::KeyQ, false);
            for _ in 0..10 {
                sample(&mut input, &mut out, &mut hash);
            }
            let expected = include_str!("equivalence.txt")
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{game} ")))
                .expect("baseline for every set");
            assert_eq!(
                format!("{hash:016x}"),
                expected,
                "{game}: cabinet trace changed"
            );
        }
    }
}

#[test]
fn dedicated_shifts_are_rebindable_without_actions_on_every_driving_set() {
    use gilrs::Button as B;
    for cabinet in cabinets()
        .into_iter()
        .filter(|c| matches!(c.scheme, Scheme::Racing | Scheme::Bike))
    {
        for game in cabinet
            .sets
            .split_whitespace()
            .filter(|g| *g != "desert" && !g.starts_with("motoraid"))
        {
            let mut input = state(game, cabinet.scheme);
            input.bindings.set_expression(S::GearUp, "F12").unwrap();
            input.bindings.set_expression(S::GearDown, "F11").unwrap();
            let mut out = Inputs::default();
            input.poll(&mut out);
            let idle = ports(&out);
            // Old action defaults (including shoulders) no longer choose a gear
            // when the dedicated signal has been rebound elsewhere.
            for button in [B::South, B::East, B::LeftTrigger, B::RightTrigger] {
                input.set_pad_button(button, true);
                input.poll(&mut out);
                let mut expected = idle;
                if game.starts_with("srally") && button == B::South {
                    expected[2] = 255; // handbrake is independent of shifting
                }
                assert_eq!(ports(&out), expected, "{game} action {button:?}");
                input.set_pad_button(button, false);
            }
            for key in [
                KeyCode::KeyJ,
                KeyCode::KeyK,
                KeyCode::KeyE,
                KeyCode::KeyQ,
                KeyCode::Space,
            ] {
                input.on_key(key, true);
                input.poll(&mut out);
                assert_eq!(ports(&out), idle, "{game} action {key:?}");
                input.on_key(key, false);
            }
            let h_gate = game.starts_with("daytona") || game.starts_with("srally");
            let vr = matches!(game, "vr" | "vformula");
            for (key, mask) in [
                (KeyCode::F12, if h_gate || vr { 0x20 } else { 0x10 }),
                (
                    KeyCode::F11,
                    if h_gate {
                        0
                    } else if vr {
                        0x10
                    } else {
                        0x20
                    },
                ),
            ] {
                input.on_key(key, true);
                let mut expected = idle;
                expected[1] ^= mask;
                for _ in 0..3 {
                    input.poll(&mut out);
                    assert_eq!(ports(&out), expected, "{game} dedicated {key:?}");
                }
                input.on_key(key, false);
                input.poll(&mut out);
                assert_eq!(
                    ports(&out),
                    if h_gate { expected } else { idle },
                    "{game} release"
                );
            }
        }
    }
}

#[test]
fn virtua_fighter_actions_match_sm2_positions_on_both_seats() {
    use crate::input::players::Player;
    use gilrs::Button as B;
    // Sega VF manual: SW1 Punch, SW2 Kick, SW3 Guard. SM2's VF2 profile:
    // RetroPad B/South Kick (02), A/East Punch (01), Y/West Guard (04).
    for game in ["vf", "vf2", "vf2a", "vf2b", "vf2o"] {
        for player in Player::ALL {
            for (signal, buttons, keys, bit) in [
                (
                    S::Action1,
                    &[B::South, B::LeftTrigger][..],
                    &[KeyCode::KeyJ, KeyCode::KeyE, KeyCode::Space][..],
                    2,
                ),
                (
                    S::Action2,
                    &[B::East, B::RightTrigger][..],
                    &[KeyCode::KeyK, KeyCode::KeyQ, KeyCode::KeyR][..],
                    1,
                ),
                (S::Action3, &[B::West][..], &[KeyCode::KeyL][..], 4),
            ] {
                let mut input = state(game, Scheme::Joystick);
                let port = if player == Player::One { 1 } else { 2 };
                let mut expected = [255; 3];
                expected[port] &= !bit;
                let mut out = Inputs::default();
                // Each OR alternative must work alone, without driving the
                // other player or leaving an action held after release.
                for &button in buttons {
                    let pad = if player == Player::One {
                        &mut input.external
                    } else {
                        &mut input.external_p2
                    };
                    pad.present = true;
                    pad.buttons.insert(button);
                    input.poll(&mut out);
                    assert_eq!(ports(&out), expected, "{game} {player:?} {button:?}");
                    input.external.buttons.clear();
                    input.external_p2.buttons.clear();
                    input.poll(&mut out);
                    assert_eq!(ports(&out), [255; 3]);
                }
                let keys = if player == Player::One {
                    keys
                } else {
                    input
                        .bindings
                        .set_player_expression(player, signal, "F12")
                        .unwrap();
                    &[KeyCode::F12]
                };
                for &key in keys {
                    input.on_key(key, true);
                    input.poll(&mut out);
                    assert_eq!(ports(&out), expected, "{game} {player:?} {key:?}");
                    input.on_key(key, false);
                    input.poll(&mut out);
                    assert_eq!(ports(&out), [255; 3]);
                }
            }
        }
    }
}

/// Touch now produces the same native catalogue as other input adapters.
/// This is adapter-contract coverage, not a frozen pre-refactor touch trace.
#[test]
fn touch_native_signals_match_bound_values_and_remain_p1_only() {
    for (game, scheme, signal) in [
        ("vr", Scheme::Racing, S::Steering),
        ("srallyc", Scheme::Racing, S::Accelerator),
        ("srallyc", Scheme::Racing, S::GearUp),
        ("vr", Scheme::Racing, S::GearDown),
        ("manxtt", Scheme::Bike, S::Steering),
        ("vf", Scheme::Joystick, S::Action1),
        ("swa", Scheme::Flight, S::SkyX),
        ("swa", Scheme::Flight, S::SkyY),
        ("swa", Scheme::Flight, S::View1),
        ("wingwar", Scheme::Flight, S::ThrottleUp),
        ("wingwar", Scheme::Flight, S::ThrottleDown),
        ("vcop", Scheme::Gun, S::GunYaw),
        ("gunblade", Scheme::Gun, S::GunPitch),
        ("waverunr", Scheme::Jetski, S::Handle),
        ("topskatr", Scheme::Skate, S::Curving),
        ("segawski", Scheme::Ski, S::WaterSlide),
        ("skisuprg", Scheme::Ski, S::Swing),
        ("skisuprg", Scheme::Ski, S::Inclining),
        ("powsled", Scheme::Sled, S::Accelerator),
    ] {
        let mut pad = state(game, scheme);
        let mut touch = state(game, scheme);
        for input in [&mut pad, &mut touch] {
            input.set_analog_roles(db_roles(game));
            for &s in S::ALL {
                input.bindings.set_expression(s, "").unwrap();
            }
        }
        pad.bindings
            .set_expression(
                signal,
                if signal.signed() {
                    "pad:LeftStickX"
                } else {
                    "pad:LeftStickX+"
                },
            )
            .unwrap();
        for amount in [-1.0_f32, -0.5, 0.0, 0.5, 1.0, 0.0] {
            let value = if signal.signed() {
                amount
            } else {
                amount.max(0.0)
            };
            pad.set_pad_stick(value, 0.0);
            touch.set_touch(&[(signal, value)]);
            for _ in 0..4 {
                let mut expected = Inputs::default();
                let mut actual = Inputs::default();
                pad.poll(&mut expected);
                touch.poll(&mut actual);
                assert_eq!(
                    format!("{actual:?}"),
                    format!("{expected:?}"),
                    "{game}: touch {signal:?}={value}"
                );
                assert_eq!(touch.signal_p2(signal), 0.0, "touch must not drive P2");
            }
        }
    }
}

fn ports(out: &Inputs) -> [u8; 3] {
    [out.in0, out.in1, out.in2]
}

#[test]
fn every_set_routes_p2_signals_without_changing_p1_or_single_seat_controls() {
    use crate::input::players::Player;
    for cabinet in cabinets() {
        for game in cabinet.sets.split_whitespace() {
            for &signal in S::ALL.iter().filter(|s| s.supports_p2()) {
                let mut input = state(game, cabinet.scheme);
                input
                    .bindings
                    .set_player_expression(
                        Player::Two,
                        signal,
                        if signal.signed() {
                            "keys:F11/F12"
                        } else {
                            "F12"
                        },
                    )
                    .unwrap();
                input.on_key(KeyCode::F12, true);
                let mut out = Inputs::default();
                input.poll(&mut out);
                let mut expected = cabinet.idle;
                let common = match signal {
                    S::Coin if cabinet.coin2 => 2,
                    S::Start if cabinet.start2 => 0x20,
                    S::Test => {
                        if game == "bel" {
                            8
                        } else {
                            4
                        }
                    }
                    S::Service => {
                        if game == "bel" {
                            4
                        } else {
                            8
                        }
                    }
                    _ => 0,
                };
                expected[0] ^= common;
                if cabinet.scheme == Scheme::Joystick
                    && !game.starts_with("von")
                    && game != "rascot2"
                {
                    expected[2] ^= match signal {
                        S::Up => 0x20,
                        S::Down => 0x10,
                        S::Left => 0x80,
                        S::Right => 0x40,
                        _ => cabinet
                            .buttons
                            .iter()
                            .find(|&&(s, p, _)| s == signal && p == 1)
                            .map_or(0, |&(_, _, mask)| mask),
                    };
                } else if cabinet.scheme == Scheme::Gun {
                    let reload = matches!(
                        game,
                        "vcop" | "vcopa" | "vcop2" | "hotd" | "hotdo" | "hotdp"
                    ) && signal == S::SecondaryFire;
                    if signal == S::PrimaryFire || reload {
                        let (port, mask) = if game == "hotd" { (2, 1) } else { (1, 2) };
                        expected[port] ^= mask;
                    }
                    if game == "bel" && signal == S::SecondaryFire {
                        expected[1] ^= 0x20;
                    }
                    assert_eq!(out.gun2_offscreen, reload, "{game}: {signal:?}");
                    assert!(!out.gun_offscreen, "P2 reload must not reload P1");
                } else if game.starts_with("swa") {
                    expected[1] ^= match signal {
                        S::SwaLaser => 4,
                        S::SwaTorpedo => 8,
                        _ => 0,
                    };
                } else if cabinet.scheme == Scheme::Sled {
                    expected[1] ^= match signal {
                        S::SledEntry => 4,
                        S::SledCall => 8,
                        _ => 0,
                    };
                }
                assert_eq!(ports(&out), expected, "{game}: P2 {signal:?}");
                input.on_key(KeyCode::F12, false);
                input.poll(&mut out);
                assert_eq!(ports(&out), cabinet.idle, "{game}: P2 release {signal:?}");
            }
        }
    }
}

#[test]
fn p2_bats_and_sled_pedals_have_independent_full_range_adc_channels() {
    use crate::input::players::Player;
    for (games, signals) in [
        ("dynabb dynabb97", &[(S::BatSwing, A::Bat1, A::Bat2)][..]),
        (
            "powsled powsledm powsledr",
            &[(S::Accelerator, A::P1R, A::P2R), (S::Brake, A::P1L, A::P2L)][..],
        ),
    ] {
        for game in games.split_whitespace() {
            for &(signal, p1_role, p2_role) in signals {
                let mut input = state(
                    game,
                    if game.starts_with("dynabb") {
                        Scheme::Joystick
                    } else {
                        Scheme::Sled
                    },
                );
                let roles = db_roles(game);
                input.set_analog_roles(roles);
                // Feed the same normalized axis path as a real controller;
                // the default RightStickY/Z source strings are tested separately.
                input
                    .bindings
                    .set_player_expression(Player::Two, signal, "pad:LeftStickX+")
                    .unwrap();
                let ch1 = roles.iter().position(|&r| r == p1_role).unwrap();
                let ch2 = roles.iter().position(|&r| r == p2_role).unwrap();
                let mut out = Inputs::default();
                for (value, expected) in [(0.0, 0), (0.5, 128), (1.0, 255), (0.0, 0)] {
                    input.external_p2.left_x = value;
                    input.poll(&mut out);
                    assert_eq!(out.analog[ch2], expected, "{game}: {signal:?}");
                    assert_eq!(out.analog[ch1], 0, "{game}: P1 unaffected");
                    assert_eq!(ports(&out), [255; 3]);
                }
            }
        }
    }
}

#[test]
fn p2_gun_calibrations_cursor_hold_and_mouse_isolation() {
    for (game, xmin, xmax, ymin, ymax) in [
        ("vcop", 0x80, 0x273, 0x27, 0x1a9),
        ("vcopa", 0x80, 0x273, 0x27, 0x1a9),
        ("vcop2", 0x86, 0x273, 0x24, 0x1a9),
        ("hotd", 0xa3, 0x254, 0x57, 0x17c),
        ("hotdo", 0xa3, 0x254, 0x57, 0x17c),
        ("hotdp", 0xa3, 0x254, 0x57, 0x17c),
    ] {
        let mut input = state(game, Scheme::Gun);
        let mut out = Inputs::default();
        input.on_cursor(0.25, 0.75);
        input.poll(&mut out);
        let p1 = (out.gun_x, out.gun_y);
        for (x, y, expected) in [(-1.0, 1.0, (xmin, ymin)), (1.0, -1.0, (xmax, ymax))] {
            input.external_p2.left_x = x;
            input.external_p2.left_y = y;
            for _ in 0..60 {
                input.poll(&mut out);
            }
            assert_eq!((out.gun2_x, out.gun2_y), expected, "{game}");
            assert_eq!((out.gun_x, out.gun_y), p1);
        }
        input.external_p2.left_x = 0.0;
        input.external_p2.left_y = 0.0;
        input.poll(&mut out);
        assert_eq!(
            (out.gun2_x, out.gun2_y),
            (xmax, ymax),
            "cursor holds on release"
        );
        input.external_p2.buttons.insert(gilrs::Button::East);
        input.poll(&mut out);
        assert_eq!((out.gun2_x, out.gun2_y), (xmin, ymin));
        input.external_p2.buttons.clear();
        input.poll(&mut out);
        assert_eq!(
            (out.gun2_x, out.gun2_y),
            (xmax, ymax),
            "reload doesn't lose aim"
        );
    }
    for (game, xrole, yrole, min, max) in [
        ("gunblade", A::Gun2X, A::Gun2Y, [0x00, 0x11], [0x96, 0xae]),
        ("bel", A::Gun2X, A::Gun2Y, [0x00, 0x11], [0x96, 0xae]),
        ("rchase2", A::Gun2X, A::Gun2Y, [0xc7, 0xcb], [0x34, 0x1c]),
        ("rchase2a", A::Gun2X, A::Gun2Y, [0x00, 0x00], [0xff, 0xff]),
    ] {
        let mut input = state(game, Scheme::Gun);
        let roles = db_roles(game);
        input.set_analog_roles(roles);
        let xch = roles.iter().position(|&r| r == xrole).unwrap();
        let ych = roles.iter().position(|&r| r == yrole).unwrap();
        let mut out = Inputs::default();
        for (x, y, expected) in [(-1.0, 1.0, min), (1.0, -1.0, max)] {
            input.external_p2.left_x = x;
            input.external_p2.left_y = y;
            for _ in 0..60 {
                input.poll(&mut out);
            }
            assert_eq!([out.analog[xch], out.analog[ych]], expected, "{game}");
        }
    }
}

fn db_roles(game: &str) -> [A; 8] {
    let db = include_str!("../../../../tgpulse-core/src/roms_db.dat");
    let header = db
        .lines()
        .find(|line| line.starts_with(&format!("G {game} ")))
        .unwrap();
    let mut roles = [A::None; 8];
    for (i, name) in header.split_whitespace().skip(4).enumerate() {
        roles[i] = match name {
            "steer" => A::Steer,
            "accel" => A::Accel,
            "brake" => A::Brake,
            "throttle" => A::Throttle,
            "stickx" => A::StickX,
            "sticky" => A::StickY,
            "stick2x" => A::Stick2X,
            "stick2y" => A::Stick2Y,
            "gun1x" => A::Gun1X,
            "gun1y" => A::Gun1Y,
            "gun2x" => A::Gun2X,
            "gun2y" => A::Gun2Y,
            "roll" => A::Roll,
            "pitch" => A::Pitch,
            "slide" => A::Slide,
            "curving" => A::Curving,
            "swing" => A::Swing,
            "incline" => A::Incline,
            "bat1" => A::Bat1,
            "bat2" => A::Bat2,
            "p1r" => A::P1R,
            "p1l" => A::P1L,
            "p2r" => A::P2R,
            "p2l" => A::P2L,
            "none" => A::None,
            _ => panic!("unaudited role {name}"),
        };
    }
    roles
}

#[test]
fn every_catalogued_set_has_an_explicit_audit_case() {
    use std::collections::BTreeSet;
    let db = include_str!("../../../../tgpulse-core/src/roms_db.dat");
    let actual: BTreeSet<_> = db
        .lines()
        .filter_map(|line| line.strip_prefix("G "))
        .map(|line| line.split_whitespace().next().unwrap())
        .collect();
    let cases = cabinets();
    let expected: BTreeSet<_> = cases
        .iter()
        .flat_map(|c| c.sets.split_whitespace())
        .collect();
    assert_eq!(actual, expected, "new/removed sets require an input audit");
    assert_eq!(
        cases
            .iter()
            .map(|c| c.sets.split_whitespace().count())
            .sum::<usize>(),
        expected.len()
    );
}

#[test]
fn all_sets_route_each_signal_without_digital_crosstalk() {
    for cabinet in cabinets() {
        for game in cabinet.sets.split_whitespace() {
            for &signal in S::ALL {
                let mut input = state(game, cabinet.scheme);
                let mut bindings = Bindings::default();
                for &s in S::ALL {
                    bindings.set_expression(s, "").unwrap();
                }
                bindings
                    .set_expression(
                        signal,
                        if signal.signed() {
                            "keys:F11/F12"
                        } else {
                            "F12"
                        },
                    )
                    .unwrap();
                input.set_bindings(bindings);
                let mut out = Inputs::default();
                input.poll(&mut out);
                assert_eq!(ports(&out), cabinet.idle, "{game} idle");
                input.on_key(KeyCode::F12, true);
                input.poll(&mut out);
                let mut expected = cabinet.idle;
                let common = match signal {
                    S::Coin => 1,
                    S::Test => {
                        if game == "bel" {
                            8
                        } else {
                            4
                        }
                    }
                    S::Service => {
                        if game == "bel" {
                            4
                        } else {
                            8
                        }
                    }
                    S::Start => cabinet.start,
                    _ => 0,
                };
                expected[0] ^= common;
                if cabinet.scheme == Scheme::Joystick && !game.starts_with("von") {
                    expected[1] ^= match signal {
                        S::Up => 0x20,
                        S::Down => 0x10,
                        S::Left => 0x80,
                        S::Right => 0x40,
                        _ => 0,
                    };
                }
                for &(s, port, mask) in cabinet.buttons {
                    if signal == s {
                        expected[port] ^= mask;
                    }
                }
                assert_eq!(ports(&out), expected, "{game}: {signal:?}");
                input.on_key(KeyCode::F12, false);
                input.poll(&mut out);
                let latching = (game.starts_with("daytona") || game.starts_with("srally"))
                    && matches!(
                        signal,
                        S::GearUp | S::Gear1 | S::Gear2 | S::Gear3 | S::Gear4
                    )
                    || game == "desert" && signal == S::DesertShift;
                assert_eq!(
                    ports(&out),
                    if latching { expected } else { cabinet.idle },
                    "{game}: release {signal:?}"
                );
            }
        }
    }
}

#[test]
fn default_indy_stcc_overrev_buttons_are_isolated() {
    use gilrs::Button as B;
    for game in [
        "indy500",
        "indy500d",
        "indy500to",
        "stcc",
        "stcca",
        "stccb",
        "stcco",
        "overrev",
        "overrevb",
        "overrevba",
    ] {
        let mut input = state(game, Scheme::Racing);
        let mut out = Inputs::default();
        for (button, expected) in [
            (B::Start, [0xbf, 255, 255]),
            (B::DPadUp, [255, 0xfe, 255]),
            (B::DPadDown, [255, 0xfd, 255]),
            (B::DPadLeft, [255; 3]),
            (B::DPadRight, [255; 3]),
            (B::RightTrigger, [255, 0xef, 255]),
            (B::LeftTrigger, [255, 0xdf, 255]),
            (B::LeftThumb, [0xfb, 255, 255]),
            (B::RightThumb, [0xf7, 255, 255]),
        ] {
            input.set_pad_button(button, true);
            input.poll(&mut out);
            assert_eq!(ports(&out), expected, "{game} {button:?}");
            input.set_pad_button(button, false);
            input.poll(&mut out);
            assert_eq!(ports(&out), [255; 3]);
        }
        input.set_pad_button(B::RightTrigger, true);
        input.set_pad_button(B::LeftTrigger, true);
        input.poll(&mut out);
        assert_eq!(ports(&out), [255; 3], "{game} conflicting shifts");
    }
}

#[test]
fn player_two_furniture_preserves_all_existing_cabinet_routes() {
    use crate::input::players::Player;
    for cabinet in cabinets() {
        for game in cabinet.sets.split_whitespace() {
            for signal in [S::Coin, S::Start, S::Test, S::Service] {
                let mut input = state(game, cabinet.scheme);
                let mut bindings = Bindings::default();
                bindings
                    .set_player_expression(Player::Two, signal, "F12")
                    .unwrap();
                input.set_bindings(bindings);
                input.on_key(KeyCode::F12, true);
                let mut out = Inputs::default();
                input.poll(&mut out);
                let mask = match signal {
                    S::Coin if cabinet.coin2 => 2,
                    S::Start if cabinet.start2 => 0x20,
                    S::Test => {
                        if game == "bel" {
                            8
                        } else {
                            4
                        }
                    }
                    S::Service => {
                        if game == "bel" {
                            4
                        } else {
                            8
                        }
                    }
                    _ => 0,
                };
                let mut expected = cabinet.idle;
                expected[0] ^= mask;
                assert_eq!(ports(&out), expected, "{game}: P2 {signal:?}");
                input.on_key(KeyCode::F12, false);
                input.poll(&mut out);
                assert_eq!(ports(&out), cabinet.idle);
            }
        }
    }
}

#[test]
fn motor_raid_attacks_remain_independent() {
    let mut input = state("motoraid", Scheme::Bike);
    input.set_pad_button(gilrs::Button::East, true);
    input.set_pad_button(gilrs::Button::South, true);
    let mut out = Inputs::default();
    input.poll(&mut out);
    assert_eq!(out.in1, 0xcf);
}

#[test]
fn default_physical_buttons_produce_only_the_expected_action() {
    use crate::input::players::Player;
    use gilrs::Button as B;
    // Profili H:W supplies the physical positions except the explicit local
    // Test/Service preference. Check the whole port image, not just one bit.
    for cabinet in cabinets() {
        let root = cabinet.sets.split_whitespace().next().unwrap();
        let buttons: &[(B, usize, u8)] = match root {
            "vf" | "fvipers" => &[(B::South, 1, 2), (B::East, 1, 1), (B::West, 1, 4)],
            "doa" => &[(B::South, 1, 4), (B::East, 1, 2), (B::West, 1, 1)],
            "vstriker" => &[(B::South, 1, 4), (B::East, 1, 1), (B::West, 1, 2)],
            "hpyagu98" | "rascot2" => &[(B::South, 1, 1), (B::East, 1, 2), (B::West, 1, 4)],
            "dynabb" | "zerogun" => &[(B::South, 1, 1), (B::East, 1, 2)],
            "motoraid" => &[
                (B::South, 1, 0x20),
                (B::LeftTrigger, 1, 0x20),
                (B::East, 1, 0x10),
                (B::RightTrigger, 1, 0x10),
            ],
            "desert" => &[
                (B::South, 1, 0x10),
                (B::RightTrigger, 1, 0x10),
                (B::East, 1, 0x20),
                (B::LeftTrigger, 1, 0x20),
                (B::West, 1, 1),
                (B::DPadDown, 0, 0x20),
                (B::DPadLeft, 0, 0x40),
                (B::DPadUp, 0, 0x80),
            ],
            "srallyc" => &[(B::South, 2, 255), (B::DPadDown, 0, 0x20)],
            "manxtt" => &[(B::DPadDown, 0, 0x40), (B::Start, 0, 0x40)],
            "von" => &[(B::LeftTrigger, 1, 2), (B::RightTrigger, 2, 2)],
            "segawski" => &[
                (B::South, 1, 1),
                (B::East, 1, 8),
                (B::RightTrigger, 1, 8),
                (B::West, 1, 4),
                (B::LeftTrigger, 1, 4),
                (B::DPadUp, 1, 2),
                (B::DPadDown, 0, 0x40),
                (B::Start, 0, 0x40),
            ],
            "skisuprg" => &[
                (B::South, 0, 0x80),
                (B::East, 0, 0x10),
                (B::West, 0, 0x40),
                (B::LeftTrigger, 2, 0xf0),
                (B::RightTrigger, 2, 0x0f),
                (B::DPadUp, 0, 0x20),
                (B::DPadDown, 1, 1),
            ],
            "topskatr" => &[
                (B::South, 1, 1),
                (B::East, 0, 0x20),
                (B::DPadLeft, 0, 0x80),
                (B::DPadRight, 0, 0x10),
            ],
            "skytargt" => &[
                (B::South, 1, 0x10),
                (B::RightTrigger, 1, 0x10),
                (B::East, 1, 0x20),
                (B::LeftTrigger, 1, 0x20),
                (B::DPadUp, 0, 0x20),
            ],
            "bel" => &[
                (B::South, 1, 1),
                (B::RightTrigger, 1, 1),
                (B::East, 1, 0x10),
                (B::LeftTrigger, 1, 0x10),
            ],
            "vcop" => &[
                (B::South, 1, 1),
                (B::RightTrigger, 1, 1),
                (B::East, 1, 1),
                (B::LeftTrigger, 1, 1),
            ],
            "gunblade" => &[(B::South, 1, 1), (B::RightTrigger, 1, 1)],
            "waverunr" => &[(B::DPadUp, 1, 1)],
            _ => &[],
        };
        for game in cabinet.sets.split_whitespace() {
            for &(button, port, mask) in buttons {
                let mut input = state(game, cabinet.scheme);
                input.set_pad_button(button, true);
                let mut out = Inputs::default();
                input.poll(&mut out);
                let mut expected = cabinet.idle;
                expected[port] ^= mask;
                assert_eq!(ports(&out), expected, "{game} {button:?}");
                if cabinet.scheme == Scheme::Joystick && cabinet.start2 {
                    input.external.buttons.clear();
                    input.external_p2.present = true;
                    input.external_p2.buttons.insert(button);
                    input.poll(&mut out);
                    expected = cabinet.idle;
                    expected[2] ^= mask;
                    assert_eq!(ports(&out), expected, "{game} P2 {button:?}");
                }
            }
            for player in Player::ALL {
                for (button, mask) in [
                    (B::LeftThumb, if game == "bel" { 8 } else { 4 }),
                    (B::RightThumb, if game == "bel" { 4 } else { 8 }),
                ] {
                    let mut input = state(game, cabinet.scheme);
                    let pad = if player == Player::One {
                        &mut input.external
                    } else {
                        &mut input.external_p2
                    };
                    pad.present = true;
                    pad.buttons.insert(button);
                    let mut out = Inputs::default();
                    input.poll(&mut out);
                    let mut expected = cabinet.idle;
                    expected[0] ^= mask;
                    assert_eq!(ports(&out), expected, "{game} {player:?} {button:?}");
                }
            }
        }
    }
    for (left, right, expected) in [
        (1.0, 0.0, [255, 254, 255]),
        (0.0, 1.0, [255, 255, 254]),
        (1.0, 1.0, [255, 254, 254]),
    ] {
        let mut input = state("von", Scheme::Joystick);
        for (signal, source, key, amount) in [
            (S::TwinLeftShot, "pad:LeftZ+", KeyCode::F11, left),
            (S::TwinRightShot, "pad:RightZ+", KeyCode::F12, right),
        ] {
            assert!(input
                .bindings
                .binding(signal)
                .uses_source(crate::bindings::parse_source(source).unwrap()));
            input
                .bindings
                .set_expression(signal, if key == KeyCode::F11 { "F11" } else { "F12" })
                .unwrap();
            input.on_key(key, amount > 0.5);
        }
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!(ports(&out), expected, "Virtual On triggers");
    }
}

#[test]
fn handbrake_retains_partial_travel_when_rebound_to_an_axis() {
    let mut input = state("srallyc", Scheme::Racing);
    input
        .bindings
        .set_expression(S::Handbrake, "pad:LeftStickX+")
        .unwrap();
    input.set_pad_stick(0.5, 0.0);
    let mut out = Inputs::default();
    input.poll(&mut out);
    assert_eq!(out.in2, 128);
}

#[test]
fn sky_and_gun_fire_bindings_can_be_reassigned_separately() {
    for (game, scheme, active, inactive, port, bit) in [
        (
            "skytargt",
            Scheme::Flight,
            S::SkyMachineGun,
            S::PrimaryFire,
            1,
            0x10,
        ),
        (
            "skytargt",
            Scheme::Flight,
            S::SkyMissile,
            S::SecondaryFire,
            1,
            0x20,
        ),
        (
            "gunblade",
            Scheme::Gun,
            S::PrimaryFire,
            S::SkyMachineGun,
            1,
            1,
        ),
        ("bel", Scheme::Gun, S::SecondaryFire, S::SkyMissile, 1, 0x10),
    ] {
        let mut input = state(game, scheme);
        input.bindings.set_expression(active, "F12").unwrap();
        input.bindings.set_expression(inactive, "F11").unwrap();
        let mut out = Inputs::default();
        input.on_key(KeyCode::F11, true);
        input.poll(&mut out);
        assert_eq!(ports(&out), [255; 3], "{game}: other game's signal");
        input.on_key(KeyCode::F11, false);
        input.on_key(KeyCode::F12, true);
        input.poll(&mut out);
        let mut expected = [255; 3];
        expected[port] ^= bit;
        assert_eq!(ports(&out), expected, "{game}: own signal");
    }
}

#[test]
fn gun_reload_is_only_for_serial_cabinets_and_calibration_is_per_set() {
    for (game, roles, corner, other) in [
        (
            "gunblade",
            [A::Gun1X, A::Gun2X, A::Gun1Y, A::Gun2Y],
            [0x69, 0x11],
            [0x50, 0x5f],
        ),
        (
            "bel",
            [A::Gun1X, A::Gun2X, A::Gun1Y, A::Gun2Y],
            [0x69, 0x11],
            [0x50, 0x5f],
        ),
        (
            "rchase2",
            [A::Gun2X, A::Gun1X, A::Gun2Y, A::Gun1Y],
            [0xca, 0xcb],
            [0x7d, 0x73],
        ),
        (
            "rchase2a",
            [A::Gun2X, A::Gun1X, A::Gun2Y, A::Gun1Y],
            [0, 0],
            [0x80, 0x80],
        ),
    ] {
        let mut input = state(game, Scheme::Gun);
        let mut channels = [A::None; 8];
        channels[..4].copy_from_slice(&roles);
        input.set_analog_roles(channels);
        input.on_cursor(0.0, 0.0);
        input.mouse_reload = true;
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert!(!out.gun_offscreen, "{game}");
        assert_ne!(out.in1 & 1, 0, "secondary must not fire {game}");
        for (role, value) in [
            (A::Gun1X, corner[0]),
            (A::Gun1Y, corner[1]),
            (A::Gun2X, other[0]),
            (A::Gun2Y, other[1]),
        ] {
            assert_eq!(
                out.analog[roles.iter().position(|r| *r == role).unwrap()],
                value,
                "{game} {role:?}"
            );
        }
    }
    for (game, expected) in [
        ("vcop", (131, 36)),
        ("vcop2", (137, 36)),
        ("hotd", (173, 87)),
        ("hotdo", (173, 87)),
        ("hotdp", (173, 87)),
    ] {
        let mut input = state(game, Scheme::Gun);
        input.on_cursor(0.0, 0.0);
        input.mouse_reload = true;
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert!(out.gun_offscreen);
        assert_eq!(out.in1 & 1, 0);
        assert_eq!((out.gun_x, out.gun_y), expected);
    }
}

#[test]
fn car_and_bike_adc_ranges_polarity_and_mirrors_cover_every_revision() {
    for cabinet in cabinets()
        .into_iter()
        .filter(|c| matches!(c.scheme, Scheme::Racing | Scheme::Bike))
    {
        for game in cabinet.sets.split_whitespace() {
            let legacy = matches!(game, "vr" | "vformula");
            for (signal, role) in [
                (S::Steering, A::Steer),
                (
                    S::Accelerator,
                    if cabinet.scheme == Scheme::Bike {
                        A::Throttle
                    } else {
                        A::Accel
                    },
                ),
                (S::Brake, A::Brake),
            ] {
                if game == "desert" && signal == S::Brake {
                    continue;
                }
                let mut input = state(game, cabinet.scheme);
                let roles = db_roles(game);
                input.set_analog_roles(roles);
                let channel = roles.iter().position(|r| *r == role).unwrap();
                let steer = signal == S::Steering;
                input
                    .bindings
                    .set_expression(signal, if steer { "keys:F11/F12" } else { "F12" })
                    .unwrap();
                let reverse = if steer {
                    cabinet.scheme == Scheme::Bike
                } else {
                    game.starts_with("overrev") || game == "sgt24h"
                };
                let (min, max) = if legacy { (32, 224) } else { (0, 255) };
                for (key, expected) in [
                    (
                        None,
                        if steer {
                            128
                        } else if reverse {
                            max
                        } else {
                            min
                        },
                    ),
                    (Some(KeyCode::F12), if reverse { min } else { max }),
                    (
                        Some(KeyCode::F11),
                        if steer && reverse {
                            max
                        } else if !steer && reverse {
                            max
                        } else {
                            min
                        },
                    ),
                ] {
                    input.keys.clear();
                    if let Some(k) = key {
                        input.on_key(k, true);
                    }
                    let mut out = Inputs::default();
                    for _ in 0..40 {
                        input.poll(&mut out);
                    }
                    assert_eq!(out.analog[channel], expected, "{game} {signal:?} {key:?}");
                    assert_eq!(
                        [out.steer, out.accel, out.brake],
                        out.analog[..3],
                        "original I/O mirrors {game}"
                    );
                }
            }
        }
    }
}

#[test]
fn independent_analog_signals_reach_the_documented_channel_only() {
    // Expected values: negative, centre, positive. Positive Y means UP.
    let specs: &[(&str, Scheme, S, usize, [u8; 3])] = &[
        ("skytargt", Scheme::Flight, S::SkyX, 2, [255, 128, 0]),
        ("skytargt", Scheme::Flight, S::SkyY, 0, [255, 128, 0]),
        ("swa", Scheme::Flight, S::SkyX, 0, [227, 127, 27]),
        ("swaj", Scheme::Flight, S::SkyY, 1, [227, 127, 27]),
        ("wingwar", Scheme::Flight, S::SkyX, 0, [255, 128, 0]),
        ("wingwarj", Scheme::Flight, S::SkyY, 1, [255, 128, 0]),
        ("wingwaru", Scheme::Flight, S::SkyY, 1, [255, 128, 0]),
        ("wingwar360", Scheme::Flight, S::SkyX, 0, [0, 128, 255]),
        ("wingwar360", Scheme::Flight, S::SkyY, 1, [0, 128, 255]),
        ("netmerc", Scheme::Flight, S::SkyX, 0, [0, 127, 255]),
        ("netmerc", Scheme::Flight, S::SkyY, 2, [0, 127, 255]),
        ("desert", Scheme::Racing, S::Elevation, 2, [255, 128, 0]),
        ("segawski", Scheme::Ski, S::WaterSlide, 0, [255, 128, 0]),
        ("topskatr", Scheme::Skate, S::Curving, 0, [255, 128, 0]),
        ("topskatrj", Scheme::Skate, S::Curving, 0, [255, 128, 0]),
        ("topskatru", Scheme::Skate, S::Curving, 0, [255, 128, 0]),
        ("topskatruo", Scheme::Skate, S::Curving, 0, [255, 128, 0]),
        ("topskatr", Scheme::Skate, S::SkaterSlide, 1, [0, 128, 255]),
        // Tested SM2 order overrides the MAME-generated metadata.
        ("skisuprg", Scheme::Ski, S::Swing, 1, [255, 128, 0]),
        ("skisuprg", Scheme::Ski, S::Inclining, 0, [0, 128, 255]),
        ("waverunr", Scheme::Jetski, S::Roll, 1, [0, 128, 255]),
        ("waverunr", Scheme::Jetski, S::Pitch, 3, [0, 128, 255]),
    ];
    for &(game, scheme, signal, channel, values) in specs {
        let mut input = state(game, scheme);
        input.set_analog_roles(db_roles(game));
        for &s in S::ALL {
            input.bindings.set_expression(s, "").unwrap();
        }
        input
            .bindings
            .set_expression(signal, "keys:F11/F12")
            .unwrap();
        let mut idle = Inputs::default();
        input.poll(&mut idle);
        for (key, value) in [
            (Some(KeyCode::F11), values[0]),
            (None, values[1]),
            (Some(KeyCode::F12), values[2]),
        ] {
            input.keys.clear();
            if let Some(k) = key {
                input.on_key(k, true);
            }
            let mut out = Inputs::default();
            for _ in 0..40 {
                input.poll(&mut out);
            }
            let mut expected = idle.analog;
            expected[channel] = value;
            assert_eq!(out.analog, expected, "{game} {signal:?} {key:?}");
            assert_eq!([out.steer, out.accel, out.brake], out.analog[..3]);
        }
    }
}

#[test]
fn gun_stick_moves_and_holds_the_same_cursor_that_is_drawn() {
    for game in [
        "vcop", "vcopa", "vcop2", "hotd", "hotdo", "hotdp", "gunblade", "bel", "rchase2",
        "rchase2a",
    ] {
        let mut input = state(game, Scheme::Gun);
        input.set_pad_stick(0.5, 0.5);
        let mut out = Inputs::default();
        input.poll(&mut out);
        let first = input.aim();
        assert!(first.0 > 0.5 && first.1 < 0.5);
        input.poll(&mut out);
        let moved = input.aim();
        assert!(moved.0 > first.0 && moved.1 < first.1);
        input.set_pad_stick(0.0, 0.0);
        input.poll(&mut out);
        assert_eq!(input.aim(), moved, "{game} release");
        input.on_cursor(0.25, 0.75);
        input.poll(&mut out);
        assert_eq!(input.aim(), (0.25, 0.75), "{game} mouse");
    }
}

#[test]
fn swa_view_uses_view1_defaults_not_action3() {
    for game in ["swa", "swaj"] {
        let mut input = state(game, Scheme::Flight);
        let mut out = Inputs::default();
        for (button, expected) in [
            (gilrs::Button::DPadDown, [255, 0xef, 255]),
            (gilrs::Button::DPadUp, [255, 0xef, 255]),
            (gilrs::Button::West, [255; 3]),
        ] {
            input.set_pad_button(button, true);
            input.poll(&mut out);
            assert_eq!(ports(&out), expected, "{game} {button:?}");
            input.set_pad_button(button, false);
            input.poll(&mut out);
            assert_eq!(ports(&out), [255; 3]);
        }
        for (key, expected) in [(KeyCode::KeyZ, [255, 0xef, 255]), (KeyCode::KeyL, [255; 3])] {
            input.on_key(key, true);
            input.poll(&mut out);
            assert_eq!(ports(&out), expected, "{game} {key:?}");
            input.on_key(key, false);
            input.poll(&mut out);
            assert_eq!(ports(&out), [255; 3]);
        }
    }
}

#[test]
fn single_view_pad_alias_preserves_multiview_and_other_cabinets() {
    use gilrs::Button as B;
    for cabinet in cabinets() {
        for game in cabinet.sets.split_whitespace() {
            let single = game.starts_with("srally") || matches!(game, "sgt24h" | "swa" | "swaj");
            let mut input = state(game, cabinet.scheme);
            if !cabinet
                .buttons
                .iter()
                .any(|&(s, _, _)| matches!(s, S::View1 | S::View2 | S::View3 | S::View4))
            {
                input.set_pad_button(B::DPadUp, true);
                assert_eq!(input.signal(S::View1), 0.0, "{game}: no view alias");
                continue;
            }
            for button in [B::DPadUp, B::DPadDown] {
                let mut baseline = Inputs::default();
                input.poll(&mut baseline);
                let mut expected = ports(&baseline);
                let signal = if button == B::DPadDown || single {
                    S::View1
                } else {
                    S::View4
                };
                // Other cabinet types can map Up/Down to joystick movement.
                let direction = if button == B::DPadUp { S::Up } else { S::Down };
                for &(s, port, mask) in cabinet.buttons {
                    if s == signal || s == direction {
                        expected[port] &= !mask;
                    }
                }
                input.set_pad_button(button, true);
                let mut out = Inputs::default();
                input.poll(&mut out);
                assert_eq!(ports(&out), expected, "{game}: {button:?}");
                input.set_pad_button(button, false);
            }
            if single {
                input.on_key(KeyCode::KeyV, true);
                let mut out = Inputs::default();
                input.poll(&mut out);
                assert_eq!(ports(&out), cabinet.idle, "{game}: no keyboard alias");
                input.on_key(KeyCode::KeyV, false);
                input.set_pad_button(B::DPadUp, true);
                input.set_pad_button(B::DPadDown, true);
                input.poll(&mut out);
                let mut expected = cabinet.idle;
                for &(s, port, mask) in cabinet.buttons {
                    if s == S::View1 {
                        expected[port] &= !mask;
                    }
                }
                assert_eq!(ports(&out), expected, "{game}: OR, not toggle/cancellation");
            }
        }
    }
}

#[test]
fn swa_throttle_default_keys_cover_both_directions_and_cancel() {
    for game in ["swa", "swaj"] {
        let mut input = state(game, Scheme::Flight);
        input.set_analog_roles(db_roles(game));
        let mut out = Inputs::default();
        for (accelerator, brake, expected) in [
            (false, false, 128),
            (true, false, 228),
            (false, true, 28),
            (true, true, 128),
        ] {
            input.on_key(KeyCode::KeyW, accelerator);
            input.on_key(KeyCode::KeyS, brake);
            input.poll(&mut out);
            assert_eq!(out.analog[2], expected, "{game} throttle");
        }
    }
}

#[test]
fn wave_runner_throttle_keeps_its_own_rest_and_range() {
    for (game, scheme, channel, rest, max) in [("waverunr", Scheme::Jetski, 2, 128, 0)] {
        let mut input = state(game, scheme);
        input.set_analog_roles(db_roles(game));
        input
            .bindings
            .set_expression(S::Accelerator, "F12")
            .unwrap();
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!(out.analog[channel], rest, "{game} rest");
        input.on_key(KeyCode::F12, true);
        input.poll(&mut out);
        assert_eq!(out.analog[channel], max, "{game} throttle");
    }
}

#[test]
fn flight_throttle_half_axes_are_independent_of_pedals_and_cancel() {
    for (game, min, max, half_up, half_down) in [
        ("swa", 28, 228, 178, 78),
        ("swaj", 28, 228, 178, 78),
        ("wingwar", 1, 255, 192, 65),
        ("wingwaru", 1, 255, 192, 65),
        ("wingwarj", 1, 255, 192, 65),
        ("wingwar360", 1, 255, 192, 65),
    ] {
        let mut input = state(game, Scheme::Flight);
        input.set_analog_roles(db_roles(game));
        let mut out = Inputs::default();
        for (up, down, expected) in [
            (false, false, 128),
            (true, false, max),
            (false, true, min),
            (true, true, 128),
        ] {
            input.on_key(KeyCode::KeyW, up);
            input.on_key(KeyCode::KeyS, down);
            input.poll(&mut out);
            assert_eq!(out.analog[2], expected, "{game} keyboard");
        }
        input.keys.clear();
        // Rebinding the shared pedals must not change the dedicated throttle.
        input
            .bindings
            .set_expression(S::Accelerator, "F11")
            .unwrap();
        input.bindings.set_expression(S::Brake, "F12").unwrap();
        input.on_key(KeyCode::F11, true);
        input.poll(&mut out);
        assert_eq!(out.analog[2], 128, "{game} independent accelerator");
        input.keys.clear();
        input.on_key(KeyCode::F12, true);
        input.poll(&mut out);
        assert_eq!(out.analog[2], 128, "{game} independent brake");
        input.keys.clear();
        // Exercise assignable half-axes with synthetic analog travel, without
        // depending on a connected controller or its native trigger mapping.
        input
            .bindings
            .set_expression(S::ThrottleUp, "pad:LeftStickX+")
            .unwrap();
        input
            .bindings
            .set_expression(S::ThrottleDown, "pad:LeftStickY+")
            .unwrap();
        for (up, down, expected) in [
            (0.0, 0.0, 128),
            (1.0, 0.0, max),
            (0.0, 1.0, min),
            (0.5, 0.0, half_up),
            (0.0, 0.5, half_down),
            (0.5, 0.5, 128),
            (1.0, 1.0, 128),
        ] {
            input.set_pad_stick(up, down);
            input.poll(&mut out);
            assert_eq!(out.analog[2], expected, "{game} analog {up}/{down}");
        }
    }
}
