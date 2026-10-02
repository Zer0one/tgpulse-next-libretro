# Model 1 native GPU verification

## Reference and adaptation

Checked the current SM2 `GPU.md`, `PORTING_PLAN.md`, Vulkan/OpenGL wrappers,
`src/libretro/checks/opengl_host.cpp`, `scripts/smoke-opengl.py` and macOS
RetroArch launcher procedure before adapting them. Supermodel's native
supersampling implementation and the standalone TGPulse shaders were inspected
for the later renderer-option phase.

The core borrows frontend devices/contexts and shares the standalone WGSL
algorithms. Only the EGL **verification host** is adapted from the SM2 host;
its minimal public Libretro callback declaration is local to the test tool.
No reference project is a build dependency.

The sole implementation status document is [the roadmap](LIBRETRO_ROADMAP.md).

## Repeatable macOS RetroArch check

Use an actual RetroArch executable, rather than a Finder alias. On this host
it is beneath `~/Retro/Apps/RetroArch/Nightly/RetroArch (ARM).app`.
Choose a new output directory for each run. The runner creates isolated paths,
records the command/core hash, bounds frames/time and terminates only its own
process on timeout. It does not modify the application or global configuration.

```sh
python3 tools/test_retroarch_gpu.py \
  --retroarch '/absolute/path/RetroArch.app/Contents/MacOS/RetroArch' \
  --core target/release/libtgpulse_next_m1_libretro.dylib \
  --rom '/absolute/path/vr.zip' \
  --output /private/tmp/model1-vulkan-check \
  --renderer vulkan --driver vulkan --frames 180 \
  --moltenvk /opt/homebrew/lib/libMoltenVK.dylib
```

`--moltenvk` is optional when RetroArch already finds its Vulkan runtime. The
runner uses a private runtime symlink and environment setting for an **existing**
library; it neither installs MoltenVK nor patches the app. A normal Vulkan
session likewise needs a Vulkan video driver and an available Vulkan runtime.

Check Auto/Vulkan with `--renderer auto --driver vulkan`; check macOS
Auto/Software with `--renderer auto --driver glcore`. `--overlay` exercises the
existing timing panel. The panel measures CPU callback/command preparation,
not asynchronous GPU execution time.

## Repeatable desktop OpenGL / GLES check

Reuse the isolated Linux builder procedure rather than changing the macOS
OpenGL installation. macOS's 4.1 context cannot execute these compute shaders.
This phase reused existing Mesa and the compiler; Rust was installed only in
the isolated build volume with the user's explicit permission.

Inside a Linux verification environment with existing EGL/Mesa development
libraries:

```sh
cargo build --offline --release -p tgpulse-libretro
g++ -shared -fPIC tools/checks/opengl_host.cpp -o /tmp/model1-gl-host.so -lEGL -lGLESv2
EGL_PLATFORM=surfaceless LIBGL_ALWAYS_SOFTWARE=1 \
  python3 tools/test_libretro_opengl.py \
  --core target/release/libtgpulse_next_m1_libretro.so \
  --host /tmp/model1-gl-host.so --rom '/absolute/path/vr.zip' \
  --output /tmp/model1-desktop-check --api desktop --frames 600
```

Repeat with `--api gles`. Use `--context-cycle 400` for normal destruction/reset,
`--context-loss-cycle 400` for reset after loss without notification, or
`--api software` for the machine/audio/NVRAM baseline. Run each in a separate
process and new output root. `--nvram` optionally imports a validated same-set
fixture; always use the identical fixture and input schedule for comparisons.
Default synthetic input is neutral. The runner stores its own Save RAM and
native PPM frame; it never writes back to a supplied fixture or ROM.

The private host's CPU readback is verification-only. Normal core video uses
hardware-frame delivery and stays on GPU. Mesa software rendering validates
API operations, shader execution and lifecycle, **not GPU performance**.

When using the local Linux builder, synchronize a source-only snapshot and
cached Cargo sources. Exclude ROMs, saves and macOS build products. Supply ROM
inputs only to a disposable container filesystem; retain reports outside that
filesystem. Leave a shared pre-existing builder VM running.

## Evidence from 2026-10-02

- 33 adapter tests pass, including standalone WGSL validation and Vulkan/GLSL
  translation. macOS ARM64 and Linux x86_64 release builds succeed.
- macOS RetroArch 1.22.2, Apple M4, existing MoltenVK: explicit Vulkan and Auto
  complete bounded runs with actual 2D/3D output and valid PNG screenshots.
  Frontend context reset/destroy occurs during initialization. Geometry changes
  are bounded, rather than reported on every frame. Auto with macOS GLCore
  completes using Software. The timing overlay remains visible on Vulkan.
- Linux Mesa EGL: desktop and GLES deliver 180 actual hardware frames. Normal
  context recreation and unannounced loss at frame 90 retain identical output,
  audio and NVRAM on this early boot fixture.
- Frame 600 verifies the shared 3D pass on both desktop and GLES. GLES also
  recreates its context at frame 400. Desktop, GLES and Software produce the
  same PCM hash `e970b39a42f1bb815f093a20c9a01b00e554dc012d9b16da0a216b9ed835b36d`
  and Save RAM hash `cbb3f861a862535cd2214aa0d043fea7800bf393caef9500f0d9e8af0f9f558a`.
  465642 stereo sample frames are produced in each 600-frame run.
