# TGPulse-Next Libretro implementation roadmap

Status: 2026-10-04. This is the single active implementation plan for the
independent port. Model 1 is the current implementation scope; the final
repository goal is three distinct Libretro cores, defined below. User-run game,
controller and presentation trials are not scheduled here except the
user-requested R1 rumble/sensor correlation check. Design and evidence live in the linked technical documents.

## Final repository outputs

User-confirmed target (2026-10-02): one repository produces three distinct cores.

| Core | Compiled scope | Content scope |
| --- | --- | --- |
| Model 1 single system build | Model 1 machine/boards and required shared components only. | Supported Model 1 sets. |
| Model 2 single system build | Model 2 machine/board variants and required shared components only. | Supported Model 2 sets and variants; availability follows actual implementation. |
| TGPulse complete | Both machine families and their required components. | Supported Model 1 and Model 2 sets. |

Use shared source and adapter infrastructure, with build-time component
selection rather than three copied codebases or catalogue filtering alone.
Each output needs its own core artifact and matching `.info`; exact new names
remain to be agreed before packaging. The Model 1 core uses the M1 suffix: `tgpulse_next_m1_libretro`,
installed locally as `tgpulse_next_dev_m1_libretro` / Sega - Model 1 (TGPulse-Next Development).
The current source now selects only Model 1 components at compile time. The
published 0.1.0.2 preview is the Model 1 single system build. Preserve the standalone
full-system build and minimize changes to imported upstream code. No Model 3
output is planned. "Single system build" supersedes the earlier "Tiny" term
at the user's request (2026-10-02).

The Model 1 feature boundaries are implemented and verified locally and on
all five native CI platforms. Before
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
| SM2 3.6 and Supermodel NVRAM campaign | Catalogue all runnable parent and clone settings before choosing menu fields. Nine starting sets have complete isolated values; NetMerc's U1 startup now enables its separate U3 campaign. Apply set-specific templates only to new Save RAM and keep native defaults distinct from approved overrides. |
| SM2 3.7–3.8: menu review and applicable options | The user-approved workbook now defines the implemented NVRAM options. Separately assess timing, display, audio and input options against current Model 1 behavior. Keep native cadence as the default. Expose the already documented, SHA-1-guarded 315-5711 bad-dump repair as an opt-out load-time option; SM2's ROM CRC toggle is validation, not a repair mechanism. |
| SM2 4: compatibility and distribution | Reference matrix, core/info pairing, license texts, source revisions, ZIPs and checksums adapted for the interim Model 1 preview. Versions 0.1.0.0, 0.1.0.1 and 0.1.0.2 are published after all five native CI gates and downloaded-package verification. |
| SM2 5: frontend-owned GPU rendering | Keep the native software path; design any later GPU adapter around frontend-owned context and the existing TGPulse renderer. No renderer is assumed ready to port. |
| SM2 6: linked cabinets and Save States | Save RAM and Save States are implemented. Linked cabinet transport is now implemented in phase 6.1 with native Model 1 roles and operator settings. |

## Current Upstream Alignment — 2026-10-04

Reviewed target: TGPulse-Next `0.1.0.1`, commit
`f303b712455571b60cbea2249e1022935d398e22`, four commits beyond the imported
`3c75f9b2b155e8bb50a225c69520948ef72ecd48` baseline. The source inspection and
adaptation boundaries are recorded in [the upstream review](LIBRETRO_UPSTREAM_REVIEW.md).
This is a finite list of changes from the reviewed delta, not a recurring
upstream-check milestone. U1–U8 are integrated locally; U9 preparation is in progress. Existing completed phases
and local follow-up changes remain intact; Model 2/combined-core work follows
this current Model 1 alignment.

### Reference Mapping Before Prioritization

