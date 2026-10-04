# NetMerc MVD Sensor Input — 2026-10-04

## References And Approved Adaptation

SM2 `PORTING_PLAN.md` phases 3.1–3.2 and 3.7 define frontend-owned input,
full/reduced profiles, lifecycle discipline and consistent live options.
Supermodel `Docs/ROADMAP.md` provides delivery conventions but neither
reference implements NetMerc MVD. The functional reference is TGPulse-Next
`input/motion.rs`, `input/mvd.rs`, its sensor ownership and MVD GUI feedback,
reviewed at upstream `f303b712455571b60cbea2249e1022935d398e22`.

The neutral orientation/calibration helper is imported at its upstream path.
A small timing constructor preserves its default two-second warmup and
five-second total window for standalone callers. The Libretro adapter selects
a one-second warmup and three-second total window, as requested by the user.
The 100-sample minimum, 0.1 rad/s mean limit and 0.02 rad/s deviation limit
remain unchanged. Low sampling rates reject calibration with a reason;
there is no successful calibration based only on elapsed time.

No SDL handles, host devices or frontend callbacks enter the emulated machine.
The existing native pose conversion retains neutral offsets and polarity.

## Options And Bindings

All six options remain visible without a loaded NetMerc title; application
is restricted to NetMerc. Changes take effect immediately.

| Option | Values | Default |
| --- | --- | --- |
| Sega NetMerc MVD Input | Auto / Off (Fixed Camera) / Right Stick / Sensors (e.g. DualSense) | Auto |
| Sega NetMerc MVD Horizontal Range | Off / 10–90 Degrees, Step 10 | 30 Degrees |
| Sega NetMerc MVD Vertical Range | Off / 10–90 Degrees, Step 10 | 20 Degrees |
| Sega NetMerc MVD Gravity Stabilization | Enabled / Disabled | Enabled |
| Sega NetMerc MVD Drift Compensation | Off / 10–100%, Step 10 | 50% |
| Sega NetMerc MVD Sensor Diagnostics | Disabled / Enabled | Disabled |

Auto prefers accepted sensor orientation, then the P1 right stick, then a
fixed camera. Explicit Sensors uses a fixed camera until orientation is
ready; this approved adaptation differs from the standalone's stick fallback
inside its Sensors path. Ranges apply only to stick input. Gravity corrects
pitch/roll, not yaw; absent accelerometer support leaves gyroscope tracking
available. Right Stick and Off release sensor acquisition.

Existing full and reduced **Special: Sega NetMerc** profiles retain P1
Calibrate (X/North) and Recenter (Y/West). These commands are virtual, do not
alter cabinet switches and fire once per press. Already-held buttons must be
released after reset, state restoration or P1 device changes.

## Frontend ABI, Units And Timing

The adapter negotiates `GET_SENSOR_INTERFACE` (experimental command 25), then
requests P1 gyroscope and accelerometer at a 120 Hz hint. Successful interface
negotiation alone is not proof of available sensors. Gyroscope enable failure
produces one unavailable notification per activation; optional accelerometer
failure reports gyroscope-only operation.

