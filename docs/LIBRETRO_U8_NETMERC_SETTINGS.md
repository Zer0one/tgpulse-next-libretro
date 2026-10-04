# U8: Reviewed NetMerc Automatic And Operator Settings

## Approved Selection And Reference

The user approved the U3 workbook on 2026-10-04. NVRAM Settings exposes Game
Difficulty, Country and Advertise Sound in native menu order. Coin Chute #1
remains captured evidence and is not exposed. Automatic Initial NVRAM Setup
selects Country = Export, native defaults for other operator fields, and the
previously approved controller endpoint policy. Existing frontend saves win.

SM2 `PORTING_PLAN.md` section 3.6 supplies complete native-sample initialization,
reviewed override fields and valid-save precedence. Supermodel's
`apply_initial_nvram_settings` keeps country/operator updates separate from
initial analog normalization. The existing adapter already registers modern
and legacy options, applies live selections and resets native caches. Reuse it.

NetMerc's smallest necessary adaptation is backup RAM storage. Its three
approved fields use the same generated Field/Value table, including Difficulty's
native dependent byte. A separate generated BackupTemplate reuses the existing
RLE decoder and payload CRC, without borrowing another game's EEPROM integrity
algorithm. Shared hardware and standalone code are unchanged.

## Startup And Save Precedence

1. Import a valid same-set frontend Save RAM first. Skip automatic setup when
   import succeeds, including controller normalization and Country = Export.
2. When no valid frontend save exists and automatic setup is Enabled, preserve
   the loaded optional `netmerc_nvram.bin` image and patch only approved Country
   and controller endpoints. Existing loader ROM-repair policy still applies.
3. When the optional file is absent, initialize from the independently saved
   and reloaded native default sample, then apply those approved startup patches.
4. When automatic setup is Disabled, retain the loader's normal native fallback
   or optional seed. No automatic template or controller/country patch is applied.

Optional-file presence is checked in the archive inventory. Image length alone
cannot distinguish a shipped seed from the loader's synthetic missing-file
fallback; both contain 64 KiB. No personal configuration or saves are modified.

## Operator Options

| Setting | Values | Default |
| --- | --- | --- |
| Game Difficulty | Easy, Normal, Hard, Hardest | Easy |
| Country | Japan, USA, Export | Export |
| Advertise Sound | Off, On | Off |

Keys use `tgpulse_next_nvram_netmerc_` followed by `difficulty`, `country` and
`advertise`. Labels use Title Case and raw selector identifiers retain native
encodings. The general NVRAM Settings toggle remains always visible and defaults
to Disabled. Reviewed per-set fields use the existing expressly agreed display
filtering. Enabling overrides applies selected values, including native defaults;
those explicit selections take precedence over automatic startup choices.

The Country selector defaults to Export, matching its approved automatic
override. Japan remains the observed native default in the catalogue and review
workbook. Every approved automatic override also defines its corresponding
operator-selector default across all Model 1 sets; fields without an automatic
override retain their native default. The generator derives both paths from
the same reviewed startup policy.

Updates modify only the photographed selector/dependent bytes in backup RAM.
They require only the complete 64 KiB buffer; the four bookkeeping bytes are
neither an operator-layout signature nor a condition for applying settings.
EEPROM, coin/credit fields, bookkeeping and
controller calibration are preserved. Changed selections follow the existing
reset path to refresh native caches. Save State and explicit reset preserve
native data; no state-format change or legacy-state migration is introduced.

## Generation And Verification

The approved workbook and NetMerc YAML drive `tools/generate_model1_nvram.py`.
It reconciles all 973 native samples, including 20 NetMerc values; output contains
43 reviewed fields across 10 sets, nine EEPROM templates and one backup template.
The Excel records approval and implementation while preserving its tables,
filters, green style and bold native defaults.

Verification passes:

- 60 adapter tests, including missing/loaded seed behavior, every approved
  backup field encoding, buffer-size validation, and unrelated-byte
  preservation; existing nine-template EEPROM tests remain intact.
- Existing NVRAM ABI runner covers the nine prior sets plus all nine NetMerc
  selector values, automatic Export, same-set save precedence, optional seed,
  disable behavior, visibility, reset and Save State preservation.
