# Star Wars Arcade Z80 DSB — integration contract

Checkpoint: 2026-09-27. Bus/UART/CPU and Layer II decoder implemented;
real firmware and opt-in 68000-to-DSB serial link verified. The board now produces
buffered MPEG audio in memory and is enabled through the normal SWA/SWAJ loader,
mixer and output-only mute. Manual listening/gameplay acceptance is pending.
Initial targets are `swa` and `swaj`, not Model 2/3 or the different DSB2 board.

## Reference and scope

Inspected local MAME checkout `/Users/andrea/dev/mame` at
`bd7e0b815842ec461e8ad2538d127f3332f5c96c`:

- `src/mame/sega/dsbz80.cpp` and `.h`: memory map, UART, playback registers.
- `src/mame/sega/model1.cpp`: SWA wiring and ROM regions.
- `src/mame/shared/segam1audio.cpp`: 68000 sound-board serial output.
- `src/devices/sound/mpeg_audio.cpp` and `.h`: Layer II decoder and history.
- `src/devices/machine/i8251.h`: UART interface and status bits.

The inspected DSB, Model 1 and MPEG files have no uncommitted changes. This is
a pinned local reference, not a claim that it equals today's upstream MAME.
SHA-256 of `dsbz80.cpp`:
`3c911d846d74136396d9779dd16e432c86fdf993fa0ec46d4718a9ee8e344ae0`.
SHA-256 of `mpeg_audio.cpp`:
`5ce25ab3bef827a847bf752f9b8b721d3b0b651b4ddfcb63f321dd9dbefe37db`.
Any adapted implementation must retain BSD-3-Clause attribution: R. Belmont /
Olivier Galibert for DSB, Olivier Galibert for MPEG, smf for i8251 as applicable.

## Existing components and decisions

- Use **`z80`**, the local adaptation in `crates/z80`, as requested (formerly
  `tgpulse-z80`; renamed after consolidation, not replaced with crates.io).
  It was selected as the destination of later Z80 consolidation; the original
  I/O-board consumer was migrated separately, after DSB implementation.
- Reuse its `Z80_io` live IRQ, acknowledge, emulated-clock and `CpuState` APIs.
  `model1io2/cpu.rs` provides an ownership/snapshot pattern, not a DSB bus:
  do not bring that board's CTC/PIO/SIO or daisy chain into this design.
- Before this checkpoint no i8251 device or MPEG decoder existed in TGPulse.
  The new DSB now has the bounded UART subset described below. The current
  `SoundBoard` UART uses immediate bytes, a receive queue and a single `tx`
  slot. Without a DSB it still ignores sound-side mode writes. The opt-in DSB
  link now adds a clocked transmitter; it does not turn the existing main-board
  receive/reply path into a timed serial implementation.
- Prefer a bounded Rust Layer II decoder adapted from the reference, with an
  isolated reference comparison before integration. This avoids a native
  production dependency and permits explicit decoder-history snapshots.
  The isolated decoder checkpoint below implements this choice. No new
  dependency has been selected, downloaded or installed.

## Wiring that must be preserved

`V60 -> existing 68000 sound firmware -> sound-board UART TX -> DSB UART RX`.

In MAME, `model1_state::swa` appends DSB RX to the sound board's output callback.
Its comment explicitly says the sound board filters commands the DSB must not
see. Do not fan out raw V60 commands or replace firmware filtering with a
hardcoded song-command table. The same output also reaches the main board.

In TGPulse the producer is the write to `0xc20001` in `SoundBoard::write8`.
The existing `tx: Option<u8>` is consumed by the main CPU; attaching the DSB by
taking that slot would steal replies and lose back-to-back writes. Introduce
ordered, timestamped delivery at the producer boundary, preserving the main
receiver. Define bounded buffering/backpressure and UART timing explicitly;
do not silently overwrite/drop DSB bytes or defer a whole frame of commands.
SWA does not use the DSB return wire in the reference, but the device should
expose serial output without owning a host callback or process-global state.

## Hardware contract