| Reference | Model 1 Adaptation And Deliberate Exclusion |
| --- | --- |
| SM2 phases 0–2 and section 7, selective upstream integration | Import neutral NetMerc I/O, DSP, resource and state changes while retaining local machine features, loader policy and adapter boundaries. Exclude desktop device/window ownership and preserve standalone buildability. |
| SM2 phases 3.1–3.2 and frontend-owned input contract | Audit native actions against the current standalone NetMerc wiring and aliases; adapt its pure pose helper without SDL dependencies or an extra dead zone. Follow SM2 3.1–3.2 and Supermodel profile/alias labels (full Button, not Btn). Keep the manual native Holder action in U2; its automatic sequencer is an eventual enhancement. MVD is P1 only. Calibration commands are frontend actions, not invented arcade controls. Neither reference has an equivalent MVD implementation to copy. |
| SM2 phases 3.1–3.2/3.7 and upstream neutral motion helper | U5 keeps host sensors in the adapter, normalizes Libretro axes/units to the upstream estimator and uses explicit monotonic polling time because the ABI omits event timestamps. User-approved three-second calibration preserves rejection thresholds; standalone default stays five seconds. No SDL device ownership, sensor state persistence or new host bindings. |
| SM2 phase 3.6 and Supermodel settings-acquisition/default policy | Add NetMerc to the existing screenshot → YAML/Excel → TOML → sample procedure after actual core startup. Acquire every operator setting/value, including network entries, before choosing menu options; retain U1's explicitly approved initial controller calibration; exclude unrelated test/diagnostic pages. Preserve existing nine-set selections and samples. |
| SM2 phases 3.4/3.5 Input conventions and percentage selectors; Supermodel Input option definitions | User-approved U5 sensor-drift follow-up: always-visible Input selector Off / 10–100%, default 50%, live/global policy. Neither reference core has an equivalent gyro-bias algorithm. Adapt the Fusion stationary-bias filter in the existing neutral standalone helper; compare GamepadMotionHelpers controller calibration conventions. Keep manual calibration/Recenter, frontend sensor ownership and machine-state boundaries. Exclude automatic recentering, sensor-driver changes and new dependencies. Follow-up effort M, estimated usage 0.5–1%. Optional CSV evidence reuses the same sensor callbacks and runner; neither reference supplies an equivalent recorder. |
| SM2 phase 3.7 and rumble/audio lifecycle | Map load-time donors and live gain/intensity preferences to scoped Core Options. Reuse the existing global Gain and Gamepad Rumble options; do not duplicate mute/binding controls. NetMerc defaults follow the current standalone; existing approved core defaults remain. |
| SM2 phase 3.7 and section 7 compatibility switches | Treat NetMerc City Workaround as a title-specific graphics correction implemented through the existing DSP conversion policy, with default On and live application. Keep it separate from controls and preserve upstream board scope; no GPU shader patch or general DSP rollout. |
| SM2 phase 5 and existing Timing-panel compositor | Compose the optional LCD within existing Software/Vulkan/OpenGL output. Exclude a desktop secondary window. Metal and sRGB remain outside the roadmap. |
| SM2 phase 6.2 and section 7 Save State import | Adopt the new complete device state and preserve destination preferences. Explicitly account for format 1 → 4 → 5 and changed resource identity; COMM state restrictions remain. |
| SM2 phase 4 and current port CI/package procedure | Retain five native platforms, component-scope checks, matching core/info, Development installation and notices. Publish only with explicit authorization. |
| SM2 phase 3.7 frontend rumble and input lifecycle; Supermodel frontend rumble | R1 compares the same physical DualSense/Bluetooth in Sega NetMerc with rumble Off/On/Off while the cabinet motor actually runs. Retain the approved core rumble policy; compare raw gyro/accelerometer, calibration and yaw under matched controller placement. Neither reference supplies a NetMerc MVD sensor/rumble interaction test. This is a user-requested physical check, separate from A1 source conformity. |
| SM2 phase 3.7 menu consistency and phase 4 delivery criteria; Supermodel NVRAM audit completion criteria | Perform the user-advanced reference conformity audit before U9 release preparation. Compare actual defaults, labels, profiles/device variants, notifications, lifecycle, settings/persistence, output and delivery with current SM2, Supermodel and standalone implementations. Record approved Model 1 adaptations separately from unexplained deviations; correct the latter and resolve genuinely ambiguous adaptations with the user. Exclude user-run gameplay/controller trials and recurring upstream maintenance. |

### Implementation Order And Estimates

Estimates are incremental account-usage percentage points at High reasoning,
not a reservation or a promise; measured account use includes other chats.
Effort is relative: S = narrow adaptation, M = several interacting adapter
changes, L = device/state import or complete acquisition. Verification is part
of each delivery, not a separate trial campaign. User game/controller trials
remain outside this roadmap except the specifically requested R1 comparison. Work proceeds one activity at a time.

