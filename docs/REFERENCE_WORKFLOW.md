# Reference adaptation workflow

## Upstream Backport Register

After each implementation phase and upstream comparison, update
[Upstream Backport Candidates](UPSTREAM_BACKPORT_CANDIDATES.md) with potentially
reusable local fixes/helpers, source evidence, dependencies and adaptation
limits. Record adoption or supersession against an explicit upstream revision.
This register is separate from the sole implementation roadmap; proposing a
candidate does not authorize changing, committing or publishing upstream.

## Apply this process to every phase

Before planning or reprioritizing a phase, inspect the corresponding current
roadmap sections in SM2-Emu Libretro `PORTING_PLAN.md` and, when relevant,
Supermodel `Docs/ROADMAP.md`. Record the reference-to-Model-1 mapping and any
exclusion in the sole [Libretro roadmap](LIBRETRO_ROADMAP.md).

1. Inspect the current reference project's complete path for the phase:
   implementation, visible behavior, build, local installation, documents,
   scripts, verification evidence and delivered artifacts. Record which parts
   are actually present; do not infer a procedure from a feature description.
2. Identify the existing solution and choose the smallest adaptation that keeps
   the user interface consistent and future upstream updates manageable.
3. Reuse existing standalone integrations where applicable. Keep host/frontend
   behavior in the adapter and shared emulation independent of Libretro.
4. Ask the user when an adaptation changes intended behavior or requires an
   unresolved product decision. Proceed with routine work already authorized.
5. Deliver implementation, reusable procedures and expected supporting artifacts
   together. On macOS, a verified release build is followed by the authorized
   `tgpulse_next_m1_libretro` public RetroArch installation and SHA-256 comparison
   (user naming update: 2026-10-05).
   Record the reference, adaptation, verification and remaining gaps. A phase
   with a failed or omitted required delivery step is incomplete.
6. Report required model/reasoning level before substantial work and account
   usage/reset after completed verification. Commit/push require explicit approval.

Project documents, YAML explanations and workbook labels are written in English.
Conversation can remain in Italian.
Maintain implementation status only in [the Libretro roadmap](LIBRETRO_ROADMAP.md);
keep this file procedural and other documents as evidence or design references.

## Sega NetMerc Menu Naming

Use **Sega NetMerc** wherever the game name appears in Core Option labels and
help text, for both modern and legacy registrations. Keep stored option keys
and technical identifiers unchanged. The existing specific controller profile
remains **Special: Sega NetMerc**.

## Input and descriptor contract

For each profile, inspect the standalone signal catalogue, sampling aliases,
player/cabinet encoding and corresponding SM2 bindings before changing input.
Every exposed gameplay control must correspond to an existing native action.
Multiple physical bindings for the same action use the **same action label**;
do not invent an `Alternate` input or a second arcade switch. Role qualifiers
may distinguish SWA Pilot/Gunner ports, but never create absent role controls.
P2 Test/Service duplication is the explicitly approved exception. Native Coin 2
is shown only for cabinets that actually have it. Preserve the user's explicit
removal of VF shoulder bindings. Frontend remapping owns extra physical input
choices; the adapter must not add unsupported VF analog gameplay controls.

Profiles must include both reference device variants per port: the complete
`<Profile> + Test/Service Slots` on base RetroPad (the default), and `<Profile>`
on subclass 0 with cabinet slots disabled. Inspect SM2 `configure_controllers`,
`service_enabled`, `joypad_enabled` and Supermodel `set_controller_info` plus
input polling together. Reduced profiles remove Test/Service descriptors and
polling, while retaining all gameplay buttons, analog axes, MVD commands and
rumble. Keep per-port selection independent and preserve SWA Pilot/Gunner names.

## Audio option contract

Master Volume uses default 100%, standalone 0–800% range and reference-core
10% steps. At the user's request, zero is labelled OFF. No Mute/Muted label or
Auto value is added to Master.
Per-source Gain uses global core keys shared by all Model 1 sets, with Auto
as the default and the agreed Mute/Auto/0–100% selector. FM Auto uses 90%
in Libretro; its standalone reference level remains 30%. Follow SM2 and
Supermodel's global audio option scope. Old set-qualified Gain keys are ignored.
Filter registered Gain keys against the loaded machine's `SoundSystem::sources()`;
do not infer hardware from the set name or from allocated ROM buffer sizes.
Only the Gains for fitted sound sources are shown while content is loaded:
MultiPCM 1/2 and YM3438 for every Model 1 set, plus DSB for Star Wars Arcade
and its clone. Hide all four source Gains before load and after unload; keep
Master Volume visible. Visibility never changes the stored global Gain values.
Future single-system adapters can map their own Gain keys to their machine's
declared audio sources through this same frontend rule.
Order the modern numeric selector ascending and use identical load-time/live parsing.
Verify live PCM changes from the same restored machine state before claiming
progressive gain; game progression must not alter the reference segment.

