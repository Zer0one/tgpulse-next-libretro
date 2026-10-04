# Model 1 Libretro frontend map (preliminary)

This map records the first porting checkpoint and its initial Virtua Racing
implementation. It maps SM2-Emu Libretro frontend patterns to the imported
TGPulse-Next Model 1 source. The experimental core has profiles for all ten
declared Model 1 sets; frontend persistence is now wired to the machine APIs. No Model 2 game is in
this port.

This document records design decisions and evidence. The
[Libretro roadmap](LIBRETRO_ROADMAP.md) is the only active implementation plan.

Current visibility policy (2026-10-04): general Core Options are always visible
except source Gains, which show only for the fitted sound hardware of loaded content.
The dated implementation sections below retain earlier verification history;
the final General Core Option Visibility section supersedes title/board-based
display restrictions. Reviewed per-set NVRAM/Linked Cabinets are explicit exceptions.

## Source boundary

| Concern | SM2-Emu Libretro reference | TGPulse-Next Model 1 source | Port decision |
| --- | --- | --- | --- |
| Content and lifecycle | `src/libretro/core.cpp` owns `retro_load_game`, `retro_run`, reset and unload. | `tgpulse-core` identifies ZIP contents and `load_model1_zip` rejects Model 2; `Model1System` advances the machine. | Add a narrow Libretro adapter. Reuse the loader and machine; reject non-Model 1 content clearly. Keep the standalone frontend buildable. |
| Input profiles | `src/libretro/input.cpp` selects per-game profiles and publishes controller descriptions. | `roms_db.dat` records each set's `Scheme` and `AnalogRole`; `crates/tgpulse/src/input/` translates host signals to native ports and ADC values. | `crates/tgpulse-libretro/src/model1_controls.rs` translates RetroPad actions to the existing native I/O. The imported desktop input source is untouched. |
| Options/menu | `src/libretro/core_options.h` registers Core Options v2 in System, Video, Audio and Input, with game-specific visibility. | `Config` and the desktop Settings menu already express machine and output preferences. | Rebuild the applicable settings as Libretro Core Options, with TGPulse defaults and semantics. RetroArch owns the menu and remapping UI. |
| Video | SM2 delivers frames through Libretro video callbacks and owns renderer selection in the adapter. | Model 1 has a CPU 3D path plus tilemap composition at 496 x 384; the desktop also has a `wgpu` compute path. | First deliver the existing native software image as XRGB8888. Consider frontend-owned GPU and widescreen paths only after a working native frame path. |
| Audio and timing | SM2 duplicates occasional frames in 60 Hz Compatibility; Supermodel runs a new machine frame on each 60 Hz callback. | `Model1System` advances by `CYCLES_PER_FRAME`; its sound board produces stereo samples from a fixed clock. | Native mode drains each frame's audio unchanged. The selectable 60 Hz mode follows Supermodel's new-frame cadence, speeding Model 1 by about 4.3%; resample each frame's audio to the native frontend sample rate. |
| Persistent data | SM2 exposes Save RAM and serialized states through Libretro. | `Model1System` exposes NVRAM/EEPROM blocks and in-memory `save_state`/`load_state`; states reject a fitted COMM board. | Use frontend-owned save data and Libretro serialization. Preserve factory defaults, existing saves and the COMM restriction. Do not reuse standalone file paths. |
| Host extras | SM2 uses Libretro rumble and Netpacket interfaces. | TGPulse has a VR drive-command decoder and experimental Model 1 TCP networking in the desktop frontend. | VR/VFormula rumble now forwards the shared standalone pad policy through Libretro's P1 motors. Design linked cabinets separately around a frontend transport; TCP settings are not copied into Core Options as if Netpacket already worked. |

## Model 1 catalogue and control profiles

The imported database declares ten Model 1 sets, grouped below by cabinet
behavior. A database entry permits loading; it does not prove that a game or
every physical control works in RetroArch.

| Cabinet profile | Sets in current database | Native behavior to preserve | Frontend path |
| --- | --- | --- | --- |
| Virtua Racing / Virtua Formula | `vr`, `vformula` | Steering, accelerator, brake, gears and four views. | Shared Driving: Sequential + VR4 profile on both ports; P2 exposes native Coin 2 and shared operators. |
| Virtua Fighter | `vf` | Two independent directional panels with Kick/Punch/Guard. | Same fighting profile name on both ports with independent gameplay inputs. |
| Wing War | `wingwar`, `wingwarj`, `wingwaru` | Flight stick, throttle, three weapons and four views. | One named flight profile for revisions with the same database signature. |
| Wing War R360 | `wingwar360` | Flight stick with distinct polarity and no view switches. | Separate controller profile and descriptors. |
| Star Wars Arcade | `swa`, `swaj` | Pilot stick/throttle/view/weapon controls; Gunner stick and Laser/Torpedo only. | Different named P1 Pilot and P2 Gunner controller ports. |
| NetMerc (experimental) | `netmerc` | Own stick ADC order and MVD Holder, no Start switch. | Special: Sega NetMerc on both ports, P2 Test/Service only. U2 adds native Trigger/Thumb/Holder aliases, correct Y polarity and P1 right-stick MVD; see [controls and verification](LIBRETRO_U2_CONTROLS.md). U5 adds frontend-owned motion sensors, three-second calibration and adjustable drift compensation (50% default); see [sensor integration](LIBRETRO_U5_SENSORS.md). Calibration seed is optional; physical gameplay acceptance remains user-run. |

For shared service controls, retain TGPulse's established preference: **Test on
L3, Service on R3**. This differs from the current SM2 Libretro mapping.
RetroArch remapping may change physical assignments; the descriptors must name
the cabinet action actually sent to the machine. Unknown or inapplicable
signals must stay inactive, rather than inheriting a misleading Model 2 label.

## Initial Core Options proposal

Names and exact values are implementation candidates, subject to verification
against the first running core. The initial menu should be small and grow when
its effects are observable.

| Category | Initial option or behavior | Basis and default | Later or excluded from first slice |
| --- | --- | --- | --- |
| System | Known Bad Dump ROM Repairs | **Enabled** keeps the standalone loader's SHA-1-gated in-memory correction of legacy `315-5711.bin`; **Disabled** leaves that chip unchanged. Content reload is required. | **Twin/Linked Cabinets** requires a defined frontend transport and game-specific acceptance. Copy neither Model 2 preset bytes nor its machine-specific fixes. |
| Video | Smooth Shadows | Existing Model 1 setting; default **On** in `Config`. Native 496 x 384 output first. | Widescreen, 2D stretch, supersampling, GPU renderer and sRGB correction require separate output-path checks. |
| Audio | Master Volume and Model 1 source gains/mutes | Master Volume defaults to **100%**. MultiPCM1/2, YM3438 and optional DSB have separate live gains and output mutes; gain defaults match the standalone values **50/50/30/100**, with all outputs unmuted. DSB controls use frontend visibility hints and appear only with a fitted board when supported. | SCSP is not a Model 1 source. |
| Input | Gamepad Rumble | Default **On** by user request, matching SM2; the standalone `Config` default remains Off. The existing standalone Model 1 VR/VFormula command-to-pad policy is shared with the Libretro adapter; its strong/weak output targets P1 through the frontend rumble interface. The option uses VR/VFormula visibility hints and stops both motors at lifecycle boundaries. | No Model 2 gun, shifter or driving-response option is imported. Physical-pad verification remains user-run. |

