# Native cabinet inputs

The frontend has one public signal catalogue, one binding file
(`config/input.conf`), and two unfiltered tabs in Settings → Input:
**Cabinet P1** and **Cabinet P2**, both showing the same catalogue.
Bindings produce logical signal values; cabinet polling builds native I/O
ports and calibrated ADC values directly from them, according to the ROM set
and its `Scheme` / `AnalogRole` metadata. There is no intermediate `Control`
catalogue or generic-port pass followed by game-specific port overrides.
The core carries native digital/ADC values,
including independent P2 lightgun coordinates through the DPRAM and serial
mux. Air Walkers also has a port-F player-pair mux; the public catalogue and
bindings remain frontend-only.

TGPulse's preferred Test/Service buttons are Test on L3 (F2) and Service on
R3 (F8), for both P1 and P2. This is an explicit local preference: the SM2
Libretro workbook and core use the opposite physical positions. The binding
loader updates only the exact former stock expressions in memory, preserving
custom and empty bindings; saving settings writes the current preference.

Desktop pad buttons are tracked per device from gilrs' logical press/release
events by default. On macOS, Settings → Input → Emulator can instead select
SDL3 as the gamepad backend; `gamepad_backend = sdl3` in `config/input.conf`
persists this choice. Only the selected provider samples physical gamepads;
both use the same P1/P2 signals, device assignment and binding expressions.
SDL3 needs the native SDL3 library available at build and launch time. Neither
provider changes keyboard, mouse, touch or the emulated I/O ports. Device
identifiers can differ between providers: if P1/P2 was assigned an
explicit controller rather than `auto`, reselect it after switching backend.

Model 2 pad rumble now interprets the drive-board byte by game family:
Daytona/Indy 500, Sega Rally and Touring Car. The device-neutral command
decoder is in `tgpulse-core`; the desktop adapter maps impacts and steering
load to P1 pad motors, with separate SDL3 low/high levels. The command tables
and treatment of streamed torque follow SM2-Emu's standalone implementation;
no board protocol is inferred for other Model 2 titles. Pad vibration is an
approximation, not directional wheel force feedback or physical-test proof.
Native-code fallback polling is deliberately avoided: on macOS an
SDL-mapped Xbox R3 can share the fallback code for D-pad Right, otherwise
activating Service and View / Select 3 together. Disconnecting clears the
device's button state; this does not change the binding catalogue.

Trigger bindings keep the `LeftZ` / `RightZ` names. When a pad mapping has no
corresponding axis, the frontend reads the analog value of `LeftTrigger2` /
`RightTrigger2` instead. Actual mapped axes take priority; the existing pedal
deadzone and calibration apply equally to both representations.

## Editing bindings

Click a Cabinet binding, edit its expression, then Apply. Cancel discards the
editor contents. An invalid expression is reported without replacing the
working binding. Emulator hotkeys retain keyboard capture.

### Player 2 — Model 1 and Model 2

Each player has independent bindings and a Controller selector. Unsupported
P2 signals remain visible, grey and non-editable; the list is not filtered by
the current game. A row is enabled when at least one supported cabinet has
that P2 counterpart. The scope includes both Model 1 and Model 2, following
the tested SM2 Libretro workbook/implementation for Model 2.
P2 only shows a parenthesized game list when the corresponding P1 signal has
one, adapting the list to P2 where applicable (Analog Joystick: SWA Gunner).

- **Virtua Fighter:** directions and Action 1/2/3 use IN.2, independently of
  P1's IN.1. Action 1 = Kick, Action 2 = Punch, Action 3 = Guard, as for P1
  and VF2; this is the explicitly requested SM2 fighting-layout exception.
