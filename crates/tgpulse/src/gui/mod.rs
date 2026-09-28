//! The Dear ImGui front end.
//!
//! The interface is drawn over the emulated image and is the same whether a
//! game is running or not: with nothing loaded, the library window fills the
//! screen; with a game running, F1 brings the same windows back over it.
//!
//! Nothing here touches the machine. Each frame produces a list of `Action`s
//! that the application applies, which keeps the emulator's state in one place
//! and makes the UI replaceable.

mod platform;
mod renderer;

use std::path::PathBuf;
use std::time::Duration;

use tgpulse_core::config::Config;
use tgpulse_core::library::{self, Entry};
use tgpulse_core::sound::AudioSource;
use tgpulse_core::tilemap::{SCREEN_H, SCREEN_W};

use crate::bindings::{Bindings, GamepadBackend, Hotkey, Source};
use crate::input::players::{PadDevice, Player};
use crate::input::signals::Signal;

pub use renderer::Renderer;

/// How much larger everything is drawn than on a desktop. A phone is held at
/// arm's length and has no pointer to aim with, so both the text and the hit
/// targets have to grow; everywhere else this is 1 and nothing changes.
const UI_SCALE: f32 = if cfg!(target_os = "android") {
    2.5
} else {
    1.0
};

/// What the interface is asking the application to do.
pub enum Action {
    Launch(PathBuf),
    CloseGame,
    Reset,
    SaveState(u32),
    LoadState(u32),
    /// A line typed into the debugger console.
    Debug(String),
    /// Settings were edited; the application re-reads them.
    SettingsChanged,
    NetworkSettingsChanged(crate::network::Config),
    /// Bindings were edited; the application re-reads and saves them.
    BindingsChanged,
    /// The ROM directory should be scanned again.
    RefreshLibrary,
    Quit,
}

/// Live numbers the status window shows.
#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub video_fps: f32,
    pub emulated_fps: f32,
    pub render_ms: f32,
}

/// Panels opened for this launch only, independent of emulation settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartupPanels {
    pub show_stats: bool,
    pub show_debugger: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) static IMGUI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn state_feedback_draws_without_the_menu_or_input_capture() {
        let _lock = IMGUI_TEST_LOCK.lock().unwrap();
        let mut gui = Gui::new(&Config::default(), StartupPanels::default());
        gui.visible = false;
        gui.set_suppressed(true);
        gui.context.io_mut().display_size = [800.0, 600.0];
        gui.context.fonts().build_rgba32_texture();
        for error in [false, true] {
            gui.report_state("Slot 0 test feedback".into(), error);
            assert!(!gui.showing());
            for frame in 0..2 {
                let ui = gui.context.frame();
                state_notice(ui, gui.state_message.as_ref());
                assert!(!ui.is_any_item_active());
                let draw = gui.context.render();
                // ImGui measures a newly auto-sized window on its first frame.
                if frame > 0 {
                    assert!(draw.total_vtx_count > 0);
                }
            }
        }
    }

    #[test]
    fn startup_panels_initialize_existing_windows() {
        let _lock = IMGUI_TEST_LOCK.lock().unwrap();
        // Keep ImGui contexts sequential: only one can be active at a time.
        for show_stats in [false, true] {
            for show_debugger in [false, true] {
                let panels = StartupPanels {
                    show_stats,
                    show_debugger,
                };
                let gui = Gui::new(&Config::default(), panels);
                assert_eq!(gui.show_stats, show_stats);
                assert_eq!(gui.show_debugger, show_debugger);
                assert!(gui.visible);
                assert!(!gui.suppressed);
            }
        }
    }

    #[test]
    fn audio_gain_rows_fit_and_do_not_modify_idle_preferences() {
        let _lock = IMGUI_TEST_LOCK.lock().unwrap();
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [800.0, 600.0];
        context.fonts().build_rgba32_texture();
        let mut reset_held = false;
        for _ in 0..2 {
            let ui = context.frame();
            ui.window("Audio rows")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([420.0, 300.0], imgui::Condition::Always)
                .build(|| {
                    for (label, reference) in [
                        ("MultiPCM 1", 50),
                        ("MultiPCM 2", 50),
                        ("FM (YM3438)", 30),
                        ("DSB (MPEG)", 100),
                        ("SCSP", 100),
                    ] {
                        let mut gain = reference;
                        let mut muted = false;
                        assert!(!audio_gain_row(
                            ui,
                            label,
                            &mut gain,
                            &mut muted,
                            reference,
                            &mut reset_held
                        ));
                        assert_eq!(gain, reference);
                        assert!(!muted);
                        assert!(ui.item_rect_max()[0] <= ui.window_pos()[0] + ui.window_size()[0]);
                    }
                });
            assert!(context.render().total_vtx_count > 0);
        }
    }
}

#[cfg(test)]
mod gain_interaction_tests {
    use super::*;
    #[test]
    fn settings_network_draft_does_not_apply_itself_on_draw() {
        let _lock = tests::IMGUI_TEST_LOCK.lock().unwrap();
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [1024.0, 1200.0];
        context.fonts().build_rgba32_texture();
        let mut config = Config::default();
        let mut network = crate::network::Config {
            port_in: 18000,
            ..Default::default()
        };
        let expected = network.clone();
        for _ in 0..2 {
            let mut actions = Vec::new();
            settings_window(
                context.frame(),
                &mut config,
                &[],
                &mut network,
                "TCP: waiting",
                &mut false,
                &mut true,
                &mut actions,
            );
            assert!(context.render().total_vtx_count > 0);
            assert!(actions.is_empty(), "network changes require Apply");
            assert_eq!(network, expected);
        }
    }

