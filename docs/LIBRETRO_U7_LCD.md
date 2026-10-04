# U7 Diagnostic LCD — 2026-10-04

Local implementation and focused delivery are complete. The user approved the
HD44780-only adaptation, missing-BIOS notification and System-first LCD/I/O
lookup order before implementation. Status remains solely in
[the roadmap](LIBRETRO_ROADMAP.md).

## Inspected References

- TGPulse-Next `f303b712455571b60cbea2249e1022935d398e22`,
  `crates/tgpulse/src/gui/diagnostic.rs`: Off/Overlay/Dedicated Window,
  HD44780/Text, four corners, background opacity; defaults Off, HD44780,
  Top Right and 80%.
- SM2 `PORTING_PLAN.md` 3.7–3.8 and
  `src/libretro/timing_overlay.cpp`: menu consistency and in-frame composition
  across Software/Vulkan/OpenGL, with explicit alpha blending.
- Supermodel `Docs/ROADMAP.md` and
  `Src/OSD/libretro/libretro_core_options.h`: frontend overlay convention.
  Neither reference core provides this diagnostic LCD.
- Current local `model1board.rs` already exposes `diagnostic_lines` and
  `diagnostic_pixels`; the hardware state and blink clock are serialized.
  The optional font loader already validates size and SHA-1.

## Delivered Video Options

All three options are global, always visible and applied live.

| Label | Values | Default |
| --- | --- | --- |
| Sega NetMerc Diagnostic Display | OFF / ON (100%) / ON (50%) | OFF |
| Sega NetMerc Diagnostic Position | Top Left / Top Right / Bottom Right / Bottom Left | Top Right |
| Sega NetMerc Diagnostic Background Opacity | 0–100%, Step 10 | 80% |

Render the two native 20-character lines. HD44780 uses existing dot output,
including custom characters, display shift, cursor and blink. HD44780 is the
only supported rendering mode; no Text selector or text fallback is exposed. Opacity affects the background; active glyphs remain opaque.
Use the existing compositor on Software/Vulkan/OpenGL, anchored to final output
geometry with legible scaling under widescreen and supersampling. Handle
simultaneous Timing / FPS and LCD overlays at their selected corners. Where
they overlap, draw Timing over the LCD; never reposition the LCD automatically.
ON (50%) halves both LCD dimensions, including padding, without a separate
size option. The existing `overlay` stored value means ON (100%);
`overlay_half` means ON (50%), and `off` remains the default.

The controls remain visible for all loaded games; the LCD is drawn only for
Sega NetMerc. OFF disables presentation, not the emulated LCD hardware.
Reset and Save State restore the native display; frontend preferences remain
those currently selected. No state format change is proposed.

## Optional LCD BIOS And Resource Priority

Search for validated `hd44780_a00.bin` in this order:

1. `hd44780.zip` under the frontend system directory's `tgpulse-next/`.
2. Adjacent `hd44780.zip`.
3. The loaded game ZIP.

The first location follows the established frontend BIOS path. Reuse one
validator for every location and continue to the next candidate when a BIOS is
missing or invalid. The LCD BIOS remains optional: if either ON mode is active and no
valid BIOS is found, do not draw the LCD and send an appropriate queued frontend
notification once when display activation is attempted. OFF produces no missing
LCD BIOS notification. Do not fail content loading or substitute Text.

Apply the same System → adjacent device ZIP → game ZIP priority to I/O
firmware: `model1io.zip` for the original board, `model1io2.zip` for the advanced
board. Sega NetMerc requires `epr-18021.6` from `model1io2.zip`; board/firmware
selection remains tied to the identified game. This resource-order change does
not make required I/O firmware optional and does not apply to donor game ROMs.
The original standalone loader reads I/O firmware from the game ZIP; the new Libretro entrypoint implements the ordered external lookup. The
standalone entrypoints retain their existing resource/presentation policy.

## Approved Adaptations

- Replace the standalone Dedicated Window with in-frame display only.
- Include full and half size in the existing Display selector.
- Keep overlapping panels at their selected corners, with Timing above LCD.
- Quantize its continuous opacity slider into 10% Core Option steps.
- Apply the user-specified System-first order to LCD and I/O BIOS lookup.
- Support HD44780 only; notify missing/invalid LCD BIOS only with display active.
- Use the existing compositor instead of introducing desktop window ownership.

