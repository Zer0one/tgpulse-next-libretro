# Model 1 Libretro renderer adaptation

## Source inspection (2026-10-01)

The native Vulkan and OpenGL/GLES adapters are implemented. Renderer-specific
widescreen and supersampling Core Options are implemented. sRGB was removed
from the roadmap by user instruction; Metal feasibility was reassessed below.

- SM2 `GPU.md`, `src/libretro/vulkan_renderer.cpp` and
  `opengl_renderer.cpp`: frontend-owned device/context, callbacks for reset and
  destruction, queue synchronization, direct hardware image delivery, explicit
  backend failure and Auto fallback. Shared upstream rendering algorithms stay
  outside the Libretro adapter. OpenGL compute requires 4.3 or GLES 3.1.
- Supermodel `Src/OSD/Libretro/libretro_core_options.h` and
  `LibretroWrapper.cpp`: native supersampling is a renderer option rather than
  a frontend scaling shader. Existing options describe cost and restart behavior.
- Standalone TGPulse `platform/gpu_model1.rs`, `gpu_model1.wgsl`,
  `gpu_resolve.wgsl`, `platform/video.rs` and `app.rs`: existing Model 1 quad
  rasterizer, resolve/composition pass, supersampling 1–4, wider projection,
  optional stretching of native tile layers and sRGB surface presentation.
- Machine boundary: `model1_video::gpu_quads_ws`,
  `tilemap::render_background` and `tilemap::render_foreground` already provide
  the renderer data. No Libretro dependency is needed in emulated hardware.

## First backend

Metal assessment: RetroArch has a Metal video driver, but its current public
Libretro hardware-context enum has no Metal context and its hardware rendering
interfaces do not expose a native Metal device/texture handoff to cores.
Checked both the SM2 vendored header and current upstream RetroArch
`libretro-common/include/libretro.h` on 2026-10-01. RetroArch's Metal driver is
in `gfx/drivers/metal.m`. Driver availability does not supply a core context.

The standalone wgpu backend can use Metal, but a private device plus CPU image
readback would be a separate rendering/transport architecture, with copies and
synchronization to measure; it is not equivalent to the frontend-owned GPU
handoff required by this design. That alternative has not been benchmarked.
Vulkan on macOS can execute through MoltenVK on Metal while using Libretro's
published Vulkan interface. This explains the first backend selection.

Sources:
- https://github.com/libretro/RetroArch/blob/master/libretro-common/include/libretro.h
- https://github.com/libretro/RetroArch/blob/master/gfx/drivers/metal.m

Start with Vulkan plus the retained Software path. The compute shaders need
features unavailable in macOS OpenGL 4.1; a Vulkan path matches the current SM2
reference procedure on this host. Additional APIs are separate adaptations.

First inspect the installed RetroArch Vulkan interface and device capabilities.
Reuse SM2's isolated macOS launcher procedure if the bundled Vulkan runtime is
insufficient. Do not change the installed application or install dependencies
without authorization.

Reuse the standalone WGSL algorithms and Model 1 data layout. Assess importing
frontend Vulkan handles into the existing wgpu abstraction before choosing a
bridge; resource ownership, enabled features and shared queue synchronization
must all be supported. If that bridge cannot meet the Libretro contract, use
an adapter-owned Vulkan compute wrapper around the same shaders and buffers,
compiled through the existing Naga shader toolchain. This decision needs code
inspection and a bounded integration prototype, not an assumed compatibility.
Do not introduce a second independent device or a host window as the normal
hardware path. The normal path must deliver a GPU image without CPU readback.

## User-visible controls

- Renderer: Auto / Vulkan / OpenGL / GLES / Software, applied at content reload. Auto has an
  explicit logged fallback; Vulkan must fail clearly when unsupported.
- Keep Aspect Ratio Auto / 4:3 / 16:9 for requested presentation geometry.
- Widescreen Mode: Stretch Entire Image / Expand 3D View /
  Expand 3D View + Stretch 2D. The first preserves the existing presentation;
  expansion modes follow the standalone wider projection and layer composition.
  Expanded rendering is used only when the effective aspect is wide. For fixed
  16:9 at native height, preserve the standalone 683-pixel render width.