- Native screenshots were inspected for orientation, colour channels, 2D text
  and 3D geometry. Pixel hashes are not identical across Software/GPU or target
  framebuffer encodings. The core reuses the standalone GPU rasterizer; this
  is not a claim of byte-identical CPU rasterization. Consistent sRGB correction
  across frontend targets belongs to the following presentation-option phase.
- Windows, physical controllers, broad game acceptance and OpenGL inside a
  desktop RetroArch frontend are not established by these bounded EGL checks.

Evidence roots (temporary, outside source):

- `/private/tmp/tgpulse-gpu-final-vulkan`
- `/private/tmp/tgpulse-gpu-auto-fixed` (final macOS build, overlay)
- `/private/tmp/tgpulse-gpu-auto-software` (final macOS build, fallback)
- `/private/tmp/tgpulse-gpu-evidence-complete` (EGL reports and native frames)
- `/private/tmp/tgpulse-gpu-software-reference` (macOS Software boot baseline)

## Development deployment

After the verified macOS build:

```sh
python3 tools/install_dev_core.py
```

The installer atomically replaces only the agreed Development core and matching
`.info`, sets the established Development display names and verifies both copies.
It leaves updater-managed names untouched. Installation on 2026-10-02:

- Core SHA-256: `a119b8e4dae543b6dbdc4ecc57afb0664597f32a35f9144183ce156ba169434b`.
- Installed `.info` SHA-256: `e500861081e28bc565fc3ed4772e6153743f993b012de9374c16cb0ce6295d8b`.
- Core: `~/Library/Application Support/RetroArch/cores/tgpulse_next_dev_libretro.dylib`.
- Info: `~/Library/Application Support/RetroArch/info/tgpulse_next_dev_libretro.info`.

No commit or push is included. This deployment records the native-renderer
checkpoint; the presentation-option evidence and deployment follow below.

## Widescreen/supersampling evidence (2026-10-02)

Both existing runners accept `--aspect auto|4_3|16_9`,
`--widescreen stretch|expand_3d|expand_3d_2d` and `--supersampling 1|2|3|4`.
The EGL runner also accepts `--software-warmup 600`: warm the same machine in
Software, serialize it, reload the requested GPU backend and restore the
snapshot before a bounded image comparison. Use a fresh output directory for
each invocation. This avoids long compute rendering of startup just to obtain
a representative 3D frame. Supplied ROMs and fixtures remain unchanged.

- 37 adapter checks pass, including native/wide geometry, option defaults,
  supersampling buffer/uniform layout and overlay foreground opacity.
- macOS ARM64 and Linux x86_64 release builds succeed.
- RetroArch Vulkan: 600-frame runs with 16:9 Stretch/1x, Expand 3D/2x,
  Expand 3D + Stretch 2D/4x and 4:3 with the latter mode/4x all exit normally
  and produce PNGs. No redundant geometry updates are issued.
- Mesa EGL: desktop Expand 3D/2x with context recreation at frame 15, and
  GLES Expand 3D + Stretch 2D/4x each produce 30 hardware frames following
  the identical 600-frame Software warmup. Output is 683 x 384; the Software
  comparison remains 496 x 384. Inspected images show centered versus stretched
  native 2D text over the wider 3D scene.
- These three warmed comparisons produce identical PCM SHA-256
  `98002e9a106f880464e94e52bf95c5ba2091c3dbe1f1872d612be017dfc06990`
  (23283 stereo sample frames), and identical Save RAM SHA-256
  `82580444d4990236092e8fb9f543ad128ad44d94daa8bf44904b671c130d9f1d`.
- Image inspection identified transparent alpha on the existing timing panel's
  already-composited foreground pixels. The shared resolve skips transparent
  foreground. The panel now marks drawn pixels opaque; the existing bounds
  check also verifies this contract. Vulkan overlay is checked again on the
  final build. The WGSL files and machine state are unchanged.
- An initial Vulkan launch failed while RetroArch loaded Ozone menu textures
  (`image_texture_load_internal`, filesystem page-in error); the isolated runner
  now requests RGUI, which needs no external Ozone assets. The retry passes.
  A sandboxed launch without application access also failed before core startup;
  the authorized frontend execution completed normally.

Evidence roots: `/private/tmp/tgpulse-options-vulkan-stretch`,
`/private/tmp/tgpulse-options-vulkan-expanded`,
`/private/tmp/tgpulse-options-vulkan-layers-final`,
`/private/tmp/tgpulse-options-vulkan-four-three-final`,
`/private/tmp/tgpulse-options-vulkan-layers-overlay-final` and
`/private/tmp/tgpulse-wide-gl-evidence`. Linux results precede the shared-panel
opacity correction; final macOS Vulkan checks cover that correction.
These are bounded rendering/lifecycle checks, not gameplay acceptance or GPU
performance benchmarks. sRGB is deferred by user instruction.

Final presentation-phase Development deployment (2026-10-02), after the
37 passing checks and the final 600-frame Vulkan/4x/overlay run:

- Installed core SHA-256: `f7972a5355d34d4362f28923d6910eb81c3cc7c162af8ed9d88799c6e983174c`.
- Installed `.info` SHA-256: `e500861081e28bc565fc3ed4772e6153743f993b012de9374c16cb0ce6295d8b`.
- The existing installer compared both installed files with their source bytes.
- Account usage at phase completion: 45%; next reset 2026-10-07 15:58:51 CEST.
  The preceding reported reading was 44%, so the observed account-wide change
  is approximately one percentage point, not exclusive metering of this task.
  Required reasoning: High.
