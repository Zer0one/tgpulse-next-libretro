# Cabinet input audit — 2026-09-26

## Libretro Device Variants Restored (2026-10-04)

The adapter previously exposed only one device type per port and always read
L3/R3. Current SM2 `input.cpp::configure_controllers`, `service_enabled` and
`joypad_enabled`, and Supermodel `libretro.cpp::set_controller_info` plus
`CLibretroInputSystem.cpp` establish two variants: full base RetroPad by default,
and a reduced device subclass. The adapter now follows that behavior on both
ports for every Model 1 profile.

| Variant | Device ID | Cabinet Slots |
| --- | --- | --- |
| `<Profile> + Test/Service Slots` | Base RetroPad (`1`) | Test L3, Service R3 |
| `<Profile>` | RetroPad Subclass 0 (`257`) | Hidden And Inactive |

Suffix capitalization follows the user's Title Case convention. Selection is
independent per port; SWA retains Pilot/Gunner names. Reduced profiles retain
all gameplay aliases, analog axes, NetMerc MVD commands and rumble eligibility.
None disables the whole port. The callbacks continue polling base Libretro
device classes rather than passing the subclass ID as an input device.

Verification: 63 adapter tests, including per-port cabinet gating and gameplay,
analog/MVD preservation; the existing NetMerc ABI runner now also checks all ten
sets with `--rom-dir`, including profile names/IDs, descriptors and transitions
between full/reduced/None. Existing NetMerc live controls, input thresholds,
notification, state and command-edge checks pass. Offline locked release build
and Model 1 artifact gate pass. Evidence is in
`/private/tmp/tgpulse-test-service-slots/`; the installed Development core matches
the tested build SHA-256
`1157cf75d5f15baaa37ed800279e1221542a764105cd6601afde2fc11f433178`.
No personal remap/save/config changes or new physical-controller trial are
included. No commit, push or release was performed.

## Authority and scope

The user-designated, tested **SM2-Emu Libretro** reference takes precedence
for every Model 2 title it contains, including disagreements with MAME:

- Repository: `Zer0one/sm2-emu-libretro`, revision
  `f4d9051e2a0dbc54dec8ce78feb9a619f2eea812`.
- `Docs/revisione_profili_model2.xlsx`, sheet `Profili`, `A1:AB44`:
  authoritative action/profile/axis inventory. SHA-256:
  `c8c83e39b9257dd211d0527f8ea35526f9111f1ff1bbc610cc33dbf32622f2c2`.
- `src/libretro/input.cpp`: actual profile selection and action translation;
  `data/games.xml`, `src/rom/game.h`, `src/rom/game_db.cpp`: channels, ranges,
  polarity, default wheel-button masks and clone inheritance.
- `src/hw/model2.cpp` and `src/hw/model2_machine_base.h`: electrical port
  interpretation and Air Walkers' matrix. The workbook's Model 3 comparison
  rows are not additional TGPulse games.

MAME `src/mame/sega/model1.cpp` / `model2.cpp` at
`bd7e0b815842ec461e8ad2538d127f3332f5c96c` supplements the reference for
Model 1 and Power Sled, not as an override to tested Model 2 behaviour.
Both reference repositories and the workbook were read-only.

The audit covers all **100 ROM sets** currently in TGPulse's database, including
clones. It verifies frontend signal-to-port translation and the supported
player's axes, not game compatibility, graphics, audio, netplay or full
multiplayer implementation.

## Corrections

### Workbook functional alignment — 2026-09-28

Acceptance is physical button → in-game action. Extra aliases may remain only
when they do not activate a second cabinet function on the same press. The
single public GUI catalogue now exposes independent Sega Rally Handbrake,
Desert Gun/Cannon, Virtual On left/right Shot/Dash, Ski Super G left/right
Foot Sensor and Select 2/3, and Water Ski Set/Pitch Left/Pitch Right signals.
No second mapping layer or game-filtered GUI list was added. Motor Raid reuses
the existing Action 1/2 OR defaults (South/L1 Kick, East/R1 Punch) but corrects
their electrical route. Fighter/baseball/soccer and Top Skater action wiring is
corrected without inventing per-game duplicate face-button signals. Default
At this checkpoint, Test/Service followed SM2 R3/L3 physical positions
respectively. The later TGPulse preference restores Test/L3 and Service/R3;
only exact former stock expressions refresh on load, while custom and empty
values are preserved. The historical audit below describes this checkpoint.

