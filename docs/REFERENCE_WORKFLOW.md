# Reference adaptation workflow

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
   `tgpulse_next_dev_m1_libretro` RetroArch installation and SHA-256 comparison.
   Record the reference, adaptation, verification and remaining gaps. A phase
   with a failed or omitted required delivery step is incomplete.
6. Report required model/reasoning level before substantial work and account
   usage/reset after completed verification. Commit/push require explicit approval.

Project documents, YAML explanations and workbook labels are written in English.
Conversation can remain in Italian.
Maintain implementation status only in [the Libretro roadmap](LIBRETRO_ROADMAP.md);
keep this file procedural and other documents as evidence or design references.

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

## Audio option contract

Master Volume uses default 100%, standalone 0–800% range and reference-core
10% steps. At the user's request, zero is labelled OFF. No Mute/Muted label or
Auto value is added to Master.
Per-source Gain uses the separately agreed Mute/Auto/0–100% selector. Order the
modern numeric selector ascending and use identical load-time/live parsing.
Verify live PCM changes from the same restored machine state before claiming
progressive gain; game progression must not alter the reference segment.

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
