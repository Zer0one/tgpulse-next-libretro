# Model 1 roadmap

This is the imported TGPulse-Next standalone source plan. The independent
Libretro port has one active implementation plan:
[TGPulse-Next Libretro roadmap](LIBRETRO_ROADMAP.md).

Baseline: 2026-09-27. This is a source-backed work plan, not a claim that every
game is playable. Keep these changes isolated from Model 2 and from the separate
macos-emulation-toolkit and MAME projects.

## Effort and consumption planning

### Consolidated verification — 2026-09-27

- The complete pending integration passes `cargo test --offline --workspace`:
  463 passed after the bounded timing corrections, FIFO retry completion,
  timed main UART integration, input updates, device/motherboard continuation
  and versioned whole-machine/desktop state validation. The offline development
  release build also passes; the existing `block` future-compatibility warning
  remains. These checks are not a fresh manual gameplay validation.
- Wing War gameplay and throttle direction were confirmed by the user;
  the bounded Model 1 timing audit is now complete (limits recorded below).
  Z80 consolidation was explicitly brought forward before complete save states.
  NetMerc/R360 are not promoted by this result.
- The frontend's Return to game menu / quit also operates in the library,
  retaining the CLI/fullscreen exception and held-chord state across game close.
  Manual UI/gamepad validation of the new library-exit path remains pending.
- No ROM, NVRAM or personal configuration is included in publication; toolkit
  and installed `current` releases are unchanged.
- DSB/MPEG is integrated for SWA/SWAJ; the user reports basic SWA playback
  working. Shared flight throttle, SWA view routing, optional sRGB correction,
  Model 1 2D palette intensity and persistent source gains are implemented.
  Master volume now also has a 100% reference and double-click reset.
- User confirms the washed-out colour issue resolved with sRGB correction.
  Player 2 and the Model 1 networking milestone are closed by user decision
  (details and evidence limits below). User testing now closes Model 1 save-state
  acceptance across the remaining catalogue games. Audio is also closed by user
  acceptance: the perceived SWA/MultiPCM clipping occurs in MAME as well.
  Further audio fidelity checks and the Wing War R360 follow-up are possible
  future work rather than open acceptance gates;
  the timing audit is closed as a bounded implementation milestone, not a
  hardware-cycle-accuracy or fresh gameplay certification.

### Current milestone status — 2026-09-28

| Stato | Attività | Cosa resta |
| --- | --- | --- |
| 🟢 | Model 1 save states | Closed after user tests on the remaining catalogue games; COMM-fitted saves remain unsupported by design. |
| 🟢 | Model 1 audio (FM, DSB, MultiPCM) | Closed by user acceptance; the perceived SWA clipping also occurs in MAME. |
| 🟢 | Model 1 pad rumble | Closed for the established VR use case. On current evidence VFormula is not an FFB-capable game; other cabinet actuators are separate possible work. |
| 🟢 | Wing War I/O, Z80 consolidation, bounded timing audit, P2, networking and sRGB correction | Milestones closed with the evidence limits below. |
| 🔵 | Extended audio fidelity checks, R360 motion/link and NetMerc | Possible separate follow-ups, not open acceptance gates for the closed milestones. |

Legenda: 🟢 Completato · 🟡 Parziale · ⚪ Da iniziare · 🔵 Possibili.

Apply the preflight/checkpoint/recap agreement in [AGENTS.md](../AGENTS.md).
The table is an initial engineering estimate, not a measured cost or completion
promise. Recommend the actual available model by name at the start of each
activity; use a capable coding/reasoning model for hardware work and a lighter
one only for well-specified mechanical changes. Obtain approval before switching.

| Activity | Initial reasoning effort | Expected consumption | Checkpoint |
| --- | --- | --- | --- |
| Documentation, ROM audit, established build/test commands | low | Low | Stop at a verified report; diagnose unexpected failures separately |
| Bounded loader/NVRAM fix with a known reference | medium | Low–medium | Regression test before expanding scope |
| Timer, CPU or rendering discrepancy investigation | high | Medium–high | Reproducer and source comparison before implementation |
| Model 1 I/O board 2 | high; xhigh if protocol behavior remains ambiguous | High | Separate register map, boot handshake, controls and persistence |
| YM3438 synthesis | high | High | Choose a verified implementation/integration approach before full audio testing |
| Z80/MPEG DSB board | high; xhigh for unresolved timing/protocol analysis | High–very high | Separate serial protocol, CPU/decoder and mixing |
| NetMerc integration and gameplay diagnosis | high | High, low confidence | Verified ROMs and I/O first; reassess after boot evidence |
| Save states | high | Medium–high | State inventory, round-trip, then gameplay continuity |
| Cabinet link / drive / motion fidelity | high–xhigh | Very high, low confidence | Specify one protocol/device and observable acceptance criteria |

Confidence is medium for bounded work and low for missing hardware. New measured
runs can refine estimates, but account-wide percentage deltas are not per-task
costs. Do not backfill a consumption figure for Phase 1: no comparable start/end
usage baseline was collected for that implementation.

## Cross-cutting architecture — possible external Model 1 Libretro port

During every integration, consider a future Libretro frontend initially limited
to Model 1: execution/lifecycle, input, audio/video, resources, portability and
save states, not serialization alone. Apply the concrete guidance in
[AGENTS.md](../AGENTS.md) where relevant, keeping changes minimal and avoiding
an unsolicited adapter or broad rewrite. A Libretro adapter is not a TGPulse
project milestone or a claim of current Libretro support.

### Model 1 pad rumble — closed VR checkpoint

The bounded standalone VR pad-rumble pass uses captured cabinet commands and
SailorSat's VR protocol description. User acceptance closes this Model 1 rumble
milestone (2026-09-28). Wheel force feedback and unrelated cabinet actuators
are outside it; an external Libretro port is not a deliverable of this project.

- [x] Decode VR's 0x1x..0x6x drive commands and all four strength bits in a
  frontend-independent core module. The prior shared decoder came from
  Daytona's `epr-16488a` firmware and remains in use only for the unaudited
  paths; the VR-motor family adapter is selected for `vr` and `vformula`.
  MAME wires both games to the same original Model 1 drive-board callback;
  that wiring alone does not establish FFB support in VFormula. The original
  Model 1 output latch remains serialized, while frontend pad effects reset on
  load, session stop and backend change.
- [x] Verify VR command decoding and stop/reset behavior with automated tests
  and the user's non-Standard-cabinet pad test. On current evidence VFormula
  does not offer FFB, so a physical pad test for it is not an acceptance gate.
  This does not certify wheel torque or other hardware families.

Current standalone limit: only P1 receives pad rumble. VR and VFormula now
share motor-board routing, but VFormula is not claimed to produce FFB. The
Standard VR cabinet's capture had only handshake outputs and therefore remains
silent. Special, Upright and 2P Link captured drive commands. Low/high SDL3
motor levels are distinct; gilrs uses the larger level as its single effect
gain. Pad buzz is not directional wheel torque.
The locked `gilrs-core` 0.5.15 macOS backend reports no force-feedback support
and its motor-output function is empty, even with `rumble = on`. The SDL3
backend can drive P1 pad motors on macOS; unit tests establish command and
adapter behavior. Physical cabinet motion remains outside this milestone.

Other Model 1 output survey (MAME `model1.cpp`, 2026-09-28): ordinary Wing War
has I/O board 2 but no drive callback; Wing War R360 adds a separate feedback
protocol whose documented bytes mostly cover cabinet states and replies, not a
pad-rumble intensity. SWA has no drive callback; its documented outputs are
lamps and coin counters, with some bits still unknown. NetMerc has a documented
trigger/thumb motor output bit, but the game and output path need separate
validation. None of these observations authorizes reusing the VR decoder or
inventing effects from game motion, audio or throttle.

Diagnostic checkpoint (2026-09-28): `RUST_LOG=warn,model1_drive=trace`
records each output from the original Model 1 I/O board's port E, including
multiple outputs in one frame; `frame=N sampled=XX` marks the byte currently
forwarded to pad rumble. The trace changes no hardware or controller behavior.
For an interactive VR capture with the configured NVRAM, run
`RUST_LOG=warn,model1_drive=trace tgpulse.dev vr 2> /tmp/tgpulse-vr-drive.log`,
drive briefly, then quit. The headless debugger reached VR gameplay with its
default NVRAM but observed only two `00` outputs during initialization and no
later port-E output; this does not establish what the configured cabinet sends.

## Phase 1 — bounded compatibility fixes

- [x] Prefer the corrected `315-5711.bin` (MAME 0.289). Recognise the complete
  legacy 8 KiB dump by SHA-1 and repair its two bad bits **in memory at load time**.
  Do not modify unknown programs or rewrite ZIPs during normal loading.
- [x] Preserve the last observed timer count when software stops either timer,
  matching MAME #15715. Cover stop/read/restart and periodic expiry.
- [x] Pass the ROM set's factory `nvram` region to Model 1 initialization.
  A complete saved user NVRAM image overrides it. Preserve the existing zero
  default for games without a factory image; do not reset users' saves.
- [x] Honour declared ROM load lengths: NetMerc reloads only 128 KiB of its
  512 KiB sound program. Previously the loader ignored that length and failed
  with a region overrun. Cover the real database record with a synthetic chip.
- [x] Supply the missing `netmerc_nvram.bin` from the author's verified MAME PR attachment.
  Wiring the region does not supply its contents or make NetMerc playable.

### ROM baseline and fallback

