# U4: NetMerc Audio Donor Integration

## Reference And Adaptation

SM2 `PORTING_PLAN.md` phase 3.7 and `src/libretro/core_options.h` provide the
global Audio category, always-visible source options and explicit change scope.
Supermodel `Src/OSD/libretro/LibretroWrapper.cpp` retains separate sound/music
volume; its current roadmap has no donor-audio equivalent. TGPulse standalone
`app.rs` and `loader/audio_donor.rs` supply the actual donor and fallback policy.

Reuse the imported strict PCM loader and transactional `SampleBanks::apply`.
Keep option registration, paths and notifications in the narrow Libretro adapter.
No shared hardware, sound program, ROM archive or standalone behavior is changed
by U4. Alternative Audio Gains remains an eventual enhancement in the sole
implementation roadmap. I/O and Diagnostic Display BIOS lookup is separate.

## Option And Resources

| Property | Value |
| --- | --- |
| Key | `tgpulse_next_netmerc_audio_donor` |
| Label | Sega NetMerc Audio Donor (Restart Required) |
| Category | Audio |
| Values | Virtua Fighter, Virtua Racing, Star Wars Arcade, Wing War, Off |
| Default | Virtua Fighter (`vf`), matching upstream |
| Visibility | Always visible; affects only NetMerc |
| Change Scope | Content reload; a preference change never replaces live banks |

The selected `vf.zip`, `vr.zip`, `swa.zip` or `wingwar.zip` is searched only
beside `netmerc.zip`. Donor game ZIPs are never searched in the system directory
or process working directory. The alternate system path is reserved for board
BIOS resources, as expressly clarified by the user on 2026-10-04.

Only `m1audio:pcm1` and `m1audio:pcm2` ROMs are required. Both complete, nonblank
4 MiB banks must validate before replacement. Donor CPU, graphics, I/O and DSB
resources are unnecessary. Missing, malformed, incomplete or blank donors leave
both original banks and the native procedural flag intact; startup continues.
Off performs no donor search. The upstream procedural fallback is selected for
NetMerc's known blank sample-descriptor dump; substitute audio is not authentic
recovery of the missing original sound content.

## Notifications And Lifecycle

Following the user's refined notification policy, a successfully loaded donor
is silent. Fallback/procedural paths use the existing frontend message callback:

| Actual Path | Message |
| --- | --- |
| Off, Procedural | `NetMerc Audio: Procedural Fallback` |
| Donor Unavailable, Procedural | `NetMerc Audio Donor Unavailable - Procedural Fallback` |
| Original | `NetMerc Audio: Original Audio Incomplete (Bad ROM Dumps)` |
| Donor Unavailable, Original | `NetMerc Audio Donor Unavailable - Original Audio Incomplete (Bad ROM Dumps)` |

