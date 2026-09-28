//! Persistent video memories, not rasterizer resources or frontend preferences.
use super::*;

/// Internal Model 1 video snapshot, captured between bus/run/render calls.
/// Upload commands execute synchronously, so there is no suspended list walker.
/// ROMs, framebuffers, GPU objects and configuration are deliberately excluded.
/// The enclosing machine state must supply matching ROMs and restore frame_num
/// (used for palette cycling and automatic list selection) with scheduler state.
/// This is not a versioned file format or a complete machine save state.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoState {
    tgp_ram: Vec<u16>,
    poly_ram: Vec<u32>,
    #[serde(with = "serde_big_array::BigArray")]
    lightparams: [LightParam; 256],
    display_list: [Vec<u8>; 2],
    listctl: [u16; 2],
    tile_ram: Vec<u8>,
    char_ram: Vec<u8>,
    palette_ram: Vec<u8>,
    colorxlat_ram: Vec<u8>,
}

impl VideoState {
    /// Validate without mutation so a future machine-level transaction can
    /// check all subsystems before committing. Decode limits belong to its
    /// outer envelope; validation here occurs after deserialization.
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.tgp_ram.len() != TGP_RAM_WORDS
            || self.poly_ram.len() != POLY_RAM_WORDS
            || self.display_list.iter().any(|list| list.len() != 0x10000)
            || self.tile_ram.len() != 0x10000
            || self.char_ram.len() != 0x80000
            || self.palette_ram.len() != 0x4000
            || self.colorxlat_ram.len() != 0xc000
        {
            return Err("invalid Model 1 video memory dimensions");
        }
        // Uploads decode each coefficient from an unsigned byte / 255. NaN,
        // infinity and values outside that range cannot come from the device.
        if self.lightparams.iter().any(|light| {
            [light.diffuse, light.ambient, light.specular]
                .iter()
                .any(|value| !(0.0..=1.0).contains(value))
        }) {
            return Err("invalid Model 1 lighting parameters");
        }
        Ok(())
    }
}

impl Model1System {
    pub fn snapshot_video(&self) -> VideoState {
        VideoState {
            tgp_ram: self.video.tgp_ram.clone(),
            poly_ram: self.video.poly_ram.clone(),
            lightparams: self.video.lightparams,
            display_list: self.display_list.clone(),
            listctl: self.listctl,
            tile_ram: self.tile_ram.clone(),
            char_ram: self.char_ram.clone(),
            palette_ram: self.palette_ram.clone(),
            colorxlat_ram: self.colorxlat_ram.clone(),
        }
    }

