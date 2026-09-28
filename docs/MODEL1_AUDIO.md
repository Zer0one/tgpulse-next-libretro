# Model 1 audio — integration contract and output mutes

## Current checkpoint (2026-09-27)

Scope: Rust YM3438 FM/DAC synthesis is now connected to the production MultiPCM
board, with deterministic clock conversion, mixing and a persistent output-only
FM mute. Device reference comparison and synthetic board tests pass; per-game
listening acceptance is still pending. The user-tested PCM/SCSP mutes predate
this new FM path.
Timing audit/Z80 consolidation are now completed as bounded milestones (see
the consolidated closure linked below). No native
production dependency or software installation was introduced. Model 2 is not
being redesigned: original Model 2 shares this MultiPCM/FM board and therefore
receives the same integration; the SCSP board is unchanged.

The [consolidated timing closure](MODEL1_ROADMAP.md#consolidated-timing-closure--2026-09-27)
replaces Model 1's instantaneous V60/68000 UART path with two clocked endpoints,
sharing `i8251.rs` with DSB. Arithmetic and serial-frame tests justify keeping
the existing chip clocks/converters. Model 2 retains its existing HLE link.
The 4 MHz DSB CPU clock
remains explicitly an estimate inherited from MAME, not a hardware measurement.

## Existing paths and reference

- `sound.rs`: 68000 at 10 MHz, two MultiPCM instances at 10 MHz, complete
  implemented YM3438 device at 8 MHz. PCM and FM are mixed together. The same
  board is used by Model 1 and original Model 2 titles such as Daytona.
- `sound2a.rs`: 68000/SCSP board. SCSP produces one combined stereo stream;
  its PCM/FM slots and DSP are not separate board outputs in this change.
- `Session::step` drains stereo samples into the host audio output. Master
  volume and host resampling are after chip generation/mixing.
- MultiPCM and SCSP already follow the author's Rust translation approach.
  `multipcm.rs` explicitly documents a literal port of the MAME implementation,
  retaining arithmetic and update order for comparison.

Inspected local MAME HEAD: `bd7e0b815842ec461e8ad2538d127f3332f5c96c`.
The reference is `src/mame/shared/segam1audio.cpp` plus `3rdparty/ymfm/src/`.
The board routes each MultiPCM at 0.5 and YM3438 at 0.30 per stereo channel.
These are reference gains, not proof of calibrated real-cabinet loudness.

Primary upstream sources:

- [MAME board](https://github.com/mamedev/mame/blob/master/src/mame/shared/segam1audio.cpp)
- [YMFM](https://github.com/aaronsgiles/ymfm), BSD-3-Clause, C++14, includes YM3438
- [Nuked OPN2](https://github.com/nukeykt/Nuked-OPN2), C, LGPL-2.1-or-later

The latter two license identifiers are from their source headers, not a legal
assessment of a future distribution. The new Rust register/timer/synthesis adaptation
retains YMFM attribution and the full notice in `LICENSES/YMFM-BSD-3-Clause.txt`.

## Chosen YM3438 approach and checkpoint history

Prefer an isolated Rust port of the required YMFM OPN2/YM3438 subset, with
origin/version/license notices and a narrow public device interface. This
matches the existing MultiPCM style and avoids introducing a C++ runtime/FFI
build into the production core. It is more implementation work than a wrapper,
so proceed in verified checkpoints rather than promising a one-shot port.

User constraint: adherence to the project's style is a preference, not an
absolute pure-Rust requirement. Do not port the whole YMFM framework just to
avoid FFI. Start with an inventory of the required subset and a bounded device
prototype. If porting entails disproportionate time, complexity or correctness
risk, present the isolated-wrapper alternative and obtain agreement before
changing approach; do not quietly turn this into a large rewrite or compromise
audio fidelity for language consistency.

A small C++ wrapper around unmodified YMFM is the lower porting-effort
alternative, but adds native build/ABI/target integration. Nuked OPN2 is another
reference candidate; it changes the timing integration and licensing choice.
Neither alternative has been added, downloaded or selected as a dependency.

Proposed checkpoints:

1. **Device boundary:** register/address banks, reset, status, busy, timers,
   key-on/operator state and an explicit integer chip-clock API. Replace the
   existing timer subset with one authoritative device; do not run competing
   timer implementations. Preserve driver polling and UART behavior.
2. **Synthesis/reference comparison:** envelopes, phase, algorithms/feedback,
   stereo routing, LFO, special channel modes and DAC behavior. Compare bounded
   synthetic register sequences with YMFM before game-driver tests. Rendering
   a sine wave alone does not establish YM3438 compatibility.
3. **Scheduling/mixing:** advance device state at emulated-time boundaries,
   honor writes between CPU instructions, and bridge native FM sample time
   (8 MHz / 144) to the existing MultiPCM stream (10 MHz / 224). Retain fractional
   phase/debt and reference gains; do not treat the two rates as identical.
   This device-local timing is required for the feature, not the deferred
   system-wide timing audit. Check slice-size-independent continuation.
4. **State/lifecycle:** reset and repeated construction, snapshots of all mutable
   chip state and scheduler/resampler history, mid-note/mid-timer round-trip
   with identical continuation. Exclude host resources and output preferences;
   do not claim complete machine save states or Libretro support from this.
5. **Integration:** add the FM mute only when FM exists; exercise sound tests
   and representative titles with each source isolated, then mixed. Separate
   sample/reference checks from manual listening, balance and gameplay tests.

Register/timer state and native-rate synthesis pass the bounded YMFM tests below,
including mid-note state continuation. Board scheduling and mixing are now
implemented; actual game audio acceptance remains separate. The sections below
retain evidence from earlier isolated checkpoints as historical results.

## YMFM inventory and feasibility checkpoint — 2026-09-27

The inventory is complete against the local MAME revision above, not a claim
about the latest online YMFM revision. No production dependency was imported
and no audio engine was changed during this checkpoint.

### Required subset

`ym3438` inherits the register/DAC interface of `ym2612`, but has its own output
generation. Its engine is `fm_engine_base<opna_registers>`: six channels,
24 operators, two outputs and 512 register bytes. The small YM3438 class is
not the whole implementation to translate.

| Source area | Retain for the Rust device | Exclude from the port |
| --- | --- | --- |
| `ymfm.h` | Integer helpers, envelope/key state, output and explicit state concepts | Debug WAV writer and generic external-memory interfaces not used by this chip |
| `ymfm_fm.h` / `.ipp` | Log-sine/attenuation/detune tables, phase/envelopes, SSG-EG, four-operator algorithms, feedback, timers/CSM, cache invalidation | OPM pitch/noise, OPL rhythm/two-operator paths and unrelated chip variants |
| `ymfm_opn.h` / `.cpp` | Six-channel OPN register decoding, frequency latches, key-on, LFO, special channel modes, YM2612 bus/DAC base and YM3438 output | Other OPN chips, SSG resampling and ADPCM board engines |
| `ymfm_adpcm.*` / `ymfm_ssg.*` | None of their synthesis engines | Entire devices; **SSG-EG inside FM envelopes still remains required** |

The eight full upstream files total 7,791 lines including comments, declarations
and unrelated chips; this is **not** the required Rust port size. Most retained
implementation lies in the shared FM engine, the OPN register block near the
start of `ymfm_opn.cpp`, and its YM2612/YM3438 section. Specializing fixed arrays
for this one chip avoids translating YMFM's generic multi-family framework.
This remains a substantial arithmetic port, not a small register-only patch.

### Integration differences found

- YMFM uses one shared nine-bit address latch. Data writes to the wrong bank
  are ignored; the current TGPulse timer stub keeps two independent latches.
- YMFM's inherited `read(offset)` returns status at offset 0 and zero at the
  other three offsets. TGPulse currently mirrors status at offsets 0 and 2.
  Cover this reference difference with explicit bus tests, not an assumption
  that both address ports necessarily expose status.
- Valid data writes set Busy for 192 chip clocks. The current subset has no
  Busy implementation. YMFM delegates deadlines/expiry to its host interface;
  its default no-op callbacks are insufficient for board integration.
- Timer B's initial interval includes the free-running divider phase;
  timer A can drive CSM key-on. Keep these in the same authoritative chip,
  replacing rather than layering over `Ym3438Timers`.
- YM3438 output must not use the YM2612 DAC-discontinuity function. Preserve
  intermediate clipping, signed nine-bit DAC behavior and stereo routing.
- Generation is at 8,000,000/144 Hz, distinct from MultiPCM's 10,000,000/224 Hz.
  Retain integer clock remainder and resampler history; the integer rate helper
  returning 55,555 is not an exact scheduling frequency.
- Constructor and reset are not interchangeable in the reference: the inherited
  reset delegates to the FM engine without explicitly clearing address/DAC
  fields. Test repeated reset and document any deliberate deviation.
- The inspected MAME board wires UART receive-ready to sound-CPU IRQ2 and does
  not wire the YM IRQ output to that line. Do not add such a connection merely
  because the device exposes IRQ state.

### Proposed component boundary and decision

Continue with a **bounded, specialized Rust port**. The inventory identifies a
separable subset and does not yet demonstrate that native integration is worth
departing from the existing MultiPCM style. This is not approval for an unlimited
pure-Rust rewrite: reconsider if the first differential checkpoint exposes
disproportionate translation/debugging effort.

Proposed ownership (names are a design, not existing modules):

- `ym3438`: register/address interface, owned operator/channel arrays, tables,
  timers, Busy, chip-clock advancement and stereo sample production. No host
  audio, GUI, filesystem, wall clock or process-global mutable state.
- `sound.rs`: sound-board bus wiring, 68000-to-chip clock remainder, conversion
  to the board sample stream, reference gain and output-only FM mute when ready.
- Device snapshots contain mutable registers, phase/envelope/feedback, LFO,
  latches, timer deadlines, Busy and clock remainder. Rebuild derived caches
  after restore. Board snapshots separately retain conversion history; host
  preferences/output queues are not a substitute for hardware state.

**First bounded milestone (now implemented below):** isolated register/timer/Busy/state
boundary with explicit chip clocks and compare synthetic traces to the pinned
YMFM reference. Include both banks and mismatched-bank writes, timer load/reset
and phase, constructor/reset distinction, and mid-timer restore with identical
continuation. Keep the existing production board unchanged until this boundary
passes. That milestone does not claim audible FM, complete synthesis or game
compatibility; full operator/synthesis comparison follows separately.

The fallback remains an isolated C ABI wrapper around unmodified YMFM, pending
user agreement. It avoids arithmetic translation but still needs clock/timer
callbacks, resampling, safe lifecycle/state handling and per-target native build
support. Building the unmodified OPN translation unit also brings definitions
for other chips: the standalone probe compiled `ymfm_opn.cpp`, `ymfm_adpcm.cpp`
and `ymfm_ssg.cpp`, although YM3438 itself owns no ADPCM or SSG generator.
Do not promise binary stripping or a portable Rust FFI build from this probe.

### Standalone reference evidence

Using the existing Apple C++ compiler in C++14 mode and the local YMFM source,
a temporary ROM-free program passed:

- Nonzero four-operator FM output from a synthetic register sequence.
- Byte-state round-trip during a note followed by 1,024 identical stereo frames
  (809-byte chip snapshot in this exact revision).
- Isolated left-only DAC output of 5,418 with right output zero.

The probe used the default host interface: it **does not verify timer/Busy
scheduling**, CSM expiry, Rust equivalence, board mixing, actual listening or
gameplay. YMFM's snapshot also does not include the caller-owned timer/Busy
deadlines. Its byte count is evidence, not a stable public save-state format.
Any wrapper must validate version/length rather than rely on the reference
decoder's zero-fill behavior on truncated input.

Temporary evidence: `/tmp/tgpulse-ymfm-inventory.k5MbsJ/probe.cpp` and `probe`.
No permanent runner, dependency installation or emulator rebuild was needed.

## Rust register/timer/state checkpoint — 2026-09-27

Implemented in `crates/tgpulse-core/src/ym3438.rs`, independently of `sound.rs`.
Public boundary: `new`, `reset`, `read`, `write`, `advance` (input chip clocks),
`irq`, `snapshot` and validated `restore`. No ROMs, GUI, host audio, CPU-specific
clock units, wall clock, paths or disk writes are required. State uses fixed-width
fields/owned arrays and existing serde support. This supports a future frontend
driving execution and owning state, but does not implement a Libretro adapter.

Implemented behavior:

- Shared address bank latch, wrong-bank rejection and Busy; four-port reads.
- 512 register bytes, shared high-frequency latches, normal key-request masks,
  DAC register state (not its output), status flags and device IRQ level.
- Timer A/B load/stop/reload, first-period B divider phase, writes during active
  periods, flag reset/enable and a pending channel-3 CSM key request.
- Free-running sample-divider phase and fractional input clocks; constructor
  versus reset semantics, including retained DAC/address/Busy state.
- In-memory subset snapshots, validation before mutation and rejection of zero
  or out-of-range timer deadlines. The format is not a stable disk ABI and must
  expand when synthesis state is added. No cache or host resource is serialized.

**Boundary deliberately retained:** normal key masks and pending CSM requests
are not operator key/envelope state. At sample boundaries the pending CSM latch
is cleared at the point where future operator preparation will consume it.
LFO, operator phases/envelopes, algorithms, feedback, DAC/FM sample output and
resampling/mixing remain unimplemented. The production board continues using
`Ym3438Timers`; it does not run both devices. Replace that stub only when the
complete device is ready. No FM mute is exposed yet.

### Reference comparison and reproducibility

`tools/ym3438-reference.cpp` is a small standalone oracle, not a production FFI
wrapper or Cargo build dependency. It compiles the pinned, unmodified local
YMFM source and supplies its required timer/Busy callbacks. It generates samples
to advance YMFM internal clocks but deliberately discards the audio. Both sides
use an explicit tie convention: expire timer A, expire timer B, tick sample,
then perform the caller's bus operation. This is a reproducible device contract,
**not verification of MAME's full scheduler or sub-cycle physical hardware**.

`crates/tgpulse-core/src/ym3438/reference.trace` stores oracle-generated hashes
for directed bank/Busy/frequency/DAC/reset/timer/CSM cases plus four seeded runs
of 4,096 operations each. Each seeded-run digest incorporates the normalized
state after **every** operation, not just its final state. Normalization covers
all register bytes, address/DAC state, four bus reads, IRQ, Busy and timer time
remaining, sample phase/counter, normal key masks and pending CSM.

The reference tool reads selected fields from the pinned 809-byte YMFM snapshot
to observe key requests and the internal sample counter. This is explicitly
revision-specific, guarded by the size assertion, not a promised YMFM save ABI.
Snapshot inspection invalidates reference caches; no synthesis-output equality
claim is made from this control-only trace. Rust tests need neither MAME, a C++
compiler, ROMs nor network access: they replay the checked-in reference results.

To independently regenerate and compare the oracle output (existing compiler
and local reference source required; never install them implicitly):

```sh
YMFM_SRC=/Users/andrea/dev/mame/3rdparty/ymfm/src
clang++ -std=c++14 -O2 -I "$YMFM_SRC" tools/ym3438-reference.cpp \
  "$YMFM_SRC/ymfm_opn.cpp" "$YMFM_SRC/ymfm_adpcm.cpp" \
  "$YMFM_SRC/ymfm_ssg.cpp" -o /tmp/tgpulse-ymfm-register-reference
/tmp/tgpulse-ymfm-register-reference \
  < crates/tgpulse-core/src/ym3438/reference.trace \
  | diff -u <(sed '/^#/d; /^$/d' crates/tgpulse-core/src/ym3438/reference.trace) -
cargo test --offline -p tgpulse-core ym3438
```

Nine Rust tests pass, including all 16 timer-B phases, divider wrap, timer-period
updates without restart, wrong-bank Busy behavior, cross-bank frequency latches,
reset versus power-on, invalid-state atomic rejection, the differential trace,
and a serialized mid-timer/Busy/CSM round-trip followed by identical continuation
with one-clock versus large advance slices. Full workspace validation passed
206 tests, with the existing one ROM-dependent test ignored. No new game/audio
test is claimed. The existing `block` 0.1.6 future-compatibility warning remains.

**Following milestone (now implemented below):** port the operator phase/envelope machinery and
four-operator channel output, with synthetic register/sample comparisons against
YMFM and mid-note continuation. Inventory any remaining LFO/special-mode/DAC
coverage explicitly before enabling the board path. Keep mixing and manual
game listening as subsequent gates; do not infer them from control-state tests.

## Native-rate synthesis checkpoint — 2026-09-27

The isolated Rust device now produces six-channel stereo FM/DAC frames. The
existing production sound board still owns the old timer subset: no game has
been switched to this device, and there is still no FM mute in the GUI.

Implementation in `ym3438/synthesis.rs` and `ym3438/tables.rs`:

- Integer phase/frequency, detune/multiplier, key scaling, ADSR envelopes and
  original log-sine/attenuation tables; no approximate floating-point sine.
- Eight four-operator algorithms, feedback history, channel panning and YM3438
  intermediate clipping/output normalization (not YM2612 DAC discontinuity).
- SSG-EG modes, LFO AM/PM, channel-3 special frequencies and CSM key preparation.
- Signed nine-bit DAC replacement of channel 6; remaining five FM channels keep
  running. DAC enable does not freeze channel-6 operators.
- Reference prepare/active-channel cadence and free-running envelope/LFO clocks,
  including the distinction between soft reset and construction.

`advance_with_output(clocks, emit)` supplies native stereo `[i32; 2]` frames to a
caller-owned callback at input clock / 144. `advance(clocks)` discards those
frames but runs exactly the same chip state. Neither API owns an audio device,
filesystem, wall clock, global mutable state or unbounded sample queue. The
caller provides the clock, controls load/reset/unload through owned objects,
and will own host conversion/output. This remains suitable for a future Libretro
frontend without introducing a Libretro dependency or adapter now.

Snapshots now include operators' phases, envelopes and effective key states,
SSG inversion, feedback, envelope/LFO counters, pending register/CSM preparation,
active-channel mask and preparation cadence, besides the earlier timing state.
Derived operator parameters are recomputed from registers, not serialized as
pointers/caches. Saving/restoring does not replay register writes or consume
samples. New synthesis invariants are checked before any restore mutation.
This extends the experimental in-memory chip format; it does not establish a
versioned machine-save format, complete Model 1 save states or NVRAM behavior.

### Evidence and exact limits

`tools/ym3438-reference.cpp --audio` uses the same pinned YMFM source, but hashes
the frame count and every signed stereo sample without inspecting snapshots:
this avoids the reference snapshot routine's cache-invalidation side effect.
`audio.trace` contains 150 synthetic fresh-chip setups and additional writes,
covering **1,668,200 native stereo frames**. All sample-stream digests match.
The fixture is not an audio recording or proof from listening.

Covered scenarios include all eight algorithms times all eight feedback levels;
each of six channels with stereo routing and key-off/release; all eight SSG-EG
modes; all eight LFO rates times eight PM sensitivities with varied AM depth;
channel-3 special/CSM modes; mid-note register changes, pan-off/on, retrigger,
extreme frequency/detune, six voices mixed, DAC switching and reset. These are
bounded parameter scenarios, not exhaustive verification of every register
combination, physical-chip quirk, MAME scheduler interaction or game driver.

Four dedicated audio tests cover the oracle trace, exact mid-note serialized
continuation with different advance slices (including SSG/LFO/CSM and non-sample
aligned cuts), discard-output continuation/reset, and signed DAC/panning.
A fifth new test rejects invalid synthesis state atomically. Earlier control
tests and their 16,384-operation fixture still pass. There are now 14 YM3438
tests; no ROM, external compiler or installed YMFM is required to run them.

Full workspace verification: `cargo test --offline --workspace` passed 211 tests,
with the existing one ROM-dependent test ignored. Development release build
(`cargo build --offline --release -p tgpulse`) passed; the pre-existing `block`
0.1.6 future-compatibility warning remains. Binary: `target/release/tgpulse`
via `tgpulse.dev`. This rebuild does not enable FM in games. No loader/gameplay
or listening test is claimed for the new device. No toolkit deployment,
installation, ROM/NVRAM/settings writes, commit or push was performed.

To regenerate/compare the audio fixture, build the reference tool using the
command above, then run:

```sh
/tmp/tgpulse-ymfm-register-reference --audio \
  < crates/tgpulse-core/src/ym3438/audio.trace \
  | diff -u <(sed '/^#/d; /^$/d' crates/tgpulse-core/src/ym3438/audio.trace) -
cargo test --offline -p tgpulse-core ym3438
```

`T pattern channel` constructs a synthetic patch on a fresh device; `U` adds
another voice without reset; `N` advances a count of native frames. `A` retains
its meaning of chip clocks. Hashes use frame count as LE u32, then each left/right
sample as LE i32, FNV-1a-64. No native production dependency was introduced.

### Board scheduling and mixing checkpoint — 2026-09-27

`SoundSystem` now uses one authoritative YM3438 device instead of the timer
stub. The existing odd-byte bus ports feed that device; even lanes are ignored.
YM IRQ remains disconnected, as in the reference board; UART IRQ2 is retained.
Writes are observed on the 68000 instruction grid, followed by advancing that
instruction's elapsed cycles. The CPU does not provide intra-instruction bus
timestamps: this is not bus-cycle-accurate scheduling.

`sound/fm.rs` converts 10 MHz CPU time to 8 MHz chip clocks with integer remainder,
and bridges native FM frames (every 180 CPU clocks) to MultiPCM frames (every
224 CPU clocks). A causal held-sample area average preserves phase and partial
interval history. This deterministic box filter is not MAME's resampler and is
not a high-order anti-alias filter. Main-CPU-to-sound budgeting also retains its
fraction instead of truncating every call, at both 16 and 25 MHz host CPU rates.

Each generated PCM source retains its existing signed-16 clamp. Mixing uses
reference gains 0.5 for each PCM and 0.30 for FM, then final signed-16 clipping.
Output-only mutes do not stop chip progression or renormalize other sources.
Host audio-device conversion remains outside this hardware path.

New `FmPathState` contains all chip and converter state, including fractional
clocks, last sample and the partially accumulated interval. Restore validates
state before committing and discards stale queued mixed samples. This is **not
a complete sound-board snapshot**: existing CPU, MultiPCM, UART and board
scheduler state will be adapted in a later phase, as agreed with the user.
Neither preferences nor ROMs, host devices, filesystem paths or wall-clock
timing belong in this new state. Full machine save states remain deferred.

Seven additional tests cover converter boundaries and serialized mid-interval
continuation, invalid-state rejection, output-only FM muting, scheduling slice
independence, bus lanes/status/IRQ wiring, mix gains/clipping, and an actual
synthetic 68000 program producing a bipolar FM waveform through the real board
bus and mixer. These complement the 14 device tests and pinned YMFM oracle.

A read-only in-memory loader/core smoke ran 600 frames each for `vr`, `vf` and
`wingwar`, without GUI/audio-device playback or NVRAM/settings writes. Each
produced 465,642 board frames and zero sound-CPU exceptions. YM bus writes were
2,134 / 18,122 / 18,154 respectively. With PCM muted, all three initial sequences
had zero nonzero FM frames: this proves execution and driver bus activity, **not
audible game FM**, nor that these games never use FM. The synthetic CPU test
separately establishes that a programmed FM tone reaches the mixed output.

Final checkpoint verification: `cargo test --offline --workspace` passed 218
tests, with one existing ROM-dependent test ignored. Development release build
passed; the existing `block` 0.1.6 future-compatibility warning remains. No
dependencies were installed, no toolkit/current release was replaced, and no
commit or push was performed.

Next checkpoint: manual in-game sound tests and isolated-source/mixed listening,
including level and performance checks. Development binary: `target/release/tgpulse`
via `tgpulse.dev`; Settings -> Audio -> Mute FM (YM3438). No calibrated cabinet
loudness or completed Model 1 audio is claimed. DSB/MPEG remains a separate task;
the broad timing audit and Z80 consolidation remain final follow-ups.

## Driver-use audit — 2026-09-27

The earlier 600-frame startup silence does not establish timers-only use.
A bounded sound-board-only probe now provides a counterexample: **VR's real
sound program generates FM for several sound commands**. This supersedes the
tentative inference from music-only VGM packs that FM may never be audible.

Method: load local ROMs without changing them or creating a full machine; run
the actual board CPU for one emulated second, then send commands 1..254, each
followed by 100 ms. Protocol comes from M1's
[`M1_SendCmd`](https://github.com/neko68k/M1-Android/blob/master/app/src/main/jni/boards/brd_segamodel1.cpp)
and [`m1.xml`](https://github.com/neko68k/M1-Android/blob/master/app/src/main/jni/m1.xml):
VR subtype 3 uses `B0, command`; VF/Wing War/Daytona subtype 2 uses
`AE, 10, command`, separated by 5,000 audio CPU cycles. Both MultiPCMs were
output-muted, and a temporary logger aggregated accepted YM register writes.
The sole persistent diagnostic hook is opt-in `ym3438` trace logging of accepted
register/data pairs; no per-game counters, automation or runner was added.

| Driver | UART bytes consumed | Nonzero isolated FM frames | Post-initialization YM activity |
| --- | ---: | ---: | --- |
| VR | 508 | 149,892 | Operator/frequency/pan programming and positive key-on, besides timer A |
| VF | 762 | 0 | Only `27=2A`, 22,217 writes (timer B control/acknowledgment) |
| Wing War | 762 | 0 | Only `27=2A`, 22,223 writes |
| Daytona (comparison only) | 762 | 0 | Only `27=2A`, 22,174 writes |

All four runs had zero sound-CPU exceptions. Initialization writes in the last
three include LFO configuration, key-offs, maximum attenuation and DAC disable;
they do not demonstrate active synthesis. Results describe this command range
and short dwell, not every possible command, long sequence or gameplay event.

VR confirmation: construct a fresh board for each command, initialize for one
second, then play for one second. Hex commands `28`, `29`, `2A`, `2C`, `2D`, `2E`,
`2F`, `30` each independently produce positive key-ons and nonzero isolated FM
(34,665–44,320 frames). Commands `31` and `32` caused key-ons during the sequential
sweep but not this independent test, so do not assign them independent meanings.
No DAC-enable use was found in these runs. Mapping these command IDs to named
gameplay events, diagnostic sounds or unused data remains open. This is sound
driver execution evidence, not visual gameplay or listening identification.
Star Wars/NetMerc and clone-specific drivers were not included in this quick audit.

**Subsequent user gameplay confirmation:** FM synthesis is audible for some
actual in-game sounds in VR. This supplements the synthetic and driver-command
evidence; exact sound names/command mapping remain unassigned. It is therefore
incorrect to describe Model 1's YM3438 as universally timer-only.

At the user's request the experimental timers-only option has been removed from
the GUI, configuration, core scheduler and FM-path state, along with its four
mode-specific tests. Synthesis always runs. The existing output-only FM mute
remains and does not stop timers, operators, envelopes or sample generation.
Old experimental configuration keys are no longer interpreted and disappear on
the next normal settings save; no user's configuration file was edited.
The FM chip/converter state remains serializable; legacy full-board serialization
is still a separate later activity. No new dependency or toolkit deployment.

Publication verification after removal: the complete development worktree passes
218 tests (one ignored) and its offline release build. The audio-only staged tree,
exported separately without the pending I/O/control/debugger changes, passes 135
tests (one ignored) and its offline release build. These counts intentionally
describe different source trees. The existing `block` 0.1.6 future-compatibility
warning remains. User gameplay evidence confirms some VR FM sounds, not a complete
per-title audio acceptance matrix.

## Output-only gain and mute contract (implemented)

Settings -> Audio shows one **gain slider / unlabelled mute checkbox / source name** row
for each implemented output: MultiPCM 1/2 and FM (YM3438), or SCSP, according
to the loaded board. SWA/SWAJ additionally expose DSB (MPEG). The library
asks to load a game rather than guessing its hardware.
No DSB switch is shown for games without this board.
The mute checkbox has only a `Mute` tooltip, no inline label; the freed space
extends the gain sliders.

Sliders select absolute output-route gain from 0 to 100 percent, not a
multiplier of the reference gain. At 80%, a MultiPCM route uses 0.8 instead
of 0.5. Initial values and fixed reference markers are 50% for each
MultiPCM, 30% for FM, and 100% for DSB and SCSP. The marker spans the
full bar height in a darker shade of the bar's blue. The native grab is 50%
opaque (idle and active), above the marker; the value text stays above both.
The master slider shares this style, with its fixed reference at 100%.
Double-click a channel slider to restore that channel's
reference without changing its mute state or other channels. Hover shows the
reference; Ctrl-click allows numeric entry. The master remains independent
with its 0–800% range; double-click restores 100% without changing channel gains
or mute states. The reset also holds while the second click remains pressed.
Initial settings preserve the previous integer mixing/rounding exactly.
Larger gains may clip; neither automatic normalization nor a limiter is added.

The checkboxes mean **muted when checked**. No CPU, IRQ, timer, MIDI/UART,
voice, envelope or DSP is paused. Chips generate normally; their outputs are
zeroed before the existing board mix/clipping. Muting one source never boosts
another. Core setters are independent of windows, host devices and paths.

Preferences are global per source, persisted with the existing settings:

```ini
gain_multipcm1 = 50
gain_multipcm2 = 50
gain_ym3438 = 30
gain_dsb = 100
gain_scsp = 100
mute_multipcm1 = off
mute_multipcm2 = off
mute_ym3438 = off
mute_dsb = off
mute_scsp = off
```

Absent gain keys use the reference values; absent mute keys default to off.
Invalid or out-of-range gain entries are warned about and retain the default.
Existing files need not be deleted or reset. GUI changes save normally;
Revert to defaults restores gains and unmutes all. Muting never changes the
stored gain, and a zero gain never stops device emulation.
Preferences apply on load/reset and after machine-state restore. They are not
saved as chip state. A setter clears the core's pending mixed samples on an
actual mute/gain change; samples already submitted to the host may still be heard
for the device's output latency. No click-free fade is claimed.

## Verification boundaries

Synthetic tests use generated waveforms, not ROMs: check each MultiPCM in
isolation, silence while muted, unchanged gain of the other source, ongoing
CPU/timer/voice progression and identical subsequent samples after unmute.
SCSP tests likewise check audible output, mute, timer progression and identical
unmuted continuation. Settings tests cover saved/loaded flags and older files.
GUI hardware selection follows the live sound-board variant, not a guessed
romset-name list. Actual in-game listening and visual GUI checks remain separate
from automated tests/build results.

The user subsequently confirmed that muting works in their manual tests. This
adds user listening/UI evidence for mute behavior, not proof of FM emulation.

Historical mute-only result: `cargo test --offline --workspace` passed 197 tests, with one
ROM-dependent test ignored. The three new sound-board tests passed; settings
round-trips now cover mutes. Offline development release build passed. The
pre-existing future-Rust-compatibility warning for `block` 0.1.6 remains.
Binary: `target/release/tgpulse` via `tgpulse.dev`; no toolkit installation,
ROM/NVRAM/configuration changes, new dependencies, commit or push in this step.
