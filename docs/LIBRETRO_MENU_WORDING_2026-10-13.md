# User-Revised Framework Menu Wording

## Functional Groups Review — 2026-10-17

### Approved Parent-First Delivery

The user subsequently approved both proposed reorders. Implemented the shared
V2/legacy comparator using the existing `mvd::KEYS` and diagnostic key groups:
MVD Input, Horizontal Range, Vertical Range, Gravity Stabilization, Drift
Compensation, Sensor Diagnostics; Diagnostic Display, Position, Background
Opacity. Each group sorts at its parent label's position. System and relative
global order remain intact. This supersedes the proposal-only status in the
original review below.

The extended existing audit independently asserts both approved group sequences
and compares all metadata against the preceding installed binary: 89 V2 and
89 legacy registrations pass; only order changes. 93 tests, signed macOS ARM64
build and artifact scope/ABI/lifecycle gate pass. Public core/info installed
with backup and matching SHA-256; core
`cef4524b77c9f4d8a70a6e3abbc099759db6394cbd1399c4491c9fd277fdeba4`.
Evidence: `outputs/menu-functional-groups-20261017/`. This narrow follow-up is
registration evidence, not a repeated visual-menu/gameplay trial. No workbook,
version, commit, push or release change. Framework handoff records adoption.

User requested a whole-core group review and Framework handoff after the MVD
range correction. Inspected all 89 actual final V2/legacy registrations in
`outputs/menu-range-group-20261017/audit/registration.json`, the functional menu
ranks, `mvd::KEYS`/Settings/pose and sensor configuration, diagnostic Settings/
rendering, visibility/ANRS and the native operator definitions. This review
does not implement additional reorders or claim new runtime/device tests.

| Functional Group | Current Finding | Required Preservation Or Proposed Adaptation |
| --- | --- | --- |
| Linked Cabinets / ANRS | All six per-set pairs and unsupported pair remain adjacent in System. | Preserve topology then role; neither alphabetical sorting nor a global-first rule should split the pair. |
| Native NVRAM/operator groups | General gate and 43 fields across ten sets retain native grouping/order. SWA and SWAJ Player/Enemy/Timer difficulty triplets remain consecutive. | Preserve explicit System/native exceptions. Do not alphabetize fields or force native role/identity fields together when that would override reviewed operator ordering. |
| Aspect Ratio / Widescreen Mode | Adjacent; widescreen hack behavior depends on aspect/native monitor capability. | Preserve Aspect then Widescreen. Do not combine them into one option. |
| Renderer / Supersampling / Smooth Shadows | Consecutive global Video options; rendering path owns their applicability. | Preserve the existing functional block and lifecycle differences. City Workaround remains a separate game-specific graphics fix. |
| Master/source audio gains | Master followed by source controls; MultiPCM 1/2 pair remains consecutive. | Preserve the pair within global source controls and fitted-source visibility; Audio Donor remains a game-specific source-selection option, not a gain. |
| Driving response/output ranges | Response and Steering Range consecutive, followed by Accelerator/Brake ranges. | Preserve Steering Response/Range and the pedal pair inside the driving block. Do not alphabetize this established global order. |
| Sega NetMerc MVD Horizontal/Vertical Range | Corrected and verified adjacent, H then V. | Preserve as a nested range group. |
| Sega NetMerc MVD input/tracking | All six options are consecutive, but alphabetical order puts Input after parameters it governs. `mvd::KEYS` and Settings already express mode, H/V ranges, gravity, drift, diagnostics. | Proposed order for review: Input, Horizontal Range, Vertical Range, Gravity Stabilization, Drift Compensation, Sensor Diagnostics. H/V applies to stick orientation; gravity/drift concern sensors, so preserve those distinctions. |
| Sega NetMerc Diagnostic LCD | All three options are consecutive, but alphabetical Background Opacity precedes Display enable/scale. Settings/rendering gates the subordinate presentation fields on Display. | Proposed order for review: Diagnostic Display, Diagnostic Position, Diagnostic Background Opacity. |

Single standalone functions such as ROM repairs, Controller Profile Type,
Gamepad Rumble, City Workaround and Audio Donor do not become compound groups
solely because their labels/categories are nearby. A/V Timing controls cadence;
Timing / FPS Overlay is independent diagnostics, so their current adjacency
does not establish a dependency. Automatic Initial NVRAM Setup and NVRAM
Settings have different authority/lifecycle and retain System's established
placement rather than being merged or moved across the networking block.

