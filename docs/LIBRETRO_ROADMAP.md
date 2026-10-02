# TGPulse-Next Libretro implementation roadmap

Status: 2026-10-02. This is the single active implementation plan for the
independent port. Model 1 is the current implementation scope; the final
repository goal is three distinct Libretro cores, defined below. User-run game,
controller and presentation trials are not scheduled here. Design and evidence
live in the linked technical documents.

## Final repository outputs

User-confirmed target (2026-10-02): one repository produces three distinct cores.

| Core | Compiled scope | Content scope |
| --- | --- | --- |
| Model 1 (Tiny) | Model 1 machine/boards and required shared components only. | Supported Model 1 sets. |
| Model 2 (Tiny) | Model 2 machine/board variants and required shared components only. | Supported Model 2 sets and variants; availability follows actual implementation. |
| TGPulse complete | Both machine families and their required components. | Supported Model 1 and Model 2 sets. |

Use shared source and adapter infrastructure, with build-time component
selection rather than three copied codebases or catalogue filtering alone.
Each output needs its own core artifact and matching `.info`; exact new names
remain to be agreed before packaging. The current interim core uses the M1 suffix: `tgpulse_next_m1_libretro`,
installed locally as `tgpulse_next_dev_m1_libretro` / Sega - Model 1 (TGPulse-Next Development).
It will be replaced by the real Model 1 Tiny implementation. Preserve the standalone full-system build and minimize changes
to imported upstream code. No Model 3 output is planned.

Complete the current Model 1 phase first. The Model 1 Tiny build establishes
the feature boundaries for the later Model 2-only and combined builds. Before
planning the Model 2 adapter/features, inspect current standalone support and
SM2/Supermodel reference workflows; do not equate a ROM database entry with
working emulation. Detailed Model 2 estimates follow that review.

## Reference review and adaptation

This plan was compared directly with SM2-Emu Libretro's
[`PORTING_PLAN.md`](/Users/andrea/dev/sm2-emu-libretro/PORTING_PLAN.md),
[`LIBRETRO_DESIGN.md`](/Users/andrea/dev/sm2-emu-libretro/LIBRETRO_DESIGN.md)
and Supermodel's
[`Docs/ROADMAP.md`](/Users/andrea/dev/libretro-supermodel-modern/Docs/ROADMAP.md)
on 2026-10-01. The SM2 milestones supply the sequence and distinction between
implementation and residual trials. Supermodel supplies the full-catalogue,
clone-specific NVRAM and native-default versus policy-override criteria. The
Model 1 machine, menus and repository determine which features apply.

| Reference phase | Model 1 adaptation |
| --- | --- |
| SM2 0–2: upstream baseline, frontend-neutral machine and first software core | Reuse the imported TGPulse-Next machine. The narrow Rust adapter and native software output are implemented; preserve the standalone build. |
| SM2 3.1–3.5: catalogue, recognized profiles and game controls | Model 1 set profiles and distinct SWA Pilot/Gunner ports are implemented. Keep exact Model 1 actions and the established Test=L3, Service=R3 preference. |
| SM2 3.6 and Supermodel NVRAM campaign | Catalogue all runnable parent and clone settings before choosing menu fields. Nine starting sets have complete isolated values; NetMerc is excluded while it cannot start. Apply set-specific templates only to new Save RAM and keep native defaults distinct from approved overrides. |
| SM2 3.7–3.8: menu review and applicable options | The user-approved workbook now defines the implemented NVRAM options. Separately assess timing, display, audio and input options against current Model 1 behavior. Keep native cadence as the default. Expose the already documented, SHA-1-guarded 315-5711 bad-dump repair as an opt-out load-time option; SM2's ROM CRC toggle is validation, not a repair mechanism. |
| SM2 4: compatibility and distribution | Reference matrix, core/info pairing, license texts, source revisions, ZIPs and checksums adapted for the interim Model 1 preview. Version 0.1.0.0 is published after all five native CI gates and downloaded-package verification. |
| SM2 5: frontend-owned GPU rendering | Keep the native software path; design any later GPU adapter around frontend-owned context and the existing TGPulse renderer. No renderer is assumed ready to port. |
| SM2 6: linked cabinets and Save States | Save RAM and Save States are implemented. Linked cabinet transport is now implemented in phase 6.1 with native Model 1 roles and operator settings. |

