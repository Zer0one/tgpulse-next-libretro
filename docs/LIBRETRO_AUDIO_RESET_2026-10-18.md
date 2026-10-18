# Audio Recovery After Reset

## Reference And Scope

Reviewed Framework F12/F14 (current edition 0.1.40), SM2's
`PORTING_PLAN.md` Audio Recovery follow-up and
`Docs/FRONTEND_MENU_AUDIO_RESET_2026-10-18.md`, plus Supermodel's
`Docs/ROADMAP.md` and `Docs/NETWORK_TWO_OPTION_2026-10-17.md` reset analysis.
SM2 supplies the PCM/reset and real-frontend recording procedure. Supermodel's
blocked asynchronous producer explains which invariants to inspect; its
queue-specific two-packet priming is not applicable to this adapter.

## Source Findings

Model 1 produces audio synchronously inside `retro_run`. `collect_audio`
drains the machine sample vector into interleaved PCM; 60 Hz mode additionally
uses `output_audio` and a fractional resampling remainder. There are no
independent producer threads, packet-ring read/write positions or wake-up
conditions in this adapter.

Explicit `retro_reset` and effective NVRAM-option reset both clear native
pending samples, frontend PCM, resampled output and the fractional remainder.
The native initial state is restored, or a COMM-fitted machine is rebuilt.
No artificial silent packets or production audio changes were required.
Batch delivery retries positive partial acceptance within the same frame;
a zero/invalid callback return ends that frame's delivery. It does not create
a retained asynchronous backlog. The checks below use full batch acceptance;
they do not qualify an indefinitely stalled frontend.

## ABI Checks

Added `tools/test_libretro_audio_reset.py`, reusing the existing `LCFHost`.
No personal settings, saves, sensors or controller state are read or modified.

- VR, VF, Star Wars Arcade and Sega NetMerc, each at native and 60 Hz timing.
- Startup followed by two explicit resets in all eight cases: nonzero PCM
  returned and both repeated-reset windows had identical sample counts and
  SHA-256 PCM hashes.
- VF Difficulty Normal → Hard exercised the option-triggered automatic reset
  at both timings; reset notification and nonzero subsequent PCM were checked.
- 30,000 machine frames total. VR/VF windows are 600 frames; SWA/NetMerc use
  1,800 to include their longer startup/attract sequence. Advertise Sound is
  explicitly On for SWA/NetMerc; their factory default Off is expected silence,
  not a reset failure. NetMerc uses its default VF donor from adjacent content.
- The existing linked Restart/Resume test passed: totals 1/2/3/1, COMM fitting
  and removal, retained lobby callbacks, native role before the first frame
  and staggered-join recovery. This is ABI transport evidence, not a new
  two-peer audio/gameplay qualification.

## Actual RetroArch Recording

RetroArch ARM Nightly 1.22.2 (`9be7ec92`), macOS Apple Silicon, VR,
frontend-owned Vulkan, 60 Hz, non-threaded video and CoreAudio3 with 64 ms
latency. This matches the user's selected renderer/timing/audio-driver path;
no physical-controller or sensor behavior was exercised.

Extended the existing isolated frontend runner with `--record-audio`. It
retains per-run options, saves, states and recording under a new output root.
During a paced 7,200-frame run, invoked the native **Command → Reset** twice
through the UI. Both resets appear in the core/frontend log; the process exited
normally with a valid final image. The recording contains approximately
120 seconds of stereo 16-bit PCM, with nonzero samples in every five-second
window, including after both resets. CoreAudio3 initialization is logged.

This RetroArch build emitted a nonstandard padded WAV header. The original
was retained; a standard WAV was written from the unchanged PCM payload for
review. This is a recording-container observation, not a core audio fix.
Neither callback activity nor the recording establishes subjective listening
acceptance. The agent did not physically listen; real speaker/controller
fidelity and two-peer playback remain user-run qualifications.

## Result And Evidence

No persistent audio-reset stall was reproduced. Technical verification is
complete without a production audio change. The separate user listening
qualification is not claimed.

Local evidence (ignored, not committed):
`outputs/audio-reset-20261018/abi.json`, `lobby/result.json`,
`network/`, `artifact.log`, and `retroarch/{result.json,run.log,audio.wav,
audio-standard.wav,pcm-analysis.json}`. The 93 adapter tests pass, as does the
native format / 25 ABI exports / dependencies / empty-lifecycle artifact gate.
Build and installed public core SHA-256 both equal
`cef4524b77c9f4d8a70a6e3abbc099759db6394cbd1399c4491c9fd277fdeba4`.
No rebuild was necessary: only verification tools and documentation changed
in this audio phase. Previously verified pending implementation changes are
included in the separately authorized source publication.