Transfer rule: identify real dependency/axis/source/native groups from the
implementation and reviewed reference, preserve member order, then apply
global/game grouping and lexical ordering to the resulting blocks. This does
not authorize copying Model 1 labels, native fields or proposed MVD/LCD order
into another core without destination review. Only the H/V correction is
implemented here; the MVD/LCD parent-first proposals are reported for review.

## Frontend Option Ordering — 2026-10-17

User correction: Horizontal/Vertical MVD ranges form a functional group.
They now remain adjacent, Horizontal first and Vertical second, at the group's
alphabetical position. This supersedes the strictly per-label MVD order
described below. Alphabetical order must not split related parameter groups.
The shared V2/legacy comparator and existing audit preserve this explicit
group. All 89 registrations pass with metadata unchanged; 93 tests and the
artifact gate pass. The corrected public core is installed with backup/hash
verification; current manifest is `outputs/menu-range-group-20261017/install.json`.
Evidence is ABI registration for this narrow follow-up, not a repeated visual
menu/gameplay trial. Framework notified; no workbook changes or commit/push.

Delivered the user-authorized Framework 0.1.25 F04 ordering. Rechecked current
SM2 PORTING_PLAN.md and Supermodel Docs/ROADMAP.md: both record this incoming
rule as pending, so their old ordering is not the new specification. The
editable workbook text and stable Order identifiers remain unchanged.

`menu_order()` now wraps the existing functional ranks. System retains its
exact registration order, including operator fields and network pairs. Other
categories keep global options in their previous relative order, followed by
reviewed `<Game Name>: ...` options sorted by the complete displayed label.
Both V2 and legacy use that same sort. This is adapter presentation metadata;
categories, keys, all strings, choices, value labels, defaults, visibility and
machine behavior are unchanged. Audio already conformed; Video/Input changed.

Extended the existing final-registration audit rather than adding a second
runner. It now requires the F04 grouping/lexical order and, with `--order-only`,
compares every field against the baseline while separately preserving System
and global relative order. The previous binary fails Video/Input ordering;
the final build passes all 89 V2/89 legacy definitions and the 34 workbook
rows/44 reviewed keys. The existing legacy-default unit check originally
compared MVD defaults positionally in the old menu order; it now checks by
option identity, preserving the same six expected defaults. Final 93 adapter
tests, release build, scope/ABI/lifecycle artifact checks and diff check pass.

Actual macOS RetroArch ARM Nightly 1.22.2 RGUI acceptance, VR loaded with
isolated options/saves: observed all four Core Options pages. Video shows
Renderer/Supersampling/Smooth Shadows/Aspect/Widescreen/Timing/Overlay before
the alphabetic Sega NetMerc group. Input shows Controller Profile Type first,
then global rumble/driving controls, then MVD Drift/Gravity/Horizontal/Input/
Sensor Diagnostics/Vertical. Audio retains global gains before Audio Donor;
System retains its functional order. No settings were changed during the
inspection. CUA observations were not saved as screenshots. Only the owned
test session was closed, normally; no personal files were used.

Evidence: `outputs/menu-ordering-20261017/` contains failing-before and final
audits/snapshots, tests/build/artifact logs, isolated frontend configuration/log
and final install manifest. Verified public core installed with backup;
build/installed SHA-256
`d9311d4fc7c6437b7361e39c342ad20260e269e96874f6186cc896e2e4cef55f`.
Matching info is unchanged. No workbook edit, new dependencies, version change,
commit, push or release. Legacy acceptance is ABI registration evidence, not
an actual old frontend run. No new gameplay/network qualification is claimed.

## Final Registered Definitions Audit — 2026-10-17

Completed the user-authorized Framework 0.1.24 F04/F14 follow-up. Rechecked the
current SM2 handoff/PORTING_PLAN.md and Supermodel Docs/ROADMAP.md. The editable
Menu Review workbook remains authoritative: 34 applicable non-operator rows
map to 44 keys. Historical Source Options is not the specification. Two
previously documented Model 1 exceptions remain: Audio Donor single colon and
Widescreen's full-image stretch/native-widescreen lifecycle description.

Reused LCFHost to capture complete final SET_CORE_OPTIONS_V2/SET_VARIABLES
registrations in `tools/test_libretro_menu_wording.py`. This checks all 89 modern
and 89 legacy definitions, complete reviewed names/help/categories, six native
role/topology value/label/default sequences, placeholder distinction, category
order and pair adjacency. It compares every raw value, value label and default
with the previous installed artifact to catch unintended changes. Legacy has no
help or independent value-label API: it checks names, default-first raw values,
recognized Gain aliases and registration order, without claiming those missing
features. Registration is static across game loads; all per-set definitions are
constructed before visibility changes. No ROM is needed for this audit.