- Nine-set 120-frame comparison against the U1a Development baseline retains
  identical video, audio, Save RAM and serialized state continuation.
- Offline locked macOS release build and Model 1 artifact/dependency/lifecycle
  gates pass.
- Two isolated RetroArch Vulkan runs at 480 frames exit successfully. Fresh-save
  startup produces Easy / Export / Off and approved initial endpoints. Manual
  overrides on an imported sample produce Hardest / USA / On, retaining its
  coin and calibration bytes. Container-validated frontend saves are imported
  into fresh ABI instances and their native Game Assignments screens confirm
  those values. This establishes settings delivery, not gameplay/controller proof.
- Development core/info installed in local RetroArch; installed core matches
  build SHA-256:
  `414d8c5d41723814b84905d1426d390d1951994e5cd25ac7cfcd6978caf29c38`.

Local evidence: `/private/tmp/tgpulse-u8/` (`tests.log`, `abi3/result.json`,
`regression-final.json`, `artifact-final.log`, `retroarch-auto`,
`retroarch-manual`, fresh menu readbacks and `deployment.json`). The optional
seed verification uses a local fixture with validated calibration and sentinel
bytes; it does not claim a new test of the original factory dump. No ROM/save
fixtures are committed. No commit, push or release is part of U8.

Required reasoning: High. Effort M; estimate 1–3 account percentage points.
Shared account use: 33%, up one rounded point from 32%, including other chats.
Reset: 2026-10-10 09:49:17 CEST.

## Follow-Up: Personal Save And Notification Correction

The initial implementation incorrectly required the campaign's first four
bookkeeping bytes before applying NetMerc operator options. A valid user save
with `FF FF FF FF` bookkeeping and native operator values exposed that error.
The guard is removed, without replacing it with another content signature.
Only buffer size is required; patches preserve all bytes outside the selected
fields. Tests include zero, erased and campaign bookkeeping prefixes.

Initialization remains separate: the supplied `.bin` startup patches apply only
when no valid frontend save exists and the seed is actually needed. Automatic
setup never patches an existing valid save. Explicit NVRAM Settings overrides
remain opt-in and follow the approved selector behavior. A read-only copy of
the user's same-set save verifies exact preservation before enabling overrides,
coin/calibration/bookkeeping preservation during overrides and no layout warning
after 1,251 native frames. Original user data is not modified.

The loader audit also moved factory bookkeeping initialization out of Libretro
ROM loading: `load_model1_zip_with_deferred_nvram` retains the full raw seed, and
`initialize_netmerc_nvram_seed` is called only after frontend import reports no
valid save and the optional seed was actually supplied. It retains the upstream
whole-image SHA-1 policy for the recognized factory dump. Explicit overrides,
reset and state restore do not call it. Absence of the optional `.bin` is silent;
Automatic Initial NVRAM Setup supplies its own complete baseline when enabled.
Native loader tests cover deferred/needed initialization and preservation of
calibration and bytes outside bookkeeping. The ABI runner additionally verifies
the known factory seed with Auto enabled/disabled and existing-save precedence.

Final correction evidence: `/private/tmp/tgpulse-notices/loader-tests.log`
(8 passed; one unrelated ROM-dependent test ignored), `tests-final.log`
(62 adapter tests), `nvram-final/result.json`, `audio-final/result.json`,
`artifact-final.log`, `retroarch-off-final/frame.png` and `deployment-final.json`.
Installed Development core matches SHA-256
`e15d4c48144051280dced1b6941f59a8acd3d00026865b2500431b9efe89907b`.

## Follow-Up: Automatic Overrides Define Selector Defaults

The generator now derives every operator-selector default from the approved
automatic startup override when one exists, using the native default otherwise.
This corrects Country to Export for VF, VFormula and NetMerc, and Cabinet to
Special for VR. Other approved startup values already equal native defaults.
The same field default drives modern registration, `(Default)` labels, legacy
ordering and missing-variable fallback. Documentary native defaults remain
unchanged; saved explicit frontend choices retain precedence.

