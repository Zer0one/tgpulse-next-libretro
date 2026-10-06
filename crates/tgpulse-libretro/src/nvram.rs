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

/// Backup-RAM settings have no EEPROM checksum or mirror policy.
pub struct BackupTemplate {
    pub set: &'static str,
    pub encoded: &'static [u8],
    pub payload_crc: u16,
    pub startup: &'static [(usize, u8)],
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
    template(set).is_some() || data::BACKUP_TEMPLATES.iter().any(|t| t.set == set)
}

/// NetMerc controller endpoint slots, verified against Service Menu evidence.
/// Apply only on first initialization; MVD tracking calibration is separate.
pub fn initial_calibration(set: &str, backup: &mut [u8]) -> bool {
    if set != "netmerc" || backup.len() != 65536 {
        return false;
    }
    for (offset, value) in [(0x3c, 0xff), (0x38, 0x00), (0x40, 0x00), (0x44, 0xff)] {
        backup[offset..offset + 4].fill(0xff);
        backup[offset] = value;
    }
    true
}

#[cfg(test)]
#[test]
fn initial_netmerc_calibration_preserves_unrelated_sram() {
    let mut backup = vec![0x5a; 65536];
    assert!(initial_calibration("netmerc", &mut backup));
    for offset in 0..backup.len() {
        let value = match offset {
            0x38 | 0x40 => 0,
            0x39..=0x3f | 0x41..=0x47 => 0xff,
            _ => 0x5a,
        };
        assert_eq!(backup[offset], value, "SRAM offset {offset:#x}");
    }
    let expected = backup.clone();
    assert!(!initial_calibration("vf", &mut backup));
    assert_eq!(backup, expected);
    assert!(!initial_calibration("netmerc", &mut backup[..128]));
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

fn decode(encoded: &[u8], payload_crc: u16) -> Result<Vec<u8>, &'static str> {
    let mut payload = Vec::with_capacity(65536 + 128);
    let mut input = 0;
    while input < encoded.len() {
        let command = encoded[input];
        input += 1;
        let count = usize::from(command & 0x7f) + 1;
        if payload.len() + count > 65536 + 128 {
            return Err("Invalid initial NVRAM template");
        }
        if command & 0x80 != 0 {
            let Some(&byte) = encoded.get(input) else {
                return Err("Truncated initial NVRAM template");
            };
            payload.resize(payload.len() + count, byte);
            input += 1;
        } else {
            let Some(bytes) = encoded.get(input..input + count) else {
                return Err("Truncated initial NVRAM template");
            };
            payload.extend_from_slice(bytes);
            input += count;
        }
    }
    if payload.len() != 65536 + 128 || checksum(payload.iter().copied(), 0) != payload_crc {
        return Err("Initial NVRAM template checksum mismatch");
    }
    Ok(payload)
}

/// Complete set-specific backup/EEPROM image, followed only by approved startup patches.
pub fn seed(set: &str) -> Result<Option<(Vec<u8>, Vec<u8>)>, &'static str> {
    let Some(t) = template(set) else {
        return Ok(None);
    };
    let mut payload = decode(t.encoded, t.payload_crc)?;
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
    let selected: Vec<_> = FIELDS.iter().filter(|f| f.set == set).enumerate()
        .map(|(i, f)| Some(choices.get(i).copied().unwrap_or(f.default))).collect();
    apply_selected(set, eeprom, &selected)
}