    #[test]
    fn reference_is_below_translucent_grab_and_native_style_is_unchanged() {
        let _lock = tests::IMGUI_TEST_LOCK.lock().unwrap();
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [800.0, 600.0];
        context.fonts().build_rgba32_texture();
        context.style_mut().colors[imgui::StyleColor::FrameBg as usize] = [0.2, 0.4, 0.8, 1.0];
        context.style_mut().colors[imgui::StyleColor::SliderGrab as usize] = [0.4, 0.6, 1.0, 1.0];
        let reference =
            imgui::ImColor32::from_rgba_f32s(0.2 * 0.55, 0.4 * 0.55, 0.8 * 0.55, 1.0).to_rgba();
        let grab = imgui::ImColor32::from_rgba_f32s(0.4, 0.6, 1.0, 0.5).to_rgba();
        let master_grab = imgui::ImColor32::from_rgba_f32s(0.4, 0.6, 1.0, 1.0).to_rgba();
        let mut gain = 50;
        let mut master = 100;
        let mut mute = false;
        let mut held = false;
        for _ in 0..2 {
            let ui = context.frame();
            ui.window("Layers")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([420.0, 300.0], imgui::Condition::Always)
                .build(|| {
                    let before = ui.clone_style();
                    audio_gain_row(ui, "MultiPCM 1", &mut gain, &mut mute, 50, &mut held);
                    assert_eq!(ui.clone_style().colors, before.colors);
                    ui.slider("Native slider", 0, 800, &mut master);
                });
            let data = context.render();
            let colours: Vec<_> = data
                .draw_lists()
                .flat_map(|list| {
                    list.idx_buffer()
                        .iter()
                        .map(|idx| list.vtx_buffer()[*idx as usize].col)
                })
                .collect();
            let marker_at = colours
                .iter()
                .position(|c| *c == reference)
                .expect("dark blue reference");
            let grab_at = colours
                .iter()
                .position(|c| *c == grab)
                .expect("50% opaque grab");
            let master_at = colours
                .iter()
                .position(|c| *c == master_grab)
                .expect("unmodified master grab");
            assert!(marker_at < grab_at && grab_at < master_at);
        }
    }

    #[test]
    fn channel_double_click_resets_only_that_gain_and_survives_held_click() {
        let _lock = tests::IMGUI_TEST_LOCK.lock().unwrap();
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [800.0, 600.0];
        context.io_mut().delta_time = 1.0 / 60.0;
        context.fonts().build_rgba32_texture();
        let (mut gain, mut other, mut master) = (80, 25, 400);
        let (mut muted, mut other_muted, mut reset_held) = (true, false, false);
        let mut point = [0.0, 0.0];
        for frame in 0..7 {
            if frame >= 2 {
                context.io_mut().add_mouse_pos_event(point);
                context
                    .io_mut()
                    .add_mouse_button_event(imgui::MouseButton::Left, matches!(frame, 2 | 4 | 5));
            }
            let ui = context.frame();
            ui.window("Double click")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([420.0, 300.0], imgui::Condition::Always)
                .build(|| {
                    let origin = ui.cursor_screen_pos();
                    point = [origin[0] + 25.0, origin[1] + 8.0];
                    let changed = audio_gain_row(
                        ui,
                        "MultiPCM 1",
                        &mut gain,
                        &mut muted,
                        50,
                        &mut reset_held,
                    );
                    if frame == 4 {
                        assert!(changed, "reset must be persisted");
                    }
                    audio_gain_row(
                        ui,
                        "Other",
                        &mut other,
                        &mut other_muted,
                        30,
                        &mut reset_held,
                    );
                    master_volume_slider(ui, &mut master, &mut reset_held);
                });
            context.render();
            if frame == 2 {
                assert!(gain < 50, "first click adjusts normally");
            }
            if frame >= 4 {
                assert_eq!(gain, 50, "frame {frame}");
            }
            assert_eq!(other, 25);
            assert!(muted);
            assert_eq!(master, 400);
        }
    }

    #[test]
    fn master_double_click_restores_100_without_changing_channels() {
        let _lock = tests::IMGUI_TEST_LOCK.lock().unwrap();
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [800.0, 600.0];
        context.io_mut().delta_time = 1.0 / 60.0;
        context.fonts().build_rgba32_texture();
        let (mut master, mut gain) = (800, 25);
        let (mut muted, mut reset_held) = (true, false);
        let mut point = [0.0, 0.0];
        for frame in 0..7 {
            if frame >= 2 {
                context.io_mut().add_mouse_pos_event(point);
                context
                    .io_mut()
                    .add_mouse_button_event(imgui::MouseButton::Left, matches!(frame, 2 | 4 | 5));
            }
            let ui = context.frame();
            ui.window("Master reset")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([420.0, 300.0], imgui::Condition::Always)
                .build(|| {
                    let origin = ui.cursor_screen_pos();
                    point = [origin[0] + 150.0, origin[1] + 8.0];
                    let changed = master_volume_slider(ui, &mut master, &mut reset_held);
                    if frame == 4 {
                        assert!(changed, "reset must be persisted");
                    }
                    audio_gain_row(ui, "MultiPCM 1", &mut gain, &mut muted, 50, &mut reset_held);
                });
            context.render();
            if frame < 2 {
                assert_eq!(master, 800);
            }
            if frame == 2 {
                assert!(
                    master > 100 && master < 800,
                    "normal adjustment preserves the master range"
                );
            }
            if frame >= 4 {
                assert_eq!(master, 100, "frame {frame}");
            }
            assert_eq!(gain, 25);
            assert!(muted);
        }
    }
}

pub struct Gui {
    pub network_draft: crate::network::Config,
    pub network_status: String,
    context: imgui::Context,
    pub controller_devices: Vec<PadDevice>,
    pub controller_labels: [String; 2],
    /// The running machine's implemented outputs; empty in the library.
    pub audio_sources: &'static [AudioSource],

    /// Whether the player wants the interface up.
    pub visible: bool,
    /// Set while something else should have the screen to itself.
    suppressed: bool,
    show_settings: bool,
    audio_gain_reset_held: bool,
    show_input: bool,
    show_debugger: bool,
    show_stats: bool,

    /// What the next key press should be bound to, while the panel is waiting
    /// for one.
    awaiting: Option<Awaiting>,
    binding_editor: BindingEditor,

    entries: Vec<Entry>,
    selected: Option<usize>,
    library_error: Option<String>,
    state_message: Option<(String, bool, std::time::Instant)>,

    debug_input: String,
    debug_log: Vec<String>,
    debug_follow: bool,

    /// Save-state slot the hotkeys and the buttons act on.
    pub state_slot: u32,
    captured: Option<(Awaiting, winit::keyboard::KeyCode)>,
}

/// What a pending key press will be bound to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Awaiting {
    Hotkey(Hotkey),
}

#[derive(Default)]
struct BindingEditor {
    return_to_menu: Option<String>,
    return_to_menu_error: Option<String>,
    signal: Option<Signal>,
    player: Player,
    text: String,
    error: Option<String>,
}

impl Gui {
    pub fn wants_text_input(&self) -> bool {
        self.context.io().want_text_input
    }

    pub fn new(config: &Config, panels: StartupPanels) -> Self {
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);
        // A sane display size before the first `Resized`: laying windows out
        // against a zero-sized display collapses them.
        context.io_mut().display_size = [(SCREEN_W * 2) as f32, (SCREEN_H * 2) as f32];
        style(&mut context);

