# GitHub 0.1.0.6 Regression Verification

## Reference And Scope

Reuse SM2-Emu `PORTING_PLAN.md` phase 4 and `CI.md`: test distributed binaries,
record exact hashes and isolate configuration/saves. The existing Model 1
scope host and NetMerc audio/control host provide deterministic comparisons;
`test_retroarch_gpu.py` and the established VR input recipe provide frontend
and driven-scene evidence. No reference machine code is imported.

Compare published GitHub **0.1.0.5** with **0.1.0.6**, rather than local
candidates. The latter is tagged at `ea3cf08dfdfb8ac5e0eb8da7d0cff435520e65f4`.
Package digests, checksums and source revisions were verified at publication.

## Initial Local Alignment

The repository-root dylib was replaced with the GitHub macOS arm64 binary.
It matches the installed `tgpulse_next_dev_m1_libretro.dylib` at
`/Users/andrea/Library/Application Support/RetroArch/cores/`.
Both SHA-256 values are
`438511892ea2ee4fba0b04a17bd1e83eb341fa0e73cbe8a674da6826d31e5191`.
Public root metadata matches the package; installed metadata retains the
separate Development names and version 0.1.0.6.

## Native Results — 2026-10-05

| Host | Emulation Comparison | Driven Replay | Frontend Video |
| --- | --- | --- | --- |
| macOS arm64, Apple M4 | PASS: nine sets plus NetMerc in both City Workaround modes | PASS: 3,750-frame VR replay, three image/Save RAM checkpoints identical | PASS: 600-frame Vulkan images identical |
| Batocera 43.1 x86_64, Ryzen 5 3400G / Vega 11 | PASS: same complete set comparison | PASS: same VR replay and checkpoints | PASS: 600-frame OpenGL/Vulkan images identical |
| Windows 11 x86_64, Ryzen 5 3400G / Vega 11 | PASS: frontend images/Save RAM for nine sets plus NetMerc in both City modes; direct PCM/state comparison blocked by OS policy | Direct ABI replay not executed after the loader rejection | PASS: VR OpenGL/Vulkan and all other sets Vulkan, 600 frames each |
| macOS Intel / Linux ARM64 | Prior native release CI gates passed; no content regression host tested here | Not tested | Not tested |

The nine sets are `vr`, `vformula`, `vf`, `wingwar`, `wingwarj`, `wingwaru`,
`wingwar360`, `swa`, `swaj`. Each runs 600 frames with fresh machine instances.
Checks compare complete framebuffer/PCM streams, sample counts, full serialized
state, Save RAM and restored-state continuation. NetMerc covers both City
Workaround modes and ten continuation frames after loading the baseline state.

| Core | GitHub 0.1.0.5 SHA-256 | GitHub 0.1.0.6 SHA-256 |
| --- | --- | --- |
| macOS arm64 | `56a46f55ba5fbdf71c867f3716e6b7953cf14b74c80b0ac2583c0a9fb2a5d0fd` | `438511892ea2ee4fba0b04a17bd1e83eb341fa0e73cbe8a674da6826d31e5191` |
| Windows x86_64 | `523f9e3823ea575422016a24836ec9ab93a1244ddaa30fcaae4b04d6e8be3246` | `6a0a312aee61fb413557089587a480c24a4381580afd39c27785873b24a6d80a` |
| Linux x86_64 | `c01f4ff75cb49d86f62337492380cf92752275bc37e49f3b5b9f8e42718013d0` | `a6e41fe84a011a08bae38678478302cb846836ca26c78a8dd605e74df4f1ec0c` |

No mismatches were observed in the exercised paths. Windows checks compare
final frontend captures and saved NVRAM; they do not establish complete PCM
streams, full states or restored-state continuation on that OS. Bounded equality does not
establish every game scene, physical controller, sound device or minimum FPS.
macOS cannot supply the compute context required by this core's OpenGL path.

## Replay Timings

Single bounded runs including startup/capture, not a replacement for paired
performance measurements; percentages from different paths must not be added.

