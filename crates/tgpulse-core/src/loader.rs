//! ROM loading for Sega Model 2: which chip goes where in each ROM region,
//! and how the interleaved ones are woven together.

#[cfg(feature = "model1")]
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

/// Optional diagnostic CGROM: game archive first, then adjacent MAME device BIOS.
/// Not required to boot and never part of mutable machine snapshots.
#[cfg(feature = "model1")]
pub fn load_hd44780_font(rom_path: &str) -> Option<Box<[u8; 4096]>> {
    let bios = std::path::Path::new(rom_path).with_file_name("hd44780.zip");
    for path in [std::path::Path::new(rom_path), bios.as_path()] {
        let Ok(file) = File::open(path) else { continue };
        let mut archive = match ZipArchive::new(file) {
            Ok(archive) => archive,
            Err(error) => {
                log::warn!(target: "loader", "{}: diagnostic font archive: {error}", path.display());
                continue;
            }
        };
        let Ok(mut file) = archive.by_name("hd44780_a00.bin") else {
            continue;
        };
        // MAME's original HD44780 A00, reconstructed from the 1985 datasheet;
        // not the different HD44780U font or an authenticated silicon dump.
        let mut image = Vec::new();
        if file.size() != 4096
            || file.read_to_end(&mut image).is_err()
            || Sha1::digest(&image)[..]
                != [
                    0x65, 0xcf, 0x07, 0x5a, 0x98, 0x8c, 0xdc, 0xbb, 0x31, 0x6b, 0x9a, 0xfd, 0xd0,
                    0x52, 0x9b, 0x37, 0x4a, 0x1a, 0x65, 0xec,
                ]
        {
            log::warn!(target: "loader", "{}: invalid hd44780_a00.bin; trying diagnostic font fallback", path.display());
            continue;
        }
        log::info!(target: "loader", "HD44780 A00 diagnostic font from {}", path.display());
        return image.into_boxed_slice().try_into().ok();
    }
    log::info!(target: "loader", "HD44780 diagnostic font unavailable; using Text rendering");
    None
}

/// MAME #15649: repair only the fully identified old 315-5711 dump, never
/// a different program or an unknown/corrupted revision. ZIPs remain untouched.
#[cfg(feature = "model1")]
fn repair_315_5711(program: &mut [u8]) -> bool {
    const OLD_SHA1: [u8; 20] = [
        0x9e, 0x21, 0xd3, 0xa0, 0x7f, 0xfa, 0x31, 0x5e, 0x01, 0x39, 0x48, 0x3b, 0x66, 0x4e, 0x3f,
        0xa2, 0x83, 0xef, 0x4e, 0x06,
    ];
    if program.len() != 0x2000 || Sha1::digest(&*program)[..] != OLD_SHA1 {
        return false;
    }
    // PC 0x8b: branch 0x59e -> 0x59c; PC 0x67e: lia #2 -> lia #0.
    program[0x8b * 4] &= !2;
    program[0x67e * 4] &= !2;
    log::info!(target: "loader", "315-5711: repaired legacy bad dump in memory (MAME #15649)");
    true
}

/// Initialize only the known NetMerc ROM-set default, never persistent SRAM.
/// The ZIP and all bytes outside bookkeeping and the credit word stay unchanged.
#[cfg(feature = "model1")]
fn repair_netmerc_nvram_default(image: &mut [u8]) -> bool {
    const DEFAULT_SHA1: [u8; 20] = [
        0x41, 0x11, 0x34, 0xc1, 0xe6, 0x30, 0x7f, 0x2e, 0x32, 0xc3, 0xb4, 0xb3, 0x72, 0x59, 0x7b,
        0x45, 0xb1, 0x4a, 0x98, 0x34,
    ];
    if image.len() != 0x10000 || Sha1::digest(&*image)[..] != DEFAULT_SHA1 {
        return false;
    }
    // epr-18120.ic5, FDD4F3: zero both 4 KiB bookkeeping banks and
    // the checksum, then select the first bank. Preserve calibration and
    // other SRAM fields; initialize the erased credit word as the native
    // clear does, without executing the full service clear.
    image[0x1000..0x3004].fill(0);
    image[0] = 0x0f;
    image[0x18..0x1a].fill(0);
    log::info!(target: "loader", "NetMerc: initialized ROM-set default bookkeeping and credits in memory; calibration preserved");
    true
}