        // Nothing on a handset is legible at the default 13 pixels, and the
        // touch interface draws its labels from this same atlas. The atlas is
        // not built until the renderer asks for it, so the size only has to be
        // settled before then.
        if UI_SCALE > 1.0 {
            context
                .fonts()
                .add_font(&[imgui::FontSource::DefaultFontData {
                    config: Some(imgui::FontConfig {
                        size_pixels: 13.0 * UI_SCALE,
                        ..imgui::FontConfig::default()
                    }),
                }]);
            context.style_mut().scale_all_sizes(UI_SCALE);
        }

        Self {
            context,
            network_draft: crate::network::Config::default(),
            network_status: String::new(),
            audio_sources: &[],
            visible: true,
            suppressed: false,
            show_settings: false,
            audio_gain_reset_held: false,
            show_input: false,
            awaiting: None,
            binding_editor: BindingEditor::default(),
            controller_devices: Vec::new(),
            controller_labels: ["No controller".into(), "No controller".into()],
            show_debugger: panels.show_debugger,
            show_stats: panels.show_stats,
            entries: library::scan(&config.rom_dir),
            selected: None,
            library_error: None,
            state_message: None,
            debug_input: String::new(),
            debug_log: Vec::new(),
            debug_follow: true,
            state_slot: 0,
            captured: None,
        }
    }

    /// Re-reads the ROM directory.
    pub fn refresh_library(&mut self, config: &Config) {
        self.entries = library::scan(&config.rom_dir);
        if self.selected.is_some_and(|i| i >= self.entries.len()) {
            self.selected = None;
        }
    }

    /// Takes a key press for whatever the input panel is waiting to bind.
    /// Returns whether it was consumed.
    pub fn capture_key(&mut self, key: winit::keyboard::KeyCode) -> bool {
        let Some(awaiting) = self.awaiting.take() else {
            return false;
        };
        // Escape abandons the binding rather than binding Escape, which would
        // leave no way out of the panel.
        if key == winit::keyboard::KeyCode::Escape {
            return true;
        }
        self.captured = Some((awaiting, key));
        true
    }

    /// The binding the panel captured since the last call, if any.
    pub fn take_capture(&mut self) -> Option<(Awaiting, winit::keyboard::KeyCode)> {
        self.captured.take()
    }

    pub fn report_error(&mut self, message: impl Into<String>) {
        self.library_error = Some(message.into());
    }

    pub fn report_state(&mut self, message: String, error: bool) {
        let duration = Duration::from_secs(if error { 10 } else { 3 });
        self.state_message = Some((message, error, std::time::Instant::now() + duration));
    }

    /// Appends debugger output.
    pub fn push_debug_output(&mut self, lines: impl IntoIterator<Item = String>) {
        self.debug_log.extend(lines);
        // Keep the console bounded; a `run` over thousands of frames can emit a
        // great deal and the scrollback is not the transcript of record.
        const LIMIT: usize = 4000;
        if self.debug_log.len() > LIMIT {
            self.debug_log.drain(..self.debug_log.len() - LIMIT);
        }
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        let captured = platform::handle_event(self.context.io_mut(), window, event);
        // Events are only stolen from the machine while the interface is up.
        captured && self.showing()
    }

    /// Builds a frame and returns what the user asked for.
    ///
    /// `running` is the title of the game currently loaded, if any.
    // A frame needs the renderer, the clock, and every piece of state the
    // windows can edit; bundling them into a struct would only move the list.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        renderer: &mut Renderer,
        dt: Duration,
        running: Option<&str>,
        config: &mut Config,
        bindings: &mut Bindings,
        stats: Stats,
        touch: Option<&mut crate::touch::TouchUi>,
    ) -> Vec<Action> {
        self.context.io_mut().delta_time = dt.as_secs_f32().max(1.0 / 1000.0);
        if self
            .state_message
            .as_ref()
            .is_some_and(|(_, _, until)| std::time::Instant::now() >= *until)
        {
            self.state_message = None;
        }

        // A handset drives the touch interface instead of these windows: they
        // are built for a pointer that can hover and a keyboard that can type,
        // and it has neither.
        if let Some(touch) = touch.filter(|t| t.enabled()) {
            let ui = self.context.frame();
            let actions = touch.render(ui, &self.entries, running, config, &mut self.state_slot);
            state_notice(ui, self.state_message.as_ref());
            renderer.capture(self.context.render());
            return actions;
        }

        let mut actions = Vec::new();
        if !self.showing() {
            // Still render, so the stats window can stay up during play.
            if self.show_stats || self.state_message.is_some() {
                let ui = self.context.frame();
                if self.show_stats {
                    stats_window(ui, stats);
                }
                state_notice(ui, self.state_message.as_ref());
                renderer.capture(self.context.render());
            } else {
                // Clear the renderer's old notice after it expires.
                let _ = self.context.frame();
                renderer.capture(self.context.render());
            }
            return actions;
        }

        // Fields the closures need but cannot borrow through `self`.
        let (
            mut show_settings,
            mut show_input,
            mut show_debugger,
            mut show_stats,
            mut selected,
            mut debug_input,
            mut state_slot,
        ) = (
            self.show_settings,
            self.show_input,
            self.show_debugger,
            self.show_stats,
            self.selected,
            std::mem::take(&mut self.debug_input),
            self.state_slot,
        );
        let entries = &self.entries;
        let debug_log = &self.debug_log;
        let debug_follow = &mut self.debug_follow;
        let library_error = &self.library_error;
        let mut awaiting = self.awaiting;
        let mut binding_editor = std::mem::take(&mut self.binding_editor);
        let mut refresh = false;

        let ui = self.context.frame();

        ui.main_menu_bar(|| {
            ui.menu("File", || {
                if ui.menu_item("Refresh library") {
                    refresh = true;
                }
                if ui
                    .menu_item_config("Close game")
                    .enabled(running.is_some())
                    .build()
                {
                    actions.push(Action::CloseGame);
                }
                ui.separator();
                if ui.menu_item("Quit") {
                    actions.push(Action::Quit);
                }
            });
            ui.menu("Machine", || {
                let enabled = running.is_some();
                if ui.menu_item_config("Reset").enabled(enabled).build() {
                    actions.push(Action::Reset);
                }
                ui.separator();
                if ui
                    .menu_item_config(format!("Save state (slot {state_slot})"))
                    .shortcut("F5")
                    .enabled(enabled)
                    .build()
                {
                    actions.push(Action::SaveState(state_slot));
                }
                if ui
                    .menu_item_config(format!("Load state (slot {state_slot})"))
                    .shortcut("F7")
                    .enabled(enabled)
                    .build()
                {
                    actions.push(Action::LoadState(state_slot));
                }
            });
            ui.menu("View", || {
                ui.checkbox("Settings", &mut show_settings);
                ui.checkbox("Input", &mut show_input);
                ui.checkbox("Debugger", &mut show_debugger);
                ui.checkbox("Statistics", &mut show_stats);
            });
            // The right-hand side of the bar is a status line.
            match running {
                Some(title) => {
                    ui.text_disabled(format!("  |  {title}  |  F1 hides this, F11 fullscreen"));
                }
                None => ui.text_disabled("  |  no game loaded"),
            }
        });

        // The library is the main screen's window, not an overlay: with a game
        // running there is nothing to pick, and it would sit over the picture.
        // Close the game and it comes back.
        if running.is_none() {
            library_window(
                ui,
                ui.io().display_size,
                entries,
                library_error.as_deref(),
                &mut selected,
                &mut refresh,
                &mut actions,
            );
        }

        if show_settings {
            settings_window(
                ui,
                config,
                self.audio_sources,
                &mut self.network_draft,
                &self.network_status,
                &mut self.audio_gain_reset_held,
                &mut show_settings,
                &mut actions,
            );
        }
        if show_input {
            input_window(
                ui,
                bindings,
                &self.controller_devices,
                &self.controller_labels,
                &mut awaiting,
                &mut binding_editor,
                &mut show_input,
                &mut actions,
            );
        }
        if show_debugger {
            debugger_window(
                ui,
                debug_log,
                &mut debug_input,
                debug_follow,
                &mut show_debugger,
                &mut actions,
            );
        }
        if show_stats {
            stats_window(ui, stats);
        }

        let _ = &mut state_slot;
        state_notice(ui, self.state_message.as_ref());
        renderer.capture(self.context.render());

        self.show_settings = show_settings;
        self.show_input = show_input;
        self.awaiting = awaiting;
        self.binding_editor = binding_editor;
        self.show_debugger = show_debugger;
        self.show_stats = show_stats;
        self.selected = selected;
        self.debug_input = debug_input;
        self.state_slot = state_slot;
        if refresh {
            self.refresh_library(config);
            self.library_error = None;
        }
        actions
    }

    /// Builds a renderer for the current surface. The interface's own state
    /// outlives it, so a suspended activity comes back with its library scan,
    /// its console scrollback and its open windows intact.
    pub fn build_renderer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Renderer {
        let atlas = self.context.fonts().build_rgba32_texture();
        Renderer::new(device, queue, format, &atlas)
    }

    /// Hides the interface without forgetting that the player wanted it up.
    pub fn set_suppressed(&mut self, suppressed: bool) {
        self.suppressed = suppressed;
    }

    fn showing(&self) -> bool {
        self.visible && !self.suppressed
    }

    pub fn set_display_size(&mut self, window: &winit::window::Window) {
        let size = window.inner_size();
        let scale = window.scale_factor();
        let logical_size =
            winit::dpi::PhysicalSize::new(size.width, size.height).to_logical::<f32>(scale);
        self.context.io_mut().display_size = [logical_size.width, logical_size.height];
        let scale = scale as f32;
        self.context.io_mut().display_framebuffer_scale = [scale, scale];
    }
}

