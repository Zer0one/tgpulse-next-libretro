//! NetMerc substitute PCM resources. No changes to its sound program, ROM
//! archives, chip implementation or mixer; no host paths in device state.
use super::Model1Roms;
use crate::{config::NetmercAudioDonor, model1board::Kind, roms_db};
use std::{fs::File, path::Path};
use zip::ZipArchive;

/// Frontend-owned resources. A Libretro frontend may provide these from memory
/// instead of using the optional ZIP helper below.
pub struct SampleBanks {
    pub pcm1: Vec<u8>,
    pub pcm2: Vec<u8>,
}

impl SampleBanks {
    /// Transactional, load-time application only. The existing machine resource
    /// hash includes both banks and rejects states from another donor.
    pub fn apply(self, roms: &mut Model1Roms) -> Result<(), String> {
        if roms.ioboard_kind != Kind::NetMerc {
            return Err("audio donors are only supported for NetMerc".into());
        }
        for bank in [&self.pcm1, &self.pcm2] {
            if bank.len() != 0x400000 || bank.iter().all(|&b| b == 0 || b == 0xff) {
                return Err("invalid or blank donor MultiPCM bank".into());
            }
        }
        roms.mpcm1 = self.pcm1;
        roms.mpcm2 = self.pcm2;
        roms.netmerc_procedural_audio = false;
        Ok(())
    }
}

/// Only sample regions are needed: no donor CPU, graphics, BIOS or DSB ROMs.
pub fn load_zip(path: &Path, donor: NetmercAudioDonor) -> Result<SampleBanks, String> {
    if donor == NetmercAudioDonor::Off {
        return Err("Off has no donor sample banks".into());
    }
    let def = roms_db::named_game(donor.as_str()).ok_or("unknown donor set")?;
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut regions = roms_db::build_selected_regions(
        def,
        &mut archive,
        Some(&["m1audio:pcm1", "m1audio:pcm2"]),
    )?;
    Ok(SampleBanks {
        pcm1: regions
            .remove("m1audio:pcm1")
            .ok_or("missing PCM1 region")?,
        pcm2: regions
            .remove("m1audio:pcm2")
            .ok_or("missing PCM2 region")?,
    })
}

/// Desktop policy: look beside the selected game, not in the process cwd.
/// Off and other boards leave the original resources entirely unchanged.
pub fn apply_adjacent(
    roms: &mut Model1Roms,
    game_path: &Path,
    donor: NetmercAudioDonor,
) -> Result<bool, String> {
    if roms.ioboard_kind != Kind::NetMerc || donor == NetmercAudioDonor::Off {
        return Ok(false);
    }
    let path = game_path.with_file_name(format!("{}.zip", donor.as_str()));
    load_zip(&path, donor)?.apply(roms)?;
    log::info!(target: "audio", "NetMerc substitute audio: {} ({})", donor.label(), path.display());
    Ok(true)
}
