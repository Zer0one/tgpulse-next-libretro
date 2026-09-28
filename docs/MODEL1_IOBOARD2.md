# Model 1 I/O board 2 — implementation checkpoint

Status: 2026-09-27. **Advanced I/O board selected for Wing War World/US/Japan.
Reference DPRAM replay, integrated startup, input delivery and EEPROM reload
checks pass.** This is a bounded step of the [Model 1 roadmap](MODEL1_ROADMAP.md),
not complete gameplay/peripheral validation. R360 and NetMerc remain deferred.

**Initial target: Wing War World/US/Japan.** NetMerc is a later, independent
milestone, not a prerequisite for enabling or validating Wing War. The official
[MAME driver](https://github.com/mamedev/mame/blob/master/src/mame/sega/model1.cpp),
checked on 2026-09-27, marks NetMerc `MACHINE_NOT_WORKING`, while the Wing War
variants have flags `0`. These are declared statuses, not new gameplay evidence.
Keep the shared board reusable; handle R360 motion/drive and NetMerc tracking
as separate cabinet-specific work. Do not infer that the common board alone
will make NetMerc playable.

## Reference and author style

Inspected local MAME revision `bd7e0b815842ec461e8ad2538d127f3332f5c96c`;
the following reference files had no working-tree changes:

- `src/mame/sega/model1io2.cpp` and `.h`: board map, firmware and pin wiring
  (BSD-3-Clause, Dirk Best).
- `src/mame/sega/model1.cpp`: Wing War, R360 and NetMerc machine connections.
- `src/mame/sega/315_5338a.cpp`: shared I/O controller (BSD-3-Clause, Dirk Best).
- `src/devices/machine/msm6253.cpp`: ADC shift/latch behavior (BSD-3-Clause, AJR).
- `src/devices/cpu/z80/tmpz84c015.cpp`: integrated CPU peripherals and interrupts
  (BSD-3-Clause, hap).
- `src/devices/machine/z80ctc.cpp` and `.h`: four-channel timer/counter and
  interrupt service (BSD-3-Clause, Wilbert Pol; based on Tatsuyuki Satoh's work).
- `src/devices/machine/z80pio.cpp`: parallel registers, direction/mask words and
  interrupt service (BSD-3-Clause, Curt Coder).
- `src/devices/machine/z80sio.cpp`: serial registers, FIFO/error flags, vectors
  and daisy-chain priorities (BSD-3-Clause, Curt Coder, Joakim Larsson Edstrom).
- `src/devices/machine/z80daisy.cpp` and `src/devices/cpu/z80/z80.lst`: inspection
  of interrupt arbitration and RETI/IFF behavior (not copied into a CPU core).

Follow TGPulse's existing `model1io`, `sound` and `sound2a` separation: execute
original firmware on the CPU, put address decoding and cabinet wiring in the
board, keep reusable chips separate. Do not synthesize successful handshakes or
create another frontend input list. The chip reference notices and terms are
retained in the source headers and [license notice](../LICENSES/MAME-BSD-3-Clause.txt).

## First checkpoint — shared chips

- Extracted `sega3155338.rs` and `msm6253.rs` from the first-generation board.
  They are crate-private and already used by `model1io.rs`, not unused stubs.
- The 315-5338A exposes register reads and write effects (parallel output mask
  or one host byte). The board applies the effects immediately and supplies
  input reads, host memory decoding and physical output connections.
- Host addresses remain 16-bit inside the chip. The existing board, not the
  chip, wraps them to its 2 KiB MB8421 dual-port RAM.
- Corrected the seven power-on output latches from `00` to `FF`, matching MAME.
  Construction does not issue output callbacks; register writes/direction
  changes do. Command/status behavior and existing cabinet wiring are retained.
- Added 14 ROM-free tests: controller registers, direction transitions, serial
  transfers, all 256 ADC values, channel selection, first-generation memory and
  digital wiring, immediate outputs, and EEPROM write/read through board pins.

The controller still completes transfers immediately and returns status `08`
(finished, active-low ACK), like the inspected MAME implementation. No serial
transport timing/slave mode was added. ADC conversion timing remains unmodeled.

## Second checkpoint — advanced bus and CTC/IRQ peripherals

Historical boundary: the following describes this checkpoint before CPU and
SIO/PIO integration. Its pending items are superseded where noted in the fourth
checkpoint below; FPGA, diagnostic LCD and external-watchdog gaps remain.

- `model1io2::Bus` owns the advanced ROM/RAM/register map and physical board
  wiring. Its constructor requires the complete 64 KiB firmware supplied as
  bytes; only the lower 32 KiB is mapped. It does **not** execute that firmware.
  Board-1 wiring remains unchanged and shared chips are reused rather than
  copied. The new `Inputs` structure describes physical pins, not GUI bindings.
- `z80ctc.rs` implements four timers/counters, prescalers 16/256, zero constant
  meaning 256, software reset, selected external edges, periodic reload, vector
  selection, nested priority, acknowledge and RETI service release. As in the
  inspected MAME implementation, running timer reads use remaining/prescaler + 1,
  including the exact reload boundary. External counter clocks are supplied
  explicitly; separately clocked counter inputs are not configured on this board.
- CTC advancement uses integer emulated CPU clocks, not wall time. Both ZC edges
  are delivered with offsets within the time slice, including channel 3 for the
  TMPZ84C015. High pulses last one CPU clock. Equal-time events are delivered in
  channel order. A future SIO must consume this timing, not just a pulse count.
- `tmpz84c015.rs` decodes CTC ports and their high-byte mirrors, implements the
  six IRQ priority orders and internal watchdog registers/deadline. Reserved
  priority values 6/7 retain MAME's explicitly guessed mapping, not a new claim
  of measured hardware behavior. WDTOUT is exposed as a pin, not forcibly wired
  to CPU reset/NMI.
- The bus exposes CTC acknowledge/RETI operations for the eventual CPU adapter.
  Other device IRQ states can be arbitrated by the peripheral component, but
  the bus currently has only CTC as an implemented interrupt source.
- Unsupported SIO/PIO port accesses, the Virtua Cop FPGA window and diagnostic
  LCD writes report errors. Unmapped reads return open-bus `FF`. The external
  MB3773 clock pin is retained, but its timeout/reset behavior is not implemented.
  DSW2/DSW3 are retained as inputs but await the PIO implementation.
- Peripheral snapshots include RAM/DPRAM, EEPROM protocol state, input/output
  latches, ADC phase, CTC deadlines/pulse/IRQ state and watchdog/priority state.
  Firmware and host objects are excluded. Restore checks timer/priority structural
  invariants and copies state without replaying port writes. This is an internal
  component snapshot, **not** a versioned/compatible full machine save state.

The unsupported diagnostic LCD write error can occur after its accompanying
EEPROM/output latch changes, as those pins are driven by the same port write.
A CPU adapter must stop/report that unsupported access, not ignore the error
and continue under the assumption that the missing peripheral worked.

### Third checkpoint — isolated CPU adaptation

At this checkpoint `crates/z80` was a separately named `tgpulse-z80` adaptation
of the installed MIT-licensed `z80` 1.0.2, preserving authors and source formatting.
See its [provenance and patch scope](../crates/z80/README.md). The registry copy
and `model1io.rs` consumer were initially unchanged. Subsequent consolidation
migrated that consumer, removed the registry dependency and renamed the local
package to `z80`. This is not a new CPU implementation or a global Cargo-cache
patch. No software was downloaded or installed.

1. Optional live IRQ and acknowledge hooks select the daisy-chain vector
   **when the CPU accepts the interrupt**, not when INT rises. DI/EI, HALT,
   NMI priority and changes delivered by the elapsed-clock callback are tested.
   The old fixed-vector API remains usable through default hook methods.
2. Canonical `ED 4D` restores IFF1 from IFF2 and notifies RETI exactly once;
   `ED 45` RETN does not release daisy-chain service. EI's delay now applies
   only to maskable IRQ, rather than also delaying NMI as the original did.
   MAME's inspected `reti` and `check_interrupts` behavior is the reference.
3. Explicit typed snapshot/restore covers all mutable production CPU fields,
   including alternate registers, flags, interrupt latches, HALT and internal
   memory pointer. Invalid mode/delay/pending bits are rejected before mutation.
   No raw struct bytes, ROMs or host resources are serialized, and restoring
   registers does not replay I/O callbacks.

18 ROM-free integration tests pass, including identical CPU/bus continuation
from 16 points across an LDIR transfer, IM2 service, port I/O and HALT. This is
not full machine serialization. The CPU remains instruction-stepped: clock
callbacks occur before IRQ sampling and after interrupt entry, with RETI and
port effects occurring during instruction execution rather than at exact
T-states. IM0 timing, undocumented RETI aliases, asserted-NMI semantics and
big-endian-host portability are outside the verified boundary.

The subsequent checkpoint below connects those hooks to the advanced-board
bus. Unification with the original Z80 consumer remains a later, separately
tested roadmap checkpoint.

## Fourth checkpoint — CPU/bus wiring and bounded SIO/PIO

- `model1io2::IoBoard` owns the CPU, bus and integer cycle debt. `run` accepts
  requested board clocks and delivers serial output pin events synchronously
  with emulated timestamps. No desktop loop, host serial port, filesystem or
  wall clock is required. Inputs and firmware are supplied by the caller.
- Unsupported reads/writes latch the first error. The current instruction can
  have partial effects, but subsequent bus effects and instructions are stopped;
  later `run` calls return the same fault until reset or state restoration.
  Faulted states are diagnostic, not a way to skip the unsupported operation.
- CPU, RAM/DPRAM, EEPROM, I/O chips, CTC/SIO/PIO state, input pins, elapsed clocks,
  instruction overshoot and a possible fault are serialized together. ROMs,
  host resources and already delivered output events are excluded. Restore
  validates component invariants before mutation and does not replay outputs.
  Soft reset retains RAM, EEPROM, external inputs and the elapsed time origin.
- PIO modes 0/1/3 implement data/control registers, direction/mask sequencing,
  bit conditions and interrupt service. DSW2/DSW3 feed ports A/B. Mode 2
  bidirectional operation is explicitly unsupported. Mode 0/1 strobes have
  independent tests, but no handshake is invented for physical DIP switches.
  Unlike the reference's unqualified pending scan, acknowledge selects an
  **enabled** pending port so a disabled request cannot steal another's vector.
- SIO implements two independent asynchronous channels: register pointers,
  RX FIFO depth 3, first/all-character IRQ modes, parity/framing/overrun flags,
  modem-status latches, CTS/DCD auto-enables, TX holding/shift state, RTS/DTR,
  break handling and vector/service arbitration. CTC2 clocks A and CTC3 clocks
  B; RX consumes rising edges and TX falling edges in chronological order.
  TX buffer-empty and all-sent are distinct conditions, not constant ready bits.
- Async framing supports RX 5–8 bits and TX 6–8 bits, parity, divisors 1/16/32/64,
  and 1/1.5/2 stop bits (1.5 stop at x1 is rejected). Sync/SDLC, five-bit TX,
  DMA/Wait-Ready operation and nonzero channel-A WR2 mode selection fail
  explicitly. Format/control reconfiguration during an active transfer is
  also rejected instead of silently corrupting the pending frame.
- This is **not full SIO fidelity**: TX uses a bounded frame serializer, not
  MAME's delayed half-bit output pipeline. Sub-instruction register effects,
  exact break/modem transition timing and external link/tracking/motion devices
  still need verification. No external peer is synthesized by the board.

21 new ROM-free tests cover peripheral sequencing and actual CPU bus access,
nested CTC/SIO/PIO arbitration, stop/error behavior, timestamped serial output,
and snapshot continuation during IRQ/RX/TX. Whole runs and one-clock slices
produce identical tested state/events. This does not establish full machine
save states or prove the original firmware handshake.

### Initial firmware probes (isolated, no main board)

Loaded exact firmware bytes from the existing user-owned ZIP, verified against
the MAME SHA-1s below. Blank EEPROM, default board pins and DIP switches, no
main-board requests or serial peer were supplied. No firmware patches applied.

| Firmware | SHA-1 | Observed boundary |
| --- | --- | --- |
| Wing War `epr-16891.6` | `3079397c7241c1a6f494fa310faff0989dfa04a0` | At 10,000,002 CPU clocks (~1.017 s), PC `0835` in the `0830` polling loop: RAM `F080=01`, waiting for `02`; watchdog output asserted. No completed handshake established. |
| NetMerc `epr-18021.6` | `bf5b9aad99c0f8f5e262e0855796f39119d11a97` | Stops at instruction PC `03A9`, next PC `03AC`, elapsed `121479`: port-F write at memory `8005` accesses the unsupported diagnostic LCD. |

Both runs left DPRAM zero and EEPROM clean. The two serial pin events per run
are not evidence of serial communication or a working external device. Probe
sources/binaries and extracted firmware were confined to a temporary directory;
no permanent one-off runner or ROM data was added to the repository.

The short Wing War probe above is superseded by the longer comparison below:
it sampled an initialization phase before it had time to finish, not a hang.
NetMerc's LCD fault remains recorded but is not a Wing War blocker.

### Wing War EEPROM initialization comparison — 2026-09-27

Compared the same `epr-16891.6` firmware with the existing MAME executable
`0.289 (mame0289-767-g39dc789dcd5)`, using a fresh temporary NVRAM directory,
default board switches and no external serial peer. MAME ran the full machine;
TGPulse ran the isolated advanced board with blank EEPROM and no host requests.
These are different machine contexts, not a full-system equivalence test.
The source reference revision listed above is newer than this runtime binary.

The firmware state dispatch at `0990` enters `09B6`, sets `F130=01` and
increments `F080` to `01`. While the foreground polls at `0830`, the handler at
`09C0` calls `11FE`/`130F` to read the EEPROM. It increments `F080` again at
instruction `09CF` only after `F130` returns to zero. This first wait therefore
does **not** require a main-board request or an external serial response.

| Observation | TGPulse isolated board | MAME full machine |
| --- | --- | --- |
| `F080: 01 -> 02` | 21,305,943 clocks, ~2.167352600 s; next PC `09D0` | ~2.167353312 s; write observed with PC `09D0` |
| EEPROM transfer state at that transition | `F130/F132/F134/F136=00`, `F13A=40` | Same |
| 64-word destination at `F180..F1FF` | 128 bytes `FF` | Same |

The approximately seven-clock difference between these observation points is
not a general claim of cycle accuracy: the CPU and tap observation boundaries
differ. What is established is the same completed blank-EEPROM initialization
path, rather than an indefinitely stalled firmware. The earlier 10,000,002-clock
probe (~1.017 s) was too short to observe it.

An isolated 20-second TGPulse run completed without a bus fault, remaining at
`F080=02` with zero DPRAM in the absence of main-board requests. The fresh MAME
three-second trace continued to `03`/`04` with its main CPU present. Neither
observation validates the TGPulse full-machine handshake, real controls, EEPROM
write persistence, nonblank EEPROM contents, or rendering/gameplay. No diagnostic
LCD access was needed on the tested normal Wing War initialization path.

Tests used temporary probes and MAME Lua memory taps; no tap replaced read/write
data and no firmware or user NVRAM was patched. `SDL_VIDEODRIVER=dummy`,
`-noreadconfig`, `-noplugins`, `-video none`, `-sound none`, explicit ROM paths
and isolated configuration/NVRAM directories avoided desktop/user settings.
The installed MAME binary still warns about its `315-5711.bin` and LCD font
`BAD_DUMP` entries; this is not validation against a newly built latest MAME.
An offline core rebuild and the 178-test workspace suite passed (one opt-in
ROM-dependent test ignored). Production emulation code was unchanged.

The subsequent DPRAM/system checkpoint below completes that initial comparison
and enables the three base Wing War sets. NetMerc remains deferred.

### Wing War DPRAM and motherboard integration — 2026-09-27

Captured an eight-second fresh-EEPROM Wing War session from the same MAME binary
using observational Lua taps on V60 DPRAM writes and the I/O chip's serial
commands. Replayed the **619 host writes** at their recorded emulated times
against the isolated TGPulse board. No board response or readiness flag was
injected. All sampled firmware states agreed; the final **2 KiB DPRAM** and
**128-byte EEPROM image** matched byte-for-byte (EEPROM words serialized in the
existing little-endian format). This validates the recorded transaction stream,
not arbitrary asynchronous scheduling or full serial-link fidelity.

Observed host/firmware contract, with DPRAM byte offsets:

- Host writes `SEGA` at `1A..1D` and command `01` at `20`.
- Firmware reads its EEPROM, publishes its 128-byte image at `100..17F`, then
  signals ready (`21=40`, ~2.195464 s) and clears the command.
- Command `03` reads configuration; command `02` transfers the host's new image
  back to EEPROM. In this cold boot, the game writes defaults, the board advances
  through firmware states `03/04/05/02`, and the host resumes normal input polls.
- Analog input channels 0/1/2 appear at DPRAM `00/01/02`; digital IN0/IN1 at
  `08/09`. Both the reference replay and full-system probes use physical inputs,
  not direct writes into these reply bytes.

`model1board.rs` is the thin motherboard boundary: it retains the original board
and selects the advanced one only for `wingwar`, `wingwaru`, `wingwarj`. The ROM
generator/database adds `epr-16891.6` only to those three sets. The board clock
is 9,830,400 Hz versus V60's 16 MHz, with an integer fractional remainder;
instruction overshoot remains inside each CPU. Original-board startup behavior
is preserved. The advanced board is **not** seeded with a fabricated ready byte.
Existing frontend input channels and EEPROM persistence APIs are reused; no
new bindings, menu or configuration key was introduced.

Full-machine testing exposed a necessary timing correction: MAME's
`model1_state::dpram_r` charges one V60 wait cycle for each low-byte DPRAM read.
Without it, Wing War exhausts its I/O timeout shortly before the real ready
reply and displays `I/O BOARD ERROR`. The V60 bus now has a default-zero
`take_wait_cycles` hook, and the advanced-board system path charges the observed
read waits. Debugger reads, writes and unconnected high-byte reads do not charge
them. **Board-1 wait timing is intentionally unchanged** for this checkpoint;
auditing it is a separate follow-up, not a claim that its present timing matches
all MAME accesses. The later [DPRAM audit](MODEL1_ROADMAP.md#dpram-checkpoint--2026-09-27)
extends the same wait to board 1 after reference and regression checks.
No arbitrary boot delay or game-ROM patch was added.

The debugger/system run API propagates a board fault and stops advancement;
the desktop frontend pauses and reports the error instead of continuing through
unsupported hardware. The advanced-board snapshot at this new boundary includes
the fractional clock remainder. Resources, delivered serial events and transient
CPU bus-access context stay outside that state. Full Model 1 snapshots remain
unimplemented; future machine snapshots must use a completed execution boundary.

A debug-profile run also exposed an existing V60 indexed-address multiplication
overflow. Using 32-bit wrapping multiplication matches the reference arithmetic
and existing release behavior; a negative-index test covers all four scales.

Verification:

- 187 workspace tests pass; one opt-in ROM-dependent test ignored. Nine new
  tests cover board selection/ROM consistency, input routing, timing/debt,
  snapshot continuation, sticky faults, DPRAM read waits and indexed arithmetic.
  The existing NVRAM round-trip test now covers both board revisions.
- Offline release build passes. The pre-existing `block` future-compatibility
  warning remains. No dependencies were installed or downloaded.
- World/US/Japan pass integrated cold initialization; World at frame 600 and
  Japan at frame 1100 produce attract-mode 3D frames. Regional startup sequences
  differ (US/Japan show additional notices). These are software-rendered debugger
  frame captures, not a desktop gamepad/audio or long-play session.
  A scripted World coin/start sequence also reaches the aircraft-selection
  screen; it is not proof of a complete playable flight.
- For all three sets: boot 600 frames, encode the real resulting NVRAM, load it
  into a fresh system, run another 600 frames, and verify identical EEPROM.
  Then change physical digital/analog inputs and run four frames: expected
  `42/B9/63` analog samples and `F7/EF` digital bytes reach DPRAM; no unimplemented
  V60 opcode is recorded in these runs. This is an in-memory lifecycle check,
  not a test of editing every operator option through the service menu.
- At 120 frames, `vr`, `vformula`, `vf`, `swa`, `swaj` retain byte-identical
  debugger CPU/FIFO state, main NVRAM and DPRAM-window dumps versus the previous
  checkpoint. No user saves, ROM ZIPs, settings, MAME checkout or toolkit install
  were changed. The development binary is `target/release/tgpulse`.

**Next:** manual base-cabinet gamepad/service-menu/audio/gameplay validation;
then the separately scoped Z80 consolidation and board-1 wait-state audit.
M1COMM/link play, R360 motion, diagnostic LCD and NetMerc are not validated by
this checkpoint and must not be inferred from the base Wing War startup.

## Future Libretro integration — Model 1 first

The integration target remains the standalone emulator; a Model 1-only Libretro
frontend is a future consumer, not part of this checkpoint. Preserve the current
frontend-independent chip boundaries and an in-memory serialization path. This
is a general criterion: execution scheduling, input delivery, audio/video output,
load/reset/unload lifecycle, resource ownership and dependency portability must
also remain frontend-independent where an integration touches them. Do not add
a Libretro adapter or a speculative framework to implement this policy.

The two shared chips now serialize their complete mutable state. The 315-5338A
snapshot includes all seven output latches, direction, command, serial data and
host address; the ADC includes the partially shifted sample. Callbacks passed
to register reads are not stored in either device. Write effects are consumed
synchronously by the board and must not be replayed during restore. Tests cover
restoring a configured serial transfer and a partially read ADC conversion.

This is **I/O-board coverage only**, not complete Model 1 save states.
The new bus snapshot additionally covers its RAM/DPRAM, EEPROM protocol state,
bank/output latches and implemented timer/IRQ state. The combined IoBoard
snapshot now additionally includes CPU, cycle debt and implemented SIO/PIO
state. Keep firmware and host resources outside serialized state; any future
emulated LCD/controller state must be added when that device is implemented.

The existing `savestate.rs` targets `Model2System`; do not treat it as an existing
Model 1 implementation. Eventually use an explicit, validated machine snapshot
format and test equal continuation after restore. Keep persistent NVRAM separate
from that snapshot and let the frontend own persistence; no implicit file writes
or dependency on a particular host clock/directory should be added to devices.

## Advanced board reference contract

### CPU and memory map

TMPZ84C015 at **9,830,400 Hz** (`19.6608 MHz / 2`), not the existing 4 MHz Z80.
Its Z80-compatible execution core alone is insufficient: it also integrates
CTC, SIO, PIO and a watchdog.

| CPU address | Device / behavior |
| --- | --- |
| `0000–7FFF` | Bottom 32 KiB of the 64 KiB I/O firmware |
| `8000–800F` | Shared 315-5338A registers |
| `8040` | Four active-low board buttons, JP4/JP3, EEPROM DO at bit 6 |
| `8080` | DSW1 |
| `8100–810F` | Virtua Cop FPGA interface; do not infer flight controls from it |
| `8200–8207` | MSM6253; channel is address bits 0–1, bit 2 mirrors |
| `E000–EFFF` | RAM labelled backup RAM in MAME |
| `F000–FFFF` | RAM |

At `8040`, idle buttons/jumpers/unused bit give `BF | (EEPROM_DO << 6)`.
MAME maps the backup area as ordinary RAM, not an NVRAM device here. Review
firmware use before changing persistent file formats or claiming battery-backed
behavior. It is distinct from the main board's 64 KiB NVRAM and the 93C45 EEPROM.

Internal I/O ports decode the low byte (high byte mirrored):

| Port | Peripheral |
| --- | --- |
| `10–13` | Four CTC channels |
| `18–1B` | SIO, `ba_cd` register order |
| `1C–1F` | PIO, alternate register order; DSW2/DSW3 on ports A/B |
| `F0` | Watchdog mode register, reset `FB` |
| `F1` | Watchdog control |
| `F4` | CTC/SIO/PIO interrupt priority |

CTC channels 2/3 clock SIO A/B. SIO A connects CN7; SIO B connects the CN8
debug terminal. There is also an external MB3773 watchdog. Do not return fixed
"ready" values for these peripherals just to advance the firmware.

The existing `z80` 1.0.2 dependency has memory/port callbacks, cycle stepping and
vectored IRQ entry, but its I/O trait exposes no interrupt acknowledge/RETI
callback. Define and test that boundary for daisy-chain interrupt service before
claiming TMPZ84C015 fidelity. Do not install another CPU dependency implicitly.

### 315-5338A cabinet wiring

Port letters correspond to register indices A=0 through G=6.

| Port | First-generation board (unchanged) | Advanced board |
| --- | --- | --- |
| A | EEPROM CLK7/CS6/DI5; bit 0 selects digital/analog bank | Digital IN0 |
| B | IN0 or DSW1 | Digital IN1 |
| C | IN1 or DSW2 | Digital IN2 |
| D | IN2 or DSW3 | Lamps / coin outputs |
| E | Drive read/write | Drive read/write and diagnostic LCD data latch |
| F | Lamps / coin outputs | EEPROM DI6/CLK5/CS4; diagnostic LCD control bits 0–3 |
| G | EEPROM DO7 / board buttons | Watchdog7; analog bank6; active-low comm-error LED5 |

Advanced-board bank selection changes **only the four analog channels**, not
the three digital ports. Diagnostic LCD support is a separate acceptance item;
do not conflate that panel with the main GUI. Preserve and test EEPROM edge
ordering against firmware instead of copying board 1's pin masks.

### Firmware and game connections

| Games | Firmware | ADC / board-specific connections in MAME |
| --- | --- | --- |
| Wing War / Japan / US | `epr-16891.6`, 64 KiB, CRC `a33f84d1` | 0=Stick X, 1=Stick Y, 2=Throttle |
| Wing War R360 | Same | 0/1=Stick X/Y, 2=constant zero; IN2 and drive output use R360 handlers |
| NetMerc | `epr-18021.6`, 64 KiB, CRC `5551837e` | 0=Stick X, **2=Stick Y**; other unconnected analog callbacks default `FF` |

These channel numbers describe the board pins, not new assignable GUI signals.
Keep any game-specific translation at the existing input/board boundary.
Virtua Cop uses another firmware and an FPGA upload/lightgun path; integrating
that Model 2 variant is outside this Model 1 milestone.

NetMerc also has a Polhemus tracking subsystem represented by an i386SX in
MAME's machine configuration. Its presence is a further integration concern,
not proof that MAME has complete tracking emulation. Wing War's M1COMM and R360
motion/drive behavior likewise remain separate from the base I/O board.

## Integration and acceptance sequence

1. **Done:** reusable chip implementations and board-1 regression coverage.
2. **Bounded implementation tested:** advanced decoding, CTC, watchdog, IRQ
   arbitration, CPU integration, PIO and asynchronous SIO subset. Unsupported
   peripheral modes/devices and serial timing limitations are listed above.
3. **Bounded reference comparison passed:** Wing War EEPROM initialization,
   DPRAM transaction replay and integrated cold boot. No firmware patched.
4. **Done for base Wing War:** select the advanced board/clock and load its
   `iocpu` region through the existing ROM database. R360 is covered by the
   follow-up below; NetMerc remains excluded.
5. **Automated checks passed; manual validation pending:** Wing War World/US/Japan
   digital/analog delivery and EEPROM lifecycle. Verify actual gamepad use,
   operator-menu edits and extended gameplay. Validate R360 cabinet behavior separately;
   NetMerc remains a later milestone. Do not alter existing bindings or user
   saves as a side effect of board integration.

Firmware now executes in the integrated base Wing War system. User configuration,
ROM archives and installed toolkit releases remain unchanged.

## R360 cabinet follow-up — 2026-09-27

`wingwar360` now reuses the advanced board, its 9,830,400 Hz clock and the existing
one-cycle V60 DPRAM read wait. The database and generator attach `epr-16891.6`
to `ioboard:iocpu`; it was previously omitted, causing a zero-filled fallback
even after changing board selection. The user's ZIP already contains this chip;
no ROM archive or firmware patch is required.

Reference: MAME `src/mame/sega/model1.cpp`, `wingwar360`, `r360_r` and `r360_w`.
The small `model1io2/r360.rs` state machine follows those handlers: commands
BF/BE/BA/B9 latch inverted 40, BD inverted 44, BC inverted 45, BB inverted 46,
AF inverted throttle; unknown commands retain the last response. IN2 reads that
latch. ADC channel 2 is zero; the already translated throttle input supplies
the AF command instead. Drive writes update the reply immediately on the bus,
including repeated commands, without sampling only the last output per frame.
This is MAME-style protocol simulation, not low-level controlboard firmware or
mechanical motion/safety simulation. Attribution is in the MAME license inventory.

The optional cabinet reply/throttle state is included in the existing board
snapshot. Restore rejects a base/R360 mismatch before mutating state and never
replays commands. Tests cover command replies, all 256 throttle values, unknown
commands, response continuation, and Z80 mid-program save/restore with identical
continued execution; existing board selection, firmware and DPRAM timing checks
also cover R360. These are device states, not complete Model 1 machine saves.

Fresh-NVRAM headless ROM probe: 1,800 frames, V60 PC `00003CBD`; captured output
shows the 3D attract sequence and INSERT COIN(S), replacing the earlier I/O BOARD
ERROR. No user NVRAM or settings were modified. The headless renderer capture
is not native Metal visual-quality acceptance. Real gamepad controls, safety
setup/start sequence, extended gameplay, audio and networking remain to validate.

Final checks: 314 workspace tests and offline release build pass. A second
fresh-directory run of the final binary reaches 1,800 frames for both base
Wing War (PC `00003A41`) and R360 (PC `00003CBD`). No commit/push or toolkit
release deployment was performed.