fn library_window(
    ui: &imgui::Ui,
    display: [f32; 2],
    entries: &[Entry],
    error: Option<&str>,
    selected: &mut Option<usize>,
    refresh: &mut bool,
    actions: &mut Vec<Action>,
) {
    // Centred on the display the first time it is laid out. `FirstUseEver`
    // rather than `Always` so dragging it somewhere else sticks.
    ui.window("ROM library")
        .size([560.0, 380.0], imgui::Condition::FirstUseEver)
        .position(
            [display[0] * 0.5, display[1] * 0.5],
            imgui::Condition::FirstUseEver,
        )
        .position_pivot([0.5, 0.5])
        .build(|| {
            if ui.button("Refresh") {
                *refresh = true;
            }
            ui.same_line();
            let can_launch = selected
                .and_then(|i| entries.get(i))
                .is_some_and(Entry::is_known);
            ui.enabled(can_launch, || {
                if ui.button("Play") {
                    if let Some(entry) = selected.and_then(|i| entries.get(i)) {
                        actions.push(Action::Launch(entry.path.clone()));
                    }
                }
            });
            ui.same_line();
            ui.text_disabled(format!("{} romset(s)", entries.len()));

            if let Some(error) = error {
                ui.text_colored([1.0, 0.45, 0.4, 1.0], error);
            }
            ui.separator();

            ui.child_window("list")
                .size([0.0, -ui.frame_height_with_spacing() * 3.0])
                .build(|| {
                    if entries.is_empty() {
                        ui.text_wrapped(
                            "Nothing here yet. Put zipped romsets in the roms \
                             directory and press Refresh.",
                        );
                        return;
                    }
                    for (i, entry) in entries.iter().enumerate() {
                        let label = if entry.is_known() {
                            format!("{}  ({})", entry.title, entry.set)
                        } else {
                            format!("{}  -- unrecognised", entry.set)
                        };
                        let colour = if !entry.is_known() {
                            [0.6, 0.6, 0.6, 1.0]
                        } else if entry.is_complete() {
                            [0.9, 0.9, 0.9, 1.0]
                        } else {
                            [1.0, 0.8, 0.4, 1.0]
                        };
                        let token = ui.push_style_color(imgui::StyleColor::Text, colour);
                        if ui
                            .selectable_config(&label)
                            .selected(*selected == Some(i))
                            .build()
                        {
                            *selected = Some(i);
                        }
                        token.pop();
                        // Double-click launches, which is what a list like this
                        // is expected to do.
                        if ui.is_item_hovered()
                            && ui.is_mouse_double_clicked(imgui::MouseButton::Left)
                            && entry.is_known()
                        {
                            actions.push(Action::Launch(entry.path.clone()));
                        }
                    }
                });

            ui.separator();
            match selected.and_then(|i| entries.get(i)) {
                Some(entry) => {
                    ui.text(format!(
                        "{}  {}  {}",
                        entry.year,
                        entry.manufacturer,
                        entry.board.map(|b| b.label()).unwrap_or("unknown board"),
                    ));
                    ui.text_disabled(entry.path.display().to_string());
                    if entry.missing.is_empty() {
                        ui.text_disabled(entry.status());
                    } else {
                        ui.text_colored(
                            [1.0, 0.8, 0.4, 1.0],
                            format!("missing: {}", entry.missing.join(", ")),
                        );
                    }
                }
                None => ui.text_disabled("Select a romset."),
            }
        });
}

