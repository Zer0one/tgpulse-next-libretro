use super::*;
use gilrs::Button as B;

fn cabinet(game: &str) -> InputState {
    let mut input = InputState::with_gilrs(None);
    input.set_game(game);
    use AnalogRole as A;
    let roles = if game.starts_with("swa") {
        let db = include_str!("../../../tgpulse-core/src/roms_db.dat");
        assert!(db.lines().any(
            |l| l == format!("G {game} m1 flight stickx sticky throttle none stick2x stick2y")
        ));
        input.set_scheme(ControlScheme::Flight);
        [
            A::StickX,
            A::StickY,
            A::Throttle,
            A::None,
            A::Stick2X,
            A::Stick2Y,
            A::None,
            A::None,
        ]
    } else if game == "vf" {
        input.set_scheme(ControlScheme::Joystick);
        [A::None; 8]
    } else if matches!(game, "vr" | "vformula") {
        input.set_scheme(ControlScheme::Racing);
        [
            A::Steer,
            A::Accel,
            A::Brake,
            A::None,
            A::None,
            A::None,
            A::None,
            A::None,
        ]
    } else {
        input.set_scheme(ControlScheme::Flight);
        [
            A::StickX,
            A::StickY,
            A::Throttle,
            A::None,
            A::None,
            A::None,
            A::None,
            A::None,
        ]
    };
    input.set_analog_roles(roles);
    input
}

#[test]
fn swa_gunner_is_independent_and_uses_the_reference_adc_channels() {
    for game in ["swa", "swaj"] {
        let mut input = cabinet(game);
        input.external.left_x = 0.5;
        input.external.left_y = -0.5;
        input.external.buttons = HashSet::from([B::South, B::DPadDown]);
        input.external_p2.left_x = -1.0;
        input.external_p2.left_y = 1.0;
        input.external_p2.buttons = HashSet::from([B::East, B::Start, B::Select]);
        let mut out = Inputs::default();
        input.poll(&mut out);
        assert_eq!([out.analog[0], out.analog[1]], [77, 177]);
        assert_eq!([out.analog[4], out.analog[5]], [227, 27]);
        assert_eq!(out.analog[2], 128, "P2 cannot move the pilot throttle");
        assert_eq!(out.in0, 0xfd, "P2 Coin; no Gunner Start");
        assert_eq!(out.in1, 0xe6, "P1 fire1/view; P2 fire2");
        input.external = ExternalPad::default();
        input.poll(&mut out);
        assert_eq!([out.analog[0], out.analog[1]], [127, 127]);
        assert_eq!([out.analog[4], out.analog[5]], [227, 27]);
        assert_eq!(out.in1, 0xf7);
        input.external_p2 = ExternalPad::default();
        input.poll(&mut out);
        assert_eq!([out.analog[4], out.analog[5]], [127, 127]);
        assert_eq!(out.in0, 255);
        assert_eq!(out.in1, 255);
    }
}

#[test]
fn vf_both_players_have_independent_actions_and_directions() {
    let mut input = cabinet("vf");
    let mut out = Inputs::default();
    for (button, bit) in [
        (B::West, 4),
        (B::South, 2),
        (B::LeftTrigger, 2),
        (B::East, 1),
        (B::RightTrigger, 1),
        (B::DPadUp, 0x20),
        (B::DPadDown, 0x10),
        (B::DPadLeft, 0x80),
        (B::DPadRight, 0x40),
    ] {
        input.external.buttons = HashSet::from([B::Start]);
        input.external_p2.buttons = HashSet::from([button]);
        input.poll(&mut out);
        assert_eq!(out.in0, 0xef);
        assert_eq!(out.in1, 255);
        assert_eq!(out.in2, 255 ^ bit, "{button:?}");
        input.external.buttons = HashSet::from([button]);
        input.poll(&mut out);
        assert_eq!(out.in1, out.in2);
    }
}

#[test]
fn duplicate_test_service_or_together_without_other_inputs() {
    for game in ["vf", "swa", "swaj", "vr", "wingwar"] {
        let mut input = cabinet(game);
        let mut out = Inputs::default();
        for (button, bit) in [(B::LeftThumb, 4), (B::RightThumb, 8)] {
            input.external.buttons = HashSet::from([button]);
            input.external_p2.buttons = HashSet::from([button]);
            input.poll(&mut out);
            assert_eq!(out.in0, 255 ^ bit);
            assert_eq!(out.in1, 255);
            assert_eq!(out.in2, 255);
            input.external.buttons.clear();
            input.poll(&mut out);
            assert_eq!(out.in0, 255 ^ bit);
            input.external_p2.buttons.clear();
            input.poll(&mut out);
            assert_eq!(out.in0, 255);
        }
    }
}

#[test]
fn p2_rebinding_has_no_effect_on_p1_and_no_default_gameplay_keys() {
    let mut input = cabinet("swa");
    input.on_key(KeyCode::KeyJ, true);
    input.on_key(KeyCode::KeyT, true);
    let mut out = Inputs::default();
    input.poll(&mut out);
    assert_eq!(out.in1, 0xfe);
    assert_eq!(out.analog[5], 127);
    input
        .bindings
        .set_player_expression(Player::Two, Signal::SwaLaser, "KeyU")
        .unwrap();
    input
        .bindings
        .set_player_expression(Player::Two, Signal::SkyY, "keys:KeyO/KeyP")
        .unwrap();
    input.on_key(KeyCode::KeyU, true);
    input.on_key(KeyCode::KeyP, true);
    input.poll(&mut out);
    assert_eq!(out.in1, 0xfa);
    assert_eq!(out.analog[5], 27);
    assert_eq!(
        input.bindings.binding(Signal::SwaLaser).text,
        Signal::SwaLaser.default_text()
    );
}

#[test]
fn p2_gameplay_does_not_leak_into_single_player_model1_cabinets() {
    for game in [
        "vr", "vformula", "wingwar", "wingwaru", "wingwarj", "netmerc",
    ] {
        let mut input = cabinet(game);
        let mut before = Inputs::default();
        input.poll(&mut before);
        input.external_p2.buttons = HashSet::from([
            B::South,
            B::East,
            B::West,
            B::North,
            B::DPadDown,
            B::DPadUp,
            B::DPadLeft,
            B::DPadRight,
        ]);
        input.external_p2.left_x = 1.0;
        input.external_p2.left_y = -1.0;
        let mut after = Inputs::default();
        input.poll(&mut after);
        assert_eq!(
            [after.in0, after.in1, after.in2],
            [before.in0, before.in1, before.in2],
            "{game}"
        );
        assert_eq!(after.analog, before.analog, "{game}");
    }
}
