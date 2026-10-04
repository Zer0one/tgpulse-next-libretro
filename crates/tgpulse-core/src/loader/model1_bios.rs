//! Ordered, validated Model 1 device resources. The caller supplies host paths;
//! no frontend ABI, device state or persistent configuration belongs here.
use std::path::{Path, PathBuf};
use sha1::{Digest, Sha1};

pub const LCD_SHA1: &str = "65cf075a988cdcbb316b9afdd0529b374a1a65ec";

pub fn load_resource(game: &Path, system: Option<&Path>, archive: &str,
                     chip: &str, size: usize, sha1: &str) -> Option<Vec<u8>> {
    let mut paths: Vec<PathBuf> = system.map(|root| root.join(archive)).into_iter().collect();
    paths.push(game.with_file_name(archive));
    paths.push(game.to_path_buf());
    for path in paths {
        let Ok(file) = std::fs::File::open(&path) else { continue };
        let Ok(mut zip) = zip::ZipArchive::new(file) else { continue };
        let Ok(mut entry) = zip.by_name(chip) else { continue };
        // Do not allocate unbounded data from an invalid ZIP member.
        if entry.size() != size as u64 { continue }
        let mut image = Vec::with_capacity(size);
        if std::io::Read::read_to_end(&mut entry, &mut image).is_err()
            || image.len() != size || format!("{:x}", Sha1::digest(&image)) != sha1 {
            log::warn!(target: "loader", "{}: invalid {chip}; trying next BIOS location", path.display());
            continue;
        }
        log::info!(target: "loader", "{chip} from {}", path.display());
        return Some(image);
    }
    None
}

pub fn load_lcd(game: &Path, system: Option<&Path>) -> Option<Box<[u8; 4096]>> {
    load_resource(game, system, "hd44780.zip", "hd44780_a00.bin", 4096, LCD_SHA1)?
        .into_boxed_slice().try_into().ok()
}

pub fn load_io(game: &Path, system: Option<&Path>, definition: &crate::roms_db::GameDef)
    -> Result<Vec<u8>, String> {
    let chip = definition.model1_io_file().ok_or("Model 1 game has no I/O firmware declaration")?;
    let (archive, sha1) = match chip {
        "epr-14869.25" => ("model1io.zip", "b65fdd0ad31794a565a0ca4dc67a3f16b329fd71"),
        "epr-14869b.25" => ("model1io.zip", "af0fe245eb9fa3c3c60e4b685f1e779f83d894f9"),
        "epr-16891.6" => ("model1io2.zip", "3079397c7241c1a6f494fa310faff0989dfa04a0"),
        "epr-18021.6" => ("model1io2.zip", "bf5b9aad99c0f8f5e262e0855796f39119d11a97"),
        _ => return Err(format!("Unsupported Model 1 I/O firmware: {chip}")),
    };
    load_resource(game, system, archive, chip, 65536, sha1)
        .ok_or_else(|| format!("Required I/O BIOS {archive}: missing or invalid {chip}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_then_adjacent_then_game_and_invalid_candidates_are_skipped() {
        use std::io::Write;
        let root = std::env::temp_dir().join(format!("tgpulse-bios-{}", std::process::id()));
        std::fs::create_dir_all(root.join("system")).unwrap();
        let image = b"synthetic device resource";
        let hash = format!("{:x}", Sha1::digest(image));
        let zip = |path: &Path, bytes: &[u8]| {
            let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
            writer.start_file("chip.bin", zip::write::FileOptions::default()).unwrap();
            writer.write_all(bytes).unwrap(); writer.finish().unwrap();
        };
        let game = root.join("game.zip");
        let system = root.join("system");
        let sys = system.join("device.zip");
        let adjacent = root.join("device.zip");
        let load = || load_resource(&game, Some(&system), "device.zip", "chip.bin", image.len(), &hash);
        // Each valid candidate must suffice even when every later path is absent.
        zip(&sys, image); assert_eq!(load().as_deref(), Some(image.as_slice()));
        std::fs::remove_file(&sys).unwrap();
        zip(&adjacent, image); assert_eq!(load().as_deref(), Some(image.as_slice()));
        zip(&sys, &[0; 25]); // Wrong size/hash cannot suppress a valid later resource.
        assert_eq!(load().as_deref(), Some(image.as_slice()));
        std::fs::remove_file(&adjacent).unwrap(); zip(&game, image);
        assert_eq!(load().as_deref(), Some(image.as_slice()));
        std::fs::remove_file(&game).unwrap(); assert!(load().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
