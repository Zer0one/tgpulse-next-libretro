//! Per-set network presets reuse reviewed NVRAM fields; host/client roles stay independent.
use std::ffi::CStr;
use crate::{ffi, netpacket, nvram, option_value};

// Same order as the native-eligible catalogue in netpacket::GAMES.
pub const KEYS: [&CStr; 6] = [
    c"tgpulse_next_automatic_network_vr", c"tgpulse_next_automatic_network_vformula",
    c"tgpulse_next_automatic_network_wingwar", c"tgpulse_next_automatic_network_wingwaru",
    c"tgpulse_next_automatic_network_wingwarj", c"tgpulse_next_automatic_network_wingwar360",
];
pub const COLORS: &[(&CStr, &CStr)] = &[
    (c"red", c"Red (Master)"), (c"orange", c"Orange (Slave)"),
    (c"skyblue", c"Skyblue (Slave)"), (c"pink", c"Pink (Slave)"),
    (c"black", c"Black (Slave)"), (c"green", c"Green (Slave)"),
    (c"yellow", c"Yellow (Slave)"), (c"blue", c"Blue (Slave)"),
    (c"live", c"Live"), (c"disabled", c"Disabled"),
];
pub const ROLES: &[(&CStr, &CStr)] = &[
    (c"master", c"Master"), (c"slave", c"Slave"), (c"disabled", c"Disabled"),
];
pub fn values(set: &str) -> &'static [(&'static CStr, &'static CStr)] {
    if set.starts_with("wingwar") { ROLES } else { COLORS }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset { pub linked: bool, pub index: usize }

pub fn read(env: ffi::Environment, set: &str) -> Option<Preset> {
    let (i, (_, linked_key, max)) = netpacket::GAMES.iter().enumerate().find(|(_, (s, _, _))| *s == set)?;
    let value = option_value(env, KEYS[i]);
    let preset = values(set).iter().position(|(key, _)| Some(key.to_bytes()) == value.as_deref().map(str::as_bytes)).unwrap_or(0);
    if values(set)[preset].0 == c"disabled" { return None; }
    let linked = option_value(env, linked_key).and_then(|v| v.parse::<u16>().ok())
        .is_some_and(|n| (2..=*max).contains(&n));
    Some(Preset { linked, index: preset })
}

pub fn choices(set: &str, preset: Preset) -> Vec<Option<usize>> {
    nvram::FIELDS.iter().filter(|f| f.set == set).map(|f| {
        let key = f.key.to_bytes();
        if key.ends_with(b"_link_id") || key.ends_with(b"_network") {
            let role = if !preset.linked {
                return Some(f.default);
            } else if preset.index == 0 { c"MASTER" }
            else if !set.starts_with("wingwar") && preset.index == 8 { c"LIVE" }
            else { c"SLAVE" };
            return f.values.iter().position(|v| v.key == role);
        }
        if key.ends_with(b"_car_color") || key.ends_with(b"_car_number") {
            if preset.linked && preset.index == 8 { return Some(f.default); }
            return Some(if preset.linked { preset.index } else { f.default });
        }
        None
    }).collect()
}

pub fn publish(env: ffi::Environment, set: &str, choices: &[Option<usize>]) {
    for (f, selected) in nvram::FIELDS.iter().filter(|f| f.set == set).zip(choices) {
        let Some(value) = selected.and_then(|i| f.values.get(i)) else { continue };
        if option_value(env, f.key).as_deref().map(str::as_bytes) == Some(value.key.to_bytes()) { continue; }
        let mut variable = ffi::Variable { key: f.key.as_ptr(), value: value.key.as_ptr() };
        unsafe { env(ffi::SET_VARIABLE, (&mut variable as *mut ffi::Variable).cast()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_presets_patch_only_reviewed_network_fields_and_restore_offline() {
        for (set, _, _) in netpacket::GAMES {
            let (_, initial) = nvram::seed(set).unwrap().unwrap();
            let fields: Vec<_> = nvram::FIELDS.iter().filter(|f| f.set == set).collect();
            for index in 0..values(set).len()-1 {
                let mut image = initial.clone();
                let selected = choices(set, Preset { linked: true, index });
                assert_eq!(selected.iter().flatten().count(), if set.starts_with("wingwar") { 1 } else { 2 });
                nvram::apply_selected(set, &mut image, &selected);
                assert!(nvram::valid(set, &image));
                for (f, chosen) in fields.iter().zip(&selected) {
                    let value = chosen.map(|i| &f.values[i]);
                    for &(offset, _) in f.values[f.default].patch {
                        let expected = value.and_then(|v| v.patch.iter().find(|p| p.0 == offset))
                            .map_or(initial[offset], |p| p.1);
                        assert_eq!(image[offset], expected, "{set}: {}", f.key.to_str().unwrap());
                    }
                }
                let offline = choices(set, Preset { linked: false, index });
                nvram::apply_selected(set, &mut image, &offline);
                assert_eq!(image, initial, "{set}: offline defaults and checksum");
            }
        }
    }
}