| Host / Path | 0.1.0.5 | 0.1.0.6 |
| --- | ---: | ---: |
| macOS direct ABI, 3,750-frame VR replay | 21.359 s | 8.785 s |
| Batocera direct ABI, same replay | 64.300 s | 25.968 s |
| macOS Vulkan, 600 frames | 10.742 s | 6.810 s |
| Batocera OpenGL, 600 frames | 11.331 s | 4.673 s |
| Batocera Vulkan, 600 frames | 11.335 s | 4.723 s |
| Windows OpenGL, 600 frames | 24.071 s | 6.594 s |
| Windows Vulkan, 600 frames | 11.660 s | 5.385 s |

## Procedure And Evidence

`tools/test_libretro_model1_regression.py` delegates the nine-set comparison to
the existing scope runner and adds NetMerc through `AudioHost`. Pass
`--baseline-core`, `--core`, `--rom-dir`, `--output`, and optionally `--bios-dir`.
The last argument copies only known device BIOS ZIPs into the isolated system.
`--netmerc-only` resumes NetMerc without repeating a completed nine-set check.
Raw content, saved machine data and captured output remain outside Git.
GPU checks use the established runner at 60 Hz, 600 frames and 90-second bounds.

Local evidence:

- `/private/tmp/tgpulse-gh-regression-macos-full-0.1.0.6/comparison.json`
- `/private/tmp/tgpulse-gh-regression-macos-render/comparison.json`
- `/private/tmp/tgpulse-gh-regression-batocera-results/emulation/nine-sets.json`
- `/private/tmp/tgpulse-gh-regression-batocera-results-b/emulation/comparison.json`
- `/private/tmp/tgpulse-gh-regression-batocera-results-b/comparison.json`
- `/private/tmp/tgpulse-gh-regression-batocera-results-b/installed-preserved.txt`

Batocera uses the existing remote-core-test skill, `root@batocera` and the
established SSH identity. Marked roots are
`/userdata/system/codex-core-tests/tgpulse-gh-regression-0106` and the `0106b`
continuation. Installed addon core/global RetroArch configuration hashes match
before/after; existing ROMs were read-only and all test output stayed isolated.

Two infrastructure issues were resolved without modifying emulation: the
initial isolated NetMerc system lacked `model1io2.zip`, so only NetMerc and
remaining checks resumed with the existing BIOS copied into the fixture.
A newline-escaping error stopped the bundle's final summary after all frontend
runs; fetched manifests/images were checked locally without repeating any
completed run. Original transport exit receipts retain those explanations.

## Windows Continuation

The fresh probe confirmed an active RetroPlayer desktop on RETROSTATION,
Windows 11, Vega 11 driver 31.0.21925.1001 and RetroArch 1.22.2. The established
Windows remote skill staged GitHub DLLs in marked test roots under
`C:/Users/RetroPlayer/Codex/windows-libretro-tests/runs/`.
All ten resident game ZIP digests match the macOS fixtures; content is read-only.