Audio warnings are sent once after the first delivered frame, after frontend
initialization and initial NVRAM/Holder events. The adapter prefers the extended
Libretro message interface: queued notifications, warning priority 3 for audio,
and native-equivalent duration of 300 frames (about 5.2 seconds). Older frontends
retain the 300-frame legacy message fallback. Existing informational notices
also use the queued interface when available, preserving their text and duration.
The diagnostic log retains resource errors. The
[RetroArch implementation](https://github.com/libretro/RetroArch/blob/master/runloop.c)
flushes legacy messages or replaces the dedicated widget message, while extended
standard notifications explicitly retain multiple queued messages.

Global Gain keys retain Auto/native levels, Mute and live numeric selection;
Master Volume keeps its existing behavior. U4 does not activate the optional
alternative mixer preset. Reset restores the actual loaded resources. Resource
identity already includes both PCM banks and the procedural flag: Save States
from another donor/fallback are rejected before mutation, even if the selected
preference text matches. Existing state format 4 and Save RAM remain unchanged.

## Verification Procedure

Reuse `tools/test_libretro_model1_scope.py` and its ABI host. The new focused
`tools/test_libretro_netmerc_audio.py` extracts PCM-only donor fixtures from
user-owned ZIPs into a fresh output directory and checks all four donors, Off,
state continuation, reset, content-reload semantics, live source mutes and atomic
rejection of incompatible states. Valid ZIP decoys in system/cwd verify that a
missing adjacent donor cannot silently load from either location. The NetMerc
content is copied rather than symlinked because the existing host canonicalizes
content paths. Original ROMs, personal configuration and saves are untouched.

`tools/test_retroarch_gpu.py --audio-donor vf|vr|swa|wingwar|off --notifications`
reuses the existing isolated frontend runner and records actual audio messages.
Use a content directory without donor ZIPs for the missing-donor case. The normal
Development deployment procedure then installs the verified release core/info
and compares SHA-256.

Tests and frontend delivery are separate from gameplay, physical controller and
listening acceptance. The original-audio warning branch is covered with a
synthetic unit fixture; the available native NetMerc dump selects procedural
fallback when no valid donor is supplied.

## Delivery Evidence — 2026-10-04

- 62 adapter tests pass, including modern/legacy donor registration and the
  original/procedural notice branches. The existing native donor/state test
  also passes, including nonzero sample continuation and sound-program preservation.
- The ABI resource runner passes all four PCM-only donors and Off, four invalid
  resource cases, system/cwd exclusion, atomic incompatible-state rejection,
  reset, same-state continuation, reload scope and live source mute checks.
- Nine existing sets retain identical video, audio, Save RAM and state
  continuation against the pre-U4 Development build over 120 frames per set.
- Offline locked macOS release build and Model 1 artifact/export/dependency
  gates pass. Two isolated 180-frame RetroArch Vulkan runs exit successfully:
  valid VF produces no audio notice; missing VF produces the expected fallback
  `SET_MESSAGE`. Final PNGs include frontend startup/asset-progress OSD;
  notification delivery is established by callback logs, not those screenshots.
- Authorized Development core/info installation passes with build/installed
  core SHA-256 `b666d569f153b55542bcc836bdd6c72e542a88d69192230e0c26118054f82bca`.

Local evidence: `/private/tmp/tgpulse-u4/` (`tests-delivery.log`, `native.log`,
`abi-delivery/result.json`, `regression-delivery.json`, `artifact-delivery.log`,
`retroarch-vf-delivery`, `retroarch-fallback-delivery`, `deployment.json`).
Earlier attempts remain local diagnostics; these delivery records use the final
artifact. No ROM/sample fixture is committed. No commit, push or release is part
of U4; the next implementation activity is U5.

Reasoning High; effort M; estimate 1–3 account percentage points. Shared account
use remains 33% at displayed precision (no rounded increase from the start).
Reset: 2026-10-10 09:49:17 CEST.

## Follow-Up: Visible Notification Verification

The first U4 delivery established `SET_MESSAGE` acceptance in logs, but this did
not prove visible coexistence with subsequent Renderer/NVRAM/Holder messages.
The deferred queued warning corrects that gap. The isolated frontend runner now
disables automatic asset extraction and uses the null audio driver, preventing
unrelated startup progress and audio-device errors from covering its OSD checks.

On 2026-10-04, `retroarch-off-final/frame.png` visibly shows `NetMerc Audio: Procedural
Fallback` with Donor Off, NVRAM Settings Enabled and an isolated copy of the user's
existing save. ABI coverage verifies that the warning is deferred until the
first delivered frame, sent once with warning priority, and retains legacy
compatibility. Successful donors remain silent. All four donor/fallback resource
cases and NetMerc's reviewed settings pass. Evidence is under
`/private/tmp/tgpulse-notices/`; this supersedes the initial notification-only
log acceptance as proof of visual delivery.

Final artifact SHA-256:
`e15d4c48144051280dced1b6941f59a8acd3d00026865b2500431b9efe89907b`.
`deployment-final.json` verifies its Development installation. Source/gameplay
publication and U5 remain outside this correction.