| Area | Reference behavior |
| --- | --- |
| Z80 clock | 4 MHz in MAME; explicitly documented there as an unknown physical clock/estimate |
| Memory | `0000..7fff` first 32 KiB of program ROM; `8000..ffff` 32 KiB RAM; no bank port in this map |
| Port decoding | Low 8 bits of the Z80 I/O address |
| `e0` | Playback command: 0 stop, 1 play once, 2 loop |
| `e2..e4` | Read playback byte position; write 24-bit start, high byte first |
| `e5..e7` | Write 24-bit end, high byte first |
| `e8` | Volume `(~data) & 0x7f`; gain denominator 128; bit 7 purpose unresolved in MAME |
| `e9` | Low two bits: 0 stereo, 1 left on both, 2 right on both; mode 3 must be investigated, not assigned a guessed meaning |
| `f0/f1` | i8251 data / status-control; RX ready drives Z80 IRQ; CTS asserted on reset |
| Serial clock | 500 kHz external clock; at x16 this is 31,250 baud, not the erroneous MHz unit in the DSB comment |
| MPEG | Layer II, MSB-first bits, no forced frame alignment, up to 1152 samples/channel/frame |
| Output | Reference DSB stream is fixed at 32 kHz, stereo; distinguish this from decoder-reported frame rate |

Start/end writes commit on their low-byte write. While playing they update loop
latches rather than the current segment. On decode exhaustion/error in loop
mode, use the latched start and optionally the nonzero latched end; prevent
infinite retry at an undecodable loop start. Position is internally in bits,
exposed in bytes. Stop clears queued decoded output. Do not infer that play
automatically clears decoder history or existing buffered samples: verify
start/stop/retrigger against the reference before changing its semantics.

ROM database already contains both DSB regions, but `build_model1` currently
discards them. Add optional owned DSB resources to `Model1Roms`, preserving
declared region sizes: program `0x20000` for both sets, MPEG `0x800000` for SWA
and `0x400000` for SWAJ. Both load the same two 2 MiB MPEG chips and program
`epr-16471.2`; the ROM region size is not the Z80 mapped window size. Missing
resources must not be replaced by a fake operational silent board.

## Frontend-independent execution and state

The new device owns CPU/RAM/UART/playback/decoder state and advances solely
from supplied emulated clocks. Expose input delivery and generated stereo
samples in memory; no filesystem, wall clock, audio device, GUI or global state.
Schedule 4 MHz DSB against 16 MHz V60 with retained instruction debt; produce
32 kHz samples with explicit phase. Preserve serial event order relative to CPU
execution. Convert/mix into the existing sound rate (`10 MHz / 224`) at the
system audio boundary, retaining fractional phase/history and clipping only
at the appropriate final mix. Output muting must not stop CPU/UART/decoding.

Snapshot inventory: CPU, 32 KiB RAM, UART mode/command/status, holding/shift
registers and bit timing, pending ordered transfers, IRQ state, cycle debt,
sample phase, partial start/end writes, current/loop addresses, bit position,
playback/volume/pan, decoded sample buffer and read cursor, decoder synthesis
history and cursors, and conversion history. Exclude immutable ROMs and host
resources; reconnect them without replaying writes. Test restore mid-UART
transfer, mid-audio-frame and at a loop transition with identical continuation.
The reference decoder has persistent synthesis history; saving only the MPEG
position is insufficient. This does not complete existing machine save states.

## Bounded implementation checkpoints

1. **Implemented in isolation:** DSB register/RAM bus and i8251 asynchronous subset, using
   the new Z80. Synthetic memory, IRQ, UART reset/framing, partial-register,
   looping-latch and snapshot tests; inspect firmware initialization/interrupt
   mode before claiming it is covered by the existing CPU tests. No fake music.
2. **Implemented in isolation:** Layer II decoding and deterministic history
   continuation, with mono/stereo/dual-channel reference comparisons and
   bounded truncated input. Joint stereo is explicitly unsupported.
3. **Implemented, opt-in:** real firmware probe and clocked 68000-output serial
   integration with isolated resources. Filtered commands and playback register
   programming verified; timing is instruction-boundary granular, not bus-cycle exact.
4. **Implemented, opt-in:** decoder and clocked buffered playback/loop handling,
   including partial-output snapshot continuation. Real SWA firmware/data produce
   PCM; no final mixer or user listening claim.
5. **Implemented; listening pending:** loader/system mixing and output-only DSB
   mute in the existing GUI list. SWA/SWAJ loader/boot and filtered command/mixed
   signal probes pass; manual gameplay and listening remain the acceptance step.

