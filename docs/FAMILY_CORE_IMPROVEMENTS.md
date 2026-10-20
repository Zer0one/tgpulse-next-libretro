# TGPulse-Next Core Family Improvement Register

## Bound Wireframe Traversal Before Rasterization — 2026-10-19

Model 1 Software now clips endpoints before integer Bresenham traversal,
following MAME `2194d88adc51`. A per-pixel bounds check alone does not bound
execution time or prevent arithmetic overflow. The complete core reuses this
Model 1 renderer. For a future Model 2 adapter, inspect its own rasterizer and
coordinate limits first; no Model 2 defect or patch is inferred. GPU uses
bounded per-pixel coverage and was not changed. See [source adaptation and
verification limits](MAME_WIREFRAME_CLIPPING_2026-10-19.md).

## Audio Reset Verification — 2026-10-18

For future M2/Complete adapters, reuse the PCM/reset test host and isolated
RetroArch recording procedure in [the Model 1 report](LIBRETRO_AUDIO_RESET_2026-10-18.md).
Inspect each destination's native buffering and producer lifecycle first;
Model 1 uses synchronous vectors and required no queue-priming correction.
Do not infer M2/Complete audio recovery from the Model 1 result. Keep recorded
PCM, actual device playback and user listening acceptance distinct.

## Frontend Option Ordering — 2026-10-17

User clarification: lexical ordering applies to functional groups rather than
splitting their related parameters. Preserve H/V range pairs consecutively
(Horizontal, Vertical); Model 1 now encodes and audits that exception. Review
other cores' actual parameter groups before adapting their order.

The user also approved parent-first MVD and Diagnostic LCD blocks. Delivered
using their existing settings-key groups, with independent final-registration
sequence assertions. Mode/enable precedes subordinate fields; diagnostic
capture follows normal tracking controls. Reuse the grouping principle, not
Model 1-specific members or another core's native operator ordering.

Delivered Framework 0.1.25 F04 through one common V2/legacy comparator. Preserve
System/native fields and global relative order; sort reviewed game-prefixed
labels after globals by full label in other categories. Reuse the extended
existing registration audit for future M2/Complete adapters; do not assume their
special groups or current menu rank tables match Model 1. Actual Model 1 RGUI
pages and all final registrations pass; no behavior change or shared workbook
edit. [Evidence](LIBRETRO_MENU_WORDING_2026-10-13.md).

## Final Registration Wording Audit — 2026-10-17

Reuse `tools/test_libretro_menu_wording.py` / LCFHost for future M2/Complete
adapters after reviewing their current applicable workbook rows and native
values. Compare full final metadata against the editable file, not templates
or counts; separately preserve choices/defaults/order across corrections.
Here the audit found common help containing per-game topology detail and a
tie in sort ranks that changed legacy placeholder order. Six-pair/common-help,
raw ID/stale text scans, legacy parity and the reconciled workbook pass.
[Report](LIBRETRO_MENU_WORDING_2026-10-13.md). Do not copy Model 1 game limits
or treat registration evidence as visual or multiplayer acceptance.

## Core Options Update Retention — 2026-10-17

Apply Framework F05's update-before-read rule to future adapters: RetroArch
GET_VARIABLE consumes its global update flag. Retain it before profile/visibility
reads, including paused callbacks; merge on Resume and clear for a new environment.
Use the existing effective-change reset logic. The Model 1 regression mock now
models consuming reads and fails on the former binary; real RGUI verifies ANRS,
NVRAM and combined topology/role changes. Source: `lib.rs` retain_option_update;
[evidence](NETWORK_TWO_OPTION_2026-10-17.md). This is a transfer of the existing
Supermodel solution, not a newly invented Framework contract.

## Two-Option Network Authority — 2026-10-17

Framework 0.1.21 supersedes the earlier board/topology/ANS policy below.
Model 1 now provides independent Linked Cabinets and ANRS, native labels,
unsupported-game placeholders, hidden-selector synchronization and last-applied
handover to NVRAM Settings. Reuse the authority/reset/publication contract for
M2/Complete after inspecting their native fields and required loopback cases.
Do not copy Model 1 limits or identity ordering: Formula's native numbering
already differs from VR despite sharing EEPROM color encodings. Six-set reset,
integrity/persistence and real VR pair evidence are in
[the delivery report](NETWORK_TWO_OPTION_2026-10-17.md). No future core is claimed
implemented by this entry.