For NetMerc donor audio, reuse the upstream strict PCM loader and atomic bank
application. Game donor ZIPs are searched only beside `netmerc.zip`; an alternate
system-directory path is reserved for I/O and Diagnostic Display BIOSes.
Keep the global donor selector always visible and apply it on content reload.
Notify only fallback/procedural operation or incomplete original audio; successful
donor loading is silent. Logs retain resource failures. Save State compatibility
uses the actually loaded banks and procedural flag, never the preference alone.
Use queued extended messages when available, with a legacy fallback; defer audio
startup warnings until the first delivered frame. Verify actual OSD pixels,
not just callback acceptance. Keep isolated notification runs free of asset
extraction and audio-driver error messages.

NetMerc initialization patches the complete optional `.bin` only when there is
no valid frontend save and that seed is needed. Never gate operator settings on
the first four bookkeeping bytes, and never normalize existing user bookkeeping
or calibration implicitly. Explicit enabled NVRAM Settings overrides remain
separate from automatic initialization. Validate buffer size and preserve every
byte outside the approved patch offsets.

## LCD And I/O BIOS Lookup Policy

User correction for U7: search the agreed frontend `system/tgpulse-next/`
location first, then the device ZIP beside the game, then the game ZIP. Apply
this order to `hd44780.zip`, `model1io.zip` and `model1io2.zip`; validate the
resource appropriate to the identified game. Donor game ROMs remain adjacent
only. LCD presentation supports HD44780 exclusively. Its BIOS is optional;
notify its absence or invalidity only when display presentation is active,
without substituting Text or failing game startup. I/O firmware remains required.

## Live reference locations

SM2-Emu Libretro: `/Users/andrea/dev/sm2-emu-libretro`.
Supermodel Libretro: `/Users/andrea/dev/libretro-supermodel-modern`.
These are reference workspaces, not source dependencies. Verify their current
files before reuse; an old milestone summary is only a navigation aid.

## Procedure audit for remaining phases (2026-10-01)

This audit checked the current reference files, not only their roadmap labels.
Recheck the listed implementation and scripts at the start of each phase because
the reference workspaces may change. The final column is the Model 1 delivery
gate; it does not claim that the pending feature is implemented.

| Model 1 phase | Verified reference procedure | Model 1 adaptation and delivery gate |
| --- | --- | --- |
| Aspect and widescreen | SM2 `src/libretro/core_options.h` and `core.cpp` select Auto/4:3/16:9 from cabinet NVRAM and send `SET_GEOMETRY` without changing the 496×384 image. Supermodel `Docs/ROADMAP.md` (Cross-core widescreen assessment), `Docs/README.md` and `Src/OSD/libretro/libretro.cpp` distinguish frontend stretch from a wider rendered 3D field and optional 2D background extension. | First establish from Model 1 settings and renderer whether any cabinet selects a native aspect. Treat presentation geometry and wider 3D output as separate decisions. Reuse the existing standalone Off/On/Auto and 2D-stretch semantics only where the active Libretro renderer can reproduce them. Document the chosen pixel geometry, restart behavior and visual evidence before adding a menu item. |
| VR rumble | SM2 `src/libretro/rumble.h`, `core.cpp` and `core_options.h` negotiate the frontend rumble interface, decode drive writes, update per frame and stop rumble at lifecycle boundaries. Supermodel `Src/OSD/libretro/libretro.cpp` also negotiates the Libretro interface. | Trace Model 1's actual VR drive command and the standalone `crates/tgpulse/src/input/model1_rumble.rs` policy. Adapt only the output to Libretro; verify unsupported frontends, option changes, unload/reset and non-VR silence. Keep physical-pad evidence separate. |
| Timing/FPS overlay | SM2 `src/libretro/timing_overlay.h`, `core.cpp` and `core_options.h` measure 61 callbacks, draw on software/GPU paths and reset on option changes. Supermodel `Src/OSD/libretro/libretro_core_options.h` warns of an extra draw pass. | Measure the Model 1 software path before selecting fields, font and draw method. Default Off; verify legibility and bounded overhead at 496×384. If a GPU path is added later, verify it separately. |
| Reviewed NVRAM menus | SM2 `scripts/libretro_nvram_samples.py`, `generate-initial-nvram-templates.py`, `audit-nvram-core-catalog.py`, `scripts/NVRAM_CAMPAIGNS.md` and `PORTING_PLAN.md` separate native samples, complete initial templates, per-field overrides and user-save precedence. Supermodel `Docs/ROADMAP.md` requires direct clone evidence and native defaults distinct from approved overrides. | Continue from the Model 1 screenshot catalogue, YAML, recipes and review workbook already acquired. Wait for the user's revised selection; then implement only reviewed options, prove each encoding/integrity rule and preserve existing Save RAM. Do not repeat the full acquisition campaign without a demonstrated gap. |
| GPU renderer | SM2 `PORTING_PLAN.md` phase 5 and `GPU.md` require frontend-owned context, loss/recreation handling and software-baseline comparison. | Adapt the existing TGPulse renderer only after checking the actual target frontend contexts. Keep the software path, verify context recreation and compare video/audio/NVRAM from the same content and inputs. |
| Linked cabinets | SM2 `PORTING_PLAN.md` phase 6.1 and `src/libretro` networking code adapt only transport to Libretro Netpacket, keep the machine protocol, and test roster, role, limits and disconnects. Supermodel's frontend netboard code is a second reference. | Identify Model 1 title support, cabinet roles and operator settings first. Preserve the TGPulse COMM protocol; define frontend transport and per-set visibility before exposing the option. Test simulated protocol and real multi-instance transport separately. |
| Final build/distribution | SM2 `CI.md`, `scripts/check-libretro-package.py` and `README.md` verify declared platforms, ABI, matching `.info`, packaged system files, checksums, licenses and no ROMs. Supermodel `Docs/README.md` documents platform installation and system assets. | At the deliberately final distribution phase, add only builds and files actually supported by Model 1. Verify each package and platform separately. This is distinct from the routine local Development-core installation after each verified macOS release build. |