## Focused Delivery Gates

1. Native display content, custom glyphs/blink and reset/state continuity;
   missing or invalid LCD BIOS generates a notification only with display active.
   Verify ordered LCD/I/O lookup and game-appropriate firmware validation.
2. Visible composition and alpha on Software/Vulkan/OpenGL, widescreen,
   supersampling and simultaneous timing overlay. Reuse existing runners.
3. Modern/legacy option registration, defaults and live updates; verify an
   unaffected title, then release-build and install the Development core/info
   with matching SHA-256.

Record implementation evidence here and assess reusable backport candidates
after completion. Stop after U7; U9 preparation is a separate reviewed step.

## Estimate

High reasoning, medium effort (M), approximately 1–3 incremental account-usage
percentage points. The interacting risks are output geometry, alpha and GPU
composition; this is presentation work over hardware already imported in U1.

## Implementation And Verification

- `crates/tgpulse-core/src/loader/model1_bios.rs` supplies a neutral, ordered
  resource loader with size/SHA-1 validation. Each game's declared I/O filename
  selects its known firmware; a different board revision cannot replace it.
  Invalid candidates are skipped before trying the next path.
- `roms_db.rs` has a separate complete-game identification policy allowing
  external I/O firmware, while still requiring all game chips. The existing
  standalone identification/loading interfaces remain intact.
- `crates/tgpulse-libretro/src/diagnostic_overlay.rs` owns optional CGROM,
  activation notices and palette/straight-alpha composition. It reads native
  dot pixels, so no LCD hardware or blink implementation is duplicated.
- Hardware output reuses the existing final-coordinate foreground compositor;
  software draws after native foreground, before the timing panel. Active glyphs
  are opaque, background opacity is applied once, and native FE tile markers
  retain their opaque semantics. Timing draws last, with no automatic LCD displacement.
- Three always-visible Video options are registered through modern and legacy
  interfaces; stored values are `off`/`overlay`/`overlay_half`, four corners and 0–100% opacity.
  No Text rendering option exists. Font lookup is attempted on activation;
  toggling OFF/ON retries a previously missing resource. Reset/state
  restore do not replay the activation warning or change presentation settings.

Focused results:

1. **73 adapter tests pass**, including sparse-alpha versus software blending,
   normal/wide geometry, original timing avoidance, opaque glyphs, and one missing-font
   notice per activation.
2. **Seven focused native tests pass**: ordered resource validation, complete
   game identification with separately required I/O, and five LCD/state tests
   covering CGROM/CGRAM, cursor/blink, display shifts, CN6/EEPROM isolation and
   partial-transfer restore.
3. `tools/test_libretro_netmerc_lcd.py` passes through the actual macOS ABI:
   modern defaults/visibility, Off silence, queued missing-font notices,
   System/adjacent/game font loading, invalid-System fallback, external I/O
   with its chip absent from the game ZIP, required-I/O rejection, and native
   state/audio preservation when display changes. Visible LCD frames also replay
   identically after Save/Load.
4. Nine previously supported sets retain video/audio/Save RAM and identical
   state continuation against the pre-U7 macOS baseline, using 60-frame segments.
5. Real macOS RetroArch Software and Vulkan each deliver **240 frames**, exit 0,
   with screenshots showing LCD and Timing together. Vulkan exercises widescreen
   expansion and 2x supersampling. Software keeps its existing native pixel
   geometry and frontend widescreen presentation.
6. Linux Mesa **desktop OpenGL and GLES each deliver 30 hardware frames**,
   following 120 software warmup frames, with a context recreation at frame 15.
   Both render HD44780 in widescreen at 2x supersampling; audio and Save RAM
   hashes match between these runs. Mesa evidence is API/composition evidence,
   not GPU-performance or full-gameplay acceptance.
7. A separate real RetroArch **180-frame** fixture without the font shows the
   queued **Sega NetMerc Diagnostic Display: HD44780 BIOS Missing Or Invalid
   (hd44780.zip)** notification in actual pixels, with no LCD or Text substitute.
8. Native macOS format, 25 ABI exports, dependencies and three empty lifecycle
   cycles pass. The release Development core and matching info are installed;
   built and installed SHA-256 match.

