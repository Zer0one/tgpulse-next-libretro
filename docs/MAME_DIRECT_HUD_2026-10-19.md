# Model 1 Direct Polygons And Above-HUD Composition

## Reference And Adaptation

Implemented locally on 2026-10-19 under the approved MAME maintenance review.
The behavior reference is official MAME `src/mame/sega/model1_v.cpp`, including
commits `70e0c03dadafe7e30b6ccc116e885aeb026e60d8`,
`1ac4de6a5daed23d567ea6d67126fa3bd6ec95dd` and
`9388840bcd07e96e5fe8437f367b85e272a5a1ef`.
The inspected current source SHA-256 is
`8747bdc4538b7313c6381abf75f075f3588b9bc4910ee3e77c8ae3175ba9c3ef`.
BSD attribution is retained in `LICENSES/MAME-BSD-3-Clause.txt`.

SM2 porting-plan section 8 and Supermodel U03 supply the shared-renderer and
same-state comparison workflow; neither supplies Model 1 command decoding.

## Changes

- Decode direct command `0x02` in the shared native renderer: screen-space
  projection, including zero/negative z, strip links, texture progression,
  luminosity/color translation and depth sorting with normal below-HUD objects.
- Use the backdrop pen for transparent moire records; suppress coincident-point
  moire records, following MAME's Wing War correction.
- Draw `0x41` objects after the native HUD. Preserve each original composed
  scene/HUD pixel when any RGB channel exceeds 8, matching current MAME.
- Share geometry and pass flags between Software, Vulkan and OpenGL/GLES.
  GPU composition reuses existing storage bindings and appends native HUD data
  only for frames containing above-HUD geometry. Diagnostic and Timing UI remain
  in the final display pass. The standalone uses the same composition rules.
- Bound malformed direct-record walks at the physical display-list limit.

No new Core Options, machine-state fields, save format, dependency or ROM
requirements. Existing render math and supersampling remain in use.

## Verification

Evidence root: `outputs/model1-hud-20261019/` (local, ignored).

- 254 Model 1 core tests pass; one previously ignored test remains ignored.
  Four added cases cover direct geometry/color/moire, z <= 0, widescreen
  centering, the 8/9 brightness boundary, bright underlying scene protection,
  and truncated records. Rendering preserves serialized native state.
- 94 adapter tests pass, including WGSL/SPIR-V/desktop GLSL/GLES translation
  and separation of the native HUD buffer from display UI.
- Standalone `cargo check` succeeds; macOS and Linux x86_64 release builds pass.
- Nine-set, 300-frame old/new direct-ABI comparison preserves video, PCM,
  Save RAM, serialized state and restored continuation. A separate SWA
  1,800-frame comparison plus 30-frame continuation also matches all outputs.
- Isolated macOS RetroArch Vulkan runs SWA for 1,800 frames, exits normally
  and captures a valid frame on Apple M4.
- Isolated Linux EGL tests start from the same 1,800-frame Software-warmed
  state and render five frames through desktop OpenGL, GLES and Software.
  GLES/Software image hashes match. PCM and NVRAM match across all three.
  Desktop image values match the framebuffer sRGB conversion within one
  8-bit level per channel; raw image hashes are therefore not equal.
- A further GLES run passes at 683×384, 2× supersampling, expanded 3D/stretched
  2D, Timing Panel enabled and context recreation at frame 2.
- macOS artifact gate and public local installation pass. Installed/build
  SHA-256: `e3372539bfecdf8f58e251261a94022154ff2af57d75e7a444766940c61171f6`.
  Metadata matches the public identity; installer backs up replaced files.

## Limits

The unchanged game captures are regression evidence, not proof that those
particular attract frames exercise the new commands. Targeted synthetic cases
establish the new CPU behavior and shared GPU geometry/buffer contract; shader
translation and real API runs establish execution. Exhaustive command coverage
in gameplay and every GPU/driver remains outside this bounded verification.
Linux runs use the existing amd64 Colima/Rosetta environment and Mesa software
rendering, not native Batocera GPU or Windows controller acceptance.
No upstream source was edited and no commit, push or release was performed.

## Transfer

The native decoder/composition is a standalone backport candidate (B21).
Retain it for the future Complete core's Model 1 path; Model 2 has different
hardware and must not inherit command meanings or this stencil unexamined.
Framework transfer is the verification/composition boundary, not a universal
Model 1 hardware contract.