## Milestones

Legend: 🟢 complete · 🟡 partial · ⚪ not started · 🔵 deferred · 🟠 blocked.
The blocked rows are last, regardless of their phase number.

| Status | Phase | Implementation state and next work | Indicative effort and Codex usage |
| --- | --- | --- | --- |
| 🟢 | 0–2. Baseline and software core | Independent source snapshot, frontend-neutral machine reuse, Model 1 loader/lifecycle, software video, stereo audio and basic Core Options are present. | Complete |
| 🟢 | 3.1–3.2. Model 1 controls | [Binding audit](LIBRETRO_MODEL1_BINDING_AUDIT.md) findings B1–B5 addressed: corrected VF native bits, removed VF L/R by user instruction, restored flight shoulder/right-stick throttle alternatives and standalone flight response. Wing War name corrected to VR4. Follow-up descriptor/native-input audit: SWA duplicate view bindings both read VR1; unapproved VF analog directions removed. 34 adapter checks pass. | Complete |
| 🟢 | 3.3. Operator-setting acquisition | Nine starting parent/clone sets have screenshot catalogues, diagnostic YAML, workbook rows and 953 isolated persistent values across 110 fields. NetMerc is excluded because the game does not start. | Complete |
| 🟢 | 3.4. NVRAM menu review | User approved 39 fields across nine sets; VR Automatic Cabinet corrected to SPECIAL. Workbook and per-set YAML record the approved selection. | Complete |
| 🟢 | 3.5. Automatic Initial NVRAM Setup | Enabled by default. Complete independent set templates initialize absent/invalid Save RAM; valid existing saves take precedence. Native calibration and unrelated settings remain in each template. VR starts EXPORT / NO LINK / SPECIAL. | Complete; High reasoning |
| 🟢 | 3.6. NVRAM Settings | Disabled by default. All 39 reviewed fields have per-set keys, observed native values/defaults, exact-set visibility and native CRC/mirror updates. Changed selections reset the machine to refresh native caches. 41 adapter checks, all nine ABI sets and isolated RetroArch save readback pass; Development installed. | Complete; High reasoning |
| 🟢 | 6.2. Frontend persistence | Save RAM and Save States use the existing machine APIs and frontend-owned files. | Complete |
| 🟢 | 3.7. Software Core Options | Smooth Shadows, presentation-only Aspect Ratio, Master Volume, per-source Model 1 Gain selectors (Mute/Auto/0–100%), VR-family Gamepad Rumble, Timing / FPS Overlay, Known Bad Dump ROM Repairs and selectable native/60 Hz timing exist. In 60 Hz mode each callback advances and renders a new machine frame, so game and sound run about 4.3% faster; audio is resampled to the native output rate. The listed options are implemented; driving ranges are implemented in phase 3.8; steering response is implemented in phase 3.9. | Complete |
| 🟢 | 5.1. Vulkan renderer | Frontend-owned Vulkan v5 device/queue, shared standalone raster/resolve shaders, synchronized image handoff and context lifecycle. Native images are verified in macOS RetroArch through existing MoltenVK. | Complete; High reasoning |
| 🟢 | 5.3. OpenGL/GLES renderer | Frontend-owned OpenGL 4.3/GLES 3.1 compute adapter. Actual 2D/3D images, normal context recreation and unannounced loss verified through the SM2-style EGL host on Linux Mesa. Audio and NVRAM match Software. macOS GL 4.1 cannot run this compute path. | Complete; High reasoning |
| 🟢 | 5.2a. Widescreen and supersampling | Three widescreen modes and native supersampling scales 1–4, applied at content reload. Widescreen modes affect wide aspect only; supersampling also applies in 4:3. Vulkan and OpenGL/GLES use the standalone shaders. | Complete; High reasoning |
| 🟢 | 3.8. Driving analog ranges | Steering, Accelerator and Brake Output Range for VR/VFormula and their recognized sets: 50–150% in 10% steps, default 100%, live updates and profile visibility. Scaling preserves Model 1 center/rest and native limits. 39 adapter checks, macOS release build, isolated RetroArch registration/frame delivery and Development installation pass. | Complete; Medium reasoning |
| 🟢 | 3.9. Driving Steering Response | Linear / Progressive (Fine Center) / FBNeo Logarithmic (Fine Center), default Linear. Current SM2/Supermodel curves adapted to Model 1 20–80–E0, before output range; VR/VFormula only, live updates. 44 adapter checks, macOS release build and bounded RetroArch registration/frame delivery pass; Development installed. | Complete; Medium reasoning |
| 🟢 | 6.1. Linked cabinets | SM2/Supermodel Netpacket workflow adapted to existing M1COMM. Per-set selectors default Disabled; native NVRAM roles retained. All six eligible sets reached game-created links in real RetroArch pairs; VR also passed LIVE relay. COMM Save States remain unavailable. 47 adapter checks, macOS build and Development install pass. | Complete; High reasoning |
| ⚪ | 4.1. Model 1-only build (Tiny) | Measure release binary contents first (thin LTO is already enabled), then introduce Cargo feature boundaries for Model 1 machine/boards and required shared components, including ROM catalogue dependencies. Establish reusable boundaries for the confirmed Model 2-only and combined cores; keep standalone defaults and full-system builds available. Record size/build-time differences and the minimum upstream adaptation. Current SM2/Supermodel are system-specific references; no existing multi-system feature split was found to copy directly. | High, 1–3 h, approximately 1–2 percentage points; refine after binary inspection |
| 🔵 | 7. Model 2 and combined core adaptation | After the current Model 1 scope, inspect actual standalone board support and reference features, then extend the shared adapter for Model 2 and build both Model 2-only and combined cores. Keep system-specific timing, inputs, rendering, NVRAM and state explicit. | High; effort and usage estimate after reference/board review |
| 🟢 | 4. Build and distribution — Model 1 preview foundation | Version 0.1.0.0 published as a prerelease. Linux x86_64/ARM64, macOS Intel/Apple Silicon and Windows x86_64 pass native builds, machine/adapter tests, ABI/dependency/lifecycle gates, licenses and core/info packaging. All five downloaded ZIPs and internal checksums match the fixed source tag. Distinct Tiny/Model 2/combined packages follow their future compiled scopes. See `LIBRETRO_CI.md`. | Complete for the interim Model 1 preview; High reasoning |
| 🟠 | NetMerc | Its game-start problem blocks NVRAM and menu inclusion. Reconsider after that issue is resolved. | Estimate after the boot issue is understood |

