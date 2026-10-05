# TGPulse-Next Libretro: Model 1 preview

This experimental core targets Sega Model 1. Current source selects only
Model 1 machine components and required shared devices (a single system build).
Version 0.1.0.2 introduced this component split; 0.1.0.7 retains it alongside
the reviewed Model 1 integration and the retained performance improvements
documented in `LIBRETRO_MODEL1_PERFORMANCE.md`. Consult the package's
BUILD_INFO for its compiled scope. Model 2 and combined Libretro
adapters remain future work. The repository is independent of its TGPulse-Next
and TGPulse source repositories.

## Installation

Download the archive for your operating system and CPU from
[GitHub Releases](https://github.com/Zer0one/tgpulse-next-libretro/releases).
Each archive contains one `tgpulse_next_m1_libretro` library, its matching
`tgpulse_next_m1_libretro.info`, this guide, source/build information, project
and dependency licenses, and SHA256SUMS.

1. Place the `.so`, `.dylib` or `.dll` file in RetroArch's cores directory.
2. Place `tgpulse_next_m1_libretro.info` in RetroArch's core information directory.
3. Load the core and a complete supported Model 1 ZIP. Keep the ZIP intact.

No separate `system` database is required: the catalogue and approved NVRAM
tables are embedded. No ROMs or user saves are supplied. Core Name is
**TGPulse-Next - Model 1**; Core Label is
**Sega - Model 1 (TGPulse-Next)**.
The local macOS installation uses this same public filename and identity.

### Upgrade From The Previous Core Name

Version 0.1.0.7 changes the runtime and metadata name from
`TGPulse-Next: Model 1` to `TGPulse-Next - Model 1`; the binary filename and
Core Label are unchanged. Close RetroArch before installing. Copy the old
core-named configuration, remap, save and state directories to the new name
where present, preserving originals and any existing destination files.
`tools/install_dev_core.py` performs this non-destructive copy for the default
local macOS paths. Custom directory layouts require the equivalent manual copy.
Refresh the Core Info cache/restart RetroArch. Netplay participants should
all upgrade because the advertised core name has changed.

## Platforms and rendering

The release workflow builds native Linux x86_64/ARM64, macOS Intel/Apple Silicon
and Windows x86_64 artifacts. Only packages that pass all CI gates are published.
Linux artifacts use Ubuntu 24.04; older glibc distributions are not guaranteed.
macOS builds target macOS 13 or newer. Windows uses GNU/MinGW with static
runtimes. All release jobs invoke the shared `Makefile.libretro` recipe.

Software rendering is retained. Vulkan and OpenGL/GLES rendering use the
frontend's device/context. OpenGL requires desktop 4.3 or GLES 3.1 compute
support; macOS OpenGL 4.1 cannot run it. On macOS select RetroArch's Vulkan
driver with an available MoltenVK runtime. This package does not install or
replace RetroArch's drivers. The Widescreen Hack applies only to games without
native widescreen support and only with a wide Aspect Ratio. It defaults to
Expand 3D View Only; the other modes also stretch 2D or stretch the full image.
Supersampling offers 1–4x on hardware renderers, including in 4:3.

## Included features

- Native Model 1 timing; optional Supermodel-style 60 Hz mode advances a new
  machine frame each callback and speeds game/sound by approximately 4.3%.
- Per-title controls for Virtua Racing/Formula, Virtua Fighter, Wing War,
  Star Wars Arcade and Sega NetMerc, including distinct Pilot/Gunner roles and
  full or reduced Test/Service profiles. Test is L3 and Service is R3 in the
  full profiles. Driving ranges/response and frontend rumble are exposed.
- Sega NetMerc MVD supports right-stick, calibrated P1 sensors and fixed-camera
  control, with virtual Holder, Calibrate and Recenter actions. Sensor drift
  compensation defaults to 50%; optional CSV diagnostics write to the frontend
  Save Directory. Sensor behavior and physical rumble require controller tests.
- Live master/source audio gain controls, NetMerc audio donor selection and
  conditional fallback notices. Donor game ZIPs belong beside `netmerc.zip`.
  Optional timing/FPS and HD44780 diagnostic LCD overlays are available.
- Frontend Save RAM and Save States; approved automatic initial NVRAM and
  configurable operator settings for ten independently acquired sets, including
  NetMerc. Existing valid user Save RAM takes precedence over automatic initial
  setup. `netmerc_nvram.bin` is an optional initial seed; its absence is silent.
  Current machine Save State format is 5, without migration from old formats.
- Experimental linked cabinets through Libretro Netpacket for supported
  Virtua Racing/Formula and Wing War sets. Configure native MASTER/SLAVE/LIVE
  roles using the service menu or NVRAM options. Save States are unavailable
  while the COMM board is fitted.
- Optional, hash-guarded in-memory repair of the known 315-5711 bad dump.

NetMerc remains experimental. Its required Model 1 I/O BIOS can be found in the
frontend System directory, beside the game or inside its ZIP, in that order.
The optional `hd44780.zip` follows the same lookup order when the diagnostic
display is enabled. Physical-controller behavior and broad gameplay acceptance
remain user-run. CI format/API/package gates do not establish those results.
Vulkan is verified in macOS RetroArch; OpenGL/GLES image delivery and context
recreation are verified through the isolated Linux Mesa host.

## Integrity and source

The release's SHA256SUMS covers each platform ZIP. Every ZIP also includes a
SHA256SUMS for its files, SOURCE_COMMIT.txt and BUILD_INFO.json. All platform
packages must identify the same source commit as the release tag.

Source, the reviewed NVRAM workbook, YAML catalogue, documentary screenshots,
recipes and repeatable verification tools are in
[the repository](https://github.com/Zer0one/tgpulse-next-libretro).
Future implementation is tracked only in
[the roadmap](https://github.com/Zer0one/tgpulse-next-libretro/blob/main/docs/LIBRETRO_ROADMAP.md).


## Source Gain update in 0.1.0.1

MultiPCM 1/2, FM (YM3438) and DSB (MPEG) Gain selectors are global across
Model 1 sets, defaulting to Auto. In 0.1.0.5, Auto uses 50/50/90/100%; FM Auto is 90%, while the standalone FM reference remains 30%.
Mute and 0–100% in 10% steps apply immediately. The 0.1.0.1 release showed
all Gain selectors for every title. Current development builds show only the
sources fitted to the loaded game: MultiPCM 1/2 and YM3438 for all Model 1
sets, plus DSB for Star Wars Arcade and its clone. No source Gain is shown
without loaded content; Master Volume remains visible.

The 0.1.0.0 set-qualified Gain keys are ignored. Existing values under the
older global keys may become active again. To use the new defaults, select
Auto for each source in Core Options and save the core options. Alternatively,
with RetroArch closed and a backup saved, remove the obsolete source Gain/Mute
entries from this core's options files. Do not remove NVRAM or unrelated keys.
The core does not edit personal option files or migrate a title's gains into
global settings automatically. Explicit frontend game-option overrides remain
frontend behavior.