The workbook-only Model 3 rows are out of scope. Air Walkers P1/P2 face
positions are aligned; the workbook's P3/P4 electrical matrix needs a separate
multiplayer milestone. No manual game/device acceptance is implied by tests.
The legacy 100-set fingerprints remain frozen and are still checked for
unchanged families; intentionally corrected families use explicit isolated
port and physical-button assertions instead of falsifying the old fingerprints.

### Signal catalogue cleanup — 2026-09-28

Extra Action is labelled `Power Sled: Cancel Error` without a redundant usage
note. View / Select 1 no longer displays its game-list note; the existing
single-view D-pad OR behavior is unchanged. Sky Target: Machine Gun and
Sky Target: Missile now have their own keys and GUI rows, independent of
Gun Primary Fire and Gun Secondary Fire. The four Gun rows (Yaw, Pitch,
Primary Fire, Secondary Fire) are consecutive in the one catalogue.
Gun configuration keeps the `primary_fire` and `secondary_fire` keys so existing
custom assignments survive; Sky Target inherits its separate defaults when its
new keys are absent. No ROM, NVRAM or user config file was rewritten.
Behind Enemy Lines' P1/P2 missile remains on Gun Secondary Fire, without a
per-game usage note. The touch missile button uses that same signal.

The GUI now separates the Gun rows from the lexicographically ordered
game-specific groups. Power Sled: Cancel Error and Dynamite Baseball: Bat Swing
are among those groups; signals within one game retain their existing relative
order. Cabinet P1/P2 keep visual separators but no section headings; the
display order does not alter saved keys, defaults or hardware routing.


| Cabinet / family | Corrected routing |
| --- | --- |
| Indy 500, all revisions | Start at IN0:40; View 1/2 at IN1:01/02; no D-pad Left → Start alias |
| Sega Touring Car, all revisions | Both view inputs restored, using SM2's default wheel masks on IN1 |
| Indy / Touring / Over Rev / Super GT / Manx | Momentary shifts have no H-gate residue; conflicting shifts are released |
| Motor Raid | Punch and Kick remain independent and may be pressed together |
| Sky Target | Start at IN0:40, not IN0:10; proper full-range stick axes and channel order |
| Behind Enemy Lines | Service IN0:04, Test IN0:08; Missile is not off-screen reload |
| Wing War | Machine Gun / Missile / Smoke at IN1:10/20/40; all four views restored; 360 variant has no view switches and has different stick polarity |
| NetMerc | MVD Holder at IN1:04, no invented Start switch; Y reaches channel 2, not the throttle mirror |
| Star Wars Arcade | At this audit: Start 2 restored; Y direction, throttle centre/range and independent idle gunner axes. Superseded by the P2 follow-up below. |
| Two-button joystick cabinets | Third/unused action line stays released; Dynamite Baseball retains its own Button 1/2 order and Bat Swing |
| Hanguk Pro Yagu / Royal Ascot / Air Walkers | Anonymous Button 1/2 follow SM2's South/East ordering rather than the fighting template |
| All Model 2 cars / bikes | Full 00..ff ADC travel, including Daytona; reversed Bank; reversed Over Rev / Super GT pedals |
| Sega Rally | Handbrake retains intermediate values when rebound to an analog source |
| Ski Super G | Tested SM2 order: ADC 0 Inclining, ADC 1 reversed Swing; not the opposite MAME-generated order |
| Water Ski / Top Skater | Reversed Slide / Curving per SM2; Top Skater Slide remains independent and direct; Water Ski Pitch Left/Right use Action 2/1 (now R1/L1) |
| Wave Runner | Reversed throttle half-range 80→00, independent Roll/Pitch; only the real coin line is used |
| Gun cabinets | Stick aim moves and holds a cursor; reload only on serial guns; correct per-title ADC / serial ranges; rchase2a remains distinct from rchase2 |
| Air Walkers | Port F bit 7 selects the other player pair; P1 controls and Start are not mirrored onto P3. Unsupported P3/P4 remain released |
| Power Sled | Action 3 no longer presses seat 2 Entry; Extra Action reaches Cancel Error |
| Original I/O boards | steer/accel/brake mirror the routed ADC channels instead of bypassing their game-specific calibration |

