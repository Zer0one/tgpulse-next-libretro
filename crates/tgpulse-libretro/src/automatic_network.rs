//! Framework 0.1.21 native role/identity automation, independent of topology.
use std::ffi::CStr;
use crate::{ffi, netpacket, nvram, option_value};
// Per-set storage retains independent choices. No previous token migration.
pub const KEYS: [&CStr; 6] = [
    c"tgpulse_next_automatic_network_vr", c"tgpulse_next_automatic_network_vformula",
    c"tgpulse_next_automatic_network_wingwar", c"tgpulse_next_automatic_network_wingwaru",
    c"tgpulse_next_automatic_network_wingwarj", c"tgpulse_next_automatic_network_wingwar360",
];
pub const UNSUPPORTED_LINK: &CStr = c"tgpulse_next_linked_cabinets_unsupported";
pub const UNSUPPORTED_ROLE: &CStr = c"tgpulse_next_automatic_network_unsupported";
pub const ROLE_HELP: &CStr = c"Apply the native role and game identity independently of Linked Cabinets, even with NVRAM Settings disabled. Disabled restores manual management. Managed native fields are hidden; unrelated settings are preserved. Effective changes reset automatically when returning to the game.";
pub const LINK_HELP: &CStr = c"Select the total number of linked cabinets, including a Relay where supported. 1 Cabinet / Disabled disconnects optional communication boards or keeps required boards in local loopback; it does not change the selected native role. Supported totals and role combinations depend on the game. Effective changes reset automatically when returning to the game.";
const COLORS: &[(&CStr, &CStr)] = &[
    (c"-1",c"Disabled"),(c"0",c"No Link"),
    (c"1",c"Red (Master)"),(c"2",c"Orange (Slave)"),
    (c"3",c"Skyblue (Slave)"),(c"4",c"Pink (Slave)"),
    (c"5",c"Black (Slave)"),(c"6",c"Green (Slave)"),
    (c"7",c"Yellow (Slave)"),(c"8",c"Blue (Slave)"),
    (c"r",c"Live"),(c"l",c"Live (Alias)"),(c"s",c"Live (Alias)"),
];
const NUMBERS: &[(&CStr, &CStr)] = &[
    (c"-1",c"Disabled"),(c"0",c"No Link"),
    (c"1",c"No.1 (Red) (Master)"),(c"2",c"No.2 (Blue) (Slave)"),
    (c"3",c"No.3 (Yellow) (Slave)"),(c"4",c"No.4 (Green) (Slave)"),
    (c"5",c"No.5 (Black) (Slave)"),(c"6",c"No.6 (Pink) (Slave)"),
    (c"7",c"No.7 (Skyblue) (Slave)"),(c"8",c"No.8 (Orange) (Slave)"),
    (c"r",c"Live"),(c"l",c"Live (Alias)"),(c"s",c"Live (Alias)"),
];
const ROLES: &[(&CStr, &CStr)] = &[
    (c"-1",c"Disabled"),(c"0",c"Stand Alone"),(c"1",c"Master"),(c"2",c"Slave"),
];
pub fn values(set: &str) -> &'static [(&'static CStr, &'static CStr)] {
    if set.starts_with("wingwar") { ROLES }
    else if set == "vformula" { NUMBERS } else { COLORS }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset { Standalone, Cabinet(u8), Relay }
