# Model 1 Performance Investigation

## Reference And Scope

The current SM2-Emu Libretro `PORTING_PLAN.md` phases 1 and 5 separate a
frontend-neutral machine/software baseline from GPU frontend verification.
Its phase 3.8 also warns that the Timing/FPS overlay has its own drawing cost.
Supermodel `Docs/ROADMAP.md` has no corresponding performance phase to copy.
This investigation keeps the Model 1 tilemap change in `tgpulse-core`, shared
by standalone and Libretro; it does not change the Libretro ABI, frontend
settings, Model 2 rendering policy or the source projects' Git histories.

## Prior RetroStation Evidence

The Batocera addon investigation on 2026-10-05 used `vr.zip` with SHA-256
`a61fbe4f1b18bbba59b4d4b136a1d854b45522fb898b7c3893493e9a1a7698ac`.
GroovyMAME 0.288 reached 164.90% of native speed in an isolated, unthrottled
run. TGPulse 0.1.0.3 at 60 Hz took 87.555 seconds for 4,800 attract frames
with its timing panel off. Full LTO showed no useful gain. Instrumentation
assigned a median 8.86 ms per sampled frame to emulation and 7.54 ms to CPU
tilemap background and foreground composition. The original tilemap walker
read the same mask bit, tile entry and character row repeatedly for adjacent
pixels. The addon's README retains the run settings and the distinction
between its MAME headroom reference and matched TGPulse measurements.

## Local Change And Verification

`draw_layer` first skips a scanline when its mask fully hides that plane. It
then reads the mask once per eight-pixel group and a tile entry and character
row once per contiguous tile span. Scroll wrapping, category, transparency,
palette selection, layer order and output dimensions retain the existing
rules. The unchanged pixel walker remains as a test-only reference over
uniform/partial masks, both plane polarities, four scroll values, all four
layers, both categories and opaque/transparent passes.

The matched baseline is a clean local rebuild of source commit
`32800c8507c93a2882b1ec158e8682e9087751df`, SHA-256
`985b828fa971bb578ba109fc528e18918866eb8744e9f06ce7ed1215951c9d8e`.
The candidate changes only `crates/tgpulse-core/src/tilemap.rs`; its macOS
arm64 release core SHA-256 is
`fa60b0211ad8622d609a0d03578c01da3cd549a85b428ab4fcb4ffea4be71fed`.
Both were built with local Rust 1.97.1, the same locked dependencies and
`Makefile.libretro`. They use the same locally owned ROMs and direct Libretro ABI capture tool,
with a fresh isolated Save RAM area per run and the Software renderer.
Elapsed time includes load, 2,400 or 1,200 `retro_run` calls, capture and
unload; it is a throughput comparison, not a claim of game FPS.

Reproduce a direct-ABI run with a fresh output directory for each build. The
recipe is kept outside Git with the generated Save RAM and screenshots:

```toml
set = "vr"
[[actions]]
frames = 2400
[[actions]]
capture = "final"
```

```sh
python3 tools/libretro_nvram_capture.py \
  --core /path/to/core.dylib --rom /path/to/vr.zip \
  --recipe /private/tmp/model1-perf.toml \
  --output /private/tmp/model1-perf-unique-run
```

Read `elapsed_seconds` and both capture SHA-256 values from `manifest.json`.

| Content / Frames | Baseline | Candidate | Time Reduction | Final Image And Save RAM |
| --- | ---: | ---: | ---: | --- |
| Virtua Racing / 2,400, first pair | 13.629 s | 9.567 s | 29.8% | SHA-256 equal |
| Virtua Racing / 2,400, second pair | 13.610 s | 9.622 s | 29.3% | SHA-256 equal |
| Virtua Fighter / 1,200 | 6.926 s | 4.911 s | 29.1% | SHA-256 equal |
| Virtua Racing / 3,750, automated race input | 21.514 s | 14.800 s | 31.2% | All three captures equal |

The automated race recipe reuses the capture tool's existing button/analog
input contract. After 2,400 boot/attract frames it inserts a coin for 10 frames,
releases for 60, presses Start for 10, releases for 180, then captures the
selection screen at frame 2,660. It presses Start for another 10 frames,
holds `r2 = 32767` for 480 frames, captures at frame 3,150, then holds
`r2 = 32767, left_x = 3000` for 600 frames and captures at frame 3,750.
The last image shows active racing at 224 km/h. All three image and Save RAM
hashes match between builds. This is controlled software-path game input,
not a physical-controller or audible-audio acceptance trial.

The full native `tgpulse-core` and `tgpulse-libretro` library suites passed:
256 and 80 tests respectively, with one ROM-dependent core test ignored.
The standalone target passed `cargo check`. The native macOS artifact passed
the 25-export ABI, dependency, Model 1 scope and empty-lifecycle checks.
It is installed locally as the Development core with matching `.info` and
verified SHA-256. These checks establish build and bounded output behavior;
they do not establish gameplay or physical-controller acceptance.

In isolated macOS RetroArch Vulkan, the same 2,400-frame VR run took 37.193 s
with the previously installed 0.1.0.5 GitHub core and 36.898 s with the
candidate. Both produced identical
final PNGs. In the same RetroArch launcher with the core's Software renderer,
the times were 36.641 and 36.797 seconds, again with identical final PNGs.
These frontend runs show no material speed gain on the Apple M4. They have
VSync/audio/shaders/panels disabled, but did not successfully activate
unlimited fast-forward. They therefore do not prove unchanged maximum core
throughput. A later same-compiler Vulkan comparison explicitly fixes AV timing
to 60 Hz: baseline 36.883 seconds and candidate 36.833 seconds, with identical
final PNGs. The latter includes the external sampling described below, so the
small timing difference should not be interpreted as a throughput estimate.

A three-second `sample` capture of the candidate's own RetroArch process
found 1,133 of 2,244 main-thread samples in `__semwait_signal`, reached through
`CAMetalLayer nextDrawable`, MoltenVK swapchain-image acquisition and
RetroArch's `vulkan_frame`. This directly locates a presentation wait during
the sampled window; it does not show Model 1 GPU saturation or establish the
same behavior on Batocera. The sampled 2,400-frame run took 36.833 seconds.
The logs identify RetroArch 1.22.2 (`9be7ec92`), Apple M4 and core AV timing
60.00 Hz. Attempts to enable fast-forward through the documented command
interface produced no GET_STATUS response and were stopped by the runner's
bound. That experimental command-control code is not retained in the runner.
Only the existing AV-timing choice is added to the established runner, with
`native` preserving its prior default and the selected value in its report.

