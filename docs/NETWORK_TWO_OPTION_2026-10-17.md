# Two-Option Network Convention Integration

## Scope And Authoritative Review

User-authorized integration of Framework 0.1.21 F14, completed locally on
2026-10-17 without commit, push or release. This supersedes the earlier
Network Board/ANS contracts for current behavior; dated reports remain history.

Read the corresponding current sections of SM2 `PORTING_PLAN.md`, Supermodel
`Docs/ROADMAP.md`, the Framework and its single-sheet Network Review workbook.
Reconciled every Model 1 row, Network Review rows 6–15: six COMM sets and four
unsupported sets. No prototype catalogue or other machine family was added.
Workbook unchanged; SHA-256:
`a41a07028027f1e84422cdee7c6e597ef89cc424b96432a45a646cd051de1a59`.
The Framework's live F14 text includes the user's subsequent hidden-selector
synchronization clarification. Shared Framework edits remain owned by that chat.

## Delivered Controls

### Local Startup Configuration Follow-Up

The user's subsequent CANCELLED/striped-screen report occurred on initial VR
startup, without menu edits or an active lobby. The actual per-core options
contained Linked Cabinets `-1` and ANRS `2` (Orange/Slave), with general NVRAM
Settings enabled. This deliberately independent combination requests a native
Slave while COMM is absent. It is not evidence of a lost lobby during reset.
After the user selected local operation, backed up the personal options file
and changed only ANRS VR to `0` (No Link). No code/contract change was made.
The installed core booted for 900 frames in isolated macOS RetroArch using a
copy of the user's saved NVRAM; the final screenshot shows the normal course
selection/demo screen, and the synchronized native Link ID selector is No Link.
Evidence: `outputs/network-resume-fix-20261017/local-no-link/`. This proves the
selected local-startup case, not every possible independent role/topology pair.

### Resume Update Signal Correction

The first delivery missed real paused-menu application despite passing its ABI
matrix. The user reported that ANRS/NVRAM changes did not reset. Reproduced in
RetroArch ARM Nightly 1.22.2 RGUI: No Link to Red (Master) followed by Resume
continued the existing scene without a reset. The mock incorrectly preserved the
update flag across GET_VARIABLE reads.

Framework F05 already documents this RetroArch behavior. Rechecked Supermodel
`Src/OSD/Libretro/libretro.cpp` (paused visibility callback and retro_run) and
SM2 `src/libretro/core.cpp` (update-before-read ordering). The smallest adapter
correction retains GET_VARIABLE_UPDATE before profile/visibility reads, merges
paused callbacks into one pending flag and consumes it on Resume. New environments
clear retained state; explicit Restart also preserves pending option edits.
Native hardware and the existing effective-change/reset coalescing are unchanged.

The revised existing ABI matrix now consumes the flag on GET_VARIABLE, repeats
paused display callbacks and includes manual application without that callback.
It fails on the previous binary and passes for all ten sets on the corrected
binary. 93 adapter tests and artifact checks also pass. Real RGUI acceptance:

- ANRS No Link to Red (Master): one automatic reset and visible OSD. With COMM
  intentionally disconnected this native role reports Cancelled; no role is
  silently converted to No Link.
- Return to No Link: one reset, native No Link visible, attract mode resumes.
- Enable NVRAM Settings and change Difficulty to Easy: one reset, OSD and native
  Difficulty Easy visible, followed by attract mode.
- Pause/Resume without edits: no additional reset (three total at this point).
- Change Linked Cabinets to 2 and ANRS to Red (Master) together: one topology
  reset (four total), native Master and Network Checking visible. No second
  cabinet was launched for this focused regression check.

Only the owned isolated session was closed. User settings/saves were untouched.
Evidence: `outputs/network-resume-fix-20261017/`, including before/after frontend
logs, failing-before/passing-after ABI logs, tests and installation manifest.
Screenshots were observed through CUA but were not saved as image files.

Follow-up on the user's active-lobby workflow: reran the existing
`test_libretro_linked_restart.py` against the installed corrected core, now
modeling consuming GET_VARIABLE reads. It passes Restart totals 1/2/3/1 and
Resume-time COMM fitting/removal, confirms native Slave/identity before the
first frame, preserves the already-open lobby callbacks and recovers a staggered
join. Evidence: `outputs/network-resume-fix-20261017/lobby-regression/`.
This is focused ABI/transport regression evidence, not a new two-frontend race.

This correction supersedes the first installed binary below. The public core
was reinstalled with backup and matching SHA-256
`3e3191b9e1114d1d29b0fe5e9dc10a193f6a638d6a871379b027cdad6302f976`;
metadata hash is unchanged. No commit/push. Account usage was 65% before the
correction and 66% after it (shared account, rounded display).

Exactly two network controls are visible for the current game: Linked Cabinets
and Auto Network Role Settings (ANRS). Per-set stored keys retain independent
choices. Only the current set's pair is shown; an unsupported pair displays
Not Supported for VF, SWA, SWAJ, NetMerc and when no game is loaded. The separate
Network Board registration and gating are removed from modern and legacy paths.

| Control | Values / Default | Effective Meaning |
| --- | --- | --- |
| Linked Cabinets | -1 Disabled/default; explicit 0/1 aliases; 2–9 VR/Formula; 2 Wing War | Normally disconnect COMM while disabled, as specified for all six Model 1 sets in the workbook. Fit COMM for linked totals. Never rewrite role because topology changed. |
| ANRS | -1 Disabled/manual; 0 No Link or Stand Alone/default; numeric playable roles; r Live with l/s aliases where supported | Apply native role/identity regardless of topology or the NVRAM Settings switch. |