- Supersampling: standalone 1–4 sample scale. Do not claim extra tile detail.
- sRGB Correction (deferred, not exposed): preserve the standalone Off default and display-encoded
  framebuffer colours. Check the effective texture format and frontend transfer
  behavior to avoid double conversion; do not substitute machine CRT gamma.
- Hide unsupported renderer controls on the Software path.

## Incremental implementation

1. Verify the frontend interface; implement context negotiation/reset/destroy,
   device ownership and a minimal hardware image handoff.
2. Reuse native quad generation and the shared raster/resolve algorithms at
   native geometry and sampling. Keep the Software baseline available.
3. Add the agreed widescreen modes and supersampling; defer sRGB, including
   geometry changes and resource resizing.
4. Keep the existing timing overlay working in both paths. Preserve audio,
   per-set Gain selection, input, NVRAM and machine-state lifecycle.
5. Verify builds, shader validation and native frontend lifecycle. Install the
   Development core and compare hashes after each verified macOS release build.

Automated rendering comparisons and lifecycle checks are implementation evidence.
Game/controller acceptance remains user-run and outside the roadmap. Do not
claim a backend completed before actual frontend image handoff works.

## Native backend implementation (2026-10-02)

The adapter compiles the **same standalone WGSL files** through Naga, to SPIR-V
for Vulkan and GLSL for OpenGL/GLES. It reuses native quad binning, background
and foreground generation; emulated hardware has no new frontend dependency.
The frontend owns the Vulkan instance/device/queue or OpenGL context/FBO.
Only core buffers, shaders, command resources and output images are allocated
by the adapter. The normal frame path has no CPU image readback.

Vulkan follows the SM2 v5 interface procedure: resources per frontend sync
index, wait before reuse, command-buffer delivery and image handoff. OpenGL
follows SM2's 4.3/GLES 3.1 compute requirement, frontend symbol lookup and FBO
presentation. Context destruction frees core resources while the context is
valid; a reset after unannounced loss discards obsolete handles without calling
a dead API or retaining host allocations.

**Renderer (Restart Required)** was the selector exposed in the native backend
phase: Auto / Vulkan / OpenGL / GLES / Software (OpenGL/GLES share one item).
Auto follows the public frontend preference; macOS GL falls back to Software
because macOS provides OpenGL 4.1, below the compute requirement. Explicit
backend selection reports rejection or initialization errors. Negotiation
rejection in Auto falls back to Software. Failure after an accepted hardware
context reports an error and requests frontend shutdown, avoiding invalid frames.

## Widescreen and supersampling adaptation (2026-10-02)

Current standalone and Supermodel option/shader implementations were inspected
before this phase. Renderer, Widescreen Mode and Supersampling are applied at
content reload, following the reference restart policy. Defaults remain Stretch
Entire Image and 1x. Supersampling scales 1–4 match the standalone limit; scales
2/3/4 average 4/9/16 3D samples. They also work at 4:3 and do not add tile detail.

All Widescreen Mode choices apply only when the effective Aspect Ratio is wide,
including Auto when the game's saved monitor setting requests widescreen. At
4:3 all choices retain 496 x 384 output. In wide aspect, Stretch Entire Image
keeps native output for frontend scaling; Expand 3D View renders 683 x 384 with
native 2D layers centered; Expand 3D View + Stretch 2D stretches both background
and foreground. This reuses the standalone Model 1 composition. Supermodel's
lower-background-only stretch is not copied over this different layer model.
Aspect Ratio remains live; geometry publication tracks actual output width
without repeatedly reporting unchanged geometry.

The timing overlay is composed into the native foreground and follows the
selected foreground placement/stretch. Hardware options are hidden on Software
using frontend display hints; legacy frontends may still show inert selectors.
The shared standalone WGSL raster/resolve files are unchanged.

**sRGB is deferred by explicit user instruction.** No sRGB selector is registered
or read. The standalone can inspect its final surface format, whereas Libretro's
Vulkan interface does not expose the frontend swapchain format. That adaptation
must be decided before claiming equivalent correction. The Metal feasibility reassessment follows below.