Per-game unused inputs are released. IN2 is reinitialized on every racing
sample so another cabinet's handbrake/port values cannot leak through.

## Deliberate differences from SM2 physical defaults

The agreed **single global signal catalogue and binding file** remain in place.
This audit is not permission to replace them with per-game RetroPad bindings.

- The remaining shared cabinet actions are labelled Button 1 / Kick (South OR
  L1), Button 2 / Punch (East OR R1), and Button 3 / Guard / Jump / Hold /
  Barrier (West). Virtua Striker, Wing War, Star Wars Arcade, Top Skater,
  Power Sled, Ski Super G, Desert Tank and NetMerc have independent
  game-prefixed action signals with the former defaults. Dedicated Gear
  Down/Up use L1/R1.
  Gun and Sky Target fire use independent Primary/Secondary Fire with
  South/R1 and East/L1, as does Desert Tank's Gun/Cannon pair. Motor Raid's
  Punch/Kick remain on Action 2/1 and are not shifts. Fighting, soccer and
  baseball retain their semantic port-order exceptions. Shoulder/face aliases
  remain OR, not chords.
- **VF and VF2 override, explicitly confirmed by the user:** Action 1
  South/L1 = Kick (`02`), Action 2 East/R1 = Punch (`01`), Action 3 West =
  Guard (`04`), for P1 and P2. All four VF2 revisions share this rule.
  Other fighters now use the workbook's South Kick / East Punch positions;
  DOA retains its distinct electrical bits and Hold on West.
- At this checkpoint Service used L3/F8 and Test R3/F2, matching the workbook.
  The later TGPulse preference is Service R3/F8 and Test L3/F2.
  BEL swaps the destination bits, not these bindings.
- Views retain the one global Down/Left/Right/Up list. Two-view driving
  cabinets consume View / Select 4 (Up) and 1 (Down), following SM2's layout.
- Start also serves the shared Manx TT/Motor Raid Start/VR and Water Ski
  Select Down lines. Ski Super G Select 3 is now independent, as in the
  workbook; Start alone no longer activates it.
- Ski Super G uses separate left/right foot sensors, a dedicated Select 1
  signal and Select 2/3 signals; it does not copy SM2's
  separate face-button and shoulder assignments.
- Keyboard ramping, deadzones, H-gate latching and initial gear policy are
  retained. This is not a port of all SM2 controller tuning options.

## Implementation boundary

The original audit changed `input/signals/routing.rs`, `input.rs` and tests;
the native refactor below replaces that compatibility route with `input/cabinet.rs`
and `input/cabinet/analog.rs`, preserving the audited electrical mappings.
Ski Super G's channel override is guarded by the old metadata order; a future
corrected database does not get swapped back. No ROM database regeneration or
second public input list is required.

Air Walkers needs a small hardware mux correction: its immutable wiring flag
comes from the identified ROM in the loader, and the already-saved port-F
latch selects the pair. No new live-input or save-state serialized fields
were added. The initial latch selects P1/P2, matching the reference.

No NVRAM or input configuration was rewritten. The development build includes
the pre-existing pending fullscreen and Return to game menu changes. The
toolkit, updater-managed build and `current` release were not modified.

## Verification and honest limits

`input/signals/audit.rs` contains independent expectations for all 100 sets:

- Every set must have exactly one explicit audit case; a new/removed set makes
  the coverage test fail instead of silently inheriting a generic layout.
- Every public signal is individually rebound, pressed and released; complete
  IN0/IN1/IN2 bytes are checked, including unused inputs and latch exceptions.
- Real default button identifiers are checked for Indy/Touring/Over Rev,
  including Start, both views, Service/Test and conflicting shifts.
- All driving revisions are checked for ADC idle/endpoints, polarity and
  original-board channel mirrors. Additional tests cover flight/body axes,
  gun calibration/cursor persistence, throttle ranges and partial handbrake.
- A core test checks that selecting Air Walkers' second pair preserves
  coin/service lines but never repeats P1/P2 Start or controls.

Run `cargo test --workspace --offline` and `cargo build --release --offline`.
Tests use a device-free input constructor: they neither poll the user's
controller nor start force-feedback worker threads.

