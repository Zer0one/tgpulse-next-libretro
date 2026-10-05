//! HLE of the Sega System 24 tilemap chip, which draws Model 2's 2D/text layer.
//!
//! Layout:
//!   tile_ram 0x8000 u16. Four 64x64 tilemaps of 8x8 tiles at u16 offsets
//!             0x0000 (0s), 0x1000 (0w), 0x2000 (1s), 0x3000 (1w).
//!             0x5000+n = h-scroll, 0x5004+n = v-scroll.
//!   char_ram 0x40000 u16 of 4bpp 8x8 character bitmaps, 32 bytes each.
//!
//! Tile entry: bits 0-13 character code, bits 7-14 colour, bit 15 category
//! (category 1 draws in front of the 3D layer, category 0 behind).

#[cfg(feature = "model2")]
use crate::system::Model2System;

/// The data a segas24 tile layer needs, abstracted over the two boards. Model 2
/// packs these RAMs into 32-bit words (i960 bus); Model 1 stores raw bytes (V60
/// bus), so the accessors are logical (indexed in u16/u32 units), not raw slices.
pub trait TileSource {
    fn tile_u16(&self, idx: usize) -> u16;
    fn char_word(&self, idx: usize) -> u32;
    fn palette_u16(&self, idx: usize) -> u16;
    fn colorxlat_u16(&self, idx: usize) -> u16;
    fn colorxlat_written(&self) -> bool;
    fn monitor_gamma(&self, v: u32) -> u32;
    /// Cached name-table pen row; colours remain resolved per frame.
    fn tile_row(&self, _index: usize, _y: usize) -> Option<&[u16]> { None }
    /// Model 1 uses palette bit 15 as full/half intensity. Model 2 does not.
    fn palette_dimmed(&self, _colour: u16) -> bool {
        false
    }
}

#[cfg(feature = "model2")]
impl TileSource for Model2System {
    fn tile_u16(&self, idx: usize) -> u16 {
        u16_at(&self.tile_ram, idx)
    }
    fn char_word(&self, idx: usize) -> u32 {
        self.char_ram.get(idx).copied().unwrap_or(0)
    }
    fn palette_u16(&self, idx: usize) -> u16 {
        u16_at(&self.palette_ram, idx)
    }
    fn colorxlat_u16(&self, idx: usize) -> u16 {
        u16_at(&self.colorxlat_ram, idx)
    }
    fn colorxlat_written(&self) -> bool {
        self.colorxlat_written
    }
    fn monitor_gamma(&self, v: u32) -> u32 {
        self.monitor[(v & 0xff) as usize] as u32
    }
}

pub const SCREEN_W: usize = 496;
pub const SCREEN_H: usize = 384;

/// Model 1's decoded tile pens. Owned by the renderer, not serialized hardware.
/// Compare the RAM itself so CPU writes, debugger edits and state restore all
/// invalidate derived pixels without depending on a particular writer.
#[cfg(feature = "model1")]
#[derive(Default)]
pub struct Model1TileCache {
    names: Vec<u16>,
    tile_ram: Vec<u8>,
    char_ram: Vec<u8>,
    pens: Vec<u16>,
    dirty_chars: Vec<bool>,
}