## Isolated implementation checkpoint — 2026-09-27

`crates/tgpulse-core/src/dsbz80.rs` owns the new Z80, RAM, playback-register
state, clock divider and instruction debt. The adjacent `uart.rs` implements
8N1 x16 pin-level receive/transmit, RX-ready IRQ, framing/overrun status,
read/error clearing, a TX holding register and ordered TX edge output.
The firmware's synchronous initialization preamble is accepted; operational
sync/parity/other baud formats, send-break and hunt commands fail explicitly.
CTS is fixed asserted as on this board; external modem-control wiring is not
claimed. Writing an already full TX holding register raises a diagnostic rather
than silently replacing a pending byte. This protective behavior is not a claim
that physical hardware reports such an error.

Direct inspection of local `epr-16471.2` shows IM1 at entry `0x0100`, UART
reset preamble `00,00,00,40`, mode `4e`, command `37`, and a DI/polling loop
starting around `0x01a3`. IM1 delivery is covered synthetically; the production
firmware was not executed at this checkpoint. Writes to unmapped `ea/eb` occur
during initialization: they remain ignored as in the reference map, not treated
as a fictional device. Unmapped reads return `ff`.

Serial events are timestamped in board clocks, but CPU accesses and externally
set RX pins remain instruction-boundary granular. A production serial scheduler
must handle instruction overshoot; no bit-exact comparison to MAME has yet been
performed. The reference's 4 MHz clock remains an estimate.

Playback ports expose start/end/loop latches, bit position, volume and pan.
No MPEG decode, sample buffer, fake position advance or automatic loop completion
exists yet. Unknown playback commands and pan mode 3 stop with an explicit
diagnostic. Soft reset stops playback while retaining addresses/pan as in MAME;
power-on creates zeroed state. UART hardware reset is distinct from its
internal return-to-mode command. RAM survives soft reset.

Typed, serializable snapshots include the implemented CPU/UART/bus state,
partial registers, in-flight serial bits, errors and scheduler debt. Restore
validates state before mutation and does not replay already delivered TX edges.
ROM ownership and callbacks are excluded. Decoder/mixer snapshots will be added
with those components; this does not establish complete machine save states.

Twelve new synthetic tests cover the map, partial/loop register writes, actual
Z80 port accesses, IM1, false starts, overrun/framing, mode/reset handling, TX
buffering, equal whole/sliced execution, combined mid-receive restore and
mid-transmit continuation, sticky faults and atomic rejection of invalid state.
Full offline workspace verification: 231 passed, zero failures.
No ROM or user settings were changed, and no machine/audio path selects this
board yet. General timing audit and migration of older Z80 consumers remain
final post-implementation activities.

## Isolated MPEG decoder checkpoint — 2026-09-27

`mpeg.rs` is a Rust adaptation of the pinned MAME Layer II implementation;
`mpeg/tables.rs` contains mechanically converted numeric tables. No native
decoder library, package installation or host resource is used in production.
MSB-first MPEG-1 Layer II sync search accepts mono, stereo and dual-channel;
sample rates are 32/44.1/48 kHz as reported by the header. CRC fields are consumed
but not validated, matching MAME. Other MPEG layers/versions and AMM are outside
this DSB subset. Bit positions follow the reference's consumed payload rather
than assuming byte alignment or including ancillary/padding data in audio.

### Reference anomaly and intentional differences

In the inspected MAME `build_next_segments`, the second loop repeats
`band < m_joint_bands` instead of processing the remaining joint bands. Rather
than reproduce or silently correct this apparent reference defect, the Rust
decoder rejects joint stereo explicitly. No fix to MAME was made. The two SWA
chips tested below decode entirely within the supported channel modes.

Bounds, header/allocation validation and error returns replace unchecked reads,
assertions and aborts. Late truncation commits neither partial PCM nor synthesis
history: callers can retry or choose another segment safely. MAME instead
returns false and can already have modified history on late failure. This is
an intentional safety difference; compare valid frames, not invalid-frame
side effects. Unsupported and truncated data are not reported as silence.

### State and lifecycle

