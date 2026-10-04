//! SM2/Supermodel-style timing panel, composed into the software framebuffer.
//! No window, device, files or machine state belong to this adapter module.

use imgui::{Condition, DrawCmd, DrawVert, FontConfig, FontId, FontSource, TextureId};
use std::time::Instant;

const WINDOW: u32 = 61;

#[derive(Clone, Copy, Debug, Default)]
pub struct Measurements {
    pub machine: f64,
    pub video: f64,
    pub audio: f64,
    pub run: f64,
    pub worst: f64,
    pub actual_fps: f64,
    pub engine_fps: f64,
    pub callback_fps: f64,
}

#[derive(Default)]
pub struct Timing {
    count: u32,
    sums: Measurements,
    previous: Option<Instant>,
    intervals: u32,
    interval_ms: f64,
    pub published: Option<Measurements>,
}

impl Timing {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn record(&mut self, start: Instant, machine: f64, video: f64, audio: f64, run: f64) {
        if let Some(previous) = self.previous.replace(start) {
            self.interval_ms += start.duration_since(previous).as_secs_f64() * 1000.0;
            self.intervals += 1;
        }
        self.sums.machine += machine;
        self.sums.video += video;
        self.sums.audio += audio;
        self.sums.run += run;
        self.sums.worst = self.sums.worst.max(run);
        self.count += 1;
        if self.count == WINDOW {
            let n = f64::from(self.count);
            let machine = self.sums.machine / n;
            let video = self.sums.video / n;
            let run = self.sums.run / n;
            self.published = Some(Measurements {
                machine,
                video,
                audio: self.sums.audio / n,
                run,
                worst: self.sums.worst,
                actual_fps: if self.interval_ms > 0.0 {
                    1000.0 * f64::from(self.intervals) / self.interval_ms
                } else {
                    0.0
                },
                engine_fps: if machine + video > 0.0 {
                    1000.0 / (machine + video)
                } else {
                    0.0
                },
                callback_fps: if run > 0.0 { 1000.0 / run } else { 0.0 },
            });
            self.count = 0;
            self.sums = Measurements::default();
            self.intervals = 0;
            self.interval_ms = 0.0;
        }
    }
}

#[derive(Default)]
pub struct Panel {
    context: Option<imgui::SuspendedContext>,
    fonts: Option<[FontId; 4]>,
    texture: Vec<u8>,
    texture_width: usize,
    texture_height: usize,
    transparent_target: bool,
}

// The context contains no platform/backend callbacks or shared atlas. It is
// suspended between draws and accessed only under the adapter's CORE mutex.
unsafe impl Send for Panel {}

impl Panel {
    /// Keep straight alpha when drawing before the GPU's final 3D resolve.
    pub fn draw_foreground(
        &mut self, frame: &mut [u32], width: usize, height: usize,
        font_size: u32, fps: f64, data: Option<Measurements>,
    ) {
        self.transparent_target = true;
        self.draw(frame, width, height, font_size, fps, data);
        self.transparent_target = false;
    }