Controls themselves primarily belong to RetroArch's controller remapping and
the core's per-game descriptors. Machine Test/Service menu settings and EEPROM
values remain game-owned unless a specific setting has a proven safe frontend
representation. The first slice does not need an external `games.xml`: the
current database is embedded in `tgpulse-core` as `roms_db.dat`.

| SM2 option family | Model 1 disposition |
| --- | --- |
| ROM verification, known repairs and initial NVRAM | Keep the current TGPulse database/loader checks. The System option controls only the already identified `315-5711.bin` repair; SM2's ROM CRC Verification controls chip validation and does not implement bad-dump repair. Prioritize a Model 1 sample campaign, then adapt SM2's `Automatic Initial NVRAM Setup` to validated Model 1 samples, only for a genuinely new save. Existing `.srm` and native defaults retain precedence. |
| Linked Cabinets | Later: TGPulse's COMM board and desktop TCP transport exist, but a Libretro transport and per-game session rules need design and validation. |
| Renderer, internal resolution and filters | Begin with native software output. The desktop `wgpu` path is a reference for a later frontend-owned GPU path, not a ready Libretro renderer. |
| Aspect, widescreen and timing | Native timing remains the default. The opt-in 60 Hz mode renders every machine frame at a faster cadence with matching audio speed; it does not duplicate video. The Libretro Aspect Ratio option now follows SM2's geometry-only Auto/4:3/16:9 approach: Auto reads VR's saved monitor setting and defaults other sets to 4:3, while the framebuffer stays 496×384. TGPulse's forced On mode really widens the 3D field of view and optionally stretches 2D layers; it remains for a renderer that supports it. |
| Audio balance and music | SM2 changes balance and DSB music volume live; Supermodel changes sound/music volume live. The Model 1 adapter applies the existing standalone per-source gains live, with Mute as the zero-gain choice and a conditional DSB selector. It does not import Model 2 SCSP controls. |
| Gun, crosshair and off-screen reload | No current Model 1 database set uses the Model 2 lightgun profile. SWA's P2 gunner uses stick/fire signals, so these SM2 options do not transfer. |
| Shifter, steering response and rumble | VR/VFormula rumble now uses its own drive decoder and standalone pad policy through Libretro. Any future shifter or steering-response option still requires Model 1 input evidence; do not copy Model 2 cabinet presets. |
| Diagnostics and per-game EEPROM options | Adapt SM2's opt-in `NVRAM Settings` only from a reviewed Model 1 setting catalogue with validated values, checksums and calibration boundaries. A compact timing display remains optional. |

## Historical first-slice criteria

1. Build a loadable core without changing the standalone target. Reject a
   Model 2 ZIP and a missing/invalid Model 1 ZIP with clear frontend messages.
2. Load `vr`, advance native Model 1 frames, deliver 496 x 384 video and stereo
   audio, and map Coin, Start, Test, Service, steering, pedals and a view button.
3. Verify reset, unload/reload and shutdown without stale devices or host paths.
4. Confirm the initial Core Options and per-game descriptors appear in
   RetroArch; confirm changed values affect the intended output.
5. Keep ABI/build tests, no-ROM launch checks, real-ROM gameplay, physical
   controller tests and save persistence as separate evidence. Expand to `vf`,
   `wingwar` and `swa` only after the first path is stable.

These criteria describe the initial checkpoint and do not form a second
current roadmap. The Libretro cabinet mapping lives in its adapter; the imported desktop input
path remains unchanged. The existing desktop cabinet-trace regression test
still passes.

## First-slice evidence and remaining limits

- `cargo test --offline --workspace` passed with local TCP socket permission.
  The standalone's cabinet trace remained unchanged; its imported input path
  is untouched by the Libretro adapter.
- The macOS ARM64 release core built and exported the expected Libretro entry
  points. A direct ABI run with a complete `vr.zip` loaded the game, delivered
  1,000 frames, 776,071 stereo audio frames and changing 496 x 384 images; the
  final image showed the in-game ranking/attract screen.
- An isolated RetroArch 1.22.2 run loaded the core and `vr` content, registered
  Core Options v2 and input descriptors, negotiated XRGB8888, initialized
  CoreAudio and exited on its bounded frame run. This is launch/integration
  evidence, not physical controller or gameplay acceptance.
- Save RAM and Save States now use the existing Model 1 NVRAM and machine-state
  APIs through fixed frontend buffers. Their direct ABI round trip and
  corruption rejection passed; real frontend and in-game persistence evidence
  is recorded separately below.
- RetroArch's automatic screenshot was affected by its inactive-window pause;
  the inspected visible frame came from the direct ABI run. Actual in-frontend
  visual and controller acceptance remains open.

## Model 1 profile expansion evidence

- Adapter-owned cabinet mapping now covers VF P1/P2 and the separate Wing War,
  R360, SWA Pilot/Gunner and NetMerc layouts. Desktop Model 1 cabinet traces
  still match their recorded baseline. In SWA/SWAJ, P2 has its own stick on
  ADC 4/5 and Laser/Torpedo on IN1:04/08; its Start, View and throttle are not
  mapped. R360 has no view buttons. NetMerc has no invented Start switch.
- Profile selection checks each known set against its expected control scheme
  and ADC channel roles before advertising it. RetroArch receives named
  controller ports through `SET_CONTROLLER_INFO`, as well as per-port action
  descriptors; disabled ports stop contributing input. VR and Virtua Formula
  share the same generic profile name on both ports. Wing War
  revisions share a profile only where their declared wiring matches.
- RetroArch descriptors name the action for each port. Wing War uses D-pad
  directions for its four views, R2/L2 for Throttle Up/Down and left stick for
  flight X/Y. SWA uses Pilot VR1 on D-pad Down with D-pad Up as an alternate, matching standalone, and separate P2 Gunner descriptors.
- At this profile expansion checkpoint the full offline workspace suite passed:
  505 tests, 0 failures, 5 ignored.
  This included local TCP tests outside the sandbox and the unchanged desktop
  cabinet trace. The macOS ARM64 release core built. Direct ABI runs with
  complete `vf`, `vformula`, `wingwar`, `wingwar360`, `swa` and `swaj` archives
  each produced 120 video frames and 93,128 stereo audio frames. Synthetic P2
  input tests verified independent VF and SWA wiring.