### Original U7 Delivery Identities

- macOS core SHA-256:
  `de20048599d9b038d42ef53465bc7c7010128d570b407851929f090124bf3090`.
- Installed core:
  `~/Library/Application Support/RetroArch/cores/tgpulse_next_dev_m1_libretro.dylib`.
- Installed info SHA-256:
  `91ff93c2f5f1bd468bf88d66b73e3fe87066bbb596bbe37b41bc135f0021fdee`.
- Linux verification core SHA-256:
  `2caf5626f3bd4bdb31be3984a6d2c0f05711b56760d8c92dfe957e57590e0bfc`.

### Evidence And Reuse

Local evidence is under `/private/tmp/tgpulse-u7/`: `abi-final/report.json`,
`regression.json`, `retroarch-software/`, `retroarch-vulkan/`,
`retroarch-missing-bios-final/`, and `linux/{desktop,gles}/`.
These files are isolated implementation evidence, not a shipped ROM/sample
archive or user gameplay acceptance.

Reuse `tools/test_libretro_netmerc_lcd.py` with `--core`, `--rom`, `--bios-dir`
and a fresh `--output`. Extend existing frontend runs using
`tools/test_retroarch_gpu.py --diagnostic-display overlay --bios-dir ...` or
`tools/test_libretro_opengl.py --diagnostic-display overlay --system-dir ...`.
The latter system directory is the frontend root; BIOS ZIPs live below
`tgpulse-next/`.

The existing Colima/Rosetta builder and cached Rust/Mesa were reused. Source
and cached packages were synchronized explicitly without macOS build products;
ROM/BIOS test inputs remained only under `/tmp` in the disposable container.
Only evidence was retained/exported. The existing builder's Cargo registry
symlink targets `/work/source/registry`; source synchronization must preserve
or restage that cache before an offline build. No dependencies were installed.
An initial macOS explicit OpenGL attempt timed out and its owned process was
terminated: macOS 4.1 cannot execute this renderer's compute path. Successful
OpenGL/GLES delivery is established by Linux, not by that failed launch.

No state format, published version or release was changed. No commit/push was
performed. U9 release preparation remains a separate reviewed activity.

Account usage at completion: **37%** (36% before U7); next reset
**2026-10-10 09:49 CEST**. The account window is shared with other chats.

## Display Size And Layering Follow-Up

The user requested OFF / ON (100%) / ON (50%) in the existing selector,
without a separate scale option, and Timing above LCD at overlapping corners.
The current implementation preserves the full-size stored value and optional
BIOS activation policy. Switching between ON sizes does not reload the BIOS
or generate another missing-BIOS notice. Native LCD state and audio remain
independent of presentation. Shared composition supplies all three renderers.

Focused verification:

- Two targeted adapter tests pass: both sizes, four corners, native/wide output,
  opaque glyphs, zero-opacity backgrounds and sparse/opaque alpha equivalence.
  At the supported output widths, LCD dimensions change from 250 × 46 to
  125 × 23 pixels, with the same eight-pixel corner margin.
- The extended LCD ABI runner passes: exact modern labels/default, live OFF/full/
  half selection, no repeated BIOS notice between ON modes, native state/audio
  preservation and deterministic continuation in both sizes. Existing ordered
  resource fixtures remain valid.
- Real macOS RetroArch Software and Vulkan each exit 0 after 120 frames.
  Software shows the half-size LCD; Vulkan shows Timing above the full-size LCD
  at Top Left, with widescreen and 2x supersampling. Screenshots were inspected.
  OpenGL uses the same foreground composition order; its prior U7 evidence
  remains applicable, but OpenGL/GLES was not rerun for this size-only follow-up.
- The release artifact's native format, exports, dependencies and three empty
  lifecycle cycles pass. Development core/info installation is hash-verified.

Current macOS Development core SHA-256:
`76ed464bd5ba852ebdf3b35fc16c015820d0f513fc5e8cd0d39ff9e121ef6457`.
The installed info hash remains the original U7 hash above. Local evidence:
`/private/tmp/tgpulse-u7/display-size-abi/report.json`,
`display-half-software/` and `display-overlap-vulkan/`.

Effort: small; current reasoning level is adequate. Account usage remains 37%;
next reset 2026-10-10 09:49 CEST. No commit, push or publication.