| Status | Activity | Required Work And Delivery | Reasoning / Effort / Estimated Usage |
| --- | --- | --- | --- |
| 🟢 | U1. NetMerc machine, DSP and state import | Apply `27db9fa` selectively: firmware/database and generator, advanced board/tracking/LCD state, default SRAM initialization, finite DSP arithmetic, procedural audio and complete snapshots. Preserve feature gates, strict content identification, 315-5711 selector, user-save precedence and current local fixes. Adapt new ROM fixture fields and external state sizing/errors. Import applicable license attribution. Delivery includes component/build and existing-title regression checks plus macOS Development installation. The user confirmed adoption of format 4 without legacy state migration on 2026-10-04. Record format-1 incompatibility explicitly; Save RAM remains separate. Import the shared DSP finite implementation. The user confirmed on 2026-10-04 that activation must remain conditional on `Kind::NetMerc` for now, matching current upstream. Completed locally: 572 workspace tests, separate M1/M2/standalone gates, nine-set frame/audio/Save RAM regression, atomic legacy-state rejection, deterministic format-4 replay, NetMerc Vulkan startup, bounded frontend title restore and Development hash verification. `netmerc_nvram.bin` is optional; Automatic Initial NVRAM Setup supplies approved controller endpoints only for new saves, following Supermodel lightgun initialization. Evidence and frontend comparison limits: `LIBRETRO_U1_INTEGRATION.md`. | High / L / 3–7% estimate; account usage 31% at completion |
| 🟢 | U2. NetMerc controls and stick MVD | Approved profile **Special: Sega NetMerc**; Trigger Button (B/L2/R2), Thumb Button (A/L1/R1), MVD Holder (Start/Down), MVD Calibrate (X) and MVD Recenter (Y), without redundant game prefixes. Native left Y follows standalone 00 up / 7F center / FF down. P1 right-stick MVD, Auto/Off/Right Stick and live horizontal/vertical ranges (30/20-degree defaults, Off or 10–90 by 10) remain frontend policy. No extra dead zone or binding options. P2 retains Test/Service only. Virtual commands dispatch once per press; actual sensor calibration/recenter comes in U5. Preserve other profiles, state/reset/device edge synchronization and fixed-camera fallback. Automatic Holder remains an eventual enhancement. Completed locally: 58 adapter tests, native-latch Holder OSD, nine-set frame/audio/Save RAM/state regression, NetMerc live-option/command ABI checks, bounded RetroArch Vulkan replay and verified Development installation. Evidence: `LIBRETRO_U2_CONTROLS.md`. | High / M / 2–5% estimate; account usage 31% at completion |
| 🟢 | U1a. NetMerc City graphics correction | Always-visible Video option, default Enabled, applied live only to NetMerc. General Core Options remain visible regardless of loaded title; retain explicitly agreed per-set NVRAM/Linked Cabinets filtering. Use the upstream `set_netmerc_city_workaround` API for the firmware-specific TGP conversion that corrects City road-plane geometry. Retain destination preference after reset/state restore and keep other titles unchanged. This is separate from the U2 controls and from the mandatory NetMerc finite-arithmetic policy imported in U1. Completed: 59 adapter tests, native scope/state preservation, NetMerc controls ABI, nine-set 120-frame regression, artifact checks and verified Development installation. Evidence: `LIBRETRO_U1A_CITY.md`. | High / S / 0.5–1% estimate; account usage 32% at completion |
| 🟢 | U3. NetMerc complete operator-setting campaign | Completed: 4 photographed operator cycles / 20 values, native-exit samples and fresh-instance readback; full-range controller calibration saved/reloaded. YAML records backup RAM selector/dependent bytes; no foreign EEPROM CRC is assumed. Existing green filterable workbook extended with bold native defaults and pending review proposals; other 110 fields preserved. Source inventory: 10 sets / 114 fields / 973 values. Evidence: `LIBRETRO_U3_NETMERC_CAMPAIGN.md`. The subsequent U8 review and integration are complete. | High / L / 3–7% original estimate; account usage 32% at completion |
| 🟢 | U8. Reviewed NetMerc Automatic/NVRAM Settings | User approved the U3 workbook on 2026-10-04. Completed: automatic Export and approved initial controller policy, optional-seed preservation and native baseline fallback; Game Difficulty, Country and Advertise Sound selectors with native defaults and dependent bytes. Reuse SM2 phase 3.6 and Supermodel initial/override separation; adapt storage to backup RAM without EEPROM CRC. 43 fields across 10 sets; 60 adapter tests, complete NVRAM ABI gates, nine-set regression, two isolated RetroArch runs/readbacks and verified Development installation. Evidence: `LIBRETRO_U8_NETMERC_SETTINGS.md`. | High / M / 1–3% estimate; account usage 33% at completion |
| 🟢 | U4. NetMerc audio options and resources | Completed: always-visible Audio Donor VF/VR/SWA/Wing War/Off (default VF, reload); donor ZIPs searched only beside NetMerc, with strict PCM-only loading and atomic upstream bank application. SM2 phase 3.7/Supermodel audio scope adapted without new gain/mute controls. Global Auto/manual gains retained; resource identity covers actual banks/fallback. Fallback/procedural and incomplete-original notifications; valid donors silent. Alternative Audio Gains remains eventual. 62 adapter tests, native donor/state test, all donor/fallback ABI gates, nine-set regression, isolated RetroArch delivery and verified Development installation pass. See `LIBRETRO_U4_AUDIO.md`. | High / M / 1–3% estimate; account usage 33% at completion |
| 🟢 | U5. MVD sensor input | Completed: neutral upstream tracker driven by frontend-owned P1 sensor callbacks. Auto prefers sensors then stick/fixed; explicit Sensors falls back to fixed. Always-visible Gravity Stabilization defaults Enabled. User-requested three-second calibration (one-second warmup/two-second measurement) preserves the 100-observation and motion thresholds; polling timestamp/axis limitations are explicit. Existing virtual Calibrate/Recenter bindings retain one event per press and lifecycle edge resynchronization. Queued calibration/result/error feedback, gyroscope-only capability and lifecycle shutdown pass 71 adapter tests and real-time mock sensor ABI checks; ten-set controls and nine-set regression pass. Isolated RetroArch Vulkan exercises unsupported-sensor fallback. Development build/info installed with matching SHA-256. Approved startup follow-up keeps neutral publication until the first complete decoded measurement in all modes; incomplete/zero/Reset/state checks, 1,000-frame real-ROM replay and ABI gates pass. Current machine state format is 5, without legacy migration; corrected Development build/info installed and hash-verified. Approved drift-compensation follow-up is delivered: Off / 10–100%, default 50%, stationary-only learning speed. The approved optional Sensor Diagnostics follow-up adds a default-disabled live Input selector and bounded CSV capture through the frontend Save Directory. 79 adapter tests, eight mock-sensor ABI groups, standalone build check and verified Development installation pass. The first user-supplied Sony DualSense/Bluetooth session has been analyzed: 50% reduced yaw change by 60.4% against an Off replay in the final low-motion interval; the exact-tracker 50% replay matched the recording. Keep 50% as the initial default; broader device validation remains outside the implementation roadmap. See `LIBRETRO_U5_SENSORS.md` and `MVD_DUALSENSE_BLUETOOTH_2026-10-04.md`. | High / M / 2–5% original estimate; account usage 35% at original completion; follow-up M / 0.5–1% estimate; account usage 39% after physical-capture analysis |
| 🟢 | U6. NetMerc Rumble And Frontend Intensity | Completed: existing always-visible Gamepad Rumble (Enabled default) now delivers NetMerc port D bit 2 to both P1 motors at upstream 0.6 effect levels. User approved RetroArch Input Rumble Gain instead of a duplicate core intensity selector. User-requested follow-up restores standalone final-latch sampling; same-frame On/Off activity no longer creates an extra pad effect. Targeted final-latch and native lifecycle checks pass; updated Development core/info installed with matching SHA-256. Disable/device/reset/state/unload/error shutdown retained; VR/VFormula decoder unchanged. Three native motor tests, 71 adapter tests, positive synthetic-firmware-to-callback delivery, real NetMerc ABI lifecycle, nine-set regression, native artifact gates and RetroArch Vulkan pass. Development core/info installed with matching SHA-256. Physical motor response remains user-run. See `LIBRETRO_U6_RUMBLE.md`. | High / S–M / 1–2% estimate; account usage 36% at completion |
| 🟢 | U7. Optional Diagnostic LCD | Completed: three always-visible Video options (OFF / ON (100%) / ON (50%), corner, background opacity), HD44780-only rendering, optional validated BIOS and queued missing/invalid notice only when active. System → adjacent BIOS ZIP → game ZIP applies to LCD and game-selected required I/O firmware. Native dot/state/blink APIs and existing Software/Vulkan/OpenGL compositor reused; wide/2x placement and alpha verified. The user-requested follow-up halves LCD dimensions in the existing selector and draws Timing above LCD at overlapping corners without displacement; see the delivery document for focused verification. 73 adapter tests, seven focused native tests, resource/state/notification ABI gates, nine-set regression, real macOS Software/Vulkan and Linux desktop OpenGL/GLES context recreation pass. Missing-font OSD verified in actual RetroArch pixels. Development core/info installed with matching SHA-256. Evidence: `LIBRETRO_U7_LCD.md`. | High / M / 1–3% estimate; account usage 37% at completion |
| 🟢 | A1. Final reference conformity audit | Completed before U9 release preparation on 2026-10-04. Source-linked comparison covers options, defaults, Title Case/menu order/visibility, complete/reduced profiles and bindings, notifications, audio/video/timing, NVRAM initialization/overrides, persistence/state/lifecycle and build/install/package conventions. Approved Model 1 adaptations and remaining physical-evidence boundary are documented in `LIBRETRO_A1_REFERENCE_AUDIT.md`. Corrected the diagnostics lifecycle description and stale README/roadmap copy. 79 adapter tests, macOS release build, ABI/scope/lifecycle gates and Development core/info hash verification pass. Publication remains subject to explicit authorization. | High / M–L / 2–5% preliminary estimate |
| 🟡 | U9. Alignment documentation and release preparation | Reconcile source provenance, current feature descriptions, notices, core/info and four-component public version under the existing version policy. Prepare the 0.1.0.3 candidate and five-platform package gates. Use isolated Batocera Linux evidence when available; prepare the matching Windows core and required game ZIPs for the user's existing RetroArch when that PC boots Windows 11. Keep build/package, frontend and user gameplay evidence distinct. A verified macOS release build requires Development install/hash check. Source commit/push, CI publication and release upload require explicit authorization. | High / M / 1–3% revised estimate; in progress |
| 🟠 | R1. DualSense rumble/MVD correlation | User-requested controlled Off/On/Off capture on the same Sony DualSense/Bluetooth, with the pad in matched positions and a verified active NetMerc motor interval. Compare gyro/accelerometer noise, calibration acceptance, stillness dwell and yaw slope; record whether host rumble timing must be added to the existing diagnostic CSV to make the comparison conclusive. The first 50% capture lacks a rumble-state marker and cannot establish causality. Await comparable physical captures; keep the current rumble and 50% drift defaults until evidence supports a change. | High / S–M / 0.5–2% provisional; blocked on user-operated hardware evidence |