The reference repositories do not provide a directly reusable local macOS
`tgpulse_next_dev_m1_libretro` installer. Their public installation instructions
cover core and `.info` placement; the exact local Development name and hash
check are this project's established convention. Verify the installed artifact
after every verified macOS release build, even when no frontend test is run.

| Phase | Reference material | Required target artifacts |
| --- | --- | --- |
| Frontend controls/options | SM2 adapter, profiles, descriptors and options; standalone integrations | Frontend map, minimal adapter changes and runtime evidence |
| NVRAM acquisition | SM2 `scripts/libretro_nvram_samples.py`, per-set recipes and `NVRAM_CAMPAIGNS.md` | Isolated samples, screenshots, recipe and manifest per set |
| Diagnostic catalogue | SM2 `data/diagnostic-menus`, menu catalogue and audit | Observed per-set YAML, explicit unknown values and integrity status |
| Options review | SM2 Model 2 and Supermodel Model 3 review workbooks | Model 1 workbook with defaults, values, selection and clone coverage |
| Initial setup/overrides | SM2 initial templates, clone policy and catalogue audit | Validated complete templates; user-save precedence; opt-in reviewed overrides |
| Frontend Save States | SM2 `scripts/test-retroarch-savestate.py` and its isolated RetroArch runner | Model 1 runner, per-set Save/Load evidence and explicit visual comparison limits |
| Frontend gameplay smoke | SM2 `scripts/smoke-retroarch.py` replay, screenshot and audio capture | Isolated Model 1 input replay with captured race screen, PCM checks and separate physical-pad status |

## Current NVRAM adaptation

See [the frontend map](LIBRETRO_MODEL1_FRONTEND_MAP.md),
[`vr.yaml`](../data/diagnostic-menus/vr.yaml) and
[the review workbook](model1_core_options_review.xlsx).

All ten Model 1 sets have isolated boot samples and fresh-instance Save RAM
reload checks. These establish transport and bounded boot evidence, not complete
menu coverage or usable automatic presets. Wing War requires a later baseline:
EEPROM is uninitialized at frame 120 and populated by frame 600.

The capture tool uses direct Libretro ABI calls with deterministic frame steps,
symbolic RetroPad input and isolated output, adapting SM2's recipe concept.
This avoids host UI timing during byte discovery. Real RetroArch tests remain a
separate verification gate. The capture host explicitly disables automatic
initialization and NVRAM overrides, preserving native service-menu values even
with a core that implements those options.

All nine starting Model 1 sets have isolated, game-committed samples for every
catalogued discrete operator value: 110 fields and 953 values. Fresh-load
screens, native EEPROM integrity and per-field byte mappings are audited.
The four Wing War sets also have full-range, fresh-load calibration samples.
The YAML and review workbook cover these results.
This verifies acquisition through the direct ABI only. The reviewed menu work
is tracked in the Libretro roadmap.
Validate each clone independently before allowing parent template reuse.
SWA's Pilot/Gunner roles must preserve different P1/P2 mappings within the
established generic/semi-generic profile structure.

## Driving Steering Response procedure (2026-10-02)

Inspect SM2 `src/libretro/input.cpp` / `core_options.h` and Supermodel
`Src/OSD/Libretro/libretro.cpp` / `libretro_core_options.h` before adaptation.
Both currently use the same quadratic law and 256-entry FBNeo table before
output-range scaling. Retain names, choice keys, Linear default, live updates
and driving-profile visibility. Model 1's 20–80–E0 span needs normalization for
FBNeo and a 96-unit side span for Progressive. Keep native sampling/ramp state
unmodified; scale only the delivered inputs and refresh existing ADC mirrors.
The established bounded RetroArch runner now accepts `--steering-response`.

## Linked-cabinet host adaptation (2026-10-02)