## Network Board / Topology Separation — 2026-10-14

Model 1 implements Framework 0.1.18 board presence, explicit local transport,
offline ANS precedence, saved-selection preservation and automatic Resume
reconfiguration. Reuse the adapter contract for M2/Complete after inspecting
their actual board/transport and supported topologies; do not copy Model 1
wire frames or assume its board-present Save State restriction is universal.
Six-set ABI checks, native device echo and separate offline frontend trials are
recorded in [the integration report](NETWORK_INTEGRATION_2026-10-14.md).
This is a future-core adaptation candidate, not delivery of Model 2 networking.

## Purpose And Scope

Track improvements learned from the Model 1 Libretro core that may be useful
when this repository adds its planned Model 2 single-system and complete
Model 1 + Model 2 cores. This is a reuse and adaptation register, not an
implementation roadmap. Phase order, effort and delivery status belong only
in [the Libretro roadmap](LIBRETRO_ROADMAP.md). The separate
[upstream backport register](UPSTREAM_BACKPORT_CANDIDATES.md) tracks possible
changes to the standalone TGPulse-Next repository instead.

The targets are **M2** (Model 2 only) and **Complete** (both machine families).
Neither target has a finished Libretro adapter. A candidate records a pattern
to inspect, not an assertion that Model 2 hardware, ROM sets or frontend
behavior already support it. Keep one shared source/adapter infrastructure
with build-time machine selection; do not copy the Model 1 adapter wholesale.

## Candidate Register

Last reconciliation: **2026-10-09**, against the current local adapter and
project evidence. Local implementation does not imply publication or adoption
by a future core; frontend and physical-device checks remain separate.