A1 is complete; its source comparison and corrections are recorded in [the A1 audit](LIBRETRO_A1_REFERENCE_AUDIT.md).
U9 follows the source/behavior audit; a later release remains separately
authorized. R1 is a targeted physical-controller comparison at the user's
request and does not block the independent A1 source audit or U9 preparation.
U8 followed U3 after workbook approval; U4–U7 are complete.
[Diagnostic LCD](LIBRETRO_U7_LCD.md) records U7 delivery and verification.
U3 coverage acquisition is separate from
U7's optional live LCD presentation. NetMerc startup is verified in the installed
Development core; published 0.1.0.2 and the initial nine-set campaign remain
unchanged. The upstream standalone's acceptance is not inherited.

U6 approved adaptation: [NetMerc Rumble](LIBRETRO_U6_RUMBLE.md) records the current
standalone/SM2/Supermodel comparison, native motor and final-latch sampling, and
delivery criteria. The user approved delegation of intensity to RetroArch on 2026-10-04,
following SM2 and replacing the original U6 core intensity-selector requirement.

U8/U4 follow-up on 2026-10-04: removed the erroneous four-byte bookkeeping
guard; deferred factory correction until missing-save initialization actually
needs the supplied optional seed; missing `.bin` remains silent. Queued audio
warnings are deferred until the first delivered frame and visibly verified in
RetroArch with Donor Off and an isolated personal-save copy. Native loader,
adapter, audio/NVRAM ABI and artifact gates pass; Development installed with
verified SHA-256. Evidence is recorded in the existing U8/U4 documents.

