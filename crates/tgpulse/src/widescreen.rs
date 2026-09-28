//! Read-only interpretation of confirmed operator EEPROM fields.
//! Offsets refer to little-endian serialized EEPROM bytes, not .nv headers.
//! Model 2 fields match SM2-Emu Libretro's automatic_widescreen().
pub fn cabinet_wide(game: &str, model1: bool, words: &[u16]) -> bool {
    let byte = |offset: usize| {
        words
            .get(offset / 2)
            .map(|word| (word >> (8 * (offset % 2))) as u8)
    };
    if model1 {
        // VR: controlled service-menu save/reload, 4:3=0 / 16:9=1.
        // Do not inherit this field for Virtua Formula without verification.
        return game == "vr" && byte(0x0a) == Some(1);
    }
    match game {
        "indy500" | "indy500d" | "indy500to" => byte(0x17) == Some(1),
        "stcc" | "stcca" | "stccb" | "stcco" => byte(0x10).is_some_and(|v| v != 0xff && v & 8 != 0),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cabinet_fields_are_byte_addressed_and_game_specific() {
        let mut words = [0; 64];
        assert!(!cabinet_wide("vr", true, &words));
        words[5] = 0xff01;
        assert!(cabinet_wide("vr", true, &words));
        assert!(!cabinet_wide("vf", true, &words));
        assert!(!cabinet_wide("vformula", true, &words));
        words[0x17 / 2] = 0x0100;
        for game in ["indy500", "indy500d", "indy500to"] {
            assert!(cabinet_wide(game, false, &words));
        }
        words[0x17 / 2] = 1;
        assert!(!cabinet_wide("indy500", false, &words));
        words[8] = 8;
        for game in ["stcc", "stcca", "stccb", "stcco"] {
            assert!(cabinet_wide(game, false, &words));
        }
        assert!(!cabinet_wide("daytona", false, &words));
        for game in ["vr", "indy500", "stcc"] {
            assert!(!cabinet_wide(game, game == "vr", &[]));
            assert!(!cabinet_wide(game, game == "vr", &[0xffff; 64]));
        }
    }
}