The helper expects SDL axes. The axis conversion follows the current primary
[RetroArch SDL2 sensor implementation](https://github.com/libretro/RetroArch/blob/master/input/drivers_joypad/sdl2_joypad.c):

- Gyroscope SDL X/Y/Z = Libretro X/Z/-Y, in rad/s.
- Accelerometer SDL X/Y/Z = Libretro X/Z/Y, multiplied by 9.80665 from g to m/s².

The frontend sensor ABI provides scalar readings without original sample
identifiers or device timestamps. Host monotonic polling time is therefore
an explicit adaptation, not an original sensor event timestamp. Polling occurs
at most once per callback and at most 120 times per second; faster callbacks
cannot fabricate extra observations. This interface cannot prove that every
reading is a new device event or distinguish stationary zero readings from
an unreported disconnection/stale driver data. Physical driver compatibility
and motion acceptance are not claimed by mock ABI tests.

A callback gap greater than 250 ms discards orientation and restarts
calibration; movement is never integrated across a pause. Nonfinite gyroscope
readings stop tracking and report an error. Accelerometer anomalies follow the
upstream gravity validity checks. Disable, P1 device change, reset, successful
state restore, content replacement, unload and deinitialization release enabled
sensors. Host estimator state and timestamps are outside machine save states
and NVRAM. The original U5 delivery retained state format 4; the subsequent
startup publication correction below introduces format 5. Destination options
are retained.

## Feedback

Existing queued extended RetroArch notifications report the three-second
stationary request, successful calibration, rejection with reason, unavailable
sensors, invalid readings, optional gyroscope-only operation and recenter
success/not-ready. Holding a command does not renew notifications. Frontends
without the extended message interface retain the existing legacy fallback.
The manual game-acknowledged Holder notices are unchanged; automatic Holder
sequencing remains outside U5.

## Verification

- 71 adapter tests pass, including the imported neutral estimator tests,
  three-second timing/reset policy, undersampling rejection, polling gaps,
  axis mapping, gyroscope-only capability and shutdown.
- Existing isolated ABI controls runner passes all ten sets, profile variants,
  native alias/axis behavior, virtual command edges and state replay.
- New `tools/test_libretro_netmerc_sensors.py` reuses that host and tests real
  monotonic three-second calibration with mock sensors, queued modern OSD,
  recenter edges, state/reset/device shutdown, reduced profile, partial sensor
  capability, unavailable fallback and unload.
- Nine-set comparison with the preceding Development binary preserves frame,
  audio, Save RAM and state continuation results.
- Isolated RetroArch Vulkan delivers 240 frames with Sensors selected. The
  local `mfi` joypad driver supplies no usable sensors: the core uses a fixed
  camera and RetroArch accepts its queued unavailable notification. This is
  frontend integration evidence, not a physical motion-controller test.
- macOS release build and native artifact/ABI/dependency gates pass.
- Development core and matching info installed; build and installed SHA-256:
  `763d38516f3ce14fc5f60bc3c04215ab0c3eb4cb5999d4e13965ba99573d7bb0`.

Local evidence: `/private/tmp/tgpulse-u5/`. Physical sensor/controller trials
remain user-run and are separate from these automated checks. No source
commit, push or publication is included in this phase.

## Calibration Tolerance Review — 2026-10-04

The user reported stationary-pad failures around the second/third attempt,
followed by consistently successful retries. The user does not recall the
rejection reason; this is not established as a first-start-only symptom.
The exact rejection text has not yet been supplied;
the cause remains unconfirmed. The mean limit (0.1 rad/s), deviation limit
(0.02 rad/s) and 100-observation minimum match the reviewed upstream helper.

A standalone probe using the actual neutral helper and three-second timing
reproduces these cases without accessing a physical controller:

| Synthetic Input | Measurement Observations | Outcome |
| --- | --- | --- |
| Stationary, Native 57.524160 Hz Polling | 115 | Accepted |
| Stationary, 60 Hz Polling | 120 | Accepted |
| Stationary, 50 Hz Polling | 100 | Accepted |
| Stationary, 49 Hz Polling | 98 | Rejected: Too Few Samples |
| Native Polling, One 0.22 rad/s X Spike During Measurement | 115 | Rejected: X Deviation 0.020515 rad/s |
| Repeat With Stationary Data And No Spike | 115 | Accepted |

Compared with the standalone's three-second measurement interval, the
adapter's two-second measurement interval leaves a smaller sample margin.
A short startup scheduling slowdown can therefore reject perfectly stationary
data; one transient reading can also reject the complete window. These are
reproduced possibilities, not measurements of the user's pad.

The next decision depends on the actual rejection category: insufficient
observations require a sampling/window policy review; excessive deviation
requires captured noise/transient evidence; excessive mean requires bias
review. Keep the three-second request and movement thresholds as implemented
until that evidence distinguishes the cause. Do not increase all tolerances
from this symptom alone. Probe source/output remain in
`/private/tmp/tgpulse-u5-calibration-review/`.

## Startup Camera Review — 2026-10-04

The user reports a temporary down/right camera at startup in multiple MVD
modes, including Off (Fixed Camera). Fixed Camera is a control case, not the
scope of the symptom. No runtime policy was changed during this review.

The adapter supplies the neutral `HmdPose` before every frame in Off mode.
Sensor acquisition is stopped in this mode. However, this updates the serial
tracker's requested pose, not the game's current pose in shared DPRAM.
The MAME-derived initialization writes DPRAM offsets 0x80–0x8b only at machine
construction; save-state restoration deliberately retains the saved bytes.

A headless probe linked against the current release machine library, using
real NetMerc ROMs and a constant neutral pose without a physical controller,
observed these DPRAM words:

| Frame | XYZ / Orientation | Observation |
| --- | --- | --- |
| 0 | 0, 0, 0 / 12868, 25736, 12868 | Correct power-on initialization |
| 255 | 0, 0, 0 / 0, 0, 0 | Startup clears the previously initialized pose |
| 928 | 0, 0, 0 / 12868, 25736, 12868 | Serial measurement delivery restores neutral pose |

The zero-pose interval lasts 673 frames, about 11.7 emulated seconds at native
Model 1 timing. This demonstrates a shared startup continuity problem that is
independent of SDL sensors; it does not by itself prove the user's exact
rendered symptom or identify the instruction that clears the words. The probe
uses the ROM loader's initial image and scripted inputs, not the user's saved
NVRAM. The upstream tracker and cold-boot initialization use the same policy.
Do not apply unconditional pose writes per frame or on Save State restoration:
a correction must preserve real serial protocol and restored machine state.

Probe source/output remain in `/private/tmp/tgpulse-camera-rumble-review/`.
The first debug-library attempt stopped on a MultiPCM arithmetic-overflow
assertion after capturing the pose transitions; the release-library probe
completed 1800 frames. No personal saves/configuration were modified.

### MAME And Standalone Comparison Follow-Up

MAME's current `netmerc_state::machine_reset` seeds the same six DPRAM words.
Its I/O board defaults ROM_EMU (JP4) to Off and MODE (JP3) to Off. Its NetMerc
configuration does not connect the Polhemus CPU to the I/O board serial peer
used by TGPulse. TGPulse's tracking path explicitly selects ROM_EMU On and
MODE Off before delivering real protocol measurements. Thus copying MAME's
initial seed does not reproduce MAME's complete boot path. These are source
differences, not a trace proving which instruction clears the pose.

The same 1800-frame neutral-pose probe was linked against the existing
standalone release machine library (`libtgpulse_core-3ecfde8f2c457155.rlib`).
It reproduced the same DPRAM transitions at frames 0, 255 and 928, with the
same 673-frame zero-pose interval. Output: `upstream-probe.txt` in the evidence
directory above. `run_frame` invokes the same frame slice and vblank as the
standalone dispatch. This establishes the machine-library behavior with equal
initial inputs; it is not a reproduction of the user's actual desktop session.
Persisted NVRAM, the point at which gameplay becomes visible, and upstream
automatic Holder sequencing can differ. No one of those differences is yet
established as the reason the user did not see the transient in standalone.

Primary MAME sources:
[Model 1 machine](https://github.com/mamedev/mame/blob/master/src/mame/sega/model1.cpp)
and [I/O board](https://github.com/mamedev/mame/blob/master/src/mame/sega/model1io2.cpp).

### Identified Firmware Instruction And Caller

The fine-step trace identifies the writer and its complete call path in the
validated `epr-18021.6` I/O firmware, rather than attributing this to SDL or
frontend bindings:

1. At reset, Z80 PC `0x0211` executes `LDIR` after initializing the first byte
   to zero, clearing local RAM `0xF000..0xF8FF`. The measurement buffer at
   `0xF480` starts at zero. This does not yet overwrite the shared pose.
2. With ROM_EMU On / MODE Off, the branch at `0x021B` selects `0x0226`;
   `0x023A` writes 1 to `0xF0B2`, enabling measurement publication. MAME's
   default ROM_EMU Off / MODE Off bypasses that branch and leaves the flag zero.
3. Routine `0x12F3` tests `0xF0B2`. When enabled, `LDIR` at `0x1303` copies
   twelve bytes from `0xF480` to `0xF4C0`; the loop beginning at `0x1306`
   publishes those bytes to shared DPRAM `0x80..0x8B`. At startup the source
   has not yet received a valid measurement.
4. `CALL 0x1504` at `0x1314` invokes the byte transfer helper. The actual
   committing instruction is **`LD (0x8009), A` at PC `0x150C`**, with A = 7.
   Command 7 of the 315-5338A writes the staged byte at the staged host address.
   Captured BC/DE values progress from `0x0086` through `0x008B`; the return
   address is `0x1317`. This proves the writer is the I/O firmware.
5. The serial peer receives continuous-stream command `C` at frame 926,
   after the firmware's preceding `c`, `S`, `u`, `f`, `m` setup commands.
   A complete neutral measurement is published by frame 928.

This is premature publication of an empty measurement buffer before the first
valid serial record. The correct constructor seed is overwritten by normal
firmware transfers; no separate frontend camera-reset instruction is involved
in the captured interval. The source-dependent publication branch explains why
MAME's fixed-pose setup is not the same execution path.

The reusable trace runner linked against existing release machine libraries
from both this port and the standalone produces **byte-identical instruction,
register, stack and tracking traces**. This demonstrates parity for the
controlled startup. Visibility in the user's session remains a separate
rendered/NVRAM comparison. No camera workaround was applied in this activity.

Reuse after a release build:

```sh
python3 tools/trace_netmerc_camera.py --rom /path/to/netmerc.zip --output /tmp/new-camera-trace
python3 tools/trace_netmerc_camera.py --deps /path/to/standalone/target/release/deps --rom /path/to/netmerc.zip --output /tmp/new-standalone-trace
```

The runner compiles only its own probe using existing `.rlib` files and Rust;
it neither installs packages nor opens host devices or personal saves. The
observation window covers the reproduced frames 254–256 and constant neutral
input with the ROM loader's initial image. A different boot/save state may need
a different window. Snapshot parsing verifies the current serialized CPU and
DPRAM fields before reading stack RAM; it is not raw object-layout access.
Caches with multiple Serde builds require selecting a compatible existing
bincode archive; the runner does so without rebuilding upstream.
Evidence: `reusable-core-trace/` and `reusable-standalone-trace-final/` under
`/private/tmp/tgpulse-camera-rumble-review/`.

### Why Publication Precedes A Valid Measurement

At firmware reset, `LDIR` clears local RAM including the decoded measurement
buffer. Publication routine `0x12F3` checks the enabled-mode flag at `0xF0B2`;
it does not test whether the serial receiver has completed its first record.
The normal dispatch reads a main-CPU request at shared offset `0x20`; request
1 enters `0x121A`, which calls the measurement publication path. The two
activities are therefore not gated by a shared first-sample-ready condition.

The emulated serial peer starts with continuous mode disabled and binary/metric
configuration unset. It emits measurements only after the firmware selects
`u`, `f` and `C`; `set_hmd_pose` alone updates its next requested pose and does
not write decoded I/O RAM. In the controlled trace, publication overwrites the
constructor seed at frame 255; `C` arrives at frame 926 and the decoded first
neutral measurement restores the shared pose at frame 928. This explains the
zero-filled buffer at the time of publication. It does not establish why the
physical original hardware avoided this interval, or whether additional
initialization/status behavior should be emulated. The existing `S` identity/BIT
request is recorded but has no fabricated response. No change to this protocol
policy has been approved or applied by this explanation.

## Approved Startup Publication Correction — 2026-10-04

The user approved keeping the Fixed Camera neutral pose until the I/O firmware
has received and decoded its first complete valid measurement. This applies
to all MVD modes at the native publication boundary, independently of frontend
sensor availability. It adds no Core Option or board DIP setting.

The reference is the verified EPR-18021 receiver/publication path described
above, compared with standalone and MAME. The smallest local adaptation guards
only NetMerc host writes to shared offsets `0x80..0x8B`. Before readiness, those
writes supply `HmdPose::default()`; later writes retain native firmware data.
The serial peer, commands, cadence and firmware ROM remain unchanged.

Readiness requires the binary decoder's `0xF08C` receive boundary to report
20 bytes, followed by its decoded-record counter increment at `0xF096`. At
that commit, the received station-1 packet at `0xF400` must match re-encoding
all six decoded words at `0xF480`, including the header, high-bit maps and
CR/LF. A queued packet or the initial sync/empty decode cannot release the
fallback. Neither nonzero coordinates nor elapsed time are validity tests.
Legitimate zero poses are accepted. Readiness latches across counter wrap;
CPU/peripheral Reset clears readiness and pending completion.

Both readiness and the pending receive boundary are machine state. The Model 1
envelope therefore moves from **4 to 5**; previous envelopes are rejected
atomically without migration. Save RAM and the Libretro outer wrapper remain
unchanged. This format change also applies to the other Model 1 titles.

### Focused Verification And Delivery

- 26 advanced-board tests and six machine-state tests pass. The new boundary
  test covers empty/partial records, complete-but-not-decoded records, valid
  zero poses, counter wrap, pending/ready state restore, Reset and isolation
  from boards without the NetMerc tracking peer.
- The real-ROM probe verifies 1,000 emulated frames with the original loader
  image. The former zero-publication interval now stays neutral. A distinct
  signed live pose and then an all-zero pose reach DPRAM normally. Complete
  state replay before/after handover and atomic format-4 rejection pass.
- The existing NetMerc controls ABI runner passes its eight check groups,
  including live modes, virtual commands, native state and Reset lifecycle.
- The macOS release artifact passes ABI/dependency/lifecycle inspection and
  is installed as `tgpulse_next_dev_m1_libretro.dylib`, with matching `.info`
  and verified SHA-256:
  `4662ea7b590aebe9c95137a402c8e8ba500b5df253f204988add15812952e0a4`.

Reuse the existing diagnostic runner:

```sh
python3 tools/trace_netmerc_camera.py --rom /path/to/netmerc.zip --output /tmp/new-bootstrap-check --verify-bootstrap
```

Evidence: `/private/tmp/tgpulse-camera-rumble-review/bootstrap-v5/` and
`bootstrap-controls-v5.json`. These checks prove native publication and ABI
behavior; they do not claim visual or physical-controller acceptance in the
user's session. Backport candidate B7 records the native correction separately.
Required reasoning: current level adequate; effort Medium. Shared account
usage after delivery: 38%, resetting 2026-10-10 at 09:49 CEST.

## Reported Horizontal Sensor Drift — 2026-10-04

The user reports leftward drift and subsequently confirms that drift is very
limited with the controller resting on a surface. No quantified observation
duration or drift rate is available. The user subsequently confirms continued
drift after moving and resting the pad again. This is consistent with a
remaining angular-rate error, rather than only a stable accumulated offset;
its device/driver/estimator origin is not established. Fixed
Camera always requests the same neutral pose
as MAME's NetMerc reset seed; sensor integration is disabled in that mode.

Comparison before this follow-up with the local standalone helper confirmed identical bias
subtraction and orientation/gravity integration, apart from the previously
approved configurable calibration timing. The gyro bias is estimated during
calibration and then held constant. Recenter resets relative orientation but
does not re-estimate bias. Gravity feedback limits tilt drift and provides no
absolute horizontal heading reference; the existing native test demonstrates
that remaining yaw bias still accumulates.

The resting-after-movement observation confirms continuing drift. Captured
raw gyro readings and calibrated bias would be needed to quantify its rate and
separate device drift from frontend/estimator effects. No thresholds or runtime
behavior were changed from the report alone.

A possible adaptation is conservative runtime bias estimation during verified
stationary intervals, preserving orientation rather than automatically
recentering. [Fusion's bias estimator](https://github.com/xioTechnologies/Fusion#bias-algorithm)
documents this established approach. Stationarity detection must avoid
absorbing intentional slow turns as far as possible: very slow constant yaw
and a small gyro bias cannot be distinguished perfectly with gyro/accelerometer
data alone. The Libretro callback lacks
original device sample timestamps. The user subsequently approved implementation
and a moderate enabled default; the delivery below supersedes the proposal.
Candidate B8 records the possible neutral-helper backport.

## Adjustable Drift Compensation — 2026-10-04

The user approved the single always-visible Input selector and subsequently
requested a moderate default to reduce drift across controllers, accepting
that no setting is perfect for all devices. The delivered default is **50%**,
with Off and 10–100% in steps of 10. Missing/invalid values resolve to 50%;
explicit Off disables learning. Modern and legacy registrations agree.
The option applies live to NetMerc sensor input and remains visible for other
titles or without content. It is a global frontend preference, not NVRAM.

### Reference Implementations And Parameter Meaning

SM2 `PORTING_PLAN.md` phases 3.4/3.5 and SM2/Supermodel Input percentage
selectors supply label, value, category and default conventions. Neither
reference core provides this gyro-bias estimator. The existing neutral
standalone orientation/calibration helper remains the integration point.

| Reference | Existing Algorithm / Parameters | Local Use |
| --- | --- | --- |
| [x-io Fusion Bias](https://github.com/xioTechnologies/Fusion/blob/main/Fusion/FusionBias.c) | Stationary gyro-bias refinement; 3-second stationary period; 3°/s threshold per axis; 0.02 Hz filter cutoff. | Base filter and 3-second dwell. Use actual callback elapsed time with an exponential coefficient; 100% corresponds to the reference cutoff. |
| [GamepadMotionHelpers](https://github.com/JibbSmart/GamepadMotionHelpers/blob/main/GamepadMotion.hpp) | Controller-specific optional auto-calibration; defaults include 0.5-second collection, 2-second minimum correction time and 3-second ease-in. Supports stillness/fusion modes and confidence. | Confirms gradual, optional correction and preservation of manual calibration; no wholesale estimator replacement. |
| Local adaptation | Gyro residual vector must remain below 0.5°/s; available accelerometer must be fresh, near 1 g and within 1° of its stationary-window direction. | Conservative gates to reduce slow-turn absorption. These limits and the 50% default are local initial tuning, not published universal gamepad settings. |

The percentage scales **learning speed**, not look sensitivity, stationary
thresholds or the fraction of bias subtracted. At 100% the time constant is
approximately 8 seconds; at 50%, 16 seconds; at 10%, 80 seconds, after the
3-second stationary dwell. These are derived from the reference filter, not
measured convergence times for particular controllers.

Gyroscope-only frontends retain gyro-based stillness detection. If an
accelerometer has supplied data, invalid/stale samples suspend learning rather
than masquerading as a gyro-only controller. Detected motion or acceleration
clears the dwell. Learning updates an additional host bias before orientation
integration; it never directly changes or recenters the current orientation.
Changing strength does not restart initial calibration. Off removes the learned
extra bias; manual calibration, polling gaps and lifecycle reset clear it.
The configured strength survives those resets. Recenter preserves calibration.

The standalone helper defaults to zero compensation, retaining its existing
policy. Only the Libretro adapter selects the approved 50% default. No new
dependency or host device access is introduced. Host bias is outside machine
Save State/NVRAM; native envelope format 5 remains unchanged.

### Verification

- 77 adapter tests pass, including stationary residual reduction at 0/10/50/100%,
  preserved current view, reset/live strength changes, 60/120 Hz equivalence,
  gyro-only support and acceleration rejection.
- A synthetic 0.002 rad/s residual over 60 seconds ends at approximately
  0.00200000 rad/s with Off, 0.00098336 at 10%, 0.00005748 at 50% and
  0.00000165 at 100%. The 50% result is about a 97% rate reduction; accumulated
  orientation is not automatically undone. These are synthetic results.
- A 1°/s intended yaw lasting 30 seconds retains its complete 30° rotation.
  This does not guarantee preservation of turns below the 0.5°/s gate.
- The existing sensor ABI runner checks live strength changes without extra
  calibration/negotiation and retains its calibration, modern OSD, lifecycle,
  unavailable-sensor and gyro-only checks.

Very slow constant yaw remains indistinguishable from bias using these sensors
alone. Physical results across gamepads are not claimed; adjust the percentage
or select Off if learning interferes with deliberate movement. Candidate B8
remains a separately reviewed upstream adaptation.

### Build And Installation

The offline release build and artifact ABI/dependency/lifecycle checks pass.
The standalone `cargo check --offline --locked -p tgpulse` also passes. The
existing mock-sensor ABI runner passes all six check groups. Local RetroArch
Development core and matching info are installed with SHA-256 verification:
`77f831ea8fd1cf9de842e53ec5eca640388e387ee5d7371c6b78591dbc00fbe6`.

Evidence: `/private/tmp/tgpulse-drift-tests-default50.txt`,
`/private/tmp/tgpulse-drift-default50-sensors.json`,
`/private/tmp/tgpulse-drift-artifact.txt` and
`/private/tmp/tgpulse-drift-standalone-check.txt`. The user can start the physical
trial at 50%, complete initial calibration and allow another 3 seconds plus
15–30 seconds of stillness to observe gradual refinement. Required reasoning:
current level adequate; effort Medium. Shared account usage after delivery:
38%, reset 2026-10-10 at 09:49 CEST. No publication was requested for this change.


## Optional Sensor Recording — 2026-10-04

### Reference And Scope

The current SM2 Input/live-option and Supermodel roadmap conventions were
reviewed; neither reference has a motion-sensor CSV recorder. Extend the existing
Libretro-owned P1 sensor acquisition and its isolated ABI runner. The neutral
orientation helper exposes a read-only diagnostic snapshot; file creation,
frontend paths, notifications and lifetime remain in the adapter. No new sensor
API, device handle or dependency is introduced.

`Sega NetMerc MVD Sensor Diagnostics` is always visible in Input, defaults to
Disabled, and takes effect immediately. It records only while Sega NetMerc P1
motion sensors are active. Choose Auto or Sensors, then enable diagnostics.
A queued notification shows the unique CSV pathname. Files are created under:

`<RetroArch Save Directory>/tgpulse-next/diagnostics/netmerc_mvd_<timestamp>_<pid>.csv`

An absent/empty/unwritable frontend save directory produces one diagnostic
failure notification per activation; tracking continues. There is no fallback
into the ROM directory or project. Disabled diagnostics create no files.

### Data Contract

Format 1 uses comment metadata followed by a header and 49 columns:

| Group | Contents / Units |
| --- | --- |
| Timing / Events | Host monotonic session timestamp and actual inter-poll delta in ns; Sample, Calibrate, Recenter or Sampling Gap event. These are callback times, not hardware timestamps. |
| State / Policy | Calibrating, Ready or Rejected phase, active gyro/accelerometer, gravity policy, drift strength, calibration sample count. |
| Raw Input | Three Libretro gyro axes in rad/s and available accelerometer axes in g. Missing vectors remain empty. |
| Converted Input | The exact SDL-coordinate gyro and acceleration supplied to the tracker, in rad/s and m/s². |
| Calibration | Running mean, accepted initial bias and standard deviation, all in rad/s; values are empty until available. |
| Drift Refinement | Learned additional bias in rad/s and detected stationary dwell in seconds. Learned bias is sampled after this row's update. |
| Applied Rates | Angular rate after bias subtraction and after optional gravity feedback, in rad/s. These are the rates actually used, before and after gravity correction. |
| Orientation / Native Pose | Relative Pitch, Yaw, Roll in radians; native position and orientation words 0/1/2, preserving their native order and neutral offsets. The native orientation words correspond to yaw, roll and pitch. While calibration is pending/rejected, the pose is neutral and relative angles/rates are empty. |

The frontend interface does not expose controller identity, connection type,
hardware timestamps or confirmation that a callback contains a new hardware
sample. Do not infer these from CSV timestamps. Identify the pad and USB/Bluetooth
connection when submitting a capture. Data collection alone does not establish
physical-controller calibration accuracy.

### Lifetime And Limits

Writes are buffered and flushed approximately once per second. Disable the
option to flush and close explicitly. Sensor shutdown, controller/mode changes,
Reset, state load and content unload close the current file. If the option
remains Enabled and sensors become active again, a new segment opens. Manual
Calibrate/Recenter are marked within the current segment. Host recording and
estimator state remain outside machine Save State and NVRAM.

A segment stops after 30 minutes or 216,000 rows. It then remains stopped until
a new activation (toggle Disabled/Enabled or restart sensors). Write/flush
failures report the partial file path; tracking continues. Unique file creation
never overwrites previous captures. An abrupt application/process termination
can lose the buffered tail.

### Suggested DualSense Capture

1. Use Sensors and the initial 50% drift-compensation default. Enable diagnostics
   before initial calibration, or press MVD Calibrate once recording starts.
2. Keep the pad resting for approximately 30 seconds after calibration.
3. Play for 2–3 minutes, including normal turns and a few deliberately slow turns.
4. Put the pad down for another 30 seconds; then disable diagnostics.
5. Supply the CSV, pad model, connection type, chosen strength and any observed
   drift or loss of intended movement. Repeat with Off only if comparison is useful.

The capture can support measured bias/noise, callback cadence, stillness gates
and compensation-convergence tuning. A stationary capture cannot by itself
prove that slow intended motion is preserved; retain both movement and resting
portions when reviewing default tuning.

### Verification And Delivery

79 adapter tests pass, including recording-disabled behavior, missing-directory
retry suppression, CSV shape and bounded recording shutdown. The existing
sensor ABI runner now has eight passing groups; it checks raw/converted axes and
units, orientation order, monotonic timestamps, correction fields, explicit
flush and queued path notifications, with no additional calibration. The
standalone build check and release artifact ABI/dependency/scope gate pass.
These use mocked frontend sensors, not a physical DualSense.

Evidence: `/private/tmp/tgpulse-sensor-trace-tests.txt`,
`/private/tmp/tgpulse-sensor-trace-final-abi.json`,
`/private/tmp/tgpulse-sensor-trace-artifact.txt`,
`/private/tmp/tgpulse-sensor-trace-standalone.txt`.
The verified release and matching info are installed as RetroArch Development.
Core SHA-256: `e4668e18e0f9cb26a41dc273670103c50f0b38f0b804d938b4346c94d42327d3`.
Info SHA-256: `91ff93c2f5f1bd468bf88d66b73e3fe87066bbb596bbe37b41bc135f0021fdee`.
Required reasoning: current level adequate; effort Medium. Shared account usage:
38%, reset 2026-10-10 at 09:49 CEST. No commit or publication requested.


## First Physical Sensor Capture — 2026-10-04

The user supplied a Sony DualSense Bluetooth recording at 50%. The
[source and exact-tracker replay analysis](MVD_DUALSENSE_BLUETOOTH_2026-10-04.md)
records calibration, timing and stationarity. The 50% replay matches all captured
orientation axes exactly. In the final low-motion segment, recorded yaw change
is 0.328° versus 0.828° with Off in offline replay (60.4% reduction); 100% replay
would yield 0.173°. Learning remains suspended during the moving 36–172 s portion.
Keep the default and gates unchanged. This is one physical sensor session,
without an external orientation reference or cross-controller validation.