- **Star Wars Arcade:** the Gunner is a subset of the Pilot: Analog Joystick
  X/Y and the dedicated Laser/Torpedo signals only, no Start, VR/view or throttle. Fire bits are IN.1
  `04/08`, stick channels ADC 4/5, centre 127 and range 27..227. Stick polarity
  matches P1. MAME declares IN.0 `20` as Start2; intentionally left inactive
  in SWA/SWAJ. The user's INPUT TEST 1/2 screenshot (2026-09-27) confirms
  Laser/Torpedo for both seats and `--` for Gunner VR Button/Start. This is
  in-game layout evidence, not proof that two controllers were exercised.
- **Model 2 joystick games:** the second joystick/actions use IN.2 with the
  same per-game logical translation as P1. This includes VF2, Fighting Vipers,
  Last Bronx, DOA, Sonic Championship, Dynamite Cop, Virtua Striker, Dynamite
  Baseball/97, Fighting Baseball, Air Walkers, Pilot Kids and Zero Gunner,
  including their catalogued revisions. Two-button cabinets leave button 3
  released. Virtual On's second stick belongs to P1; Royal Ascot II has no
  second gameplay panel or Start2. Neither is repurposed as P2.
- **Dynamite Baseball/97:** independent P2 Bat Swing drives `bat2`, 0..255;
  default Right Stick down, as for P1.
- **Model 2 guns:** independent cursor, calibrated ADC/serial coordinates and
  trigger for P2. Gun Yaw/Pitch default to Left Stick X/Y; Primary Fire shoots,
  Secondary Fire reloads only in serial gun cabinets (Virtua Cop/2, HOTD).
  Behind Enemy Lines uses Secondary Fire for its missile on both seats. Mounted guns
  Gunblade NY and Rail Chase 2 have no off-screen reload. The mouse remains
  P1-only. Red/blue reticles distinguish P1/P2; P2 appears when its controller
  is connected or its gun controls have been used.
- **Power Sled:** independent P2 Entry/Call signals, right/left pedals
  on Accelerator/Brake (R2/L2), ADC channels 5/7 with 0..255 travel.
  This game is absent from the SM2 workbook; its wiring follows MAME.
- Coin 2 / Start 2 move out of P1 to P2 Coin / Start. Default keys are 6 / 2,
  with Select / Start on the P2 controller. Start remains available for VF and
  existing applicable cabinets, not SWA's Gunner.
- P2 Test and Service are deliberately independent binding aliases for the
  shared machine lines: OR with P1, never toggle twice or invent a second
  hardware line. Defaults: F2 / L3 and F8 / R3, as for P1.
- Other enabled P2 signals inherit P1's **default gamepad** conventions only;
  no default gameplay keyboard bindings. Custom keyboard bindings remain allowed.
  Changing P1's bindings does not change P2's. Unsupported signals start unbound.

