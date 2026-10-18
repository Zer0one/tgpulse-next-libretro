# Network Board And Linked Cabinets Feasibility

Date: 2026-10-14. Scope: isolated feasibility only. Production sources, built
core and installed RetroArch core are unchanged. No deployment, commit or push.

## References And Contract

Inspected Libretro Core Framework edition 0.1.18, F14; the corresponding
Supermodel roadmap section and `Docs/NETWORK_LOOPBACK_FEASIBILITY_2026-10-13.md`;
SM2's current Automatic Network Settings reference alignment and
`src/hw/m2comm.cpp`. SM2 creates `LoopbackTransport` initially and restores it
through `set_transport(nullptr)`. This is a transport reference, not copied
source or evidence of Model 1 compatibility.

The canonical workbook's F13/F121 comments were read and applied only in the
prototype. Workbook SHA-256:
`d691cc31ebb32b1e666c962629f83357370e80b002bb5acc5de32242e1f09f91`.
The workbook and shared Framework files were not edited.

| Board | Linked Cabinets | Contract |
| --- | --- | --- |
| Disabled | Saved Selection Inactive | Board absent; hide Linked Cabinets and ANS without changing their selections. Active ANS prepares native offline settings under the NVRAM Settings gate. |
| Enabled | Disabled | Board present; local transport loopback, no external transport calls. Active ANS prepares native No Link/Standalone regardless of the retained role. |
| Enabled | Supported Total >= 2 | Existing native linked transport and selected supported ANS role. |

NVRAM Settings Disabled does not apply automatic presets. ANS Disabled retains
manual authority. Board/topology changes apply through automatic reconfiguration
on Resume, not a mandatory manual content reload. A native initiating role used
to probe device echo is not the offline-game configuration required by ANS.

## Isolated Adaptation

Prototype: `/private/tmp/tgpulse-network-feasibility-20261014`.
Current working sources were copied; build caches were copied using APFS clones,
without sharing writable build files. Changes are limited to four adapter files:
`lib.rs`, `netpacket.rs`, `automatic_network.rs`, `menu_wording.rs`.
The emulated CommBoard, CPUs, ROMs, renderer and standalone sources are unchanged.

The exploratory Network Board selector controls the existing Single/Twin board
attachment boundary. Its prototype Enabled default is not a proposed published
default. All existing choices, value labels, defaults and categories were
compared with the production registration and remain identical.

For an internal total of one, the adapter marks the local transport connected
and returns complete transmitted frames through `receive`, respecting native
RX capacity. It does not tick the board, fabricate handshake state, send frontend
packets or call frontend network polling. Totals >= 2 retain the existing path.
Board presence gates visibility and ANS linked eligibility. Reset construction
now follows actual board presence rather than total >= 2.

## Results

- Six Rust Netpacket tests passed: the existing session/ring/burst cases plus
  an explicit one-node device probe. The probe completes native Master handshake,
  ID/count 1/1 and local payload publication with frontend client ID 5 and
  forbidden frontend send/poll callbacks. This is device evidence only.
- Two existing ANS Rust tests passed, covering canonical/legacy tokens and all
  supported presets across six sets, native integrity and offline restoration.
- ABI trials passed for `vr`, `vformula`, `wingwar`, `wingwaru`, `wingwarj`,
  `wingwar360`: present board with local topology; offline native role despite
  retained ANS 2; absent-board visibility; preserved linked total/ANS selections;
  automatic Resume removal/restoration; restored linked Slave role; return to
  offline defaults; NVRAM Settings Disabled preserves operator EEPROM and does
  not publish the automatic role. Save RAM container integrity passes.
- Full registration comparison preserves all existing values/defaults/categories.
  Only the exploratory board selector is added. Help for ANS/Linked Cabinets
  follows the approved workbook, in the prototype only.

| Actual RetroArch Trial | Frames | Exit | Inspected Final Image |
| --- | --- | --- | --- |
| VR, Board Enabled, Linked Disabled, Retained ANS 2 | 1,200 | 0 | Attract demo / Course Ranking / Credit 0 |
| Virtua Formula, Same Configuration | 1,200 | 0 | Attract demo / Course Ranking / Credit 0 |
| Wing War, Same Configuration | 1,200 | 0 | Rendered attract demo / Credit 0 |

Trials used RetroArch 1.22.2 Nightly on macOS ARM64, software core rendering
with the GL frontend driver. Each reused `test_retroarch_gpu.py` with its own
configuration, saves, screenshots and owned bounded process. The copied runner
adds only the exploratory board argument and explicit global core-options mode.
All three retained ANS 2 values remain in their isolated frontend option files.
No other frontend was controlled or closed. Each owned process exited normally.

## Findings And Limits

The initial prototype exposed a lifecycle dependency: loading a present board
with one-node topology selected the Single save-state reset path and failed.
Changing reset selection to board presence fixed startup without changing the
native board or save-state format. Save States remain conservatively unavailable
whenever the board is present, including local mode; qualifying local-board
serialization would be separate work.

The first sandboxed frontend attempt aborted before logging; the normal
authorized desktop launch then exposed the lifecycle failure above. Excluded
attempts are retained in the temporary prototype. A later ABI assertion was
corrected to accept a selector's default when no publication was necessary.
Successful results above refer to corrected trials only.

Feasibility is positive for the mechanism and offline startup in both Model 1
network families. Normal NoLink startup may leave CN inactive; attract-mode
success does not prove active board echo. That capability has its separate probe.
Visibility is verified through actual ABI display callbacks, not a new visual
menu inspection. Resume behavior is ABI-tested, not an interactive paused-menu
trial. No new external multiplayer, complete race, physical controller,
Windows/Linux, clone-attract or distributed-state acceptance is claimed.

Existing topology boundaries are retained: Wing War family has two playable
nodes; VR/Virtua Formula allow up to eight playable identities and one optional
LIVE relay, with nine total requiring a relay. Existing native ring tests cover
Master/Slave and Master/Slave/Relay; this study does not requalify all selectable
totals or introduce a new topology limit/default.

Production adoption needs separate authorization and review of the new selector's
default, legacy registration, lifecycle/visibility return signaling, board-present
state policy and complete topology validation. No automatic deployment is implied
by this prototype build; the user explicitly excluded production delivery here.

## Retained Evidence

`outputs/network-loopback-feasibility-20261014/` (ignored generated evidence)
contains the four-file source patch, reference fingerprint, ABI probe/results,
device/ANS logs, registration snapshots, production-preservation manifest and
frontend logs/result JSON/screenshots. The prototype retains build caches and
the copied runner. Reproduce from current sources plus `prototype.patch`; use
the existing Cargo tests, `abi-probe.py` and recorded frontend `command.json`.
No ROMs or personal settings are included in the repository evidence bundle.

SHA-256 checks confirm 234 source/core files unchanged, including production
and installed dylibs. Account usage moved from 52% to 53% during the study;
the shared CAP is 55%. Additional optional trials were stopped with margin.
