# TGPulse-Next Libretro

This is an independent repository for a Libretro core named
`tgpulse-next-libretro`, initially targeting Sega Model 1. It is **not** a
GitHub fork of either source repository. The initial tree is a snapshot of
[TGPulse-Next](https://github.com/Zer0one/TGPulse-Next) `main` at
`3c75f9b2b155e8bb50a225c69520948ef72ecd48`, itself based on
[TGPulse](https://github.com/deepblueworks/TGPulse). The snapshot was imported
as a fresh history; the remotes `upstream-next` and `upstream-original` retain
access to both source histories.

**Port status:** an experimental Libretro adapter lives in
`crates/tgpulse-libretro`. It accepts complete Model 1 sets with racing,
Virtua Fighter, Wing War, Star Wars Arcade Pilot/Gunner and experimental
NetMerc input profiles. It has native software and frontend-owned Vulkan/OpenGL compute video, stereo audio, RetroPad
controls, named cabinet controllers and Core Options for timing, aspect, audio,
VR-family rumble and ROM compatibility. Save RAM and Save States
are exposed through Libretro. An isolated RetroArch input replay has reached a
Virtua Racing race; physical controller and broader gameplay validation remain
pending. The
standalone source remains buildable. SM2-Emu Libretro is an architectural and
workflow reference, not vendored code.

The [Libretro implementation roadmap](docs/LIBRETRO_ROADMAP.md) lists the
remaining port features. User-run game and controller trials are outside it.

Model 1 previews are packaged through the
[Libretro build/release workflow](docs/LIBRETRO_CI.md).
Version [0.1.0.1](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.1)
is available as an experimental prerelease with five native platform packages.
See the [installation guide](docs/LIBRETRO_RELEASE.md) for platform requirements
and [GitHub Releases](https://github.com/Zer0one/tgpulse-next-libretro/releases)
for verified binary packages.

The [preliminary Model 1 frontend map](docs/LIBRETRO_MODEL1_FRONTEND_MAP.md)
records the first porting checkpoint: applicable SM2-Emu patterns, TGPulse
machine interfaces, control profiles and initial Core Options.

Build the development core without downloading dependencies:

```sh
cargo build --offline --release -p tgpulse-libretro
```

The library appears under `target/release/` as
`libtgpulse_next_m1_libretro.dylib` on macOS,
`libtgpulse_next_m1_libretro.so` on Linux or
`tgpulse_next_m1_libretro.dll` on Windows. Release CI verifies native Linux
x86_64/ARM64, macOS Intel/Apple Silicon and Windows x86_64 builds. The published
libraries omit Cargo's leading `lib` so their names match the `.info` basename.
After each verified macOS release build, install the development copy as
`~/Library/Application Support/RetroArch/cores/tgpulse_next_dev_m1_libretro.dylib`
and its matching metadata as
`~/Library/Application Support/RetroArch/info/tgpulse_next_dev_m1_libretro.info`.
Label the installed metadata **Sega - Model 1 (TGPulse-Next Development)** and compare the
installed core's SHA-256 with `target/release/libtgpulse_next_m1_libretro.dylib`.
Its Core Name is **TGPulse-Next: Model 1 Development**; Core Label uses the
descriptive Sega/Model 1 name above.
The current M1 artifact is an interim Model 1 core; the future real Tiny build
will replace it after component selection is implemented.
Keep updater-managed cores, ROMs, personal settings and saves untouched.

The `A/V Timing` Core Option defaults to native Model 1 timing (about
57.524 Hz). Its optional 60 Hz mode advances and renders a new machine frame
on every callback, making gameplay and sound about 4.3% faster. It resamples
audio to the same frontend output rate. Reload content after changing it.
The `Aspect Ratio` option defaults to Auto: Virtua Racing follows its saved
monitor setting, while other Model 1 sets use 4:3 unless explicitly overridden.
Software retains 496×384 output. Hardware expansion follows the effective wide aspect and the selected Widescreen Mode.
`Driving Steering Output Range`, `Driving Accelerator Output Range` and
`Driving Brake Output Range` apply to VR/VFormula: 50–150% in 10% steps,
default 100%, with immediate updates. Scaling respects Model 1 center/rest and
native limits. Other profiles hide these options when frontend display hints
are supported. Frontend remapping still owns physical input assignments.
`Driving Steering Response` applies to VR/VFormula and defaults to Linear.
Progressive (Fine Center) and FBNeo Logarithmic (Fine Center) follow the current
SM2/Supermodel curves, adapted to the native Model 1 steering span. Changes
apply immediately; the curve is applied before Steering Output Range.

`Automatic Initial NVRAM Setup` defaults to Enabled (restart required). It
initializes missing or invalid Save RAM using the approved set-specific image;
valid existing saves take precedence. VR's initial policy is EXPORT / NO LINK /
SPECIAL. `NVRAM Settings` defaults to Disabled; enabling it exposes the approved
operator fields for the current set. Their defaults match the native menu.
Changing an applied setting resets the game to refresh its native settings.
Nine sets have independent templates and selectors; NetMerc is excluded.
See the [review workbook](docs/model1_core_options_review.xlsx) and
[NVRAM procedure](docs/NVRAM_CAPTURE.md) for the reviewed list and evidence.

`Linked Cabinets (Restart Required)` defaults to Disabled and is separate per
eligible set. It connects the existing Model 1 COMM board through RetroArch
Netplay/Libretro Netpacket. Configure native MASTER/SLAVE and optional VR LIVE
roles through NVRAM Settings or the service menu; the transport does not assign
them. Each frontend needs its own saves. Save States are unavailable with COMM
fitted. See the [linked-cabinet guide](docs/LIBRETRO_MODEL1_LINKED_CABINETS.md)
for supported sets, startup and reproducible multi-instance evidence.

Source Gain options are global across Model 1 sets and default to Auto.
MultiPCM 1/2, FM (YM3438) and DSB (MPEG) use the standalone reference levels
50/50/30/100% respectively. Each selector offers Mute, Auto and 0–100% in
10% steps, with immediate updates. DSB is shown only when the board is fitted
and the frontend supports visibility hints. Old set-qualified Gain keys are
ignored; the core does not rewrite personal option files. Global source Gains
are restored in 0.1.0.1; the 0.1.0.0 preview used per-set Gain keys.

`Gamepad Rumble` defaults to On and uses the standalone VR/VFormula pad policy
through the frontend's P1 strong and weak motors when available.
`Timing / FPS Overlay` defaults to Off. Auto uses a 13 px font; 11–14 px
choices match SM2. The panel shows 61-frame averages and draws into software and GPU
images. Its reported video and callback costs include the overlay itself.

`Renderer (Restart Required)` offers Auto, Vulkan, OpenGL / GLES and Software.
OpenGL compute requires desktop 4.3 or GLES 3.1. On macOS, use Vulkan through
MoltenVK; Auto keeps Software when the frontend prefers macOS OpenGL 4.1.
Native Vulkan images are verified in RetroArch; desktop OpenGL/GLES images and
context recreation are verified with the isolated Mesa EGL check host. This
phase adds `Widescreen Mode (Restart Required)`: Stretch Entire Image (default),
Expand 3D View, or Expand 3D View + Stretch 2D. All modes apply to a wide aspect;
4:3 keeps native geometry. `Supersampling (Restart Required)` offers scales 1–4
(default 1), including in 4:3. These options are hidden on Software when frontend
visibility hints are supported. sRGB correction is not exposed by the core.

See [renderer adaptation](docs/LIBRETRO_GPU_ADAPTATION.md) and
[reusable GPU verification procedure](docs/LIBRETRO_GPU_VERIFICATION.md).
After the verified build, the authorized local installation is performed with:

```sh
python3 tools/install_dev_core.py
```

## Imported TGPulse-Next documentation

### <img src="assets/logo.png" alt="TGPulse Emulator" width="420">

This fork is maintained as [TGPulse-Next](https://github.com/Zer0one/TGPulse-Next),
based on [deepblueworks/TGPulse](https://github.com/deepblueworks/TGPulse).
The executable name remains `tgpulse`; the local development launcher remains `tgpulse.dev`.

Emulator for Sega's Model 1 and Model 2 arcade boards, written in Rust.
Currently with Linux, Windows and Android builds (Android branch).

Every processor is emulated at instruction level: the NEC V60 and MB86233 "TGP"
on the Model 1; the Intel i960, the TGP, the ADSP-21062 SHARC and the MB86235
"TGPx4" across the Model 2 revisions. No dump of the geometry engine exists, so
its display-list stream is decoded behaviourally, as MAME's is. Pixel coverage
is a hardware model in a compute shader rather than a modern rasterizer, so
dither patterns and other artefacts are reproduced.

## Running

```sh
cargo build --release
mkdir -p roms && cp ~/wherever/vf2.zip roms/
./target/release/tgpulse
```

**ROMs.** One zip per set in `roms/`, or `--roms <dir>`. Use the standard
zipped chip dumps; do not unzip them or rename their contents. Archive names
are ignored, since sets are identified by matching contents against the ROM
database. None are included here; you need the rights to the data.

**Launching.** With no arguments, the library window lists the sets found in
`roms/`, identifies them, and reports anything missing. Double-click to run;
Refresh after adding archives.

```sh
./target/release/tgpulse vf2        # by set name
./target/release/tgpulse --list     # list roms/, no window
./target/release/tgpulse --help     # all options
```

**Diagnostic panels.** Open Statistics and/or the GUI Debugger at startup:

```sh
./target/release/tgpulse wingwar --show-stats --show-debugger
```

Both flags also work without a romset, starting in the library. They apply only
to this launch and do not save panel preferences. These are the existing
View menu panels, not `--debug`, which runs the scriptable debugger without a
window. GUI panel flags cannot be combined with `--debug` or `--list`.
The existing visibility rules still apply: Statistics remains visible in
fullscreen, while the Debugger requires the interface to be visible. Add
`--fullscreen off` to inspect the Debugger if games normally start fullscreen.

**Controls.** Coin `5` (Select), start `Enter` (Start), digital movement on
arrows/WASD or the d-pad. Shared Button 1/Kick is South OR L1, Button 2/Punch
is East OR R1, and Button 3/Guard/Jump/Hold/Barrier is West. Eight cabinets
have independent game-prefixed action signals; Power Sled: Cancel Error uses North. Gun Primary/Secondary
Fire and Sky Target: Machine Gun/Missile have separate assignable signals.
Dedicated Gear Down (E/L1) and Gear Up (Q/R1) control
sequential shifting independently of Action 1/2. Analog driving uses left
stick X and R2/L2. Direct gears 1–4 use right-stick diagonals and latch when
released; West selects neutral. Gun aim uses the mouse or left stick.
Service is R3 and Test L3. The full, unfiltered signal list is editable in
Settings → Input and stored in `config/input.conf`.
See [input bindings and game routing](docs/INPUTS.md) for keyboard assignments,
combination syntax, migration and deliberate differences from SM2-Emu.

**Widescreen.** Settings → Widescreen offers Off, On and Auto. Auto follows
the saved monitor/cabinet setting for VR, Indy 500 and Sega Touring Car;
unknown games use 4:3. CLI: `--widescreen auto`. See
[native aspect detection and supported sets](docs/WIDESCREEN.md).

**Colours.** Settings → **sRGB correction** enables correct display-RGB sampling
for the game framebuffer (Model 1 and Model 2), without changing GUI colours.
It takes effect immediately and persists as `srgb = on` in
`config/settings.conf`; default `off` retains the previous presentation.
Non-sRGB output surfaces already preserve RGB bytes and need no conversion.
Separately, Model 1's 2D palette intensity bit is always emulated: bit 15 clear
halves RGB after expansion, matching MAME. This hardware correction is not
controlled by the sRGB option and does not alter Model 2 palette behaviour.

**Audio sources.** While a game is loaded, Settings → Audio shows a gain slider,
Mute checkbox and name for each output: MultiPCM 1/2 and FM (YM3438), or SCSP;
SWA/SWAJ also show DSB (MPEG). Sliders set absolute gains (50% = 0.5), with a
full-height dark-blue reference marker: MultiPCM 50%, FM 30%, DSB/SCSP 100%.
The channel slider's grab is 50% opaque and drawn above the reference marker.
Channel range 0–100%; double-click a channel slider to restore its default.
Master volume remains separate (0–800%), with the same marker style and a
100% reference restored by double-click. Gains and mutes persist in `config/settings.conf`.
Mute keeps the selected gain; chip/CPU/timer emulation continues even at zero.
YM3438 synthesis always runs: FM sounds in Virtua Racing have also been
confirmed in-game by the user. Muting FM silences only its output.
See [audio integration and limits](docs/MODEL1_AUDIO.md).

**Model 1 networking (experimental).** `cabinet = twin` fits the network board
and enables TCP for supported games. Settings exposes `AddressIn`, `PortIn`,
`AddressOut` and `PortOut`, following Supermodel Standalone's naming with MAME's
local bind address. Apply saves the configuration; reload/reset the game to use
it. TCP uses MAME's M1COMM ring frames, with no automatic loopback. Transport,
board and VR/Wing War boot/link tests pass; synchronized gameplay is not yet
validated. Wing War R360 boots, but its link test currently fails after the game
reinitializes the supplied EEPROM configuration. See
[setup, verification boundaries and remaining work](docs/MODEL1_NETWORK.md).

**Player 2 (Model 1 and Model 2).** Settings → Input has Cabinet P1/P2 tabs with
independent bindings and controller selection. P2 supports local two-player
joystick games, Model 2 guns, Dynamite Baseball's bat, Power Sled's second seat
and SWA/SWAJ's Gunner (stick and two fire buttons, no Start/view/throttle).
Signals without a P2 counterpart remain grey. P2 uses P1 gamepad conventions, with no default gameplay keys;
Coin/Start and shared Test/Service retain keyboard defaults. See
[player assignment and migration](docs/INPUTS.md#player-2--model-1-and-model-2).

| Key | |
| --- | --- |
| `F1` | show or hide the interface over a running game |
| `F3` | reset the machine |
| `F5` / `F7` | save and load the current state slot |
| `F4` / `F6` | previous and next slot |
| `F9` | pause |
| `F11` | fullscreen |
| `Tab` | fast forward while held |

**Model 1 save states.** Use Machine → Save/Load state or `F5`/`F7`, with
slots 0–9 selected by `F4`/`F6`. States use `states/<set>.<slot>.state`
in the emulator's runtime directory, like Model 2. A versioned, ROM-identified
snapshot restores the standalone machine, including in-memory NVRAM/EEPROM,
without immediately writing persistent NVRAM. Current audio/video preferences
and pause mode remain unchanged. Success/errors appear briefly even in fullscreen.
Model 1 save/load requires `cabinet = single`; a fitted COMM board is refused
even before a peer connects. This does not snapshot a network session.
The desktop integration has automated coverage and the user confirms basic
save/load working in VR and SWA; other games and broader scenarios remain to validate.
F4/F6 display the selected slot briefly, including in fullscreen.
See the [checkpoint evidence](docs/MODEL1_ROADMAP.md#desktop-saveload-checkpoint--2026-09-27).

**Files written.** Battery-backed RAM (high scores, rankings, test menu
settings) to `nvram/<set>.nv` on close, reloaded on the next run. Save states to
`states/`, options and bindings to `config/`. Nothing is written elsewhere.

## Rendering

Native output is 496x384 with no antialiasing. The rasterizer is reproduced
exactly, including dither patterns, stipple transparency and painter-sort
artefacts.

- `--ssaa 1..4` (default 2) supersamples the 3D layer and presents it at twice
  board resolution; tile layers (HUD, text, sky) scale by integer factors.
  `--ssaa 1` is the board's output pixel for pixel.
- `--widescreen on` renders 16:9 by widening the frustum and viewport around
  the centre rather than stretching. Not hardware behaviour: games composed for
  4:3 may show edge-pinned HUD elements and scenery ending where the original
  camera stopped. 2D layers stretch to fill unless `--widescreen-stretch-2d
  off`.
- `--smooth-shadows off` reproduces the board's lack of an alpha channel:
  checkerboard-stipple shadows and thresholded per-texel coverage on
  translucent textures. The default blends them.

## Status

The ROM database covers 100 sets, but an entry only means the memory image can
be built, not that the game runs. Twelve are tested: they boot, play, and
render frames checked against MAME. The rest may do anything from black-screen
to subtly wrong.

| Board | Tested |
| --- | --- |
| Model 1 | Virtua Racing, Virtua Fighter, Star Wars Arcade |
| Model 2 | Daytona USA (and Special Edition), Virtua Cop |
| Model 2A | Sega Rally Championship, Virtua Fighter 2 |
| Model 2B | Virtua Striker, Sonic Championship |
| Model 2C | The House of the Dead, Wave Runner |

**No multiplayer.** M2COMM is modelled only as far as one cabinet needs: the
ring closes on itself so the network check passes. Twin Daytona and Virtua
Striker's versus play run as a single machine.

## Roadmap

The [Libretro roadmap](docs/LIBRETRO_ROADMAP.md) is the single active plan for
this port. The imported [standalone Model 1 roadmap](docs/MODEL1_ROADMAP.md)
is source history, not a second port plan.

## Layout

```
crates/i960        Intel i960KB              -- Model 2 main CPU
crates/v60         NEC V60                   -- Model 1 main CPU
crates/mb86233     Fujitsu MB86233 "TGP"     -- Model 1 and 2/2A geometry
crates/mb86235     Fujitsu MB86235 "TGPx4"   -- Model 2C geometry
crates/sharc       Analog Devices ADSP-21062 -- Model 2B geometry
crates/sega-crypt  Sega 315-5881 decryption
crates/tgpulse-core  the machine: memory maps, geometry, sound, save states
crates/tgpulse       the front end: window, renderer, interface, input, audio
tools/               the ROM database generator and a MAME comparison harness
```

`tgpulse-core` has no window, GPU or controller: a front end drives it a frame
at a time and reads back the framebuffer, so the debugger and the comparison
harness can run it headlessly.

## Accuracy

MAME is the reference. Memory maps follow its ordering so the two can be read
side by side, and divergences are found by diffing instruction traces, work RAM
and rendered frames (`tools/mamediff.sh`).

The debugger is script-driven and the machine deterministic, so investigations
replay exactly:

```sh
./target/release/tgpulse vf2 --debug -c "run 1400; geo; vertices"
```

Output is one `kind key=value` line at a time, and every command reports what
it did.

## Building

A Cargo workspace with no vendored dependencies. A stable toolchain from
[rustup](https://rustup.rs) is enough; the binary lands in
`target/release/tgpulse`.

**Linux.**

```sh
sudo apt install build-essential libudev-dev libasound2-dev   # Debian, Ubuntu
sudo pacman -S base-devel systemd-libs alsa-lib               # Arch
sudo dnf install gcc systemd-devel alsa-lib-devel             # Fedora

cargo build --release
```

`libudev` is for gamepad detection, ALSA for audio. Rendering uses Vulkan where
the driver offers it and GL otherwise.

**Windows.** Native builds need the MSVC toolchain and Visual Studio's C++
build tools, then `cargo build --release`. Cross-compiling from Linux needs
only the target and mingw, the linker already being named in
`.cargo/config.toml`:

```sh
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64          # or: pacman -S mingw-w64-gcc
cargo build --release --target x86_64-pc-windows-gnu
```

The result is `target/x86_64-pc-windows-gnu/release/tgpulse.exe`, which wants
the same `roms/` directory beside it.

**Tests.** `cargo test --release`. The processor tests assemble their own
programs and need no ROM data.

Android is packaged with `cargo-apk` and covered in
[docs/BUILDING.md](docs/BUILDING.md); it has not been run on a device.

## Thanks

**The MAME project** — public documentation of these boards, down to chip
identification, bus wiring and undocumented registers, is what makes an
independent implementation practical, and is the reference used here.

**ElSemi** — Nebula Model 2 established much of the current understanding of
the geometry pipeline and rasterizer, and widescreen mode follows it.

Neither is affiliated with this project. Any inaccuracy is this program's own.

## License

MIT, in [LICENSE](LICENSE). ROM images are copyrighted by their publishers and
none are included here.

The YM3438 register/timer and synthesis adaptation retains Aaron Giles' YMFM
[BSD-3-Clause notice](LICENSES/YMFM-BSD-3-Clause.txt). Its current scope and
reference checks are documented in [Model 1 audio](docs/MODEL1_AUDIO.md).

## Logging

Off by default, addressable per subsystem:

```sh
RUST_LOG=info ./target/release/tgpulse vf2
RUST_LOG=warn,geo=trace ./target/release/tgpulse vf2
```

Targets include `geo`, `fifo`, `io`, `sound`, `copro`, `nvram`, `backup`,
`comm`, `library` and `video`.
