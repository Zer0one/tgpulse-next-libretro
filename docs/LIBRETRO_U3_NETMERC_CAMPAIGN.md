# U3: NetMerc Operator-Setting Campaign

## Reference And Scope

This extends the existing Model 1 campaign using SM2-Emu's photographed
inventory, isolated TOML recipes, native save/exit and fresh-import workflow.
Supermodel's `Docs/ROADMAP.md` requires observed menu values before sample
claims and keeps operator acquisition separate from gameplay acceptance.
Unlike its narrower supplemental campaign, this project's agreed scope captures
all settings, including those not proposed as Core Options.

The current Model 1 catalogue contains one NetMerc parent and no declared
NetMerc clone. The installed Development candidate identifies and loads the
available complete ZIP. The optional `netmerc_nvram.bin` is absent from that
ZIP; acquisition disables both Automatic Initial NVRAM Setup and NVRAM Settings.
Source histories remain separate. Other games' existing evidence is preserved.

## Authoritative Inventory

| Menu | Setting | Observed Values In Cycle Order | Native Default |
| --- | --- | --- | --- |
| Game Assignments | Game Difficulty | Easy, Normal, Hard, Hardest | Easy |
| Game Assignments | Country | Japan, USA, Export | Japan |
| Game Assignments | Advertise Sound | Off, On | Off |
| Coin Assignments | Coin Chute #1 | 1–9, No Use, Free Play | 1 |

All four cycles have documentary screenshots including return to the initial
value. There are 20 discrete values. No networking selector is present in the
observed operator pages. Controller Unit Test supplies four continuous endpoint
slots, acquired separately. Its Motor row is a diagnostic action; MVD Test's two
pages report tracking diagnostics, not persisted operator settings. No unrelated
memory, audio, output, bookkeeping or CRT diagnostic campaign was added.

Catalogue: `data/diagnostic-menus/netmerc.yaml`. Curated documentary images:
`docs/diagnostic-evidence/netmerc/`. Raw samples, survey captures, logs and
manifests remain ignored under
`validation/nvram-campaigns/2026-10-04/netmerc/`.

## Recipes And Commit Path

`tools/generate_model1_netmerc_recipes.py` reads the photographed YAML through
the same standard Ruby YAML reader used by existing Model 1 generators. It
produces 20 single-value recipes, four per-field reload recipes, a baseline and
a full-range controller sample/reload pair. Acquisition uses the existing
`tools/libretro_nvram_capture.py`; no alternate capture implementation is used.

Every discrete variation imports the same immutable native baseline. Service
selects a row and Test changes discrete values. Exit from Game Assignments
returns to its selected root row; five Service presses reach root Exit. Coin
Assignments requires four. Test on root Exit returns to game execution before
the final saved sample. Repeating Test without navigating root Exit would reopen
the page; initial exploratory captures are not treated as final presets.

Controller calibration uses Service to select an endpoint and Thumb Button
(Btn-East/A) to capture its live ADC value. Test leaves the page. From the last
endpoint, two Service presses reach page Exit, including the Motor diagnostic
row; six further Service presses reach root Exit. Full left-stick travel yields
Left 00 / Right FF / Up 00 / Down FF, confirmed after a fresh import.
These observed profile endpoints are not a universal cabinet preset. The already
approved automatic initial-calibration policy remains unchanged pending review.

## Storage And Integrity

NetMerc stores these settings in backup RAM, while its 128-byte I/O EEPROM
remains erased. Reusing the other titles' EEPROM CRC would be incorrect.

| Field | Backup RAM Offsets | Native Encoding |
| --- | --- | --- |
| Game Difficulty | 0x28, 0x2A | Easy 2/0; Normal 4/1; Hard 6/2; Hardest 8/3 |
| Country | 0x5C | Japan 0; USA 1; Export 2 |
| Advertise Sound | 0x2C | Off 1; On 0 |
| Coin Chute #1 | 0x30, 0x34, 0x6C | All 11 tuples recorded in YAML; No Use and Free Play also change mode at 0x30 |
| Controller Endpoints | 0x3C, 0x38, 0x40, 0x44 | Four native slots; acquired and reloaded independently |

Across the 20 discrete variations, only mapped field/dependent bytes and the
running counter at 0x04/0x05 change. Native firmware retains all photographed
values after import into a fresh instance. The frontend container CRC validates
every sample and reload independently. No separate native backup checksum is
claimed or guessed. The audit verifies primary fields, dependent encodings,
unrelated field preservation, imported sample hashes and screenshot regions.

## Review Workbook And Proposals

The existing `docs/model1_core_options_review.xlsx` now includes four NetMerc
rows in Service Menu order, a calibration record and updated campaign/automatic
records. Existing 110 operator rows remain unchanged. All four tables retain
filters, green `TableStyleMedium8` and frozen headers. Native defaults are bold
within Observed Values; there is no separate default column.

Proposals follow existing reference selections:

- NVRAM Settings: Game Difficulty, Country and Advertise Sound.
- Coin Chute #1: fully acquired, not proposed, matching the references' general
  exclusion of coin/credit selectors.
- Automatic setup: Country = Export; other discrete operator fields retain
  native defaults. Preserve the existing approved controller policy, optional
  seed handling and valid-user-save precedence.

The user approved these proposals on 2026-10-04. U8 now implements the three
operator selectors and automatic policy; see [U8 integration](LIBRETRO_U8_NETMERC_SETTINGS.md).

## Verification And Reproduction

`tools/audit_model1_netmerc_campaign.py` passes four fields, 20 saved/reloaded
values and full-range calibration. Selected and fresh-import screenshot regions
match the photographed inventory; YAML encodings record both selector bytes and
native dependent bytes. First and subsequent boot baselines are retained.
The complete source inventory now covers 10 sets, 114 fields and 973 values.

```sh
python3 tools/generate_model1_netmerc_recipes.py /private/tmp/netmerc-recipes
# Run each recipe through tools/libretro_nvram_capture.py with isolated output.
# For variations, --load points to the same baseline/base.srm.
# For reloads, --load points to that variation's final saved.srm.
python3 tools/audit_model1_netmerc_campaign.py \
  --evidence validation/nvram-campaigns/2026-10-04/netmerc \
  --output validation/nvram-campaigns/2026-10-04/netmerc/audit.json
ruby tools/audit_model1_campaign_inventory.rb --source-only
```

Evidence records the core/ROM SHA-256, complete recipe, frame timings, option
state and import hashes. `campaign-source-state.json` records source provenance.
This is direct ABI acquisition and native menu readback, not real RetroArch,
gameplay or hardware calibration acceptance. No emulator production code changed
in U3, so no new core build/deployment is required. No commit/push/publication.

Required reasoning: High. Original effort estimate: L, 3–7 account percentage
points; actual inventory is four discrete fields plus controller calibration.

Shared account usage at completion: 32% (unchanged rounded reading; exact phase
consumption unavailable). Next reset: 2026-10-10 09:49:17 CEST.