Reuse SM2 `netpacket.cpp`, its per-game Linked Cabinets option convention and
multi-instance launcher procedure; inspect Supermodel's Netpacket/NetBoard as
well. Keep native game protocol/roles separate from frontend host/client IDs.
For Model 1 preserve the existing complete 453-byte M1COMM frames and its
COMM Save State restriction. Use native cold construction for linked resets,
retaining loaded ROMs and preserving NVRAM/audio preferences. Keep independent
frontend paths for every test instance. Record roster readiness separately from
sustained game-written COMM status, native role/ID/count and loss handling.
Reusable target runner: `tools/test_retroarch_linked.py`, using the existing
bounded `tools/test_retroarch_gpu.py`; procedures and evidence are documented
in `docs/LIBRETRO_MODEL1_LINKED_CABINETS.md`.

## Source publication procedure

Follow the reference's separation of source publication, build verification
and gameplay evidence. Inspect SM2 `CI.md` and its package checks before
publishing; retain this project's independent Git history and remotes.

1. Require explicit authorization for commit/push. Review the complete diff
   and the remote branch before staging only the intended project files.
   Before a release, verify the agreed four-component public core version
   (`UPSTREAM_MAJOR.MINOR.PATCH.PORT_REVISION`) in the runtime, `.info`, tag,
   release notes and package names. Cargo's three-component SemVer is separate.
2. Include the adapter, matching core metadata, English documentation, reviewed
   workbook, YAML, documentary screenshots, acquisition recipes and validated
   derived NVRAM tables. Keep ROMs, personal settings, raw Save RAM, build
   products, isolated test outputs and duplicate delivery files outside Git.
3. Run the existing workspace tests and NVRAM generator/inventory checks.
   Report ignored tests and keep actual frontend/gameplay evidence separate.
   A source publication does not complete the final build/distribution phase.
4. Write English commit messages, push to `origin` without rewriting history,
   and verify that the remote branch points to the local commit.
5. Update GitHub's English description/topics to match implemented features.
   Keep planned Tiny and Model 2 outputs distinct from current Model 1 support.


## Machine scope and Cargo features

Build a dedicated core separately from full/default-feature library or
standalone targets: Cargo unifies dependency features within one invocation.
Use `CARGO_NET_OFFLINE=true make -f Makefile.libretro` for Model 1. This is the
single release build recipe for local development, GitHub CI and Libretro
GitLab jobs. Windows uses GNU/MinGW throughout the public core build matrix.
Require `check_libretro_artifact.py --machine-scope model1` before installing
or packaging; it rejects a feature-unified combined artifact.

Derive selected catalogue records from the canonical upstream `roms_db.dat`
at build time; do not maintain independent copied catalogues. Keep stable
shared configuration/state types while excluding unused machine modules and
CPU dependencies. The standalone keeps both machine features by default.
Packaging license inventory must follow `cargo tree -p tgpulse-libretro`, not
workspace-wide metadata's unified feature graph. Compare pre-split M1 state,
Save RAM, software video and audio through the existing ABI host before
claiming runtime equivalence. See [build scope](LIBRETRO_BUILD_SCOPE.md).

## Timing panel composition and option order (2026-10-03)

Reference implementations rechecked: SM2 `src/libretro/core_options.h` and
`timing_overlay.cpp`; Supermodel `Src/OSD/Libretro/libretro_core_options.h` and
`libretroGui.cpp`. Both timing panels use ImGui background alpha 0.55 (55%
opacity, 45% transparency). SM2 composes over the complete software image;
Supermodel blends the panel over the rendered framebuffer.

The M1 hardware adapter previously blended against only the sparse 2D
foreground, then marked the result opaque. Its panel therefore hid 3D while
allowing 2D through. Preserve straight alpha in the foreground compositor and
blend intermediate alpha in the adapter's final GPU resolve. Native FE/FF tile
markers remain opaque. Keep this resolve adaptation inside Libretro; imported
standalone shaders remain unchanged. Vulkan and OpenGL/GLES share the adapted
resolve. Software still blends over the complete image.

Core Option categories retain SM2's System, Video, Audio, Input order. Within
System, SM2's repair/compatibility options precede automatic NVRAM, linked
cabinets, the NVRAM master switch and its fields. Video starts with the renderer
and sampling/image controls, then aspect/widescreen, cadence and timing panel.
Audio keeps Master followed by the four source Gains. Input keeps rumble,
steering response, then steering/accelerator/brake ranges. Supermodel confirms
sampling among image controls and response immediately before output ranges.
Stable sorting retains the existing field and set order within dynamic groups.
Modern and legacy registration use the same ordering; keys, defaults and
visibility rules are unchanged.

Verification: 48 adapter tests, including sparse-layer alpha versus complete
image composition, passed. SPIR-V and desktop OpenGL/GLES shader translation,
the native ABI/dependency/lifecycle gate and the macOS release build passed.
The existing isolated RetroArch Vulkan runner delivered a 240-frame VR image
and exited normally; the captured image visibly preserves the 3D scene beneath
the panel. This phase does not claim a new live OpenGL frontend run.