The local verification passed **89 tests** (55 in the frontend) and the release
build. `tgpulse.dev --list` also completed through the installed launcher and
found 100 ROM sets. This smoke check does not launch a game.

These are source-backed automated routing checks, **not physical-controller
or in-game proof for all 100 titles**. Validate using `tgpulse.dev`, starting
with Indy 500's input test and gameplay, then revised analog cabinets.
Existing NVRAM calibrated against the former ADC range may need the game's
normal input-calibration procedure; never erase it automatically.

At the time of this audit TGPulse exposed one gameplay controller; this is
superseded for Model 1 and Model 2 by the 2026-09-27 P2 follow-ups below, not by
a full SM2 P1–P4 implementation. Air Walkers P3/P4 remain unimplemented;
Power Sled's second seat is now routed. Its extra IN3 network-check switch has no
frontend/core input lane and was not invented as another action or public
configuration list. No full multiplayer or device-emulation expansion is
claimed by this mapping audit.

### Model 1 P2 follow-up — 2026-09-27

Cabinet P1/P2 now share the catalogue with independent device/binding selection.
VF routes P2 directions/actions to IN.2; SWA/SWAJ routes the Gunner's two fire
buttons and stick to IN.1 bits 04/08 and ADC 4/5. Per user clarification the
Gunner has **no Start, view or throttle**: the previously routed Start2 bit
is intentionally left inactive in these two games despite its MAME port label.
The supplied in-game INPUT TEST 1/2 screenshot confirms Gunner Laser/Torpedo
and no Gunner VR Button/Start; all switches were OFF, so this verifies the
layout, not actual device routing or the analog page.
Test/Service are independently bindable aliases for the same hardware switches.
Coin2 and other games' existing Start2 routes remain, now owned by P2 bindings.
See [P2 contract and verification limits](INPUTS.md#player-2--model-1-and-model-2).

### Model 2 P2 follow-up — 2026-09-27

P2 was requested for the emulator, not restricted to Model 1. Source baseline:

- SM2 Libretro commit `75ced234792520c325aa63aa7be3dfaf87b07d46`:
  `Docs/revisione_profili_model2.xlsx`, sheet `Profili`, rows 12–17 and 20–34;
  `src/libretro/input.cpp` (player counts, per-player sampling and port routing),
  `data/games.xml` and `src/rom/game_db.cpp` (ADC/lightgun metadata and inheritance).
  Workbook SHA256: `c8c83e39b9257dd211d0527f8ea35526f9111f1ff1bbc610cc33dbf32622f2c2`.
- Power Sled is not in that workbook: MAME `model2.cpp`, `powsled` ports,
  revision `bd7e0b815842ec461e8ad2538d127f3332f5c96c`, is the supplementary source.

Joystick P2 reuses the existing P1 semantic translation, with an independent
binding/controller and IN.2. Physical default conventions remain those agreed
for TGPulse (Action 1 South/L1, Action 2 East/R1), not a replacement of the P1
layout by SM2's physical buttons. Virtual On and Royal Ascot II remain single
local gameplay panels; the latter no longer exposes a spurious Start2 route.
Air Walkers' second player uses its first pair; P3/P4 stay released.

Bat Swing drives the second Baseball ADC. Power Sled's Entry/Call use IN.1
04/08; right/left pedals use ADC 5/7. Gunblade/BEL and Rail Chase 2 use
independent calibrated positional axes; VC/VC2/HOTD use independent serial
coordinates and off-screen flags. Initial positional P2 aim retains the old
calibrated rest value; subsequent movement scales its own cursor through the
actual min/max range. BEL's secondary button drives P2 missile IN.1:20.

**Reference nuance:** `hotd` explicitly sets `p2_trigger="in2"`; `hotdo` and
`hotdp` declare their own lightgun axes without that flag. In the inspected
SM2 loader, a present child lightgun spec does not inherit the parent's spec,
so those two revisions resolve to IN.1:02. TGPulse follows that effective
reference, while `hotd` uses IN.2:01. This distinction is source-backed and
still needs real input-test confirmation for the clones.

The old core supplied constants for the second gun. Both DPRAM publication
and the 315-5649 serial mux now forward P2, including independent off-screen
flags. Snapshot format 2 retains those new values and the pending mux index;
old format-1 machine states are rejected explicitly. NVRAM is unaffected.

Automated checks cover every signal across all 100 sets for both players,
P1 isolation, action release, two-button masks, analog endpoints/polarity,
independent gun cursor hold/reload, migration and both native gun transports
with mid-mux snapshot/restore. These are not two-controller gameplay proof;
real device/input-test validation remains pending.

### Native input refactor — 2026-09-27

Work isolated on `codex/native-cabinet-inputs`, from `main` at `1fedad9`.
Merge requires the user's manual acceptance. No default, expression grammar,
binding-file version, GUI catalogue, ROM database, core input structure or
machine snapshot format change is part of this refactor.

The old `Signal -> Control -> generic ports -> route_ports` path is removed.
Each family builds ports directly from the public signals; calibrated ADCs
retain the existing game/role decisions. `input/sampling.rs` owns binding
evaluation; `input/cabinet.rs` and `cabinet/analog.rs` own cabinet behavior.
These remain frontend modules; the core continues accepting plain `Inputs`.

Before changing production polling, `cabinet_trace_equivalence` captured one
deterministic FNV-1a fingerprint per set in `input/signals/equivalence.txt`.
It covers all 100 database sets: idle, keyboard holds/releases, P1/P2 pad
buttons, assignable signals rebound to full/half axes, deadzone/threshold
samples, simultaneous shift inputs and stateful continuation. Each sample
contains all `Inputs` fields plus both displayed gun positions. This is a
regression baseline, not new hardware truth: the independent source-backed
port/ADC audits remain necessary. Do not regenerate fingerprints to conceal
a mismatch. A legitimate output/trace-format change needs explicit review
against the base revision and the source-backed assertions.

| Checkpoint / family | Equivalence and independent audit |
| --- | --- |
| Joysticks / twin sticks / P2 | 100-set trace and 19 audit tests passed after the family replacement. |
| Cars / bikes / H-gate / sequential shifts | Same full gate passed; ramps, thresholds, latch behavior and bindings retained. |
| Flight / SWA Gunner / throttle | Same full gate passed; polarity and half-axis conventions unchanged. |
| Serial and positional guns / P2 | Same full gate passed, including cursor hold, reload and calibrated channels. |
| Jetski / skate / ski / sled | Same full gate passed. The first comparison caught a Ski Super G Start threshold regression (0 instead of 0.5); new code was corrected without changing the baseline. |
| Removal of legacy enum / sampling / touch | Full input suite passes; dedicated touch adapter tests cover signed axes and native signal values. |

Checkpoint command: `cargo test --offline -p tgpulse --bin tgpulse input::signals::audit`.
Final gate: `cargo test --offline --workspace`, then
`cargo build --offline --release -p tgpulse`.
Tests are device-free and do not poll the user's controller or alter settings,
NVRAM or saves. TCP unit tests need permission to bind local ports.

Final verification on this branch: **466 workspace tests passed**, release
build succeeded, and `target/release/tgpulse --list` found all 100 ROM sets.
The focused native-input suite has 50 tests and the touch suite 12. Remaining
compiler warnings come from the unchanged `m68000` / `block` dependencies.
The list command is a loader/catalogue smoke check, not a gameplay test.

Touch necessarily moves from old control identifiers to native signals;
its numbered actions now refer to the public Action signals, and its sticks
emit signed game-appropriate axes. Dedicated adapter-contract tests are kept
separate from the frozen keyboard/gamepad traces: old touch behavior is not
claimed bit-for-bit equivalent or tested on Android hardware. No P2 touch
support is added.

Before merge, use the development binary `target/release/tgpulse`
(`tgpulse.dev`), for example: VR and Sega Rally (ramps/view/shifts), Indy 500
(Start/view), VF/VF2 (actions/P2), Wing War/SWA (stick/throttle/Gunner),
and one gun game (aim/reload). Existing config, calibration and save data
must be retained. Automated equivalence does not replace this manual
acceptance or imply fresh gameplay proof for every ROM set.

### VF / VF2 semantic correction after equivalence — 2026-09-27

The user requested SM2's South=Kick, East=Punch, West=Guard exception for
both Virtua Fighter generations. Source: SM2 Libretro
`src/libretro/input.cpp`, `fighting_bindings` (RetroPad B `02` Kick,
A `01` Punch, Y `04` Guard); the Sega
[VF service manual](https://manualzz.com/doc/23138502/sega-virtua-fighter-arcade-game-service-manual)
identifies SW1 Punch, SW2 Kick, SW3 Guard. MAME `model1.cpp`'s VF ports map
SW1/2/3 to `01/02/04` for both players.

The earlier VF expectation incorrectly reused DOA's port order. VF2's earlier
Action order also failed the requested SM2 physical convention. These were
pre-existing semantic errors, not refactor regressions: the original audit
expectations and the frozen trace reproduced them. They are now corrected
for `vf`, `vf2`, `vf2a`, `vf2b`, `vf2o`, without modifying global binding
defaults, user config, NVRAM, other titles or sequential shifting.

Regression strategy: **all original fingerprints remain untouched**. For
only these five sets, the trace comparison maps a copy of the two player
ports back to the old bit order (VF swaps `01/04`; VF2 swaps `01/02`). Thus
all other bits, axes and stateful behavior still compare to the original
baseline. A separate test asserts the actual, unnormalized Punch/Kick/Guard
bits for both players, each face/shoulder OR alternative, default P1 keyboard
keys, custom P2 keys and release/isolation. It failed before the production
fix (`vf`, P1 East) and passes with the correction. Real-game acceptance of
the rebuilt binary remains for the user; no merge is implied.

Verification after this correction: **467 workspace tests passed** and
`cargo build --offline --release -p tgpulse` succeeded. The development
binary is `target/release/tgpulse`, launched by `tgpulse.dev`; toolkit
current/my releases and personal configuration/NVRAM were not modified.

### Dedicated sequential gear signals — 2026-09-27

User-authorized catalogue extension after the native refactor: `Gear Down`
(`gear_down = KeyE, pad:LeftTrigger`) and `Gear Up`
(`gear_up = KeyQ, pad:RightTrigger`). Cabinet P1 displays them after H-Gate;
P2 shows disabled rows because no current local P2 gearbox exists. The GUI
uses the existing catalogue editor, not a separate section or binding layer.

Native sequential cabinets and sequential stepping of H-gates now consume
these signals, not Action 1/2. Direct H-gate assignments, native port polarity,
momentary versus latched behavior and conflicting-input policy are unchanged.
Motor Raid attacks and Desert Tank fire/toggle are not sequential gears and
retain their Action signals. The touch G+/G- controls follow the new signals
except for those existing non-sequential action layouts.

No user configuration is rewritten. Missing new keys inherit defaults and
explicit bindings/empty values survive save/load. Historical merged Action
keyboard aliases are retained for other games, but cannot shift a racing
cabinet unless independently assigned to Gear Down/Up. Old separate gear
keyboard entries migrate directly into the dedicated signals.

Original 100-set trace fingerprints are unchanged. A test-only adapter
replays historical Action1/2 requests on GearDown/Up and omits the two new
signals from the old stimulus sequence; this checks unchanged cabinet
behavior, not equality of the deliberately changed defaults. Independent
raw-port tests cover each new signal across the full roster and rebind both
gear signals on every applicable driving set, proving the old Action keys,
faces and shoulders no longer shift after that rebind. Default tests check
L1/R1, no South/East shift, E/Q persistence, release/conflicts, H-gate latching,
P2-disabled capability, legacy import and touch/native agreement.

Verification: **469 workspace tests passed**, development release built.
Manual GUI/gamepad acceptance remains with the user. No commit, merge,
toolkit deployment or ROM/NVRAM modification is part of this checkpoint.

## U2 NetMerc Reference Alignment — 2026-10-04

The current standalone signal catalogue supersedes NetMerc's initial
Button 1/L1, Button 2/R1 and West/Holder layout. The approved profile is
Special: Sega NetMerc. Trigger Button uses South/L2/R2; Thumb Button uses East/L1/R1;
MVD Holder uses Start/D-pad Down, with identical labels per native alias.
North and West are the separate virtual MVD Calibrate/Recenter commands,
without native cabinet bits. Native ADC Y now maps up to 00 and down to FF,
matching standalone. P1 right-stick MVD uses the upstream orientation signs;
P2 retains Test/Service only. Full Button spelling matches current SM2 and
Supermodel. Detailed mappings, axis signs, limits and delivery evidence are in
[NetMerc Controls](LIBRETRO_U2_CONTROLS.md); other profiles retain their mappings.