The decoder borrows input bytes and synchronously returns a complete frame.
Its typed snapshot contains both synthesis histories/cursors; cosine tables
are derived and ROM/resource pointers are excluded. Restore validates lengths,
cursor alignment/range and finite history values before changing state.
`clear()` resets history; input seeking alone deliberately does not, permitting
continuous loop transitions. There is no host I/O, global state or process exit.

Calls are atomic at a decoded-frame boundary, so there is no suspended parser
operation to snapshot. The future DSB owner must serialize the returned PCM
buffer and its consumption cursor for a mid-output-frame save; that integration,
the board's stop/retrigger behavior and final mixer remain unimplemented.
Current decoder round-trips do not establish full machine save states.

### Verification

- Five new Rust tests: synthetic reference vectors; history round-trip, seek and
  clear; every bit truncation of a nontrivial synthetic frame; invalid headers
  and joint-stereo rejection; 500 deterministic malformed inputs.
- Eighteen synthetic streams, five frames each: 90 frames / 103,680 PCM sample
  frames. Cover mono/stereo/dual-channel, all three rates, four used allocation
  tables, grouped/ungrouped quantizers, all scale-factor selection modes, CRC
  presence, non-byte-aligned sync and history across frame boundaries.
- C++ oracle is the unmodified pinned MAME decoder built separately with the
  tiny compile-only shim in `tools/mpeg-reference/emu.h`. It is not linked into
  the emulator. Oracle PCM FNV-1a hashes, rates, channels and next-bit positions
  are stored in `mpeg/reference.txt` for the normal offline regression tests.
  **Tolerance for these vectors: zero**, matching every output hash/position
  on this macOS arm64 host with contraction disabled in the C++ build. This
  does not prove cross-platform floating-point identity on untested targets.
- Local user-owned SWA `mpr-16514.57` and `mpr-16515.58`, decoded separately
  from cleared history, match MAME for 3,640 and 3,639 complete frames:
  **7,279 frames / 8,385,408 stereo sample frames** at 32 kHz, exact PCM hashes
  and bit positions. Both chip tails end in incomplete data: Rust reports
  truncation where MAME returns false. This is not a song-boundary, firmware,
  board-mixing or gameplay test. No ROM/PCM data was added to the repository.
- Offline workspace tests: 236 passed, zero failures. The five MPEG tests also
  pass in release. No existing game path uses this decoder yet.

Reproduce the synthetic comparison using the existing compiler and local MAME:

```sh
mpeg_audit_dir=$(mktemp -d)
TGPULSE_MPEG_AUDIT_DIR="$mpeg_audit_dir" cargo test --offline -p tgpulse-core synthetic_reference_vectors
clang++ -std=c++20 -O2 -ffp-contract=off -I tools/mpeg-reference \
  -I /Users/andrea/dev/mame/src/devices/sound tools/mpeg-reference.cpp \
  /Users/andrea/dev/mame/src/devices/sound/mpeg_audio.cpp -o "$mpeg_audit_dir/reference"
for n in {0..17}; do
  "$mpeg_audit_dir/reference" "$mpeg_audit_dir/case-$n.mp2" > "$mpeg_audit_dir/case-$n.mame.txt"
  diff -u "$mpeg_audit_dir/case-$n.mame.txt" "$mpeg_audit_dir/case-$n.rust.txt" || break
done
```

The standalone oracle is only for controlled valid fixtures: the underlying
reference still asserts on some malformed headers. Use the Rust tests for
malformed-input safety checks. No firmware or user listening test was performed
in this checkpoint; production serial wiring, buffered playback and mixing are
the next bounded integration work.

## Firmware and filtered serial checkpoint — 2026-09-27

