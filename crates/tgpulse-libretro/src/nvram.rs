//! Reviewed operator fields and complete initial images in the frontend adapter.
//! Algorithms/byte order follow the saved/reloaded Model 1 campaign, not Model 2.
use std::ffi::CStr;

#[path = "nvram_data.rs"]
mod data;
pub use data::FIELDS;

pub struct Value {
    pub key: &'static CStr,
    pub label: &'static CStr,
    pub patch: &'static [(usize, u8)],
}
pub struct Field {
    pub set: &'static str,
    pub key: &'static CStr,
    pub label: &'static CStr,
    pub default: usize,
    pub values: &'static [Value],
}
pub struct Layout {
    pub start: usize,
    pub end: usize,
    pub initial: u16,
    pub swap: bool,
    pub big: bool,
    pub mirror: bool,
}
pub struct Template {
    pub set: &'static str,
    pub encoded: &'static [u8],
    pub payload_crc: u16,
    pub startup: &'static [(usize, u8)],
    pub layout: Layout,
}

fn checksum(bytes: impl IntoIterator<Item = u8>, initial: u16) -> u16 {
    let mut crc = initial;
    for byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = (crc << 1) ^ if crc & 0x8000 != 0 { 0x1021 } else { 0 };
        }
    }
    crc
}

fn template(set: &str) -> Option<&'static Template> {
    data::TEMPLATES.iter().find(|t| t.set == set)
}
pub fn supported(set: &str) -> bool {
    template(set).is_some()
}

fn repair(image: &mut [u8], layout: &Layout) {
    let crc = checksum(
        (layout.start..layout.end).map(|i| image[if layout.swap { i ^ 1 } else { i }]),
        layout.initial,
    );
    image[8..10].copy_from_slice(&if layout.big {
        crc.to_be_bytes()
    } else {
        crc.to_le_bytes()
    });
    if layout.mirror {
        image.copy_within(6..60, 66);
    }
}

pub fn valid(set: &str, image: &[u8]) -> bool {
    let Some(t) = template(set) else { return false };
    if image.len() != 128 {
        return false;
    }
    let mut fixed = image.to_vec();
    repair(&mut fixed, &t.layout);
    fixed == image
}

/// Complete set-specific backup/EEPROM image, followed only by approved startup patches.
pub fn seed(set: &str) -> Result<Option<(Vec<u8>, Vec<u8>)>, &'static str> {
    let Some(t) = template(set) else {
        return Ok(None);
    };
    let mut payload = Vec::with_capacity(65536 + 128);
    let mut input = 0;
    while input < t.encoded.len() {
        let command = t.encoded[input];
        input += 1;
        let count = usize::from(command & 0x7f) + 1;
        if payload.len() + count > 65536 + 128 {
            return Err("Invalid initial NVRAM template");
        }
        if command & 0x80 != 0 {
            let Some(&byte) = t.encoded.get(input) else {
                return Err("Truncated initial NVRAM template");
            };
            payload.resize(payload.len() + count, byte);
            input += 1;
        } else {
            let Some(bytes) = t.encoded.get(input..input + count) else {
                return Err("Truncated initial NVRAM template");
            };
            payload.extend_from_slice(bytes);
            input += count;
        }
    }
    if payload.len() != 65536 + 128 || checksum(payload.iter().copied(), 0) != t.payload_crc {
        return Err("Initial NVRAM template checksum mismatch");
    }
    let mut eeprom = payload.split_off(65536);
    if !valid(set, &eeprom) {
        return Err("Initial NVRAM native integrity mismatch");
    }
    for &(offset, byte) in t.startup {
        eeprom[offset] = byte;
    }
    repair(&mut eeprom, &t.layout);
    Ok(Some((payload, eeprom)))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Apply {
    NotReady,
    Unchanged,
    Changed,
}

/// Patch only reviewed fields and their native integrity/mirror bytes.
pub fn apply(set: &str, eeprom: &mut [u8], choices: &[usize]) -> Apply {
    if !valid(set, eeprom) {
        return Apply::NotReady;
    }
    let t = template(set).unwrap();
    let before = eeprom.to_vec();
    for (index, field) in FIELDS.iter().filter(|f| f.set == set).enumerate() {
        let chosen = choices.get(index).copied().unwrap_or(field.default);
        let value = field
            .values
            .get(chosen)
            .unwrap_or(&field.values[field.default]);
        for &(offset, byte) in value.patch {
            eeprom[offset] = byte;
        }
    }
    repair(eeprom, &t.layout);
    if before == eeprom {
        Apply::Unchanged
    } else {
        Apply::Changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nine_distinct_templates_valid_startup_and_preservation() {
        assert_eq!(data::TEMPLATES.len(), 9);
        assert_eq!(FIELDS.len(), 39);
        assert!(!supported("netmerc"));
        for t in data::TEMPLATES {
            let (backup, mut eeprom) = seed(t.set).unwrap().unwrap();
            assert_eq!(backup.len(), 65536);
            assert!(valid(t.set, &eeprom));
            for &(offset, byte) in t.startup {
                assert_eq!(eeprom[offset], byte);
            }
            let fields: Vec<_> = FIELDS.iter().filter(|f| f.set == t.set).collect();
            let mut choices: Vec<_> = fields.iter().map(|f| f.default).collect();
            apply(t.set, &mut eeprom, &choices);
            let baseline = eeprom.clone();
            for (index, field) in fields.iter().enumerate() {
                for (step, value) in field.values.iter().enumerate() {
                    eeprom.clone_from(&baseline);
                    choices[index] = step;
                    apply(t.set, &mut eeprom, &choices);
                    assert!(valid(t.set, &eeprom));
                    for &(offset, byte) in value.patch {
                        assert_eq!(eeprom[offset], byte);
                    }
                    for offset in 0..128 {
                        let allowed = offset == 8
                            || offset == 9
                            || value.patch.iter().any(|p| p.0 == offset)
                            || (t.layout.mirror && (66..120).contains(&offset));
                        if !allowed {
                            assert_eq!(eeprom[offset], baseline[offset], "{}:{}", t.set, offset);
                        }
                    }
                }
                choices[index] = field.default;
            }
            let mut invalid = [0xff; 128];
            assert_eq!(apply(t.set, &mut invalid, &choices), Apply::NotReady);
            assert_eq!(invalid, [0xff; 128]);
        }
        let (_, vr) = seed("vr").unwrap().unwrap();
        assert_eq!([vr[0x1a], vr[0x0d], vr[0x1b]], [2, 0, 0]);
        assert_ne!(vr, seed("vformula").unwrap().unwrap().1);
        assert_ne!(
            seed("swa").unwrap().unwrap().1,
            seed("swaj").unwrap().unwrap().1
        );
    }
}