The user confirmed the state compatibility change without migration on
2026-10-04. MVD sensor timing was resolved for U5: monotonic host polling, a bounded
poll rate and the user-requested three-second calibration window, with the
original sample and movement thresholds retained. Source review confirms that finite arithmetic
is implemented in the shared DSP, while current upstream selects it only for
the NetMerc board; preserve that distinction in implementation and reporting. No additional CPU, full Polhemus simulation, general DSP policy
rollout, Metal or sRGB is required by this alignment.

### Eventual Enhancements

User classification updated on 2026-10-04: these features are outside the
required alignment sequence and do not gate U9. Implement them only if selected
later; the existing manual Holder command and global audio gains remain part
of the required core behavior. Their reference is the current TGPulse-Next
frontend convenience/output policy, adapted under SM2 phase 3.7 conventions;
neither feature has a matching SM2/Supermodel implementation to copy.

| Status | Activity | Possible Adaptation | Reasoning / Effort / Estimated Usage |
| --- | --- | --- | --- |
| 🔵 | E1. Automatic MVD Holder | After U2, optionally reuse upstream's frame-bounded Holder sequencer with Auto/Manual selection and read-only game context. Preserve manual Holder binding, pulse limits, native acknowledgements and state/lifecycle behavior. This automation is separate from the native Holder input. | High / S–M / 0.5–2% |
| 🔵 | E2. NetMerc Alternative Audio Gains | After U4, optionally expose the live upstream preset, default Off. Select coefficients from the actually loaded donor/procedural/original path; preserve stored manual gains, master volume and mutes, and restore manual gains when disabled. | High / S / 0.5–1% |