pub fn parse(set: &str, value: &str) -> Result<Option<Preset>, ()> {
    match value {
        "-1" => Ok(None), "0" => Ok(Some(Preset::Standalone)),
        "r" | "l" | "s" if !set.starts_with("wingwar") => Ok(Some(Preset::Relay)),
        _ => match value.parse::<u8>() {
            Ok(id) if (1..=if set.starts_with("wingwar") {2} else {8}).contains(&id)
                && id.to_string() == value => Ok(Some(Preset::Cabinet(id))),
            _ => Err(()),
        },
    }
}
pub fn read(env: ffi::Environment, set: &str) -> Option<Preset> {
    let i = netpacket::GAMES.iter().position(|(s,_,_)| *s == set)?;
    parse(set, option_value(env, KEYS[i]).as_deref().unwrap_or("0"))
        .unwrap_or(Some(Preset::Standalone))
}
pub fn manages(field: &nvram::Field) -> bool {
    netpacket::GAMES.iter().any(|(s,_,_)| *s == field.set)
        && [b"_link_id".as_slice(),b"_network",b"_car_color",b"_car_number"]
            .iter().any(|suffix| field.key.to_bytes().ends_with(suffix))
}
// Publish after releasing CORE: frontend variable callbacks may re-enter us.
pub fn needs_publish(env: ffi::Environment, set: &str, choices: &[Option<usize>]) -> bool {
    nvram::FIELDS.iter().filter(|f| f.set == set).zip(choices).any(|(f,selected)| {
        selected.and_then(|i|f.values.get(i)).is_some_and(|value|
            option_value(env,f.key).as_deref().map(str::as_bytes)!=Some(value.key.to_bytes()))
    })
}
pub fn publish(env: ffi::Environment, set: &str, choices: &[Option<usize>]) {
    for (field,selected) in nvram::FIELDS.iter().filter(|f|f.set==set).zip(choices) {
        let Some(value)=selected.and_then(|i|field.values.get(i)) else {continue};
        if option_value(env,field.key).as_deref().map(str::as_bytes)==Some(value.key.to_bytes()) {continue;}
        let mut variable=ffi::Variable{key:field.key.as_ptr(),value:value.key.as_ptr()};
        unsafe {env(ffi::SET_VARIABLE,(&mut variable as *mut ffi::Variable).cast());}
    }
}
pub fn choices(set: &str, preset: Preset) -> Vec<Option<usize>> {
    nvram::FIELDS.iter().filter(|f| f.set == set).map(|f| {
        if !manages(f) { return None; }
        let key=f.key.to_bytes();
        if key.ends_with(b"_link_id") || key.ends_with(b"_network") {
            let role=match preset {
                Preset::Standalone => return Some(f.default),
                Preset::Cabinet(1) => c"MASTER", Preset::Cabinet(_) => c"SLAVE",
                Preset::Relay => c"LIVE",
            };
            return f.values.iter().position(|v| v.key==role);
        }
        match preset {
            // VF's native table follows color encoding rather than car number.
            Preset::Cabinet(id) if set=="vformula" => {
                let prefix=format!("NO.{id} (");
                f.values.iter().position(|v| v.key.to_bytes().starts_with(prefix.as_bytes()))
            }
            Preset::Cabinet(id) => Some(usize::from(id-1)),
            Preset::Standalone | Preset::Relay => Some(f.default),
        }
    }).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approved_tokens_and_native_number_order() {
        for (set,_,_) in netpacket::GAMES {
            assert_eq!(parse(set,"-1"),Ok(None));
            assert_eq!(parse(set,"0"),Ok(Some(Preset::Standalone)));
            for old in ["disabled","master","slave","red","relay","live","99",""] {
                assert!(parse(set,old).is_err());
            }
        }
        for alias in ["r","l","s"] {
            assert_eq!(parse("vr",alias),Ok(Some(Preset::Relay)));
            assert!(parse("wingwar",alias).is_err());
        }
        let fields:Vec<_>=nvram::FIELDS.iter().filter(|f|f.set=="vformula").collect();
        let chosen=choices("vformula",Preset::Cabinet(2));
        let id=fields.iter().position(|f|f.key.to_bytes().ends_with(b"_car_number")).unwrap();
        assert_eq!(fields[id].values[chosen[id].unwrap()].key,c"NO.2 (BLUE)");
    }
    #[test]
    fn every_role_preserves_native_integrity_and_offline_roundtrip() {
        for (set,_,_) in netpacket::GAMES {
            let (_,initial)=nvram::seed(set).unwrap().unwrap();
            for (raw,_) in values(set) {
                let Some(preset)=parse(set,raw.to_str().unwrap()).unwrap() else {continue};
                let selected=choices(set,preset);
                assert_eq!(selected.iter().flatten().count(),if set.starts_with("wingwar"){1}else{2});
                let mut image=initial.clone();
                nvram::apply_selected(set,&mut image,&selected);
                assert!(nvram::valid(set,&image));
                nvram::apply_selected(set,&mut image,&choices(set,Preset::Standalone));
                assert_eq!(image,initial);
            }
        }
    }
}