VR uses the workbook's Red, Orange, Skyblue, Pink, Black, Green, Yellow, Blue
identity order. Formula instead uses No.1 Red, No.2 Blue, No.3 Yellow, No.4 Green,
No.5 Black, No.6 Pink, No.7 Skyblue, No.8 Orange. Its existing EEPROM field table
is arranged by color encoding, so Formula's number is resolved against the native
No.n key rather than treated as a table index. Both provide Live. Wing War and
its US/Japan/R360 variants provide Stand Alone, Master and Slave.

Nine total VR/Formula cabinets require a Live relay and at most eight playable
identities. This study does not qualify every possible total/role combination.
Valid but incompatible manual/topology choices retain native failure behavior;
in particular disconnected Slave is not silently converted to No Link.

Previous textual network values, including disabled, red, master and relay, are
not accepted as compatibility aliases. Observed invalid values fall back to the
new default with notification. Required aliases 0/1 and l/s are registered and
parsed explicitly; legacy frontends receive the same raw choices/defaults.

## Native Settings Authority And Resume

ANRS overrides only native role and identity, preserving unrelated fields and
their integrity policy. It works with NVRAM Settings disabled. Its managed native
selectors are hidden while automation is active and synchronized with the values
actually applied, including while hidden. Publication uses the established
frontend callback path after releasing CORE, avoiding re-entry deadlock.

When ANRS becomes Disabled, the fields return with their last applied values;
earlier manual selections are not restored. NVRAM Settings regains authority
when enabled. With both disabled, the native service-menu/save remains under
the game's management. All other manual NVRAM fields retain their usual gate.

Effective topology and NVRAM changes coalesce before native execution on Resume.
A newly constructed/reset machine is marked fresh so applying its pending NVRAM
does not reset it again. Related changes produce one automatic reset; unchanged
Resume, authority handover without a native value change and Disabled aliases
produce none. Existing Netpacket lobby callbacks are retained.

Linked Disabled disconnects COMM for Model 1 and restores Save State availability.
Linked totals fit COMM and keep the native board-present Save State restriction.
Native simulation, wire format, game ROMs and standalone sources are unchanged.

## Verification

- 93 adapter tests pass, including approved-token parsing, removal of old
  aliases, native Formula number mapping and all-preset integrity round-trips.
- Every Model 1 workbook row matches registered native labels, canonical values
  and defaults. Not Supported is checked for each unsupported game. Existing
  non-network controls keep their earlier behavior and compatibility policy.
- The ten-game ABI runner checks exactly two visible controls, absent obsolete
  Network Board, ANRS with NVRAM Settings off, hidden-field synchronization,
  last-applied handover, manual control, unrelated Difficulty preservation,
  topology/role independence, required aliases and invalid-value notifications.
  All six COMM sets pass actual Save RAM reload plus container and native EEPROM
  checksum/mirror checks. Selector publication includes synchronous re-entry.
- Reset logs verify one reset for each of six combined topology/role changes;
  no reset for unchanged Resume, unchanged handover or Disabled aliases.
- Existing lobby/Restart runner passes fitting/removing COMM, totals 1/2/3/1,
  role application before the first native frame, retained callbacks and
  independent role preservation while disconnecting the board.
- Existing LCF profile/paused-callback/legacy-device checks remain passing after
  updating only superseded network registration expectations.
- Actual macOS RetroArch VR pair: both instances execute 2,400 frames, exit 0,
  establish the roster and maintain game-created COMM online for 600 consecutive
  frames. Observed native Master role/ID/count 1/1/2, Slave 2/2/2. Saved EEPROM
  role/color bytes are Master 1/0 and Slave 2/7; native/container CRCs pass.
- macOS release and artifact checks pass: Model 1 scope, 25 Libretro exports,
  dependency checks and three empty lifecycle cycles. `git diff --check` passes.

The linked trial reused `test_retroarch_linked.py` and `test_retroarch_gpu.py`
with isolated configuration, saves and owned processes. GL frontend selection
was added to the existing runner; no new launcher or dependency was introduced.
No frontend belonging to another chat was controlled or closed.

These checks do not establish a synchronized race, physical controller response,
all topology combinations or fresh Linux/Windows behavior. Modern visibility is
checked through the frontend display/update callbacks; no new interactive menu
inspection or legacy-frontend runtime is claimed. Legacy APIs expose raw value
codes rather than V2's per-value display labels.

## Local Delivery And Framework Reporting

Installed public core `tgpulse_next_m1_libretro.dylib` and matching `.info` using
the existing installer, with previous files backed up. Build/installed SHA-256:
`8413d680c885e17788296eb3167f4ff5ce4e52c518fb1fb316e2230134b81d5a`.
Metadata SHA-256:
`e7ddd4c8355b22f2c6c2810c27443eae9b0b29a2984cc2f67ddd2912ba5cab00`.
Runtime name and metadata identity remain public and unchanged. No configuration
migration or Git publication was performed.

The user authorized meaningful progress and delivery reports to Framework chat
`01a13152-dbfa-7e30-bc9d-0477abc2cccc`. Progress, the Formula mapping finding and
the hidden-selector clarification were reported there; completion includes this
report and the installed hash. Framework publication is not inferred from sending
a message. The sole roadmap and family/upstream registers are updated locally.

Generated evidence: `outputs/network-two-option-20261017/` contains workbook
fingerprint/reconciliation, compile/test/build logs, ABI matrix, reset counts,
Restart/profile checks, real linked-cabinet logs/results/screenshots, artifact
checks, installation manifest and hash verification. ROMs and personal data
are not committed. The user removed a specific CAP for this integration;
account usage was 64% at start and 65% near completion, shared across chats.