The existing ABI runner now enables NVRAM Settings without supplying individual
choices and checks the resulting persisted values against the reviewed policy
for all ten sets. Existing explicit-value, save-precedence, optional-seed,
calibration, reset and Save State checks also pass. The option-registration test,
973-sample generator audit, locked offline release build and Model 1 artifact
gate pass. These checks establish ABI behavior, not a new gameplay trial.

Evidence: `/private/tmp/tgpulse-nvram-defaults/` (`abi/result.json`, `options.log`,
`build.log`, `artifact.log`, `deployment.json`). The installed Development core
matches build SHA-256
`a5fe73af82b149e163fc23e31680aa4d1d40ef195b13c6e1e2dc51a2bd0aa016`.

## Diagnostic Follow-Up: Native Backup Data Clear Comparison

On 2026-10-04, the user reported an apparently persistent credit. A fresh
isolated ABI instance imported the exact `difficulty-0/saved.srm` campaign
sample used to generate the backup template. Both automatic setup and operator
overrides were disabled. Test/Service inputs selected Backup Data Clear,
confirmed Yes, exited the completed-clear page and root menu, and saved the
result. A separate fresh instance reloaded that result.

| Image | Credit Word At Backup Offset `0x18` |
| --- | --- |
| Campaign Sample Used For The Template | `FFFF` (65,535) |
| After Native Clear And Menu Exit | `0000` |
| After Fresh Import And 1,200 Frames | `0000` |
| Known Optional Factory Seed | `FFFF` (65,535) |

Game Difficulty, Country, Advertise Sound, coin settings (`30..37`), controller
endpoints (`38..47`), EEPROM and already initialized bookkeeping banks/checksum
(`1000..3003`) are unchanged by this comparison. The immediate capture differs
at `04..05` (elapsed runtime counter), `18..1B`, `1E..1F`, `48..5B` and `60..6B`.
These observed ranges are evidence, not a proposed blanket live-save patch.

The actual game ZIP has no optional seed, so the embedded campaign baseline
is independently responsible for uninitialized credits in that startup path.
The recognized factory seed (SHA-1 `411134c1e6307f2e32c3b4b372597b45b14a9834`)
contains the same erased credit word; the current initialization repair touches
only the bookkeeping banks/checksum and selector and leaves that word intact.
Container validation and reviewed operator-value validation therefore did not
establish a fully initialized credit/session baseline.

Evidence: `/private/tmp/tgpulse-netmerc-clear-compare/comparison.json`, isolated
recipes, `committed/manifest.json`, `committed/cleared.png` showing RAM Now Clear,
and `reload/manifest.json` plus the normal Service Menu screenshot. This is a
diagnostic comparison only: no core correction, personal save mutation, build,
deployment or publication was performed in this activity.

## Credit Initialization Correction

Following approval, the optional recognized factory seed now also initializes
its 16-bit credit word at `18..19` to zero. Its full-image SHA-1 guard and
deferred-save-import policy are retained. No other seed bytes beyond the
existing selector/bookkeeping repair and this credit word are changed. Valid
frontend saves, explicit resets, Save State loads and manual operator updates
do not invoke this initialization repair.

The archived `initialized-baseline/saved.srm` under the NetMerc campaign now
contains the native Backup Data Clear sample, captured after menu exit, with
cold-reload evidence, recipes, screenshots and manifests alongside it. The
generator uses this explicit initialized baseline and checks zero credits and
all native operator defaults. Historical single-field variations remain intact
as encoding evidence. The embedded backup template is regenerated from the
corrected sample; approved Export and controller startup patches are unchanged.

Verification passes four targeted loader tests, all 62 adapter tests, the
973-sample generator audit and the ten-set ABI runner. The ABI runner checks
both seed Auto modes, zero credits in the automatic template, and byte-exact
preservation of an isolated existing save with nonzero credits even when a
factory seed is available, including reset. Locked offline macOS release build
and Model 1 artifact gates pass. Evidence is in
`/private/tmp/tgpulse-netmerc-credit-fix/`; the installed Development core matches
SHA-256 `b69890cc85b6ab69ee38e9e81f913d0eacb1c549e7f4270f7a389ebedf44097e`.
Personal saves are untouched; an existing save retains its existing credit word
until the user performs the native clear. No commit, push or release is included.