fn audio_gain_row(
    ui: &imgui::Ui,
    label: &str,
    gain: &mut u32,
    muted: &mut bool,
    reference: u32,
    reset_held: &mut bool,
) -> bool {
    let _id = ui.push_id(label);
    let style = ui.clone_style();
    // Reserve only the unlabelled checkbox, gaps and widest source name.
    // Use all remaining width for the aligned channel sliders.
    let label_width = [
        "MultiPCM 1",
        "MultiPCM 2",
        "FM (YM3438)",
        "DSB (MPEG)",
        "SCSP",
    ]
    .iter()
    .map(|name| ui.calc_text_size(name)[0])
    .fold(0.0_f32, f32::max);
    let trailing_width = ui.current_font_size()
        + 2.0 * style.frame_padding[1]
        + 2.0 * style.item_spacing[0]
        + label_width;
    ui.set_next_item_width((ui.content_region_avail()[0] - trailing_width).max(80.0));
    let mut changed = audio_gain_slider(
        ui,
        gain,
        reference,
        tgpulse_core::config::AudioGains::MAX,
        reset_held,
    );
    ui.same_line();
    changed |= ui.checkbox("##mute", muted);
    if ui.is_item_hovered() {
        ui.tooltip_text("Mute");
    }
    ui.same_line();
    ui.text(label);
    changed
}

fn master_volume_slider(ui: &imgui::Ui, volume: &mut u32, reset_held: &mut bool) -> bool {
    let _id = ui.push_id("master_volume");
    let changed = audio_gain_slider(ui, volume, 100, 800, reset_held);
    ui.same_line();
    ui.text("Volume %");
    changed
}

fn audio_gain_slider(
    ui: &imgui::Ui,
    gain: &mut u32,
    reference: u32,
    max: u32,
    reset_held: &mut bool,
) -> bool {
    let mut value = (*gain).min(max) as i32;
    if !ui.is_mouse_down(imgui::MouseButton::Left) {
        *reset_held = false;
    }
    let style = ui.clone_style();
    let mut changed = false;
    {
        let draw = ui.get_window_draw_list();
        // Separate the frame/reference from ImGui's native grab and text.
        // This keeps keyboard/numeric editing intact and puts the reference
        // below the translucent grab, not on top of it.
        draw.channels_split(2, |channels| {
            channels.set_current(1);
            {
                let _frame = ui.push_style_color(imgui::StyleColor::FrameBg, [0.0; 4]);
                let _hover = ui.push_style_color(imgui::StyleColor::FrameBgHovered, [0.0; 4]);
                let _active = ui.push_style_color(imgui::StyleColor::FrameBgActive, [0.0; 4]);
                let mut grab = style.colors[imgui::StyleColor::SliderGrab as usize];
                let mut active = style.colors[imgui::StyleColor::SliderGrabActive as usize];
                grab[3] = 0.5;
                active[3] = 0.5;
                let _grab = ui.push_style_color(imgui::StyleColor::SliderGrab, grab);
                let _grab_active = ui.push_style_color(imgui::StyleColor::SliderGrabActive, active);
                changed = ui
                    .slider_config("##gain", 0, max as i32)
                    .display_format("%d%%")
                    .build(&mut value);
            }
            let min = ui.item_rect_min();
            let end = ui.item_rect_max();
            let background = style.colors[if ui.is_item_active() {
                imgui::StyleColor::FrameBgActive
            } else if ui.is_item_hovered() {
                imgui::StyleColor::FrameBgHovered
            } else {
                imgui::StyleColor::FrameBg
            } as usize];
            channels.set_current(0);
            draw.add_rect(min, end, background)
                .filled(true)
                .rounding(style.frame_rounding)
                .build();
            let usable = (end[0] - min[0] - 4.0).max(0.0);
            let grab = (usable / (max + 1) as f32)
                .max(style.grab_min_size)
                .min(usable);
            let x = min[0] + 2.0 + grab * 0.5 + (usable - grab) * reference as f32 / max as f32;
            let reference_colour = [
                background[0] * 0.55,
                background[1] * 0.55,
                background[2] * 0.55,
                1.0,
            ];
            draw.add_line([x, min[1]], [x, end[1]], reference_colour)
                .thickness(2.0)
                .build();
        });
    }
    let reset = ui.is_item_hovered() && ui.is_mouse_double_clicked(imgui::MouseButton::Left);
    if reset {
        *gain = reference;
        *reset_held = true;
        changed = true;
    } else if *reset_held {
        // Do not let the second click turn into a drag on following frames.
        changed = false;
    } else if changed {
        *gain = value.clamp(0, max as i32) as u32;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Default: {reference}%. Double-click to reset; Ctrl-click to type a value."
        ));
    }
    changed
}