/// Initialize the recognized factory seed only when the caller needs it.
/// Persistent saves and state/reset paths must never call this operation.
#[cfg(feature = "model1")]
pub fn initialize_netmerc_nvram_seed(image: &mut [u8]) -> bool {
    repair_netmerc_nvram_default(image)
}

/// The ROM regions the i960 and the TGP see. Each is a byte image of a the reference
/// ROM_REGION, already interleaved, indexed by region-relative byte offset.
#[cfg(feature = "model2")]
pub struct Roms {
    /// "maincpu": i960 program, 2MB region (only the first 256KB is populated).
    pub maincpu: Vec<u8>,
    /// "main_data": 32MB data region, visible at 0x02000000 and 0x06000000.
    pub main_data: Vec<u8>,
    /// "copro_data": 8MB coprocessor data (collision / height maps).
    pub copro_data: Vec<u8>,
    /// CPU-board lookup ROMs used by the TGP math ports.
    pub copro_tables: Vec<u8>,
    /// Geometry engine model ROM, 16MB.
    pub polygons: Vec<u8>,
    /// Rasterizer texture ROM, 16MB (half populated on Daytona).
    pub textures: Vec<u8>,
    /// Factory contents of the serial EEPROM, when the romset ships one.
    /// The reference provides this for the handful of sets whose cabinet configuration
    /// cannot be reached from the game's own menus -- Manx TT's DX and twin
    /// modes, for instance.
    pub eeprom: Vec<u8>,

    // --- Model 1 sound board (segam1audio) ---
    /// "m1audio:sndcpu": the board's 68000 program, 768KB region.
    ///
    /// The reference declares this ROMREGION_BE|ROMREGION_16BIT and loads it with
    /// ROM_LOAD16_WORD_SWAP, i.e. the bytes of every 16-bit word are exchanged
    /// on the way in. Doing that is what makes the reset vectors read back as a
    /// stack pointer at the top of the board's RAM and an entry point inside
    /// the ROM; without it both are garbage.
    pub sndcpu: Vec<u8>,
    /// "m1audio:pcm1"/"pcm2": 4MB of samples for each MultiPCM.
    pub mpcm1: Vec<u8>,
    pub mpcm2: Vec<u8>,
    /// True on Model 2A (SCSP sound board). `sndcpu` is then the SCSP board's
    /// 68000 program and `mpcm1`+`mpcm2` concatenated form the 8MB "samples"
    /// region (already word-swapped), banked into the top of the board's map.
    pub sound_scsp: bool,
    /// The geometry coprocessor the board carries. The original and 2A boards
    /// use the MB86234 TGP (fully emulated); 2B uses the ADSP-21062 SHARC.
    pub coprocessor: crate::roms_db::Board,
    /// Air Walkers selects players 1/2 or 3/4 through port F bit 7.
    pub airwalkers_matrix: bool,
}

/// Copies a chip in with the byte order of each 16-bit word exchanged.
pub(crate) fn load16_word_swap(dest: &mut [u8], offset: usize, src: &[u8]) -> Result<(), String> {
    if offset + src.len() > dest.len() {
        return Err(format!("chip at {:#x} overruns its region", offset));
    }
    for (i, pair) in src.chunks_exact(2).enumerate() {
        dest[offset + i * 2] = pair[1];
        dest[offset + i * 2 + 1] = pair[0];
    }
    Ok(())
}