Development installation passed with matching SHA-256:
`d243163f8dc37f977470dfa39ed4ec5cb6b7d34d830325217b0fb8a58620d35b`.
Required reasoning: High. Shared account usage reads 0% after this phase
(rounded reading); reset: 2026-10-10 09:49:17 CEST.

## Widescreen Hack scope and defaults (2026-10-03)

User clarification: this feature applies only to games without native widescreen
support. Native support includes modes selected in the Service Menu. Exclude
confirmed native-capable sets from the hack regardless of a saved hack choice;
keep the native projection and existing Aspect Ratio/Auto cabinet interpretation.
Keep the hack visible for every set; exclude native-capable sets at runtime.

Rechecked the standalone `widescreen.rs` and `VideoSettings::of`, SM2 aspect
selection and Supermodel's expansion/background controls. The smallest adapter
change is a confirmed native-support predicate used for the runtime mode. Do not infer native support from related set names.
No emulated machine or standalone renderer changes are needed.

The user-facing label is **Widescreen Hack (Restart Required)**. Description:
“Applies only to games without native widescreen support.” It names no titles.
The ordered values are **Expand 3D View Only** (default), **Expand 3D View +
Stretch 2D**, and **Stretch 3D and 2D View**. Keep the existing stored keys and
values; existing selections remain valid. The missing-value fallback matches
the new default in modern and legacy registration. The hack requires hardware
rendering and widescreen presentation; 4:3 stays at native geometry.

Verification: 49 adapter tests pass, including default fallback and exclusion
of every hack mode for native-capable content. The native ABI, shader generation
and macOS release build pass. Two isolated 120-frame Vulkan launches with the
same 16:9/expansion request pass: native-capable content keeps 496x384 render
geometry, while non-native content expands to 683x384. Both deliver an image
and exit normally. These are rendering checks, separate from user gameplay
acceptance. Existing publication 0.1.0.2 is unchanged.

Development core installation and SHA-256 comparison passed:
`2340b6325e7fd02adc85ce2cac9570c059c35a70c648b18b1f0601702084a4c3`.
Required reasoning: High; shared-account usage after the phase: 1% (rounded).
Reset: 2026-10-10 09:49:17 CEST.

## Timing panel anchor with widescreen expansion (2026-10-03)

References rechecked: SM2 `build_timing_overlay` uses the internal framebuffer
size and an 8-pixel base margin; Supermodel `Libretro_DrawTimingOverlay` uses the
final display dimensions and an 8-pixel margin. The adapter previously drew
into the 496-pixel native foreground before its final wide-screen mapping.
3D-only expansion therefore centered the panel along with the native layer;
2D stretching also stretched the panel itself.

When the panel is enabled, map the game's foreground into its selected final
render geometry first, then draw the panel at (8, 8) in that geometry. Reuse a
scratch buffer, preserving each game's centered/stretched 2D pixels. Mark this
foreground as already mapped in the adapter resolve so Vulkan and OpenGL/GLES
do not center/stretch it a second time. Panel alpha still blends over the
resolved scene. Panel-disabled output follows the existing shader path;
software output and frontend-owned aspect stretching remain unchanged.
No upstream machine or standalone shader change is required.

Verification: 50 adapter tests passed, including native 2D boundary/margin
mapping and shader translation to SPIR-V and desktop OpenGL/GLES. The macOS
release build and native ABI/dependency/lifecycle gate passed. Isolated Vulkan
runs of all three Widescreen Hack choices delivered images and exited normally
at 180 frames; 3D-only expansion also used 2x supersampling. The captured panels
sit at the upper left in each case, while the game layer's selected mapping is
preserved. The complete-image stretch choice remains scaled by the frontend,
like the SM2 reference. No new live OpenGL run is claimed. This is rendering
verification, separate from gameplay/controller acceptance.

Development installation and SHA-256 comparison passed:
`679e0d92a285348cded0de2aefd2c42d9522c95c04bc8d744ffbbf5f7d43211c`.
Required reasoning: High; shared-account usage remains 1% (rounded).
Reset: 2026-10-10 09:49:17 CEST.

## Reader-Facing Labels And Review Workbook Conventions

Title Case is the default for UI labels, selector labels and document/workbook
headings. Preserve required acronyms (NVRAM, EEPROM, ID, CPU, GPU), numeric and
technical codes, explicitly agreed exceptions and literal documentary evidence.
Do not expose uppercase Service Menu text as an interface label by default.
Inspect the current reference's formatting function and complete delivery
conventions before adapting an analogous feature. Apply pertinent conventions
without requiring the user to enumerate them again.

For this correction, SM2 `retroarch_option_label` and its reference review
workbook, and Supermodel's reference review workbook were inspected directly.
Adapt the label formatter in `generate_model1_nvram.py`, preserving stored keys,
EEPROM encodings and automatic-startup policy. The workbook reader resolves
columns by header name and display names back to canonical catalogue fields.
It validates the Service Menu ordering and rejects ambiguous names.