The earlier row-skip-only candidate took 36.831 s in the original macOS run;
its Batocera attract result was 78.090 versus 87.555 seconds for 4,800 frames,
with matching screenshots.

## Linux x86_64 Build And Mac-Hosted Check

Both clean baseline and tilemap-only candidate were built using Rust 1.97.1,
the same locked dependencies and default Thin LTO `Makefile.libretro` recipe
in the existing `tgpulse-linux-rust-builder:bookworm` amd64 container. The
user authorized fetching the missing Cargo sources into that isolated build
volume. No host or RetroStation dependencies were installed.

The Linux baseline SHA-256 is
`af6998a8e6afee42512983ca93ff3d373168e6a6d2333da98a3334f858ab854d`;
the candidate is
`8121e6cac978e507848729dfa518d7a11a5a0e18c19d1a532aa421693d55d59c`.
The candidate passes ELF x86_64 format, 25 Libretro exports, dependency,
Model 1 scope and three empty-lifecycle checks in the container. Dependencies
are libc, libm and libgcc_s; no SDL, libstdc++ or linked Vulkan requirement.

The same 2,400-frame direct-ABI Software VR recipe under Rosetta on the Mac
took 27.262 seconds for baseline and 15.241 seconds for candidate: 44.1% less
time. Both image and Save RAM hashes match each other and the native ARM
captures. These are translated x86_64 results on Apple M4, not native
Ryzen/RetroStation FPS. They demonstrate that the improvement also exists
in the x86_64 artifact, without requiring Windows or Batocera to be running.

## Native RetroStation Verification

The user brought Batocera online on 2026-10-05. `batocera.local` still failed
name resolution, but the previously verified SSH name `batocera` connected.
The host reports Batocera 43.1, AMD Ryzen 5 3400G and Radeon Vega 11 graphics.
The existing remote-core skill stages and runs the exact two Linux builds
below `/userdata/system/codex-core-tests/`, preserving installed artifacts.
The VR ROM hash matches the local and prior addon measurements.

The first bundle stopped during inventory because `/etc/batocera-release`
does not exist on this device; the corrected bundle reads `/etc/os-release`.
Batocera lacks `readelf` and `nm`, so it reuses the artifact checker's ctypes
runtime checks with a Python ELF-header check, rather than installing tools.
Full export/format verification already passed on the exact SHA-256 builds
in the builder. Both native cores pass loading, required ABI-symbol lookup,
three empty lifecycle cycles and `ldd` dependency resolution.

The fixed-frame comparison disables audio, VSync, shaders and timing panels.
Both hardware runs explicitly deliver 60.00 Hz AV timing to RetroArch.
Each run has separate fresh configuration, Save RAM, states and output paths.
The figures include initialization and capture; frame count divided by wall
time is a whole-run average, not a frame-pacing or minimum-FPS measurement.

| VR Path / Frames | Baseline | Candidate | Time Reduction | Whole-Run Frames/s, Before / After |
| --- | ---: | ---: | ---: | ---: |
| RetroArch OpenGL / 2,400 | 42.888 s | 29.166 s | 32.0% | 55.96 / 82.29 |
| RetroArch Vulkan / 2,400 | 42.948 s | 29.071 s | 32.3% | 55.88 / 82.56 |
| Direct ABI Software / 2,400 | 40.001 s | 27.000 s | 32.5% | 60.00 / 88.89 |
| Direct ABI Software With Race Input / 3,750 | 63.946 s | 42.112 s | 34.1% | 58.64 / 89.05 |

Both hardware comparisons have equal final PNG SHA-256. Every direct-ABI
image and Save RAM capture matches its baseline and the native ARM captures,
including the three selection/race/driving images. Hashes of the installed
addon core and `/userdata/system/configs/retroarch/retroarchcustom.cfg` match
before and after the campaign. The test cores remain staged artifacts; this
does not update the installed Batocera addon.

Reproducible first-campaign evidence is retained in the marked remote root
`tgpulse-model1-grouped-b-20261005` and the Mac results directory
`/private/tmp/tgpulse-m1-perf-batocera-b-results`. Its `comparison.json`, per-run
reports/configuration/logs, captures, core hashes and installed-artifact
checks identify the exact comparison. Windows is not needed for this device
result because it shares the RetroStation hardware with Batocera.

The native measurements confirm that the tilemap correction removes a large
CPU cost and creates throughput above 60 frames/s in these VR conditions.
They do not establish equality with MAME's differently timed 164.90% result,
shader performance, audible output or physical-controller acceptance.

### Frontend Race And Audio Follow-Up

The second isolated campaign runs the candidate before the baseline, using
the same established GPU runner and the replay-v1 generator from
`tools/test_retroarch_gameplay.py`. It tests the Software renderer inside
RetroArch and a 4,800-frame OpenGL replay with coin, Start, view/gear changes,
accelerator and steering input. The replay SHA-256 is
`4c1aeb0025020482a7a295347085f40d086f65c2a62fdb28892c801432628db1`.

| VR Frontend Path / Frames | Baseline | Candidate | Time Reduction | Whole-Run Frames/s, Before / After |
| --- | ---: | ---: | ---: | ---: |
| Software With GLCore Presentation / 2,400 | 42.241 s | 28.623 s | 32.2% | 56.82 / 83.85 |
| OpenGL With Race Replay / 4,800 | 86.931 s | 57.317 s | 34.1% | 55.22 / 83.74 |

Both pairs finish with equal final PNG hashes and no replay errors. The final
race image shows the running game at 200 km/h. The longer race result confirms
the gain outside the first short attract segment: about 51.7% more whole-run
throughput for this replay, with output preserved.

The candidate then passes the existing gameplay/audio runner on native
Batocera with OpenGL/GLCore, PulseAudio, udev input and native AV timing.
The runner gains only platform/renderer/timing arguments and report fields;
its prior macOS defaults, replay generator and PCM checks remain in place.
No new replay format, dependencies or installed frontend settings are added.