See [GPU verification procedure](LIBRETRO_GPU_VERIFICATION.md) for commands,
source references, evidence limits and repeatable runners. Implementation
status is maintained only in [the sole roadmap](LIBRETRO_ROADMAP.md).

## Effort and reasoning

High reasoning is required for borrowed resources and synchronization. No
additional model change is required for the completed native backend work.
OpenGL verification uses the existing isolated Linux/Mesa environment; the
user authorized adding Rust only inside its build volume. No macOS toolchain,
RetroArch installation or system dependency was changed.

## Metal feasibility reassessment (2026-10-02)

Re-read the current official RetroArch `libretro-common/include/libretro.h` and
`gfx/drivers/metal.m`, plus SM2 `GPU.md` and vendored `libretro.h`, and the
Supermodel Libretro options/roadmap. SM2 explicitly documents no Metal backend;
Supermodel supplies no native Metal core/context bridge. Neither provides a
Metal adaptation procedure to reuse.

The current public hardware-context enum lists GL/GLES, Vulkan and Direct3D,
with no Metal context. The hardware-interface enum likewise has no Metal
texture/device/queue handoff. RetroArch's Metal driver uploads pixel frames
through its private `FrameView` implementation. This is frontend presentation,
not a published Metal rendering interface for cores.

Standalone TGPulse selects `wgpu::Backends::METAL` on macOS in
`crates/tgpulse/src/platform/video.rs`, owns its device and surface, and uses
the existing Model 1 compute shaders. The algorithms can therefore target
Metal. The missing part is standard Libretro native texture delivery.

| Route | Feasibility and adaptation |
| --- | --- |
| Existing Vulkan core + MoltenVK | Implemented and verified; MoltenVK executes the GPU work on Metal and uses the published Libretro Vulkan interface. RetroArch must use its Vulkan driver. |
| Existing Software core + RetroArch Metal driver | Standard pixel delivery is available; Model 1 rendering remains on CPU. This does not enable hardware widescreen/supersampling. |
| Private Metal compute renderer + CPU pixel delivery | Technically feasible by adapting standalone wgpu to an offscreen device, reading resolved output into host-visible memory, then using standard video callbacks. GPU/CPU synchronization and frontend upload costs require measurements. No prototype or performance claim is included here. |
| Native Metal core with direct texture handoff | No public interface currently exists. Requires coordinated Libretro/RetroArch API and frontend work, or a private extension tied to a modified frontend. This is not a core-only integration. |

The private-renderer route changes the current frontend-owned, no-readback
architecture and needs an explicit user adaptation decision before implementation.
It would keep the shared WGSL and machine code unchanged, but add macOS-only
resource lifecycle, readback buffers, synchronization, and failure behavior.
No Metal selector has been added to imply native support.

Official source links checked on 2026-10-02:

- https://github.com/libretro/RetroArch/blob/master/libretro-common/include/libretro.h
- https://github.com/libretro/RetroArch/blob/master/gfx/drivers/metal.m

Tiny scope: the sole roadmap now schedules a Model 1-only Cargo build. Current
release settings already use thin LTO and one codegen unit; inspect surviving
binary contents before changing features. References are single-system ports,
so their architecture supplies the narrow-core criterion, not an existing
multi-system Cargo split. No Tiny implementation is included in this assessment.

### Installed RetroArch negotiation check

The existing isolated runner now accepts `--driver metal`. On the installed
RetroArch 1.22.2, the bounded Auto/Metal invocation reports
`GET_PREFERRED_HW_RENDER: RETRO_HW_CONTEXT_NONE`; the core selects Software.
The frontend repeatedly initializes its video driver and does not complete the
180-frame run or produce a PNG within 60 seconds. The runner terminates only
its owned process. Reports are in `/private/tmp/tgpulse-metal-assessment`.
This confirms the observed negotiation result; it does **not** establish
working frame delivery on this installed Metal frontend. No source rendering
change, build, dependency installation or Development redeployment was needed.

Assessment reasoning: High. Account usage reads 45%; next reset
2026-10-07 15:58:51 CEST. No exclusive task cost can be inferred from the
unchanged rounded account percentage.
