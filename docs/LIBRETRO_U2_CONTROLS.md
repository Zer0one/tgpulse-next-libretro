# NetMerc Controls And Stick MVD — 2026-10-04

## References And Adaptation

Inspected the current standalone `input/signals/mod.rs`, `input/cabinet.rs`,
`input/cabinet/analog.rs`, `input/mvd.rs` and MVD GUI selectors. The source
frontend delta is upstream `e60e53f7da49795ba349b1a6d1c0455657081f2f`.
SM2-Emu `PORTING_PLAN.md` sections 3.1–3.2 and `src/libretro/input.cpp`, plus
Supermodel `Docs/CONTROL_PROFILES.md` and `Src/OSD/libretro/libretro.cpp`, establish
pertinent per-profile controls, identical names for aliases, and full **Button**
labels rather than **Btn**. Neither reference implements NetMerc MVD.

The user approved the U2 adaptation and finalized the profile name as
**Special: Sega NetMerc**. Both ports advertise that name; P2 exposes only the
explicitly approved duplicate Test and Service commands. Action labels have no
redundant game prefix. The standalone's SDL device policy remains separate;
the Libretro adapter owns only callback sampling, pose conversion and command
edges. Existing flight profiles retain their previous sampling policy.

## Published P1 Inputs

| Action | RetroPad Binding | Native Destination |
| --- | --- | --- |
| Coin | Select | Coin 1 |
| Test | L3 | Shared Test, also on P2 |
| Service | R3 | Shared Service, also on P2 |
| Stick X / Stick Y | Left Stick X / Y | ADC Channels 0 / 2 |
| Trigger Button | B (South) / L2 / R2 | IN1 Bit 0 |
| Thumb Button | A (East) / L1 / R1 | IN1 Bit 1 |
| MVD Holder | Start / D-pad Down | IN1 Bit 2 |
| MVD Calibrate | X (North) | Virtual Sensor Command |
| MVD Recenter | Y (West) | Virtual Sensor Command |
| MVD Look X / MVD Look Y | Right Stick X / Y | Serial Tracking Pose |

All aliases of an action have the same displayed name and reaches the same native switch.
Analog L2/R2 use the standalone activation threshold above 50%, with digital
trigger fallback. Both triggers activate Trigger Button; L1/R1 activate Thumb Button.
This user-approved mapping supersedes the standalone default aliases. Start is only a
physical binding for Holder; no native Start switch or Coin 2 is invented.
North/West do not press Trigger, Thumb or Holder.

## Axis Polarity And Ranges

Libretro stick X is positive right and Y positive down. The standalone's native
flight signal Y is positive up, so its NetMerc inversion is applied at the
cabinet boundary. The resulting ADC endpoints match the standalone:

| Input | Negative Travel | Center | Positive Travel |
| --- | --- | --- | --- |
| Left X | Left: 00 | 7F | Right: FF |
| Left Y | Up: 00 | 7F | Down: FF |
| Right X | Look Left: Lower XANG | Forward | Look Right: Higher XANG |
| Right Y | Look Up: Lower ZANG | Forward | Look Down: Higher ZANG |

The MVD adapter preserves upstream's neutral pose `[12868, 25736, 12868]` and
scale of 25736 raw units per 180 degrees. It applies absolute deflection, not
velocity: releasing the stick returns forward. Position and roll are preserved.
There is no extra adapter dead zone or axis-assignment option for NetMerc;
RetroArch owns those choices.

Input options always visible, applied live only to NetMerc:

| Option | Values | Default |
| --- | --- | --- |
| Sega NetMerc MVD Input | Auto / Off (Fixed Camera) / Right Stick | Auto |
| Sega NetMerc MVD Horizontal Range | Off / 10–90 Degrees, Step 10 | 30 Degrees |
| Sega NetMerc MVD Vertical Range | Off / 10–90 Degrees, Step 10 | 20 Degrees |