The audio-enabled 4,800-frame run takes 81.088 seconds, exits normally, reports
no audio-driver or replay error and produces a 14,900,624-byte PCM recording:
3,725,142 stereo frames at the recorded integer rate of 44,642 Hz, with
7,353,148 nonzero samples. Its final image SHA-256
`42360faa061f20caed26f7d58bf13ac43fbad72be9002e72b0879d736ac9b3fd`
matches both the baseline and candidate audio-disabled race replay.
This verifies generated audio and a synchronous frontend audio path, not
physical speaker audibility or a minimum-FPS measurement.

The core reports native 57.52 Hz and 44,642.86 samples/s. RetroArch logs an
adjusted audio input rate of 46,564.28 samples/s; its ratio to the core rate
is consistent with synchronization toward a 60 Hz display. Therefore the
81.088-second wall time is not evidence of an exact 57.52 Hz host cadence.
It does show that the candidate handles this normal audio-enabled replay
within the frontend's pacing, with the same emulated-frame output.

The second campaign exits with status 0 and again preserves installed core
and configuration hashes. Evidence is retained below the marked remote root
`tgpulse-model1-grouped-gameplay-20261005` and the Mac directory
`/private/tmp/tgpulse-m1-perf-batocera-gameplay-results`. Publication and addon
deployment remain separate from these verified staged-core tests. Broader
games, shaders and user-operated controller/audio trials remain distinct
from the scoped tilemap implementation and this VR performance result.

### Temporary Addon Binary Deployment

On 2026-10-05, after the isolated tests above, the user explicitly requested
replacing the installed Batocera addon binary for a personal trial. The
verified `payload/grouped.so` candidate was installed atomically at
`/userdata/system/addons/tgpulse-next-libretro/libretro/tgpulse_next_m1_libretro.so`.
No RetroArch process was running at replacement. File ownership and mode
were retained, and both the installed file and its existing
`/usr/lib/libretro/tgpulse_next_m1_libretro.so` link resolve to SHA-256
`8121e6cac978e507848729dfa518d7a11a5a0e18c19d1a532aa421693d55d59c`.

The original binary was backed up before replacement under
`/userdata/system/codex-core-tests/tgpulse-model1-grouped-gameplay-20261005/original-addon-core/tgpulse_next_m1_libretro.so`,
with SHA-256
`a452bf4946948441f96fd35473ceec45d17b27f321256ce552053ae41f7b38e4`.
That directory also retains `original.sha256` and `deployment.txt`.
For restoration, with RetroArch stopped, copy the backup with preserved
attributes to a temporary file beside the installed binary, verify its
original SHA-256, then rename it over the installed binary.

This is a temporary binary replacement, not a canonical addon update.
The candidate reports port version 0.1.0.5; addon package metadata and the
existing `.info` remain at their installed revision. The user will handle
the subsequent canonical update. The next normal addon launch uses this
optimized binary; personal gameplay acceptance remains a separate trial.

## CPU Scheduling Follow-Up

### Reference And Profile

The user requested further structural optimization on 2026-10-05, without
pursuing small FPS differences. The corresponding reference remains SM2
`PORTING_PLAN.md` phase 1: preserve machine execution, audio and state against
a matched baseline, including standalone compilation. Phase 5 separates
machine throughput from frontend presentation. Those sections and Supermodel
`Docs/ROADMAP.md` were rechecked before recording P2 in the sole port roadmap.

The local MAME checkout at commit
`6c504efbee8342f4cbf813d066176523a720d7d5` provides two useful comparisons:

- `src/devices/cpu/mb86233/mb86233.cpp:726` executes against persistent device
  registers; its instruction loop does not reconstruct CPU objects.
- `src/mame/sega/segaic24.cpp` obtains cached tile pixmaps/flags and marks tiles
  and character graphics dirty at RAM writes (`tile_w` and `char_w`). Its
  compositor has uniform 128-pixel mask fast paths. This suggests a potential
  tile-cache design, but no such cache is introduced in this change.

A bounded three-second `/usr/bin/sample` capture of the tilemap-optimized
macOS direct-ABI Software core collected 2,518 main-thread samples. Of these,
1,628 were within machine execution; 508 were in memory copy/clear routines
immediately below `Model1System::run_slice`. This is sample occupancy on this
host and scene, not a platform-independent time budget. The raw profile is
`/private/tmp/tgpulse-m1-next-profile.sample.txt`.

Inspection explains that memory work: every 64-clock quantum moved the V60
and MB86233 structures out of the machine, constructed reset placeholders,
then copied the CPUs back. These structures contain large diagnostic arrays.
A normal frame has 4,346 quanta, so this bookkeeping was repeated thousands
of times per frame despite not advancing emulated hardware.

### Change And Verification

`Model1System::run_slice` now moves both executing CPUs out once per call,
passes them by mutable reference to the unchanged 64-clock scheduling loop,
and restores them after the loop, including every returned board fault.
The IRQ synchronization helper accepts the executing V60 explicitly.
FIFO handshakes, clock ratios, instruction debt, timers, I/O/audio ordering,
diagnostic counters, finite arithmetic policy and Save State format remain
unchanged. Existing bus callbacks read the placeholder CPUs only for trace
messages. The implementation uses safe Rust and adds no machine fields,
frontend options or new dependencies.

The matched baseline is the previous tilemap-optimized macOS core, SHA-256
`fa60b0211ad8622d609a0d03578c01da3cd549a85b428ab4fcb4ffea4be71fed`,
retained at `/private/tmp/tgpulse-m1-next-baseline.dylib`. The new core uses
the same Rust 1.97.1, locked dependencies and `Makefile.libretro` release
recipe; SHA-256 is
`35d28ebf6e45a1a99c06e61a65f6b34959d9f1512ab3617f575b086fffcbf573`.

The established 3,750-frame VR input recipe above takes 14.871 seconds with
the baseline and 10.907 seconds with the candidate: **26.7% less elapsed
time**, with all three image and Save RAM capture hashes equal. Runs were
sequential, candidate first, in new isolated output directories:
`/private/tmp/tgpulse-m1-next-drive-candidate` and
`/private/tmp/tgpulse-m1-next-drive-baseline`. These direct-ABI measurements
include load/capture/unload and do not establish frontend FPS on RetroStation.

Exploratory 2,400-frame VR runs also show candidate times near 7.1 seconds,
but one later baseline run takes an anomalous 54.421 seconds. Its cause is
not established; exclude that pair from speedup estimates. The race pair
above and unchanged execution/state output are the reported evidence.

Verification reuses the existing procedures:

- `cargo test --offline --locked -p tgpulse-core -p tgpulse-libretro --lib`:
  257 core and 80 adapter tests pass; one ROM-dependent test remains ignored.
  A focused regression forces an I/O board fault after V60 execution and
  checks that completed V60 work, diagnostic counters and the not-yet-run
  TGP state survive both the error and a repeated call.
- `tools/test_libretro_model1_scope.py --frames 600 --state-policy identical`:
  all nine starting Model 1 sets have equal every-frame video/PCM hashes,
  Save RAM and full snapshots, including cross-build state restore and
  subsequent continuation. Report: `/private/tmp/tgpulse-m1-next-scope.json`.
- NetMerc extends the same ABI hosts (`AudioHost`/`Host`) for 600 frames in
  each City Workaround mode. Video, PCM, Save RAM, full state and ten-frame
  cross-build state continuation match. Report:
  `/private/tmp/tgpulse-m1-next-netmerc/comparison.json`.
- Standalone `cargo check --offline --locked -p tgpulse` and native artifact
  checks pass: arm64 format, Model 1 scope, 25 ABI exports, resolved
  dependencies and three empty lifecycle cycles.
- The macOS Development core and matching Development `.info` are installed
  through `tools/install_dev_core.py`; installed hashes match the candidate
  above and metadata SHA-256
  `7f5be1952c4b61b0f7f35b50af5416d5014b0ffcc4f57c5422057e257867a3bb`.

### Native Verification And Addon Deployment

The user subsequently authorized updating the Batocera addon after every
successful optimization. The existing isolated Linux builder reproduced
`Makefile.libretro` with the same locked dependencies, Thin LTO and baseline
x86_64 target. The P2 ELF core passes the 25-export, dependency and empty
lifecycle gates; SHA-256 is
`add99311f78c1645cb9bfd2da8e8a23c9de6c5f4cdec25c36c09cbca2a831dd6`.
Native Batocera load/lifecycle checks pass on that exact file.

| Native Batocera VR Path | P1 Baseline | P2 Candidate | Time Reduction | Output |
| --- | ---: | ---: | ---: | --- |
| Direct ABI Software Race / 3,750 Frames | 41.914 s | 30.221 s | 27.9% | Three Image/Save RAM Captures Equal |
| OpenGL/GLCore / 2,400 Frames | 28.965 s | 21.703 s | 25.1% | Final PNG Equal |

The isolated campaign exits with status 0 and preserves the installed core
and RetroArch configuration during the comparisons. Its remote root is
`/userdata/system/codex-core-tests/tgpulse-model1-cpu-slices-20261005`;
results are fetched to `/private/tmp/tgpulse-m1-cpu-batocera-results`.

After verification, the P2 candidate was installed atomically at the normal
addon core path. The installed file and existing `/usr/lib/libretro` link
match the candidate SHA-256. The P1 binary is retained at
`tgpulse-model1-cpu-slices-20261005/original-addon-core/tgpulse_next_m1_libretro.so`
below the remote test parent, with its checksum and deployment receipt.
RetroArch was stopped at replacement. Package metadata is unchanged; this
continues the explicitly requested temporary addon trial. No commit,
upstream modification or publication was made. Windows was not tested.

## Decoded Tile Cache Follow-Up

### Reference And Adaptation

P3 follows the current SM2 phase 1 output/state comparison and phase 5
software/GPU preservation gates. Supermodel has no matching cache phase.
The MAME reference is the System 24 tile/character dirty handling and cached
pixmaps in `src/mame/sega/segaic24.cpp`, at the local revision recorded above.
No MAME code is imported.

`Model1TileCache` in the shared `tilemap.rs` stores eight decoded palette
indices per tile row. The standalone Session and Libretro Game own this
derived renderer data; no cache fields enter the emulated machine, Save RAM
or Save State. Both software and GPU paths use the same cached compositor.
Model 2 keeps its existing uncached path.

RAM snapshots identify changed name-table entries and character data.
Only changed tiles or tiles referring to changed characters are decoded.
A whole-RAM comparison avoids scanning individual characters when only the
name tables change. Comparing actual RAM covers CPU writes, debugger edits,
reset, restored states and changes between composition passes without
coupling invalidation to one bus handler. The palette is resolved at each
pass, so palette/gamma/intensity updates do not leave stale cached colours.
Scroll, window masks, category and transparency retain the same draw logic.
The existing split-plane compositor remains available with the same output.
Cache allocation is lazy and adds about **2.6 MiB** per loaded Model 1 renderer.

### Local Verification

The P2 macOS baseline SHA-256 is
`35d28ebf6e45a1a99c06e61a65f6b34959d9f1512ab3617f575b086fffcbf573`.
The final P3 candidate uses the same compiler, locked dependencies and release
recipe, SHA-256
`54663d4b29a97355aad96966cd19d611429dee464e59fe1f060ad7a7ee84283a`.
The established 3,750-frame VR input recipe takes **10.931 to 10.048 seconds**,
an 8.1% reduction, with three equal image/Save RAM captures. Candidate runs
first, then baseline; reports are retained in
`/private/tmp/tgpulse-m1-cache-final-drive-candidate` and
`/private/tmp/tgpulse-m1-cache-final-drive-baseline`.

The 258 core and 80 adapter tests pass; one ROM-dependent test is ignored.
The new pixel reference test compares cold and warm caches against the
uncached compositor with patterned characters/names, four masks, four scroll
values, both categories, palette changes, layer disable and split mode. It
then forces a visible character-only edit and verifies that the pixel changes,
and checks output after machine state restore. Standalone compilation passes.
Native arm64 artifact/scope/25-export/dependency/empty-lifecycle checks pass.

The existing nine-set ABI host verifies every-frame video and PCM, full state,
Save RAM and cross-build state continuation for 600 frames per set. NetMerc
matches for 600 frames and restored continuation in each City Workaround mode.
Final reports:

- `/private/tmp/tgpulse-m1-cache-final-scope.json`
- `/private/tmp/tgpulse-m1-cache-netmerc/comparison.json`

The final macOS Development core and matching `.info` are installed through
the existing installer, with the candidate hash above and the same verified
Development metadata hash recorded for P2.

### Native Batocera Verification And Deployment