#[cfg(feature = "model1")]
impl Model1TileCache {
    fn refresh(&mut self, sys: &crate::model1::Model1System) {
        let first = self.names.is_empty();
        if first {
            self.names.resize(0x4000, 0);
            self.tile_ram.resize(0x8000, 0);
            self.char_ram.resize(0x80000, 0);
            self.pens.resize(0x4000 * 64, 0);
            self.dirty_chars.resize(0x4000, false);
        }
        let tiles = &sys.tile_ram[..0x8000];
        let tiles_changed = first || tiles != self.tile_ram;
        let chars_changed = first || sys.char_ram != self.char_ram;
        if !tiles_changed && !chars_changed {
            return;
        }
        self.dirty_chars.fill(false);
        if chars_changed {
            for (code, (current, previous)) in sys
                .char_ram
                .chunks_exact(32)
                .zip(self.char_ram.chunks_exact_mut(32))
                .enumerate()
            {
                let dirty = first || current != previous;
                self.dirty_chars[code] = dirty;
                if dirty {
                    previous.copy_from_slice(current);
                }
            }
        }
        const SHIFTS: [u32; 8] = [12, 8, 4, 0, 28, 24, 20, 16];
        for (index, bytes) in tiles.chunks_exact(2).enumerate() {
            let value = u16::from_le_bytes([bytes[0], bytes[1]]);
            let code = (value & TILE_MASK) as usize;
            if !first && self.names[index] == value && !self.dirty_chars[code] {
                continue;
            }
            self.names[index] = value;
            let pen_base = ((value >> 7) & 0xff) * 16;
            let decoded = &mut self.pens[index * 64..(index + 1) * 64];
            for y in 0..8 {
                let offset = code * 32 + y * 4;
                let word =
                    u32::from_le_bytes(self.char_ram[offset..offset + 4].try_into().unwrap());
                for x in 0..8 {
                    decoded[y * 8 + x] = pen_base + ((word >> SHIFTS[x]) & 0xf) as u16;
                }
            }
        }
        self.tile_ram.copy_from_slice(tiles);
    }

    pub fn render_background(&mut self, sys: &crate::model1::Model1System, out: &mut [u32]) {
        self.refresh(sys);
        render_background(&CachedModel1 { sys, cache: self }, out);
    }

    pub fn render_foreground(&mut self, sys: &crate::model1::Model1System, out: &mut [u32]) {
        self.refresh(sys);
        render_foreground(&CachedModel1 { sys, cache: self }, out);
    }
}

#[cfg(feature = "model1")]
struct CachedModel1<'a> {
    sys: &'a crate::model1::Model1System,
    cache: &'a Model1TileCache,
}

#[cfg(feature = "model1")]
impl TileSource for CachedModel1<'_> {
    fn tile_u16(&self, idx: usize) -> u16 {
        self.cache
            .names
            .get(idx)
            .copied()
            .unwrap_or_else(|| self.sys.tile_u16(idx))
    }
    fn tile_row(&self, index: usize, y: usize) -> Option<&[u16]> {
        let offset = index * 64 + y * 8;
        Some(&self.cache.pens[offset..offset + 8])
    }
    fn char_word(&self, idx: usize) -> u32 {
        self.sys.char_word(idx)
    }
    fn palette_u16(&self, idx: usize) -> u16 {
        self.sys.palette_u16(idx)
    }
    fn colorxlat_u16(&self, idx: usize) -> u16 {
        self.sys.colorxlat_u16(idx)
    }
    fn colorxlat_written(&self) -> bool {
        self.sys.colorxlat_written()
    }
    fn monitor_gamma(&self, value: u32) -> u32 {
        self.sys.monitor_gamma(value)
    }
    fn palette_dimmed(&self, colour: u16) -> bool {
        self.sys.palette_dimmed(colour)
    }
}

#[cfg(all(test, feature = "model2"))]
mod palette_tests {
    use super::*;

    #[test]
    fn model2_palette_bit15_does_not_dim_colours() {
        let roms = crate::loader::Roms {
            maincpu: vec![],
            main_data: vec![],
            copro_data: vec![],
            copro_tables: vec![],
            polygons: vec![],
            textures: vec![],
            eeprom: vec![],
            sndcpu: vec![],
            mpcm1: vec![],
            mpcm2: vec![],
            sound_scsp: false,
            coprocessor: crate::roms_db::Board::Model2o,
            airwalkers_matrix: false,
        };
        let mut sys = Model2System::new(&roms);
        // Explicit identity monitor, independent of cabinet gamma defaults.
        for (i, value) in sys.monitor.iter_mut().enumerate() {
            *value = i as u8;
        }
        for colour in 0..=0x7fffu32 {
            sys.palette_ram[0] = colour | ((colour | 0x8000) << 16);
            let channel = |shift: u32| {
                let v = (colour >> shift) & 31;
                (v << 3) | (v >> 2)
            };
            let expected = 0xff00_0000 | channel(0) << 16 | channel(5) << 8 | channel(10);
            assert_eq!(pen_color(&sys, 0), expected);
            assert_eq!(pen_color(&sys, 1), expected);
        }
    }
}
/// Tilemaps are 64x64 tiles of 8x8 pixels and wrap at 512.
const MAP_MASK: u32 = 511;