/// Loads a Model 2 game, dispatching on the ROM set in the archive: Daytona
/// USA, Sega Rally Championship (Model 2A), or Virtua Cop.
#[cfg(feature = "model2")]
pub fn load_model2_zip(path: &str) -> Result<Roms, String> {
    let names = archive_names(path)?;
    let def = crate::roms_db::identify(&names)
        .ok_or_else(|| format!("{path}: no matching Model 2 game in the ROM database"))?;
    if def.board.is_model1() {
        return Err(format!("{} is a Model 1 game, not Model 2", def.name));
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    log::info!(target: "loader", "{} ({:?})", def.name, def.board);
    let regions = crate::roms_db::build_regions(def, &mut archive)?;
    Ok(build_model2(regions, def.board, &def.name))
}

/// Maps the built ROM regions onto the Model 2 `Roms` the system consumes.
#[cfg(feature = "model2")]
fn build_model2(
    mut regions: std::collections::HashMap<String, Vec<u8>>,
    board: crate::roms_db::Board,
    game: &str,
) -> Roms {
    let take = |r: &mut std::collections::HashMap<String, Vec<u8>>, name: &str, size: usize| {
        r.remove(name).unwrap_or_else(|| vec![0u8; size])
    };
    // Sound is either the Model 1 audio board (MultiPCM) or the 2A SCSP board,
    // told apart by which regions the set carries.
    let (sndcpu, mpcm1, mpcm2, sound_scsp) = if regions.contains_key("m1audio:sndcpu") {
        (
            take(&mut regions, "m1audio:sndcpu", 0xc0000),
            take(&mut regions, "m1audio:pcm1", 0x400000),
            take(&mut regions, "m1audio:pcm2", 0x400000),
            false,
        )
    } else {
        let sndcpu = take(&mut regions, "audiocpu", 0x80000);
        let mut samples = take(&mut regions, "samples", 0x800000);
        if samples.len() < 0x800000 {
            samples.resize(0x800000, 0);
        }
        let mpcm2 = samples.split_off(samples.len() / 2);
        (sndcpu, samples, mpcm2, true)
    };
    Roms {
        maincpu: take(&mut regions, "maincpu", 0x200000),
        main_data: take(&mut regions, "main_data", 0x2000000),
        eeprom: take(&mut regions, "eeprom", 0),
        copro_data: take(&mut regions, "copro_data", 0x800000),
        copro_tables: take(&mut regions, "copro_tgp_tables", 0x40000),
        polygons: take(&mut regions, "polygons", 0x1000000),
        textures: take(&mut regions, "textures", 0x1000000),
        sndcpu,
        mpcm1,
        mpcm2,
        sound_scsp,
        coprocessor: board,
        airwalkers_matrix: game == "airwlkrs",
    }
}

/// Maps the built ROM regions onto the Model 1 `Model1Roms`. The u32 regions
/// are little-endian views of their byte images.
#[cfg(feature = "model1")]
fn build_model1(
    regions: std::collections::HashMap<String, Vec<u8>>,
    ioboard_config: Vec<u8>,
    ioboard_kind: crate::model1board::Kind,
    apply_known_rom_repairs: bool,
) -> Result<Model1Roms, String> {
    build_model1_with_nvram_policy(regions, ioboard_config, ioboard_kind,
                                  apply_known_rom_repairs, true)
}

#[cfg(feature = "model1")]
fn build_model1_with_nvram_policy(
    mut regions: std::collections::HashMap<String, Vec<u8>>,
    ioboard_config: Vec<u8>,
    ioboard_kind: crate::model1board::Kind,
    apply_known_rom_repairs: bool,
    initialize_seed: bool,
) -> Result<Model1Roms, String> {
    let dsb = match (
        regions.remove("dsbz80:mpegcpu"),
        regions.remove("dsbz80:mpeg"),
    ) {
        (None, None) => None,
        (Some(firmware), Some(mpeg)) => {
            if firmware.len() != crate::dsbz80::FIRMWARE_SIZE
                || !matches!(mpeg.len(), 0x400000 | 0x800000)
            {
                return Err("invalid Model 1 DSB ROM region sizes".into());
            }
            Some(DsbRoms { firmware, mpeg })
        }
        _ => return Err("incomplete Model 1 DSB ROM regions".into()),
    };
    let take = |r: &mut std::collections::HashMap<String, Vec<u8>>, name: &str, size: usize| {
        r.remove(name).unwrap_or_else(|| vec![0u8; size])
    };
    let words = |b: Vec<u8>| -> Vec<u32> {
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    let mpcm1 = take(&mut regions, "m1audio:pcm1", 0x400000);
    let netmerc_procedural_audio = ioboard_kind == crate::model1board::Kind::NetMerc
        && mpcm1
            .get(..512 * 12)
            .is_some_and(|table| table.iter().all(|&b| b == 0xff));
    Ok(Model1Roms {
        dsb,
        comm_board: false,
        ioboard_kind,
        netmerc_procedural_audio,
        maincpu: {
            let mut m = take(&mut regions, "maincpu", 0x2000000);
            // The V60 region is ROMREGION_ERASEFF; build_regions already filled
            // it, but guard the fallback path too.
            if m.iter().all(|&b| b == 0) {
                m.fill(0xff);
            }
            m
        },
        tgp: {
            let mut program = take(&mut regions, "tgp_copro", 0x2000);
            if apply_known_rom_repairs {
                repair_315_5711(&mut program);
            }
            program
        },
        nvram_default: {
            let mut image = take(&mut regions, "nvram", 0);
            if ioboard_kind == crate::model1board::Kind::NetMerc {
                if image.is_empty() {
                    // Optional calibration file absent: create blank SRAM with
                    // valid empty bookkeeping, leaving calibration to the host.
                    image = vec![0xff; 0x10000];
                    image[0x1000..0x3004].fill(0);
                    image[0] = 0x0f;
                } else if initialize_seed {
                    repair_netmerc_nvram_default(&mut image);
                }
            }
            image
        },
        copro_tables: words(take(&mut regions, "copro_tables", 0x40000)),
        polygons: words(take(&mut regions, "polygons", 0x1000000)),
        copro_data: words(take(&mut regions, "copro_data", 0x200000)),
        sndcpu: take(&mut regions, "m1audio:sndcpu", 0xc0000),
        mpcm1,
        mpcm2: take(&mut regions, "m1audio:pcm2", 0x400000),
        iocpu: take(&mut regions, "ioboard:iocpu", 0x10000),
        ioboard_config: {
            // The romset's 93C45 dump is preferred; `vr_defaults.nv` is the
            // same thing under an older name.
            let dumped = take(&mut regions, "ioboard:eeprom", 0);
            if dumped.is_empty() {
                ioboard_config
            } else {
                dumped
            }
        },
    })
}

/// The reference: interleaves a 16-bit chip onto the even or odd
/// 16-bit lane of a little-endian 32-bit region (offset's low bits pick which).
pub(crate) fn load32_word(dest: &mut [u8], offset: usize, src: &[u8]) -> Result<(), String> {
    if !src.len().is_multiple_of(2) {
        return Err(format!(
            "chip size {} is not a whole number of 16-bit words",
            src.len()
        ));
    }
    let end = offset + (src.len() - 2) * 2 + 2;
    if end > dest.len() {
        return Err(format!(
            "chip at offset {:#x} overruns its {:#x}-byte region",
            offset,
            dest.len()
        ));
    }
    for (i, word) in src.chunks_exact(2).enumerate() {
        dest[offset + i * 4] = word[0];
        dest[offset + i * 4 + 1] = word[1];
    }
    Ok(())
}

/// The reference: scatters a byte-wide chip onto one of four byte
/// lanes in a little-endian 32-bit region.
pub(crate) fn load32_byte(
    dest: &mut [u8],
    offset: usize,
    lane: usize,
    src: &[u8],
) -> Result<(), String> {
    if lane >= 4 {
        return Err(format!("invalid 32-bit byte lane {}", lane));
    }
    if offset + src.len() * 4 > dest.len() {
        return Err(format!("chip at {:#x} overruns region", offset));
    }
    for (i, &byte) in src.iter().enumerate() {
        dest[offset + i * 4 + lane] = byte;
    }
    Ok(())
}

/// Reads a chip out of the archive by exact filename.
/// Lists the file names in a ROM archive, for board autodetection.
pub fn archive_names(path: &str) -> Result<Vec<String>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    Ok((0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect())
}

/// Loads a Model 1 game, dispatching on the ROM set in the archive: Virtua
/// Racing (315-5573 TGP program) or Virtua Fighter (315-5724).
#[cfg(feature = "model1")]
pub fn load_model1_zip(path: &str) -> Result<Model1Roms, String> {
    load_model1_zip_with_options(path, true)
}

/// Loads Model 1 content with an explicit policy for recognized legacy bad
/// dumps. Only the fully SHA-1-identified 315-5711 program is repaired, in RAM.
#[cfg(feature = "model1")]
pub fn load_model1_zip_with_options(
    path: &str,
    apply_known_rom_repairs: bool,
) -> Result<Model1Roms, String> {
    load_model1_zip_policy(path, apply_known_rom_repairs, true)
}

/// Load the complete seed without repairing it before persistent-save import.
/// The caller initializes the factory seed only if no valid saved image exists.
#[cfg(feature = "model1")]
pub fn load_model1_zip_with_deferred_nvram(
    path: &str,
    apply_known_rom_repairs: bool,
) -> Result<Model1Roms, String> {
    load_model1_zip_policy(path, apply_known_rom_repairs, false)
}

/// Frontend-supplied BIOS paths; preserve strict game-chip identification and
/// defer optional calibration seed patching until Save RAM initialization.
#[cfg(feature = "model1")]
pub fn load_model1_zip_with_bios(path: &str, apply_known_rom_repairs: bool,
                               system: Option<&std::path::Path>) -> Result<Model1Roms, String> {
    let names = archive_names(path)?;
    let def = crate::roms_db::identify_complete_with_external_io(&names)
        .filter(|def| def.board.is_model1()).ok_or("Incomplete or unrecognized Model 1 game ZIP")?;
    let firmware = model1_bios::load_io(std::path::Path::new(path), system, def)?;
    let mut archive = ZipArchive::new(File::open(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let ioboard_config = read_chip(&mut archive, "vr_defaults.nv").unwrap_or_default();
    let regions = crate::roms_db::build_regions_with_io(def, &mut archive, &firmware)?;
    let mut roms = build_model1_with_nvram_policy(regions, ioboard_config,
        crate::model1board::Kind::for_set(&def.name), apply_known_rom_repairs, false)?;
    roms.comm_board = crate::model1comm::present_for_set(&def.name);
    Ok(roms)
}

#[cfg(feature = "model1")]
pub mod model1_bios;

#[cfg(feature = "model1")]
fn load_model1_zip_policy(path: &str, apply_known_rom_repairs: bool,
                          initialize_seed: bool) -> Result<Model1Roms, String> {
    let names = archive_names(path)?;
    let def = crate::roms_db::identify(&names)
        .ok_or_else(|| format!("{path}: no matching Model 1 game in the ROM database"))?;
    if !def.board.is_model1() {
        return Err(format!("{} is a Model 2 game, not Model 1", def.name));
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    log::info!(target: "loader", "{} ({:?})", def.name, def.board);
    // The I/O board's operator-config default, if the set ships one; the real
    // Z80 firmware mirrors it into dpram so the game boots configured.
    let ioboard_config = read_chip(&mut archive, "vr_defaults.nv").unwrap_or_default();
    let regions = crate::roms_db::build_regions(def, &mut archive)?;
    let kind = crate::model1board::Kind::for_set(&def.name);
    let mut roms = if initialize_seed {
        build_model1(regions, ioboard_config, kind, apply_known_rom_repairs)?
    } else {
        build_model1_with_nvram_policy(regions, ioboard_config, kind,
                                      apply_known_rom_repairs, false)?
    };
    roms.comm_board = crate::model1comm::present_for_set(&def.name);
    Ok(roms)
}

#[cfg(feature = "model1")]
pub mod audio_donor;

/// Loads Star Wars Arcade, building the V60 memory image
/// `ROM_START(swa)` lays it out.
#[cfg(feature = "model1")]
pub struct Model1Roms {
    /// Load-time fallback policy for the known NetMerc blank descriptor dump.
    /// A frontend providing valid donor banks clears this; no paths in core state.
    pub netmerc_procedural_audio: bool,
    pub dsb: Option<DsbRoms>,
    /// MAME machine configuration selects the M1COMM board, not the filename.
    pub comm_board: bool,
    /// Selected from the identified ROM set, not a file path or frontend setting.
    pub ioboard_kind: crate::model1board::Kind,
    /// Factory battery-backed RAM image (NetMerc). Saved user NVRAM wins.
    pub nvram_default: Vec<u8>,
    /// "maincpu": the V60's whole 32MB address image. The reset vector lives at
    /// region offset 0xfffff0, inside the boot ROMs at 0xfe0000.
    pub maincpu: Vec<u8>,
    /// "tgp_copro": the 8KB program uploaded to the MB86233 (315-5573.bin).
    pub tgp: Vec<u8>,
    /// "copro_tables": the CPU board's math lookup ROM (sin/cos, atan, 1/x,
    /// 1/sqrt(x)), 0x10000 32-bit entries built from opr14742/opr14743. The
    /// TGP's I/O-mapped geometry accelerators index straight into this.
    pub copro_tables: Vec<u32>,
    /// Geometry-board model ROM, exposed as little-endian 32-bit words.
    pub polygons: Vec<u32>,
    /// TGP external data ROM addressed through the I/O 0x8000-0xffff window.
    pub copro_data: Vec<u32>,
    /// The I/O board's Z80 firmware (`epr-14869`), which answers the V60's
    /// commands and owns the 93C45 the operator settings live in.
    pub iocpu: Vec<u8>,
    /// Model 1 sound board (segam1audio): 68000 program + two sample banks.
    pub sndcpu: Vec<u8>,
    pub mpcm1: Vec<u8>,
    pub mpcm2: Vec<u8>,
    /// I/O-board EEPROM/config defaults (`vr_defaults.nv`), if present: the
    /// operator config the real Z80 firmware mirrors into dpram 0x100.. so the
    /// game boots with valid settings instead of the setup menu. One config
    /// byte per 16-bit word (low byte), starting with the "SEGA" magic.
    pub ioboard_config: Vec<u8>,
}

/// Owned DSB resources; region padding is preserved, no implicit file access.
#[cfg(feature = "model1")]
pub struct DsbRoms {
    pub firmware: Vec<u8>,
    pub mpeg: Vec<u8>,
}

/// Loads Virtua Racing, building the V60 memory image
/// `ROM_START(vr)` lays it out. Only the layout is established here; nothing is
/// executed yet.
pub(crate) fn load16_byte(
    dest: &mut [u8],
    offset: usize,
    lane: usize,
    src: &[u8],
) -> Result<(), String> {
    if offset + src.len() * 2 > dest.len() {
        return Err(format!("chip at {:#x} overruns region", offset));
    }
    for (i, &b) in src.iter().enumerate() {
        dest[offset + i * 2 + lane] = b;
    }
    Ok(())
}

pub(crate) fn copy_at(dest: &mut [u8], offset: usize, src: &[u8]) -> Result<(), String> {
    dest.get_mut(offset..offset + src.len())
        .ok_or_else(|| format!("chip at {:#x} overruns region", offset))?
        .copy_from_slice(src);
    Ok(())
}

pub(crate) fn read_chip(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>, String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| format!("ROM '{}' not found in archive", name))?;
    let mut buf = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

#[cfg(all(test, feature = "model1"))]
mod model1_tests {
    use super::*;

    #[test]
    fn netmerc_recovery_requires_its_known_blank_descriptor_table() {
        use crate::model1board::Kind;
        for (kind, fill, expected) in [
            (Kind::NetMerc, 0xff, true),
            (Kind::NetMerc, 0, false),
            (Kind::Original, 0xff, false),
        ] {
            let regions = [
                ("maincpu".into(), vec![0; 16]),
                ("tgp_copro".into(), vec![]),
                ("copro_tables".into(), vec![]),
                ("polygons".into(), vec![]),
                ("copro_data".into(), vec![]),
                ("m1audio:pcm1".into(), vec![fill; 512 * 12]),
            ]
            .into();
            let loaded = build_model1(regions, vec![], kind, true).unwrap();
            assert_eq!(loaded.netmerc_procedural_audio, expected);
        }
    }

    fn blank_netmerc_default() -> Vec<u8> {
        // Handcrafted calibration seed, not a copyrighted ROM fixture.
        let mut image = vec![0xff; 0x10000];
        image[0x38] = 0;
        image[0x40] = 0;
        image
    }

    #[test]
    fn netmerc_default_initializes_bookkeeping_and_credits_and_is_idempotent() {
        let mut image = blank_netmerc_default();
        let original = image.clone();
        assert!(repair_netmerc_nvram_default(&mut image));
        assert_eq!(image[0], 0x0f);
        assert!(image[0x1000..0x3004].iter().all(|&b| b == 0));
        assert_eq!(image[1..0x18], original[1..0x18]);
        assert_eq!(image[0x18..0x1a], [0, 0]);
        assert_eq!(image[0x1a..0x1000], original[0x1a..0x1000]);
        assert_eq!(image[0x3004..], original[0x3004..]);
        let initialized = image.clone();
        assert!(!repair_netmerc_nvram_default(&mut image));
        assert_eq!(image, initialized);
    }

    #[test]
    fn unknown_netmerc_defaults_are_never_patched() {
        for offset in [0, 4, 0x38, 0x40, 0x1000, 0x1fff, 0x2000, 0x3003, 0xffff] {
            let mut image = blank_netmerc_default();
            image[offset] ^= 1;
            let original = image.clone();
            assert!(!repair_netmerc_nvram_default(&mut image));
            assert_eq!(image, original);
        }
        for size in [0, 1, 0xffff, 0x10001] {
            let mut image = vec![0xff; size];
            let original = image.clone();
            assert!(!repair_netmerc_nvram_default(&mut image));
            assert_eq!(image, original);
        }
    }

    #[test]
    fn deferred_seed_is_untouched_until_required_initialization() {
        let original = blank_netmerc_default();
        let regions = [("nvram".to_owned(), original.clone())].into();
        let roms = build_model1_with_nvram_policy(regions, vec![],
            crate::model1board::Kind::NetMerc, true, false).unwrap();
        assert_eq!(roms.nvram_default, original);
        let mut needed = roms.nvram_default;
        assert!(initialize_netmerc_nvram_seed(&mut needed));
        assert_eq!(needed[0], 0x0f);
        assert!(needed[0x1000..0x3004].iter().all(|&v| v == 0));
        assert_eq!(&needed[1..0x18], &original[1..0x18]);
        assert_eq!(&needed[0x18..0x1a], &[0, 0]);
        assert_eq!(&needed[0x1a..0x1000], &original[0x1a..0x1000]);
        assert_eq!(&needed[0x3004..], &original[0x3004..]);
        assert!(!initialize_netmerc_nvram_seed(&mut needed));
    }

    #[test]
    fn only_netmerc_rom_loading_initializes_the_known_default() {
        for kind in [
            crate::model1board::Kind::Original,
            crate::model1board::Kind::WingWar,
            crate::model1board::Kind::WingWarR360,
            crate::model1board::Kind::NetMerc,
        ] {
            let original = blank_netmerc_default();
            let regions = [("nvram".to_owned(), original.clone())]
                .into_iter()
                .collect();
            let roms = build_model1(regions, vec![], kind, true).unwrap();
            if kind == crate::model1board::Kind::NetMerc {
                let mut expected = original;
                expected[0] = 0x0f;
                expected[0x1000..0x3004].fill(0);
                expected[0x18..0x1a].fill(0);
                assert_eq!(roms.nvram_default, expected);
            } else {
                assert_eq!(roms.nvram_default, original);
            }
        }
    }

    #[test]
    fn dsb_regions_are_preserved_and_incomplete_resources_are_rejected() {
        for size in [0x400000, 0x800000] {
            let regions = [
                ("dsbz80:mpegcpu".into(), vec![0x5a; 0x20000]),
                ("dsbz80:mpeg".into(), vec![0xa5; size]),
            ]
            .into_iter()
            .collect();
            let r =
                build_model1(regions, vec![], crate::model1board::Kind::Original, true).unwrap();
            let d = r.dsb.unwrap();
            assert_eq!(d.firmware, vec![0x5a; 0x20000]);
            assert_eq!(d.mpeg, vec![0xa5; size]);
        }
        for name in ["dsbz80:mpegcpu", "dsbz80:mpeg"] {
            assert!(build_model1(
                [(name.into(), vec![0; 0x20000])].into_iter().collect(),
                vec![],
                crate::model1board::Kind::Original,
                true
            )
            .is_err());
        }
    }

    #[test]
    fn unknown_tgp_program_is_never_patched() {
        for size in [0, 0x2000, 0x2001] {
            let mut program = vec![0xff; size];
            if size >= 0x2000 {
                program[0x8b * 4..0x8b * 4 + 4].copy_from_slice(&0xbf60059eu32.to_le_bytes());
                program[0x67e * 4..0x67e * 4 + 4].copy_from_slice(&0x39000002u32.to_le_bytes());
            }
            let before = program.clone();
            assert!(!repair_315_5711(&mut program));
            assert_eq!(program, before);
        }
    }

    #[test]
    fn model1_loader_preserves_factory_nvram() {
        let factory = vec![0x5a; 0x10000];
        let regions = [("nvram".to_owned(), factory.clone())]
            .into_iter()
            .collect();
        assert_eq!(
            build_model1(regions, vec![], crate::model1board::Kind::Original, true)
                .unwrap()
                .nvram_default,
            factory
        );
    }

    // No copyrighted ROM fixture is shipped. Run explicitly with a user-owned
    // swa/swaj/wingwar/netmerc ZIP supplied via TGPULSE_MODEL1_TEST_ZIP.
    #[test]
    #[ignore = "requires a user-owned 315-5711 ZIP via TGPULSE_MODEL1_TEST_ZIP"]
    fn legacy_and_corrected_tgp_load_identically() {
        let path = std::env::var("TGPULSE_MODEL1_TEST_ZIP").expect("ROM ZIP path");
        let mut archive = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let mut program = read_chip(&mut archive, "315-5711.bin").unwrap();
        let original = program.clone();
        repair_315_5711(&mut program);
        assert_eq!(
            format!("{:x}", Sha1::digest(&program)),
            "d5c61ea6e4744f10170ea556068c248bd43bb111"
        );
        let corrected = program.clone();
        assert!(!repair_315_5711(&mut program));
        assert_eq!(program, corrected);
        program[0x8b * 4] |= 2;
        program[0x67e * 4] |= 2;
        assert_eq!(
            format!("{:x}", Sha1::digest(&program)),
            "9e21d3a07ffa315e0139483b664e3fa283ef4e06"
        );
        assert!(repair_315_5711(&mut program));
        assert_eq!(program, corrected);
        assert_eq!(
            load_model1_zip_with_options(&path, false).unwrap().tgp,
            original
        );
        assert_eq!(
            load_model1_zip_with_options(&path, true).unwrap().tgp,
            corrected
        );
        assert_eq!(load_model1_zip(&path).unwrap().tgp, corrected);
    }
}