fn settings_window(
    ui: &imgui::Ui,
    config: &mut Config,
    audio_sources: &[AudioSource],
    network: &mut crate::network::Config,
    network_status: &str,
    audio_gain_reset_held: &mut bool,
    open: &mut bool,
    actions: &mut Vec<Action>,
) {
    ui.window("Settings")
        .size([420.0, 340.0], imgui::Condition::FirstUseEver)
        .opened(open)
        .build(|| {
            let mut changed = false;

            ui.text_disabled("Video");
            let mut ssaa = config.ssaa as i32;
            if ui.slider("Supersampling", 1, 4, &mut ssaa) {
                config.ssaa = ssaa as u32;
                changed = true;
            }
            ui.text_disabled("1 is the board's own output, without antialiasing.");
            let mut wide = match config.widescreen {
                tgpulse_core::config::Widescreen::Off => 0,
                tgpulse_core::config::Widescreen::On => 1,
                tgpulse_core::config::Widescreen::Auto => 2,
            };
            if ui.combo_simple_string("Widescreen", &mut wide, &["Off", "On", "Auto"]) {
                config.widescreen = [
                    tgpulse_core::config::Widescreen::Off,
                    tgpulse_core::config::Widescreen::On,
                    tgpulse_core::config::Widescreen::Auto,
                ][wide];
                changed = true;
            }
            ui.text_disabled("Auto follows supported games' saved monitor/cabinet setting.");
            changed |= ui.checkbox(
                "Stretch 2D layers when widescreen",
                &mut config.widescreen_stretch_2d,
            );
            changed |= ui.checkbox("Smooth shadows", &mut config.smooth_shadows);
            ui.text_disabled("Blends the hardware's stipple instead of reproducing it.");
            changed |= ui.checkbox("Fullscreen during games", &mut config.fullscreen);
            changed |= ui.checkbox("sRGB correction", &mut config.srgb);
            ui.text_disabled("Preserves framebuffer colours on sRGB displays; off keeps the legacy look.");

            ui.separator();
            ui.text_disabled("Audio");
            changed |= master_volume_slider(ui, &mut config.volume, audio_gain_reset_held);
            ui.text_disabled("Double-click an audio slider to restore its default.");
            ui.text_disabled("SCSP titles mix quiet; try 400 for Sega Rally.");
            if audio_sources.is_empty() {
                ui.text_disabled("Load a game to show its audio sources.");
            } else {
                for source in audio_sources {
                    let reference = tgpulse_core::config::AudioGains::REFERENCE;
                    let (label, muted, gain, default) = match source {
                        AudioSource::MultiPcm1 => (
                            "MultiPCM 1",
                            &mut config.audio_mutes.multipcm1,
                            &mut config.audio_gains.multipcm1,
                            reference.multipcm1,
                        ),
                        AudioSource::MultiPcm2 => (
                            "MultiPCM 2",
                            &mut config.audio_mutes.multipcm2,
                            &mut config.audio_gains.multipcm2,
                            reference.multipcm2,
                        ),
                        AudioSource::Scsp => (
                            "SCSP",
                            &mut config.audio_mutes.scsp,
                            &mut config.audio_gains.scsp,
                            reference.scsp,
                        ),
                        AudioSource::Ym3438 => (
                            "FM (YM3438)",
                            &mut config.audio_mutes.ym3438,
                            &mut config.audio_gains.ym3438,
                            reference.ym3438,
                        ),
                        AudioSource::Dsb => (
                            "DSB (MPEG)",
                            &mut config.audio_mutes.dsb,
                            &mut config.audio_gains.dsb,
                            reference.dsb,
                        ),
                    };
                    changed |=
                        audio_gain_row(ui, label, gain, muted, default, audio_gain_reset_held);
                }
            }

            ui.separator();
            ui.text_disabled("Machine");
            changed |= ui.checkbox("Force feedback to pad rumble", &mut config.rumble);
            let mut twin = config.cabinet == tgpulse_core::config::Cabinet::Twin;
            if ui.checkbox("Network board fitted (twin cabinet)", &mut twin) {
                config.cabinet = if twin {
                    tgpulse_core::config::Cabinet::Twin
                } else {
                    tgpulse_core::config::Cabinet::Single
                };
                changed = true;
            }
            ui.text_disabled("Takes effect the next time a game is loaded.");

            ui.separator();
            ui.text_disabled("Model 1 networking (TCP ring)");
            ui.input_text("AddressIn", &mut network.address_in).build();
            let mut port_in = i32::from(network.port_in);
            if ui.input_int("PortIn", &mut port_in).build() { network.port_in = port_in.clamp(0, 65535) as u16; }
            ui.input_text("AddressOut", &mut network.address_out).build();
            let mut port_out = i32::from(network.port_out);
            if ui.input_int("PortOut", &mut port_out).build() { network.port_out = port_out.clamp(0, 65535) as u16; }
            ui.text_wrapped("Outgoing connects to the next cabinet's incoming endpoint. Configure roles in the game's test menu.");
            let validation = network.endpoints();
            if let Err(error) = &validation { ui.text_wrapped(error); }
            {
                let _disabled = ui.begin_disabled(validation.is_err());
                if ui.button("Apply network settings") {
                    actions.push(Action::NetworkSettingsChanged(network.clone()));
                }
            }
            ui.text_disabled("Apply saves settings; reload/reset the game to connect.");
            ui.text_wrapped(network_status);

            ui.separator();
            if ui.button("Revert to defaults") {
                let shipped = Config::default();
                config.ssaa = shipped.ssaa;
                config.widescreen = shipped.widescreen;
                config.widescreen_stretch_2d = shipped.widescreen_stretch_2d;
                config.smooth_shadows = shipped.smooth_shadows;
                config.fullscreen = shipped.fullscreen;
                config.srgb = shipped.srgb;
                config.volume = shipped.volume;
                config.audio_mutes = shipped.audio_mutes;
                config.audio_gains = shipped.audio_gains;
                config.rumble = shipped.rumble;
                config.cabinet = shipped.cabinet;
                *network = crate::network::Config::default();
                actions.push(Action::NetworkSettingsChanged(network.clone()));
                changed = true;
            }

            if changed {
                actions.push(Action::SettingsChanged);
            }
        });
}