MAME change: [#15649](https://github.com/mamedev/mame/pull/15649), included in
[0.289](https://www.mamedev.org/?p=565).

| `315-5711.bin` | CRC32 | SHA-1 |
| --- | --- | --- |
| Legacy | `c5ddb8fc` | `9e21d3a07ffa315e0139483b664e3fa283ef4e06` |
| Corrected | `6a21f304` | `d5c61ea6e4744f10170ea556068c248bd43bb111` |

Clear bit 1 in bytes `0x22c` and `0x19f8` (little-endian instructions at PCs
`0x8b` and `0x67e`). The result matches MAME's corrected SHA-1. This is a repaired
dump, still marked BAD_DUMP by MAME, not a newly verified physical-chip dump.
Affected sets: `swa`, `swaj`, `wingwar`, `wingwaru`, `wingwarj`, `wingwar360`, `netmerc`.
Updating these ZIP members does not certify unrelated ROMs or missing devices.

NetMerc factory image: 65536 bytes, CRC32 `09866826`,
SHA-1 `411134c1e6307f2e32c3b4b372597b45b14a9834`.
Source: author-provided `netmerc_nvram.zip` in
[MAME PR #15642](https://github.com/mamedev/mame/pull/15642),
[attachment](https://github.com/user-attachments/files/29659125/netmerc_nvram.zip).
This is the initialization/calibration image supplied with that change, not a
replacement for a user's persistent gameplay NVRAM.

## Phase 2 — missing hardware (separate implementation tasks)

1. **Model 1 I/O board 2 — Wing War first:** implement the shared board with
   `wingwar`, `wingwaru` and `wingwarj` as the initial acceptance targets.
   Validate boot handshake, digital and analog controls, EEPROM persistence
   and regional variants against the reference. R360 cabinet-specific behavior
   is a separate follow-up; NetMerc is not an acceptance gate. The current
   frontend signal mapping is not a substitute for this board.
   [Register/wiring contract and checkpoints](MODEL1_IOBOARD2.md): shared
   315-5338A/ADC extraction, advanced bus map, CTC, internal watchdog and IRQ
   arbitration, CPU-to-bus wiring, PIO and a bounded asynchronous SIO subset
   are tested. Base Wing War now selects the advanced board and passes startup,
   a MAME DPRAM trace replay, physical input delivery and EEPROM reload checks.
   World/Japan attract-mode 3D frames were inspected. The user subsequently
   reported successful Wing War play/control tests, including the corrected
   throttle polarity. This does not certify every regional set, audio,
   extended play or link support. NetMerc remains possible future work.
   R360 source audit (2026-09-27): MAME declares `wingwar360` without a
   NOT_WORKING flag and reuses MODEL1IO2, wiring IN2/drive commands to
   `r360_r`/`r360_w` (cabinet safety/setup acknowledgements and throttle), with
   ADC channel 2 held at zero. The R360 follow-up now selects the advanced board,
   loads `epr-16891.6` via the database/generator, applies the shared DPRAM wait
   cycle and implements the MAME cabinet response protocol synchronously on
   drive writes. Reply/throttle state is serialized with the existing board;
   restoration rejects a different cabinet type. Cold boot reaches attract mode
   at 1,800 frames instead of I/O BOARD ERROR. Manual controls/gameplay/audio,
   R360 networking and mechanical motion remain unvalidated; this is not a full
   motion-system simulation. See the R360 checkpoint in MODEL1_IOBOARD2.md.
   **R360 follow-up is possible future work by user decision**, including the
   EEPROM/link diagnosis; it does not keep the main networking milestone open.
   The bounded timing audit and Z80 consolidation are complete; whole-machine
   save/load is implemented, with broader in-game acceptance still open.

   **DPRAM timing follow-up completed:** the one-cycle V60 read wait now also
   applies to the original I/O-board games, matching MAME's common memory map.
   See the bounded [timing checkpoint](#dpram-checkpoint--2026-09-27) below;
   this is not a completed system-wide alignment.
2. **YM3438 synthesis:** Rust FM/DAC synthesis is implemented and connected to
   the production MultiPCM board. Audio is closed by user acceptance; the
   historical test checkpoints below are not claims of exhaustive per-title
   waveform equivalence.
   **Current checkpoint:** [audio integration contract](MODEL1_AUDIO.md).
   Output-only mute controls for MultiPCM/SCSP, FM and the integrated DSB are
   available. The user confirmed mute operation and FM sounds in VR gameplay.
   The YMFM subset inventory and isolated
   C++ reference smoke/state-continuation probe are complete; see the contract
   for exclusions and evidence limits. Recommendation remains a specialized
   Rust port, with a native wrapper only as an explicitly agreed fallback.
   The isolated Rust register/timer/Busy/state boundary now passes nine tests,
   including a pinned YMFM trace with 16,384 seeded operations and serialized
   mid-timer continuation. Clock/state
   APIs are frontend-independent; no native production dependency was added.
   Isolated operators/envelopes, eight algorithms, feedback, LFO/SSG-EG,
   special frequencies/CSM and DAC now match the pinned YMFM audio oracle for
   1,668,200 stereo frames across bounded synthetic scenarios. Mid-note serialized
   continuation also passes; 14 YM3438 tests in total. Seven new bridge/board
   tests cover fractional scheduling, conversion-state continuation, real 68000
   bus programming of a tone, mix gains/clipping and output-only FM mute.
   The timer stub is removed. Native FM is converted with a causal box filter,
   not MAME's resampler; bus timing remains instruction-granular. A 600-frame
   smoke for VR/VF/Wing War has no audio-CPU exceptions, but isolated FM is silent
   in those initial sequences; actual game listening acceptance is still open.
   New chip/converter state is serializable now; adapting the existing sound
   CPU/PCM/UART/scheduler belongs to the later complete-machine-state phase.
   **Follow-up audit:** a 1..254 sound-command sweep finds timer-only activity
   after initialization in VF/Wing War/Daytona, but **VR generates real FM**:
   eight commands reproduce it independently from a fresh sound board. The user
   subsequently confirmed FM sounds during actual VR gameplay. Exact command-to-
   event names remain unassigned, but FM is not merely unused/diagnostic code.
   The experimental timers-only option has been removed; synthesis always runs
   and only output muting remains. See the
   [audit method and limits](MODEL1_AUDIO.md#driver-use-audit--2026-09-27).
   Further per-game listening is possible future work; DSB integration is recorded below.
3. **Star Wars DSB:** implement the Z80/MPEG board and its filtered serial command
   path. Test music independently from the existing Model 1 sound board.
   [Source audit and integration contract](MODEL1_DSB.md) complete: use the local
   `z80` in `crates/z80`; receive the 68000 sound firmware's output, not raw V60 commands.
   Isolated bus/i8251/CPU boundary implemented with twelve synthetic tests,
   including mid-transfer state continuation. Isolated Layer II decoder passes synthetic and
   local SWA-data comparisons against MAME, plus history restore/truncation tests;
   joint stereo is explicitly unsupported (reference anomaly documented).
   Real firmware now runs with an opt-in clocked 68000-output serial link:
   15 controlled commands transmitted/consumed, with playback/loop/pan register
   programming and seven additional synthetic integration tests. Buffered 32 kHz playback/loop handling and
   mid-buffer restore now work in the isolated board: six SWA firmware commands
   produced PCM for 60-second probes; two looping runs reproduced their next
   32,000 stereo frames exactly after restore. Nine new synthetic tests cover
   playback, clock slicing, segment transitions and snapshot/error boundaries.
   Loader resources, error propagation, causal rate conversion/mixing and DSB mute
   are now integrated: normal SWA/SWAJ constructors enable the board. Settings
   persist `mute_dsb`; missing/wrong-size DSB chips fail explicitly. The filtered
   68000 -> DSB -> mixed signal path passes for both sets. Basic SWA playback
   is user-confirmed. Extended SWA/SWAJ listening, transitions and balance are
   possible future checks, not a condition for the audio milestone's closure;
   no MAME-resampler-equivalence claim follows.
   Verification: 260 workspace tests and 36 release DSB tests passed; normal SWA/SWAJ
   boot probes completed 1,800 release frames each. Six musical commands produced
   nonzero isolated DSB mix for both sets; five Model 1 titles pass 120-frame CLI smoke.
   Separately investigate the existing
   V60 string-operation multiplication overflow encountered in the debug probe
   (`ops.rs`, register-28 update); do not mask it with global overflow settings.
4. **NetMerc initialization — possible separate milestone:** after Wing War,
   with verified ROMs and I/O, validate factory NVRAM,
   startup and gameplay. MAME itself still marks NetMerc not working, so it is
   not a complete gameplay oracle.
   Review the additional Polhemus/i386SX tracking subsystem separately from
   the I/O board; the reference machine configuration includes it. Do not require
   NetMerc's boot, diagnostic LCD or tracking work to complete the Wing War
   milestone. Shared hardware fixes remain reusable, not NetMerc-specific hacks.

Priority confirmed on 2026-09-27 against the
[official MAME driver](https://github.com/mamedev/mame/blob/master/src/mame/sega/model1.cpp):
NetMerc is declared `MACHINE_NOT_WORKING`; Wing War World/US/Japan and R360
have flags `0`. This is MAME's declared status, not a claim of perfect hardware
fidelity or a new gameplay test. NetMerc's Polhemus/i386SX subsystem is additional
to the shared I/O board and cannot be treated as a proven complete reference.

## Phase 3 — fidelity and remaining features

- [ ] **🔵 Possible — Wave Runner throttle travel (Model 2 follow-up):**
  user reports RT drives Throttle Lever and LT does not. This matches the
  current SM2 Libretro profile and the pre-refactor behavior, but equivalence
  alone does not establish correct hardware travel. The original Sega
  [WaveRunner owner's manual](https://manualzilla.com/doc/7426256/sega-waverunner-owner-s-manual)
  describes gripping/releasing an accelerator and specifies about `E0 ±9`
  with the lever released; the current mapping produces `80 -> 00` on RT.
  Later compare useful travel, polarity and the game's saved Volume Setting
  calibration before deciding whether the ADC mapping needs correction.
  Do not infer a bidirectional lever merely from MAME's centered analog port.
  **User explicitly requested leaving it unchanged for now (2026-09-27):**
  preserve binding, ADC range, NVRAM and frozen refactor baseline.

- [x] **Per-source output gains and mute:** GUI rows expose an absolute gain
  slider, independent Mute checkbox and source name, with a fixed reference
  tick (PCM 50%, FM 30%, DSB/SCSP 100%). Persist gains in settings and reapply
  on load/reset/state restore. This is frontend-controlled output mixing,
  not chip state or a replacement for the clipping investigation below.
  Channel sliders are limited to 0–100%, with full-height dark-blue markers
  drawn below the 50%-opaque native grab (idle and active);
  double-click resets just that gain, including while the second click is
  held. Master retains its 0–800% range, with the same marker style at 100%
  and double-click reset to 100%.
  Verification: 276 workspace tests passed, including absolute gain/default
  rounding, mute/gain independence, PCM/DSB/SCSP continuation, settings
  persistence/validation, headless ImGui row layout and double-click/held-click
  reset isolation for channels and master, draw-order/opacity and no style leakage
  into other widgets. Development release
  rebuilt; real-game listening and manual slider interaction remain pending.

- [x] **MultiPCM1 distortion during SWA acceleration/deceleration:** user
  compared the perceived clipping with MAME and reports the same artifact there
  (2026-09-28). The audio milestone is closed on that acceptance basis; this is
  not a measured sample-by-sample equivalence. If revisited, a possible fidelity
  check would capture pre/post-mix peaks in both emulators before changing gains.
  Do not lower nominal board gains speculatively.
- [x] **Model 1 washed-out colours: resolved with sRGB correction.**
  User confirmation on 2026-09-27 closes this reported defect, not every possible
  per-title rendering discrepancy.
  Two bounded corrections are implemented (2026-09-27):
  - Optional **sRGB correction**, persisted as `srgb = on/off` (default off),
    updates immediately. The frontend samples display RGB through an sRGB
    texture only when the output surface also encodes sRGB. The non-sRGB
    fallback stays byte-preserving; core/compute pixels and GUI are unchanged.
    Applies to both Model 1 and Model 2; no host conversion is baked into the
    reusable core. Apple M4 Metal offscreen readback verifies all 256 grey
    levels with correction off/on/off and both surface
    formats (tolerance one byte). Without correction, 128 becomes about 188
    on an sRGB surface. The GPU test reports when no adapter is available;
    it was explicitly rerun outside the sandbox and exercised Metal here.
  - Model 1 tile pens now always halve RGB8 channels when palette bit 15 is
    clear, matching `model1_paletteram_w`. A board-specific trait hook keeps
    Model 2 unchanged; this affects selected 2D pens, not every polygon.
    Reference: [MAME Model 1 video source](https://github.com/mamedev/mame/blob/master/src/mame/sega/model1_v.cpp).
  The user confirmed the sRGB correction resolves the reported appearance;
  synthetic colour checks alone do not certify every title's rendering fidelity.
  Verification: 267 workspace tests passed; the GPU case was also explicitly
  exercised on Metal. Exhaustive RGB555/intensity tests cover Model 1 and
  unchanged Model 2 palette expansion; settings round-trips include sRGB.
  Offline release build passed; reported in-game colour defect is user-accepted.

SWA input follow-up (2026-09-27): `swa`/`swaj` hardware button 3 now routes to
`View / Select 1` (default D-pad Down / Z), matching Sega Rally's view binding,
not to `Action 3`. SWA and Wing War now share `Throttle Up` / `Throttle Down`,
with both game families listed below the GUI entries, independent of the
driving Accelerator/Brake bindings. Up is L2 OR right stick up (W/Up keys);
Down is R2 OR right stick down (S/Down keys). The triggers were subsequently
swapped by user request, and the signal polarity inverted independently, leaving
labels and RS-Y/keyboard bindings unchanged. Up now raises the ADC and Down
lowers it; both rest at 128, retaining SWA's 28..228 and Wing War's 1..255 ranges.
Verification: 263 workspace tests passed, including SWA/SWAJ default-view,
shared throttle polarity/partial-travel/pedal-isolation and binding migration
tests plus the all-set digital crosstalk audit.
Offline development release build passed. The new view binding still needs
manual gameplay confirmation; the clipping check above remains open.

- Compare rendering and timing per title: clipping, moire, palette translation,
  HUD ordering, gamma and monitor modes. In particular, the Model 1 tile source
  still returns `false` for `colorxlat_written()` and identity monitor gamma;
  do not change this merely by analogy without tracing actual game writes.
- Validate VR/Virtua Formula, VF, SWA and the Wing War variants in-game; the
  README's tested-title list is not a complete compatibility matrix.
- [x] **Player 2 controls — Model 1 and Model 2, closed by user decision.**
  Cabinet P1/P2 share one unfiltered signal catalogue with independent bindings;
  unsupported P2 rows are grey/non-editable. Coin 2 / Start 2 move to P2;
  deliberately duplicated Test/Service bindings OR into shared machine lines.
  Default P2 gameplay bindings are gamepad-only, following P1 conventions;
  Coin/Start/Test/Service also retain keyboard defaults. Controller selection
  persists; disconnect never promotes the other player's assigned pad.
  SWA/SWAJ Gunner uses ADC 4/5 and IN.1 bits 04/08, with no Start/view/throttle
  per user convention; MAME's declared Start2 line is intentionally not routed.
  The user's INPUT TEST 1/2 screenshot confirms the Gunner/Pilot digital
  control distinction (all switches OFF, not a two-device input test).
  VF's second joystick/actions use IN.2. Model 2 follows the authoritative
  SM2 Libretro workbook and current source: local joystick/actions, Baseball
  Bat Swing and independent positional/serial guns; Power Sled's second seat
  uses supplementary MAME wiring because it is absent from the workbook.
  No P2 gameplay is invented for linked single-seat cabinets or Royal Ascot II.
  Air Walkers P3/P4 remain outside scope. Native P2 gun fields are serializable;
  Model 2 snapshot format 2 rejects old format-1 saves; NVRAM is unchanged.
  [Contract, device identity limits and migration](INPUTS.md#player-2--model-1-and-model-2).
  Headless tests cover P1/P2 isolation, polarity/ranges, default keys, migration,
  shared Test/Service, GUI disabled rows and device assignment/reconnection.
  Verification: 293 workspace tests pass (89 frontend), including all 100 sets,
  independent gun transports and mid-mux save/restore; offline release build
  and `tgpulse.dev --list` pass (100 sets). No new physical-controller or
  gameplay validation is claimed.
  The user subsequently closed this milestone; this status change does not
  invent new two-controller, reconnect or gameplay test evidence.
  Identical controllers may need re-selection after a restart
  if the OS changes their enumeration order. This is local same-cabinet play,
  not cabinet-link emulation; no host dependencies are added to the core.
- Add Model 1 machine save states (separate from persistent NVRAM), preserving
  an in-memory API suitable for a future Model 1-only Libretro frontend.
  New integrations should inventory state and add serialization/continuation
  coverage where applicable now, without waiting for the full adapter. The
  standalone machine now has a versioned, ROM-identified in-memory save/load
  API with bounded decoding and atomic rejection. Desktop integration is
  implemented; broader in-game acceptance remains, and saves with COMM fitted
  are refused.
  See [save-state restart](#save-state-restart--2026-09-27).
- [x] **Cabinet link — closed by user decision.** The following records the
  actual automated evidence, not additional manual gameplay/LAN certification.
  First M1COMM HLE checkpoint implemented from MAME's active
  simulation path, with 4 KiB V60 mapping, VINT scheduling, ring protocol and
  serializable board state. Nine new tests cover board/bus behavior; 302 workspace
  tests pass and the offline release build passes. VR standalone completes 1,800
  debugger frames, but no linked gameplay is established. See
  [source inventory, protocol and evidence](MODEL1_NETWORK.md).
  The next desktop checkpoint adds a frontend-owned TCP ring, persistent
  settings and GUI names based on the actual Supermodel Standalone Networking
  controls. Loopback TCP board exchange, fragmented I/O, missing peer,
  disconnect/cleanup and configuration round-trip pass; 309 workspace tests
  pass. Two narrowly scoped V60 overflow fixes match MAME's wrapping arithmetic.
  Per-instance `--config` and read/write `nvram` profiles now preserve the caller's
  explicit relative paths through toolkit launchers. ROM-free checks cover
  independent files, invalid/missing NVRAM, CLI precedence and path resolution;
  311 workspace tests pass. Example VR profiles do not set operator roles.
  User-configured VR NVRAM now passes real-ROM boot/link acceptance for both
  MASTER/SLAVE and MASTER/SLAVE/LIVE: 2,400 frames each, online for the last 600.
  Roles/IDs/counts are 1/1/2, 2/2/2 and LIVE relay 0/0/2, without forced COMM
  success or writes to the original NVRAM. The same test uses real loopback TCP
  within one host process, not separate desktop instances.
  Wing War World now passes the same 2,400-frame MASTER/SLAVE ROM/NVRAM link
  probe (roles/IDs 1 and 2, count 2, last 600 frames online). R360 does not:
  its two machines advance but COMM RAM remains zero and no link is established.
  Repeated R360 diagnostics show the game rewriting the provided EEPROM in
  memory (signature/configuration changes), with CN/FG still FE/FE. Diagnose
  fixture reinitialization before transport; originals were not saved or changed.
  Diagnose that game-specific activation/configuration path before claiming
  R360 link support. Neither result establishes synchronized human gameplay.
  `cabinet = single|twin` now selects COMM absence/presence for eligible Model 1
  games too; the GUI control is shared with Model 2. On the desktop, `twin` now
  also starts TCP; the redundant Network toggle/config key is removed.
  Operator NVRAM roles are not overridden. Regression coverage
  checks both cabinet values with supported/unsupported sets and NVRAM retention.
  Separate desktop synchronized gameplay, physical LAN and MAME interoperability
  were not established by these probes; the user accepts closure with these
  evidence limits. R360 EEPROM/link diagnosis is possible separate follow-up work.
  The earlier scripted
  menu preparation failure is superseded by user-configured fixtures;
  no synchronized gameplay result is claimed. No automatic fake loopback,
  invented protocol, Z80 COMM firmware claim or full-machine save-state claim.
  Drive/motion-board fidelity remains separate from cabinet link and the existing
  controller rumble approximation. The VR-family standalone pad checkpoint is
  closed above; wheel force feedback is separate possible work, not part of the
  networking milestone. A Libretro adapter belongs to an external project.
- Consider checksum-aware ROM diagnostics beyond the narrowly guarded TGP
  fallback. The current general loader matches names, not expected hashes.

## Phase 4 — consolidation

### Native cabinet input cleanup — 2026-09-27

🟡 Implemented on the dedicated `codex/native-cabinet-inputs` branch and
integrated into `main`; manual gameplay acceptance remains. The single public signal catalogue now feeds
cabinet ports/ADCs without the old `Control` compatibility catalogue or
post-poll port corrections. Model 1 and Model 2 mappings are preserved by
100-set baseline traces and independent wiring tests after each control-family
checkpoint. The core input/snapshot APIs remain unchanged; this is not a
Libretro adapter. [Checkpoint evidence and limits](INPUT_AUDIT.md#native-input-refactor--2026-09-27).

Subsequent user-approved VF/VF2 exception: South/L1 Kick, East/R1 Punch,
West Guard, for P1/P2 and every VF2 revision. The five affected sets compare
to the frozen trace after only the documented action-bit permutation; raw
port tests separately assert the corrected semantics. All other sets retain
strict equivalence. [Rationale and regression strategy](INPUT_AUDIT.md#vf--vf2-semantic-correction-after-equivalence--2026-09-27).

Dedicated Gear Down/Up now separate sequential shifting from Action 1/2,
with E/L1 and Q/R1 defaults and ordinary catalogue GUI rows. Direct H-Gate,
Motor Raid attacks and Desert Tank's toggle remain unchanged. Old trace
stimuli are adapted explicitly to the new channels; separate tests assert
binding independence. Workspace tests and release build passed; manual
gameplay acceptance remains separate. [Details](INPUT_AUDIT.md#dedicated-sequential-gear-signals--2026-09-27).

### Save-state restart — 2026-09-27

The preceding integrations were published separately by type on `origin/main`
through `c0823dd`; this section describes the subsequent save-state work.
Scope remains Model 1 and an in-memory core API first, not a new Libretro
adapter, Model 2 snapshot redesign or host-file side effect.

| Status | Component | Current boundary / next work |
| --- | --- | --- |
| 🟢 | MultiPCM device state | Mutable voices, register selectors, bank, cached sample metadata, envelopes, LFO phases, interpolation and counters captured; continuation tests below. |
| 🟢 | Audio 68000 state | Same 0.2.3 implementation, now local with an explicit state API including STOP, current opcode and pending exceptions. No opcode/timing change. |
| 🟢 | Model 1 audio-board state API | CPU, board RAM, both PCM chips, FM, timed serial, optional DSB and clock debt restore together; integrated into the machine envelope. |
| 🟢 | I/O-board state API | Original Z80/bus/RAM/ADC/EEPROM/debt and advanced Wing War/R360 boards with fractional clocks and variant checks; integrated into the machine envelope. |
| 🟢 | Video state API | Persistent polygon/color/light uploads, both display lists/control registers and 2D video RAM; integrated into the machine envelope. |
| 🟢 | Motherboard / V60 / MB86233 state API | CPU state, RAM, FIFOs/latches, timers/IRQ, frame counter, TGP fractional phase and retry/debt state captured at stable execution boundaries; synthetic and bounded ROM continuation verified. |
| 🟢 | Machine envelope / COMM boundary | Version 1, loaded-ROM identity, bounded decoding, all-or-nothing restore and core audio invalidation. Save/load explicitly refused whenever COMM is fitted. No implicit NVRAM disk writes. |
| 🟢 | Desktop save/load integration | Existing menu/F5/F7/slots now dispatch Model 1; bounded reads, atomic slot writes, output invalidation and fullscreen feedback covered by automated tests. Model 2 snapshot path unchanged. |
| 🟢 | Machine continuation acceptance | Machine API passes fresh-resource restore plus 30 equal frames/audio/state on seven sets; user reports successful save/load tests across the remaining catalogue games after VR and SWA. The milestone is closed by user acceptance; COMM-fitted saves remain explicitly unsupported. |

Acceptance update (2026-09-28): the user's wider catalogue tests supersede the
earlier VR/SWA-only manual acceptance boundary in the historical checkpoints
below. They do not change the explicit COMM limitation or establish deterministic
long-running gameplay after every restore.

**Completed first checkpoint:** the previously paused MultiPCM draft is tested
using synthetic ROM resources, without touching game ROMs or NVRAM. Both 8-bit
and packed 12-bit banked samples are captured 137 samples into an active voice;
bincode round-trip into a separately initialized chip produces 4,096 identical
subsequent stereo samples and identical final state. The sequence includes
looping, pitch/amplitude LFO phases, total-level interpolation and a second
round-trip during envelope release. Restoring does not replay register writes
or key-on, and the state excludes sample ROM and rate-derived lookup tables.
Wrong-clock, invalid slot/address/bank and invalid voice selectors are rejected
before mutation; rejection tests compare the entire pre/post encoded state.
This is selector/clock validation, not a general hostile-input decoder guarantee.
The enclosing machine format must enforce resource identity and decode limits.

Verification: 347 workspace tests pass, including the two new MultiPCM tests.
The development release builds offline. No new manual gameplay/save-load proof
is claimed; the desktop still has no complete Model 1 save-state integration.

**Historical next checkpoint (superseded by the acceptance update above):**
extend real-window save/load acceptance beyond VR/SWA, including paused/running
scenarios and audio continuity. The wider user tests close this milestone;
individual edge-case regressions can still be investigated when reported.

### Audio CPU / board save-state checkpoint — 2026-09-27

`crates/m68000` is a local copy of the same crates.io 0.2.3 implementation,
not another emulation core. Origin, checksum, upstream revision and MPL-2.0
notice are in its README. `src/lib.rs` only adds the new `state` module;
the other original CPU source files are byte-identical to the dependency cache.
The new state includes registers, STOP, current opcode and pending exception
vectors, reconstructed with the original priority-set insertion semantics.
Restore does not call reset, touch the bus or queue new interrupts. Tests
cover pending masked IRQ/STOP, opcode-dependent exception frames, continuation,
and rejection without mutation. CPU model/version checks belong to the outer
machine envelope; the board API always instantiates Mc68000.

`SoundSystem::snapshot_model1/restore_model1` now assemble that CPU state with
sound RAM, both MultiPCM states, FM/converter, timed serial endpoints, optional
DSB/converter, main-clock fraction, instruction debt and diagnostic counters.
The API is Model 1-only, memory-owned and captures between run calls. ROMs,
host callbacks, audio output queue, mutes and gains are excluded; restore keeps
current frontend preferences and clears stale output only on success. All
fallible checks run before committing live state; DSB restore is itself atomic
and is the last fallible step. Full machine restore still needs an enclosing
transaction, resource identity and bounded decoding. No NVRAM file writes.

New board tests restore into a separately initialized board after advancing
another timeline, midway through serial transmission with live PCM/FM and
nonzero fractional clock debt, with/without the DSB board. Subsequent stereo
samples and serialized board state match; stopped-CPU continuation also matches
across different run partitions. Rejection tests cover CPU, buffer dimensions,
PCM clock, variant and DSB conversion inconsistencies without changing state or
pending host audio. The combined fixture exercises DSB serial/CPU state, not
MPEG playback; MPEG mid-operation continuity remains covered by the existing
device-level DSB tests, not claimed as a new whole-board music acceptance run.

The upstream stable-compatible assembler, memory and status-register tests are
enabled. Its `operators.rs` uses the obsolete nightly `bigint_helper_methods`
feature, is retained unchanged as reference, and is not registered as a stable
test target. No nightly installation or ignored-test success claim. Existing
upstream lifetime/function-pointer test warnings are not new emulation errors.

This checkpoint still does **not** implement Model 1 GUI save/load, complete
machine snapshots, deterministic network restore, Libretro or run-ahead.

Verification: 441 workspace tests pass (including 88 upstream CPU tests/doc
examples newly registered in this workspace, three new CPU state tests and
three new board state tests). Offline release build passes. Seven Model 1 sets
(`vr`, `vformula`, `vf`, `swa`, `swaj`, `wingwar`, `wingwar360`) and the nine
declared-working Model 2 sets listed below reach 600 frames with identical
before/after debugger state, sampled bus memory and PPM images, without logged
runtime errors. This is bounded no-regression evidence, not new gameplay,
listening, full 3D Model 2 capture or actual-game save/load acceptance. Evidence
directory: `/tmp/tgpulse-soundstate.j5JgPX` (temporary, not a build dependency).
The separate toolkit and installed current/my builds were not changed.

### I/O-board save-state checkpoint — 2026-09-27

The original 837-8950-01 board now exposes `model1io::IoBoard::snapshot/restore`:
Z80 registers/internal latches, RAM, DPRAM, the existing 315-5338A and ADC state,
cabinet input latch, EEPROM contents/protocol/dirty flag, drive/lamp outputs,
panel bank selection and instruction overshoot debt. It reuses the shared chip
serialization; no new chip implementation, bus/timing behavior or disk writes.
Restore copies latches directly without reset, port-write replay, new ADC
conversion, EEPROM edges or DPRAM transfers. Firmware remains owned by the
destination board and must match; resource identity belongs to the future
machine envelope. Invalid CPU selectors, DPRAM dimensions and instruction debt
are rejected before changing the destination. This is not a hardened decoder
for arbitrary external data; bounded decoding and enclosing validation remain
machine-format work.

`model1board::IoBoard::snapshot/restore` replaces the advanced-only API with one
tagged `BoardState` for both revisions, retaining the V60-to-board fractional
clock remainder. Original/advanced and Wing War/R360 mismatches are rejected
without changing any state. Restore does not reapply constructor boot-status
seeds. The advanced board continues to use its existing device-state path;
there is no second implementation of it.

Verification: 446 workspace tests pass; offline release build passes. Four new
original-board tests exercise CPU continuation with different run partitions,
partially shifted ADC, partially read EEPROM, partial EEPROM command/data write,
staged DPRAM transfer, HALT/debt, output and input latches, resource ownership
and atomic rejection. The motherboard continuation test now covers all three
variants through a bincode round-trip into a fresh board; a fifth new test
rejects all six cross-variant restore combinations. EEPROM rollback/dirty state
is checked in memory, without persisting anything.

VR, Wing War and Wing War R360 also reach 600 debugger frames with identical
state lines, sampled 64 KiB + 4 KiB bus memory and PPM images compared with the
preceding audio-board build evidence. This checks ordinary execution without
using restore; it is not an actual-game save/load or gameplay certification.
Temporary evidence: `/tmp/tgpulse-iostate.SBtEXo`; suite/build logs:
`/tmp/tgpulse-iostate-workspace.log`, `/tmp/tgpulse-iostate-build.log`.

Complete Model 1 GUI save/load, machine continuation, network restore and
Libretro support are still pending. These device APIs are only captured between
run calls. No commit/push or deployment to toolkit/current/my in this checkpoint.

### Video save-state checkpoint — 2026-09-27

`Model1System::snapshot_video/restore_video` collects persistent polygon RAM,
TGP colour upload RAM, lighting parameters, both display-list buffers and their
control registers, tile/character RAM, palette and colour-translation RAM.
The existing scanner and renderer remain unchanged. Restore copies this state
directly: it does not replay uploads, rasterize a frame, switch lists, trigger
vblank/IRQs or write NVRAM. All dimensions and byte-derived lighting coefficient
ranges are checked before mutation. Version/resource identity and bounded
deserialization remain the responsibility of the future machine envelope.

The API captures between bus/run/render calls. Upload commands and list walking
are synchronous in this implementation; there is no suspended walker cursor to
serialize. A partially written next-frame list is retained verbatim. `frame_num`
is deliberately not duplicated in video state: it drives palette cycling and
automatic buffer selection but also the motherboard schedule, so the enclosing
machine snapshot must capture and restore it once, alongside timing state.

Polygon ROMs, frontend preferences (including smooth shadows), derived views,
sorted quads, CPU framebuffers and GPU objects are excluded. Source inspection
of the desktop GPU path confirms it rebuilds quad/tile/bin buffers on each
render; the frontend still must regenerate its output after whole-machine
restore, rather than present a stale frame. No GPU/desktop dependencies were
added to the core, and no Model 2 state format was changed.

Three new tests cover a bincode round-trip into a fresh machine after one-time
colour/light/polygon uploads have disappeared from the display list, identical
nonempty CPU 2D/3D pixels and GPU-input quad streams over subsequent vblanks,
and retained preferences. Another sequence resumes a partial inactive-list
write via the V60 bus and the automatic list switch without prematurely applying
its upload. Rejection checks cover every buffer dimension and invalid lighting
coefficients without changing existing state. The synthetic tests explicitly
supply the same frame counter; they do not claim full machine continuation or
actual GPU execution/save-load gameplay.

Verification: 449 workspace tests pass and the offline release builds. VR,
Wing War and R360 reach 600 debugger frames with state lines, sampled bus RAM
and PPM output identical to the preceding I/O-state build, and empty error logs.
This is ordinary-execution non-regression evidence, not actual-game restore or
new gameplay validation. Temporary evidence: `/tmp/tgpulse-videostate.ygTSzw`;
suite/build logs: `/tmp/tgpulse-videostate-workspace.log` and
`/tmp/tgpulse-videostate-build.log`. No commit/push or toolkit/current/my
deployment in this checkpoint.

### Motherboard save-state checkpoint — 2026-09-27

`Model1System::snapshot_motherboard/restore_motherboard` reuses the existing
V60/MB86233 serialization with main work RAM and battery-backed RAM (in memory
only), TGP data/coprocessor RAM, FIFO contents and halfword latches, pending
retry signals, math-unit/address latches, ROM-bank register and mapping latch,
timer period/count/last-read state, GLUE IRQ state, input/drive latches, frame
counter and fractional TGP clocks. Both CPUs' instruction budgets are retained.
ROM resources and the separately snapshotted video/I/O/audio/COMM devices are
not duplicated. Config, host handles and filesystem operations are excluded.

Important implementation choices, verified against the current execution paths:

- A positive TGP `icount` after a FIFO retry/HALT is abandoned quantum time,
  not invalid state or debt. Preserve it; the unchanged scheduler carries only
  negative instruction overshoot. The half-clock remainder is independent.
- Accept all 17 FIFO entries at the producer-HALT boundary, not only 16. Never
  discard the overflow word or replay a pending transfer on restore.
- `bank_base` cannot always be derived from `bank_reg`: a write whose low nibble
  is not 1 changes the register but intentionally retains the previous mapping.
- Restore IRQ lines/latches as captured, without calling `sync_irq`, reset or
  register handlers. The next normal scheduling boundary performs its usual work.
- Reject capture/restore while V60 bus execution or access-wait accounting is
  active. At a stable boundary these two transient fields are false/zero; CPU
  retry and instruction debt remain in the snapshot. No sub-instruction capture.
- Preserve the destination debugger's trace settings but clear stale trace and
  CPU coverage on successful restore. Direct and bincode snapshots have the
  same diagnostic policy. The bounded FIFO event ring is retained for diagnosis.

All fallible dimension/timing/selector checks precede mutation. Validation is
not a hardened untrusted-file decoder; outer allocation limits, ROM identity,
cross-device consistency and all-or-nothing machine restoration remain pending.
The currently exposed method restores this board only, not the complete machine.

Five new tests cover V60 IN retry without a duplicate pop/destination write;
TGP empty-FIFO retry, full-output HALT and fractional instruction debt; partial
RAM/FIFO transfers and auto-incrementing addresses; latched bank/NVRAM/frame
state; timer expiry with HALT/IRQ acknowledgement; malformed states and unsafe
capture boundaries rejected without mutation. The test harness aligns I/O and
audio via their existing APIs when comparing subsequent scheduler runs.

Verification: 454 workspace tests pass; offline release build passes. A temporary
ROM probe composes the existing device APIs (not a production machine format):
`vr`, `vformula`, `vf`, `swa`, `swaj`, `wingwar`, `wingwar360` run 600 frames,
serialize/deserialize each component into a newly initialized machine with the
same resources, then produce 30 identical frames under fixed default inputs.
Motherboard/I/O/audio state, audio samples and software-composited pixels match
each frame; final video state also matches. It uses ROM-supplied defaults only,
does not load/save personal NVRAM, and runs with COMM absent/no TCP. This is
bounded continuation evidence, not manual gameplay, GPU output, networking,
long-running determinism or full production save/load acceptance. R360/NetMerc
compatibility statuses are not promoted by it.

Temporary reproducible probe source/binary/log:
`/tmp/tgpulse-motherboard.vSGpIU/{check.rs,check,continuation.log}`. Suite/build
logs: `/tmp/tgpulse-motherboard-workspace.log`,
`/tmp/tgpulse-motherboard-build.log`. No new permanent runner, dependencies,
commit/push or toolkit/current/my deployment in this checkpoint.

### Whole-machine envelope checkpoint — 2026-09-27

`Model1System::save_state() -> Result<Vec<u8>, String>` and
`load_state(&mut self, bytes: &[u8]) -> Result<(), String>` assemble the
motherboard, I/O, video and audio APIs. The caller owns the bytes; the core
does not open files, write NVRAM, access host transports or reset devices.

- Format 1 uses `TGP1STAT`, explicit little-endian version/length, a resource
  identity and a payload checksum, followed by fixed-integer little-endian
  bincode. Incompatible future layouts must change the version; the existing
  Model 2 format is separate and unchanged.
- Identity hashes the normalized loaded ROM contents, optional DSB resources,
  I/O-board kind and COMM capability, not ZIP names or operator NVRAM/EEPROM
  defaults. It is cached at construction: ROM resources must remain immutable;
  recreate the machine if replacing them. SHA-1 reuses the loader dependency
  for identification/corruption detection, not authenticity/security claims.
- `MAX_STATE_BYTES` is 64 MiB. The core checks header, size, version, identity,
  exact payload length and checksum before bounded deserialization, which
  rejects trailing bytes. Frontends must enforce the same limit before file
  allocation. This is not exhaustive fuzzing or a hardened hostile-file claim.
- Read-only motherboard/video/I/O validation precedes atomic audio restore,
  the final fallible device step. Successful validation then commits the other
  devices without replaying register writes. Returned errors leave the live
  machine, NVRAM and pending audio unchanged. Normal allocation/panic failures
  are not a rollback guarantee.
- Successful restore preserves frontend settings/gains/mutes and debugger
  trace policy, clears stale core audio and diagnostics as documented by the
  component APIs. The frontend must flush its own output queues and redraw;
  that wiring is the next checkpoint.
- Capture/load require a stable execution boundary without a latched I/O,
  serial or DSB fault. Both are refused whenever `comm.is_some()`, including
  before link establishment: use `cabinet = single`. No independent rewind of
  a linked cabinet or restoration of TCP/peer state is implied. Coordinated
  network snapshots require a separate scope decision.

Five new tests cover the original/Wing War/R360 boards and optional DSB;
same-resource continuation without replacing preferences; malformed headers,
lengths, checksums, oversized/trailing payloads and forged vector lengths;
ROM identity across all resources; late audio and I/O variant rejection with
unchanged machine/audio; and COMM refusal before connection.

Verification: 459 workspace tests pass and the offline development release
build passes. A temporary probe uses the actual machine byte API (not manual
component assembly) on `vr`, `vformula`, `vf`, `swa`, `swaj`, `wingwar` and
`wingwar360`: save at frame 600, load into fresh same-resource machines, then
compare 30 frames of motherboard/I/O/audio state, samples and software pixels,
plus the complete encoded state immediately and after continuation. All match.
States are about 19.6 MB uncompressed. These runs use fixed default inputs and
ROM-supplied defaults, no personal NVRAM or TCP. They do not certify manual
gameplay, GPU/frontend behavior, arbitrary save points or networking, and do
not change the possible R360 compatibility follow-up.

Temporary evidence: `/tmp/tgpulse-envelope.3ycXBp/{check.rs,check,continuation.log}`;
suite/build logs: `/tmp/tgpulse-envelope-workspace.log` and
`/tmp/tgpulse-envelope-release.log`. No new dependency, permanent runner,
commit/push, personal settings/save change or toolkit deployment. Development
binary remains `target/release/tgpulse`; desktop Model 1 save/load is not yet wired.

### Desktop save/load checkpoint — 2026-09-27

- Existing Machine menu, keyboard bindings and touch actions now dispatch
  both machine types. Model 1 uses the new in-memory API from `app/state.rs`;
  Model 2 keeps the existing `savestate::save_to_file/load_from_file` path and
  format. No hardware-core filesystem dependency or Libretro adapter added.
- Existing slot convention remains `states/<set>.<slot>.state` relative to the
  emulator runtime directory, slots 0–9 (`F5` save, `F7` load, `F4/F6` select).
  Model 1 reads check regular-file size first and limit bytes actually read to
  64 MiB + a one-byte oversize sentinel, handling growth after metadata too.
- Capture/COMM/resource validation occurs before touching a slot. Writes use
  an exclusively created sibling, flush/sync, then rename over the destination;
  ordinary write/rename failure preserves the prior slot and removes only the
  owned temporary file. This is not a directory-fsync/power-loss durability claim.
- Only successful Model 1 loads discard the host audio ring and reset callback
  interpolation/DC-filter history at its next invocation. Audio already handed
  to the OS/in-flight callback cannot be recalled. Gains/volume stay unchanged.
  CPU image buffers are cleared, then regenerated with fresh GPU quads by the
  existing redraw path, also while paused; live widescreen detection still runs.
- Reset the wall-clock catch-up anchor after a successful Model 1 load, without
  changing pause/fullscreen preferences. Reset the periodic NVRAM countdown,
  avoiding an immediate flush; normal later periodic/session-close persistence
  still applies to the restored in-memory NVRAM. Load itself never writes it.
- Success (3 s) and error (10 s) notices render even with hidden menus or
  fullscreen, without taking input focus; detailed errors remain in the log.
  Expired notices are removed from captured UI draw data. COMM is refused by
  the core with a visible `cabinet = single` explanation, not silently ignored.

Four new automated tests cover Model 1 file replacement/reload, malformed,
missing and oversized files, rename failure cleanup; session output invalidation
only on success and unchanged persistent NVRAM; callback history reset with
unchanged gain; and ImGui feedback draw data with menus hidden/suppressed.
`cargo test --offline --workspace`: 463 passed; offline release build passes.
No new warnings beyond the existing m68000 lifetime / `block` compatibility notes.

An isolated VR desktop startup succeeded (temporary settings, volume zero and
ROMs read-only), but Computer Use could not identify the unbundled executable
as an app. No F5/F7 or visual GPU save/load acceptance is claimed; this remains
the next checkpoint/manual test. The owned test process was stopped; no user
configuration/NVRAM, toolkit or installed `current`/`my` binary was changed.
Logs: `/tmp/tgpulse-state-frontend-workspace.log`,
`/tmp/tgpulse-state-frontend-build.log`, `/tmp/tgpulse-state-ui.2kjUfP/run.log`.
Development binary: `target/release/tgpulse` (`tgpulse.dev`); no commit/push.

User follow-up: basic VR and SWA save/load work. F4/F6 changed slots but only logged
the selection; they now also use the existing three-second state notice,
visible with menus hidden/fullscreen. No slot numbering or bindings changed.
This user evidence does not certify other games or every paused/audio scenario.

Updated user priority: bring Z80 unification forward before full Model 1 save
states. Keep the general timing audit after feature implementations; the CPU
migration does not authorize a wider opcode or timing rewrite.

### Consolidate the Z80 implementations

**Implemented:** the original Model 1 I/O board now uses the local `z80`,
as I/O board 2 and Star Wars DSB already did. The registry `z80` dependency and
lockfile entry are removed. The former `tgpulse-z80` package now takes the name
`z80`, with an explicit local path dependency. Existing bus wiring, scheduler debt and the local
CPU implementation are unchanged. No full-machine save-state claim.

Model 2 does not execute either Z80 implementation: its I/O/drive handling is
high-level, and its sound CPUs are 68000. Nevertheless all nine README-declared
tested Model 2 sets were checked before/after migration at 600 frames:

| Board | Regression sets |
| --- | --- |
| Model 2 | `daytona`, `daytonase`, `vcop` |
| Model 2A | `srallyc`, `vf2` |
| Model 2B | `vstriker`, `schamp` |
| Model 2C | `hotd`, `waverunr` |

All debugger state/memory output and captured images match byte-for-byte.
The Model 2 debugger capture is its existing tilemap output, not a full 3D
renderer comparison. Model 1 has the same 600-frame comparison for `vr`,
`vformula`, `vf`, `swa`, `swaj`, `wingwar`, `wingwaru`, `wingwarj`, `wingwar360`.
All nine Model 1 sets also match at 1,800 frames after the same scripted coin
and analog-input sequence. Compared observations are the debugger's `state`,
64 KiB at `0x400000`, 4 KiB at `0xc00000`, and its PPM image; this is not a
complete-machine-state or audio-waveform comparison. The same address ranges
on Model 2 are sampled bus reads, not a claim that its memory map matches Model 1.
Checks use isolated temporary directories and no personal NVRAM. These are
bounded regressions, not fresh playability, listening or physical-input tests.

The workspace passes 316 tests, including the 18 existing CPU interrupt/state
tests and two new original-board execution tests (I/O and cycle slicing; HALT
without repeated side effects). Offline release build passes. The pre-existing
MultiPCM save-state draft is separate, unchanged and not certified by this suite.

Consolidation criteria retained for future CPU changes:

- Inventory existing Z80 consumers and preserve their I/O contracts. Optional
  interrupt/clock hooks must retain compatible default behavior.
- Compare reset, IRQ/NMI, EI/HALT, RETI, cycle accounting and representative
  execution before/after migration, separately from manual audio/gameplay.
- Move consumers to one implementation only after compatibility checks pass;
  then remove the redundant dependency and reconcile the lockfile.
- User decision: converge existing consumers onto the local `z80`
  adaptation, also selected for the new DSB. Keep its origin and delta auditable;
  keep existing consumers covered by regression checks.
- CPU hook/state tests and Wing War firmware validation remain prerequisites;
  NetMerc support is not. Do not migrate solely to eliminate duplication.

### Model 1 timing audit against MAME

**Status: completed as a bounded phase on explicit user request.** DPRAM,
FIFO/retry, V60/TGP clock, timers/IRQ, audio clocks, timed main UART and vblank
ordering have been assessed and the justified corrections implemented. See
the consolidated closure below for evidence and deliberately retained limits.
This does not establish full hardware accuracy.

- Inventory access waits across the Model 1 memory/device map: DPRAM, other
  mapped devices and coprocessor accesses. Distinguish explicit extra cycles
  from accesses for which the reference models no additional delay.
- Compare V60/TGP FIFO full/empty behavior, stall/HALT and resumption conditions,
  including interrupt and instruction-boundary interactions.
- Compare device clock ratios, fractional cycle debt, timers, interrupt delivery
  and serial scheduling between the main CPU, I/O board, TGP and sound devices.
- The DPRAM discrepancy is corrected for both board revisions, with per-title
  original-board regression checks recorded below. Preserve that bounded
  evidence separately from future FIFO and scheduler investigations.
- Record reference revisions and approximation boundaries. MAME's V60 uses an
  eight-cycle average per instruction, so agreement with MAME is not proof of
  cycle-exact hardware timing. Do not invent delays where the reference is
  incomplete, or replace real handshakes with forced ready values.

Deliver an evidence-backed discrepancy list before fixes. Apply confirmed
corrections in small, independently reviewable changes, with bounded traces,
slice-size/continuation tests where applicable and per-title regression checks.
MAME is not an automatic hardware-correctness oracle: choose the more plausible
observable result using device logic, independent invariants and available
hardware evidence. Keep valid implementation alternatives; identify assumptions
and confidence explicitly. The retrospective review below applies this criterion
to the already pending corrections.
Preserve frontend-independent emulated time and snapshot-relevant scheduling
state for a future Model 1 Libretro core. Keep automated timing/boot evidence
separate from manual controls, audio and gameplay validation.

### DPRAM checkpoint — 2026-09-27

Reference: local MAME commit `bd7e0b815842ec461e8ad2538d127f3332f5c96c`;
inspected files are unmodified.
`src/mame/sega/model1.cpp` maps `0xc00000..0xc00fff` through `dpram_r` with
`umask16(0x00ff)` for both I/O-board revisions. `dpram_r` subtracts one V60
cycle unless side effects are disabled. Writes call `mb8421::right_w` directly.
The Z80-side callbacks do not introduce this V60 wait. MB8421's BUSY outputs
are explicitly not emulated by MAME; no speculative contention model is added.

| Access | MAME reference | TGPulse after correction |
| --- | --- | --- |
| V60 read on connected low-byte lane | 1 extra cycle | 1 extra cycle, both board revisions |
| Unconnected high-byte-only read | No DPRAM handler | No extra cycle |
| Write | No explicit extra wait | No extra cycle |
| Debugger/renderer inspection | Side effects disabled | No extra cycle |

The change removes only the board-kind restriction in `model1.rs`; the shared
V60 core and Model 2 are unchanged. Expanded tests cover all three board kinds,
byte/word/dword accesses, unaligned and end-of-window reads, writes and host
inspection. A V60 instruction-fetch test verifies 8 base cycles plus one wait,
once only, with the extra cycle carried into the next slice.

Verification: 317 workspace tests pass; offline release build passes. Isolated
before/after runs reach 1,800 frames for `vr`, `vformula`, `vf`, `swa`, `swaj`,
`wingwar` and `wingwar360`. Captured images and the 4 KiB I/O window match for
all seven. Wing War/R360 debugger state and sampled memory remain identical.
Original-board PCs differ with the new budget; sampled 64 KiB RAM differs in
0/1/85/1/1 bytes respectively for VR/Virtua Formula/VF/SWA/SWAJ. These differences
are recorded, not asserted to be a complete-machine equivalence or a proven
gameplay defect. No personal NVRAM was loaded/saved by these boot probes.

The real-ROM VR MASTER/SLAVE/LIVE loopback test also passes 2,400 frames with
the last 600 online, using user fixtures read-only. It is an extra regression,
not a new LAN/manual-play certification. This checkpoint compares implementation
semantics against MAME source, not a fresh synchronized MAME execution trace;
manual audio/controls/extended play remain outside its evidence.

### FIFO checkpoint — 2026-09-27

Reference: the same local MAME revision as the DPRAM checkpoint, unmodified
`model1_m.cpp`, `gen_fifo.h/.cpp`, and V60 `op12.hxx` (`opINB/H/W`).

| Boundary | Finding / implementation |
| --- | --- |
| FIFO capacity / overflow | Both queues have 16 nominal words. MAME preserves the overflow word and halts its producer; existing TGPulse `len > 16` matches this boundary. No change to capacity or data ordering. |
| V60 low/high halfwords | Low write latches, high write pushes; low read pops, high read returns the latch. Existing ordering retained. |
| Empty result FIFO | Previously pumped TGP up to 2,000 extra 64-cycle runs, then committed a zero if still empty. Removed: V60 IN now yields before writing its destination, keeps its PC, and resumes when normal TGP execution produces a word. |
| Empty command FIFO | Existing TGP instruction retry retained and tested: PC, destination registers and address post-increment remain unchanged until a real word is available. |
| Debugger inspection | Peeks without popping, pumping the TGP or setting CPU wait requests. The low-half read still updates the halfword latch, like the reference driver. |

The V60 bus gains a default-false `take_io_stall` hook. Only IN.B/H/W consumes
that request, matching the reference; this is not a generic rollback mechanism
for arbitrary memory instructions. Model 1 owns separate pending-read and
retry-request flags; these must be included in its future machine snapshot.
There is no host-clock wait, ROM patch or manufactured successful FIFO transfer.
The core TGP implementation and Model 2 code are unchanged; Model 2 does not use
the V60. No new Model 2 runtime certification is implied by this checkpoint.

Five new tests cover IN widths and delayed completion, ordered overflow/drain,
both real CPU producers stopping after the overflow instruction and resuming,
TGP empty-read retry with address increment, and debugger/halfword behavior.
Workspace: 322 tests pass; development release builds offline.
Seven isolated Model 1 sets (`vr`, `vformula`, `vf`, `swa`, `swaj`, `wingwar`,
`wingwar360`) reach 1,800 frames with identical images and sampled DPRAM before/
after. CPU PCs and some sampled RAM bytes differ with corrected scheduling;
this is not full state equality or a manual gameplay/audio acceptance test.
The additional real-ROM loopback regressions pass for VR MASTER/SLAVE/LIVE
and Wing War MASTER/SLAVE (2,400 frames each, last 600 online). NVRAM fixtures
are read-only; these do not reopen or expand the closed networking milestone.

Remaining approximation: producer/consumer interleave uses 64-V60-cycle slices,
not MAME's event scheduler. Do not claim exact sync latency or whole-machine
slice-size invariance. This checkpoint identified half-clock loss in `(step * 5) / 2`
for odd-sized V60 slices and left instruction-budget handling to the separate
clock checkpoint below. General IRQ/audio timing remains open.

### V60/TGP clock checkpoint — 2026-09-27

Reference: the same local MAME commit `bd7e0b815842ec461e8ad2538d127f3332f5c96c`;
`model1.cpp` configures the V60 at 32 MHz / 2 and MB86233 at 40 MHz.
`mb86233.cpp::alu_post_2` charges one additional clock for floating-point ALU
operations (explicitly an assumed two-cycle cost in MAME). `schedule.cpp` uses
the final instruction counter to advance CPU-local time, including overshoot;
unused time while suspended is not a future execution credit.

| Boundary | Finding / correction |
| --- | --- |
| Fractional clock ratio | `(step * 5) / 2` discarded half a clock on each odd V60 slice. Retain a 0/1 numerator remainder across calls; 257 V60 clocks now provide 642 TGP clocks plus a half-clock phase, independent of partition. |
| Instruction overshoot | The TGP already charges two clocks for the corresponding ALU operations, but its next `execute` call replaces the budget. The Model 1 scheduler now deducts negative `icount` from the next allocation, retaining the completed instruction's extra clock. |
| FIFO stall / external HALT | Positive unused budgets are discarded, not banked. Fractional phase and existing negative debt advance during output-FIFO HALT, without executing instructions or creating a catch-up burst on release. |

Only `model1.rs` changes. The shared MB86233 core API and Model 2 scheduler are
unchanged; this is not a Model 2 clock audit or new Model 2 runtime validation.
No new opcode costs, wall-clock timing, busy waits or ROM patches are introduced.
The fractional phase is explicit machine-owned state; future full-machine
snapshots must include it together with the already serializable TGP `icount`.
This does not implement or certify complete machine save states.

Four synthetic tests cover one-cycle instructions under nine partitions,
all 13 two-cycle ALU operations under six partitions (CPU serialization matches
the unsplit run), zero/negative time requests, odd-clock HALT/debt retirement,
and empty-FIFO recovery without banking unused time. The equality claim concerns
an isolated TGP workload, not arbitrary CPU/device interleavings across the machine.

Verification: 326 workspace tests pass; offline release build passes. Seven
isolated before/after probes (`vr`, `vformula`, `vf`, `swa`, `swaj`, `wingwar`,
`wingwar360`) reach 1,800 frames with identical debugger state, 64 KiB sampled
backup RAM, 4 KiB I/O window and PPM images. Normal 64-cycle V60 slices already
have an integral 160-clock TGP allocation; the small-slice tests exercise the
fractional boundary directly. VR MASTER/SLAVE/LIVE and Wing War MASTER/SLAVE
loopback regressions also pass 2,400 frames, last 600 online, with NVRAM fixtures
read-only. No manual gameplay/audio or fresh synchronized MAME trace is claimed.

The following checkpoint covers Model 1 timer expiry/reload and IRQ delivery.
The 64-cycle interleave remains an approximation; do not interpret correct clock
totals as exact bus latency.

### Timer / IRQ checkpoint — 2026-09-27

Reference: local MAME commit `bd7e0b815842ec461e8ad2538d127f3332f5c96c`, unmodified
`src/mame/sega/model1.cpp` (timer and GLUE handlers) and
`src/devices/cpu/v60/v60.cpp` (IRQ acceptance / vector callback).

| Boundary | Finding / correction |
| --- | --- |
| Timer clock and registers | Existing period is `value * 0x800` V60 clocks. Zero stops the timer; period writes restart it; count writes do nothing. `timer_mode` is stored but has no modeled effect in MAME. Retained. |
| Timer expiry / reload | Previously reloaded a full period at the end of a slice, losing its overshoot. Now retains the residual phase, including multiple expiries in one advance. IRQ0 is a pending bit, not an event counter. |
| Mask semantics | Active-low masks prevent new timer/vblank raises; they do not clear previously pending levels. Masked timers continue counting/reloading; unmasking does not replay old expiries. Retained and tested. |
| Timer inspection | CPU reads update the stopped-count latch. Debugger reads now return the current count without changing that latch, matching MAME's side-effect-disabled access. |
| IRQ acceptance | Previously `sync_irq` overwrote `last_irq` even with CPU interrupts disabled, and the CPU reused a stale vector. A default-optional V60 bus callback now selects/latches the lowest pending level only when the CPU actually accepts it. |
| IRQ control / RETI | `0x20` clears the last accepted level, `0x10` clears all. Pending levels can select a new vector immediately after RETI within the same CPU slice. |
| UART IRQ mask write | Re-evaluates TxRDY/RxRDY immediately, like MAME `irq_mask_w`. V60 samples both edges of the live controller line at instruction boundaries, including an IRQ newly raised by MMIO during the current slice. |

The V60 bus callback defaults to the existing latched-vector behavior for simple
buses. Model 2 does not use the V60 and is unchanged. No new host resources,
global state or wall-clock dependency is introduced. Timer remainder, latch,
IRQ status/mask/last-accepted vector and CPU state remain explicit existing
fields to include in future complete machine snapshots; this work does not
complete or certify those snapshots.

Six new tests cover expiry boundaries and multi-period partition invariance,
mask/stop/restart and byte/aligned-word register accesses, debugger latch isolation,
IE-disabled acceptance and IRQ clear semantics, two real ISR/RETI sequences in
one slice, and UART unmask/interrupt entry immediately after a real OUT instruction.
Existing V60 tests retain coverage of the default bus callback and HALT wakeup.

Verification: 332 workspace tests pass, with an offline development release
build. Seven isolated before/after probes (`vr`, `vformula`, `vf`, `swa`, `swaj`,
`wingwar`, `wingwar360`) reach 1,800 frames; debugger state, sampled 64 KiB
backup RAM / 4 KiB I/O window and PPM images match byte-for-byte. VR
MASTER/SLAVE/LIVE and Wing War MASTER/SLAVE loopback regressions pass 2,400 frames,
last 600 online, with personal NVRAM fixtures read-only. These are bounded
regressions, not full-state equivalence, listening/gameplay validation or a new
synchronized MAME execution trace. Installed releases and other projects are
untouched.

Remaining boundaries: timer writes/reads and IRQ expiry delivery are still
quantized by the 64-V60-cycle scheduler; retaining the reload phase is not a
cycle-exact bus-time model. MAME raises vblank IRQ1 at scanline 384 of 424;
TGPulse currently couples it to `trigger_vblank` at the frame-step boundary.
Absolute vblank phase, renderer upload timing and COMM tick ordering need a
separate bounded follow-up, not a speculative change within this patch.
UART ready remains the existing high-level model polled after sound slices;
serial edge timing belongs to the next sound/serial audit.

### Retrospective decisions / FIFO retry completion — 2026-09-27

- **Keep clock fractions, instruction debt and periodic timer phase (high
  confidence in accounting):** elapsed emulated time must not depend on caller
  slice partition. This does not independently validate MAME's assumed TGP ALU
  instruction costs or make the coarse scheduler cycle-exact.
- **Keep FIFO wait instead of unbudgeted TGP pumping (high confidence in the
  transfer contract):** an empty read must not fabricate a completed zero-word
  transfer or grant extra CPU time. The previous absolute-address tests missed
  register side effects during source address decoding.
- **Complete IN retry:** IN.B/H/W now restores pre-decode architectural registers
  on a stalled transfer. Source auto-increment/decrement therefore happens only
  once on completion; the destination is still not decoded/written until then.
  This is a local rollback of CPU registers, not a generic rollback of MMIO
  side effects. Decoder scratch is rebuilt; no host state or new snapshot fields.
  The local MAME IN path decodes before checking its stall flag too; matching
  that ordering alone was not sufficient evidence of correct retry semantics.
- **Keep IRQ acknowledgement and side-effect-free timer inspection (high
  confidence in the existing controller contract):** clearing a different,
  merely pending interrupt is not equivalent to clearing the accepted one;
  debugger inspection must not change the stopped timer latch.
- **DPRAM +1 V60 clock remains provisional (limited independent evidence):**
  MAME and the already implemented advanced-board path support consistency, but
  no measured original-board bus trace establishes that one clock is better
  than zero. The Fujitsu MB8421 datasheet describes access timing/arbitration,
  not the motherboard's selected V60 wait count. Do not present this as a
  hardware-proven delay or revert it without better evidence.
- **Keep immediate UART-ready evaluation on unmask, qualified:** coherent for
  the present level/ready model, but TxRDY is currently always true. This is not
  evidence of accurate serial transmission timing (assessment below).

Two new V60 regression tests exercise repeated stalls across all three transfer
widths, source auto +/- including SP, destination auto-increment, and a genuine
zero result. The independent reproducer now retains R1=`D80000` during the stall,
retries `D80000`, and finishes at `D80004`, rather than reading `D80004` and
finishing at `D80008`. No affected in-game sequence has been established.

### Audio / serial clock assessment — 2026-09-27

Reference: local MAME `bd7e0b815842ec461e8ad2538d127f3332f5c96c`, unmodified
`src/mame/shared/segam1audio.cpp`, `src/mame/sega/dsbz80.cpp`, and `model1.cpp`.
The decision criteria include arithmetic clock conservation and 8N1 framing,
not only matching another emulator's output.

| Path | Evidence and decision |
| --- | --- |
| V60 16 MHz -> audio 68000 10 MHz | Existing conversion retains the 5/8 remainder and instruction overshoot. MAME labels the 68000 clock verified on hardware. Keep; no new timing constant. |
| MultiPCM / FM | Existing PCM period is 224 sound clocks; MAME records DAC WORDCLK = 10 MHz / 224. FM conversion retains its 4/5 ratio and native 144-chip-clock phase. Keep the causal integration filter: differing from MAME's resampler is not itself a timing bug. |
| Sound CPU STOP | Chips and DSB must continue while the CPU is stopped. New tests independently check sample count against floor(sound clocks / 224), nonzero audio, equal continuation under 1/3/64/4097-main-clock partitions, and DSB elapsed-time conversion. Existing production behavior passes; no correction justified. |
| 68000 -> DSB | Existing clocked UART preserves framing, backpressure, fractional time and in-flight snapshot continuation. New framing test verifies exactly ten 16-tick bits for 8N1: 160 ticks at 500 kHz = 320 us. Keep. |
| DSB Z80 4 MHz | MAME explicitly calls this an estimated clock. Keep provisionally, not relabeled as a measured hardware frequency by passing arithmetic tests. |
| V60 <-> sound 68000 | Missing timed link, not merely a different scheduler: `send` delivers a byte immediately, receive storage queues up to eight bytes, TxRDY is always true, and IRQ2 is injected per consumed byte. Functional HLE, not equivalent serial timing. No arbitrary per-byte delay added. |

No production audio clock, gain, chip core or serial behavior changes in this
checkpoint. The three new audio/serial tests supplement existing FM/DSB
partition, waveform and in-flight restore tests. Model 2 production code is
unchanged; the shared audio board receives tests only.

Verification of this checkpoint plus the retry correction: 337 workspace tests
pass; the development release builds offline. Seven isolated Model 1 sets
(`vr`, `vformula`, `vf`, `swa`, `swaj`, `wingwar`, `wingwar360`) reach 1,800 frames
with identical before/after debugger state, sampled 64 KiB backup RAM / 4 KiB
I/O window and PPM images. VR MASTER/SLAVE/LIVE and Wing War MASTER/SLAVE
loopback regressions pass 2,400 frames, last 600 online, using NVRAM read-only.
This is not a fresh listening/gameplay acceptance test or hardware timing trace.

**Plan at that checkpoint (now completed below):** inventory the actual mode/command
sequences used by Model 1's two main-link UARTs and reuse/adapt the existing DSB
i8251 subset where compatible. Then introduce a frontend-independent timed
V60/68000 link with explicit shift/holding state, ready/IRQ semantics and
mid-character continuation tests. Change both ends coherently; a delay on the
existing queue alone would not model transmitter backpressure. Keep the shared
Model 2 board's existing path separate unless specifically extending the scope.
Vblank was included in the consolidated closure; full machine save states
remain separate pending work.

### Consolidated timing closure — 2026-09-27

User request: combine the remaining checkpoints into one completion phase.

| Status | Checkpoint | Decision / evidence |
| --- | --- | --- |
| 🟢 | Access waits, FIFO/retry, clock debt and timers/IRQ | Earlier checkpoints and retrospective decisions above remain applicable; no new speculative wait constants. |
| 🟢 | Audio clock conservation | Keep the measured/reference ratios and existing converters; independent STOP/partition/sample-count tests pass. |
| 🟢 | V60 ↔ 68000 UART | Replace the Model 1 immediate queue/reply slot with two clocked i8251 endpoints; shared implementation with DSB, 500 kHz / 8N1 x16. |
| 🟢 | Vblank / uploads / COMM | Scan the completed display list before automatic next-buffer selection. Retain one COMM tick followed by IRQ1 per frontend-driven frame. |
| 🟢 | Consolidated regressions | 344 workspace tests, offline release, seven Model 1 and nine Model 2 ROM probes; VR/Wing War loopback regressions. |

Legend: 🟢 completed. Closure covers the bounded audit, not every possible
hardware timing question or manual gameplay acceptance.

**UART rationale and implementation.** Firmware traces from VR, VF, SWA and
Wing War variants use the supported `00 00 00 40 4e 37` reset/8N1 initialization
where observed; no unsupported mode/command was encountered in the seven-set
1,800-frame regression. A real shift register takes time to transmit a byte,
has a bounded holding register, and can overrun its one-byte receiver. Those
properties independently justify replacing instantaneous queued delivery; the
baud clock reference is MAME's Model 1 wiring, not inferred from matching PCs.
`i8251.rs` now owns the single UART implementation used by both the main link
and DSB. The existing Model 2 HLE path is deliberately unchanged.

The 68000 TX pin fans out to the main receiver and optional DSB receiver; the
Model 1 production path no longer runs a second DSB-owned copy of that sender.
The DSB-owned sender remains available to isolated existing board fixtures.
STOP continues advancing the serial/chip clocks. IRQ2 is injected only when
RXRDY is live and the 68000 can accept it, avoiding stale queued interrupts
after a masked polling read. Main TX-ready is no longer forced true.
Unsupported framing/commands and writes to a full holding register surface as
machine errors, not silent fallback to HLE. RX overrun remains a status bit.

New tests cover duplex delay/backpressure, receiver overrun, serialized
mid-character continuation, STOP with masked polling/no phantom interrupt,
actual IRQ2 wake/receive, and 68000-pin fanout to both main and DSB under
different run partitions. `SerialState` contains only emulated endpoints,
phase and fault; the in-memory snapshot/restore API is not a complete board
snapshot. Full restoration must also restore the CPU, run/conversion debt,
DSB and audio devices together. No host resources or filesystem writes added.

**Vblank rationale.** MAME performs `tgp_scan` before `end_frame`. Independently,
consuming the completed buffer before changing its selector prevents reading
an unfinished following list. A two-buffer synthetic upload test establishes
that ordering; automatic alternation still occurs every other frame. The
frontend continues to define a frame boundary at vblank, with 656 × 424 V60
clocks between edges. MAME labels its edge scanline 384 of 424; TGPulse exposes
no beam-position counter here. Relabeling the frame origin alone would not
establish better timing, so no unsupported scanline phase shift was introduced.
COMM and IRQ1 remain ordered at that edge; IRQ masking does not stop COMM.

**Verification.** `cargo test --offline --workspace`: 344 passed. Offline
development release builds. Seven isolated sets (`vr`, `vformula`, `vf`,
`swa`, `swaj`, `wingwar`, `wingwar360`) reach 1,800 frames with no logged errors;
their debugger state, sampled 64 KiB backup RAM / 4 KiB I/O window and PPM
images match the pre-UART baseline. All nine Model 2 sets listed in the Z80
matrix above match the same bounded observations at 600 frames before/after.
Model 2 captures are the debugger's tilemap output, not full 3D comparisons.
VR MASTER/SLAVE/LIVE and Wing War MASTER/SLAVE pass 2,400-frame loopback probes,
last 600 online. Personal NVRAM fixtures are read-only. No new synchronized
gameplay, listening or hardware trace result is claimed. Temporary evidence:
`/tmp/tgpulse-timing-final.PLXCTB` (not a permanent repository dependency).

**Retained limits.** V60 devices/IRQ observation remain on the 64-clock scheduler
grid; 68000 MMIO is instruction-boundary, not microcycle accurate. DSB pin
sampling retains Z80 instruction overshoot; chunking at serial-clock edges
limits producer batching but does not prove hardware-edge equivalence. The
4 MHz DSB CPU clock and DPRAM's extra cycle retain their previously documented
confidence limits. The existing constructor's reset/first-instruction timing
is unchanged. No sub-instruction raster/beam model, full-machine save state,
NetMerc/R360 playability, or complete audio fidelity claim follows from closure.
At this timing checkpoint, fresh listening, extended gameplay and the MultiPCM
distortion investigation were still separate; audio and Model 1 save states were
later closed by user acceptance as recorded above. No broader hardware-fidelity
claim follows from that acceptance.

## Verification

No ROM data is stored in Git. Automated regression tests run offline:

```sh
cargo test --offline --workspace
TGPULSE_MODEL1_TEST_ZIP="$PWD/roms/swa.zip" cargo test --offline -p tgpulse-core \
  legacy_and_corrected_tgp_load_identically -- --ignored
```

The opt-in test works with either dump revision, verifies both SHA-1 values,
idempotence and the actual loader path. Run it on each affected ZIP. Normal tests
also reject an unknown program even if its two instruction words match the old
dump, and cover factory NVRAM precedence and timer register behavior.

Manual acceptance still needed: VF hair/physics and gameplay after timer changes,
SWA graphics/collision behavior with the repaired program, audio and extended play.
Passing unit tests or `--list` alone does not establish those results.

The Model 2 Manx TT sound-board selection issue is outside this Model 1 change.
Publication and toolkit/current release replacement require separate requests.

### Local verification — 2026-09-27

- 95 workspace tests passed; the one ROM-dependent test is excluded by default
  and was explicitly run successfully on all seven affected sets, both original
  and updated ZIPs (14 runs).
- Updated only `315-5711.bin` in the seven local `roms/` archives. All other member
  hashes were verified unchanged. Original ZIPs are retained under
  `roms/.backup-315-5711-20260927/`; the separate MAME collection is unchanged.
- A size scan across all 100 local sets found NetMerc's sound reload to be the
  only load where a present ZIP member exceeds the database's declared length.
- Offline release build passed. `tgpulse.dev --list` lists 100 sets; this is a
  launcher check, not a completeness/CRC audit.
- VR, VF and SWA each executed 120 debugger frames without a crash, with an
  isolated temporary working directory and no user NVRAM writes. This short
  smoke test does not validate a complete boot, visuals, sound or gameplay.
- These checks precede publication; publication revisions are recorded in Git
  history. No toolkit release update was performed.

### NetMerc factory NVRAM recovery — 2026-09-27

- The inspected local Model 1 and MAME archives lacked the file. Downloaded the
  author's separate attachment above and verified length, CRC32 and SHA-1 against
  the local MAME driver before installation.
- Added only `netmerc_nvram.bin` to local `roms/netmerc.zip`; every pre-existing
  member's SHA-1 was unchanged, including the corrected TGP program. Backup:
  `roms/.backup-netmerc-nvram-20260927/netmerc.zip`.
- In an isolated debugger run, read all 65536 bytes at `0x400000` before execution:
  their SHA-1 matched the factory image exactly. Then executed 120 frames without
  a crash. `tgpulse.dev --list` now reports NetMerc with no missing database files.
- No existing user saves, MAME archives, toolkit files or executable changed.
  No new claim of playable NetMerc: I/O board 2 and further integration are still
  outstanding. This recovery required no emulator code changes.

### I/O board 2 shared-peripheral checkpoint — 2026-09-27

- [Implementation contract](MODEL1_IOBOARD2.md) records the MAME revision,
  memory/port maps, firmware, cabinet wiring and remaining CPU/IRQ work.
- Extracted the 315-5338A and MSM6253 into crate-private reusable modules used
  by the existing board. Corrected power-on output latches to `FF` per MAME;
  no advanced-board firmware or new game support is enabled yet.
- Added 14 ROM-free tests. `cargo test --offline --workspace`: 109 passed,
  one ROM-dependent test ignored (not rerun for this peripheral-only change).
- `cargo build --offline --release -p tgpulse` passed. The existing dependency
  warning about future Rust compatibility of `block` 0.1.6 remains unrelated.
- Compared the previous and new release builds on `vr`, `vformula`, `vf`, `swa`
  and `swaj`, each in an isolated temporary working directory. At frame 120,
  complete debugger output matched byte-for-byte: `state`, 64 KiB at `400000`
  and the 4 KiB V60 window at `C00000` containing the 2 KiB dual-port RAM.
  This is a bounded regression check, not complete CPU-state equality or proof
  of gameplay/audio/rendering correctness.
- The development executable is `target/release/tgpulse`. No user ROMs, saves,
  bindings, MAME checkout or toolkit-managed release was modified. Publication
  was subsequently authorized; publication revisions are recorded in Git.

### Libretro-oriented integration policy — 2026-09-27

- Recorded the general Model 1-first architecture criterion in `AGENTS.md` and
  the board integration contract: frontend-independent execution, lifecycle,
  inputs, audio/video, resources and portability, including (not limited to)
  serialization. No Libretro adapter or complete machine save-state claim.
- Added serialization for the two extracted I/O chips using existing serde /
  bincode dependencies. Two additional tests restore a configured host transfer
  and a partially consumed ADC sample, then verify identical continuation.
- Re-ran `cargo test --offline --workspace`: 111 passed, one opt-in ROM-dependent
  test ignored. No runtime behavior or persistent user-file format was changed
  by these serialization derives; the snapshot APIs are still crate-private.

### I/O board 2 bus / CTC / interrupt checkpoint — 2026-09-27

- Added an isolated advanced-board bus using the existing 315-5338A, MSM6253
  and EEPROM components. ROM/RAM mapping, physical pin wiring, ADC mirrors,
  EEPROM protocol and host dual-port RAM access are covered without game ROMs.
- Implemented four-channel CTC timing/counters and nested IRQ service, the
  TMPZ84C015 priority register/port mirrors and internal watchdog. Clocks are
  emulated integers; CTC output edges retain their order and clock offsets.
- Added peripheral-bus snapshots without firmware/host resources, plus tests
  for continued execution across snapshots and different time-slice lengths.
  The bus remains independent of the desktop frontend and does not own a CPU.
- Inspected Z80 1.0.2 and recorded the remaining CPU integration requirements:
  explicit acknowledge/RETI hooks, canonical RETI IFF behavior and complete
  CPU state access. The installed dependency and all existing game execution
  paths are unchanged. SIO/PIO and other known missing devices fail explicitly
  at the new bus boundary rather than returning an invented ready status.
- Added 28 ROM-free tests. `cargo test --offline --workspace`: 139 passed,
  one opt-in ROM-dependent test ignored. Offline release build passed; no new
  dependencies were installed. The pre-existing `block` warning remains.
- At 120 frames, the same debugger state, 64 KiB main NVRAM and 2 KiB shared
  RAM snapshots still match the earlier baseline for `vr`, `vformula`, `vf`,
  `swa` and `swaj`. This verifies that the unchanged board-1 game path remains
  stable in this bounded check, not that the new board firmware has booted.
- No advanced-board firmware has been run and no new game support is enabled.
  The next checkpoint is the CPU adaptation and remaining SIO/PIO work before
  Wing War firmware handshake testing. Changes remain local pending publication
  authorization; MAME, toolkit releases and user ROMs/saves were not modified.

### I/O board 2 isolated Z80 checkpoint — 2026-09-27

- Added local `tgpulse-z80` (now named `z80` after consolidation), based on the already installed MIT-licensed Z80
  1.0.2 source. Original authors, source hash, minimal code delta and limitations
  are documented in [the component README](../crates/z80/README.md).
- Added live interrupt acknowledgement, canonical RETI notification/IFF
  restoration, integer emulated-clock callbacks and typed CPU snapshot/restore.
  Corrected EI delaying NMI in this local adaptation only. The original registry
  dependency and the running first-generation I/O board remain unchanged.
- Added 18 synthetic integration tests, all passing in development and release
  profiles. State tests verify registers and identical continued execution at
  16 cut points through a block transfer, IM2 service, port I/O and HALT.
- `cargo test --offline --workspace`: 157 passed, one opt-in ROM-dependent test
  ignored. `cargo build --offline --release -p tgpulse` passed. The existing
  `block` 0.1.6 future-compatibility warning remains unrelated.
- At 120 debugger frames, `vr`, `vformula`, `vf`, `swa` and `swaj` still match
  the preceding baseline byte-for-byte for reported CPU/FIFO state, 64 KiB
  main NVRAM and the 4 KiB window containing shared RAM. Runs used an isolated
  temporary working directory and did not write user saves. This is a bounded
  old-path regression check, not a gameplay or new-firmware boot test.
- Checkpoint reached before SIO/PIO: CPU-to-bus wiring, combined board/scheduler
  snapshots and firmware execution are still pending. IM0 timing, undocumented
  RETI aliases and asserted-NMI behavior are not certified by this adaptation;
  it remains instruction-stepped, not a T-state-accurate bus implementation.
- Development executable: `target/release/tgpulse`. No toolkit installation,
  ROM/save/configuration change, dependency download, commit or push performed.
  Z80 unification remains the separate follow-up above, after board validation.

### I/O board 2 CPU/SIO/PIO checkpoint — 2026-09-27

- Connected the adapted CPU to the actual advanced-board bus. CTC/SIO/PIO
  acknowledge and RETI use live priority/service state; unsupported operations
  latch a diagnostic fault and prevent later instructions from running.
- Added PIO register/bit-control operation and a bounded asynchronous SIO model
  clocked from CTC2/3, with FIFO/error handling and timestamped serial pin
  events. Unsupported modes fail explicitly. This is not full SIO fidelity;
  supported formats and remaining timing/protocol limits are documented in
  [the integration contract](MODEL1_IOBOARD2.md#fourth-checkpoint--cpubus-wiring-and-bounded-siopio).
- Added combined CPU/peripheral/scheduler snapshots. Tests check continuation
  inside interrupt service and serial transfers, integer cycle debt, no replayed
  output events, and identical results for whole versus one-clock time slices.
- 21 new ROM-free tests; `cargo test --offline --workspace`: 178 passed,
  one opt-in ROM-dependent test ignored. Release-profile core tests also passed
  (77 passed, the same opt-in test ignored). Offline release build passed. The
  existing `block` 0.1.6 warning is unrelated.
- At 120 frames the same debugger CPU/FIFO and memory snapshots match the
  previous baseline for `vr`, `vformula`, `vf`, `swa` and `swaj`. This checks
  unchanged first-generation game paths, not advanced-board gameplay.
- First isolated, hash-verified firmware probes: Wing War executes 10,000,002
  clocks but remains polling `F080=01` for `02` at `0830`; NetMerc stops on its
  diagnostic LCD write at instruction `03A9` (memory `8005`, 121479 clocks).
  Neither established a complete handshake; main-board/serial peers were absent.
  No firmware wait was patched or missing-device error ignored.
- **Next (priority clarified):** diagnose Wing War's wait against the reference.
  Add the diagnostic LCD only if that path proves necessary for Wing War;
  defer NetMerc-specific blockers to its separate milestone. Keep game selection
  disabled until the corresponding title's initialization/handshake checks pass.
- No ROM/save/configuration changes, toolkit deployment, new software download,
  commit or push. Development executable remains `target/release/tgpulse`.

### Wing War reference-trace checkpoint — 2026-09-27

- Diagnosed `F080=01` without changing emulation code or patching the firmware.
  The earlier ~1.017-second probe ended during a normal EEPROM read sequence.
  TGPulse reaches `F080=02` at 21,305,943 board clocks (~2.167352600 s);
  the installed MAME 0.289 binary reaches the same transition at ~2.167353312 s.
  Both complete the same 64-word blank-EEPROM buffer and clear the same transfer
  state. This is not a general cycle-accuracy or full-machine equivalence claim.
- The isolated TGPulse board runs for 20 seconds without a bus fault; normal
  Wing War initialization did not need the diagnostic LCD. With no main-board
  requests supplied, DPRAM remains zero and the firmware stays at state `02`.
  The fresh MAME three-second trace continues into subsequent states with its
  main CPU present. No successful host exchange is synthesized in TGPulse.
- See [the comparison and limits](MODEL1_IOBOARD2.md#wing-war-eeprom-initialization-comparison--2026-09-27)
  for reference binary/source versions, trace points and isolated test setup.
  Offline core rebuild passed; workspace tests remain 178 passed, one opt-in
  ROM-dependent test ignored. No production code or release binary changed.
- **Next:** validate the main-board/DPRAM request-response contract against
  Wing War, then wire the advanced board and its clock into the existing system
  boundary. World/US/Japan boot, controls and EEPROM persistence remain pending;
  R360 and NetMerc remain separate. No commit, push or deployment performed.

### Wing War motherboard integration checkpoint — 2026-09-27

- Replayed 619 real MAME host writes against the isolated board: sampled
  firmware states, final 2 KiB DPRAM and 128-byte EEPROM match the reference.
- Added the narrow `model1board` boundary and enabled firmware/clock selection
  for World/US/Japan only. Inputs and NVRAM use existing interfaces; no new GUI
  controls. Advanced snapshots include the fractional V60/board clock ratio.
- Fixed the integrated I/O timeout by modeling MAME's one-cycle DPRAM read wait
  on this new path. Original-board timing remains isolated pending its audit.
  Also fixed V60 scaled negative-index overflow in debug builds without changing
  release arithmetic. Unsupported board accesses stop with a reported error.
- All three base sets pass boot and in-memory NVRAM reload; digital/analog
  changes reach the expected DPRAM bytes. World/Japan attract-mode 3D frames
  were inspected. Manual controls, service-menu option edits, audio and extended
  gameplay remain unvalidated. R360, NetMerc and link play remain separate.
- 187 tests passed, one ROM opt-in ignored; offline release build passed.
  The five existing Model 1 debugger/memory baselines remain byte-identical.
  See [the detailed evidence and limits](MODEL1_IOBOARD2.md#wing-war-dpram-and-motherboard-integration--2026-09-27).
- Development binary: `target/release/tgpulse`. No user saves/settings/ROM ZIPs,
  MAME or toolkit installations changed; no commit/push performed.