| ID | Improvement To Reuse | Target And Adaptation Boundary | Evidence And Required Check |
| --- | --- | --- | --- |
| FC1 | Compile and package a genuinely scoped core | **M2 and Complete.** Select machine features and the catalogue at build time; give each output a distinct runtime name, binary, `.info` and package. Keep the standalone default combined. | [Build scope](LIBRETRO_BUILD_SCOPE.md), `crates/tgpulse-libretro/Cargo.toml`, `crates/tgpulse-core/build.rs` and `tools/check_libretro_artifact.py`. Verify compiled markers, dependencies, supported sets and metadata against each artifact; a ROM filter alone is insufficient. |
| FC2 | Keep frontend callbacks and machine state behind a narrow adapter | **M2 and Complete.** Reuse the frontend lifecycle, explicit timing/input/audio/Save RAM boundaries and deferred Core Option publication. Rebuild or reset the selected machine before its next frame when a setting changes the hardware topology. Keep set-specific state and persistence distinct in Complete. | `crates/tgpulse-libretro/src/lib.rs`, [frontend map](LIBRETRO_MODEL1_FRONTEND_MAP.md) and [linked-cabinet correction](LIBRETRO_MODEL1_LINKED_CABINETS.md). Check callback re-entry, Resume/Restart order, save preservation and first-frame output with the target machine. Do not assume Model 2 needs Model 1's COMM reconstruction. |
| FC3 | Generate reviewed operator options from authoritative per-set evidence | **M2 and Complete.** Reuse the screenshot → diagnostic YAML → filterable workbook → recipe/sample workflow, exact-set visibility, valid-save precedence and the NVRAM Settings master-switch/override contract. Model 2 needs its own field catalogue, encodings, integrity rules and approval. Complete should present only the loaded set's fields. | [NVRAM workflow](REFERENCE_WORKFLOW.md), `crates/tgpulse-libretro/src/nvram.rs`, `nvram_data.rs` and the Model 1 campaign artifacts. Verify parent/clone coverage, every possible value, sample round-trips, automatic defaults and user-save preservation before exposing a selector. |
| FC4 | Derive controls from native actions, with explicit cabinet/player profiles | **M2 and Complete.** Apply per-game names, role-specific ports, identical labels for multiple bindings to one action and the complete/reduced Test/Service Slots variants. Build new Model 2 mappings from its standalone signals; do not transfer Model 1 button or axis assignments. | [Input contract](REFERENCE_WORKFLOW.md), `model1_controls.rs` and [binding audit](LIBRETRO_MODEL1_BINDING_AUDIT.md). Check native bit/ADC polarity, port isolation, frontend remapping and descriptor/polling agreement for every supported set. |
| FC5 | Show global audio Gains only for fitted sound sources | **M2 and Complete.** Reuse source-driven visibility and global Gain persistence; map Model 2 sources and defaults from its own machine. Keep Master Volume independent of content. Store the displayed Mute value as raw `0`; retain published textual values as readable aliases until the user selects a canonical value. Do not rewrite options on read or require an offline migration. Model 1 FM Auto is 90%; this is not a Model 2 default. | [Audio option contract](REFERENCE_WORKFLOW.md#audio-option-contract) and `crates/tgpulse-libretro/src/lib.rs`. Isolated RetroArch checks cover global/per-game alias preservation and canonical reload; adapter checks cover zero Gain. Verify declared sources, live PCM response and visibility on load, unload and set changes for each target. Do not infer fitted hardware solely from a ROM name. |
| FC6 | Reuse frontend-owned renderers and presentation controls where applicable | **M2 and Complete.** Keep Software, Vulkan and OpenGL/GLES lifecycle in the adapter, then evaluate each Model 2 renderer feature against its native standalone path. Reuse menu wording only when behavior matches; Model 1 widescreen/supersampling shaders are not a Model 2 implementation. | [Renderer and option inventory](LIBRETRO_ROADMAP.md), `crates/tgpulse-libretro/src/gpu/` and [widescreen behavior](WIDESCREEN.md). Compare pixels, aspect, context recreation and fallback separately for each machine and backend. |
| FC7 | Keep Netpacket transport separate from native cabinet roles | **M2 and Complete, only for sets with verified native linking.** Reuse callback ownership, roster/session discipline and transient setup handling where the target protocol permits it. Adapt packet size, timing, eligibility, role/NVRAM rules and cabinet limit from that board; do not assume M1COMM semantics. | `crates/tgpulse-libretro/src/netpacket.rs` and [Model 1 link evidence](LIBRETRO_MODEL1_LINKED_CABINETS.md). Require an actual native link and multi-instance frontend test, not merely a connected lobby. |
| FC8 | Apply the same release and installation gates to every output | **M2 and Complete.** Extend the shared Makefile/CI/package recipe with separately named artifacts; verify native builds, ABI, compiled scope, licenses, checksums, source commit and installed-core hash. Never substitute a combined build for a dedicated core. | [CI procedure](LIBRETRO_CI.md), `Makefile.libretro`, `.github/workflows/libretro-ci.yml` and `tools/package_libretro.py`. Check each platform's archive and frontend identity independently. |
| FC9 | Retain Model 1 machine improvements in Complete without changing Model 2 paths | **Complete only.** The shared library should retain reviewed Model 1 tilemap, CPU scheduling, UART, lighting and audio improvements when both features are enabled. Any optimization of Model 2 needs its own baseline and equivalent-output evidence. | [Performance results](LIBRETRO_MODEL1_PERFORMANCE.md) and [backport candidates B11–B18](UPSTREAM_BACKPORT_CANDIDATES.md). Run both machine families under the combined feature selection and compare video, PCM, Save RAM and restored continuation. |
| FC10 | Check frontend sensor delivery before enabling motion controls | **Complete for NetMerc; M2 only if a future title has verified native motion input.** Keep host sensor acquisition in the shared Libretro adapter and its unavailable-sensor fallback. Require a frontend driver that actually supplies Libretro sensor callbacks; do not add direct evdev access to a core to compensate for a frontend limitation. | [Batocera sensor review](UPSTREAM_BACKPORT_CANDIDATES.md#batocera-dualsense-sensor-review--2026-10-07): Batocera 43.1 exposes live DualSense motion events, but its RetroArch 1.22.2 `udev` joypad driver has null sensor callbacks; the installed SDL2 path also reports no sensors. Current RetroArch source has `udev` sibling-sensor support, still unverified in a Batocera build. Check P1 gyro/accelerometer callback delivery, calibration, fallback and device disconnect in each target frontend. |
| FC11 | Show the NVRAM Settings switch only when the loaded set has reviewed operator fields | **M2 and Complete.** Keep the per-set field filter and the master-switch contract from FC3. Hide the general switch for sets with no supported field; expose only the loaded set's reviewed fields while it is enabled. A Complete core must recalculate visibility when switching between Model 1 and Model 2 content. | SM2 `src/libretro/core_options.h` `update_option_visibility()` hides the master switch when `options_for_game()` is empty; Model 1 `publish_nvram_option_visibility()` now follows reviewed-set support under the approved 2026-10-12 LCF alignment. Check load, unload, set changes, menu refresh and frontend fallback before adoption. |

## Latest Reconciliation — 2026-10-09

- **FC2 / FC7:** retain the published Resume/Restart ordering correction.
  An active Automatic Network Settings preset is authoritative only through
  enabled NVRAM Settings; Disabled preserves manual values. Refit topology
  before applying operator values and before the next native frame. Publish
  frontend selectors outside the core lock. Model 2 must use its own native
  topology and role mappings; a connected lobby alone is not proof of a link.
- **FC5:** replace the earlier mandatory textual-Gain migration proposal with
  compatibility registration and user-selected canonical values. Modern and
  legacy option APIs must both preserve recognized published values. Remove
  temporary aliases on a subsequent load once no active value needs them.
  A manually exercised menu change is separate from the completed persistence
  probes; no such device/menu acceptance is claimed here.
- **FC8:** reuse the sole `Makefile.libretro` release recipe for local builds,
  GitHub CI and source-side Libretro CI. The current five desktop targets are
  Linux x86_64/ARM64, macOS Intel/Apple Silicon and Windows GNU x86_64.
  Keep each future core's public filename, runtime identity and metadata
  consistent. GitHub package verification does not establish that the
  Libretro-hosted pipeline has run; see [submission evidence](LIBRETRO_SUPER_PR_REVIEW.md).
- **FC10:** the Batocera limitation remains in frontend sensor delivery.
  Do not transfer direct host-device access into any family core as a workaround.
- **FC11:** remains a proposal, not a delivered Model 1 visibility change.
  Review visibility conventions and explicit user exceptions before adaptation.

## Maintenance Rule

After a relevant Model 1 change, compare its actual implementation and test
evidence with these entries. Add or revise a stable ID only when the change
offers a concrete benefit to M2 or Complete. Record the source path, target,
machine-specific adaptation and verification limit. When a future roadmap
phase evaluates a candidate, link that decision here and record adoption,
rejection or supersession without deleting its history. Candidate registration
does not authorize implementing a future core, changing standalone upstream or
publishing a release.

## LCF Wording Follow-Up — 2026-10-12

FC5's current Model 1 source Gain labels now omit Absolute in both registration
paths. Common output-only Mute and immediate-effect wording is aligned with
LCF/SM2, retaining native Auto levels and saved-value compatibility. FC11 is
now an approved Model 1 roadmap requirement (phase 3.11), still unimplemented;
no future-core visibility change is delivered by the wording update.

## Authorized LCF Implementation Follow-Up — 2026-10-12

FC4: reuse the global Controller Profile Type, legacy-device normalization,
None preservation and shared L3 Service / R3 Test order. Keep native actions
and special-command exceptions per target. FC7: reuse canonical frontend ANS
IDs/relay while adapting native roles and retaining published-value reads;
never derive roles from host/client status. FC11 is implemented locally for
Model 1, with the master selector following actual reviewed-set support.
For future native driving feedback, reuse frontend-processed steering only for
existing synthetic cornering with the 5% onset; preserve native effects and
standalone policy. ABI callback tests and bounded frontend startup are distinct
from physical controls and visible paused-menu acceptance.

## Model 1 Direct/HUD Maintenance — 2026-10-19

Retain the shared direct-polygon decoder and above-HUD composition in the future
Complete core's Model 1 renderer. Keep native scene/HUD composition separate
from frontend Timing/Diagnostic UI; reuse buffers conditionally without new
binding layouts. Model 2 must retain its own command interpretation and layer
rules. See [implementation and bounded evidence](MAME_DIRECT_HUD_2026-10-19.md)
and B21; no future-core implementation is authorized by this record.
