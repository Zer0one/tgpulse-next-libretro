# U6 NetMerc Rumble — 2026-10-04

## References And Approved Adaptation

Rechecked the standalone `input/model1_rumble.rs`, `input.rs`, its NetMerc
frame dispatch and GUI intensity control. NetMerc uses port D bit 2, converted
to equal strong/weak pad levels of 0.6 when active and zero when inactive.
The standalone applies a separate global intensity percentage after conversion.
VR/VFormula retain their existing drive-command decoder.

SM2 `PORTING_PLAN.md` phase 3.7 and `src/libretro/rumble.cpp` use the frontend
rumble interface, P1 motors, explicit shutdown and observed drive writes.
SM2 expressly delegates overall intensity to RetroArch Input Rumble Gain.
Supermodel's current Libretro input implementation likewise forwards native
force effects through that interface and stops motors at lifecycle boundaries;
its core selector is an enable/disable control. Neither reference implements
NetMerc's binary motor output.

The user approved extending the existing global Gamepad Rumble selector to
NetMerc, retaining Enabled as its default and always-visible presentation.
Overall intensity belongs to RetroArch Input Rumble Gain, following SM2; no
second core gain selector is added. This replaces the original U6 roadmap
intensity-selector requirement. The single implementation roadmap records
status; this document records design and evidence.

## Motor Output And Lifecycle

- Use the actual NetMerc port D bit 2, not button states, LCD bus traffic or
  fabricated firing/engine effects.
- Reuse the upstream pure binary-motor conversion: strong = weak = 0.6 when
  active, before the frontend applies its gain. This is 100% of the upstream
  effect level, not full hardware motor power.
- Match standalone sampling: use the final `netmerc_motor()` port-D latch
  once per emulated frame. An On/Off pulse completed within that frame does not
  create an extra frontend effect. Native pulse observations remain an optional
  helper candidate, outside this adapter's output path.
- Preserve the VR/VFormula decoder and its current default effect levels.
- Explicitly send zero to both motors on disable, P1 removal/device change,
  reset, successful state load, unload/deinitialization and emulation errors.
  Frontend pause/device ownership remains with RetroArch.
- Unsupported rumble interfaces/controllers remain silent without changing
  machine behavior. No new routine rumble OSD is proposed.

## Delivery Criteria

Verify native motor-bit isolation from LCD writes, same-frame pulses, binary
motor levels, enable/disable and shutdown through the existing unit/ABI hosts.
Check VR/VFormula output against the preceding build. Build and verify the
macOS artifact, install the Development core/info and compare SHA-256. Physical
pad trials remain user-run. Record any newly reusable neutral helper in the
upstream backport register after implementation.

Required reasoning: High, adequate at the current level. Effort: S–M.
Estimated account use: 1–2 percentage points. The implementation was approved on 2026-10-04. Publication is outside this phase.

## Implementation And Verification

`model1io2::Bus` retains one transient `motor_seen_on` flag outside BusState.
Port D writes and direction-driven output updates observe real On activity;
`model1board::take_netmerc_motor_activity` drains that flag and retains held
native On output. Every Libretro frame drains observations, even while rumble
is disabled or P1 absent, so reenable cannot replay historical pulses. Restore
and reset clear transient observations; native state format 4 and output latches
remain unchanged. No Libretro types enter the hardware modules.

The pure `binary_motor_levels` function matches the upstream 0.6/0.6 policy.
The adapter sends 39321 to both 16-bit P1 motors when active and zero when
inactive. Existing VR/VFormula effects retain their decoder and motor ratio.
The option help now identifies all three games and frontend intensity control.
No routine rumble notifications or new bindings are introduced.

Verification:

- Three native motor tests cover port-bit/LCD isolation, held output,
  same-slice On/Off pulses, single consumption, reset/restore and unchanged
  serialized native state.
- 71 adapter tests pass, including positive synthetic I/O firmware → pulse
  observation → both frontend motors → zero on the next callback.
- `tools/test_libretro_netmerc_rumble.py` reuses the existing isolated ABI host.
  Real NetMerc frames negotiate/use both P1 callbacks; state continuation,
  disable/reenable, full/reduced/None device transitions, reset and unload pass.
  Its bounded coin/Holder/Trigger session produced zero positive motor callbacks;
  it proves routing/lifecycle, not native gameplay motor activation. Positive
  pulse delivery is demonstrated by the synthetic-firmware test above.