Python was absent. With explicit user authorization, the official
[Python 3.14.8 portable package](https://www.python.org/downloads/release/python-3148/)
was extracted only inside the isolated test payload. Its archive SHA-256 is
`a93abe456ab01bd96d7a085b3cdb6566b3063f4241360d114142fbdb07f0a310`.
No system installation, PATH change or package installation was performed.

The direct ABI campaign stopped before its first frame: `ctypes.CDLL` returned
WinError 4551 for the GitHub 0.1.0.5 baseline DLL. Code Integrity event 3077
identifies that exact DLL and Python process; event 3118 reports Smart App
Control. This is an OS loader rejection, not an emulation mismatch. Security
policies were not modified. A separate frontend-only bundle checks the ordinary
RetroArch loader with both GitHub versions and OpenGL/Vulkan. All four VR
runs succeeded, with identical captured images for each renderer. Further
600-frame Vulkan checks cover the other eight sets and NetMerc in both City
Workaround modes. All twenty additional runs succeeded; captured PNG bytes
and saved NVRAM files match between releases for every tested configuration.
VR saved NVRAM also matches on both renderers. The staged frontend runner adds
only the existing `tgpulse_next_netmerc_city_workaround` core option to its
isolated configuration; emulation source is unchanged. The longer OpenGL baseline run includes initialization and
is not an isolated steady-state performance measurement.

Installed core and global configuration hashes remain unchanged after the
first attempt:

- Core: `1bac1ca1c58bf0bcc3e07b6479e0ce5ebb4ffa3c920da2921417b3aed6246983`.
- Configuration: `3bd0450c1f0230416bdb8701a4900015076ce14997b63d85ed55dd0e3c640ae8`.

Evidence: `/private/tmp/tgpulse-gh-regression-windows-results/`, including
`emulation.log`, `runner.log`, runtime provenance and the original exit code.
Frontend evidence:

- `/private/tmp/tgpulse-gh-regression-windows-gpu-results/comparison.json`
- `/private/tmp/tgpulse-gh-regression-windows-titles-results/comparison.json`
- Both result roots include `installed-before.json`, `installed-after.json`
  and `installed-preserved.txt`.

Marked remote runs are `tgpulse-gh-regression-0106`,
`tgpulse-gh-regression-0106-gpu` and `tgpulse-gh-regression-0106-titles`.

### Windows Development Deployment

After the user's explicit request for a permanent supplementary test copy,
the exact GitHub 0.1.0.6 DLL was copied to
`C:/RetroArch-Win64/cores/tgpulse_next_dev_m1_libretro.dll`, with matching
metadata at `C:/RetroArch-Win64/info/tgpulse_next_dev_m1_libretro.info`.
The name is **TGPulse-Next: Model 1 Development**, label
**Sega - Model 1 (TGPulse-Next Development)**, version **0.1.0.6**.
The DLL hash matches the Windows candidate digest above. Metadata SHA-256:
`c61bf19e7d44ce03f1b67e142cc7ca4efcd7610b9360f246119fc30638380ea5`.
The previous public core and global configuration still match their original
hashes. Existing ROMs are already available under
`C:/RetroArch-Win64/downloads/Sega - Arcade (Model 1)/`.
Deployment receipt:
`/private/tmp/tgpulse-gh-regression-windows-development-deployment.json`.

No commit or push was made during this verification phase. Subsequent source
publication includes this evidence and the Netplay corrections documented in
[Linked Cabinets](LIBRETRO_MODEL1_LINKED_CABINETS.md#public-local-naming-update--2026-10-05).

Required reasoning: High. Usage: 64% before the campaign, 65% after macOS and
Batocera, 67% during Windows continuation (shared-account readings).
Reset: 2026-10-10 09:49 CEST.

## macOS Core Rename And Metadata Cache

The user renamed the local DLL-equivalent dylib to
`tgpulse_next_m1_libretro.dylib` for networking tests. The binary still matches
GitHub 0.1.0.6 and loads through the Libretro ABI; its matching `.info` exists
and declares ZIP support. RetroArch's compressed `core_info.cache` retained an
entry with `has_info: false` and empty supported extensions. The cache was
moved to a timestamped `core_info.cache.before-tgpulse-refresh-*` backup so the
next full RetroArch launch can rebuild metadata. The current application must
be restarted; frontend recognition after that restart remains user-confirmed.
No core binary, configuration, save or content was changed by this repair.

## Netplay Core Name Lookup Repair

RetroArch 1.22.2 `tasks/task_netplay_find_content.c` compares the advertised
runtime `library_name` against each local `.info` `corename` when selecting a
core from a lobby. Both installed macOS and active Batocera 0.1.0.6 cores report
`TGPulse-Next`, while distributed metadata declares `TGPulse-Next: Model 1`.
This prevents automatic core discovery even when filenames match.

For the currently installed GitHub binary, the user's renamed public macOS
metadata temporarily uses `corename = "TGPulse-Next"`; its display label is
unchanged. Original metadata and the stale cache have timestamped backups.
A full Mac RetroArch restart is needed before retrying the lobby. The ongoing
Batocera host was inspected read-only and was not restarted or changed.
Actual connection success after this repair remains user-confirmed.

The source now reports `TGPulse-Next: Model 1`, matching the public metadata for
future releases. The existing ABI lifecycle test compares the runtime name to
the real distributed `.info`, preventing another lookup mismatch. The focused
test passed. No release binary was rebuilt or replaced for this source fix.
When installing a future release containing it, restore that release's matching
public `.info` rather than carrying forward the temporary compatibility name.

Reference source:
https://github.com/libretro/RetroArch/blob/69a4f0e/tasks/task_netplay_find_content.c