/// One unfiltered signal list with expression editing, plus hotkey capture.
fn input_window(
    ui: &imgui::Ui,
    bindings: &mut Bindings,
    devices: &[PadDevice],
    controller_labels: &[String; 2],
    awaiting: &mut Option<Awaiting>,
    editor: &mut BindingEditor,
    open: &mut bool,
    actions: &mut Vec<Action>,
) {
    ui.window("Input")
        .size([820.0, 600.0], imgui::Condition::FirstUseEver)
        .opened(open)
        .build(|| {
            if awaiting.is_some() {
                ui.text_colored([1.0, 0.85, 0.3, 1.0], "Press a key, or Escape to cancel.");
            } else {
                ui.text_disabled("Emulator: capture a key. Cabinet: edit a binding expression.");
            }
            ui.same_line();
            if ui.button("Revert to defaults") {
                *bindings = Bindings::default();
                *editor = BindingEditor::default();
                actions.push(Action::BindingsChanged);
            }
            ui.separator();

            if let Some(tabs) = ui.tab_bar("input_tabs") {
                if let Some(tab) = ui.tab_item("Emulator") {
                    #[cfg(target_os = "macos")]
                    {
                        let preview = match bindings.gamepad_backend {
                            GamepadBackend::Gilrs => "gilrs",
                            GamepadBackend::Sdl3 => "SDL3",
                        };
                        if let Some(_combo) = ui.begin_combo("Gamepad backend", preview) {
                            for (backend, label) in [
                                (GamepadBackend::Gilrs, "gilrs"),
                                (GamepadBackend::Sdl3, "SDL3"),
                            ] {
                                if ui.selectable_config(label).selected(bindings.gamepad_backend == backend).build() {
                                    bindings.gamepad_backend = backend;
                                    actions.push(Action::BindingsChanged);
                                }
                            }
                        }
                        ui.text_disabled("One gamepad backend at a time; keyboard and cabinet bindings are shared.");
                        ui.separator();
                    }
                    for hotkey in Hotkey::ALL {
                        let bound = bindings
                            .hotkey(*hotkey)
                            .map(|k| Source::Key(k).to_string())
                            .unwrap_or_else(|| "-".into());
                        binding_row(
                            ui,
                            hotkey.label(),
                            &bound,
                            *awaiting == Some(Awaiting::Hotkey(*hotkey)),
                            || *awaiting = Some(Awaiting::Hotkey(*hotkey)),
                        );
                    }
                    let bound = &bindings.return_to_menu.text;
                    binding_row(
                        ui,
                        "Return to game menu / quit",
                        if bound.is_empty() { "Unbound" } else { bound },
                        false,
                        || {
                            *awaiting = None;
                            editor.return_to_menu = Some(bound.clone());
                            editor.return_to_menu_error = None;
                            ui.open_popup("return_to_menu_binding");
                        },
                    );
                    if let Some(_popup) = ui.begin_popup("return_to_menu_binding") {
                        let text = editor.return_to_menu.get_or_insert_with(|| bound.clone());
                        ui.input_text("Expression", text).build();
                        ui.text_disabled("Comma = alternatives; & = simultaneous.");
                        if ui.button("Apply") {
                            match crate::input::signals::expression::Binding::parse(text, false) {
                                Ok(binding) => {
                                    bindings.return_to_menu = binding;
                                    editor.return_to_menu_error = None;
                                    actions.push(Action::BindingsChanged);
                                    ui.close_current_popup();
                                }
                                Err(error) => editor.return_to_menu_error = Some(error),
                            }
                        }
                        ui.same_line();
                        if ui.button("Cancel") {
                            ui.close_current_popup();
                        }
                        if let Some(error) = &editor.return_to_menu_error {
                            ui.text_colored([1.0, 0.4, 0.3, 1.0], error);
                        }
                    }
                    tab.end();
                }
                for player in Player::ALL {
                    if let Some(tab) = ui.tab_item(if player == Player::One {
                        "Cabinet P1"
                    } else {
                        "Cabinet P2"
                    }) {
                        let _id = ui.push_id_usize(player.index());
                        let current = &bindings.controllers[player.index()];
                        let preview = match current.as_str() {
                            "auto" => format!("Auto — {}", controller_labels[player.index()]),
                            "none" => "None (keyboard only)".into(),
                            _ => controller_labels[player.index()].clone(),
                        };
                        if let Some(_combo) = ui.begin_combo("Controller", preview) {
                            for (key, label) in [
                                ("auto", "Automatic (distinct device per player)"),
                                ("none", "None (keyboard only)"),
                            ]
                            .into_iter()
                            .map(|(k, l)| (k.to_owned(), l.to_owned()))
                            .chain(devices.iter().map(|d| {
                                (
                                    d.key.clone(),
                                    format!(
                                        "{}{}",
                                        d.label,
                                        if d.connected { "" } else { " (disconnected)" }
                                    ),
                                )
                            })) {
                                if ui
                                    .selectable_config(&label)
                                    .selected(bindings.controllers[player.index()] == key)
                                    .build()
                                {
                                    bindings.set_controller(player, key);
                                    actions.push(Action::BindingsChanged);
                                }
                            }
                        }
                        ui.text_wrapped("Comma = alternatives; & = simultaneous.");
                        if let Some(signal) = editor.signal.filter(|s| {
                            editor.player == player && (player == Player::One || s.supports_p2())
                        }) {
                            ui.text(cabinet_signal_label(player, signal));
                            ui.input_text("Expression", &mut editor.text).build();
                            if ui.button("Apply") {
                                match bindings.set_player_expression(player, signal, &editor.text) {
                                    Ok(()) => {
                                        editor.error = None;
                                        actions.push(Action::BindingsChanged);
                                    }
                                    Err(e) => editor.error = Some(e),
                                }
                            }
                            ui.same_line();
                            if ui.button("Cancel") {
                                *editor = BindingEditor::default();
                            }
                            if let Some(error) = &editor.error {
                                ui.text_colored([1.0, 0.4, 0.3, 1.0], error);
                            }
                            ui.separator();
                        }
                        for signal in Signal::ALL {
                            if *signal == Signal::GunYaw || *signal == Signal::Elevation {
                                ui.separator();
                            }
                            cabinet_signal_row(ui, player, *signal, bindings, editor);
                        }
                        tab.end();
                    }
                }
                tabs.end();
            }
        });
}

fn cabinet_signal_row(
    ui: &imgui::Ui,
    player: Player,
    signal: Signal,
    bindings: &Bindings,
    editor: &mut BindingEditor,
) {
    let enabled = player == Player::One || signal.supports_p2();
    let _disabled = ui.begin_disabled(!enabled);
    let bound = &bindings.player_binding(player, signal).text;
    binding_row(
        ui,
        cabinet_signal_label(player, signal),
        if !enabled {
            "Not available for P2"
        } else if bound.is_empty() {
            "Unbound"
        } else {
            bound
        },
        editor.player == player && editor.signal == Some(signal),
        || {
            editor.player = player;
            editor.signal = Some(signal);
            editor.text = bound.clone();
            editor.error = None;
        },
    );
    let usage = match (player, signal, signal.usage()) {
        (Player::One, _, usage) => usage,
        (Player::Two, Signal::SkyX | Signal::SkyY, Some(_)) => Some("(Star Wars Arcade (Gunner))"),
        _ => None,
    };
    if let Some(usage) = usage {
        ui.text_disabled(usage);
        ui.spacing();
    }
}

fn cabinet_signal_label(player: Player, signal: Signal) -> &'static str {
    match (player, signal) {
        (Player::One, Signal::SwaLaser) => "Star Wars Arcade (Pilot): Laser",
        (Player::One, Signal::SwaTorpedo) => "Star Wars Arcade (Pilot): Torpedo",
        (Player::Two, Signal::SwaLaser) => "Star Wars Arcade (Gunner): Laser",
        (Player::Two, Signal::SwaTorpedo) => "Star Wars Arcade (Gunner): Torpedo",
        _ => signal.label(),
    }
}