- Existing NetMerc controls and virtual-command runner passes.
- Nine-set prior-binary comparison preserves video, audio, Save RAM and
  identical state continuation, including VR/VFormula.
- macOS release build and native artifact/ABI/dependency gates pass; isolated
  RetroArch Vulkan delivers 240 frames and exits successfully.

Physical pad behavior remains user-run. Local evidence is retained in
`/private/tmp/tgpulse-u6/`. No source commit, push or release is part of U6.

Development core and matching info are installed in local RetroArch. Build and
installed core SHA-256:
`88fc7f6a75039b5fbdbec8224d020534489d14b26213b09196acee7837b44983`.
Final artifact evidence is in `/private/tmp/tgpulse-u6/final/`; the preceding
candidate's controls run remains separately recorded in the parent directory.
Account-wide usage at completion: 36%; reset 2026-10-10 09:49:17 CEST.

## Perceived Rumble Difference Review — 2026-10-04

The current standalone still samples only `netmerc_motor()` at each frame end.
The adapter consumes `take_netmerc_motor_activity()`, preserving genuine On/Off
activity within a frame for one callback. Both use the same 0.6 strong/weak
motor levels, but their delivered pulse sequences can differ. This is the
existing U6 adaptation and backport candidate B5, not a sensor-axis issue.
The neutral startup probe above did not produce motor activity; it is not
physical rumble evidence.

The inspected local RetroArch configuration selects `sdl2` joypads with
`input_rumble_gain = 100`. Standalone SDL3 sends both motor levels together,
refreshing a 100 ms effect; the adapter sends separate Libretro strong/weak
updates and delegates host delivery to RetroArch. SDL2 and SDL3 APIs both
accept two 16-bit amplitudes and a duration; the major version alone does not
establish the cause.

The current primary RetroArch SDL2 driver source has an additional concrete
suspect: `sdl2_joypad_set_rumble` zero-initializes a two-motor effect on every
call, assigns only the requested motor, then submits both amplitudes with a
5000 ms duration. The second separate motor update can therefore replace the
first, instead of preserving its amplitude. The installed nightly's exact
source/build and selected device backend have not been verified against this
implementation; do not claim this as a measured hardware cause.

Primary references:
[SDL2 API](https://wiki.libsdl.org/SDL2/SDL_GameControllerRumble),
[SDL3 API](https://wiki.libsdl.org/SDL3/SDL_RumbleGamepad), and
[RetroArch SDL2 driver](https://github.com/libretro/RetroArch/blob/master/input/drivers_joypad/sdl2_joypad.c).
Controller model, USB/Bluetooth connection and whether the difference concerns
strength or pulse duration remain necessary to distinguish these cases.
No gains, pulse policy, frontend configuration or runtime code were changed.

### Reference Parity Clarification

Preserving within-frame pulses was introduced to avoid losing real motor
transitions at the frontend sampling boundary, following the reference SM2
drive-write observation approach. NetMerc's standalone dispatch nevertheless
uses only the final latch. This changes output pulse delivery and is not
required by the Libretro ABI or SDL2. Synthetic tests establish that the
observation works, but do not establish equivalent perceived feedback or a
need for this behavioral deviation. The user questioned the departure from
standalone; exact parity would retain final-latch sampling and adapt only
output delivery/gain/lifecycle. This clarification changes no runtime policy.

## Final-Latch Rumble Alignment — 2026-10-04

At the user's request, Libretro now reads `netmerc_motor()` exactly as the
standalone does. Only frontend motor delivery, global gain ownership and
lifecycle shutdown are adapted. The previous within-frame observation is no
longer connected to gamepad feedback. No input/camera policy or native state
format changed.

The focused adapter test passes, including synthetic On/Off in one slice
producing silent frontend output at the final Off latch. The actual NetMerc ABI
runner passes native levels and reset/state/device/disable/unload lifecycle
gates. Its scripted session had zero positive motor callbacks, so physical
feedback parity remains the user's controller test, not a claimed test result.
Release build and native artifact gates pass. Development core/info are installed
and hash-verified. Core SHA-256:
`cbd56516b94ce39d530fff701bf60e3d513ef8035f72f16969f3c11527cb8088`.
Evidence: `/private/tmp/tgpulse-camera-rumble-review/rumble-final-latch.json`.
No commit, push or publication. Account usage 37%; reset 2026-10-10 09:49 CEST.