The isolated Linux build uses the same recipe and passes ELF x86_64,
25-export, dependency and empty-lifecycle gates. Its SHA-256 is
`962268f318b4192a11f3332383a3abb7ad83016dadf52839ab562ad03838df54`.
The matched P2 Linux baseline is `add99311…31dd6`, recorded in full above.
Native load/lifecycle and dependency checks pass on both exact binaries.

| Native Batocera VR Path | P2 Baseline | P3 Candidate | Time Reduction | Output |
| --- | ---: | ---: | ---: | --- |
| Direct ABI Software Race / 3,750 Frames | 30.991 s | 29.976 s | 3.3% | Three Image/Save RAM Captures Equal |
| OpenGL/GLCore / 2,400 Frames | 21.656 s | 21.110 s | 2.5% | Final PNG Equal |
| Vulkan / 2,400 Frames | 21.705 s | 21.060 s | 3.0% | Final PNG Equal |

These are sequential candidate-first comparisons, one pair per native path;
the small differences are observed wall times, not a statistical minimum-FPS
claim. The cache gain is smaller on RetroStation than on the Mac. The
implementation removes repeated decoding while preserving measured output;
the larger P2 gain remains separately established above.

The campaign exits with status 0 and preserves installed core/configuration
hashes during comparison. Remote evidence remains under
`/userdata/system/codex-core-tests/tgpulse-model1-tile-cache-20261005`, fetched
to `/private/tmp/tgpulse-m1-cache-batocera-results`.

After verification, the user-authorized P3 candidate replaces the addon core
atomically. The addon file and existing `/usr/lib/libretro` link match its
SHA-256. The previous P2 core is backed up at
`/userdata/system/codex-core-tests/tgpulse-model1-tile-cache-20261005/original-addon-core/tgpulse_next_m1_libretro.so`;
checksum and deployment receipts are retained beside it. The prior P1 and
official-addon backups remain in their earlier test roots. RetroArch was
stopped at replacement. Configuration, saves, package metadata and `.info`
remain unchanged. The next normal launch uses P1 + P2 + P3. Canonical addon
publication and user-operated gameplay/controller/audio acceptance remain
separate from this temporary binary trial.

## UART Clock Batching Follow-Up

### Reference And Change

P4 applies the current SM2 phase 1 machine/audio/state equality gate and
phase 5 software/GPU frontend gate. Supermodel has no corresponding UART
performance phase. This frontend-neutral change stays in the shared Model 1
sound code and does not change Libretro settings or the standalone build.

A fresh three-second macOS `sample` capture of the P3 core during direct-ABI
VR execution found 1321 samples inside `Model1System::run_frame`; one
606-sample `run_slice` branch includes 387 inside `SoundSystem::run`, with
141 in `render_cycles` outside child calls. This identified repeated 10 MHz
per-clock UART iteration as a
bounded target. The display-list clone did not appear as a leading cost.
The profile is a sampled window, not a complete attribution of all wall time.

The existing sound loop already divides work at the next 20-clock i8251 edge
and renders the preceding audio interval. P4 advances the serial phase by that
known interval and calls the existing single-clock edge operation only when
the interval ends at the edge. Pin sampling, endpoint order, emulated clock
phase, audio rendering order and serialized state stay unchanged. A focused
test compares the batched and prior single-clock progression across partial,
exact-edge and multi-edge intervals.

### Local And Native Evidence

The matched P3 macOS baseline core is
`54663d4b29a97355aad96966cd19d611429dee464e59fe1f060ad7a7ee84283a`;
the P4 core is
`ed6b9fe41508149a51b6448469fa7da3a32c30e803c52026eba82a1a2d3bd48e`.
The established 3,750-frame VR drive recipe takes **9.938 versus 9.464
seconds**, 4.8% less with P4. The three screenshot and Save RAM hashes match.
Candidate ran first, then baseline, once each. Reports are at
`/private/tmp/tgpulse-p4-mac-candidate` and
`/private/tmp/tgpulse-p4-mac-baseline`.

All 259 core and 80 adapter tests pass; one ROM-dependent core test remains
ignored. The existing nine-set ABI host compares every-frame video and PCM,
Save RAM, full state and restored continuation for 600 frames per set against
P3. NetMerc matches for 600 frames and continuation in both City Workaround
modes. Standalone compilation and the native macOS artifact/25-export/
dependency/lifecycle gates pass. The P4 Development core and matching `.info`
are installed locally with verified hashes. Reports are at
`/private/tmp/tgpulse-p4-scope.json` and
`/private/tmp/tgpulse-p4-netmerc/comparison.json`.

The isolated x86_64 Linux candidate passes ELF, dependency, 25-export and
empty-lifecycle checks. Its SHA-256 is
`8ddf913d9ea43122277338ed2d4da48dd3012283a068af7b2c91bc35ab38618a`;
the P3 Linux baseline is
`962268f318b4192a11f3332383a3abb7ad83016dadf52839ab562ad03838df54`.
The native Batocera comparison uses the same P3/P4 pair, VR content, bounded
input recipe and isolated RetroArch configurations as P3:

| Native Batocera VR Path | P3 Baseline | P4 Candidate | Time Reduction | Output |
| --- | ---: | ---: | ---: | --- |
| Direct ABI Software Race / 3,750 Frames | 29.482 s | 28.718 s | 2.6% | Three Image/Save RAM Captures Equal |
| OpenGL/GLCore / 2,400 Frames | 20.195 s | 19.651 s | 2.7% | Final PNG Equal |
| Vulkan / 2,400 Frames | 22.054 s | 20.454 s | 7.3% | Final PNG Equal |

These are single candidate-first pairs, so the especially large Vulkan
difference needs repetition before attributing its full size to P4. The
isolated test exits successfully, confirms both native cores load, and
preserves the installed addon core and global configuration hashes during
comparison. Evidence is retained under
`/userdata/system/codex-core-tests/tgpulse-model1-uart-edge-20261005`
and `/private/tmp/tgpulse-p4-batocera-results`.

After successful comparison, the user-authorized P4 core replaces only the
addon binary atomically. The installed file and active symlink both hash to
`8ddf913d9ea43122277338ed2d4da48dd3012283a068af7b2c91bc35ab38618a`.
The previous P3 binary is backed up with hash and deployment receipt at
`/userdata/system/codex-core-tests/tgpulse-model1-uart-edge-20261005/original-addon-core/`.
RetroArch was stopped at replacement. Canonical addon publication and
user-operated gameplay/controller/audio acceptance remain separate.