#[cfg(test)]
mod player_binding_tests {
    use super::*;
    #[test]
    fn star_wars_arcade_labels_distinguish_pilot_and_gunner() {
        assert_eq!(
            cabinet_signal_label(Player::One, Signal::SwaLaser),
            "Star Wars Arcade (Pilot): Laser"
        );
        assert_eq!(
            cabinet_signal_label(Player::Two, Signal::SwaLaser),
            "Star Wars Arcade (Gunner): Laser"
        );
        assert_eq!(
            cabinet_signal_label(Player::One, Signal::SwaTorpedo),
            "Star Wars Arcade (Pilot): Torpedo"
        );
        assert_eq!(
            cabinet_signal_label(Player::Two, Signal::SwaTorpedo),
            "Star Wars Arcade (Gunner): Torpedo"
        );
        assert_eq!(
            cabinet_signal_label(Player::Two, Signal::StrikerShoot),
            Signal::StrikerShoot.label()
        );
    }
    #[test]
    fn unsupported_p2_rows_are_visible_but_cannot_open_an_editor() {
        let _lock = tests::IMGUI_TEST_LOCK.lock().unwrap();
        for (player, signal, enabled) in [
            (Player::One, Signal::Steering, true),
            (Player::Two, Signal::Steering, false),
            (Player::Two, Signal::SkyX, true),
            (Player::Two, Signal::Service, true),
            (Player::Two, Signal::GunYaw, true),
            (Player::Two, Signal::GunPitch, true),
            (Player::Two, Signal::BatSwing, true),
            (Player::Two, Signal::Accelerator, true),
            (Player::Two, Signal::Brake, true),
            (Player::Two, Signal::Action4, false),
        ] {
            let mut context = imgui::Context::create();
            context.set_ini_filename(None);
            context.io_mut().display_size = [1000.0, 700.0];
            context.fonts().build_rgba32_texture();
            let bindings = Bindings::default();
            let mut editor = BindingEditor::default();
            let mut point = [0.0, 0.0];
            for frame in 0..4 {
                if frame >= 2 {
                    context.io_mut().add_mouse_pos_event(point);
                    context
                        .io_mut()
                        .add_mouse_button_event(imgui::MouseButton::Left, frame == 2);
                }
                let ui = context.frame();
                ui.window("Player binding")
                    .position([0.0, 0.0], imgui::Condition::Always)
                    .size([900.0, 300.0], imgui::Condition::Always)
                    .build(|| {
                        let origin = ui.cursor_screen_pos();
                        point = [origin[0] + 310.0, origin[1] + 8.0];
                        cabinet_signal_row(ui, player, signal, &bindings, &mut editor);
                        assert!(
                            ui.cursor_screen_pos()[1] > origin[1],
                            "disabled rows remain in the list"
                        );
                    });
                context.render();
            }
            assert_eq!(editor.signal, enabled.then_some(signal));
            if enabled {
                assert_eq!(editor.player, player);
            }
        }
    }
}

/// One "name.... binding" row whose right-hand side is the button.
fn binding_row(
    ui: &imgui::Ui,
    label: &str,
    bound: &str,
    listening: bool,
    mut on_click: impl FnMut(),
) {
    ui.text(label);
    ui.same_line_with_pos(280.0);
    let caption = if listening {
        "  ...  ".to_string()
    } else {
        format!("{bound}##{label}")
    };
    let token =
        listening.then(|| ui.push_style_color(imgui::StyleColor::Button, [0.55, 0.42, 0.12, 1.0]));
    if ui.button_with_size(caption, [ui.content_region_avail()[0].max(120.0), 0.0]) {
        on_click();
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(bound);
    }
    if let Some(token) = token {
        token.pop();
    }
}

fn debugger_window(
    ui: &imgui::Ui,
    log: &[String],
    input: &mut String,
    follow: &mut bool,
    open: &mut bool,
    actions: &mut Vec<Action>,
) {
    ui.window("Debugger")
        .size([620.0, 400.0], imgui::Condition::FirstUseEver)
        .opened(open)
        .build(|| {
            ui.checkbox("Follow output", follow);
            ui.same_line();
            if ui.button("help") {
                actions.push(Action::Debug("help".into()));
            }
            ui.same_line();
            if ui.button("state") {
                actions.push(Action::Debug("state".into()));
            }
            ui.same_line();
            if ui.button("regs") {
                actions.push(Action::Debug("regs".into()));
            }
            ui.separator();

            ui.child_window("log")
                .size([0.0, -ui.frame_height_with_spacing()])
                .horizontal_scrollbar(true)
                .build(|| {
                    for line in log {
                        ui.text(line);
                    }
                    if *follow {
                        ui.set_scroll_here_y_with_ratio(1.0);
                    }
                });

            ui.set_next_item_width(-1.0);
            if ui
                .input_text("##command", input)
                .enter_returns_true(true)
                .build()
            {
                if !input.trim().is_empty() {
                    actions.push(Action::Debug(std::mem::take(input)));
                }
                ui.set_keyboard_focus_here();
            }
        });
}

fn stats_window(ui: &imgui::Ui, stats: Stats) {
    ui.window("Statistics")
        .size([220.0, 92.0], imgui::Condition::Always)
        .position([12.0, 40.0], imgui::Condition::FirstUseEver)
        .resizable(false)
        .collapsible(false)
        .build(|| {
            ui.text(format!("Display   {:6.1} fps", stats.video_fps));
            ui.text(format!("Emulated  {:6.1} fps", stats.emulated_fps));
            ui.text(format!("Layers    {:6.2} ms", stats.render_ms));
        });
}

/// Non-interactive feedback also visible with the menu hidden/fullscreen.
fn state_notice(ui: &imgui::Ui, message: Option<&(String, bool, std::time::Instant)>) {
    let Some((text, error, _)) = message else {
        return;
    };
    ui.window("Save state status")
        .position([12.0, 42.0], imgui::Condition::Always)
        .size(
            [ui.io().display_size[0].min(660.0) - 24.0, 0.0],
            imgui::Condition::Always,
        )
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .always_auto_resize(true)
        .draw_background(true)
        .focus_on_appearing(false)
        .no_inputs()
        .build(|| {
            let colour = if *error {
                [1.0, 0.45, 0.4, 1.0]
            } else {
                [0.65, 1.0, 0.65, 1.0]
            };
            let _colour = ui.push_style_color(imgui::StyleColor::Text, colour);
            ui.text_wrapped(text);
        });
}

/// A dark, slightly rounded theme, sized for a window that is usually showing a
/// 4:3 image behind it.
fn style(context: &mut imgui::Context) {
    let style = context.style_mut();
    style.window_rounding = 4.0;
    style.frame_rounding = 3.0;
    style.grab_rounding = 3.0;
    style.window_padding = [10.0, 10.0];
    style.item_spacing = [8.0, 6.0];
    style.window_border_size = 0.0;
    style.colors[imgui::StyleColor::WindowBg as usize] = [0.07, 0.08, 0.10, 0.94];
    style.colors[imgui::StyleColor::TitleBgActive as usize] = [0.16, 0.29, 0.48, 1.0];
    style.colors[imgui::StyleColor::Header as usize] = [0.20, 0.36, 0.58, 0.80];
    style.colors[imgui::StyleColor::HeaderHovered as usize] = [0.26, 0.46, 0.72, 0.90];
    style.colors[imgui::StyleColor::Button as usize] = [0.20, 0.30, 0.44, 1.0];
    style.colors[imgui::StyleColor::ButtonHovered as usize] = [0.28, 0.42, 0.62, 1.0];
}
