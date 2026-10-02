//! Fixed frontend buffers around the existing Model 1 in-memory formats.
use tgpulse_core::model1::MAX_STATE_BYTES;

pub const SAVE_HEADER: usize = 64;
pub const BACKUP_BYTES: usize = 0x10000;
pub const EEPROM_BYTES: usize = 128;
pub const SAVE_BYTES: usize = SAVE_HEADER + BACKUP_BYTES + EEPROM_BYTES;
pub const STATE_HEADER: usize = 32;
pub const STATE_BYTES: usize = STATE_HEADER + MAX_STATE_BYTES;
const SAVE_MAGIC: &[u8; 8] = b"TGP1SRAM";
const STATE_MAGIC: &[u8; 8] = b"TGP1LRST";

fn checksum(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub fn export_save(set: &str, backup: &[u8], eeprom: &[u8], target: &mut [u8]) -> bool {
    if target.len() != SAVE_BYTES
        || backup.len() != BACKUP_BYTES
        || eeprom.len() != EEPROM_BYTES
        || set.is_empty()
        || set.len() >= 32
    {
        return false;
    }
    target.fill(0);
    target[..8].copy_from_slice(SAVE_MAGIC);
    target[8..12].copy_from_slice(&1u32.to_le_bytes());
    target[12..16].copy_from_slice(&(BACKUP_BYTES as u32).to_le_bytes());
    target[16..20].copy_from_slice(&(EEPROM_BYTES as u32).to_le_bytes());
    target[24..24 + set.len()].copy_from_slice(set.as_bytes());
    target[SAVE_HEADER..SAVE_HEADER + BACKUP_BYTES].copy_from_slice(backup);
    target[SAVE_HEADER + BACKUP_BYTES..].copy_from_slice(eeprom);
    let crc = checksum(&target[SAVE_HEADER..]);
    target[20..22].copy_from_slice(&crc.to_le_bytes());
    true
}

pub fn import_save<'a>(
    set: &str,
    source: &'a [u8],
) -> Result<Option<(&'a [u8], &'a [u8])>, &'static str> {
    if source.len() != SAVE_BYTES {
        return Err("Save RAM size mismatch");
    }
    if source.iter().all(|&value| value == 0) {
        return Ok(None);
    }
    if &source[..8] != SAVE_MAGIC
        || source[8..12] != 1u32.to_le_bytes()
        || source[12..16] != (BACKUP_BYTES as u32).to_le_bytes()
        || source[16..20] != (EEPROM_BYTES as u32).to_le_bytes()
    {
        return Err("Invalid Model 1 Save RAM header");
    }
    let stored = &source[24..56];
    let end = stored
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(stored.len());
    if stored[..end] != *set.as_bytes() {
        return Err("Save RAM belongs to another set");
    }
    if u16::from_le_bytes(source[20..22].try_into().unwrap()) != checksum(&source[SAVE_HEADER..]) {
        return Err("Save RAM checksum mismatch");
    }
    Ok(Some((
        &source[SAVE_HEADER..SAVE_HEADER + BACKUP_BYTES],
        &source[SAVE_HEADER + BACKUP_BYTES..],
    )))
}

pub fn export_state(
    machine: &[u8],
    controls: [i32; 3],
    audio_remainder: u64,
    target: &mut [u8],
) -> bool {
    if target.len() != STATE_BYTES
        || machine.len() > MAX_STATE_BYTES - 8
        || audio_remainder >= 224 * 60
    {
        return false;
    }
    target.fill(0);
    target[..8].copy_from_slice(STATE_MAGIC);
    target[8..12].copy_from_slice(&2u32.to_le_bytes());
    target[12..20].copy_from_slice(&(machine.len() as u64).to_le_bytes());
    for (index, value) in controls.into_iter().enumerate() {
        target[20 + 4 * index..24 + 4 * index].copy_from_slice(&value.to_le_bytes());
    }
    target[STATE_HEADER..STATE_HEADER + machine.len()].copy_from_slice(machine);
    target[STATE_HEADER + machine.len()..STATE_HEADER + machine.len() + 8]
        .copy_from_slice(&audio_remainder.to_le_bytes());
    true
}

pub fn import_state(source: &[u8]) -> Result<(&[u8], [i32; 3], u64), &'static str> {
    if source.len() != STATE_BYTES
        || &source[..8] != STATE_MAGIC
        || !matches!(u32::from_le_bytes(source[8..12].try_into().unwrap()), 1 | 2)
    {
        return Err("Invalid Libretro state header");
    }
    let len = u64::from_le_bytes(source[12..20].try_into().unwrap());
    if len > MAX_STATE_BYTES as u64 || len == 0 {
        return Err("Invalid Libretro state length");
    }
    let controls = std::array::from_fn(|index| {
        i32::from_le_bytes(source[20 + 4 * index..24 + 4 * index].try_into().unwrap())
    });
    if controls
        .iter()
        .any(|&value| !(0x20..=0xe0).contains(&value))
    {
        return Err("Invalid saved control position");
    }
    let remainder = if source[8..12] == 1u32.to_le_bytes() {
        0
    } else {
        if len > (MAX_STATE_BYTES - 8) as u64 {
            return Err("Invalid Libretro timing state length");
        }
        let start = STATE_HEADER + len as usize;
        let remainder = u64::from_le_bytes(source[start..start + 8].try_into().unwrap());
        if remainder >= 224 * 60 {
            return Err("Invalid Libretro timing state");
        }
        remainder
    };
    Ok((
        &source[STATE_HEADER..STATE_HEADER + len as usize],
        controls,
        remainder,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_ram_rejects_wrong_set_and_corruption() {
        let mut bytes = vec![0; SAVE_BYTES];
        assert!(import_save("vr", &bytes).unwrap().is_none());
        assert!(export_save(
            "vr",
            &vec![0xa5; BACKUP_BYTES],
            &vec![0x5a; EEPROM_BYTES],
            &mut bytes
        ));
        let (backup, eeprom) = import_save("vr", &bytes).unwrap().unwrap();
        assert_eq!(backup[0], 0xa5);
        assert_eq!(eeprom[0], 0x5a);
        assert!(import_save("vf", &bytes).is_err());
        bytes[SAVE_HEADER] ^= 1;
        assert!(import_save("vr", &bytes).is_err());
    }

    #[test]
    fn state_envelope_roundtrip_and_length_check() {
        let mut bytes = vec![0; STATE_BYTES];
        assert!(export_state(b"sample", [128, 32, 33], 12345, &mut bytes));
        assert_eq!(
            import_state(&bytes).unwrap(),
            (&b"sample"[..], [128, 32, 33], 12345)
        );
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(import_state(&bytes).unwrap().2, 0);
        bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        bytes[STATE_HEADER + 6..STATE_HEADER + 14].copy_from_slice(&(224u64 * 60).to_le_bytes());
        assert!(import_state(&bytes).is_err());
        bytes[STATE_HEADER + 6..STATE_HEADER + 14].copy_from_slice(&12345u64.to_le_bytes());
        bytes[12..20].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(import_state(&bytes).is_err());
    }
}