The revised workbook retains all 110 acquired fields and four original sheets.
All four data regions are native named green Excel tables with filter buttons,
TableStyleMedium8 and alternate rows. Header and identifying columns are frozen;
column widths and row heights fit their contents. The 110 native defaults are
bold rich-text runs inside Observed Values; there is no separate Native Default
column. Existing selections remain intact apart from the requested Monitor
addition. Header case, setting/value labels and automatic-setup presentation
use Title Case. Selection cues and reviewer controls remain available.

Author tables and formatting with the bundled Artifact Tool runtime using
`tools/update_model1_review.mjs`. Its JSON input is the existing workbook's
`workbook_rows` result with authorized edits; input columns are retained by
header, except Native Default. Set CODEX_WORKSPACE_NODE_MODULES to the dependency
loader's bundled Node package directory. Export to a temporary workbook first.
The documented API has no rich-text authoring operation, so the narrow
`tools/bold_model1_review_defaults.py` pass adds native bold runs from catalogue
defaults without changing any cell values or other ZIP members. Validate and
render the final saved workbook before replacing the authoritative file.

For future updates, inspect native tableParts, autoFilter, table style, pane
boundaries and default rich-text runs; coloured cells alone do not establish a
filterable table. Keep the acquisition catalogue's exact source spelling and
all original sample evidence. Apply the reference's other relevant conventions
before creating or changing deliverables. The reusable campaign skill and
user preference notes now record this contract.

## Monitor Integration (2026-10-03)

The user approved Monitor in VR NVRAM Settings. Insert it after Country and
before Cabinet, matching Game System order. Values are 16:9 Wide (Default)
and 4:3 Normal; existing samples establish EEPROM byte 0x0a as 1 and 0.
The approved selection is now 40 fields across nine sets. Automatic setup
policy is unchanged. Aspect Ratio Auto follows the persisted monitor value;
Widescreen Hack remains excluded for native-capable content.

Verification: generator check and inventory audit pass for all 953 samples,
110 source fields and four separate calibrated ranges. All 50 adapter tests and
the nine-set NVRAM ABI runner pass, including initial templates, save precedence,
live overrides, visibility, reset/state preservation and disabled behavior.
Two isolated 120-frame RetroArch Vulkan runs select each monitor value. Both
exit normally, deliver an image and save the requested EEPROM byte with native
CRC integrity. Exported workbook checks confirm four native filterable tables,
110 exact default bold runs and retained source values/selections. Final views
of all four sheets were rendered and inspected. This evidence is distinct from
user gameplay/controller acceptance. Existing release 0.1.0.2 is unchanged.

The verified macOS release build was installed as the local Development core
with its matching info file. Installed/build SHA-256 comparison passed:
`842b40b2c9656b3ec1dad9a3eed5db24c8459a5515d0c6636243962ae0009e38`.
Required reasoning: High. Account usage at phase completion: 1%, with the
reported window resetting on 2026-10-10 at 09:49 CEST. Changes remain local.

## U1 Selective Machine Import

The 2026-10-04 phase follows current SM2 plan sections 6/7 for shared
state/device imports and Supermodel initial lightgun calibration/default
contracts. [U1 integration](LIBRETRO_U1_INTEGRATION.md) records provenance,
feature guards, state incompatibility, optional-resource adaptation and
reproducible existing-runner gates. Keep ROM/save evidence outside tracked
source; preserve both upstream histories and stop after the authorized phase.

## NetMerc MVD Input Adaptation

The approved virtual Calibrate/Recenter commands are host actions, separate from
native cabinet pins. Follow the standalone signal catalogue and pure MVD pose
geometry; exclude SDL ownership and any extra Libretro dead zone. Use full
Button labels, identical names for physical aliases and no redundant title
prefix inside the specific profile. NetMerc's profile is Special: Sega NetMerc.
Verify both native ADC endpoints and camera orientation signs; Libretro Y is
positive down while the standalone flight signal is positive up. Keep sensor
commands distinct from absolute stick look, and suppress already-held events
after successful reset/restore/device changes. Record delivery evidence in
[NetMerc Controls](LIBRETRO_U2_CONTROLS.md), with status only in the roadmap.

## General Core Option Visibility (2026-10-04)

General Core Options remain visible before content load, for every title and
after unload, except source Gains as explicitly requested on 2026-10-04.
State applicability in the description and enforce it at runtime.
Do not introduce other title-based display filtering without an explicit user request.
This includes driving ranges, MVD controls, rumble, video options
and the general NVRAM Settings toggle. Input descriptors still reflect native
actions and the selected profile. The existing reviewed per-set NVRAM fields
and per-set Linked Cabinets selectors retain their explicitly agreed filtering.

