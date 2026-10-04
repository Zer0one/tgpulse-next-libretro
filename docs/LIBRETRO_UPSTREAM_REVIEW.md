# Upstream Alignment Review — 2026-10-04

## Compared Revisions

- Imported baseline: `3c75f9b2b155e8bb50a225c69520948ef72ecd48` (TGPulse-Next 0.1.0.0).
- Fetched `upstream-next/main`: `f303b712455571b60cbea2249e1022935d398e22`, also the peeled upstream
  `0.1.0.1` tag at review time.
- Current port HEAD: `bc58710`; published Libretro version: `0.1.0.2`.
- The local Timing panel, Widescreen Hack, option-order and Monitor/workbook
  changes are uncommitted and must be preserved during any integration.
- The initial inspection only fetched refs. U1 was subsequently authorized and
  integrated locally; see [U1 evidence](LIBRETRO_U1_INTEGRATION.md). Upstream and
  port histories remain independent.

## Source Changes

Four upstream commits change 59 files (7,654 insertions, 193 deletions):

| Commit | Content |
| --- | --- |
| [`27db9fa`](https://github.com/Zer0one/TGPulse-Next/commit/27db9fa58aa631517dd9f7538a4d99efca2e0214) | NetMerc machine, I/O, tracking peer, audio substitution and DSP arithmetic. |
| [`e60e53f`](https://github.com/Zer0one/TGPulse-Next/commit/e60e53f7da49795ba349b1a6d1c0455657081f2f) | Desktop MVD input, motion helpers, LCD presentation and output controls. |
| [`a5be47d`](https://github.com/Zer0one/TGPulse-Next/commit/a5be47d) | Third-party notices in desktop release archives. |
| [`f303b71`](https://github.com/Zer0one/TGPulse-Next/commit/f303b712455571b60cbea2249e1022935d398e22) | Repository documentation, NetMerc acceptance and 0.1.0.1 release notes. |

The upstream release records standalone user acceptance of tracking, LCD,
rumble and substitute audio. It is source-project evidence, not proof that
those functions work in this Libretro adapter. Extended later-level gameplay
remains an upstream limitation; no complete hardware-accuracy claim is made.

## Emulation And Resource Contracts

- NetMerc gains its own `model1board::Kind`, advanced TMPZ84C015 I/O firmware
  `epr-18021.6`, clocked CN7 tracking peer and cold-boot forward pose. The peer
  implements the serial protocol; it does not emulate a Polhemus i386SX or
  magnetic sensor. The new firmware is added to the canonical ROM database
  and its generator, not supplied as a ROM in this repository.
- Finite arithmetic is implemented in the shared MB86233 DSP and is available
  to any owner selecting `FloatMode::Finite`. Rechecked on 2026-10-04: published
  `upstream-next/main` remains `f303b71`; both that revision and the standalone
  checkout currently select Finite in `Model1System::new` only inside the
  `Kind::NetMerc` condition. The enum default is IEEE and other boards retain
  that selection. Implementation scope and activation scope are distinct.
  The user confirmed on 2026-10-04 that the Libretro integration must retain
  conditional activation for `Kind::NetMerc` for now. The separately selectable City Workaround is a NetMerc-specific graphics
  correction, planned separately in U1a rather than U2 controls. It defaults On and
  changes conversion at firmware instruction 0x02e1; it is live, does not
  rewrite ROM/RAM or reset the machine, and is reapplied after state restore.
- The loader initializes bookkeeping only in the SHA-1-identified NetMerc
  ROM-set default SRAM image. The port additionally makes `netmerc_nvram.bin`
  optional and supplies normalized controller endpoints through Automatic
  Initial NVRAM Setup only when no valid personal save exists, following
  Supermodel lightgun initialization. Existing Save RAM, calibration and state imports
  are not repaired implicitly. This is distinct from the current selectable
  315-5711 bad-dump repair and from reviewed Automatic NVRAM Settings.
- Procedural audio replaces the missing sample output when NetMerc PCM1's
  descriptor table is all FF. Optional VF/VR/SWA/Wing War donor banks override
  it at load time. Donor Off means no donor; it does not disable the procedural
  fallback. Strict donor loading needs PCM regions only, and applies both banks
  atomically without altering NetMerc's sound program or archive.
- Audio Donor defaults to VF. Missing/invalid donor resources leave the fallback
  intact. Alternative Audio Gains defaults Off; enabled coefficients use the
  actually loaded path, preserve manual gains and master volume, and apply live.
  The user classifies its Libretro selector as an eventual enhancement, not
  required U4 work.
  Substitute audio is not authentic recovery of the missing ROM content.
  U4 now exposes the donor through an always-visible Audio option, searches
  game ZIPs only beside NetMerc, and delivers notifications only for fallback,
  procedural audio and incomplete original audio. Successful donor loading is
  silent. See [U4 integration evidence](LIBRETRO_U4_AUDIO.md).

## State And Local Adaptation Boundaries

The complete machine envelope moves from version 1 to version 4. Sound snapshots
move from version 1 to version 2; advanced I/O snapshots add board identity,
LCD and serial tracking state. Resource identity adds the procedural-audio flag.
Direct adoption rejects prior format-1 states, including non-NetMerc titles;
Save RAM remains a separate persistent format. No legacy state migration exists
in this upstream delta. The user explicitly confirmed adoption without legacy
state migration on 2026-10-04. Communicate the incompatibility when shipping;
Save RAM is not converted or discarded by this state-format change.

The later approved local startup publication correction serializes first-record
readiness and moves the current envelope to format 5, rejecting older states
without migration. See [U5 correction and verification](LIBRETRO_U5_SENSORS.md#approved-startup-publication-correction--2026-10-04).
This is a local follow-up, not part of the reviewed upstream snapshot.

Changed upstream files that also contain committed local port adaptations:

- `.github/workflows/release.yml`
- `README.md`
- `crates/tgpulse-core/src/loader.rs`
- `crates/tgpulse-core/src/roms_db.rs`
- `crates/tgpulse-core/src/sound.rs`
- `docs/MODEL1_ROADMAP.md`
- `docs/RELEASING.md`
- `docs/releases/0.1.0.1.md`

These overlaps require a reviewed patch, not wholesale file replacement:
retain Model 1/Model 2 feature gates, OUT_DIR-generated scoped catalogue,
strict Libretro content identification, explicit bad-dump-repair policy and
Model 1 audio helpers. New `Model1Roms` fields also require fixture/reset updates
in the adapter. LCD/tracking/audio helpers must follow Model 1 feature gates.
The shared DSP stays available to relevant Model 2 builds without enabling the
NetMerc arithmetic policy for them.

## Frontend Contracts

- NetMerc cabinet joystick Y now follows the explicit standalone inversion:
  up maps channel 2 to 00, center 7F, down FF. Its left-stick sampling no longer
  adds the desktop 15% dead zone. Libretro must keep frontend dead-zone/remap
  ownership and maintain other flight profiles unchanged.
- MVD pose is a separate P1 right-stick/sensor function, not a second native
  cabinet ADC. U2 adopts upstream's Start/D-pad Down Holder
  aliases. Multiple physical aliases must share one native action label.
- MVD Auto/Off/Right Stick/Sensors, 30/20-degree initial stick ranges and
  Auto/Manual Holder are frontend policies. The bounded Holder helper uses
  emulated frames and the existing read-only game context. The user classifies
  automatic Holder activation as an eventual enhancement; U2 retains the
  manual native Holder input and aliases.
- `motion.rs` is independent of SDL and takes explicit samples/timestamps.
  Libretro's sensor interface exists in the reference header, but this adapter
  has no sensor ABI yet; availability depends on the frontend/input driver.
  Polling does not expose the original sensor event timestamps. Units, axes,
  sampling cadence, calibration and fallback therefore require an explicit
  adaptation rather than copying SDL event handling. Calibration/recenter are
  host commands, not additional arcade input bits. The user explicitly requires
  both as remappable P1 virtual inputs in U2: MVD Calibrate uses North
  (RetroPad X), MVD Recenter uses West (RetroPad Y), matching upstream.
  Each press emits one command; U5 connects them to the sensor handlers.
- NetMerc motor output is port-D bit 2, distinct from LCD bus traffic. Upstream
  observes pulse writes at the I/O boundary; a Libretro adapter must preserve
  bounded pulses rather than relying only on the final frame latch. Existing
  VR/VFormula rumble remains On by the user's choice. Intensity defaults 100%.
- Diagnostic Display defaults Off. An optional in-frame LCD presentation can
  reuse hardware state, text/dot pixels and validated CGROM. Dedicated windows,
  fullscreen switching and SDL rendering are desktop concerns. This LCD feature
  does not expand the NVRAM campaign into unrelated diagnostic/test pages.

## References And Exclusions

Current SM2 `PORTING_PLAN.md` sections 0–2 and 7 were read for selective imports,
CPU/audio/state boundaries and explicit host exclusions; sections 3 and 6 supply
controls, NVRAM and output conventions. Supermodel `Docs/ROADMAP.md` supplies
source-menu order, clone evidence, native-default policy and review sequencing.
Reference options do not implement NetMerc MVD, donor audio or its diagnostic LCD;
use the actual standalone helpers for those functions.

Keep the existing Libretro CI/package/license workflow. Update the applicable
MAME notice for the LCD implementation; do not replace the core workflow with
upstream desktop packaging. Library filtering, desktop debugger/CLI, SDL device
selection, auxiliary windows and desktop configuration files are not Libretro
features. Preserve standalone buildability when shared APIs change. Metal and
sRGB remain excluded as instructed; this delta creates no requirement to add
them. Full Polhemus emulation, speculative arithmetic changes for other games
and corrected-audio-ROM acquisition are not alignment requirements.

The only active implementation order and estimates are in
[LIBRETRO_ROADMAP.md](LIBRETRO_ROADMAP.md).
The original review did not change emulator code or installed binaries. U1 is
now integrated and installed locally; later activities remain pending in the
single roadmap. No release was published by U1.

U2 frontend adaptation is documented in [NetMerc Controls](LIBRETRO_U2_CONTROLS.md),
including approved labels, ADC and MVD polarity, live stick modes/ranges and the
virtual-command boundary awaiting U5 sensors.