    /// Restore in memory, without scanning/rasterizing lists or triggering
    /// vblank. Keep current smooth-shadow/config preferences. The frontend must
    /// regenerate its output on the next render rather than reuse an old frame.
    pub fn restore_video(&mut self, state: &VideoState) -> Result<(), &'static str> {
        state.validate()?;
        self.video.tgp_ram.clone_from(&state.tgp_ram);
        self.video.poly_ram.clone_from(&state.poly_ram);
        self.video.lightparams = state.lightparams;
        self.display_list.clone_from(&state.display_list);
        self.listctl = state.listctl;
        self.tile_ram.clone_from(&state.tile_ram);
        self.char_ram.clone_from(&state.char_ram);
        self.palette_ram.clone_from(&state.palette_ram);
        self.colorxlat_ram.clone_from(&state.colorxlat_ram);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, loader::Model1Roms, tilemap};
    use v60::Bus;

    fn machine() -> Model1System {
        Model1System::new(&Model1Roms {
            dsb: None,
            comm_board: false,
            ioboard_kind: crate::model1board::Kind::Original,
            nvram_default: vec![],
            maincpu: vec![],
            tgp: vec![],
            copro_tables: vec![],
            polygons: vec![0xdeadbeef],
            copro_data: vec![],
            iocpu: vec![0x76],
            sndcpu: vec![],
            mpcm1: vec![],
            mpcm2: vec![],
            ioboard_config: vec![],
        })
        .unwrap()
    }

    fn list(system: &mut Model1System, bank: usize, words: &[u32]) {
        system.display_list[bank].fill(0);
        for (dest, word) in system.display_list[bank].chunks_exact_mut(4).zip(words) {
            dest.copy_from_slice(&word.to_le_bytes());
        }
    }

    fn encode(system: &Model1System) -> Vec<u8> {
        bincode::serialize(&system.snapshot_video()).unwrap()
    }

    fn frame(system: &mut Model1System) -> Vec<u32> {
        let mut pixels = vec![0; SCREEN_W * SCREEN_H];
        tilemap::render_background(system, &mut pixels);
        let stats = render_below_hud(system, &mut pixels);
        assert!(stats.source_quads > 0);
        assert!(stats.pixels > 0);
        let mut hud = vec![0; pixels.len()];
        tilemap::render_foreground(system, &mut hud);
        for (dst, src) in pixels.iter_mut().zip(hud) {
            if src != 0 {
                *dst = src;
            }
        }
        pixels
    }

    fn scene(system: &mut Model1System) {
        // One uploaded quad, with lighting enabled and no ROM polygon data.
        let f = f32::to_bits;
        let polygon = [
            f(-1.0),
            f(-1.0),
            f(4.0),
            f(-1.0),
            f(1.0),
            f(4.0),
            0x4601,
            f(0.0),
            f(0.0),
            f(1.0),
            f(1.0),
            f(-1.0),
            f(4.0),
            f(1.0),
            f(1.0),
            f(4.0),
        ];
        let mut upload = vec![
            4,
            TGP_RAM_BASE,
            0,
            1,
            6,
            0,
            1,
            0x20808080,
            5,
            0x800000,
            polygon.len() as u32,
        ];
        upload.extend(polygon);
        upload.push(0xf);
        list(system, 0, &upload);
        system.listctl = [0, 0x1f];
        system.irq_mask = 0xff;
        system.trigger_vblank(); // uploads persist without ever rasterizing
        let draw = [
            3,
            0,
            248,
            231,
            0,
            39,
            495,
            422, // viewport
            9,
            f(100.0),
            f(100.0), // zoom
            0xa,
            f(0.0),
            f(0.0),
            f(1.0), // light
            1,
            TGP_RAM_BASE,
            0x800000,
            1,
            0xf,
        ];
        list(system, 0, &draw); // no uploads remain to reconstruct persistent RAM
        list(system, 1, &[0xf]);
        system.palette_ram[0x2002..0x2004].copy_from_slice(&0x421fu16.to_le_bytes());
        for component in 0..3 {
            for colour in 0..32 {
                for light in 0..64 {
                    let index = (component * 0x2000 + colour * 256 + light) * 2;
                    let value = (colour * light / 63 * 8) as u16;
                    system.colorxlat_ram[index..index + 2].copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        // Visible 2D tile/character and its palette, unrelated to the 3D upload.
        system.tile_ram[..2].copy_from_slice(&1u16.to_le_bytes());
        system.char_ram[32..64].fill(0x11);
        system.palette_ram[2..4].copy_from_slice(&0x83e0u16.to_le_bytes());
    }

    #[test]
    fn persistent_uploads_restore_into_fresh_resources_and_render_identically() {
        let mut first = machine();
        scene(&mut first);
        let bytes = encode(&first);
        let saved: VideoState = bincode::deserialize(&bytes).unwrap();
        let mut second = machine();
        second.video.smooth_shadows = true;
        second.config = Config {
            smooth_shadows: true,
            ..Config::default()
        };
        second.frame_num = first.frame_num; // supplied by the enclosing machine, not video
        second.irq_status = 0xa5;
        second.restore_video(&saved).unwrap();
        assert_eq!(encode(&second), bytes);
        assert_eq!(second.frame_num, first.frame_num);
        assert_eq!(second.irq_status, 0xa5); // restore is not a vblank/IRQ event
        assert!(second.video.smooth_shadows && second.config.smooth_shadows);
        assert_eq!(second.polygons, [0xdeadbeef]);
        for _ in 0..3 {
            let pixels = frame(&mut first);
            assert!(pixels.contains(&0xff00ff00)); // restored 2D character/palette
            assert_eq!(pixels, frame(&mut second));
            let a = gpu_quads(&mut first);
            let b = gpu_quads(&mut second);
            assert!(!a.is_empty());
            assert_ne!(a[0].color & 0xffffff, 0); // actual lit/translated 3D colour
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&a),
                bytemuck::cast_slice::<_, u8>(&b)
            );
            first.trigger_vblank();
            second.trigger_vblank();
            assert_eq!(encode(&first), encode(&second));
        }
        // A scene without persistent polygon RAM is not an equally empty pass.
        second.video.poly_ram.fill(0);
        assert!(gpu_quads(&mut second).is_empty());
    }

    #[test]
    fn partial_next_list_and_automatic_selection_continue_without_replaying_uploads() {
        let mut first = machine();
        scene(&mut first);
        first.listctl = [4, 0x1f];
        first.frame_num = 1;
        list(&mut first, 1, &[5, 0x800000, 1, 0x1234, 0xf]);
        let saved = first.snapshot_video();
        let mut second = machine();
        second.frame_num = first.frame_num;
        second.restore_video(&saved).unwrap();
        assert_eq!(encode(&first), encode(&second));
        assert_ne!(second.video.poly_ram[0], 0x1234);
        for sys in [&mut first, &mut second] {
            // Finish the high half of the inactive list's upload data on the bus.
            sys.write_u16(0x61000e, 0x5678);
            sys.trigger_vblank(); // consume old list, select bank 1
            assert_eq!(sys.listctl[0] & 0x40, 0x40);
            assert_ne!(sys.video.poly_ram[0], 0x56781234);
            sys.trigger_vblank(); // now consume the new upload
            assert_eq!(sys.video.poly_ram[0], 0x56781234);
        }
        assert_eq!(encode(&first), encode(&second));
    }

    #[test]
    fn invalid_video_state_is_rejected_before_any_mutation() {
        let mut system = machine();
        scene(&mut system);
        let before = encode(&system);
        let mut saved = system.snapshot_video();
        // Each vector is independently dimension-checked, including both lists.
        for field in 0..8 {
            let mut bad = saved.clone();
            match field {
                0 => {
                    bad.tgp_ram.pop();
                }
                1 => {
                    bad.poly_ram.push(0);
                }
                2 | 3 => {
                    bad.display_list[field - 2].pop();
                }
                4 => {
                    bad.tile_ram.pop();
                }
                5 => {
                    bad.char_ram.pop();
                }
                6 => {
                    bad.palette_ram.pop();
                }
                _ => {
                    bad.colorxlat_ram.pop();
                }
            }
            assert!(system.restore_video(&bad).is_err());
            assert_eq!(encode(&system), before);
        }
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            saved.lightparams[0].ambient = value;
            assert!(system.restore_video(&saved).is_err());
            assert_eq!(encode(&system), before);
        }
    }
}