- Isolated RetroArch runs for `vf` and `swa` registered Core Options v2 and
  input descriptors, negotiated XRGB8888 and exited successfully after their
  bounded frame runs. The frontend stored its options under the test directory;
  no global TGPulse-Next configuration was created.
- The available `netmerc.zip` failed the complete-ROM-set check, so its real
  archive load remains unverified. These ABI and frontend runs do not establish
  gameplay, physical controller behavior or sound quality. At the time of
  these first-slice runs, only Smooth Shadows and Master Volume were exposed;
  per-source audio was added later. Rumble and linked cabinets still need
  separate validation. Save persistence needs
  a real operator-setting change and reload in RetroArch.

### Profile review against SM2-Emu Libretro

SM2-Emu Libretro selects named cabinet layouts only when the known game and
declared input signature match, publishes controller names with
`SET_CONTROLLER_INFO`, publishes action descriptors separately, and clears
both surfaces on unload. TGPulse-Next now follows that frontend contract for
the available Model 1 metadata: known set name, control scheme and ADC role
order. Its database does not encode SM2's full digital input signature, so
the digital bit wiring remains explicit in the Libretro adapter and is checked
by cabinet tests. The imported desktop trace remains unchanged. This does not replace a
physical controller test.

After the profile correction, the offline workspace suite passed 506 tests
with 0 failures and 5 ignored. Direct ABI runs loaded `vr`, `vformula`, `vf`,
`wingwar`, `wingwar360`, `swa` and `swaj` with their named controller ports,
30 native video frames and 23,282 stereo audio frames each. Isolated
RetroArch runs for `vf` and `swa` registered both controller information and
input descriptors, then exited successfully. The control menus were not
inspected visually; physical gamepad behavior remains unverified.

## Framework adaptation and future upstream imports

For each frontend decision, first inspect the SM2-Emu Libretro implementation,
then check whether Model 1 has the same machine capability and user-facing
meaning. Keep the interface convention when the behavior matches; translate
only the hardware details in the adapter. Defer a menu item when its effect is
not implemented or verified.

| Choice | SM2-Emu solution | Model 1 adaptation now | Condition for more work |
| --- | --- | --- | --- |
| Repository boundary | Libretro code under `src/libretro/`; machine stays frontend-neutral. | ABI, options, descriptors, controller profiles and native input translation live under `crates/tgpulse-libretro/`. | Add a neutral machine accessor only when an existing one cannot serve the frontend. |
| Content | External `games.xml`, chip validation and game metadata. | Reuse embedded `roms_db.dat` and the existing loader; `identify_complete()` is the single generic core addition for a self-contained ZIP. | Do not duplicate the catalogue or add a system file unless Model 1 resources require it. |
| Controller UI | Named cabinet families, per-port controller info and action descriptors. | Use the same `Driving:` / `Joystick (Standard):` style; Model 1 flight profiles use `Flight:`. Distinguish SWA Pilot/Gunner per port, R360 from Wing War, and only group revisions with matching metadata. | Recheck names, actions, channel order and idle values if upstream changes game definitions. |
| Core Options | System, Video, Audio and Input categories; applicability controls visibility. | Known Bad Dump ROM Repairs (System), Smooth Shadows and A/V Timing (Video), Master Volume and per-source Model 1 Gain selectors including Mute (Audio) are registered. Load-time choices are read before content; DSB controls use visibility hints. | Add Input when its first working option is available; do not show empty or inert settings. |
| Video/audio | Explicit native timing and frontend callbacks; optional renderer and source controls. | Native 496×384 software frames and clock-derived timing; optional 60 Hz faster machine cadence with per-frame stereo resampling. Model 1 source gains/mutes change output live. | Add GPU or aspect controls only with an implemented path and separate verification. |
| Persistence/link | Fixed Save RAM buffer and serialized state callbacks; frontend owns files and slots. | A fixed set-identified Save RAM buffer wraps `nvram_blocks`/`set_nvram_blocks`; a fixed serialization buffer wraps `save_state`/`load_state` and preserves the adapter's VR control ramps. COMM-linked states remain rejected by the machine API. | Validate RetroArch persistence and state restore independently; linked cabinet transport remains a later milestone. |

The imported source remains structurally intact: `crates/tgpulse/src/input/`
has no port-specific edits. The only upstream-facing implementation change is
`roms_db::identify_complete()`; the Cargo workspace adds the adapter target.
For a future TGPulse-Next import, compare the old and new upstream revisions,
revalidate this one loader API, then rebuild the standalone and Libretro
targets and run the cabinet/ABI and frontend checks. Record upstream imports
separately from adapter changes when commits are authorized.

## Model 1 NVRAM acquisition checkpoint

The [acquisition procedure](NVRAM_CAPTURE.md) and
[operator catalogue](MODEL1_DIAGNOSTIC_SETTINGS_CATALOG.md) now cover every
catalogued discrete operator value for the nine starting parent/clone sets:
110 fields and 953 isolated saved values. NetMerc was excluded from this
initial campaign because it did not start. U1 now verifies startup; its complete
operator-setting acquisition is the separate U3 activity. Diagnostic YAML and the
[review workbook](model1_core_options_review.xlsx) hold the per-set results.
The proposed `Automatic Initial NVRAM Setup` and `NVRAM Settings` menus await
the user's revised workbook. Workbook proposals do not authorize an override.
See the [Libretro roadmap](LIBRETRO_ROADMAP.md) for implementation status.

All 10 local archives loaded for diagnostic capture; `netmerc` required the
complete archive in the TGPulse checkout because the external collection lacks
`netmerc_nvram.bin`. Every Save RAM buffer was 65,728 bytes with a valid
container CRC. At frame 120, Wing War EEPROM was still all `FF`; at frame
600 its initialized bytes differed from that blank image and remained stable
at frame 1200. The five clone captures differed from their parent in backup
SRAM at frame 600; `vformula`, `swaj` and `wingwar360` also differed in
EEPROM. Therefore no clone is approved to inherit a parent template yet.
The VR 4:3 sample differs from its 16:9 baseline at EEPROM offsets `0x08`,
`0x09`, `0x0A` and `0x7D`; the Monitor field is not a safe one-byte override.
This is acquisition and reload evidence only; it does not certify gameplay,
the Test menu for every game, or a safe automatic preset.

## Persistence checkpoint

SM2-Emu keeps a fixed Save RAM buffer untouched until the frontend has had a
chance to restore its `.srm`, then imports it into the machine. The Model 1
adapter follows that sequence with the existing 64 KiB backup SRAM and 128
byte I/O EEPROM accessors. A 64 byte header identifies the set and checks the
payload; a blank buffer leaves the ROM set's factory defaults intact. The
frontend owns the save path. No standalone `nvram/` path is used.

