# A1 — Model 1 Reference Conformity Audit

Date: 2026-10-04. Scope: current local Model 1 port, compared with the current
SM2-Emu Libretro and Supermodel Libretro checkouts and imported TGPulse-Next
standalone code. This is a source and automated-verification audit. Controller
feel, physical rumble/MVD coupling and full game presentation still require
hardware observation; the separate R1 roadmap item tracks the requested
DualSense comparison. This document is evidence for the single active
implementation roadmap, not another status plan.

Source entry points: [SM2 Core Options](https://github.com/Zer0one/sm2-emu-libretro/blob/75ced234792520c325aa63aa7be3dfaf87b07d46/src/libretro/core_options.h),
[SM2 Input](https://github.com/Zer0one/sm2-emu-libretro/blob/75ced234792520c325aa63aa7be3dfaf87b07d46/src/libretro/input.cpp),
[SM2 core lifecycle](https://github.com/Zer0one/sm2-emu-libretro/blob/75ced234792520c325aa63aa7be3dfaf87b07d46/src/libretro/core.cpp),
[Supermodel adapter](https://github.com/Zer0one/Libretro-Supermodel/blob/4566c9470ab89ec8e2738bfb43c18281df580971/Src/OSD/libretro/libretro.cpp),
[Model 1 adapter](../crates/tgpulse-libretro/src/lib.rs),
[native control mapping](../crates/tgpulse-libretro/src/model1_controls.rs),
[NVRAM policy](../crates/tgpulse-libretro/src/nvram.rs),
[Save RAM/State envelope](../crates/tgpulse-libretro/src/persistence.rs),
[MVD sensors](../crates/tgpulse-libretro/src/sensors.rs) and
[standalone rumble](../crates/tgpulse/src/input/model1_rumble.rs). The two
external commits identify the inspected reference revisions.

| Area | Reference and current source | Finding |
| --- | --- | --- |
| Core Options structure | SM2 `src/libretro/core_options.h` category and display definitions; Supermodel `Src/OSD/libretro/libretro.cpp` option registration; local `crates/tgpulse-libretro/src/lib.rs` `menu_order`, `publish_global_option_visibility`, `publish_nvram_option_visibility` | System, Video, Audio, Input ordering and Title Case labels follow the references. General options remain visible; per-set NVRAM selectors and linked-cabinet fields retain their expressly approved applicability filtering. Legacy registration uses the same ordering. |
| Profiles and native controls | SM2 `src/libretro/input.cpp` full/reduced profiles and Supermodel `Src/OSD/libretro/libretro.cpp` Test/Service slots; local `crates/tgpulse-libretro/src/lib.rs` profile names/descriptors and `model1_controls.rs`; standalone `crates/tgpulse/src/input/` | Full `+ Test/Service slots` variants exist. SWA Pilot/Gunner are distinct; duplicated VR1 direction is one native input with two bindings. NetMerc virtual Calibrate/Recenter are frontend commands, while the other labels map to native actions. P2 keeps Test/Service where no second gameplay station exists. Approved Model 1 pad alternatives and VF exclusions remain deliberate adaptations. |
| Rumble and motion | SM2 `src/libretro/core_options.h` Gamepad Rumble and `src/libretro/core.cpp` lifecycle; Supermodel `Src/OSD/libretro/libretro.cpp` rumble shutdown; local `crates/tgpulse-libretro/src/lib.rs` `PadRumble`, `crates/tgpulse/src/input/model1_rumble.rs`, `sensors.rs`, `mvd.rs` | Existing Gamepad Rumble and RetroArch gain remain the user-approved controls. NetMerc uses final native motor latch; host sensor acquisition and calibration stay adapter-owned. The references have no NetMerc gyro/motor interaction to copy; R1 needs controlled physical evidence. |
| Notifications and diagnostics | SM2 frontend message and save lifecycle in `src/libretro/core.cpp`; local `crates/tgpulse-libretro/src/lib.rs` queued notices, `sensors.rs`, `sensor_trace.rs`, `diagnostic_overlay.rs` | Fallback and procedural-audio notices and optional BIOS notice are conditional. Sensor CSV creates a new segment after reset or state load if still enabled; the option description had incorrectly said recording must be toggled, and is corrected. The 30-minute cap still requires a toggle to start another capture. |
| Audio, rendering and timing | SM2 `src/libretro/core_options.h` gains, rumble and display options; Supermodel `Src/OSD/libretro/libretro.cpp` renderer lifecycle; local `crates/tgpulse-libretro/src/lib.rs`, `gpu.rs`, `timing_overlay.rs`, `diagnostic_overlay.rs` | Global Auto/manual chip gains, user-approved NetMerc audio donor policy and retained core-specific Software/Vulkan/OpenGL paths are intentional adaptations. Widescreen Hack applies only to non-native-capable games under a wide aspect. Timing has priority at an overlapping diagnostic LCD corner. The current README used the old widescreen name/default and contained an interrupted renderer sentence; both are corrected. |
| NVRAM and initial save policy | SM2 `src/libretro/initial_nvram.cpp`, `nvram_settings.cpp`, `core.cpp`; Supermodel `Src/OSD/libretro/libretro.cpp` initial NVRAM and `.srm` precedence; local `crates/tgpulse-libretro/src/lib.rs`, `nvram.rs`, `nvram_data.rs`, `persistence.rs` | Catalogue/sample-derived selections and approved Automatic overrides are separate from native defaults. A valid frontend save takes precedence. NetMerc's optional seed is used only when initial data is needed, with approved calibration and bookkeeping corrections; missing seed is silent. This follows the reference policy while adapting to Model 1 backup RAM. |
| State and lifecycle | SM2 `src/libretro/core.cpp` frontend envelope and state-load reset; local `crates/tgpulse-libretro/src/persistence.rs`, `lib.rs`, `crates/tgpulse-core/src/model1/state.rs` | Save RAM and full Save State have distinct envelopes. Current machine state format is 5 with no legacy migration by user decision. Host sensor/rumble state is stopped or reset on restore; emulated state is restored independently. COMM state restrictions and current frontend timing/input preferences remain explicit. |
| Build and distribution | SM2 `PORTING_PLAN.md` phase 4 and Supermodel `Docs/ROADMAP.md` package gates; local `docs/LIBRETRO_CI.md`, `tools/check_libretro_artifact.py`, `tools/install_dev_core.py`, `tgpulse_next_m1_libretro.info` | Model 1 compilation scope is narrower than imported standalone; five native CI targets and matching core/info checks are established. Current local edits are not a published release. A verified macOS release build must be installed as the authorized Development core/info and hash-checked. U9 owns release preparation and separate publication approval. |

## Corrections from this audit

- Corrected the sensor diagnostics Core Option lifecycle description.
- Corrected the active README renderer/widescreen wording and an interrupted sentence.
- Reconciled the roadmap's old U3 review statement and state-format mapping.

The comparison found no further unexplained source-level difference requiring
a behavioral change in A1. The report does not claim physical-controller
equivalence: that evidence remains with the user-operated R1 comparison.

## Verification

- 79 Libretro adapter unit tests passed.
- The offline macOS release build passed the Model 1 scope, 25 ABI export,
  dependency and three-cycle empty-lifecycle gates.
- The authorized Development core and matching `.info` were installed in
  local RetroArch. Installed/source SHA-256 matched for both files:
  core `a399ed08e84203c96e4344debd73dabbd8438ef52337a079a8cbe6d3c6eb379b`,
  info `91ff93c2f5f1bd468bf88d66b73e3fe87066bbb596bbe37b41bc135f0021fdee`.
- No new physical-controller or multi-title gameplay trial was run for A1.
