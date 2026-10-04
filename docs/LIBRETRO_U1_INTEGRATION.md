# U1 Machine Integration — 2026-10-04

## Source And Adaptation

Selective import of TGPulse-Next commit
`27db9fa58aa631517dd9f7538a4d99efca2e0214`, relative to the initial
`3c75f9b2b155e8bb50a225c69520948ef72ecd48` snapshot. The reviewed upstream
release is `0.1.0.1` / `f303b712455571b60cbea2249e1022935d398e22`.
No upstream history was merged into this independent repository.

The 27-file machine delta adds advanced NetMerc I/O firmware identification,
serial tracking peer, cold-boot pose, LCD hardware state, procedural sound,
resource identity and complete snapshots. Twenty-four files match the source
commit byte for byte. The loader, ROM database parser and sound module retain
local machine feature guards, strict identification, scoped catalogue,
315-5711 repair policy and existing adapter-facing audio helpers. A standalone
fixture adds the new ROM flag. The MAME LCD notice follows the upstream delta.

Finite arithmetic is a shared MB86233 implementation. Activation remains
conditional on `Kind::NetMerc`, as explicitly agreed; other boards retain IEEE.
The City Workaround API and upstream default are imported with the DSP, but
its Core Option belongs to U1a. Desktop bindings, SDL devices, auxiliary windows,
MVD frontend policies and donor selection are outside U1.

## Save State Contract