NetMerc City Workaround follows the standalone live machine API and Enabled
default, using the reference cores' Video-category correction conventions.
It remains visible for every title and applies only to NetMerc. This does not
change the separately selected DSP finite-arithmetic policy.

## Backup RAM Operator Settings (NetMerc, 2026-10-04)

Validate a complete initial sample against native Backup Data Clear, normal
menu exit and cold reload, including zero credits/session counters. Correct
operator encodings and a valid transport/bookkeeping checksum alone do not
establish an initialized baseline. Archive the native-clear sample and its
recipe, screenshot and reload evidence; retain historical variation evidence.
The generator must use that explicit initialized baseline. For an optional
known factory seed, initialize the credit word alongside its bookkeeping only
when no valid frontend save exists and the seed is actually needed. Never apply
this initialization repair during reset, state load or manual operator updates.

### Automatic Overrides Define Selector Defaults

For every title, an approved Automatic Initial NVRAM Setup override also
defines the default for the corresponding NVRAM Settings selector, its
`(Default)` label, legacy value order and missing-variable fallback. Apply this
rule in the generator using the reviewed startup policy; do not maintain a
separate list of country, cabinet or network exceptions. Fields without an
automatic override retain their native default. This follows SM2
`nvram_settings.cpp::initial_values` and Supermodel
`LibretroNvramSettings::GetDefaultValue`, which share defaults between initial
setup and operator selectors.

Keep the observed native defaults in the diagnostic catalogue and workbook
unchanged: those document hardware behavior, not the chosen core policy.
Existing frontend selections remain explicit overrides; a new core default
does not migrate personal options or saved NVRAM.

Reuse the reviewed Field/Value option pipeline and native cache-reset behavior.
Route field patches to the storage identified by independently committed/reloaded
samples; an erased EEPROM must not reject initialized backup RAM. Keep complete
backup templates separate from EEPROM integrity policy, while sharing the RLE
decoder and transport validation. Detect an optional seed from archive presence,
not image length: the loader can synthesize an equally sized fallback. Preserve
loaded seed bytes outside approved automatic fields and existing-save precedence.
Verify selector/dependent bytes and preservation through ABI and actual frontend
save readback before deploying Development. See `LIBRETRO_U8_NETMERC_SETTINGS.md`.

## U5 Sensor Adaptation — 2026-10-04

Rechecked SM2 phases 3.1–3.2/3.7, Supermodel roadmap conventions and the
reviewed TGPulse-Next neutral motion/MVD implementation. Neither Libretro
reference supplies an MVD implementation. The smallest adaptation imports
the pure estimator and connects it to frontend-owned P1 sensors, with
explicit Libretro units/axis normalization and monotonic polling time.
The user-requested three-second window retains upstream calibration
thresholds; the helper's standalone default remains five seconds.

71 adapter tests, real-time mock sensor ABI/queued-notification gates,
ten-set profile/control checks, nine-set prior-binary regression, the native
artifact gate and bounded RetroArch Vulkan fallback pass. Development core
and info installed with SHA-256 matching the build. Details, reproducible
runners and ABI limitations are recorded in `LIBRETRO_U5_SENSORS.md`.
Physical sensor acceptance remains user-run. The sole roadmap marks U5
complete and keeps U6 next; the final conformity audit remains deferred.

## U6 Rumble Adaptation — 2026-10-04

Rechecked standalone motor conversion/output dispatch and SM2/Supermodel
rumble callbacks/lifecycle. The user approved frontend-owned intensity,
following SM2 Input Rumble Gain, replacing the originally planned core gain.
Reuse the upstream binary levels and final-latch sampling. On user request,
within-frame pulse observation is no longer connected to the Libretro rumble
output; keep the neutral helper as optional B5 only. No synthetic button/LCD effects or rumble OSD.

Three native motor checks, 71 adapter checks, positive synthetic firmware
pulse-to-callback delivery, ABI lifecycle/state gates, nine-set regression,
artifact checks, bounded RetroArch Vulkan and verified Development deployment
pass. The bounded native gameplay session produced no positive motor events;
physical response remains user-run. Record that limit in `LIBRETRO_U6_RUMBLE.md`.
Backport candidate B5 records the optional neutral pulse helper. The
final-latch follow-up uses the targeted synthetic sampling test and existing
ABI lifecycle runner, followed by verified Development installation. U7 is complete;
final conformity audit remains deferred.

## U7 Diagnostic LCD Delivery — 2026-10-04

The user-approved HD44780-only display reuses the native dot/blink/state APIs
and the existing timing/GPU foreground compositor. Keep OFF / ON (100%) /
ON (50%) in the existing Display selector, plus corner and background opacity as global always-visible Video options; no Text selector.
Draw Timing over LCD at overlapping corners without automatic repositioning.
Preserve `overlay` as full size and add `overlay_half` for half size; no separate
scale option. Size/SHA-1 validation and ordered System/adjacent/game lookup apply to LCD and
exact game-selected I/O firmware through `loader/model1_bios.rs`. Missing LCD
BIOS notifies once per activation only while the display is enabled; font
absence never prevents game startup. Required I/O firmware still does.