RetroArch may query `retro_serialize_size` before loading content, so the
adapter reports a fixed 64 MiB plus 32 byte envelope, matching the existing
Model 1 state API's maximum. The envelope stores the actual state length and
the adapter's three Virtua Racing steering/pedal ramp values; the machine
payload remains the standalone-compatible `TGP1STAT` format. `load_state`
validates ROM identity and state integrity before changing the machine.
The COMM-board restriction remains in force.

The direct ABI test with `vr.zip` passed Save RAM export/reload, state
save/advance/load/re-save equality, and rejection of a corrupt state without
machine changes. The Save RAM reload test changed one SRAM byte in a valid
set-identified buffer and confirmed it survived import and a native frame.
In a separate direct ABI session, the Virtua Racing Test menu changed
`MONITOR` from `16:9 WIDE` to `4:3 NORMAL`. Selecting `EXIT`, then
`YES (SAVED)`, changed EEPROM byte `0x0A` from `1` to `0` in the exported
65,728 byte Save RAM. A fresh core instance imported that exact buffer and
showed `4:3 NORMAL` in the Test menu. This verifies an operator setting
changed by the emulated game and restored through the Libretro Save RAM ABI.
An isolated RetroArch run wrote a 65,728 byte `vr.srm` with the expected
header. A fresh 120-frame RetroArch launch with a valid pre-existing `.srm`
preserved its modified SRAM byte when the frontend saved the file on exit;
the rest of the payload changed as the machine ran. This verifies frontend
Save RAM import/export. The operator-setting sequence has not yet been
repeated through RetroArch's interactive input and `.srm` handling.

RetroArch saved and loaded a state through its slot commands. Two fresh
launches using `--entryslot=0` reported successful state loading and displayed
the Virtua Racing ranking scene; a launch without that state displayed the
Test menu at the same bounded run. At 15 frames, the central game region of
the two loaded screenshots matched byte for byte and differed from the
no-state run. The complete screenshots differ because RetroArch overlays a
progress notification below that region. Loading the exact RetroArch state
bytes through the Libretro ABI twice and advancing 60 native frames also
produced identical video hashes for every frame. Interactive RetroArch Test
menu navigation and a physical controller test remain separate validation
tasks.

### Isolated RetroArch Save State round-trip (2026-10-01)

The Model 1 runner in `tools/test_retroarch_savestate.py` adapts SM2-Emu's
`scripts/test-retroarch-savestate.py` to the current core and RetroArch command
behavior. It uses a local RetroArch configuration, save paths, state paths,
history/playlists and screenshots, plus a process-owned UDP command port. The
local adaptation uses `PAUSE_TOGGLE`; it briefly resumes emulation for each
`LOAD_STATE`, then pauses and advances one frame before capture. It records
core, ROM and state hashes, frontend version, log confirmations and clean exit.
Its macOS screenshot helper is in `tools/retroarch_savestate_helpers.py`.

The release core built offline for this check had SHA-256
`cf1ebcbfac4b924621545e9eb33a56256650781b39cedf9bf46f069aad562e8d`.
RetroArch was 1.22.2 (Git `9be7ec92`). In attract mode, `vr`, `swa` and
`wingwar` each saved a 67,108,920-byte state, visibly advanced, loaded the
state twice, produced identical central game regions on both restores, advanced
again after the first restore, and exited cleanly. The saved and restored game
regions also matched exactly for `vr` and `wingwar`. For `swa`, the two
restores matched one another, but the restored region differed from the image
captured at save time; that stricter visual equivalence remains unverified.
These checks do not establish gameplay or physical controller behavior.

Local raw evidence is under `validation/savestate-frontend/2026-10-01/` and
is ignored by Git because it contains generated states and screenshots. The
project runner itself completed the `vr` run; the `swa` and `wingwar` evidence
was produced by the temporary adaptation of the same reference runner and
copied there. To repeat a run, invoke the project runner with explicit
`--retroarch`, `--core`, `--system`, `--rom`, `--set-name` and a new `--output`
directory. `--boot-wait` and `--advance-wait` adjust capture timing. Never
reuse an existing output directory.

### Virtua Racing frontend gameplay smoke (2026-10-01)

`tools/test_retroarch_gameplay.py` adapts SM2-Emu's
`scripts/smoke-retroarch.py` replay-v1 input method. It feeds Coin, Start,
steering, accelerator, view and gear actions through RetroArch using an
isolated configuration and fresh save paths. The 3,800-frame run on RetroArch
1.22.2 exited normally and produced a race screenshot showing a timed lap,
position and moving car at 211 km/h. Its 67-second stereo PCM capture contains
nonzero samples; the recording is normalized to a standard WAV file for
inspection. This verifies a bounded frontend gameplay path and audio
production for `vr`. It does not verify sound quality, a physical controller,
other sets or sustained gameplay stability.

The generated replay, screenshot, original and normalized recordings, log and
result are under `validation/gameplay-frontend/2026-10-01/vr/`, ignored by Git.
The runner requires explicit `--retroarch`, `--core`, `--system`, `--rom` and a
new `--output` directory. It never installs a core globally.

## Timing / FPS overlay adaptation (2026-10-01)

Reference: SM2 `src/libretro/timing_overlay.{h,cpp}`, `core.cpp` and
`core_options.h`, plus Supermodel's Libretro timing panel. The adapter reuses
TGPulse's existing imgui 0.12 dependency, with no new dependency installation.
Its software triangle compositor follows SM2's atlas sampling and XRGB blending;
the context stays suspended between draws and is owned by the adapter.

The Video option matches SM2: Off (default), Auto (13 px), or 11–14 px.
The panel reports 61-callback averages for machine, video (including the panel
and video callback), audio/pacing (including resampling and audio callback),
total `retro_run`, and worst callback. Actual FPS measures callback-start
intervals; Engine cap is 1000/(machine+video ms), and Callback cap is
1000/total ms. These capacities are estimates and may include frontend blocking.
Measurements restart after reset, state load or font/enable changes, with no
telemetry in the machine Save State. The Off path creates no ImGui context.

24 offline tests cover window arithmetic, option registration and bounded
software drawing. A native 496×384 preview was inspected. With a 13 px font,
120 warmed synthetic draws averaged 0.421 ms in the optimized test build; this
measures only the panel and is not a real-game benchmark. The macOS release
core was built and installed as `tgpulse_next_dev_libretro`, with SHA-256
verified against the build. User-run presentation checks remain separate.

To reproduce the native preview and bounded drawing measurement:

```sh
TGPULSE_OVERLAY_PREVIEW=/private/tmp/tgpulse-overlay.rgb \
  cargo test --offline -p tgpulse-libretro \
  timing_overlay::tests::panel_draws_readable_content_inside_frame_bounds -- --nocapture
```