    pub fn draw(
        &mut self,
        frame: &mut [u32],
        width: usize,
        height: usize,
        font_size: u32,
        fps: f64,
        data: Option<Measurements>,
    ) {
        let Some(data) = data else { return };
        if font_size == 0 || width == 0 || height == 0 || frame.len() < width * height {
            return;
        }
        let suspended = self
            .context
            .take()
            .unwrap_or_else(imgui::SuspendedContext::create);
        let mut context = match suspended.activate() {
            Ok(context) => context,
            Err(suspended) => {
                self.context = Some(suspended);
                return;
            }
        };
        if self.fonts.is_none() {
            context.set_ini_filename(None);
            context.set_log_filename(None);
            context.style_mut().use_dark_colors();
            self.fonts = Some(std::array::from_fn(|index| {
                context.fonts().add_font(&[FontSource::DefaultFontData {
                    config: Some(FontConfig {
                        size_pixels: (11 + index) as f32,
                        ..FontConfig::default()
                    }),
                }])
            }));
            let atlas = context.fonts().build_rgba32_texture();
            self.texture = atlas.data.to_vec();
            self.texture_width = atlas.width as usize;
            self.texture_height = atlas.height as usize;
            context.fonts().tex_id = TextureId::new(1);
        }
        context.io_mut().display_size = [width as f32, height as f32];
        context.io_mut().delta_time = (1.0 / fps.max(1.0)) as f32;
        let ui = context.frame();
        let font = ui.push_font(self.fonts.unwrap()[(font_size.clamp(11, 14) - 11) as usize]);
        ui.window("##timings")
            .position([8.0, 8.0], Condition::Always)
            .bg_alpha(0.55)
            .no_decoration()
            .no_inputs()
            .movable(false)
            .save_settings(false)
            .always_auto_resize(true)
            .build(|| {
                ui.text(format!("Model 1 timing ({fps:.3} Hz)"));
                ui.separator();
                ui.text("61-frame averages");
                for (label, value, warning, critical) in [
                    ("Machine", data.machine, 12.0, 17.0),
                    ("Video", data.video, 12.0, 17.0),
                    ("Audio/pacing", data.audio, 4.0, 8.0),
                    ("retro_run", data.run, 16.0, 20.0),
                    ("Worst", data.worst, 20.0, 34.0),
                ] {
                    let color = if value >= critical {
                        [1.0, 0.3, 0.3, 1.0]
                    } else if value >= warning {
                        [1.0, 1.0, 0.0, 1.0]
                    } else {
                        [0.4, 1.0, 0.4, 1.0]
                    };
                    ui.text_colored(color, format!("{label:12}: {value:5.1} ms"));
                }
                ui.text(format!("Actual      : {:5.1} FPS", data.actual_fps));
                ui.text(format!("Engine cap  : {:5.1} FPS", data.engine_fps));
                ui.text(format!("Callback cap: {:5.1} FPS", data.callback_fps));
            });
        drop(font);
        let draw_data = context.render();
        // ImGui may hide an auto-sized window for its first layout frame.
        // imgui 0.12's zero-list iterator otherwise forms a null empty slice.
        if draw_data.draw_lists_count() == 0 {
            self.context = Some(context.suspend());
            return;
        }
        for list in draw_data.draw_lists() {
            for command in list.commands() {
                if let DrawCmd::Elements { count, cmd_params } = command {
                    if cmd_params.texture_id != TextureId::new(1) {
                        continue;
                    }
                    for indices in list.idx_buffer()
                        [cmd_params.idx_offset..cmd_params.idx_offset + count]
                        .chunks_exact(3)
                    {
                        let vertex =
                            |index| list.vtx_buffer()[cmd_params.vtx_offset + index as usize];
                        self.triangle(
                            frame,
                            width,
                            height,
                            [vertex(indices[0]), vertex(indices[1]), vertex(indices[2])],
                            cmd_params.clip_rect,
                        );
                    }
                }
            }
        }
        self.context = Some(context.suspend());
    }