Ranges are per side of forward. Off locks that axis. Auto uses P1's available
RetroPad; unavailable input and Off use the fixed forward pose. Explicit Right
Stick also has a neutral fallback without an enabled P1 pad. Modern and legacy
Core Options share defaults and stable Input-menu order; options remain visible outside NetMerc and after unload. The general visibility
policy supersedes the initial title-based filtering.

## Commands And Lifecycle

Calibrate and Recenter emit one virtual event per press. Holding a button does
not repeat it. A successful state restore, reset or P1 device change
resynchronizes edges: an already-held button must be released before reuse.
Failed state restoration retains the preceding frontend edge state.

U2 prepared the virtual bindings; [U5](LIBRETRO_U5_SENSORS.md) now connects
those commands to frontend sensors, calibration and orientation recentering.
The mode selector now includes Sensors. Calibrate requests three stationary
seconds and reports actual acceptance/rejection; Recenter reports readiness.
Neither command modifies absolute-stick deflection. Automatic Holder
sequencing remains an eventual enhancement, separate from manual aliases.

The native tracking pose is already part of U1 machine-state format 4. Mode and
range are destination frontend preferences, outside snapshots. No new state
format or migration is introduced. Unload drops the command state.

## Verification

Raw evidence lives under `/private/tmp/tgpulse-u2`, outside Git. Reuse:

- `cargo test --offline --locked -p tgpulse-libretro --lib`: 57 passed.
- `tools/check_libretro_artifact.py`: native macOS ARM64, Model 1-only marker,
  25 ABI exports, dependencies and three empty lifecycle cycles.
- `tools/test_libretro_model1_scope.py`, with the preserved U1 core and 120
  frames per starting set: nine sets retain identical video, audio, Save RAM,
  complete machine snapshots and snapshot continuation.
- `tools/test_libretro_netmerc_controls.py` reuses that ABI Host for profile,
  aliases, P2 scope, live modes/ranges, fixed fallback, pose-state replay,
  command edge suppression after hold/reset/restore and unload visibility.
  The native switch threshold and exact ADC endpoints also have unit coverage.
- `tools/test_retroarch_gpu.py` accepts the established replay-v1 fixture and
  explicit MVD mode/ranges for bounded real-frontend verification.

Physical controller and gameplay acceptance remain user-run. These checks do
not establish sensor operation, automatic Holder sequencing or other pending
NetMerc enhancements.

The final profile-name correction was verified again with the unit, artifact
and NetMerc ABI checks. Nine-set regression used the preceding U2 binary
(`c763fef12e942af12dd31986112ebbe07cf130642354124f16fbb5700e8cddde`),
whose only subsequent production change was the NetMerc profile label.

The final RetroArch Vulkan replay completed its 2,400-frame bound with exit 0
in 47.024 seconds, valid PNG output and both virtual-command notifications.
Screenshot inspection confirms the NetMerc title screen; this is frontend
startup/input evidence, not a 3D scene or physical-controller acceptance claim.
The runner's sandboxed GUI launch failed before core initialization; the same
isolated runner succeeded with authorized GUI execution outside the sandbox.
No personal RetroArch settings or saves were changed.

The final core was installed through `tools/install_dev_core.py` as
`tgpulse_next_dev_m1_libretro.dylib`, with matching Development metadata.
Installed SHA-256 equals the verified build:
`ae0d6ed9a7c4dda2f8a926e00dfcb466c9eecc2c2d9a34dc13095881f9baec5c`.
Matching info SHA-256:
`91ff93c2f5f1bd468bf88d66b73e3fe87066bbb596bbe37b41bc135f0021fdee`.
Commit, push and publication were not requested for this phase.

Account usage after verification: 31% of the weekly window, resetting
2026-10-10 09:49:17 CEST. This shared rounded account figure does not isolate
U2 consumption from other chats; the starting and ending displayed values
were both 31%.

## Approved Binding Correction — 2026-10-04