## Post-P4 Optimization Audit — 2026-10-05

### Method And Limits

This is a read-only candidate audit, not an implementation milestone. The
audited source is local port commit `32800c8507c93a2882b1ec158e8682e9087751df`
plus the uncommitted P1–P4 changes above; the macOS arm64 core SHA-256 is
`ed6b9fe41508149a51b6448469fa7da3a32c30e803c52026eba82a1a2d3bd48e`.
The local MAME source is clean at
`6c504efbee8342f4cbf813d066176523a720d7d5`. Its corresponding upstream
source files are [Model 1 video](https://github.com/mamedev/mame/blob/master/src/mame/sega/model1_v.cpp),
[System 24 tiles](https://github.com/mamedev/mame/blob/master/src/mame/sega/segaic24.cpp),
and [V60 CPU](https://github.com/mamedev/mame/blob/master/src/devices/cpu/v60/v60.cpp).
MAME is a design reference; no source is copied and its percent-of-native
benchmark is not a matched Libretro wall-time comparison.

The existing direct-ABI Software capture driver ran 24,000 VR, 16,000 VF and
16,000 SWA frames on the Mac. One five-second `sample` window per run captured
the main thread while emulation was active. The windows need not represent
whole games or other renderers. Results and raw profiles remain outside Git
under `/private/tmp/tgpulse-audit-{p4,vf,swa}*`. The samples localize work;
they do not measure a proposed change. Batocera has no preinstalled `perf`,
`gprof` or `strace` on this host, so these are Mac Software estimates rather
than native RetroStation profiles. Percentages below are inclusive main-thread
sample shares and overlapping subtrees must not be added.

| Sampled path | VR | VF | SWA |
| --- | ---: | ---: | ---: |
| Machine frame, including sound | 55.6% | 60.3% | 68.5% |
| Software 3D below HUD | 20.3% | 17.6% | 13.1% |
| 2D background and foreground | 21.2% | 19.6% | 15.0% |
| Sound within machine frame | 20.9% | 21.5% | 34.6% |
| YM3438 synthesis within sound | 4.5% | 5.1% | 5.2% |
| Tile cache refresh within 2D | 1.2% | 0.9% | 0.7% |

An estimated gain below means a possible reduction in whole Software-frame
CPU time for these workloads, relative to P4, not measured Batocera FPS.
Confidence describes the **gain estimate**, not certainty that the source
difference exists: 4/5 has a repeatedly sampled redundant operation and a
small, clear replacement; 3/5 has a measured hot path and explicit mechanism;
2/5 lacks an A/B prototype or has large game dependence; 1/5 also changes
timing or scheduling and needs trace proof. Ranges are independent and must
not be summed. The lighting candidate was subsequently implemented as P5 below;
its row retains the original pre-change estimate for comparison.

Structural impact grades the expected code surface and changes to data or
timing ownership, independently of gain confidence. Very low changes one
renderer calculation without adding state; low–medium stays inside 3D data
handling; medium adds derived audio state and invalidation; medium–high changes
tile composition; high changes sound scheduling.

| Candidate | Source evidence and reference | Estimated whole-frame gain | Confidence | Structural impact |
| --- | --- | ---: | ---: | --- |
| Select polygon lighting without copying `View` | `model1_video.rs` clones `View` inside `push_object` only to place `lightparams[light_mode]` at index 0 for `object_color`. The compiled copy is 0x1088 bytes (4,232); its `memmove` branch consumes 141/4,236 VR, 161/4,224 VF and 146/4,220 SWA samples. MAME's `model1_v.cpp` indexes the selected lighting parameter directly. Pass that parameter to colour calculation while preserving numeric order. | **2.5–4%** | **4/5** | **Very low** — local lighting argument; no new state or ABI |
| Compose 2D from cached scanline/pixmap spans | Current `draw_plane_region` reads a tile entry and pen for each screen pixel in split mode; `draw_layer` still resolves spans at render time. MAME's `segaic24.cpp` draws from cached tile pixmaps, with full/hidden 128-pixel mask branches and dirty writes. P3 already caches decoded pen rows, so this is a further output/palette-safe row-composition change, not a second decode cache. The 2D share is 15–21%; split-mode alone takes about 0–12% depending on game. | **3–9%** across the sampled games; near zero for a scene without applicable visible 2D rows | **2/5** | **Medium–high** — tile cache and compositor behavior |
| Reduce sound instruction-grid overhead | `SoundSystem::run` calls `render_cycles` after every 68000 instruction, retaining register write order. MAME uses device/stream timing rather than this specific per-instruction loop. The whole sound path takes 21% in VR/VF and 35% in SWA, but most is required emulation. Batch only intervals proven free of sound-register/UART events. | **2–6%**, potentially more in SWA; speculative | **1/5** | **High** — CPU-to-sound timing and event boundaries |
| Cache derived YM3438 operator parameters | `Synthesis::sample` rebuilds 24 `Params` from registers for every output sample. They can be derived on relevant register changes and reconstructed after restore, leaving serialized state portable. The entire synthesis branch is only 4.5–5.2% of samples, bounding the likely whole-frame benefit. | **1–3%** | **3/5** | **Medium** — derived cache, write invalidation and restore handling |
| Reduce 3D per-polygon allocation and large-value sorting | `fill_quad` creates a small `Vec` of distinct points; `draw_queued` sorts `Quad` values. MAME uses a fixed polygon store and sorts pointers. Both changes require preserving stable equal-depth order, clipping and wireframe pixels. The entire Software 3D path is 13–20%, and the lighting-copy proposal overlaps it. | **1–4%** after the lighting copy, Software only | **2/5** | **Low–medium** — 3D queue and polygon representation only |

The V60 interpreter and MB86233 remain significant mandatory CPU work. MAME
uses an opcode table for V60 and an instruction loop for MB86233, while this
port uses Rust dispatch and explicit bus callbacks. The sample does not isolate
a comparable redundant operation in either interpreter; no credible gain
number is assigned to an ISA or scheduler rewrite. Changing the fixed
64-cycle cooperation quantum could alter FIFO, IRQ and sound ordering and
needs a separate trace-led experiment.

The MAME-like dirty-write approach for the existing decoded tile cache is
low priority for speed: `Model1TileCache::refresh` uses under 1.2% of samples
in all three windows, so even eliminating it completely cannot save more than
that fraction there. Full LTO was previously measured without useful gain.
The adapter/host portion of these direct-ABI windows is small compared with
the machine and renderer; they do not profile a real frontend. None of these
findings justifies a claim of a remaining MAME-size throughput gap, because
the original MAME and Libretro
measurements use different timing and frontend conditions.

## P5. Direct Polygon Light Parameter Selection

### Reference And Change

The current SM2 phases 1 and 5 call for equal software machine output and
preserved GPU frontend paths; Supermodel has no matching Model 1 lighting
phase. In local MAME `model1_v.cpp`, polygon shading reads the selected light
parameter directly. The existing Model 1 `push_object` instead cloned the
4,232-byte `View` for every lit polygon, replaced element zero of its light
table and passed that copy to `object_color`. P5 passes the selected
`LightParam` by value, retaining the original element-zero fallback for an
out-of-range index. Colour arithmetic, clipping, ordering, output formats,
frontend settings, machine state and standalone ownership are unchanged.
The implementation is limited to `crates/tgpulse-core/src/model1_video.rs`.

### Local Verification

The matched P4 macOS arm64 baseline is SHA-256
`ed6b9fe41508149a51b6448469fa7da3a32c30e803c52026eba82a1a2d3bd48e`;
P5 is `c6617c9862438f6eab1557abcdbfe471cb5aabddb280acf60a2e004242d04f01`.
Both use the same locked release recipe and VR ROM. The existing 3,750-frame
Software race recipe takes **9.473 seconds with P4 and 9.101 seconds with
P5**, a 3.9% reduction in this candidate-first single pair. Selection, race
and driving image and Save RAM hashes all match. The nine-set ABI comparison
passes every-frame video and PCM, full state, Save RAM and restored continuation
for 600 frames per set. NetMerc also matches video, PCM, state, Save RAM and
continuation for 600 frames in both City Workaround modes.

All 259 core and 80 adapter tests pass; one ROM-dependent core test remains
ignored. The standalone target compiles. The native macOS artifact passes
format, 25-export, dependency, Model 1 scope and three empty-lifecycle gates.
P5 and its matching Development `.info` are installed in local RetroArch with
the build hash verified. The global `cargo fmt --all --check` encounters
pre-existing formatting differences outside this change; `git diff --check`
passes. Local reports are under `/private/tmp/tgpulse-p5-mac-{candidate,baseline}-drive`,
`/private/tmp/tgpulse-p5-scope.json` and `/private/tmp/tgpulse-p5-netmerc`.

### Native Batocera Verification And Trial Binary

The existing amd64 Rust builder produced P5 with the same Linux x86_64 recipe
as P4. Its SHA-256 is
`0da7ccbe3b7ef9c39c9deea2f568c399da0fb59335ccf0ff538b6d0f07f23627`;
the matched P4 Linux baseline is
`8ddf913d9ea43122277338ed2d4da48dd3012283a068af7b2c91bc35ab38618a`.
P5 passes ELF x86_64, dependency, 25-export and empty-lifecycle checks.
Batocera 43.1 loaded both staged cores in isolated directories with the same
VR content, race recipe and 60 Hz frontend setting:

| Native Batocera VR Path | P4 Baseline | P5 Candidate | Time Reduction | Output |
| --- | ---: | ---: | ---: | --- |
| Direct ABI Software Race / 3,750 Frames | 28.825 s | 28.276 s | 1.9% | Three Image/Save RAM Captures Equal |
| OpenGL/GLCore / 2,400 Frames | 19.749 s | 19.348 s | 2.0% | Final PNG Equal |
| Vulkan / 2,400 Frames | 20.503 s | 20.050 s | 2.2% | Final PNG Equal |

The isolated run exited successfully and retained the installed addon core
and global RetroArch configuration hashes. These are single candidate-first
pairs, include startup/capture time and do not establish minimum game FPS or
audible/physical-controller acceptance. Evidence is at
`/userdata/system/codex-core-tests/tgpulse-model1-light-direct-20261005`
and `/private/tmp/tgpulse-p5-batocera-results`.

After the successful comparison, the user-authorized P5 trial core replaced
only the addon binary atomically. The installed file and active
`/usr/lib/libretro` symlink hash to the P5 Linux artifact above. RetroArch
was stopped; P4 was backed up, hash-checked and retained below the same test
root in `original-addon-core/`, with a deployment receipt. Package metadata,
`.info`, configuration and saves were not changed. Canonical addon publication
and user-operated gameplay remain separate.

## Retention Campaign: P3 Reassessment And C1–C4 — 2026-10-05

### Method And Decision Rule

The user required at least **5% reduction in repeated native Batocera elapsed
time** for medium, medium–high and high structural changes, with a near-threshold
result kept when uncertain. Low and low–medium changes have no numeric
retention threshold. Each pair used the same ROM, direct-ABI capture driver,
3,750-frame input recipe, 60 Hz setting and source/build recipe. The two-pair
runner used candidate–baseline–baseline–candidate order unless noted, checked
three image/Save RAM captures for equality and verified that the installed
addon and global RetroArch config hashes had not changed during testing.
Star Wars Arcade reused the same bounded button/analog sequence with only the
recipe `set` changed to `swa`; capture names are labels, not a claim that the
game reached the same scene as Virtua Racing. These are elapsed runs including
startup and capture, not minimum gameplay FPS or physical-controller proof.

| Change Versus Named Baseline | Impact | Native Batocera Paired Time Reduction | Decision |
| --- | --- | --- | --- |
| P3 decoded tile cache versus P2, repeated historical pair | Medium–high | VR 3.87%, 2.88%; original Software/OpenGL/Vulkan single pairs 3.3%, 2.5%, 3.0% | Remove as an independent change. |
| P3 decoded tile cache versus current P5 without P3 | Medium–high | VR 0.15%, 0.36% | Confirms standalone P3 is below threshold in the current code. |
| C1 fixed polygon points and stable index sort versus P5 without P3 | Low–medium | VR 1.56%, 0.73%; one OpenGL pair +0.3%, Vulkan −0.2% | Keep the bounded 3D change. |
| C2 derived YM3438 operator cache versus C1 | Medium | VR 5.35%, 4.33%, 4.26%, 4.83%; SWA 5.92%, 5.39% | Keep: repeated SWA gain exceeds 5%, VR is near threshold. |
| C3 stateless 128-pixel mask-block path versus C2 | Low–medium | VR −0.22%, +0.54%; SWA +0.49%, +0.17% | Keep the small positive SWA change with no state cost. |
| C3b decoded tile rows plus C3 mask-block composition versus C3 | Medium–high | VR 4.68%, 4.25%; SWA 6.56%, 6.08%; one VR frontend pair OpenGL 5.8%, Vulkan 5.6% | Keep: combined cache passes the 5% rule on SWA and native GPU paths. |
| C4 idle-UART stopped-CPU batching versus C3 | High | VR about −0.2%; SWA about −0.35% in two pairs each | Revert: no significant gain. |

The final C3b core was also compared directly with the pre-P1 baseline
`af6998a8e6afee42512983ca93ff3d373168e6a6d2333da98a3334f858ab854d`
on native Batocera using the same 3,750-frame VR recipe and ABBA order.
Candidate elapsed times were **25.812/25.942 s**; baseline times were
**64.004/65.081 s**. The pair means are **25.877/64.543 s**, a **59.91%**
reduction in elapsed replay time, or **2.49×** as many completed replays per
unit time. All three captured PNG and Save RAM checkpoints match byte for byte
in both candidate runs. The installed addon core and global RetroArch config
were hash-preserved during the comparison. Evidence is in
`/private/tmp/tgpulse-final-total-results/` and the isolated Batocera root
`/userdata/system/codex-core-tests/tgpulse-final-total-20261005/`.
The total is measured directly; individual percentages in the table are from
different baselines/scenes and must not be added.

P3's cache was first removed to isolate its effect, then restored only inside
C3b after the combined change passed the user-set rule. Its RAM comparison,
palette-at-composition, restore invalidation and standalone/Libretro ownership
remain as in the original P3 design. C1 replaces a four-point `Vec` with a
fixed array and stably sorts polygon indices instead of moving `Quad` values.
C2 caches only derived `Params` outside serialized YM3438 `State`, invalidates
on register writes and reset, and rebuilds after restore. C3 reads each mask
word once and uses full/hidden 128-pixel branches, following the algorithmic
shape of local MAME `segaic24.cpp` without copying its code. C3b uses cached
pen rows in those visible spans. C4's stopped-CPU interval was guarded by an
idle main transmitter and idle sound receiver; it preserved tested output but
did not improve native elapsed time, so its `i8251.rs`/serial/scheduler code
was restored exactly.

The matched Linux SHA-256 sequence is P3-pruned
`556f33c7446bd51bb821cf0bd515f865df72007816ad3b735f2dda8c7f179bf8`,
C1 `7fa8998042a1ec80d6159f943de96fb845afe0f49a188382cdb8a7da5d39cee9`,
C2 `28a74bb9a4933e278b96ec973ba89a990611b4eff1988bfa7ef998eb0c9c0582`,
C3 `22df11c8eac8daac15b3cac7cde5c4d18173cb3b73dd82773e813ed72055171a`,
C3b `92263280be8415f8626219038b9fc5918760182d84d8cee771f1f8c97526513d`,
and rejected C4 `c84784bfebd3fb5762b19e540e7441527d0eeac02954c8fb76d803d28d0c2bf9`.
The C3b macOS release SHA-256 is
`3b387b3ce620f640f3e3c83362ab0b2018d255550e496edc47d319b30e9079ff`.
Source baseline was commit `32800c8507c93a2882b1ec158e8682e9087751df`
with local P1–P5 changes. No ROM, personal config or generated core is part
of the source changes.

Core/adapter unit suites, nine-set 600-frame per-frame video/PCM/state/Save RAM
and continuation comparisons, targeted tile-cache invalidation tests and
standalone compilation were run across the retained changes. The final C3b
NetMerc comparison checks both City Workaround modes against the P3-pruned baseline
for video, PCM, Save RAM, state and continuation. C3b's Batocera OpenGL/Vulkan
final PNGs equal C3, as do its three direct-ABI captures. The addon binary
was replaced atomically after each retained candidate with a hash-checked
backup of the preceding trial. C4 was never installed on Batocera. The current
addon trial and active `/usr/lib/libretro` symlink both hash to the C3b Linux
artifact above; local RetroArch Development uses the matching C3b macOS build
and Development `.info`. Package publication and user-played game/controller
acceptance remain separate.

Evidence directories: `/private/tmp/tgpulse-{p3-repeat,p3-current,c1,c2,c2-repeat,c2-swa2,c3,c3-swa,c3b,c3b-swa,c4,c4-swa}-results`,
`/private/tmp/tgpulse-c3b-gpu-results`, and the corresponding isolated roots
under `/userdata/system/codex-core-tests/`. The local scope reports are
`/private/tmp/tgpulse-{c1,c2,c3,c3b,c4}-scope.json`; final NetMerc evidence is
`/private/tmp/tgpulse-final-c3b-netmerc/comparison.json`. All remote comparisons
kept the installed addon core and global RetroArch config unchanged until the
separate, user-authorized addon binary update.


## Publication Boundary — 0.1.0.6

The retained C3b source is committed as
`ea3cf08dfdfb8ac5e0eb8da7d0cff435520e65f4`, tagged `v0.1.0.6` and published
through the five native GitHub jobs. All published archives were downloaded
and checked for version, Model 1 scope, exact inventory, licenses, source
revision and external/internal hashes. The GitHub macOS arm64 artifact passed
local loading/ABI checks and replaced the local Development core with a
verified matching hash. Exact archive and installed hashes are recorded in
`LIBRETRO_CI.md`. The preceding performance figures describe the locally
measured C3b builds, not a new benchmark of every published GitHub binary.
Windows CI validates build, tests and native ABI/lifecycle behavior; Windows
gameplay, physical controllers and a canonical Batocera addon publication
remain separate from this release.


The subsequent [GitHub-binary regression campaign](LIBRETRO_0.1.0.6_REGRESSION.md)
confirms identical video, PCM, Save RAM, full states and continuation on macOS
and native Batocera, with matching VR replay and GPU captures. Native Windows
RetroArch checks also match final video captures and NVRAM for all sets, with
VR exercised on OpenGL and Vulkan. Windows Smart App Control blocks the direct
Python ABI comparison before emulation, so Windows complete PCM/state and
driven ABI replay checks remain unverified. The GitHub Windows DLL is installed
as a separate Development core for supplementary user testing.