/// The tile index is 14 bits wide.
const TILE_MASK: u16 = 0x3fff;

/// Reads a 16-bit device word out of a 32-bit-word backing store. The reference u16
/// handlers on the i960's little-endian bus put device word 2i in the low half
/// of CPU word i.
#[inline]
#[cfg(feature = "model2")]
fn u16_at(mem: &[u32], idx: usize) -> u16 {
    match mem.get(idx >> 1) {
        Some(w) => (*w >> ((idx & 1) * 16)) as u16,
        None => 0,
    }
}

/// Resolves one of the 4096 tile pens to a packed 0xAARRGGBB colour.
///
/// Each RGB5 component is expanded into 8 bits. Model 1 additionally uses
/// bit 15 as full/half intensity; Model 2 retains its colour translation.
pub fn pen_color<S: TileSource>(sys: &S, pen: u16) -> u32 {
    let palcolor = sys.palette_u16(pen as usize);
    let r5 = (palcolor & 0x1f) as usize;
    let g5 = ((palcolor >> 5) & 0x1f) as usize;
    let b5 = ((palcolor >> 10) & 0x1f) as usize;

    let (r, g, b) = if sys.colorxlat_written() {
        (
            sys.colorxlat_u16(0x0080 / 2 + r5 * 0x100) as u32 & 0xFF,
            sys.colorxlat_u16(0x4080 / 2 + g5 * 0x100) as u32 & 0xFF,
            sys.colorxlat_u16(0x8080 / 2 + b5 * 0x100) as u32 & 0xFF,
        )
    } else {
        // Before the game programs the translation RAM, expand 5 bits to 8 so
        // the screen is legible instead of black.
        let e = |v: usize| ((v << 3) | (v >> 2)) as u32;
        (e(r5), e(g5), e(b5))
    };

    let (r, g, b) = if sys.palette_dimmed(palcolor) {
        (r >> 1, g >> 1, b >> 1)
    } else {
        (r, g, b)
    };
    let (r, g, b) = (
        sys.monitor_gamma(r),
        sys.monitor_gamma(g),
        sys.monitor_gamma(b),
    );
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Model 2 cabinet monitor gamma, used by the 3D solid rasterizer.
#[inline]
#[cfg(feature = "model2")]
pub(crate) fn monitor(sys: &Model2System, v: u32) -> u32 {
    sys.monitor[(v & 0xff) as usize] as u32
}

/// Fetches the 4bpp pixel at (x, y) of character `code`.
///
/// The character layout is `{8,8, 4bpp, planes {0,1,2,3}, xoffs STEP8(0,4),
/// yoffs STEP8(0,32)}` with an LE bit-address xormask of 8, which works out to
/// nibbles in the byte order b1,b1,b0,b0,b3,b3,b2,b2 across the row.
#[inline]
fn char_pixel<S: TileSource>(sys: &S, code: u16, x: u32, y: u32) -> u8 {
    let word = sys.char_word((code as usize) * 8 + y as usize);
    let b = word.to_le_bytes();
    match x {
        0 => b[1] >> 4,
        1 => b[1] & 0xF,
        2 => b[0] >> 4,
        3 => b[0] & 0xF,
        4 => b[3] >> 4,
        5 => b[3] & 0xF,
        6 => b[2] >> 4,
        _ => b[2] & 0xF,
    }
}

/// Draws one 64x64 tilemap layer into `out`.
///
/// `category` selects which tiles to draw (bit 15 of the tile entry); `opaque`
/// draws pen 0 instead of treating it as transparent.
///
/// Layers pair up as (0s, 0w) and (1s, 1w): a per-scanline mask bitmap decides,
/// for each group of 8 pixels, which half of the pair is visible. The `s` layer
/// shows where the mask bit is 0 and the `w` layer where it is 1, so an all-zero
/// mask means the `w` layers draw nothing at all.
fn draw_layer<S: TileSource>(
    sys: &S,
    out: &mut [u32],
    palette: &[u32; 4096],
    layer: usize,
    category: u16,
    opaque: bool,
) {
    let base = layer * 0x1000;
    let win = layer & 1 != 0;
    let mask_base = if layer & 2 != 0 { 0x6800 } else { 0x6000 };

    let hscr_raw = sys.tile_u16(0x5000 + layer);
    let vscr_raw = sys.tile_u16(0x5004 + layer);

    // Layer disable
    if vscr_raw & 0x8000 != 0 {
        return;
    }

    // The reference: hscr = (-hscr) & 0x1ff; vscr = (+vscr) & 0x1ff. The scroll values
    // are the tilemap coordinate that lands at screen (0,0).
    let hscr = (hscr_raw.wrapping_neg() & 0x1ff) as u32;
    let vscr = (vscr_raw & 0x1ff) as u32;

    for sy in 0..SCREEN_H as u32 {
        let my = (sy + vscr) & MAP_MASK;
        let ty = my >> 3;
        let py = my & 7;

        // The mask is 4 words per scanline, each word covering 128 pixels as 16
        // groups of 8, MSB first.
        let mask_row = mask_base + (sy as usize) * 4;
        // A uniform mask hides this plane for the whole scanline. Avoid
        // probing every pixel when the other plane owns the entire row.
        if (0..4).all(|word| {
            let bits = sys.tile_u16(mask_row + word);
            if win {
                bits == 0
            } else {
                bits == u16::MAX
            }
        }) {
            continue;
        }

        let row = &mut out[sy as usize * SCREEN_W..(sy as usize + 1) * SCREEN_W];
        // One mask bit covers eight screen pixels. A visible group crosses at
        // most two tiles after horizontal scrolling, so fetch each tile and
        // character row once for its contiguous span.
        for word in 0..4 {
            let mask = sys.tile_u16(mask_row + word);
            // A mask word owns sixteen eight-pixel groups. Skip the whole
            // 128-pixel block when this plane has none of them, as in the
            // System 24 reference's full/hidden pixmap branches.
            if (win && mask == 0) || (!win && mask == u16::MAX) {
                continue;
            }
            let all_visible = (win && mask == u16::MAX) || (!win && mask == 0);
            for bit in 0..16 {
                let group = word * 16 + bit;
                let start = group * 8;
                if start >= SCREEN_W {
                    break;
                }
                if all_visible || ((mask >> (15 - bit)) & 1 != 0) == win {
                    let end = (start + 8).min(SCREEN_W);
                    let mut sx = start;
                    while sx < end {
                        let mx = (sx as u32 + hscr) & MAP_MASK;
                        let px = (mx & 7) as usize;
                        let span = (8 - px).min(end - sx);
                        let tile_index = base + ((ty * 64 + (mx >> 3)) as usize);
                        let val = sys.tile_u16(tile_index);
                        if (val >> 15) == category {
                            if let Some(pens) = sys.tile_row(tile_index, py as usize) {
                                for offset in 0..span {
                                    let pen = pens[px + offset];
                                    if pen & 0xf != 0 || opaque {
                                        row[sx + offset] = palette[pen as usize];
                                    }
                                }
                            } else {
                                let data = sys.char_word(((val & TILE_MASK) as usize) * 8 + py as usize);
                                let pen_base = ((val >> 7) & 0xff) * 16;
                                const SHIFTS: [u32; 8] = [12, 8, 4, 0, 28, 24, 20, 16];
                                for offset in 0..span {
                                    let nib = ((data >> SHIFTS[px + offset]) & 0xf) as u16;
                                    if nib != 0 || opaque {
                                        row[sx + offset] = palette[(pen_base + nib) as usize];
                                    }
                                }
                            }
                        }
                        sx += span;
                    }
                }
            }
        }
    }
}

/// Renders the full 2D layer into a 496x384 0xAARRGGBB framebuffer.
///
/// Follows the layer order in the reference. The 3D
/// layer would be composited between the two category passes.
/// Everything the compositor draws *behind* the 3D layer, split out of
/// `render` so the GPU rasterizer can start from an identical background and
/// the two paths can be compared pixel for pixel.
/// Draws one name-table plane straight into a row range, with no mask/window
/// selection -- the segas24 vertical-split mode replaces the mask split with a
/// screen split, so each region shows a whole plane.
#[allow(clippy::too_many_arguments)]
fn draw_plane_region<S: TileSource>(
    sys: &S,
    out: &mut [u32],
    palette: &[u32; 4096],
    plane: usize,
    hscr: u32,
    vscr: u32,
    y0: usize,
    y1: usize,
    opaque: bool,
) {
    let base = plane * 0x1000;
    for sy in y0..y1 {
        let my = (sy as u32 + vscr) & MAP_MASK;
        let (ty, py) = (my >> 3, my & 7);
        for sx in 0..SCREEN_W as u32 {
            let mx = (sx + hscr) & MAP_MASK;
            let (tx, px) = (mx >> 3, mx & 7);
            let val = sys.tile_u16(base + (ty * 64 + tx) as usize);
            if (val >> 15) != 0 {
                continue;
            }
            let nib = char_pixel(sys, val & TILE_MASK, px, py);
            if nib == 0 && !opaque {
                continue;
            }
            let pen = ((val >> 7) & 0xff) * 16 + nib as u16;
            out[sy * SCREEN_W + sx as usize] = palette[pen as usize];
        }
    }
}

/// segas24's special window/scroll mode. When the control word (the vscroll of
/// the even plane in a pair) has bits 0x6000 set, the plane pair is not
/// mask-split into `s`/`w` but screen-split: the reference, `ctrl &
/// 0x6000`. Daytona uses case 1 (a horizontal cut) for the sky -- the top of
/// the screen shows one plane, the bottom the other, which is how the blue sky
/// reaches the top instead of the panorama's wrapped ground band.
///
/// Returns false when the mode is not this case, so the caller falls back to
/// the ordinary mask-based draw.
fn draw_split_pair<S: TileSource>(
    sys: &S,
    out: &mut [u32],
    palette: &[u32; 4096],
    plane: usize,
    opaque: bool,
) -> bool {
    let hscr_raw = sys.tile_u16(0x5000 + plane);
    // For an even plane, 0x5004 + (plane & 2) == 0x5004 + plane, so the control
    // word is this plane's own vscroll register.
    let ctrl = sys.tile_u16(0x5004 + plane);
    if ctrl & 0x6000 == 0 || ctrl & 0x8000 != 0 {
        return false; // ordinary mode, or the pair is disabled
    }
    // Per-line hscroll (bit 15 of hscr) and cases 2/3 are not yet needed by any
    // exercised game; fall back rather than draw them wrong.
    if hscr_raw & 0x8000 != 0 || (ctrl & 0x6000) >> 13 != 1 {
        return false;
    }

    let vscr = (ctrl & 0x1ff) as u32;
    let hscr = (hscr_raw.wrapping_neg() & 0x1ff) as u32;
    let split = (ctrl.wrapping_neg() & 0x1ff) as usize;

    // The reference: `if(!((-vscr) & 0x200)) layer ^= 1;` then top = layer, bottom = layer^1.
    let mut top = plane;
    if ctrl.wrapping_neg() & 0x200 == 0 {
        top ^= 1;
    }
    let bottom = top ^ 1;

    let split = split.min(SCREEN_H);
    draw_plane_region(sys, out, palette, top, hscr, vscr, 0, split, opaque);
    draw_plane_region(
        sys, out, palette, bottom, hscr, vscr, split, SCREEN_H, opaque,
    );
    true
}

/// The tile layers drawn behind the 3D image. Shared by `render` and
/// `render_background` so the CPU compositor and the GPU renderer start from
/// exactly the same background.
fn draw_bg_layers<S: TileSource>(sys: &S, out: &mut [u32], palette: &[u32; 4096]) {
    // Opaque pair (planes 2/3). The even plane's control word may put it in the
    // vertical-split mode; otherwise draw both planes the ordinary way.
    if !draw_split_pair(sys, out, palette, 2, true) {
        draw_layer(sys, out, palette, 3, 0, true);
        draw_layer(sys, out, palette, 2, 0, true);
    }
    // Transparent pair (planes 0/1).
    if !draw_split_pair(sys, out, palette, 0, false) {
        draw_layer(sys, out, palette, 1, 0, false);
        draw_layer(sys, out, palette, 0, 0, false);
    }
}

pub fn render_background<S: TileSource>(sys: &S, out: &mut [u32]) {
    let palette: [u32; 4096] = std::array::from_fn(|i| pen_color(sys, i as u16));
    out.fill(palette[0]);
    draw_bg_layers(sys, out, &palette);
}

#[cfg(feature = "model2")]
pub fn render(sys: &Model2System, out: &mut [u32]) {
    // Palette and translation RAM are stable for one frame. Resolve each pen
    // once instead of repeating three table lookups plus gamma per tile pixel.
    let palette: [u32; 4096] = std::array::from_fn(|i| pen_color(sys, i as u16));
    let backdrop = palette[0];
    out.fill(backdrop);

    draw_bg_layers(sys, out, &palette);

    crate::geometry::rasterize_solids(sys, out);

    // In front of the 3D layer.
    for layer in (0..=3).rev() {
        draw_layer(sys, out, &palette, layer, 1, false);
    }
}

/// Produces an alpha mask/colour image for the tile categories in front of
/// 3D. Used by the compute rasterizer to preserve the exact composition order.
pub fn render_foreground<S: TileSource>(sys: &S, out: &mut [u32]) {
    out.fill(0);
    let palette: [u32; 4096] = std::array::from_fn(|i| pen_color(sys, i as u16));
    for layer in (0..=3).rev() {
        draw_layer(sys, out, &palette, layer, 1, false);
    }
}

#[cfg(test)]
mod mask_tests {
    use super::*;

    struct Source {
        tiles: Vec<u16>,
        patterned_chars: bool,
    }

    impl TileSource for Source {
        fn tile_u16(&self, idx: usize) -> u16 {
            self.tiles[idx]
        }
        fn char_word(&self, idx: usize) -> u32 {
            if self.patterned_chars {
                (idx as u32).wrapping_mul(0x9e37_79b9).rotate_left(11)
            } else {
                0x1111_1111
            }
        }
        fn palette_u16(&self, _idx: usize) -> u16 {
            0
        }
        fn colorxlat_u16(&self, _idx: usize) -> u16 {
            0
        }
        fn colorxlat_written(&self) -> bool {
            false
        }
        fn monitor_gamma(&self, value: u32) -> u32 {
            value
        }
    }

    #[test]
    fn uniform_and_partial_masks_select_the_correct_plane() {
        let mut source = Source {
            tiles: vec![0; 0x7000],
            patterned_chars: false,
        };
        source.tiles[0..0x2000].fill(1);
        let mut palette = [0; 4096];
        palette[1] = 0xff12_3456;
        let mut pixels = vec![0xdead_beef; SCREEN_W * SCREEN_H];

        // An all-zero mask hides the window plane, including the last row.
        draw_layer(&source, &mut pixels, &palette, 1, 0, false);
        assert!(pixels.iter().all(|&pixel| pixel == 0xdead_beef));

        // An all-one mask hides the standard plane.
        for row in 0..SCREEN_H {
            source.tiles[0x6000 + row * 4..0x6000 + row * 4 + 4].fill(u16::MAX);
        }
        draw_layer(&source, &mut pixels, &palette, 0, 0, false);
        assert!(pixels.iter().all(|&pixel| pixel == 0xdead_beef));

        // A partial mask must still draw the selected eight-pixel group.
        for row in 0..SCREEN_H {
            source.tiles[0x6000 + row * 4..0x6000 + row * 4 + 4].fill(0);
        }
        source.tiles[0x6000] = 0x8000;
        draw_layer(&source, &mut pixels, &palette, 1, 0, false);
        assert!(pixels[..8].iter().all(|&pixel| pixel == palette[1]));
        assert!(pixels[8..].iter().all(|&pixel| pixel == 0xdead_beef));
    }

    fn draw_layer_reference(
        sys: &Source,
        out: &mut [u32],
        palette: &[u32; 4096],
        layer: usize,
        category: u16,
        opaque: bool,
    ) {
        let base = layer * 0x1000;
        let win = layer & 1 != 0;
        let mask_base = if layer & 2 != 0 { 0x6800 } else { 0x6000 };
        let hscr = (sys.tile_u16(0x5000 + layer).wrapping_neg() & 0x1ff) as u32;
        let vscr = (sys.tile_u16(0x5004 + layer) & 0x1ff) as u32;
        for sy in 0..SCREEN_H as u32 {
            let my = (sy + vscr) & MAP_MASK;
            let mask_row = mask_base + sy as usize * 4;
            for sx in 0..SCREEN_W as u32 {
                let bits = sys.tile_u16(mask_row + (sx >> 7) as usize);
                if ((bits >> (15 - ((sx >> 3) & 15))) & 1 != 0) != win {
                    continue;
                }
                let mx = (sx + hscr) & MAP_MASK;
                let val = sys.tile_u16(base + ((my >> 3) * 64 + (mx >> 3)) as usize);
                if (val >> 15) != category {
                    continue;
                }
                let nib = char_pixel(sys, val & TILE_MASK, mx & 7, my & 7);
                if nib != 0 || opaque {
                    let pen = ((val >> 7) & 0xff) * 16 + nib as u16;
                    out[sy as usize * SCREEN_W + sx as usize] = palette[pen as usize];
                }
            }
        }
    }

    #[test]
    fn grouped_tile_walk_matches_pixel_reference() {
        let mut source = Source {
            tiles: vec![0; 0x7000],
            patterned_chars: true,
        };
        for (idx, tile) in source.tiles[..0x4000].iter_mut().enumerate() {
            *tile = (idx as u16).wrapping_mul(2371);
        }
        for base in [0x6000, 0x6800] {
            for row in 0..SCREEN_H {
                let mask = match row % 4 {
                    0 => 0,
                    1 => u16::MAX,
                    2 => 0x55aa,
                    _ => 0xa531,
                };
                source.tiles[base + row * 4..base + row * 4 + 4].fill(mask);
            }
        }
        let palette = std::array::from_fn(|i| 0xff00_0000 | (i as u32 * 0x10203));
        for scroll in [0, 3, 7, 511] {
            for layer in 0..4 {
                source.tiles[0x5000 + layer] = scroll;
                source.tiles[0x5004 + layer] = scroll & 0x1ff;
                for category in 0..=1 {
                    for opaque in [false, true] {
                        let mut expected = vec![0xdead_beef; SCREEN_W * SCREEN_H];
                        let mut actual = expected.clone();
                        draw_layer_reference(
                            &source,
                            &mut expected,
                            &palette,
                            layer,
                            category,
                            opaque,
                        );
                        draw_layer(&source, &mut actual, &palette, layer, category, opaque);
                        assert_eq!(
                            actual, expected,
                            "scroll={scroll} layer={layer} category={category} opaque={opaque}"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "model1"))]
mod cache_tests {
    use super::*;
    use crate::{loader::Model1Roms, model1::Model1System, model1board::Kind};

    fn machine() -> Model1System {
        Model1System::new(&Model1Roms {
            maincpu: vec![],
            tgp: vec![],
            copro_tables: vec![],
            polygons: vec![],
            copro_data: vec![],
            iocpu: vec![],
            sndcpu: vec![],
            mpcm1: vec![],
            mpcm2: vec![],
            ioboard_config: vec![],
            nvram_default: vec![],
            comm_board: false,
            ioboard_kind: Kind::Original,
            dsb: None,
            netmerc_procedural_audio: false,
        })
        .unwrap()
    }

    fn compare(cache: &mut Model1TileCache, sys: &Model1System) {
        let mut expected = vec![0; SCREEN_W * SCREEN_H];
        let mut actual = expected.clone();
        render_background(sys, &mut expected);
        cache.render_background(sys, &mut actual);
        assert_eq!(actual, expected, "background");
        render_foreground(sys, &mut expected);
        cache.render_foreground(sys, &mut actual);
        assert_eq!(actual, expected, "foreground");
    }

    #[test]
    fn cached_pixels_follow_ram_palette_masks_scroll_and_restore() {
        let mut sys = machine();
        let mut cache = Model1TileCache::default();
        for (index, byte) in sys.char_ram.iter_mut().enumerate() {
            *byte = (index as u32).wrapping_mul(0x9e37_79b9).rotate_left(11) as u8;
        }
        for index in 0..0x4000 {
            let value = (index as u16).wrapping_mul(0x417);
            sys.tile_ram[index * 2..index * 2 + 2].copy_from_slice(&value.to_le_bytes());
        }
        for (index, byte) in sys.palette_ram.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let saved = sys.save_state().unwrap();
        for mask in [0u16, u16::MAX, 0xaaaa, 0x8001] {
            for base in [0x6000, 0x6800] {
                for word in base..base + SCREEN_H * 4 {
                    sys.tile_ram[word * 2..word * 2 + 2].copy_from_slice(&mask.to_le_bytes());
                }
            }
            for scroll in [0u16, 3, 7, 511] {
                for index in 0x5000..0x5008 {
                    sys.tile_ram[index * 2..index * 2 + 2].copy_from_slice(&scroll.to_le_bytes());
                }
                compare(&mut cache, &sys);
            }
        }
        // Direct edits bypass bus handlers, just like debugger/resource tooling.
        for index in 0x5000..0x5008 {
            sys.tile_ram[index * 2..index * 2 + 2].fill(0);
        }
        for base in [0x6000, 0x6800] {
            sys.tile_ram[base * 2..(base + SCREEN_H * 4) * 2].fill(0);
        }
        sys.tile_ram[..2].copy_from_slice(&0x8081u16.to_le_bytes());
        sys.char_ram[0x81 * 32..0x82 * 32].fill(0x32);
        sys.char_ram[0x3fff * 32..].fill(0xf0);
        sys.palette_ram[..2].copy_from_slice(&0xffffu16.to_le_bytes());
        compare(&mut cache, &sys);
        let mut foreground = vec![0; SCREEN_W * SCREEN_H];
        cache.render_foreground(&sys, &mut foreground);
        let previous = foreground[0];
        // A visible character-only write must invalidate every referring tile.
        sys.char_ram[0x81 * 32 + 1] ^= 0xf0;
        compare(&mut cache, &sys);
        cache.render_foreground(&sys, &mut foreground);
        assert_ne!(foreground[0], previous);
        // Split mode and layer disable retain the existing compositor rules.
        sys.tile_ram[0x5006 * 2..0x5006 * 2 + 2].copy_from_slice(&0x2064u16.to_le_bytes());
        compare(&mut cache, &sys);
        sys.tile_ram[0x5005 * 2..0x5005 * 2 + 2].copy_from_slice(&0x8000u16.to_le_bytes());
        compare(&mut cache, &sys);
        sys.load_state(&saved).unwrap();
        compare(&mut cache, &sys);
    }
}