Found and corrected:

- The six Linked Cabinets definitions shared one help, but it still included
  game-specific topology prose. Applied the common full comment recorded in
  the SM2 handoff and Framework F04/F14, then reconciled it into Menu Review.
- Unsupported ANRS and the first supported Linked Cabinets key shared a sort
  rank, producing different legacy/V2 orders. Gave supported pairs distinct
  ranks. Only those two legacy positions changed; the V2 order stayed intact.

No raw lowercase ROM identifiers or the reviewed stale wording/typos occur in
final names/help. Stored keys, choices, value labels, defaults, native operator
definitions and emulation behavior are unchanged. No new gameplay or visual
menu qualification is claimed for this metadata-only correction.

### Shared Workbook Reconciliation

The user additionally authorized updating the reference Excel. Changed only
Menu Review E13/F13 (Order 034, ANRS), F121 (035, Linked Cabinets), F51 (084,
lightgun typo), F124 (077, selected cabinet wording), and F137/I137/J137 (222,
retired Network Board, no remaining core membership). The old Name is retained
for Order 034 and Name (New) holds Auto Network Role Settings. The user then
required removing Network Board entirely: Order 222 has been deleted from Menu
Review, with its table/filter reduced to A1:J203 (202 data rows). No retired
placeholder remains. Other Orders are unchanged; Source Options is historical
registration evidence and was not edited. A narrow OOXML deletion preserved all
remaining values/styles and every unrelated ZIP part, since Artifact Tool has
no documented row-deletion API. Rendered/inspected the result with Artifact Tool
and reran the complete registration audit successfully against the updated file.

Authored with Artifact Tool; rendered affected ranges and compared every cell
and style before delivery. Eight cell values changed; only rows 13/121 grew
to fit help. Source Options, styles, sheet order, table ranges/filterable
TableStyleMedium8 tables, freeze panes, merges, validation and hidden rows are
preserved. Backup: `outputs/registered-menu-audit-20261017/workbook/before.xlsx`.
No concurrent changes were overwritten (source hash checked before copying).
Final shared workbook SHA-256:
`4ceb9be04d7ad32201c78ae53ac2b103987f8867df69675230996dab7c9c125a`.
The pre-deletion backup is `workbook/before-remove.xlsx` in the evidence root.

Reran the audit against that exact workbook and the normal installed core:
34 rows/44 reviewed keys, 89 V2/89 legacy, six network pairs, zero errors.
The network help is now checked directly against the workbook as well as the
independent F14 expected text, with no network-text override in the test.

### Delivery And Evidence

93 adapter tests, signed macOS ARM64 release build, Model 1 artifact/ABI/lifecycle
check and diff whitespace check pass. Installed the verified public core and
matching info with backup using the established installer; SHA-256:
`f9ab52ea399875bf6edfd921cec665703956c053cdac3f4f01a2abf1446716db`.
Info remains `e7ddd4c8355b22f2c6c2810c27443eae9b0b29a2984cc2f67ddd2912ba5cab00`.
Evidence root: `outputs/registered-menu-audit-20261017/`, including before/final
registration snapshots, workbook-aligned audit, build/tests/artifact logs,
workbook verification/renders and installation manifest. No commit, push,
release, version change, new dependencies or personal setting changes.
Account usage: 73% before / 74% after (shared account, rounded display), reset
2026-10-22 09:44 Europe/Rome. Required reasoning High; effort M.

## Reviewed Source And Scope

Source: `libretro-core-framework/outputs/01a13152-dbfa-7e30-bc9d-0477abc2cccc/Framework_Menu_Review.xlsx`, Menu Review, 203 data rows.
Workbook SHA-256 before and after this work:
`72b96114c437313fef061c3517a2812daa7563be0e9b15b1d270e14a375e374a`.
The workbook and shared Framework documents were not edited.

Selected 34 rows with `Core: TGPulse-Next = True` and
`Per-Game NVRAM = False`. Effective names use Name (New) when populated,
otherwise Name. Historical Source Options supplied key evidence only;
superseded split rows were not restored. Automatic Network Settings and
Linked Cabinets each map to six existing per-set keys, yielding 44 keys.
The general NVRAM Settings switch is included; native per-game operator
fields are excluded. Off-Screen Reload Shortcut is not applicable.