The output is headerless RGB, 496×384, and is kept outside the repository.

## Controller correction (2026-10-01)

Reference inspection: SM2 `src/libretro/input.cpp` controller configuration,
operator descriptors/polling and VR4 mapping; Supermodel
`Src/OSD/Libretro/libretro.cpp` VR4 descriptors; standalone TGPulse
`crates/tgpulse/src/input/signals/mod.rs` View1–View4 defaults.

VR/Virtua Formula and Wing War views follow SM2 and the standalone:
Down=1, Left=2, Right=3, Up=4. Supermodel currently uses a different ordering;
the chosen ordering preserves the existing TGPulse standalone contract.
Driving views never contribute to steering; steering uses the left analog stick.
The deterministic gameplay replay uses Down for VR1.

Both ports advertise the same generic family name, including single-player
cabinets. SWA retains the Pilot/Gunner exception. Both ports expose Test=L3 and
Service=R3, and either port operates those shared switches independently of the
other port's enabled state. P2 Coin feeds native Coin 2 in VR, Virtua Formula,
VF, Wing War R360 and SWA; Wing War and NetMerc omit that unused native input.
P2 gameplay remains limited to VF and SWA; no extra Start is invented for
single-player cabinets or the SWA Gunner.

Regression checks exercise all four driving view switches without steering,
both controller names, both operator descriptors, native Coin 2 routing and
disabled-port isolation across every Model 1 profile. These are callback and
native-port checks, not physical controller acceptance.

Verification: 26 adapter tests passed, offline macOS release build passed and
the dylib loaded with Libretro ABI version 1. The authorized RetroArch
`tgpulse_next_dev_libretro.dylib` and Development `.info` were updated; installed
and built SHA-256 both equal
`ef010149041ed6edc84c69c223272ea53c2993d4abb804bbde2c0c783bc93176`.
Physical controller confirmation remains user-run.

## Audio selector simplification (2026-10-01)

Per the user instruction, the four separate source Mute switches are removed
from modern and legacy Core Options. Source Gain values display Mute for zero;
modern options retain the numeric stored value `0`, while legacy options use
`Mute` because their API has no separate display labels. Both decode to zero
output gain without pausing chip/decoder execution. Master Volume is unchanged.
Old separate mute keys are no longer read. Standalone audio controls are unchanged.

Verification: 26 adapter tests and the offline macOS release build passed.
The Development core was installed in RetroArch with matching SHA-256
`58fbdbb31bbb6a5758189d92384e582bbde22265fa7d973d8054186bab45c02d`.

### Gain Auto and explicit zero (2026-10-01)

The current selector order is Mute, Auto, 0%, 10%, …, 100%, with Auto default.
Auto resolves to the standalone source levels: MultiPCM 1/2 50%, FM 30%, DSB
100%. Mute and explicit 0% both silence output. Modern options use distinct
`mute`, `auto` and numeric values; saved numeric gains remain accepted. Legacy
options put Auto first (required for their default) and Mute last, so circular
left/right navigation retains the intended order. Value colours are controlled
by the frontend; Libretro option values have no colour field. Master is unchanged.

Standalone supersampling is renderer sampling/resolution work; sRGB correction
is presentation texture/surface handling in `platform/video.rs`. Neither is an
emulated hardware setting. Both need explicit adaptation in the GPU phase; the
current software XRGB8888 callback has no standalone wgpu presentation surface.

Verification: 26 adapter tests passed, including Auto decoding and the new
Mute/Auto/zero order; macOS release build and ABI load passed. Development
installation SHA-256 matches the build:
`74d19131a336813c22026e1b87e1ea93981ecabb029af78e50a53f532d6b2820`.

### Per-game Gain storage (2026-10-01)

Superseded by the global Gain restoration documented below (2026-10-02).

Reference: SM2 `src/libretro/core_options.h` registers game-qualified NVRAM
keys and changes visibility in `set_option_game`; `core.cpp` applies that scope
at load/unload. Model 1 Gain keys now follow the same pattern:
`tgpulse_next_<set>_<source>_gain`. All ten catalogue sets have independent
keys, including clones; only the loaded set's sources are shown, and DSB remains
conditional on the actual fitted sound board. Legacy frontends without visibility
hints receive set-qualified labels. Auto retains standalone levels, while Master
remains global. Previous global source Gain keys are no longer read; each set
starts at Auto until explicitly configured. Values remain frontend-owned Core
Options, not game EEPROM fields. No extra per-game files are introduced.

Gain selection is read after identification before machine creation, refreshed
live using the loaded set name, and hidden on unload or failed load.
Cross-set selection and exact-set/DSB visibility are covered by regression checks.

Stretch 2D Layers When Widescreen is explicitly in the GPU roadmap row alongside
real wider 3D rendering. The software adapter currently exposes geometry-only
aspect selection, which stretches the complete image through the frontend and
cannot selectively stretch tile layers. Standalone has the independent toggle.

Verification: 27 adapter tests, offline macOS release build and ABI load passed.
Development installation SHA-256 matches the build:
`cb6e28535523063361797d4f02812d02d4e11e1f9380b273ba0cdf02b482ebe1`.

## Flight profile names and SWA VR1 correction (2026-10-02)

User-requested names: `Flight: Wing War + V4`,
`Flight: Star Wars Arcade (Pilot) + VR1` and
`Flight: Star Wars Arcade (Gunner) + VR1`. Wing War revisions retain the same
name on both ports; the R360 profile remains separate because it has no views.

Standalone `input/signals/mod.rs` binds View1 to D-pad Down;
`input/sampling.rs` aliases the View4 gamepad binding (default Up) for SWA/SWAJ.
`input/cabinet.rs` maps that P1 signal to IN1:0x10 and provides no P2 View.
The core now reproduces Down/Up on Pilot, replacing X. Gunner has the requested
profile name but no invented VR action. SM2's single-view defaults use Down,
while Supermodel uses Up; the standalone alias accepts both.

Verification: 28 adapter tests passed, including independent Down/Up activation
of Pilot VR1, inactive old X binding and inactive Gunner view directions.
macOS release build and ABI load passed. Development installation hash matches:
`a17a764e96c151e7f926bfa1b8461bca48c938c78d1c6e205dfa550e9fc7b1fe`.

## Reviewed binding corrections (2026-10-02)

See `LIBRETRO_MODEL1_BINDING_AUDIT.md` for references and the reviewed correction.
VF face buttons now drive the corrected Kick/Punch native bits. User instruction
removes VF shoulder aliases entirely; descriptors remain face-only for actions.
Flight profiles gain standalone shoulder aliases and the Pilot right-stick
throttle. Flight ADC sampling preserves standalone travel outside the deadzone;
VR steering keeps its racing curve. The Wing War profile is
`Flight: Wing War + VR4` (V4 was a typo).