### Phase 3.7 option inventory

| Candidate | Reference and Model 1 criterion | Next action |
| --- | --- | --- |
| Known Bad Dump ROM Repairs | TGPulse-Next already repairs the SHA-1-identified legacy `315-5711.bin` in memory. The Libretro System option defaults to Enabled, preserving the standalone behavior; Disabled leaves the original bytes intact. It affects SWA, Wing War and, once bootable, NetMerc sets that contain this chip. A restart is required. SM2's ROM CRC Verification is a different function. | Implemented; option registration and build are verified. A gated test covers both loader choices when a user-owned matching ZIP is supplied; that ROM-dependent test has not run here. Other repairs require individually documented original hashes and corrected bytes before inclusion. |
| A/V Timing | SM2's 60 Hz Compatibility duplicates occasional video frames without speeding the machine. Supermodel instead runs one machine frame per callback at 60 Hz and couples video and sound timing. Model 1 follows the user's Supermodel-style choice: Native 57.524160 Hz remains default; optional 60 Hz advances and renders every callback, speeds game and sound by about 4.3%, and resamples generated audio to the same output sample rate. | Implemented in the adapter, including the fractional audio position in Save States. Offline tests, option registration and the macOS release build pass. User-run presentation trials remain outside this roadmap. |
| Model 1 source audio | SM2 applies audio balance and DSB music volume live; Supermodel applies separate sound/music volume live. TGPulse standalone instead has output-only MultiPCM1/2, YM3438 and optional DSB gains/mutes. | Implemented as separate source controls with independent keys per ROM set, following SM2 NVRAM option registration/visibility. Parent and clone selections remain independent. Defaults retain standalone gains (50/50/30/100); source Gain selectors default to Auto (standalone levels), with Mute on the left and 0–100% in 10% steps on the right, without separate Mute switches; DSB options are visible only when that board is fitted, subject to frontend display-hint support. Changes apply live without stopping chip execution. Offline option tests and the macOS release build pass; listening remains user-run. |
| Aspect and widescreen | SM2 reports Auto/4:3/16:9 geometry without changing its 496×384 framebuffer. TGPulse standalone already has both behaviors: Auto reads VR's saved monitor setting and presents the native image; On widens the 3D field of view to 683×384 with optional 2D stretch, like Supermodel. The current Libretro software output is fixed at 496×384. | Presentation-only Aspect Ratio is implemented with Auto default, VR EEPROM byte 0x0a and 4:3 fallback. Changes report geometry live after option, Save RAM or Save State updates; the image stays 496×384. Offline tests and macOS release build pass. Hardware renderers now support wider 3D rendering and optional stretch of both native 2D layers; 4:3 and Software retain native geometry. |
| Driving analog ranges and response | Current SM2 and Supermodel expose live Steering/Accelerator/Brake Output Range (50–150%, default 100%) and Steering Response. Model 1 VR/VFormula currently maps steering around 0x80 and pedals from 0x20, within 0x20–0xE0. | Ranges implemented in phase 3.8; Steering Response implemented in phase 3.9. Scaling uses the existing center/rest, preserving native defaults and limits. Model 2 presets 63% (30-80-D0) and 75.3% (00-C0) require Model 1-specific validation rather than direct copying. These control emulated ADC output, not frontend binding assignment. |
| VR rumble | SM2 negotiates Libretro's rumble interface, drives P1 strong/weak motors and clears them at lifecycle boundaries. TGPulse standalone already decodes VR/VFormula motor commands into pad levels; its rumble default is Off. | Implemented by sharing the standalone pad policy and adapting only its output to Libretro. At the user's request the Libretro Gamepad Rumble option defaults On, matching SM2 while leaving the standalone default unchanged. It is shown only for VR/VFormula when frontend visibility hints are supported, and is inert when no rumble interface exists. Reset, unload, state load, input removal and disable stop both motors. Offline tests and macOS release build pass; physical-pad behavior remains user-run. |
| Timing/FPS display | SM2 composes its Supermodel-style Dear ImGui panel into software/GPU frames, uses 61-callback averages and Off/Auto/11–14 px options. TGPulse already supplies imgui 0.12. | Implemented using the existing ImGui dependency and an adapter-only software compositor. Defaults Off; Auto is 13 px. Reset/state load and option changes clear measurements. 24 offline tests and native-size visual inspection pass; 120 warmed synthetic draws averaged 0.421 ms. macOS release build and Development install are verified. This is not a gameplay benchmark. |

Vulkan, OpenGL/GLES, widescreen and supersampling are complete at this checkpoint.
The approved automatic templates and NVRAM menus are implemented. The first
five-platform Model 1 preview is published as 0.1.0.0. Tiny component selection
and later Model 2/combined outputs remain planned; local Development
installation continues after verified macOS release builds.

Estimates in the roadmap are planning ranges, not measured task costs. Codex
usage is an account-wide percentage; the ranges can change after source
inspection and are not additive. User-run trials are excluded.

For each phase, inspect the corresponding reference code and deliverables
again immediately before implementation. Record the smallest Model 1
adaptation in the [frontend map](LIBRETRO_MODEL1_FRONTEND_MAP.md). Keep the
reviewed [workbook](model1_core_options_review.xlsx),
[catalogue](MODEL1_DIAGNOSTIC_SETTINGS_CATALOG.md) and
[capture procedure](NVRAM_CAPTURE.md) as the NVRAM authority. Do not create
parallel implementation-status tables in those documents.