`crates/tgpulse-libretro/src/menu_wording.rs` holds frontend metadata only.
V2 registration uses the reviewed names, comments and categories. Legacy
registration shares the names while retaining its original complete raw-value
suffix. No workbook dependency exists at runtime. Keys, values, value labels,
defaults, visibility, menu ordering and emulation paths remain unchanged.
This wording revision introduces no synthetic feedback or new inputs.

## Precise Exceptions

| Reviewed Entry | Adaptation And Reason |
| --- | --- |
| Order 022, Sega NetMerc Audio Donor | The workbook has `Sega NetMerc:: Audio Donor`. Registration uses `Sega NetMerc: Audio Donor (Restart Required)`, following the explicit single-colon convention. The source workbook retains its original text. |
| Order 207, Widescreen Mode | Adopted the reviewed name. The common comment describes only field-of-view expansion; the existing third value stretches the complete image. Target help therefore names both operations, requires a wide Aspect Ratio and hardware renderer, preserves native widescreen, and retains the content-reload instruction. No renderer behavior changed. |

Other reviewed comments are concise summaries, not exhaustive runtime
specifications. For example, Supersampling preserves its existing 3D-only
sampling boundary; Audio Donor retains its current missing-donor fallbacks.
No implementation was changed to make a generic comment appear universal.

### Approved Rumble Comment Revision

The subsequent user-approved edit to Menu Review!F98 (Order 071) replaces the
previous common comment with exactly:
"Enable gamepad vibration using the game's force-feedback or rumble effects."
This text is now applied to Gamepad Rumble, superseding the earlier target
Rumble exception. Choices, value labels, default and behavior are unchanged.
The other factual exceptions above remain valid.

Revised workbook SHA-256:
`8d13f8ba49275ce2f8e23fb088cba182c39c10c87c2e2ca62dee8e51b97118ef`.
The initial hash above identifies the first review; the new hash identifies
the approved follow-up. Neither revision was edited by this core task.

Follow-up verification: complete before/after registration comparison found
only the Gamepad Rumble help changed. All modern and legacy definitions,
choices, value labels, defaults and ordering otherwise match the previous
build. The macOS release build and artifact/ABI/lifecycle checks passed; the
public core and matching metadata were installed with backup and SHA-256
verification. Updated installed-core SHA-256:
`f258f3944828cbf2815d042540d1d8ff474bf51a5f8e8aa5608fe71e3e7545e0`.
No new visual-menu or physical-controller acceptance is claimed; the visible
menu limitation below remains open. No commit or push was made.

## Verification And Installation

- Compared before/after complete V2 and legacy registration snapshots using
  the existing `LCFHost` ABI harness, without loading ROMs. All 44 mapped
  keys have the expected reviewed text or the exceptions above. Option order,
  categories, values, labels and defaults match the baseline. Every unmapped
  definition, including native per-game NVRAM, is byte-for-byte unchanged.
- `CARGO_NET_OFFLINE=true cargo test --locked -p tgpulse-libretro --lib`:
  92 passed. Two existing legacy-label assertions were updated for the
  approved single-colon labels; their value/default checks remain intact.
- `CARGO_NET_OFFLINE=true make -f Makefile.libretro`: macOS ARM64 release
  succeeded. The artifact checker passed Model 1 scope, 25 Libretro exports,
  dependencies and three empty lifecycle cycles.
- Installed the public core and matching metadata with `install_dev_core.py`;
  previous files backed up. Build and installed-core SHA-256:
  `2ce244e0745aa3d07a037f9201587e557dd07878821166f8f84170d890114118`.
- `git diff --check` passed. No commit, push, release, version change or shared
  Framework modification was made.

## Visible Menu Check: Incomplete

Launched VR in RetroArch 1.22.2 Nightly with isolated files under
`/private/tmp/tgpulse-menu-wording-ui-20261013`. CUA showed the correct
`RetroArch TGPulse-Next-M1 0.1.0.10` window and paused Quick Menu.
Coordinate clicks into Core Options returned `noWindowsAvailable`, although
the process and window remained present. Keyboard navigation did not change
the selected row. No resulting Core Options page or help text was observed;
the visible wording acceptance is therefore still pending.

An empty instance created by application selection was identified by its PID
and closed separately; the isolated VR instance was then closed through native
Quit RetroArch. The final process check found no RetroArch process. Personal
configuration and saves were not used for the VR test. This attempt does not
replace the earlier completed E6 paused-profile UI evidence.