## Milestones

Legend: 🟢 complete · 🟡 partial · ⚪ not started · 🔵 deferred · 🟠 blocked.
The blocked rows are last, regardless of their phase number.

| Status | Phase | Implementation state and next work | Indicative effort and Codex usage |
| --- | --- | --- | --- |
| 🟢 | 0–2. Baseline and software core | Independent source snapshot, frontend-neutral machine reuse, Model 1 loader/lifecycle, software video, stereo audio and basic Core Options are present. | Complete |
| 🟢 | 3.1–3.2. Model 1 controls | [Binding audit](LIBRETRO_MODEL1_BINDING_AUDIT.md) findings B1–B5 addressed: corrected VF native bits, removed VF L/R by user instruction, restored flight shoulder/right-stick throttle alternatives and standalone flight response. Wing War name corrected to VR4. Follow-up descriptor/native-input audit: SWA duplicate view bindings both read VR1; unapproved VF analog directions removed. 34 adapter checks pass. | Complete |
| 🟢 | 3.3. Operator-setting acquisition | Nine starting parent/clone sets have screenshot catalogues, diagnostic YAML, workbook rows and 953 isolated persistent values across 110 fields. NetMerc was excluded from that initial campaign; U1 now verifies startup for the subsequent U3 acquisition. | Complete |
| 🟢 | 3.4. NVRAM menu review | User approved 40 fields across nine sets, including Monitor in VR; Automatic Cabinet corrected to Special. Workbook and per-set YAML record the approved selection. | Complete |
| 🟢 | 3.5. Automatic Initial NVRAM Setup | Enabled by default. Complete independent set templates initialize absent/invalid Save RAM; valid existing saves take precedence. Native calibration and unrelated settings remain in each template. VR starts EXPORT / NO LINK / SPECIAL. | Complete; High reasoning |
| 🟢 | 3.6. NVRAM Settings | Disabled by default. All 40 reviewed fields have per-set keys, observed native values/defaults, exact-set visibility and native CRC/mirror updates. Changed selections reset the machine to refresh native caches. 41 adapter checks, all nine ABI sets and isolated RetroArch save readback pass; Development installed. | Complete; High reasoning |
| 🟢 | 6.2. Frontend persistence | Save RAM and Save States use the existing machine APIs and frontend-owned files. | Complete |
| 🟢 | 3.7. Software Core Options | Smooth Shadows, presentation-only Aspect Ratio, Master Volume, per-source Model 1 Gain selectors (Mute/Auto/0–100%), VR-family Gamepad Rumble, Timing / FPS Overlay, Known Bad Dump ROM Repairs and selectable native/60 Hz timing exist. In 60 Hz mode each callback advances and renders a new machine frame, so game and sound run about 4.3% faster; audio is resampled to the native output rate. The listed options are implemented; driving ranges are implemented in phase 3.8; steering response is implemented in phase 3.9. | Complete |
| 🟢 | 5.1. Vulkan renderer | Frontend-owned Vulkan v5 device/queue, shared standalone raster/resolve shaders, synchronized image handoff and context lifecycle. Native images are verified in macOS RetroArch through existing MoltenVK. | Complete; High reasoning |
| 🟢 | 5.3. OpenGL/GLES renderer | Frontend-owned OpenGL 4.3/GLES 3.1 compute adapter. Actual 2D/3D images, normal context recreation and unannounced loss verified through the SM2-style EGL host on Linux Mesa. Audio and NVRAM match Software. macOS GL 4.1 cannot run this compute path. | Complete; High reasoning |
| 🟢 | 5.2a. Widescreen and supersampling | Three widescreen modes and native supersampling scales 1–4, applied at content reload. Widescreen modes affect wide aspect only; supersampling also applies in 4:3. Vulkan and OpenGL/GLES use the standalone shaders. | Complete; High reasoning |
| 🟢 | 3.8. Driving analog ranges | Steering, Accelerator and Brake Output Range for VR/VFormula and their recognized sets: 50–150% in 10% steps, default 100%, live updates; controls remain visible for all titles. Scaling preserves Model 1 center/rest and native limits. 39 adapter checks, macOS release build, isolated RetroArch registration/frame delivery and Development installation pass. | Complete; Medium reasoning |
| 🟢 | 3.9. Driving Steering Response | Linear / Progressive (Fine Center) / FBNeo Logarithmic (Fine Center), default Linear. Current SM2/Supermodel curves adapted to Model 1 20–80–E0, before output range; VR/VFormula only, live updates. 44 adapter checks, macOS release build and bounded RetroArch registration/frame delivery pass; Development installed. | Complete; Medium reasoning |
| 🟢 | 6.1. Linked cabinets | SM2/Supermodel Netpacket workflow adapted to existing M1COMM. Per-set selectors default Disabled; native NVRAM roles retained. All six eligible sets reached game-created links in real RetroArch pairs; VR also passed LIVE relay. COMM Save States remain unavailable. 47 adapter checks, macOS build and Development install pass. | Complete; High reasoning |
| 🟢 | 4.1. Model 1 single system build | Reference: SM2 `PORTING_PLAN.md` phase 4 system-specific build/ABI/core-info/license/package gates; Supermodel `Docs/ROADMAP.md` has no multi-system split to copy. Adaptation: `model1`/`model2` Cargo machine features, both by default for standalone; M1 adapter selects only `model1`. Single-source catalogue is selected at build time: 10 M1 entries; i960, MB86235, SHARC, Model 2 machines/SCSP and the combined debugger are excluded. Library M2-only and combined configurations remain available; M2 Libretro is still phase 7. Local M1/full/M2 and adapter tests, nine-set pre-split video/audio/state/Save RAM equality, RetroArch Vulkan, package/scope checks and Development installation pass. Binary 3,557,184 → 3,422,352 bytes (−3.79%); warm scoped rebuild 18.540 → 13.097 s. See `LIBRETRO_BUILD_SCOPE.md`. | Complete; High reasoning; account 55% → 56% during implementation. Version 0.1.0.2 passes all five native CI builds and downloaded-package verification. |
| 🔵 | 7. Model 2 and combined core adaptation | After the current Model 1 scope, inspect actual standalone board support and reference features, then extend the shared adapter for Model 2 and build both Model 2-only and combined cores. Keep system-specific timing, inputs, rendering, NVRAM and state explicit. | High; effort and usage estimate after reference/board review |
| 🟢 | 4. Build and distribution — Model 1 preview foundation | Versions 0.1.0.0, 0.1.0.1 and 0.1.0.2 published as prereleases. Linux x86_64/ARM64, macOS Intel/Apple Silicon and Windows x86_64 pass native builds, machine/adapter tests, ABI/dependency/lifecycle gates, licenses and core/info packaging. All five downloaded ZIPs and internal checksums match the fixed source tag. Version 0.1.0.2 delivers the single-system component split; Model 2/combined Libretro packaging follows their future adapters. See `LIBRETRO_CI.md`. | Complete for the Model 1 preview; High reasoning |
| 🟢 | NetMerc NVRAM inclusion | U1 startup, U2 controls, U3 complete acquisition and user-approved U8 integration are complete. Automatic Export and the three reviewed operator selectors pass ABI, RetroArch readback and Development installation gates. | Complete; see U8 and `LIBRETRO_U8_NETMERC_SETTINGS.md` |

