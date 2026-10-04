# Sony DualSense Bluetooth MVD Capture — 2026-10-04

## Capture And Method

Controller and transport are user-reported: Sony DualSense, Bluetooth.
Recorded option settings: Drift Compensation 50%, Gravity Stabilization Enabled,
P1 gyroscope and accelerometer available. Input file:
`netmerc_mvd_1791139793452067000_4833.csv`.
SHA-256: `7ab88558df5e19539443e7da8c309e5277301a615ff6b018d975519819c5343d`.
The original remains in the user's RetroArch save directory; it is not copied
into the repository. Times below are seconds from the first CSV row.

The existing read-only native Rust trace workflow is adapted to sensor CSV:
`tools/replay_netmerc_sensor_trace.rs` imports the actual neutral motion helper,
replays recorded converted samples and Calibrate/Recenter events, and compares
Off / 50% / 100%. It contains no duplicate orientation estimator. The 50% replay
matches all three captured orientation axes exactly: maximum error **0 radians**.
Off and 100% results are offline alternatives for the same measured input,
not separate physical trials.

## Timing And Calibration

- 12,374 rows over 206.231 seconds, approximately 60 callback readings/s.
- Inter-poll median 16.587 ms, p95 17.383 ms, maximum 27.793 ms; no 250 ms
  scheduling-gap reset occurs.
- The first calibration is rejected at 3.012 s. Gyro X sample deviation is
  9.870°/s, exceeding the 1.146°/s limit. The recorded variation is too large
  for stationary calibration; this capture does not justify relaxing that limit.
- Manual Calibrate occurs at 9.895 s; acceptance occurs at 12.895 s with
  119 observations. Accepted bias X/Y/Z is −0.0564/−0.5859/+0.4325°/s;
  sample deviation is 0.2080/0.0879/0.1772°/s.
- Five Recenter events occur at 39.529, 57.813, 138.647, 166.931 and 174.347 s.
- 444 adjacent rows repeat the raw gyro triple. This alone does not prove stale
  input: stationary/quantized samples can repeat, and the frontend does not
  expose hardware sample identifiers. Callback cadence is not hardware cadence.

## Drift And Stationarity

Low-motion intervals appear around 20.5–35.7 s and 176–204.2 s. They are
consistent with resting portions of the suggested capture, but the CSV does
not independently establish that the controller was physically untouched.
Use that qualification when interpreting drift as error.

Detected stillness lasts up to 28.517 s in the final low-motion segment.
Learning is active for approximately 34.668 s across the full recording.
During the predominantly moving 36–172 s interval, no sample reaches the
three-second learning dwell: learned bias remains constant while playing.
This is evidence that the current gates reject these recorded movements; it
cannot rule out absorption of slower intended turns in other captures.

### Final Low-Motion Segment, 176–204.2 s

| Strength | Absolute Yaw Change | Reduction Against Off | Evidence |
| --- | --- | --- | --- |
| Off | 0.828° | — | Offline replay |
| 50% | 0.328° | 60.4% | Recorded; identical replay |
| 100% | 0.173° | 79.1% | Offline replay |

Initial drift is accumulated before the three-second gate and gradual learning
converge; bias learning does not undo that accumulated orientation or recenter.

Within 190–200 s, fitted yaw slope is −0.029005°/s with Off,
−0.007466°/s at 50% and −0.002103°/s at 100%. The 50% slope is approximately
74% smaller than Off in that interval. These are local fitted rates, not
predictions for an entire session. Gravity feedback and controller attitude
also influence the observable yaw; body gyro Y alone is not the yaw-angle slope.

## Tuning Decision And Limits

Retain the moderate 50% default, the 0.5°/s residual gyro gate, 1° gravity
consistency gate and three-second stationary dwell. The existing detector
already admits long low-motion intervals and suspends learning during play;
this sample does not demonstrate a need to widen its gates. A higher user
strength can improve convergence for this recording, but one controller/session
is insufficient to make 100% the general default or demonstrate behavior on
very slow intended turns across devices.

The trace validates real callback data from this DualSense/Bluetooth session,
not visual agreement with an external orientation reference. USB behavior,
other controller models, temperature changes and extremely slow motion remain
unmeasured. No runtime parameter, core build or installation was changed for
this analysis.

### Rumble Coupling Hypothesis

The user reports less visible drift with Gamepad Rumble disabled. The current
core polls P1 motion before running the machine frame and dispatches NetMerc
motor output to both frontend rumble channels after that frame. Rumble settings
do not directly change the orientation or bias calculations. Physical haptic
vibration could affect later gyroscope/accelerometer readings, calibration
variance and the three-second stillness detector. MEMS gyroscopes can respond
to linear vibration; accelerometers measure vibration as well. The CSV does
not record frontend rumble output or its timing, so this session cannot
establish the cause or quantify a rumble-specific change. A core option being
Enabled also does not establish that the game's motor was active.

For a controlled user-run check, capture the same NetMerc scene and controller
placement with Rumble Disabled, Enabled during a clearly felt motor event, then
Disabled again. Keep MVD input, calibration procedure and drift strength fixed;
identify the active motor interval and avoid moving the pad during each short
comparison window. Compare raw gyro variance/mean, accelerometer variance,
stationary dwell and yaw slope. Run calibration with the motor quiet for the
drift comparison, then separately test whether motor activity during
calibration causes rejection. This is a follow-up observation procedure, not
an implementation milestone or a confirmed driver fault.

## Reproduction And Evidence

```sh
rustc --edition=2024 tools/replay_netmerc_sensor_trace.rs -o /private/tmp/replay_netmerc_sensor_trace
/private/tmp/replay_netmerc_sensor_trace /path/to/capture.csv > /private/tmp/dualsense-bt-mvd-replay.csv
```

The runner requires a constant-50% format-1 capture starting at tracker startup,
including calibration/recenter events. It checks the replay against all captured
angles; a truncated session, changed source behavior or different strength must
not silently be treated as a validated comparison.

Reasoning: current level adequate; effort Medium. Shared account usage after
analysis: 39%, reset 2026-10-10 at 09:49 CEST. No commit or publication requested.