Reuse `tools/test_libretro_netmerc_lcd.py` for isolated resource, notification
and state checks; use the existing RetroArch/EGL runners' diagnostic flags for
visible output. See [U7 delivery](LIBRETRO_U7_LCD.md) for actual renderer/platform
coverage, cache synchronization and evidence limits. Register neutral loader
reuse as B6. U7 ends after verified Development installation; U9 is separate.

## NetMerc Startup Pose Investigation

Reuse `tools/trace_netmerc_camera.py` with existing port/standalone release
libraries to compare pose-write instruction, stack/register and serial readiness
traces. The initial constructor DPRAM seed is overwritten by I/O firmware
publication before a valid measurement; MAME's default jumper path disables
that publication. See `LIBRETRO_U5_SENSORS.md` for exact ROM-qualified PCs,
reproduction boundaries and the difference between machine traces and visible
frontend/gameplay acceptance. The trace itself does not apply a workaround.
The subsequently approved startup correction guards native pose publication
until a complete station-1 record has been received and decoded. Reuse this
runner's `--verify-bootstrap` mode for neutral startup, distinct/zero live poses
and full-machine replay. Keep the firmware-qualified receive/commit validation
in the native board, with serialized readiness and explicit format-5 state
rejection; do not move it into frontend input selection or add DIP options.
Focused native/ABI checks and the required Development install passed; visual
controller trials remain user-run. See the U5 correction record and B7.

## Adjustable MVD Drift Compensation

Reuse the neutral standalone orientation helper and its explicit timestamps.
The user approved live Off / 10–100% and subsequently an enabled moderate
50% default. Keep modern/legacy defaults and parser fallback aligned, expose
the option globally in Input, and retain frontend-owned sensors. Inspect
Fusion's stationary-bias filter and GamepadMotionHelpers calibration settings
before changing estimator parameters. Record sourced values separately from
local gates/default tuning; percentages scale learning speed. Preserve current
orientation without auto-recentering, and test drift reduction together with
intended slow movement, acceleration, live option changes and lifecycle reset.
Use existing sensor ABI/artifact/install runners; physical controller response
remains user-run. See U5 and backport candidate B8.


## Optional MVD Sensor Evidence

For real-controller tuning, reuse the existing P1 sensor callback path and
`tools/test_libretro_netmerc_sensors.py`. Keep raw callbacks, converted units,
calibration statistics, learned bias and actual integrated rates distinct.
Use the frontend Save Directory for unique, bounded, buffered CSV segments;
show start/close paths through queued OSD. Disabled recording must be silent
and create no files. Logging failure must preserve tracking. Keep recording
outside NVRAM and machine state. Document callback-time/controller-identity
limitations, and distinguish mock API verification from physical measurements.
See the U5 recording procedure before requesting a real-controller sample.

## Core Identity And Filesystem Paths

The public Model 1 `library_name` and `.info` `corename` are both
`TGPulse-Next-M1` (latest user naming update: 2026-10-06). Keep the runtime,
metadata, package gate and submission metadata aligned. Avoid filesystem-reserved
characters in runtime identities. Future Model 2 builds use the distinct
`TGPulse-Next-M2` identity to avoid shared configuration paths and
ambiguous Netplay discovery. Preserve the public binary basename and Core Label.
When changing identities, preserve existing core-named settings/save directories
and copy their files to the new name without overwriting destination files.

## Automatic Network Preset Policy

Reuse the current linked-cabinet set registry and reviewed NVRAM values. Keep
Automatic Network Settings immediately after Linked Cabinets in modern and
legacy menus. VR/VFormula default to Red (Master), provide unique Slave colors
and Live (with default Red); Wing War variants default to Master. Disabled never writes operator
fields, including on Linked Cabinets OFF. Hide automation and make no network
operator writes while the NVRAM Settings master switch is disabled. When it is
enabled, active automation sets and maintains its managed NVRAM Settings
selectors, including on startup and Linked Cabinets changes; with Linked
Cabinets OFF these receive reviewed offline defaults. Manual selectors become
authoritative when automation is Disabled. Preserve unrelated fields and
checksum/mirror policy. Apply the preset's managed selector values through the
NVRAM Settings path before the next emulated frame, then synchronize displayed
Core Options via SET_VARIABLE only after releasing the core lock. Keep native
roles independent of frontend host/client.
Reconsider the preset and selectors on option changes, not every frame.
Returning from Core Options or using RetroArch Restart rereads Linked Cabinets,
rebuilds the machine only for a topology change and retains open Netpacket
callbacks while refreshing its handshake. Fit COMM before applying the
authoritative operator role and before the next emulated frame. Keep Save RAM
and operator role persistence across either transition. Treat a temporary
packet-count difference while peers resume at different times as pending
setup; retain the game/set identity check.