## Wing War throttle polarity correction (2026-10-04)

For Wing War World/US/Japan and R360, the user requested a lower ADC for
Throttle Up while preserving the existing physical response of L2 and R2.
The right-stick bindings remain Up = Throttle Up and Down = Throttle Down;
their resulting ADC direction is reversed. R2 now binds Throttle Up and L2
binds Throttle Down. ADC 2 remains centred at 128, with Up reaching 1 and
Down reaching 255. Star Wars Arcade retains its previous mappings and
28–228 range. This is an explicit Libretro adaptation to the imported
standalone mapping; the separate standalone path was not changed.

## Master Volume correction (2026-10-02)

The old adapter selector had only 100/0/50/150/200%, in that non-progressive
order, with an unrequested `Muted` label. Master now uses 0–800% in 10% increments, default 100%, applied live.
At the user's subsequent request, the zero value is labelled OFF. The range follows the
standalone `gui/mod.rs::master_volume_slider` (maximum 800), while the 10% step
follows Supermodel `supermodel_sound_volume`. SM2's music volume is a separate
source control and must not replace Master semantics.

Core Options v2 presents ascending values, including OFF for zero rather than Mute.
The legacy variables protocol requires the default first; it retains 100% as
its default and lists remaining values numerically. Per-set Gain selectors keep
their separately agreed Mute/Auto/0–100% contract.

Both load-time and live-option parsing accept the same 0–800% range. A direct
ABI probe restored the identical machine snapshot for each volume, then compared
46566 interleaved PCM samples per selection. Each sample matched the exact
linear multiplier with integer conversion and native saturation; RMS increased
monotonically. In the bounded VR segment, peaks were 0/7/35/63/70/77/105/140/280/560
at 0/10/50/90/100/110/150/200/400/800%. This verifies PCM scaling, not perceived
loudness on a physical audio device.

Repeat the check without user-save writes:

```sh
python3 tools/test_libretro_master_volume.py \
  --core target/release/libtgpulse_next_libretro.dylib \
  --rom '/absolute/path/vr.zip' \
  --output /private/tmp/model1-master-volume-new.json
```

Measured evidence: `/private/tmp/tgpulse-master-volume-progression.json`.
The 34 adapter checks and macOS release build pass. Development deployment
follows the normal hash-checked installer procedure.

## Widescreen and supersampling reference adaptation (2026-10-02)

The current standalone Model 1 renderer supplies 683 x 384 wide projection,
centered or stretched native background/foreground, and supersampling scales
1–4. Supermodel supplies renderer-option restart semantics and sample-count
labels; SM2 supplies frontend context and visibility discipline. These are now
adapted in the Libretro adapter using the unchanged standalone shaders.
All widescreen modes apply only to an effective wide aspect; 4:3 stays native.
Supersampling applies at either aspect. Defaults are Stretch Entire Image and
1x; Software ignores both hardware selectors. The user deferred sRGB because
the final-surface-format contract needs a separate adaptation decision.
See the sole roadmap and GPU verification document for current phase evidence.

## Driving analog option review (2026-10-02)

Inspected current SM2 `src/libretro/core_options.h` and `input.cpp`, and
Supermodel `Src/OSD/Libretro/libretro_core_options.h`. Both expose Driving
Steering/Accelerator/Brake Output Range, 50–150% with 100% defaults, and a
separate Steering Response selector. They adjust the emulated analog output
and therefore apply independently of frontend button/axis remapping.

The Model 1 adapter's `vr_inputs` currently uses steering center 0x80 and
pedal rest 0x20, with bounds 0x20–0xE0. Adapt live range scaling for VR/VFormula
around those native reference positions; 100% must reproduce current output.
Validate reference-specific ADC presets before exposing them. The Model 2
00–C0 pedal preset cannot be labelled unchanged against Model 1's 20–E0 path.
Flight analog channels have distinct centers, limits and polarities; this
driving adaptation does not imply new flight options or inputs.
The three output ranges are implemented in the adapter, applied only to the
VR/VFormula profiles. Shared names match SM2/Supermodel. Values are 50–150%
in 10% steps; modern options default to 100%, legacy options place 100 first.
Invalid selections fall back to 100%. Updates are read on the next input frame.
Frontend visibility hints hide these controls for other profiles and no content.
Scaling follows sampling, keeping its ramp state unscaled and avoiding feedback
when selections change. Values clamp to the native 20–E0 span; steering scales
around 80, pedals around 20. Native ADC mirrors receive the same scaled values.
Flight and digital profiles do not enter this path.
The generic reference percentages are retained; hardware-specific 63% and
75.3% ADC presets are omitted because their labels describe different native
endpoints. Steering Response remains a separate roadmap activity.

Verification: 39 adapter checks pass. The range checks cover every native ADC
value at 100%, monotonicity across all selectable percentages, rest/center,
saturation, digital-bit preservation, ADC mirrors, modern labels/defaults and
invalid-value decoding. macOS ARM64 release compilation succeeds. The existing
RetroArch runner accepts `--steering-range`, `--accelerator-range` and
`--brake-range`; these select only the isolated run's options. A bounded Software
run selects 50/150/80, with reports in `/private/tmp/tgpulse-ranges-frontend`.
This verifies frontend registration and frame delivery, not physical controls.

The 180-frame RetroArch check exited normally, delivered a PNG and retained
the requested 50/150/80 selections in its isolated core options file. The
Development installer then verified core SHA-256
`dc5ab0936fc7325a50f7e73f72e45545658a71f3baf623942a2e1e5db1b94231`
and `.info` SHA-256
`e500861081e28bc565fc3ed4772e6153743f993b012de9374c16cb0ce6295d8b`
in the established local RetroArch Development destinations. No commit/push.
Reasoning required: Medium. Account usage remained at the displayed 45%;
next reset 2026-10-07 15:58:51 CEST. No sub-percentage task cost can be inferred
from the unchanged rounded account reading.

## Reviewed automatic NVRAM and menu adaptation — 2026-10-02

Current SM2 initial template generation, core option defaults and display
callbacks, and Supermodel's seeding/manual-option lifecycle were inspected
before this phase. Model 1 uses those procedures with complete set-specific
backup RAM/EEPROM images and its own independently acquired encodings.
The approved workbook and YAML select 40 fields across nine sets. VR's
additional automatic override is CABINET = SPECIAL; it does not change the
native CABINET selector default. Parent and clone keys and templates remain
independent. Detailed evidence and reusable commands are in
[NVRAM_CAPTURE.md](NVRAM_CAPTURE.md); implementation status stays in the single
[roadmap](LIBRETRO_ROADMAP.md).