Wiring reference: local MAME `src/mame/sega/model1.cpp`, revision
`bd7e0b815842ec461e8ad2538d127f3332f5c96c`, `vf` / `swa` input definitions.
The native `Inputs` byte/ADC interface remains frontend-independent; no GUI or
host gamepad objects enter the core. These are local seats, not cabinet linking.
Air Walkers P3/P4 and Power Sled's extra network-check input remain outside this
P2 implementation. See [source audit](INPUT_AUDIT.md#model-2-p2-follow-up--2026-09-27).

Model 2 snapshots now include both guns' coordinates/off-screen flags. The
binary snapshot format is **2**: older format-1 machine states are rejected
with an explicit version error, not silently misread. NVRAM is unchanged.
This does not add complete Model 1 machine snapshots or claim the existing
Model 2 snapshots cover previously omitted devices.

Controller selection is saved as `controller_p1` / `controller_p2`:
`auto`, `none`, or device UUID plus ordinal. Auto assigns distinct devices;
disconnecting one leaves its seat empty rather than promoting the other pad.
Explicitly choosing a device already assigned to the other player removes it
from that player; a hand-edited duplicate cannot drive both seats. P1 alone
owns driving rumble and the gamepad emulator exit chord. Disconnect clears
cached buttons; returning to the library/opening another game retains device
assignments without carrying over the old cabinet's gear/analog ramps.

UUID identifies the controller model, not necessarily an individual serial
number. Identical controllers use enumeration ordinals; verify/reselect them
if their connection order changes across application restarts. The Android
single-pad/touch adapter remains P1-only; this implementation targets desktop
gamepads. Two-physical-gamepad gameplay/reconnect acceptance is still pending.

### Return to game menu / quit

Esc or Select + Start stops the current game using the same action as Stop:
save NVRAM, leave fullscreen and return to the library without quitting or
changing the fullscreen startup preference. It also works while paused.
Exception: when a romset was supplied on the command line and the window is
currently fullscreen, Close Game acts as Quit (including NVRAM saving),
returning to the terminal. Windowed CLI runs and games launched from the
library still return to the library. The current fullscreen state counts,
including an F11 toggle, not just the saved startup preference.
With no game running, the same command quits. The held-chord state is retained
when returning to the library: release and press again to quit, rather than
closing the application immediately with the same press.
Settings → Input → Emulator lists "Return to game menu / quit" after the other
commands, using the same binding row. Click it to edit the expression:
`return_to_menu = Escape, pad:Select & pad:Start` in config/input.conf.
Existing files without this entry inherit the default; an empty value disables
it. Keyboard exit keys are ignored while editing text or capturing a binding.

The exit action is edge-triggered and checked before the machine advances.
Hold Select then press Start: a completed chord does not send Start to the
game. A button pressed alone in an earlier frame cannot be retrospectively
consumed (Select alone may insert a coin, Start alone may start a game).

## Shared defaults and game meanings

One signal has one binding, regardless of the game. SM2-Emu positions are
retained where compatible with this rule. Gun and Sky Target fire are separate
assignable signals; their matching defaults do not couple later edits.

| Signal | Pad | Keyboard | Examples of routed functions |
| --- | --- | --- | --- |
| Button 1 / Kick | South OR L1 | J, E, Space | Remaining shared first-button cabinets, including Motor Raid Kick |
| Button 2 / Punch | East OR R1 | K, Q, R | Remaining shared second-button cabinets, including Motor Raid Punch |
| Button 3 / Guard / Jump / Hold / Barrier | West | L | Remaining shared third-button cabinets |
| Power Sled: Cancel Error | North | I | Power Sled only |
| Sky Target: Machine Gun / Missile | South OR R1 / East OR L1 | J, E, Space / K, Q, R | Independent Sky Target fire signals |
| Gun Primary Fire / Gun Secondary Fire | South OR R1 / East OR L1 | J, E, Space / K, Q, R | Shot / secondary action according to cabinet |
| View / Select 1–4 | Down, Left, Right, Up | Z, X, C, V | VR buttons, view changes, menu selections and zoom |
| H-Gate gears 1–4 | Right-stick diagonals | 1–4 | Direct gear selection |
| H-Gate neutral | West | 0 | Neutral |
| Gear Down | L1 | E | Sequential downshift / step down through an H-gate |
| Gear Up | R1 | Q | Sequential upshift / step up through an H-gate |

Virtua Striker, Wing War, Star Wars Arcade, Top Skater, Power Sled, Ski Super G,
Desert Tank and NetMerc now expose their actions as separate, game-prefixed
signals grouped by game, without section headings. Their physical defaults match the former Action
bindings, but later edits are independent. When loading an older binding file,
an absent dedicated entry inherits an explicit old Action 1/2/3 binding once;
saving writes the independent entries. No user file is rewritten on load.

**Virtua Fighter / Virtua Fighter 2 exception (both players, all VF2 revisions):**
Action 1 (South/L1) is **Kick**, Action 2 (East/R1) is **Punch**, Action 3
(West) is **Guard**, matching the tested SM2 Libretro fighting positions.
Keyboard keys remain bound to those same Actions, so they follow the new game
meaning without any config rewrite. Other fighting games and sequential
shifts retain their existing mappings; this is not a global Action swap.

Sega Rally (all revisions), Super GT 24h and Star Wars Arcade/SWAJ have
one View button: their `View / Select 1` also accepts the gamepad binding of
`View / Select 4`. With the defaults this is **D-pad Down OR D-pad Up**.
The GUI has no game list below View / Select 1. Keyboard bindings are unchanged;
reassigning View / Select 4's gamepad binding also reassigns the alternative.
Multi-view cabinets remain separate. Titles already using View / Select 4 for
their single view already accept D-pad Up and need no extra binding.

Face buttons and shoulders are OR alternatives, not a chord: either one
activates the same signal. R2/L2 remain analog pedals. The aliases preserve
the requested global action positions. Sequential shifts now use dedicated
**Gear Down / Gear Up** signals, default L1/R1 and E/Q. They also step through
an H-gate; its direct gear bindings and latching policy do not change.
South/East and J/K/Space/R no longer shift by default. Rebinding Action 1/2
cannot affect the gearbox, nor can rebinding Gear Down/Up affect actions.
E/Q remain existing Action keyboard aliases in other games; those bindings
and user customizations are not silently removed or rewritten. Racing games
read E/Q through the dedicated Gear bindings, not the Action aliases.
Motor Raid's Punch/Kick remain Action 1/2; Desert Tank's toggle now has its
own Shift signal. Neither consumes Gear Down/Up.
The new rows follow H-Gate in Cabinet P1, with disabled counterparts in
Cabinet P2 (no local second gearbox). Existing `signals-v3` files without
`gear_down`/`gear_up` inherit defaults; saving includes the new keys. Explicit
empty or custom entries are preserved. Legacy separate gear keyboard entries
are imported into the new signals, not merged into Action 1/2.
Virtual On uses the four action signals for its four independent shot/dash
functions, with both sticks kept independent.

Axis defaults use left X for steering/bank/handle/curving/swing; left X/Y for
gun aim and analog flight; right X for roll/inclining; and right Y
(downward half) for Bat Swing. Accelerator and Brake use R2/L2.
Star Wars Arcade and Wing War share two assignable entries, `Throttle Up`
(`throttle_up = KeyW, ArrowUp, pad:LeftZ+, pad:RightStickY+`) and
`Throttle Down`
(`throttle_down = KeyS, ArrowDown, pad:RightZ+, pad:RightStickY-`).
L2 OR right stick up selects Throttle Up; R2 OR right stick down selects Throttle Down.
By user request, the ADC polarity is inverted and the trigger assignments are
swapped; signal labels, right-stick Y and keyboard bindings remain unchanged.
The unchanged stick/keyboard bindings consequently have inverted ADC effects.
Both triggers read positive travel; gilrs right-stick Y is positive up.
Cabinet sampling subtracts Down from Up on the single throttle ADC. Released/equal inputs
give 0x80. Full Up/Down give 0xFF/0x01 for Wing War and 228/28 for SWA,
preserving each cabinet's ADC range with the same polarity. Alternatives use the
stronger input, not a sum; opposite directions compensate.
This centred rest is a gamepad adaptation, not MAME's minimum idle value.
It applies to both families, independently of Accelerator/Brake.
Existing binding files inherit these defaults for absent entries; GUI edits
save them normally, without requiring a reset of existing bindings.
Previously customized `wingwar_throttle_up/down` entries are accepted on load
and saved as `throttle_up/down`; there are no duplicate signals in the GUI.
Slide is split into Water Ski: Slide (left X, arrows/A/D) and Top Skater:
Slide (right X, U/O), matching their respective SM2-Emu defaults.
The GUI shows game-family names in parentheses below Analog Joystick X/Y;
below each Throttle entry it shows `(Star Wars Arcade (Pilot), Wing War)`.
In Cabinet P1 the Star Wars Arcade action rows name the Pilot; in Cabinet P2
they name the Gunner. The P2 analog-joystick note uses the Gunner name too.
Power Sled: Cancel Error is explicit. Sega Rally's analog Handbrake,
Virtual On's four triggers/dashes, separate Gun and Sky Target fire controls,
Water Ski's Set/Pitch controls,
and Ski Super G's two foot sensors and Select 2/3 have their own assignable
rows. The list is not filtered by game. The new signals inherit defaults in an
existing `signals-v3` file without overwriting it. Exact former stock Test,
Service and Elevation expressions refresh in memory; customized or empty
expressions remain unchanged. The existing `primary_fire`/`secondary_fire`
config keys now name the Gun signals; Sky Target has new independent keys.
Gun Secondary Fire also drives Behind Enemy Lines' missile but has no
game-list note in the GUI.
The Gun rows appear together immediately before the alphabetical game-specific
groups; Power Sled: Cancel Error is in its own game's group. The relative order
within each game remains unchanged.
Driving signals are ordered Steering / Bank, Accelerator, Brake, then all
H-Gate gears and neutral. Cars and bikes use the same `steering` binding.

Keyboard steering retains arrows and A/D; pedals retain W/S and Up/Down.
Independent roll/Top Skater slide/inclining use U/O. Flight Y, elevation and Wave Runner
pitch use G/T, avoiding simultaneous pedal or action activation. Flight X is
arrows/A/D. Virtual On uses WASD for the left stick and arrows for the right.
The generic Analog Joystick X/Y labels cover both Sky Target and Model 1
flight games, so they are no longer prefixed with one game's name.

Examples:

```ini
format = signals-v3
controller_p1 = auto
controller_p2 = auto
p2.coin = Digit6, pad:Select
p2.start = Digit2, pad:Start
p2.action1 = pad:South, pad:LeftTrigger
p2.analog_x = pad:LeftStickX
steering = keys:ArrowLeft/ArrowRight, pad:LeftStickX
gear1 = Digit1, pad:RightStickX- & pad:RightStickY+
gear2 = Digit2, pad:RightStickX- & pad:RightStickY-
gear3 = Digit3, pad:RightStickX+ & pad:RightStickY+
gear4 = Digit4, pad:RightStickX+ & pad:RightStickY-
neutral = Digit0, pad:West
```

- Commas separate alternatives. The strongest absolute value wins.
- `&` requires every source simultaneously above 0.5.
- `pad:Axis` reads a signed axis; `pad:Axis~` reverses it.
- `pad:Axis+` and `pad:Axis-` read one half of an axis.
- `keys:Negative/Positive` creates a signed keyboard axis.
- An empty value unbinds a signal; a missing entry retains its default.
- Full signed axes and keyboard pairs cannot be used in a digital chord.

The frontend uses positive stick Y for **up**; SDL3's native downward-positive
Y is inverted at the backend boundary. This is opposite to Libretro's Y
convention; the H-gate expressions account for that difference. Direct gear
selection latches when released. Conflicting direct selections leave the
current gear unchanged. Sequential shifting remains edge-triggered.

Virtua Racing / Virtua Formula do not use the Daytona H-gate encoding:
Gear Down/Up drive their native active-low Shift Down/Up switches. Both are
released at rest and on conflicting requests. Direct H-gate signals are
unused for those two games. VR4 remains independent of the shift switches.

Old-format files are migrated at startup after a backup is written to
`input.conf.pre-signals`. Existing keyboard directions, common actions and
hotkeys are adapted; controller assignments use the new defaults. The backup
is never overwritten. It is not a second active binding file.
`signals-v1` files are backed up separately as `input.conf.pre-players` before
conversion to `signals-v3`. P1 entries keep their names and values; `coin2` and
`start2` become `p2.coin` and `p2.start`. Old shipped 6/2 defaults gain the P2
Select/Start aliases; customized or explicitly empty bindings are preserved.
The migration is idempotent and does not overwrite an existing backup.
The short-lived Model 1-only `signals-v2` is backed up as
`input.conf.pre-model2-players`: its previously disabled, empty P2 Gun Yaw/Pitch,
Bat Swing and Accelerator/Brake entries acquire their gamepad defaults.
Nonempty custom values and all previously enabled P2 entries remain unchanged.
In v3, explicitly empty bindings stay empty on subsequent launches.
Merged actions retain the union of their old keyboard keys. The old
Space-for-view-change alias is intentionally not imported: Space now belongs
to Action 1, so importing it would also fire in Sky Target. Use V for View 4.
Previously shared flight-Y and body-axis keys use the independent defaults
above to avoid co-activating pedals or actions.

## Upstream reapplication boundary

- `input/signals/mod.rs`: sole public catalogue and defaults.
- `input/signals/expression.rs`: expression parser/evaluator.
- `input/sampling.rs`: physical binding evaluation and native touch signals.
- `input/cabinet.rs`: direct digital wiring and polling by cabinet family.
- `input/cabinet/analog.rs`: calibrated ADC roles, independent axes and
  stateful gear/cursor setup.
- `bindings.rs`: persistence, migration and hotkeys.
- `input.rs`: host event/device state, per-frame dispatch and ADC publication.
- `input/players.rs`: frontend-only device identity and independent seat assignment.
- `app.rs`: passes the identified ROM set to input and the touch overlay.
- `touch/`: emits the same native P1 signals, including signed axes.
- `gui/mod.rs`: displays the full catalogue and edits its bindings.
- Core `config.rs` / `memory.rs` / `system.rs`: native P2 lightgun values and
  publication; `savestate.rs` versions the resulting snapshot layout.

The legacy `Control` enum and `signals/routing.rs` have been removed.
Digital cabinet wiring is not fully represented in the ROM database, so
game-specific wiring remains in the cabinet module rather than changing the
core. This is a frontend refactor, not a Libretro adapter: the host-independent
core `Inputs` contract and all machine save-state formats remain unchanged.
The initial native refactor had no catalogue change. The subsequent workbook
functional-alignment checkpoint adds dedicated cabinet signals while retaining
the single GUI catalogue and `signals-v3` file format.
See [per-family equivalence checkpoints](INPUT_AUDIT.md#native-input-refactor--2026-09-27).

## Per-cabinet audit

See [INPUT_AUDIT.md](INPUT_AUDIT.md) for the reference revisions, corrections,
intentional binding differences and remaining capability limits. The tested
SM2-Emu Libretro workbook, metadata and runtime input path are authoritative
for Model 2, including where they differ from MAME. MAME is used for the
Model 1 cabinets and Power Sled, which are absent from that reference.

With the shared defaults, Indy 500 / Sega Touring Car / Over Rev now use:
Start → Start; D-pad Up → View 1; D-pad Down → View 2; R1 → Shift Up;
L1 → Shift Down. D-pad Left/Right do not start the game or change views.
Rebinding these signals in the existing list still works; no per-game binding
file or second GUI list was introduced.

Model 2 driving axes use SM2's 00..ff travel (including Daytona), with reversed
Bank and reversed Over Rev / Super GT pedals. VR / Virtua Formula retain their
20..e0 Model 1 calibration. Ski Super G uses Inclining on ADC 0 and reversed
Swing on ADC 1, overriding the old generated metadata. Wave Runner throttle
uses the reference's reversed positive half, 80 at rest and 00 at full travel.
Already calibrated NVRAM is not reset: a cabinet calibrated against the old
incorrect range may need its normal service-menu analogue calibration repeated.

Gun sticks move a persistent cursor like SM2's Analog Stick mode; releasing
the stick holds aim instead of snapping to centre. Only serial lightgun games
use off-screen reload. BEL's second action is Missile; Gunblade and Rail Chase
ignore Reload. ADC/lightgun ranges and Rail Chase 2 revision polarity are
set-specific, and the drawn crosshair uses the same stored cursor.

Automated tests check expressions, persistence/migration, analog travel,
game-dependent routing, H-gate latching and D-pad/pedal separation. They do
not substitute for physical-controller and in-game testing.