### Phase 3.7 option inventory

| Candidate | Reference and Model 1 criterion | Next action |
| --- | --- | --- |
| Known Bad Dump ROM Repairs | TGPulse-Next already repairs the SHA-1-identified legacy `315-5711.bin` in memory. The Libretro System option defaults to Enabled, preserving the standalone behavior; Disabled leaves the original bytes intact. It affects SWA, Wing War and, once bootable, NetMerc sets that contain this chip. A restart is required. SM2's ROM CRC Verification is a different function. | Implemented; option registration and build are verified. A gated test covers both loader choices when a user-owned matching ZIP is supplied; that ROM-dependent test has not run here. Other repairs require individually documented original hashes and corrected bytes before inclusion. |
| A/V Timing | SM2's 60 Hz Compatibility duplicates occasional video frames without speeding the machine. Supermodel instead runs one machine frame per callback at 60 Hz and couples video and sound timing. Model 1 follows the user's Supermodel-style choice: Native 57.524160 Hz remains default; optional 60 Hz advances and renders every callback, speeds game and sound by about 4.3%, and resamples generated audio to the same output sample rate. | Implemented in the adapter, including the fractional audio position in Save States. Offline tests, option registration and the macOS release build pass. User-run presentation trials remain outside this roadmap. |
| Model 1 source audio | SM2 applies audio balance and DSB music volume live; Supermodel applies separate sound/music volume live. TGPulse standalone instead has output-only MultiPCM1/2, YM3438 and optional DSB gains/mutes. | Implemented as separate source controls with global keys shared by all ROM sets, following SM2 and Supermodel audio option scope. Defaults retain standalone gains (50/50/30/100); source Gain selectors default to Auto (standalone levels), with Mute on the left and 0–100% in 10% steps on the right, without separate Mute switches; DSB options are visible only when that board is fitted, subject to frontend display-hint support. Changes apply live without stopping chip execution. Offline option tests and the macOS release build pass; listening remains user-run. |
| Aspect and widescreen | SM2 reports Auto/4:3/16:9 geometry without changing its 496×384 framebuffer. TGPulse standalone already has both behaviors: Auto reads VR's saved monitor setting and presents the native image; On widens the 3D field of view to 683×384 with optional 2D stretch, like Supermodel. The current Libretro software output is fixed at 496×384. | Presentation-only Aspect Ratio is implemented with Auto default, VR EEPROM byte 0x0a and 4:3 fallback. Changes report geometry live after option, Save RAM or Save State updates; the image stays 496×384. Offline tests and macOS release build pass. Hardware renderers now support wider 3D rendering and optional stretch of both native 2D layers; 4:3 and Software retain native geometry. |
| Driving analog ranges and response | Current SM2 and Supermodel expose live Steering/Accelerator/Brake Output Range (50–150%, default 100%) and Steering Response. Model 1 VR/VFormula currently maps steering around 0x80 and pedals from 0x20, within 0x20–0xE0. | Ranges implemented in phase 3.8; Steering Response implemented in phase 3.9. Scaling uses the existing center/rest, preserving native defaults and limits. Model 2 presets 63% (30-80-D0) and 75.3% (00-C0) require Model 1-specific validation rather than direct copying. These control emulated ADC output, not frontend binding assignment. |
| VR rumble | SM2 negotiates Libretro's rumble interface, drives P1 strong/weak motors and clears them at lifecycle boundaries. TGPulse standalone already decodes VR/VFormula motor commands into pad levels; its rumble default is Off. | Implemented by sharing the standalone pad policy and adapting only its output to Libretro. At the user's request the Libretro Gamepad Rumble option defaults On, matching SM2 while leaving the standalone default unchanged. It remains visible for all titles, applies to VR/VFormula and is inert when no rumble interface exists. Reset, unload, state load, input removal and disable stop both motors. Offline tests and macOS release build pass; physical-pad behavior remains user-run. |
| Timing/FPS display | SM2 composes its Supermodel-style Dear ImGui panel into software/GPU frames, uses 61-callback averages and Off/Auto/11–14 px options. TGPulse already supplies imgui 0.12. | Implemented using the existing ImGui dependency and an adapter-only software compositor. Defaults Off; Auto is 13 px. Reset/state load and option changes clear measurements. 24 offline tests and native-size visual inspection pass; 120 warmed synthetic draws averaged 0.421 ms. macOS release build and Development install are verified. This is not a gameplay benchmark. |

The 2026-10-03 follow-up corrects Timing panel alpha over GPU 3D output and
aligns modern/legacy option grouping with current SM2/Supermodel definitions.
48 adapter checks, shader translation, ABI, isolated VR Vulkan output and
Development installation pass; evidence is in `REFERENCE_WORKFLOW.md`.
The Widescreen Hack follow-up excludes native-capable games, defaults to 3D-only
expansion and preserves stored option values. 49 adapter checks and native/
expanded Vulkan geometry checks pass. The Timing position follow-up anchors the panel to final render coordinates,
preserving the game's 2D mapping; 50 adapter checks and all three widescreen
Vulkan captures pass. These corrections are local and are not part of the
published 0.1.0.2 tag.

Vulkan, OpenGL/GLES, widescreen and supersampling are complete at this checkpoint.
The approved automatic templates and NVRAM menus are implemented. The latest
five-platform Model 1 preview is published as 0.1.0.2. Model 1 component selection
is implemented and verified locally and across the native CI matrix. Model 2/combined Libretro outputs remain
planned; local Development installation continues after verified macOS builds.

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
