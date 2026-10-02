# TGPulse-Next Libretro: Model 1 preview

This experimental core targets Sega Model 1. Current source selects only
Model 1 machine components and required shared devices (a single system build).
Version 0.1.0.2 introduces this component split; previous previews used the
combined library. Consult the package's BUILD_INFO for its compiled scope. Model 2 and combined Libretro
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
**TGPulse-Next: Model 1 Development**; Core Label is
**Sega - Model 1 (TGPulse-Next Development)**.
The regular release filename is separate from the locally installed
`tgpulse_next_dev_m1_libretro` development copy.

## Platforms and rendering

The release workflow builds native Linux x86_64/ARM64, macOS Intel/Apple Silicon
and Windows x86_64 artifacts. Only packages that pass all CI gates are published.
Linux artifacts use Ubuntu 24.04; older glibc distributions are not guaranteed.
macOS builds target macOS 13 or newer. Windows uses the native MSVC toolchain
and the existing static C/C++ runtime configuration.

Software rendering is retained. Vulkan and OpenGL/GLES rendering use the
frontend's device/context. OpenGL requires desktop 4.3 or GLES 3.1 compute
support; macOS OpenGL 4.1 cannot run it. On macOS select RetroArch's Vulkan
driver with an available MoltenVK runtime. This package does not install or
replace RetroArch's drivers. Widescreen expansion and 1–4x supersampling are
available on hardware renderers; all Widescreen Mode choices apply to a wide
aspect ratio.

## Included features

- Native Model 1 timing; optional Supermodel-style 60 Hz mode advances a new
  machine frame each callback and speeds game/sound by approximately 4.3%.
- Per-title controls for Virtua Racing/Formula, Virtua Fighter, Wing War and
  Star Wars Arcade, including distinct Pilot/Gunner roles. Test is L3 and
  Service is R3. Driving ranges/response and supported frontend rumble are exposed.
- Live master/source audio gain controls and optional timing/FPS overlay.
- Frontend Save RAM and Save States; approved automatic initial NVRAM and
  configurable operator settings for nine independently acquired sets. Existing
  valid user Save RAM takes precedence over automatic initial setup.
- Experimental linked cabinets through Libretro Netpacket for supported
  Virtua Racing/Formula and Wing War sets. Configure native MASTER/SLAVE/LIVE
  roles using the service menu or NVRAM options. Save States are unavailable
  while the COMM board is fitted.
- Optional, hash-guarded in-memory repair of the known 315-5711 bad dump.

NetMerc cannot currently start and is excluded from NVRAM management. Physical
controller behavior, broad gameplay acceptance and distributed race/dogfight
validation remain user-run. CI format/API/package gates do not establish those
results. Vulkan is verified in macOS RetroArch; OpenGL/GLES image delivery and
context recreation are verified through the isolated Linux Mesa host.

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
Model 1 sets, defaulting to Auto. Auto uses the standalone mix (50/50/30/100%).
Mute and 0–100% in 10% steps apply immediately; DSB is shown only with a fitted
board when frontend visibility hints are supported.

The 0.1.0.0 set-qualified Gain keys are ignored. Existing values under the
older global keys may become active again. To use the new defaults, select
Auto for each source in Core Options and save the core options. Alternatively,
with RetroArch closed and a backup saved, remove the obsolete source Gain/Mute
entries from this core's options files. Do not remove NVRAM or unrelated keys.
The core does not edit personal option files or migrate a title's gains into
global settings automatically. Explicit frontend game-option overrides remain
frontend behavior.