    fn triangle(
        &self,
        frame: &mut [u32],
        width: usize,
        height: usize,
        vertices: [DrawVert; 3],
        clip: [f32; 4],
    ) {
        let [a, b, c] = vertices;
        let area = (b.pos[0] - a.pos[0]) * (c.pos[1] - a.pos[1])
            - (b.pos[1] - a.pos[1]) * (c.pos[0] - a.pos[0]);
        if area.abs() < 0.0001 {
            return;
        }
        let min_x = clip[0]
            .max(a.pos[0].min(b.pos[0]).min(c.pos[0]))
            .floor()
            .max(0.0) as usize;
        let min_y = clip[1]
            .max(a.pos[1].min(b.pos[1]).min(c.pos[1]))
            .floor()
            .max(0.0) as usize;
        let max_x = (clip[2]
            .min(a.pos[0].max(b.pos[0]).max(c.pos[0]))
            .ceil()
            .max(0.0) as usize)
            .min(width);
        let max_y = (clip[3]
            .min(a.pos[1].max(b.pos[1]).max(c.pos[1]))
            .ceil()
            .max(0.0) as usize)
            .min(height);
        for y in min_y..max_y {
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let wa =
                    ((b.pos[0] - px) * (c.pos[1] - py) - (b.pos[1] - py) * (c.pos[0] - px)) / area;
                let wb =
                    ((c.pos[0] - px) * (a.pos[1] - py) - (c.pos[1] - py) * (a.pos[0] - px)) / area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let u = wa * a.uv[0] + wb * b.uv[0] + wc * c.uv[0];
                let v = wa * a.uv[1] + wb * b.uv[1] + wc * c.uv[1];
                let tx = ((u * self.texture_width as f32) as usize).min(self.texture_width - 1);
                let ty = ((v * self.texture_height as f32) as usize).min(self.texture_height - 1);
                let texel = &self.texture[(ty * self.texture_width + tx) * 4..][..4];
                let color = |channel: usize| {
                    (wa * a.col[channel] as f32
                        + wb * b.col[channel] as f32
                        + wc * c.col[channel] as f32)
                        * texel[channel] as f32
                        / 255.0
                };
                let alpha = color(3) / 255.0;
                let pixel = &mut frame[y * width + x];
                // Software draws over the complete image. GPU draws over a
                // sparse foreground: preserve alpha until the 3D resolve.
                let destination_alpha = if self.transparent_target {
                    if *pixel >> 24 >= 254 { 1.0 } else { (*pixel >> 24) as f32 / 255.0 }
                } else { 1.0 };
                let output_alpha = alpha + destination_alpha * (1.0 - alpha);
                if output_alpha == 0.0 { continue; }
                let mut output = ((output_alpha * 255.0).round() as u32) << 24;
                for (channel, shift) in [(0, 16), (1, 8), (2, 0)] {
                    let value = (((*pixel >> shift) & 255) as f32
                        * destination_alpha * (1.0 - alpha) + color(channel) * alpha)
                        / output_alpha;
                    output |= (value.round().clamp(0.0, 255.0) as u32) << shift;
                }
                *pixel = output;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    static PANEL_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn averages_and_cadence_include_the_complete_window() {
        let mut timing = Timing::default();
        let start = Instant::now();
        for index in 0..61 {
            timing.record(
                start + Duration::from_millis(index * 20),
                4.0,
                2.0,
                1.0,
                8.0,
            );
        }
        let data = timing.published.unwrap();
        assert_eq!(data.machine, 4.0);
        assert_eq!(data.run, 8.0);
        assert_eq!(data.actual_fps, 50.0);
        assert_eq!(data.callback_fps, 125.0);
        timing.reset();
        assert!(timing.published.is_none());
    }

    #[test]
    fn sparse_foreground_retains_panel_alpha_and_matches_software_composition() {
        let _guard = PANEL_TEST.lock().unwrap();
        let mut panel = Panel::default();
        let mut foreground = vec![0; 496 * 384];
        // Native FE foreground tiles are opaque, including underneath the panel.
        foreground[30 * 496 + 30] = 0xfe12_3456;
        panel.draw_foreground(&mut foreground, 496, 384, 13, 60.0,
            Some(Measurements::default()));
        foreground.fill(0);
        foreground[30 * 496 + 30] = 0xfe12_3456;
        panel.draw_foreground(&mut foreground, 496, 384, 13, 60.0,
            Some(Measurements::default()));
        assert_eq!(foreground[383 * 496 + 495], 0);
        assert_eq!(foreground[30 * 496 + 30] >> 24, 255);
        assert!(foreground.iter().any(|p| (130..150).contains(&(p >> 24))));
        let mut software = vec![0xff80_a0c0; 496 * 384];
        software[30 * 496 + 30] = 0xfe12_3456;
        panel.draw(&mut software, 496, 384, 13, 60.0, Some(Measurements::default()));
        for (&fg, &expected) in foreground.iter().zip(&software) {
            if fg == 0 || fg >> 24 >= 254 { continue; }
            let alpha = fg >> 24;
            for (shift, base) in [(16, 128), (8, 160), (0, 192)] {
                let resolved = (((fg >> shift) & 255) * alpha + base * (255 - alpha) + 127) / 255;
                assert!(resolved.abs_diff((expected >> shift) & 255) <= 2);
            }
        }
    }

    #[test]
    fn panel_draws_readable_content_inside_frame_bounds() {
        let _guard = PANEL_TEST.lock().unwrap();
        let mut panel = Panel::default();
        for size in 11..=14 {
            let mut frame = vec![0x304050; 496 * 384];
            for _ in 0..2 {
                panel.draw(
                    &mut frame,
                    496,
                    384,
                    size,
                    60.0,
                    Some(Measurements {
                        machine: 4.5,
                        video: 2.1,
                        audio: 0.3,
                        run: 7.2,
                        worst: 9.3,
                        actual_fps: 60.0,
                        engine_fps: 151.5,
                        callback_fps: 138.9,
                    }),
                );
            }
            assert!(frame.iter().filter(|&&pixel| pixel != 0x304050).count() > 1000);
            assert!(frame
                .iter()
                .filter(|&&pixel| pixel != 0x304050)
                .all(|pixel| pixel >> 24 == 255));
            assert_eq!(frame[383 * 496 + 495], 0x304050);
            if size == 13 {
                if let Ok(path) = std::env::var("TGPULSE_OVERLAY_PREVIEW") {
                    let rgb: Vec<u8> = frame
                        .iter()
                        .flat_map(|pixel| [(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8])
                        .collect();
                    std::fs::write(path, rgb).unwrap();
                    let start = Instant::now();
                    for _ in 0..120 {
                        panel.draw(
                            &mut frame,
                            496,
                            384,
                            size,
                            60.0,
                            Some(Measurements::default()),
                        );
                    }
                    eprintln!(
                        "Overlay draw mean (120 warm draws): {:.3} ms",
                        start.elapsed().as_secs_f64() * 1000.0 / 120.0
                    );
                }
            }
        }
    }
}