Automatic Initial NVRAM Setup defaults Enabled and runs only after the frontend
has supplied Save RAM, preserving a valid matching save. NVRAM Settings defaults
Disabled. Only the active set's fields appear when enabled, including paused
menu visibility updates through the reference display callback. Concrete values
retain native defaults; there are no extra Keep/Auto selector values. Updates
modify reviewed bytes plus native CRC/mirror regions and reset only when data
changes. Exact Save State restoration is not overwritten automatically.

The final macOS ARM64 build passed 41 adapter tests, nine-set ABI verification
and bounded real RetroArch persistence checks. It was installed through the
established atomic Development installer, which verified core SHA-256
`7ca24e7646b0a4568ca089325c270d2a7eda2a724c97c6ba6fde73231dc64629`
and info SHA-256
`e500861081e28bc565fc3ed4772e6153743f993b012de9374c16cb0ce6295d8b`.
No commit/push. Required reasoning: High. Account usage: 46%; next reset
2026-10-07 15:58:51 CEST. The previous displayed account reading was 45%;
this rounded account-wide change is not a precise task consumption measurement.

## Driving Steering Response — 2026-10-02

Inspected current SM2 `src/libretro/input.cpp` and `core_options.h`, and
Supermodel `Src/OSD/Libretro/libretro.cpp` and `libretro_core_options.h`.
Their complete 256-entry FBNeo tables were compared and are identical. The
adapter retains the exact table and the quadratic Progressive law; names,
choice keys, Linear default and curve-before-range ordering match both ports.

Model 1 steering is 20–80–E0 rather than 00–80–FF. Progressive uses the same
nearest-rounded square law over each native 96-unit side. FBNeo normalizes
left/right sides to the reference's 128/127 spans, looks up the unchanged table,
and maps back with nearest rounding. This preserves the native center and
full-lock endpoints. Reference table asymmetry and endpoint saturation are
retained. No new hardware inputs or frontend bindings are introduced.

`tgpulse_next_steering_response` is visible only for VR/VFormula profiles.
Load-time and live option updates use the same parser; unknown values fall
back to Linear. Curves affect delivered steering only, leaving native ramp
state untouched; output ranges run afterwards and refresh existing ADC mirrors.
Pedals, buttons, flight and fighter profiles retain their existing paths.

44 adapter checks pass, including native-span monotonicity, center/endpoints,
fine-center behavior, default Linear identity, all range combinations, pedals,
digital bits and ADC mirrors. macOS ARM64 release compilation succeeds. The
existing isolated RetroArch runner's `--steering-response fbneo` check with
120% steering range completed 180 frames and produced a PNG and retained
options at `/private/tmp/tgpulse-steering-response-frontend`. This verifies
registration and frame delivery, not physical-controller acceptance.
The Development installer verified core SHA-256
`c1ef527b2dc8d63a2bd9d505000f74c11d9abaa36067f27275c812b250c1086d`
and matching info SHA-256
`e500861081e28bc565fc3ed4772e6153743f993b012de9374c16cb0ce6295d8b`.
Required reasoning: Medium. Account usage remains 46%; reset
2026-10-07 15:58:51 CEST. An unchanged rounded account reading does not quantify
this phase's exact consumption. No commit/push.

## Linked-cabinet adaptation — 2026-10-02

The current SM2 Netpacket implementation/options/multi-instance runner and
Supermodel Libretro NetBoard/API were inspected. The adapter reuses the native
TGPulse M1COMM HLE and replaces desktop TCP with frontend-owned complete-frame
Netpacket delivery. Parent/clone selectors are independent and default Disabled;
roles remain the reviewed native NVRAM fields. Save RAM remains supported;
Save States report unavailable while COMM is fitted. Linked reset uses the
native constructor with retained immutable ROMs, preserving NVRAM and gains.

All six eligible sets passed actual local RetroArch pairs, and VR passed the
MASTER/SLAVE/LIVE relay, each with at least 600 consecutive native online frames.
The final build passes 47 adapter checks and is installed under the agreed
Development name with SHA-256 verification. Procedures, exact hashes, native
limits and evidence are in [the linked-cabinet guide](LIBRETRO_MODEL1_LINKED_CABINETS.md).
All nine single-cabinet NVRAM/Save State regression sets pass, recorded at
`/private/tmp/tgpulse-netpacket-unlinked-regression`. Status stays
in the single roadmap. Reasoning required: High; account usage 47% (previous
46%), next reset 2026-10-07 15:58:51 CEST. This rounded account-wide change is
not an exact measured phase cost. No commit/push.

## Interim M1 core identity — 2026-10-02

