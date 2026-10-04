//! HD44780-only presentation. Native LCD state/blink belongs to the board;
//! resource paths, preferences and output composition belong to the adapter.
use std::ffi::CStr;
use std::path::PathBuf;
use tgpulse_core::{loader::model1_bios, model1io2::DiagnosticPixels};

pub const KEYS: [&CStr; 3] = [c"tgpulse_next_netmerc_diagnostic_display",
    c"tgpulse_next_netmerc_diagnostic_position", c"tgpulse_next_netmerc_diagnostic_opacity"];
pub const MISSING_BIOS: &str = "Sega NetMerc Diagnostic Display: HD44780 BIOS Missing Or Invalid (hd44780.zip)";

#[derive(Clone, Copy)]
pub struct Settings { pub enabled: bool, pub half: bool, pub corner: u8, pub opacity: u32 }
impl Default for Settings {
    fn default() -> Self { Self { enabled: false, half: false, corner: 1, opacity: 80 } }
}

pub struct Display {
    game: PathBuf,
    system: Option<PathBuf>,
    font: Option<Box<[u8; 4096]>>,
    attempted: bool,
    notice: bool,
}
impl Display {
    pub fn new(game: PathBuf, system: Option<PathBuf>) -> Self {
        Self { game, system, font: None, attempted: false, notice: false }
    }
    pub fn prepare(&mut self, enabled: bool) {
        if !enabled { self.attempted = false; self.notice = false; return; }
        if !self.attempted {
            self.attempted = true;
            if self.font.is_none() { self.font = model1_bios::load_lcd(&self.game, self.system.as_deref()); }
            self.notice = self.font.is_none();
        }
    }
    pub fn font(&self) -> Option<&[u8; 4096]> { self.font.as_deref() }
    pub fn take_notice(&mut self) -> bool { std::mem::take(&mut self.notice) }
}

/// Straight-alpha source-over, matching the existing Timing panel compositor.
/// FE native tile markers are opaque. Software targets are always opaque.
fn blend(target: u32, color: u32, alpha: u32, transparent: bool) -> u32 {
    if alpha == 0 { return target; }
    let da = if transparent && target >> 24 < 254 { target >> 24 } else { 255 };
    let out = alpha + (da * (255 - alpha) + 127) / 255;
    let mut result = out << 24;
    for shift in [0, 8, 16] {
        let source = (color >> shift) & 255;
        let dest = (target >> shift) & 255;
        let value = (source * alpha * 255 + dest * da * (255 - alpha) + out * 127)
            / (out * 255);
        result |= value.min(255) << shift;
    }
    result
}

pub fn draw(frame: &mut [u32], width: usize, height: usize, pixels: &DiagnosticPixels,
            settings: Settings, transparent: bool) {
    if !settings.enabled || width == 0 || height == 0 || frame.len() < width * height { return; }
    let scale = (2 * width / 496).max(1).min(width.saturating_sub(24) / 121)
        .min(height.saturating_sub(24) / 19);
    if scale == 0 { return; }
    let pad = 4;
    let divisor = if settings.half { 2 } else { 1 };
    let w = (121 * scale + pad * 2) / divisor;
    let h = (19 * scale + pad * 2) / divisor;
    let margin = 8;
    let x = if matches!(settings.corner, 1 | 2) { width.saturating_sub(w + margin) } else { margin };
    let y = if settings.corner >= 2 { height.saturating_sub(h + margin) } else { margin };
    let alpha = settings.opacity.min(100) * 255 / 100;
    for py in 0..h {
        for px in 0..w {
            let sx = px * divisor;
            let sy = py * divisor;
            let active = sx >= pad && sx < 121 * scale + pad && sy >= pad && sy < 19 * scale + pad;
            let pen = if active { pixels[(sy - pad) / scale][(sx - pad) / scale] } else { 0 };
            // Upstream/MAME diagnostic palette; active glyphs remain opaque.
            let (color, alpha) = match pen {
                1 => (0x5c_5358, 255),
                2 => (0x83_888b, alpha),
                _ => (0x8a_9294, alpha),
            };
            if x + px < width && y + py < height {
                let target = &mut frame[(y + py) * width + x + px];
                *target = blend(*target, color, alpha, transparent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_and_half_geometry_alpha_match_software_and_gpu() {
        let mut pixels = [[2; 121]; 19]; pixels[0][0] = 1;
        for width in [496, 683] { for corner in 0..4 { for half in [false, true] {
            let settings = Settings { enabled: true, half, corner, opacity: 80 };
            let mut fg = vec![0; width * 384];
            let mut software = vec![0xff_203040; width * 384];
            draw(&mut fg, width, 384, &pixels, settings, true);
            draw(&mut software, width, 384, &pixels, settings, false);
            assert!(fg.iter().any(|p| p >> 24 == 204));
            assert!(fg.contains(&0xff5c5358));
            for (&p, &expected) in fg.iter().zip(&software) {
                assert_eq!(blend(0xff203040, p & 0xffffff, p >> 24, false), expected);
            }
            let w = if half { 125 } else { 250 };
            let h = if half { 23 } else { 46 };
            let x = if matches!(corner, 1 | 2) { width - w - 8 } else { 8 };
            let y = if corner >= 2 { 384 - h - 8 } else { 8 };
            assert_eq!(fg.iter().filter(|&&p| p != 0).count(), w * h);
            assert_ne!(fg[y * width + x], 0);
            assert_ne!(fg[(y + h - 1) * width + x + w - 1], 0);
        }}}
        // Zero background opacity still keeps native glyphs visible at the anchor.
        let settings = Settings { enabled: true, half: true, corner: 0, opacity: 0 };
        let mut frame = vec![0; 496 * 384];
        draw(&mut frame, 496, 384, &pixels, settings, true);
        assert_eq!(frame[10 * 496 + 10], 0xff5c5358);
        assert_eq!(frame.iter().filter(|&&p| p != 0).count(), 1);
    }

    #[test]
    fn missing_font_notifies_once_per_activation_and_off_is_silent() {
        let mut display = Display::new(PathBuf::from("/absent/game.zip"), None);
        display.prepare(false); assert!(!display.take_notice());
        display.prepare(true); assert!(display.take_notice());
        display.prepare(true); assert!(!display.take_notice());
        display.prepare(false); assert!(!display.take_notice());
        display.prepare(true); assert!(display.take_notice());
        assert!(display.font().is_none());
    }
}