Historical checkpoint: superseded for the Model 1 production sender by the
[consolidated timing closure](MODEL1_ROADMAP.md#consolidated-timing-closure--2026-09-27).
The 68000 endpoint now lives in `sound::SerialState` and its one TX pin feeds
both main and DSB receivers. `i8251.rs` is shared rather than duplicated. The
DSB-owned transmitter described below is retained for isolated fixtures; a
production snapshot must now include the external serial state as well as DSB.

`SoundSystem::with_dsb` supplies ROM resources in memory and connects the wire
before the constructor's first 68000 instruction. Normal game constructors still
use `SoundSystem::new`, with no DSB attached. No loader/settings/GUI changes were
made in this checkpoint. The filtered producer remains the actual firmware's
write to `0xc20001`; `0xc20003` now configures its optional transmitter and
reports TX-ready/TX-empty backpressure. The existing main-board reply slot is
preserved, and raw incoming V60 commands are not mirrored to the DSB.

The source UART lives with the DSB receiver, clocked at 500 kHz. Execution follows
each 68000 instruction using the exact 10 MHz to 4 MHz ratio (2/5), retaining
the fractional remainder and Z80 instruction overshoot. This preserves ordered
transfers rather than batching a whole frame. Accesses remain at instruction
boundaries; there is no claim of exact bus T-state timing or UART-edge equivalence
to MAME. The existing constructor's reset/first-instruction clock accounting and
the immediate main-board UART path have not been redesigned.

The DSB snapshot now includes source holding/shift registers, both ends' partial
serial bits, the conversion remainder and diagnostic counts. A mid-transfer
restore produces identical continuation. It is still **not** a snapshot of the
68000/PCM board, its scheduler or the whole machine. Unsupported modes or full
source holding-register writes latch an observable error and stop the opt-in
pair. The probe checks that error after every run; **machine/frontend error
propagation is required before enabling this path in games**.

### Firmware evidence

Local user-owned `epr-16471.2`, SHA-1
`f12b214e6f195b0e5f49ba9f41d8e54bfcea9acc`, runs unmodified on the local `z80`.
After its startup delay it enters the UART polling loop, with IM1 configured
but maskable interrupts disabled, and initializes the volume to 59/128.
The isolated probe ran ten emulated seconds without a device fault.

Inspection of the SWA 68000 firmware's dispatcher at `0x2300` identifies
`AE 50 xx` as a path to its UART transmit routine at `0x1740`. This is **probe
input, not an emulator-side command table**: the real 68000 firmware still
parses it and emits `xx`, then the real DSB firmware programs the registers.
The controlled probe boots the pair for three sound seconds, sends each byte
with 10,000 sound clocks between writes, and runs another 10,000,000 clocks per
command. Sequence `00,01,00,02,00,03,00,05,00,08,00,09,00,11,00`:

| DSB command | Start byte | End byte | Mode | Pan |
| --- | ---: | ---: | --- | --- |
| `01` | 3358720 | 4145152 | Once | Stereo |
| `02` | 52296 | 1150151 | Loop | Left duplicated |
| `03` | 1166272 | 1885440 | Once | Left duplicated |
| `05` | 380040 | 1310855 | Loop | Right duplicated |
| `08` | 2378752 | 2786400 | Once | Left duplicated |
| `09` | 2068992 | 2322432 | Once | Right duplicated |

All 15 transmitted bytes were consumed, receiver status ended at `05` after
each command (no UART error), and `00` stopped playback. Register behavior was
observed in TGPulse, not compared against an executing MAME firmware trace.
An exploratory 100 ms command sweep did not consume every byte: the firmware
has blocking/fade delays and a single-byte hardware receiver. That sweep is not
lossless protocol evidence; no artificial receive FIFO was added to conceal it.
Song duration/position and loop completion remain untested because the decoder
is not yet driven by the board.

### Verification boundary and next step

Seven new synthetic tests cover actual 68000 writes, preservation of main replies
and existing PCM output, exclusion of raw main commands, source backpressure,
ordered bytes, fractional-clock slicing, in-flight restore and sticky pair errors.
Offline workspace: **243 passed, zero failures**; targeted release DSB tests:
**19 passed, zero failures**. Development release build passed.

A temporary, no-NVRAM-write SWA machine probe ran **1,800 frames in release**
with the opt-in pair. The real V60/68000 startup path emitted three DSB bytes,
all consumed without UART errors. These were stop commands; this establishes
boot/serial integration, not music playback or complete game correctness.
The debug machine probe encountered integer multiplication overflow in the
unchanged V60 string-operation code (`crates/v60/src/ops.rs`, register-28 update).
Release completed; the V60 arithmetic issue remains outside this DSB checkpoint
and should be reviewed separately rather than disabling overflow checks globally.
The isolated firmware and synthetic tests run in debug as well.

Next connect decoded-frame buffering and emulated sample advancement, testing
stop/retrigger/loop transitions and mid-buffer restore; then enable ROM loading,
error propagation, rate conversion, mixing and output-only muting. No listening
or complete DSB gameplay result is claimed yet.

## Buffered playback checkpoint — 2026-09-27

`dsbz80/audio.rs` connects the existing decoder to the register state.
`Board::with_mpeg(firmware, owned_region)` supplies immutable resources in memory;
`run_with_audio` advances explicit emulated clocks and emits timestamped normalized
stereo `f32` frames. At the reference's 4 MHz CPU rate, one frame is emitted every
125 clocks (32 kHz), including silence while stopped. Delivery occurs after each
CPU instruction, carrying existing instruction debt; it is not host-clock driven
or bus-cycle accurate. The short delivery vector is drained each instruction,
not an unbounded host queue. `run` discards samples but still advances playback,
allowing future output-only muting without suspending the device.

### Reference semantics retained

- Decode a complete frame on buffer exhaustion; expose its next-bit position
  through `e2..e4`. Position describes decoder progress, not the currently played
  PCM sample. A failed decode does not advance position, as in the reference.
- Stop clears buffered PCM. A play/retrigger changes the compressed position
  but does **not** discard already decoded PCM or clear synthesis history.
- Writes while playing still update the existing loop latches. Exhaustion in
  loop mode seeks the latched start, adopts a nonzero latched end, and otherwise
  retains the current end. Undecodable loops stop after at most one retry.
- Volume and pan apply to each emitted sample, including buffered data. Gain is
  `volume / 128`, with stereo or left/right duplicated, following MAME's output
  normalization. No extra clipping or resampling is introduced here.
- Soft reset clears buffer/phase and stops playback without clearing decoder
  history, matching the reference's `device_reset` history retention. A new
  board starts with cleared history. No disk or NVRAM operations occur.

The DSB output subset explicitly accepts stereo/dual-channel Layer II at 32 kHz.
Although the isolated decoder supports more formats, MAME's DSB indexes stereo
data at a fixed rate: mono and other rates fail explicitly here rather than
introducing an unverified conversion. Empty/out-of-range resources, invalid
headers and unsupported formats latch an observable board error. No-sync and
truncated segment tails follow the reference stop/loop path. As already noted,
the safe decoder retains history on failed frames; MAME can partially mutate it.
That intentional error-path difference remains, not a claim of malformed-input
identity. No new dependencies or changes to MAME were made.

### State and verification

The board snapshot now includes decoder history, current interleaved PCM frame,
read cursor and sample-clock phase, alongside CPU/UART/register/scheduler state.
Immutable ROM bytes, derived cosine tables and callbacks remain excluded. Restore
validates buffer shape/cursor, phase, history and resource presence/size before
changing any board state. The caller must provide the **same ROM contents**;
matching size alone is not a content identity check. Previously delivered samples
are not replayed. Existing 68000/PCM and complete-machine snapshots remain outside
this work.

Nine additional synthetic tests cover exact PCM/gain, position ports, dynamic
pan/volume, stop/retrigger/reset, same- and different-segment looping, nonzero/zero
loop-end latches, malformed tails and sticky errors, whole/sliced clocks,
discarded output, atomic snapshot rejection and serialized continuation both
mid-buffer and across loop boundaries. **252 workspace tests pass**, and **28
targeted DSB release tests pass**; development release build also succeeds.

A temporary in-memory probe used the unmodified SWA firmware and the two local
MPEG chips in their declared offsets, padded to the SWA 8 MiB region. Each fresh
board booted for three emulated seconds, then received one byte through the
500 kHz sender/UART path and ran for 60 seconds. Commands `01,02,03,05,08,09`
each produced exactly 1,920,000 output frames, with nonzero signal. `01,03,08,09`
ended in stopped mode; `02,05` continued looping. Output peaks ranged from
0.236799 to 0.379702. Restoring the two looping runs into fresh boards reproduced
the next 32,000 timestamped stereo frames **exactly** for each run. A separate
`10` command did not start playback from idle; its contextual meaning was not
inferred from the command number.

These are signal/firmware/continuation checks, **not listening, full-game playback
or full-machine save-state proof**. They inject DSB-side commands; the preceding
checkpoint separately verified their filtered 68000 path. No ROM, PCM capture or
user save/settings data is stored in Git. The next milestone is supplying loader
resources to the connected pair, forwarding errors, rate conversion to the
existing mixer and exposing its output-only mute, followed by SWA/SWAJ listening.

## Loader, mixer and frontend checkpoint — 2026-09-27

The normal Model 1 loader preserves optional owned `DsbRoms` and constructs the
complete sound pair for SWA/SWAJ before the first 68000 instruction. Regions
remain 128 KiB firmware plus 8 MiB (SWA) or 4 MiB (SWAJ) MPEG, preserving fill
and offsets. Missing or wrong-sized Model 1 DSB chips now fail explicitly;
other missing-file behavior and Model 2 selection remain unchanged. This is
presence/size validation, not a new hash audit.

The actual 68000 firmware's serial output still feeds the Z80 firmware; no
host-side song table or direct V60-to-DSB shortcut was added. `Model1System::Error`
wraps I/O-board and DSB failures. DSB faults stop the sound pair and propagate
through `run_slice`/`run_frame` to the existing GUI pause/error dialog and debugger
reporting, rather than continuing silently with a failed audio device.

`sound/dsb.rs` converts 32 kHz to the existing `10 MHz / 224` output using causal
held-sample integration, as in the FM path. Integer 20 MHz ticks represent both
periods exactly: 625 per DSB sample, 448 per mixed output. Samples produced during
Z80 instruction overshoot remain pending until their emulated time arrives.
Fractional DSB volume is retained in integer accumulators; only the converted
result is rounded to PCM16 units. Existing MultiPCM/FM gains remain unchanged;
DSB is added at unity (MAME's route gain), before final clipping. This is **not
MAME's resampling filter** or a claim of frequency-response equivalence. Listening
may reveal a need for resampling/balance refinement.

Settings -> Audio adds **Mute DSB (MPEG)** only for the attached Model 1 board,
in the existing list. `mute_dsb = off` is the default; missing keys leave audio
enabled, so existing settings need no manual edit. Changes persist through the
normal settings writer. Muting does not halt CPU, UART, decoder, clocks or
conversion, and never boosts other sources. Pending mixed core output is cleared
on a mute change; already submitted host samples may remain audible for the
device latency. No click-free transition is claimed.

`snapshot_dsb_path`/`restore_dsb_path` include the board plus conversion time,
held sample, accumulated area and future samples. Restore validates the converter
and its clock relationship to the board before committing. ROMs and mute
preferences remain external. A complete restore must also restore matching
68000, PCM, FM and scheduler state; tests restore FM's phase alongside the DSB
path. These APIs do **not** complete the existing full-machine save format.

### Verification and listening handoff

- **260 workspace tests** and **36 targeted release DSB tests** passed; the
  development release build succeeds. New tests cover converter continuation,
  mute/unmute state equality, clipping, board selection/error propagation and
  missing/wrong-size resources. Settings round-trip tests include the new key.
- Normal loader/constructor probes ran **1,800 release frames each for SWA and
  SWAJ** without DSB faults. No user NVRAM was loaded or saved. Startup emitted
  only stop commands, so boot alone does not establish music playback.
- A separate normal-constructor sound probe fed `AE 50 xx` into the 68000 for
  both sets. With MultiPCM/FM muted, `01,02,03,05,08,09` each produced nonzero
  DSB mix; interspersed `00` commands produced silence. Each measured second
  contained 44,642 or 44,643 stereo frames, consistent with the fractional rate.
  All 15 filtered bytes were consumed without UART errors. This verifies
  firmware -> DSB -> mixed signal, not a user listening session.
- Headless CLI smoke: VR, VF, Wing War, SWA and SWAJ each completed 120 frames.
  Build/loader/boot checks are distinct from gameplay and host audio quality.
- No dependencies installed, runtime settings/ROMs/saves changed, toolkit/current
  deployment or publication. The existing `block 0.1.6` warning and previously
  recorded V60 debug overflow remain outside this change.

Use `tgpulse.dev --rom swa` or `tgpulse.dev --rom swaj` (development binary
`target/release/tgpulse`). To isolate listening, mute MultiPCM 1, MultiPCM 2 and
FM while leaving DSB unmuted, then invert that selection. **Manual gameplay,
transitions, long-running music and subjective quality/balance remain pending.**