/// None leaves a field untouched; use the same integrity policy as manual settings.
pub fn apply_selected(set: &str, eeprom: &mut [u8], choices: &[Option<usize>]) -> Apply {
    if !valid(set, eeprom) {
        return Apply::NotReady;
    }
    let t = template(set).unwrap();
    let before = eeprom.to_vec();
    for (index, field) in FIELDS.iter().filter(|f| f.set == set).enumerate() {
        let Some(chosen) = choices.get(index).copied().flatten() else { continue };
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

/// New saves: patch the optional factory image, or initialize from the native
/// campaign baseline when no factory image was loaded. Existing saves skip this.
pub fn initial_backup(set: &str, backup: &mut [u8], factory_loaded: bool)
    -> Result<bool, &'static str> {
    let Some(t) = data::BACKUP_TEMPLATES.iter().find(|t| t.set == set) else {
        return Ok(false);
    };
    if backup.len() != 65536 { return Err("Invalid backup RAM size"); }
    if !factory_loaded {
        let payload = decode(t.encoded, t.payload_crc)?;
        backup.copy_from_slice(&payload[..65536]);
    }
    for &(offset, byte) in t.startup { backup[offset] = byte; }
    initial_calibration(set, backup);
    Ok(true)
}

/// Apply the same reviewed field table to the set's actual storage.
pub fn apply_settings(set: &str, backup: &mut [u8], eeprom: &mut [u8], choices: &[usize]) -> Apply {
    if set != "netmerc" { return apply(set, eeprom, choices); }
    // NetMerc patches the complete supplied backup image. Bookkeeping bytes
    // are not an operator-layout signature and must neither gate nor be repaired.
    if backup.len() != 65536 {
        return Apply::NotReady;
    }
    let mut changed = false;
    for (index, field) in FIELDS.iter().filter(|f| f.set == set).enumerate() {
        let value = field.values.get(choices.get(index).copied().unwrap_or(field.default))
            .unwrap_or(&field.values[field.default]);
        for &(offset, byte) in value.patch {
            changed |= backup[offset] != byte;
            backup[offset] = byte;
        }
    }
    if changed { Apply::Changed } else { Apply::Unchanged }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn netmerc_backup_initialization_overrides_and_preservation() {
        let mut backup = vec![0; 65536];
        assert!(initial_backup("netmerc", &mut backup, false).unwrap());
        assert_eq!(&backup[..4], &[0x0f, 0xff, 0xff, 0xff]);
        assert_eq!((backup[0x5c], backup[0x28], backup[0x2a], backup[0x2c]), (2, 2, 0, 1));
        assert_eq!([backup[0x3c],backup[0x38],backup[0x40],backup[0x44]], [255,0,0,255]);
        let mut factory = vec![0x5a; 65536];
        assert!(initial_backup("netmerc", &mut factory, true).unwrap());
        for i in 0..factory.len() {
            if i != 0x5c && !(0x38..0x48).contains(&i) { assert_eq!(factory[i], 0x5a); }
        }
        let fields: Vec<_> = FIELDS.iter().filter(|f| f.set == "netmerc").collect();
        let mut choices = vec![0; fields.len()];
        let mut eeprom = vec![0xff; 128];
        apply_settings("netmerc", &mut backup, &mut eeprom, &choices);
        for (index, field) in fields.iter().enumerate() {
            for (step, value) in field.values.iter().enumerate() {
                let mut trial = backup.clone(); choices[index] = step;
                assert_ne!(apply_settings("netmerc", &mut trial, &mut eeprom, &choices), Apply::NotReady);
                for &(offset, byte) in value.patch { assert_eq!(trial[offset], byte); }
                for i in 0..trial.len() {
                    if !value.patch.iter().any(|p| p.0 == i) { assert_eq!(trial[i], backup[i]); }
                }
                assert_eq!(eeprom, vec![0xff;128]);
            }
            choices[index] = 0;
        }
        assert_eq!(apply_settings("netmerc", &mut [0; 128], &mut eeprom, &choices), Apply::NotReady);
        for prefix in [[0;4], [0xff;4], [0x0f,0xff,0xff,0xff]] {
            let mut image = vec![0xff;65536];
            image[..4].copy_from_slice(&prefix);
            let before = image.clone();
            assert_eq!(apply_settings("netmerc", &mut image, &mut eeprom, &choices), Apply::Changed);
            assert_eq!(&image[..4], &prefix);
            for offset in 0..image.len() {
                if !fields.iter().any(|f| f.values[f.default].patch.iter().any(|p| p.0 == offset)) {
                    assert_eq!(image[offset], before[offset]);
                }
            }
        }
    }
    #[test]
    fn nine_distinct_templates_valid_startup_and_preservation() {
        assert_eq!(data::TEMPLATES.len(), 9);
        assert_eq!(FIELDS.len(), 43);
        assert!(supported("netmerc"));
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