U1 introduced complete machine format **4**, and sound state **2**. The approved
[startup publication correction](LIBRETRO_U5_SENSORS.md#approved-startup-publication-correction--2026-10-04)
subsequently moves the current machine envelope to **5**, without migration.
Old machine
format-1 states are rejected without migration and without altering the running
machine. This applies to existing Model 1 games as well as NetMerc. The fixed
Libretro buffer and outer wrapper remain unchanged. Save RAM format 1 remains
compatible; user calibration and operator settings are not converted or erased.

## Optional NetMerc Calibration Seed

At the user's request, `netmerc_nvram.bin` is optional. It supplies controller
calibration data, not executable firmware. `epr-18021.6` and the other declared
executable/resource chips remain required for strict content identification.
The canonical upstream database and generator retain their original record;
the optional-resource exception is narrow and explicit in the parser/loader.
No ROM archive is changed.

With the seed absent, the loader creates blank FF SRAM with empty valid
bookkeeping (selector 0F, both banks and checksum zero). It supplies no axis
calibration. In Libretro the complete supplied seed remains unmodified during
ROM loading. Only after persistent-save import has found no valid saved image
does the adapter invoke the SHA-1-guarded bookkeeping and credit initialization.
Calibration is preserved. Standalone loading retains its existing behavior.
Missing optional seeds generate no user-facing notification.

`Automatic Initial NVRAM Setup`, default Enabled, supplies normalized controller
endpoints only when no valid frontend save exists. This follows Supermodel's
`apply_initial_nvram_settings()` / `normalizeAnalogInputs` lightgun policy:
initialization and calibration belong to the frontend, and personal saves win.
No new calibration menu or input binding is added.

| Native Service Item | SRAM Offset | Initial Value |
| --- | --- | --- |
| Controller Left Min | 0x003C | FF |
| Controller Right Max | 0x0038 | 00 |
| Controller Up Max | 0x0040 | 00 |
| Controller Down Min | 0x0044 | FF |

These are the low bytes of four-byte slots whose remaining bytes are FF. They
are hexadecimal ADC endpoints, not angles or percentages. The mapping is
supported by the standalone Service Menu/ROM investigation and the sparse
factory image; file-free automatic initialization is byte-identical to the
initialized factory seed. Existing personal saves, reset restoration and state
imports retain their calibration. Disabled keeps blank calibration when the
optional file is absent. Controller calibration is separate from MVD tracking
calibration/recentering planned in U2/U5.

This narrow, explicitly approved calibration policy does not select other
NetMerc operator settings. Complete acquisition and workbook review remain
U3/U8; the nine-set workbook and generated tables are unchanged by U1.

## Verification And Limits

Evidence is isolated under `/private/tmp/tgpulse-u1`; raw ROMs, states,
Save RAM and build products are not committed.

- Final offline workspace: **572 passed, 5 ignored**, zero failures.
- Model 1-only library: **235 passed, 1 ignored**; Model 2-only: **100 passed**.
- Standalone library/executable check passes; the standalone fixture is updated.
- Dedicated macOS ARM64 release build passes. Artifact checks confirm Model 1
  scope, 25 ABI exports, dependencies and three empty lifecycle cycles.
- Nine existing sets run 600 frames against the saved pre-U1 Development core:
  video/audio hashes, frame/sample counts and Save RAM match exactly. Old states
  are rejected atomically; format-4 continuation is deterministic.
- The existing NVRAM runner passes all 40 reviewed fields across nine sets and
  verifies optional-file NetMerc calibration, factory equivalence, Disabled,
  personal-save precedence, reset preservation and hidden unreviewed menus.
- NetMerc format-4 Save/Load replays 60 frames with identical video/audio and
  complete resulting state after a 2,400-frame boot through the direct ABI.
- RetroArch 1.22.2 (`9be7ec92`), Vulkan/MoltenVK, boots the seed-free media ZIP
  to the **SEGA NETMERC title screen** after 2,400 frames, exit 0. This establishes
  startup, not completed gameplay or acceptance of later MVD/rumble/LCD work.
- Development core and matching `.info` are installed; built and installed
  SHA-256 match `67b6edfaa5d0f6570dff40b8c252ce44e23a3cd325479562f23b09fd392b7fce`.

Frontend state evidence and runner limitations are recorded below separately
from deterministic ABI replay. Cross-platform CI/publication belongs to U9;
U1 has not changed the published version, committed, pushed or created a release.

## Reusable Procedure

Follow SM2 `PORTING_PLAN.md` sections 6/7 and Supermodel `Docs/ROADMAP.md`
state/default contracts. Compare the source delta first, apply each shared file
against its imported ancestor, retain feature guards/local policies, check both
machine configurations and the standalone, then build the M1 core separately
from the feature-unifying workspace. Keep the old core for reproducible frame,
audio, Save RAM and compatibility comparison. Run the existing NVRAM and
RetroArch runners; install through `tools/install_dev_core.py` and verify hashes.

The additional calibration gate is reusable through
`tools/test_libretro_nvram_settings.py --netmerc-seed-rom PATH`: use a ROM
folder whose NetMerc ZIP omits the seed and an existing comparison ZIP containing
it. The script reads both archives without altering them. The scope runner's
`--state-policy format-1-to-4` explicitly verifies this approved incompatibility;
its default `identical` policy preserves earlier component-split checks.

## Frontend Save State Evidence

The isolated Software run `netmerc-state-confirmed` uses the installed core,
RetroArch 1.22.2 and the seed-free media ZIP. It writes a 67,108,920-byte
RetroArch state, confirms and executes Reset twice, advances the reboot, and
loads the saved state twice. Both reboot images differ from the saved image;
the restored title region matches the saved region exactly on both loads.
The frontend resumes and exits 0 without forced termination; its histories,
options, states and saves are isolated.

This proof compares the center 40% of image width, from 40% through 80% of
image height. It excludes the native blinking `Fire To Start` prompt: the
asynchronous frontend load and redraw advance its blink phase. It establishes
visible title restoration, not an exact whole-frame frontend replay or active
NetMerc gameplay. Full frame/audio/state replay is separately verified by ABI.

Initial runner attempts were inadequate: a static title did not naturally
change, real-time VR restores advanced different frame counts, and NetMerc
Reset confirmation widgets overlapped the comparison. The final capture disables
frontend widgets/fonts in its isolated config and confirms the two-press Reset
policy. Its original default-crop result still fails because of the native
prompt; retained raw images were then reanalyzed with the documented region.
`netmerc-state-confirmed/region-verification.json` records that successful
comparison separately, preserving the original result rather than replacing it.
The saved and both restored regions have SHA-256
`1ab22fa2097d47dd40b6154cf0988c9f190d36e91c801032c1df77e2fe4acb68`;
both reboot regions have
`6bdfab6badede213506da28e551e2be3658ee1c957dc0e7747b2062a60743ecf`.

The existing runner now supports reproducing this bounded proof with
`--reset-before-advance --boot-wait 50 --advance-wait 0.1 --crop-top 0.4
--crop-height 0.4`. Its default comparison region remains unchanged. Do not
use this title-screen result to claim completion of U2 controls or the U3
operator-setting campaign.
