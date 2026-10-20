# Model 1 Software Wireframe Clipping

## Reference And Adaptation

Adapted the viewport clipping algorithm from MAME
[`2194d88adc5172c316e0eda805c61ded7e9e1810`](https://github.com/mamedev/mame/commit/2194d88adc5172c316e0eda805c61ded7e9e1810).
The target source was inspected before editing. SM2's current PORTING_PLAN
Direct MAME Audio Review provides the focused native-device adaptation and
bounded old/new comparison procedure. Supermodel ROADMAP U03 provides the
renderer edge-fix/unchanged-scene verification pattern. Neither is a source
for this Model 1 clipping implementation. Framework F02/F03 keeps it in the
frontend-neutral machine renderer; no Libretro callback dependencies were added.

## Symptom And Cause

Software `draw_line` checked each pixel against the viewport but walked the
entire untrimmed Bresenham segment. Projected endpoints can span i32 limits:
integer differences/error terms can overflow, or traversal can take an
excessive number of steps. MAME reports a Wing War boot lockup in this area.
That specific game/host lockup was not reproduced in this project; the source
risk and extreme-coordinate behavior were the target of this maintenance.

## Delivered Correction

`clip_wireframe` applies Liang-Barsky clipping in f64 before any integer
subtractions. It intersects the viewport with the actual Software framebuffer
(496×384), rejects external/empty segments, rounds and bounds newly clipped
endpoints, and preserves original endpoints when already visible. This
framebuffer intersection is the small target-specific guard for malformed
or oversized viewports. Existing Bresenham tie rules, colors, moire and blend
handling remain unchanged.

GPU shaders use bounded closed-form per-pixel coverage rather than this
unbounded line walker. They were not changed. Offscreen-line clipping can
change edge coverage relative to an untrimmed Bresenham; whole-scene GPU/CPU
pixel parity is not established by this change. Projection, native timing,
ROMs, Save States, Model 2, direct polygons and above-HUD composition are outside
the change. The shared Model 1 renderer also benefits standalone Software use.
MAME attribution was added to the retained BSD notice.

## Verification And Delivery

- Four targeted tests cover viewport/corner clipping, external/empty rejection,
  in-view endpoint identity and known Bresenham pixels, and i32 MIN/MAX lines in
  both directions. Rasterized extremes finish within the framebuffer line bound.
- Model 1 core tests: 250 passed, one existing ignored; adapter tests: 93 passed.
- The existing ABI baseline/candidate comparison passed all nine supported
  fixtures at 300 frames each: equal video, PCM, Save RAM, machine snapshots
  and continued state replay. It includes all four Wing War sets and both SWA
  sets. These are bounded startup comparisons, not complete gameplay coverage.
- Isolated RetroArch 1.22.2 ARM Software/GLCore Wing War run completed 600 frames
  in 10.868 seconds, exited normally and delivered a valid final image. Frontend
  audio was disabled for this smoke check; PCM comparison is separate above.
- The macOS release build passed native format, 25 exported ABI symbols,
  dependencies and three empty-lifecycle cycles. Installed under the existing
  public identity with backup and matching SHA-256: `c275f043df351288f7f15cc29d8db004f9d93680368564b15150ede8443b433f`.
  Matching `.info` SHA-256: `fcd427ff7dffde030f7e04f80f54d8ab9c40b655be616aeb13dd1eea60ccffd3`.

Unrelated existing adapter/network/menu and PR-review edits were preserved.
No dependencies were installed; no commit, push, version bump or release was
made. No Windows/Linux, physical controller or GPU runtime acceptance is claimed.

Evidence is retained under ignored `outputs/wireframe-clip-20261019/`:
`core-tests.log`, `adapter-tests.log`, `comparison.json`, `artifact.log`,
`retroarch-wingwar/result.json`, `build.log` and `install.json`.
The comparison baseline was built from the actual pre-change working tree,
including its already-pending unrelated edits, so the renderer delta is isolated.