At the user's request, the current artifact is now `tgpulse_next_m1_libretro`
(with the platform's library prefix/extension), and the Libretro runtime name
is `TGPulse-Next M1`. The agreed local Development installation is now
`tgpulse_next_dev_m1_libretro.dylib` with matching `.info` and display name
`Sega - Model 1 (TGPulse-Next M1 Development)`. This is the current interim
Model 1 core; its description explicitly says the real Model 1 Tiny build will
replace it. No component selection is claimed by this rename.

Cargo library naming, core metadata, installer defaults and current deployment
instructions were updated together. Historical evidence retains its original
artifact names. Offline macOS release compilation succeeds; an ABI query
verifies the new runtime name and existing ZIP/full-path contract. Installed
core SHA-256: `3722819382a5de427146b9461a143ed19a7cd39133e6a2c551ae5410e7630dd9`;
info SHA-256: `d7a20dbb9becbad9cc3bd559c3e06a43dab7e87ebae58813448c1205793e68ed`.
The two old Development filenames were hash-checked against the previous
verified installation, copied to a temporary backup, then removed after the
new installation succeeded. Backup:
`/var/folders/kl/swh8kv5j2zb0ptqpmyg5dzfw0000gn/T/tgpulse-m1-rename-backup-hm0r8kms`.
Required reasoning: Low. Account usage remains 47%; reset
2026-10-07 15:58:51 CEST. No commit/push.

### Display-name correction — 2026-10-02

The user-selected exact identity is now **TGPulse-Next: Model 1 Development**
in the runtime ABI, source `.info` and installed `.info`. The earlier M1 display
name above is superseded. Artifact filenames keep the `m1` suffix. The installer
copies the canonical metadata directly. Release compilation and runtime-name
inspection pass; installed hashes are core
`557ced646e7d93d1964c1fb3162ca052115dcddc06835a27110ebb149e00ebcf` and info
`b7a35c7d2c11dc9ed497eb6f966d5be36aa91b57917c5c48532d276082bdae60`.
Reasoning: Low; account usage remains 47%; reset 2026-10-07 15:58:51 CEST.

### Original descriptive name restored — 2026-10-02

The user restored the original display name:
**Sega - Model 1 (TGPulse-Next Development)**. The short metadata name is
`TGPulse-Next Development`; the runtime ABI again reports `TGPulse-Next`.
This supersedes the display-name correction above. Artifact filenames retain
`m1`: `tgpulse_next_dev_m1_libretro.dylib` and matching `.info`. The interim
build description still records future replacement by the real Tiny build.
Offline release compilation, ABI name inspection, installed metadata equality
and installer hashes pass: core
`a16832495ec06cb186cb5a8e5017341787650a843382040f32a6120a097e625f`, info
`b6c71c1ba1d98801c42bac426de82911892057109232d1a440c5cc1e971324f8`.
Reasoning: Low; account usage remains 47%; reset 2026-10-07 15:58:51 CEST.

### Separate Core Name / Core Label — 2026-10-02

The screenshot clarified the two metadata fields. The final `corename`
(Core Name) is **TGPulse-Next: Model 1 Development**. `display_name`
(Core Label) is **Sega - Model 1 (TGPulse-Next Development)**. This supersedes
only the previous short metadata name. The runtime ABI still reports
`TGPulse-Next`; filenames retain `m1`. Canonical and installed metadata match;
core SHA-256 remains
`a16832495ec06cb186cb5a8e5017341787650a843382040f32a6120a097e625f`,
new info SHA-256
`8bc720a57504da7dd9543dc09b0626566896fce67c7112d4b82756154c01fdf1`.
Required reasoning: Low; account usage 47%; reset 2026-10-07 15:58:51 CEST.


### Global source Gain restoration — 2026-10-02

At the user's request, source gains now use global keys shared by all Model 1
parents and clones. Current references: SM2 `src/libretro/core_options.h`
(`sm2_music_volume`) and `core.cpp`; Supermodel
`Src/OSD/Libretro/libretro_core_options.h` (`supermodel_sound_volume`,
`supermodel_music_volume`) and `libretro.cpp`. Both apply fixed global audio
keys at load and during live updates. The smallest adaptation retains Model 1's
four source selectors and standalone reference mix.

Keys: `tgpulse_next_multipcm1_gain`, `tgpulse_next_multipcm2_gain`,
`tgpulse_next_ym3438_gain`, `tgpulse_next_dsb_gain`. Default: Auto, resolving to
50/50/30/100%. Mute and 0–100% in 10% steps retain their established behavior.
Modern and legacy registrations expose four source Gain options, without
ROM-set suffixes. Load and live updates read the same global keys. DSB remains
visible for every title and applies when a board is fitted.
Old set-qualified Gain keys are ignored. This supersedes per-game Gain storage
above and is a local change after the published 0.1.0.0 preview.

Verification: 47 adapter tests pass, including global modern/legacy option
registration, Auto fallback, numeric/Mute parsing and fitted-board visibility.
The offline locked macOS release build and native ABI/dependency/empty-lifecycle
gate pass. Development core and metadata were installed with SHA-256 equality:
core `b80c324380d040acbee9d350fe598436bb05b0f86ebb6a9f5460fd86e30314db`,
info `35e97e9499bbad25c2edb7fb3a15bea46691f015e8ba9417089349fd48cf423b`.
These checks do not constitute listening or gameplay validation.

The user also authorized local configuration cleanup. After confirming
RetroArch was closed, 48 stale source Gain/Mute entries were removed from the
core's options file: 40 per-set Gain keys, four previous global Gain values and
four obsolete Mute switches. A backup was saved next to the original file;
all other lines were preserved. Missing global source values use Auto.
No personal configuration or backup is added to the repository.
Required reasoning: Medium; account usage 49%; reset 2026-10-07 15:58:51 CEST.


### Global Gain release checkpoint — 2026-10-02

The global Gain restoration is now published as **0.1.0.1**, with upstream
snapshot unchanged and port revision incremented. All five native platform
jobs and publication gates pass. Downloaded release assets pass checksums,
exact inventory, metadata/version and tagged source-revision validation.
[Publication evidence](LIBRETRO_CI.md#verified-0101-publication-2026-10-02)
records the fixed source commit, CI run and archive hashes. The local
Development installation has also been rebuilt and hash-checked at 0.1.0.1.


### Model 1 single system build — 2026-10-02

Machine selection now occurs at compilation: the M1 adapter enables only
`model1`, excludes Model 2 modules and optional CPU dependencies, and embeds
only the ten canonical Model 1 catalogue records. Default standalone builds
retain both families. Common configuration/state layouts remain intact.
The user chose "Model 1 single system build" instead of the provisional Tiny
name. Artifact, Core Name and Core Label retain their agreed M1 Development
identity; the descriptions now reflect actual component selection.

[Build scope and evidence](LIBRETRO_BUILD_SCOPE.md) records the inspected
SM2/Supermodel references, feature boundaries, reusable commands, measurements,
full/M1/M2/adapter checks, nine-set pre-split runtime/state equality and actual
RetroArch Vulkan delivery. It also records package and installed hashes and
the distinction between local verification and the previous published build.
Implementation status and future priority remain solely in the roadmap.

## Timing transparency and menu grouping correction (2026-10-03)

The timing panel uses the reference 0.55 background opacity. Hardware output
now preserves its alpha through final 3D/2D composition; previously it became
opaque over 3D. Software composition is unchanged. Modern and legacy options
now share the System/Video/Audio/Input grouping and related-control ordering
from current SM2/Supermodel definitions. Details and verification are recorded
in `REFERENCE_WORKFLOW.md`. Release 0.1.0.2 remains the previous published
artifact until a separately authorized publication includes this correction.

The following local Widescreen refinement renames the control to Widescreen
Hack, makes Expand 3D View Only the default and preserves native-capable games
from every hack mode. The menu description states its non-native scope without
naming individual games. See `REFERENCE_WORKFLOW.md` for reference adaptation
and the 49-test/native-and-expanded Vulkan verification.

The subsequent Timing position correction maps the native game foreground
before drawing the panel in final render coordinates. The 8-pixel upper-left
anchor no longer inherits native 2D centering or stretching in wide 3D modes.
Vulkan checks of all three choices, including 2x supersampling, and 50 adapter
tests pass. Evidence and the minimal adapter change are in `REFERENCE_WORKFLOW.md`.

## General Core Option Visibility

General options remain visible regardless of the loaded title, including before
load and after unload, except source Gains. The adapter compares each Gain key
with `SoundSystem::sources()` on the loaded machine, without a title-name rule.
MultiPCM 1/2 and YM3438 appear for every loaded Model 1 set; DSB appears only
when the loaded machine has that board (Star Wars Arcade parent/clone).
No source Gain appears without content. Their
global values persist across games, and Master Volume remains visible.
Descriptions state applicability; runtime logic guards
unsupported titles or rendering paths. Reviewed per-set NVRAM fields and
Linked Cabinets selectors retain their explicitly agreed filtering. Native input
descriptors remain profile-specific. Sega NetMerc City Workaround is always visible
in Video, defaults to Enabled and applies live only to NetMerc.