The user's final binding selection is Trigger Button = South/L2/R2 and
Thumb Button = East/L1/R1. All aliases keep the same action label and native
switch; this overrides the initial U2 bindings documented by its historical
verification evidence. The profile remains Special: Sega NetMerc.

The focused correction passed all 57 adapter tests, the Model 1 macOS artifact
gate and the existing NetMerc ABI runner. That runner now compares complete
native snapshots and frame/audio output for every alias, analog-trigger
thresholds and P2 isolation. Its empty controller-info handling was corrected
for initialization/unload; the final run has no callback exceptions.

Evidence: `/private/tmp/tgpulse-u2-bindings`. The final release build and
installed Development core have matching SHA-256:
`dba274dedcc53f5b1e36a4ef559ca22a87511511028c50ea60ef8f0cc67553a9`.

The matching Development info is unchanged. No additional real-frontend or
physical-controller trial was needed for this binding-only correction.
Account usage remains 31% (shared rounded figure); next reset is
2026-10-10 09:49:17 CEST. Effort: High / S.

## Standalone Notification Audit — 2026-10-04

Inspected standalone `Gui::update_mvd_holder`, `App::redraw`,
`App::calibrate_mvd`, `App::recenter_mvd`, `InputState::take_mvd_notice` and the
motion calibration outcome callback. The existing SM2/Supermodel Libretro
message path uses `RETRO_ENVIRONMENT_SET_MESSAGE`; the adapter reuses its
existing callback, with 180-frame informational Holder messages (about three
seconds), rather than a separate renderer or diagnostic panel.

| Standalone Feedback | Libretro Adaptation |
| --- | --- |
| Holder Latched / Cleared | Added. Observe the native game-acknowledged latch through `mvd_holder_latched()`, not the binding or an invented toggle. Publish only on state changes, including successful reset/restore; unchanged states do not renew the message. |
| Keep P1 Controller Still For 5 Seconds | U5 requests three seconds in Libretro while the standalone default remains five. |
| Sensor Calibration Successful / Rejected, With Reason | Implemented in U5 from actual estimator outcomes, with rejection reasons. |
| Calibration Unavailable: No Gyro / Device Error | U5 negotiates frontend sensor capabilities and reports unavailable/invalid input. |
| MVD Recentered / Sensors Not Ready | Implemented in U5 according to actual orientation readiness. |
| Saved / Loaded Slot, Slot Selection And Save/Load Failure | Owned by RetroArch. Core serialization failures already use its notification callback. |

Holder notices use Title Case: `MVD Holder: Latched` and
`MVD Holder: Cleared`. The previous observation lives only in the frontend
Game object, outside machine snapshots; comparing it with restored native RAM
reports real state changes without altering the latch. Other systems return
no Holder state and receive no such messages. The frontend owns message
rendering, queueing and expiry; old messages expire normally on content unload.

The user's existing RetroArch configuration enables OSD, widgets and Save
State notifications. It disables notifications while the menu is active;
this audit leaves personal configuration unchanged.

Notification verification: 58 adapter tests pass, including state-change,
unchanged-latch and lifecycle observation tests; the existing ABI runner
confirms one initial native Cleared notification of 180 frames and unchanged
control/pose/state behavior. The Model 1 artifact gate passes. A bounded
720-frame Vulkan RetroArch run with OSD enabled exits successfully and logs
receipt of the Holder message via `SET_MESSAGE`. Its automatic screenshot
contains the native frame without the OSD, so it is not proof of the toast's
visual appearance or a real-game Latched transition.

Evidence is isolated under `/private/tmp/tgpulse-mvd-notices`. The verified
Development installation matches build SHA-256:
`4c061b826f9a361d03d27aa038439abb93daafd40dc216b6565c23299d510e6c`.
Matching info SHA-256 remains
`91ff93c2f5f1bd468bf88d66b73e3fe87066bbb596bbe37b41bc135f0021fdee`.
Account usage after verification: 31%, next reset 2026-10-10 09:49:17 CEST.
