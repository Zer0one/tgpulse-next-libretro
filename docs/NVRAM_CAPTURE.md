# Repeatable Model 1 NVRAM acquisition

## Reference and adaptation

This follows SM2-Emu's isolated sample/recipe workflow. The acquisition backend
is the existing direct ABI probe, consolidated into a tool in this repository.
TOML recipes use Python's standard library without installing a parser.
Frame steps replace wall-clock waits. Every capture includes Save RAM, a PNG,
hashes, elapsed time, frame number and the complete recipe in a manifest.
The core and ROM hashes identify the exact inputs even with uncommitted builds.

## Usage

Requires Python 3.11 or newer and an already built native core. Use a fresh output
directory outside the checkout. Content is read only; the tool never writes into
RetroArch's saves or changes an installed core.

```sh
python3 tools/libretro_nvram_capture.py \
  --core /absolute/path/tgpulse_next_dev_m1_libretro.dylib \
  --rom /absolute/path/vr.zip \
  --recipe tools/nvram-recipes/vr-baseline.toml \
  --output /private/tmp/model1-vr-capture --dry-run
```

Remove `--dry-run` to acquire the samples. Add `--load /absolute/path/sample.srm`
to import a validated sample before the first frame in a fresh instance.
Each invocation unloads/deinitializes its own core when it completes or raises
a Python exception. Frame counts are bounded; native crashes/hangs require the
caller to stop its own process and retain the partial output for diagnosis.

## Recipes

Actions are TOML `[[actions]]` tables with either `frames = 20` and optional
`buttons = ["L3", "B"]`, or `capture = "menu-start-credit"`.
Buttons remain held for every frame in a step.
The next frame step replaces the button set; a step without `buttons` releases
all buttons. Capture actions do not advance time. Symbolic names follow RetroPad.
The capture host explicitly disables Automatic Initial NVRAM Setup and NVRAM
Settings so acquired values come from the native service menu. The manifest
records these option values.
Model 1 Test is L3 and Service is R3; do not copy the reversed SM2 bindings.
For calibration, frame actions may also contain an `analog` TOML table with
`left_x`, `left_y`, `l2` and `r2`, each a signed 16-bit Libretro input value.
Analog inputs also reset at the next frame action unless specified again.

Create a baseline per set, then use the screenshot-derived catalogue and review
workbook to define separate recipes for single-setting variations.
Set the recipe's `set` to the exact ROM filename stem. Start every variation
from the same baseline. Store recipes in `tools/nvram-recipes`; store binary
samples and screenshots outside Git. Observed value labels go into diagnostic
YAML only after inspecting screenshots. Recipe names never establish a value.

The VR baseline recipe captures boot frames 120 and 600. It deliberately makes
no menu-setting claim. The nine `*-saved.toml` recipes are **one verified
non-default setting per starting set**, retained as navigation and commit
examples. They are not the full variation campaign. Intermediate captures in
these recipes document menu state; only the final `saved`, `after-exit`,
`after-second-exit`, `after-yes` or `committed` capture is a candidate for a
persistent setting.
Container CRC validation proves transport integrity only; native EEPROM
checksum and game-owned settings still require independent validation.

### Game System recipe expansion

The screenshot-derived `vr.yaml` and `vformula.yaml` can be expanded into
one isolated recipe per observed Game System value without installing a YAML
package. Ruby's standard YAML library is used only to generate TOML; the
capture tool still uses Python's standard TOML parser:

```sh
ruby tools/generate_model1_game_system_recipes.rb vr /private/tmp/vr-game-system-recipes
ruby tools/generate_model1_game_system_recipes.rb vformula /private/tmp/vformula-game-system-recipes
```

The generator checks set identity, contiguous menu positions, EXIT position,
closed value cycles and default values. It produces 63 VR and 59 Virtua Formula
sample recipes, plus 10 and 9 field-specific `--verify--` recipes for fresh
imports. Each sample recipe starts a fresh core, selects one field, changes it by the number
of steps documented in YAML, uses the game's native YES (SAVED) path, and
captures the selected menu and committed Save RAM. The generated recipes are
an acquisition plan; the count does not claim 122 saved or reloaded values.
Run each through `libretro_nvram_capture.py` with a separate output directory,
then import its `saved.srm` in a fresh instance with the corresponding
`--verify--` recipe and inspect the field before marking it persistent.
Generated recipes and raw samples stay outside Git. The two audits compare
screenshots, native CRC, fresh-load import hashes and YAML byte mappings:

```sh
python3 tools/audit_model1_game_system_campaign.py --set vr \
  --recipes validation/nvram-campaigns/2026-10-01/vr/recipes \
  --samples validation/nvram-campaigns/2026-10-01/vr/samples \
  --reloads validation/nvram-campaigns/2026-10-01/vr/reloads
ruby tools/audit_model1_game_system_yaml.rb vr \
  validation/nvram-campaigns/2026-10-01/vr/samples
```

Run the same checks with `vformula` and its three separate output paths.

### Virtua Fighter recipe expansion

`vf.yaml` also expands from the screenshot catalogue into 98 isolated values
across Game Assignment, Coin Assignment and nested Manual Setting:

```sh
ruby tools/generate_model1_vf_recipes.rb /private/tmp/vf-recipes
```

The generator checks all 98 documentary screenshot paths, menu positions,
closed value cycles and native defaults. It writes one sample recipe per value
and 13 field-specific fresh-load recipes. Game and Coin save on their own EXIT.
Manual returns first to Coin; the second EXIT from Coin commits the EEPROM.
Leaving Manual alone produced unchanged EEPROM bytes in the pilot. Do not
press Game Assignment INITIALIZE; it is an action, not a setting. Audit local
captures and the YAML byte map with:

```sh
python3 tools/audit_model1_vf_campaign.py \
  --recipes validation/nvram-campaigns/2026-10-01/vf/recipes \
  --samples validation/nvram-campaigns/2026-10-01/vf/samples \
  --reloads validation/nvram-campaigns/2026-10-01/vf/reloads
ruby tools/audit_model1_vf_yaml.rb \
  validation/nvram-campaigns/2026-10-01/vf/samples
```

### Star Wars Arcade and Wing War recipe expansion

The six independently documented parent/clone YAML catalogues expand with:

```sh
ruby tools/generate_model1_assignment_recipes.rb swa /private/tmp/swa-recipes
ruby tools/generate_model1_assignment_recipes.rb swaj /private/tmp/swaj-recipes
ruby tools/generate_model1_assignment_recipes.rb wingwar /private/tmp/wingwar-recipes
```

Repeat for `wingwaru`, `wingwarj` and `wingwar360`. The generator checks each
set's own value cycles and documentary screenshots. It adapts the root menu
positions, nested Manual exit route, R360 Coin page and Wing War Communication
page. Star Wars Manual returns to Coin with MANUAL SETTING selected: press
Service/R3 once, then Test/L3 on Coin EXIT. Wing War requires an additional
root EXIT; its native EEPROM is committed only after that exit. These routes
were separately piloted and freshly reloaded for the parent and every clone.

For Star Wars, exiting without a change can leave EEPROM virgin even though
the menu displays the default after reload. Create an initialized default
through the game's own save route: save ADVERTISE SOUND changed, import that
sample, restore the native value, save again, then use the resulting sample as
the clean base for isolated default recipes. Audit native CRC and the EEPROM
mirror; do not accept the virgin EEPROM as an automatic NVRAM template.

Wing War's VOLUME SETTING is a continuous three-axis calibration procedure.
The four `*-volume-calibration.toml` recipes move the virtual stick and
throttle through their full ranges, then hold Test on SET & EXIT and leave
the root Test menu. Neutral-axis SET samples were rejected on fresh load;
full-sweep samples persisted for all four sets. Use the matching
`*-volume-calibration-reload.toml` recipe and run
`python3 tools/audit_model1_wingwar_calibration.py` to check native CRC,
mirrored bytes, import hashes, saved endpoints and fresh-load EEPROM identity.
These samples document the procedure, not a reusable preset for another
controller or cabinet.

## Native commit and fresh-instance gate

The direct ABI capture tool exports the current Save RAM even while a Test
menu is open. A changed byte in that file is **not** proof of a persistent
operator setting. In representative tests, the menu-open samples for `swa`,
`wingwar`, `vf` and `vformula` all reloaded to the native default. Keep such
samples as documentary diagnostics. Produce variation samples only after the
game's own save path, then import each candidate in a fresh core and inspect
the same selected field.

| Family | Observed commit path after changing one setting | Fresh reload proof |
| --- | --- | --- |
| Star Wars Arcade | Select Game or Coin EXIT with Service/R3, press Test/L3; from Manual exit to Coin, advance once from MANUAL SETTING to Coin EXIT, then press Test/L3 | `swa` 92/92 and `swaj` 90/90 Game/Coin/Manual values redisplayed from committed Save RAM |
| Wing War | Select page EXIT with Service/R3, press Test/L3, then leave the root Test menu; Manual first returns through Coin EXIT | All 551 Game/Coin/Manual/Communication values redisplayed; full-range calibration verified separately |
| Virtua Fighter | Select Game or Coin EXIT with Service/R3 and press Test/L3; from Manual select its EXIT, then Coin EXIT to reach root and commit | All 98 Game/Coin/Manual values redisplayed after fresh imports |
| Virtua Racing / Formula | Select page EXIT, enter with green/X, choose YES (SAVED) with red/B, confirm with green/X | All 63 `vr` and 59 `vformula` Game System values redisplayed after fresh imports |

The earlier representative sample and reload directories are preserved under
`validation/nvram-campaigns/2026-10-01/<set>/representative/`.
They are ignored by Git and contain local evidence, not tracked presets. Manual
Setting pages can require an exit sequence back through Coin Assignment
before the EEPROM changes. Verify the complete sequence separately for each
set; the VF route above does not establish other families' routes.

## Full campaign scope and completion criteria

The active campaign covers **every operator setting and every reachable value**
in each currently starting Model 1 parent and clone. `netmerc` is deferred by
user direction until its game starts. This includes game-specific menus,
networking and linked-cabinet entries,
settings that will not become Core Options, and settings that prove unsafe to
override. The workbook selection is a later product decision; acquisition covers
every operator setting and reachable value even if only a subset becomes
Core Options. Capture default and changed states from a
clean, set-specific baseline. A single-setting sample keeps other settings
at that baseline except for native dependent effects recorded in the catalogue.
Capture each distinct value, including the
native default, unless a value cannot be reached; document such gaps.

| Family | Set | Relationship | Boot samples | Menu/value samples | YAML |
| --- | --- | --- | --- | --- | --- |
| Virtua Fighter | `vf` | parent | captured | Game/Coin/Manual 98/98 values saved, CRC-checked and reloaded | 13 mapped cycles |
| Virtua Racing | `vr` | parent | captured | Game System 63/63 values saved, CRC-checked and reloaded | Game System complete |
| Virtua Racing | `vformula` | clone | captured | Game System 59/59 values saved, CRC-checked and reloaded | Game System complete |
| Star Wars Arcade | `swa` | parent | captured | Game/Coin/Manual 92/92 values saved, CRC-checked and reloaded | 11 mapped cycles |
| Star Wars Arcade | `swaj` | clone | captured | Game/Coin/Manual 90/90 values saved, CRC-checked and reloaded | 11 mapped cycles |
| Wing War | `wingwar` | parent | captured | Game/Coin/Manual/Communication 134/134 values saved, CRC-checked and reloaded | 14 mapped cycles; calibration and operator settings complete |
| Wing War | `wingwaru` | clone | captured | Game/Coin/Manual/Communication 134/134 values saved, CRC-checked and reloaded | 14 mapped cycles; calibration and operator settings complete |
| Wing War | `wingwarj` | clone | captured | Game/Coin/Manual/Communication 134/134 values saved, CRC-checked and reloaded | 14 mapped cycles; calibration and operator settings complete |
| Wing War | `wingwar360` | clone | captured | Game/Coin/Manual/Communication 149/149 values saved, CRC-checked and reloaded | 14 mapped cycles; calibration and operator settings complete |
| Net Merc | `netmerc` | parent | captured | deferred: game does not start | deferred |

For each set, inventory all menu pages and selectable fields in visible order;
record the native default, full value cycle, unavailable/conditional fields,
and effects on other fields. Acquire screenshots and Save RAM at first boot,
after native initialization, at menu entry, and after saving each isolated
value. Repeat when a setting depends on another choice. Compare parent and
clone menus and samples directly; a clone may have a different layout or
default even when its title and controls match the parent.

Catalogue each observed setting in that set's diagnostic YAML and in the
review workbook, including fields rejected for Core Options. Map changed
bytes and native integrity/bookkeeping separately; confirm the visible value
after importing a sample in a fresh instance. A set is complete only when its
menu inventory, value cycles, baseline and variation samples, byte mapping,
YAML, workbook rows and fresh-instance checks agree. Initial templates and
frontend options have separate implementation and RetroArch verification gates.

The existing boot samples for all ten sets were a starting point, not this
completion criterion. The nine currently starting sets now have complete
campaigns for every catalogued discrete operator setting: 110 fields and
953 isolated values saved and reloaded. The four Wing War sets also have
freshly reloaded full-range calibration samples, kept separate from discrete
Core Options. The fields and dependent displays are catalogued in
[the operator settings catalogue](MODEL1_DIAGNOSTIC_SETTINGS_CATALOG.md).
The [review workbook](model1_core_options_review.xlsx) records the user's
approved selection: 39 NVRAM Settings fields across nine sets. Automatic setup
has nine approved complete native templates with explicit country/offline
startup overrides. On 2026-10-02 the user added VR CABINET = SPECIAL.
The menu fields and automatic templates are now implemented. Automatic setup
seeds the complete native template, including fields omitted from NVRAM Settings;
the startup marks identify explicit policy values only. Valid existing Save RAM
is preserved. NetMerc remains excluded. Wing War's separate full-range analog
calibration evidence is not substituted for native template calibration.
Each set's YAML includes the approved field keys and startup values; acquisition
verification notes retain their original date and scope.
The generated per-set recipes, manifests and raw samples are retained locally
under `validation/nvram-campaigns/2026-10-01/` and ignored by Git. The
boot-first Test entry and distinct parent/clone navigation remain in those
recipes.

## Verification gates

1. Inspect the screenshot and compare the complete Save RAM payload.
2. Repeat from the same baseline; isolate unrelated bookkeeping changes.
3. Import the sample in a fresh core and confirm the setting in the native menu.
4. Update YAML, the review workbook and per-clone coverage together.
5. Verify the reviewed implementation in real RetroArch before declaring frontend
   support. Direct ABI evidence does not replace frontend testing.

## Verified on 2026-10-01

The permanent VR baseline recipe completed 600 frames and produced two validated
Save RAM containers and readable screenshots. Importing the frame-600 sample
before the first frame in a fresh instance preserved all 128 EEPROM bytes.
Evidence is retained at `/private/tmp/tgpulse-vr-repeatable-20261001` and
`/private/tmp/tgpulse-vr-repeatable-reload-20261001`.
These temporary paths are local evidence locations, not a permanent archive.

The full Game System campaign acquired 63 VR and 59 Virtua Formula isolated
saved samples. Every selected-menu screenshot and every fresh-load selected
menu exactly matched its set's documentary image, and every imported hash
matched the corresponding committed sample. Every sample also passed the
native EEPROM CRC. The verified recipes, samples and reloads are preserved
locally under `validation/nvram-campaigns/2026-10-01/{vr,vformula}/` (28 MiB).
That directory is ignored by Git and contains no ROMs. It is local evidence,
not a published archive; back it up separately before moving to another host.
The ignored `campaign-source-state.json` records the source commit, dirty
worktree state and exact core hash used for this local run.

For both sets the game's 16-bit CRC is stored in EEPROM offsets `0x08–0x09`
in big-endian order. Compute CRC-16/CCITT with polynomial `0x1021` and initial
value zero over offsets `0x0A–0x7F`, reading the high byte then low byte of
each little-endian stored word. All 122 committed samples match. The outer
Save RAM container has its own separate CRC; the two must not be confused.
The per-field byte encodings are in each set's YAML, checked against every
sample by `audit_model1_game_system_yaml.rb`.

The shared fields have matching byte encodings, independently observed in the
parent and clone. Their full default EEPROM images differ at offsets `0x0A`,
`0x1A`, `0x1B` and `0x7D`, plus the CRC bytes, so a parent template must not
be used as the clone's complete initial image. VR LINK ID = LIVE was saved and
redisplayed by the field-specific Test route. A normal boot without a linked
peer instead reached a striped `CANCELLED` communication screen after 1500
frames; a clean NO LINK control run reached attract mode. Linked operation
remains unverified, and this value is not an approved automatic preset.

The VF campaign acquired 98 isolated saved values across 13 fields. All
selected-menu screenshots and fresh-load menus exactly match the documentary
images. Every sample passed both the Save RAM container CRC and VF's native
EEPROM CRC; fresh-load import hashes identify the corresponding committed
sample. Recipes, raw samples and reloads are preserved locally under
`validation/nvram-campaigns/2026-10-01/vf/`, ignored by Git. The field byte
encodings are in `vf.yaml` and checked against all 98 samples by
`audit_model1_vf_yaml.rb`. The default EEPROM image is identical across the
13 isolated field runs. VF stores its CRC at EEPROM offsets `0x08–0x09` in
big-endian order, using CRC-16/CCITT, polynomial `0x1021`, initial value zero,
over `0x0C–0x7F`, high byte first in each stored little-endian word. Offset
`0x0B` changes with saved value steps and is excluded as bookkeeping. This
integrity mapping was tested on all 98 captured values. Real RetroArch behavior remains unverified.

The Star Wars Arcade campaign acquired 92 `swa` and 90 `swaj` isolated values.
All selected and fresh-load screenshots match the set's documentary images,
and all sample imports match the committed hashes. Every sample passed the
container CRC, native EEPROM CRC and mirrored data block check. The local
ignored evidence is under `validation/nvram-campaigns/2026-10-01/{swa,swaj}/`;
field encodings are in the corresponding YAML and are checked by
`audit_model1_assignment_yaml.rb`. The native CRC uses CRC-16/CCITT,
polynomial `0x1021`, initial `0x5A81`, traversing `0x0A–0x41` high byte first
per stored word; the checksum is big-endian at `0x08–0x09`. Settings bytes
`0x06–0x3B` are mirrored at `0x42–0x77`; offset `0x0B` is bookkeeping.
The parent's and clone's complete default EEPROM images differ, as do their
Pilot/Gunner value sequences, so they retain separate recipes and templates.
Real RetroArch behavior remains unverified.

The four Wing War campaigns acquired 134 values each for `wingwar`,
`wingwaru` and `wingwarj`, plus 149 for `wingwar360`. Every selected and
fresh-load menu matches the corresponding set's documentary screenshot;
all 551 samples passed container CRC, native EEPROM CRC and mirror checks.
The native CRC uses CRC-16/CCITT, polynomial `0x1021`, initial `0x3A19`,
over `0x0A–0x41` in stored byte order. The checksum at `0x08–0x09` is
little-endian; bytes `0x06–0x3B` are mirrored at `0x42–0x77`, and `0x0A`
is bookkeeping. Per-set YAML byte maps are checked against every sample by
`audit_model1_assignment_yaml.rb`. The R360 Coin page lacks CREDIT TO
CONTINUE and has an extra chute multiplier, so it retains its own recipes
and template. NETWORK's saved operator value is verified in the menu; linked
operation is a separate runtime gate. The four calibration samples were
created by a full analog sweep and retained only as procedural evidence,
not transferable presets. All local Save RAM evidence is ignored by Git.

## Reviewed integration verification — 2026-10-02

Reference procedures were inspected in current SM2 `core_options.h`,
`initial_nvram.cpp`, generated templates and `core.cpp`, and Supermodel's
Libretro seeding/options paths. The minimum adaptation keeps the same
repeat/literal sample encoding and startup/save precedence, with Model 1's
native EEPROM encodings, CRC byte order and mirrored regions.

`tools/generate_model1_nvram.py` reconciles the approved workbook, YAML mappings
and all 953 saved samples, including fields excluded from the menu. Generated
adapter data contains derived settings/Save RAM payloads, source hashes and
39 field maps; no ROM data. Original recipes and samples remain ignored.
Regenerate after approved catalogue changes, then check:

```sh
python3 tools/generate_model1_nvram.py --check
python3 tools/test_libretro_nvram_settings.py --core target/release/libtgpulse_next_m1_libretro.dylib --rom-dir /path/to/model1/roms --output /path/to/new/evidence-directory
```

All nine ABI sets verify complete automatic images, valid-save precedence,
manual selection changes, native integrity, reset and exact Save State
restoration, disabled behavior and visibility while paused. 41 adapter tests
pass. Automatic setup defaults Enabled and requires restart; NVRAM Settings
defaults Disabled. Field selectors use observed values with native defaults
marked `(Default)`, as in the references. A changed selection resets the
machine to reload firmware caches; unrelated backup RAM/EEPROM is preserved.
Networking choices persist operator values; frontend linking is documented in
[the linked-cabinet guide](LIBRETRO_MODEL1_LINKED_CABINETS.md).

Five isolated 600-frame RetroArch runs verify VR automatic SPECIAL, retention
of an existing STANDARD save, and manual VF, SWA and Wing War selections through
saved-byte readback and native integrity checks. Evidence:
`/private/tmp/tgpulse-nvram-frontend-{vr-auto,vr-existing,vf-options,swa-options,wingwar-options}`.
These used core SHA-256
`4471f1699071293c71c846ecc2d30b3733d6aab7c2596dbe7fc8f655f8d07d37`.
The final display-callback build passes the nine-set ABI run at
`/private/tmp/tgpulse-nvram-abi-display-callback` and a 180-frame RetroArch VF
manual-settings run at `/private/tmp/tgpulse-nvram-frontend-final`, including
USA / ASTRO CITY2 Save RAM readback. Final core SHA-256:
`7ca24e7646b0a4568ca089325c270d2a7eda2a724c97c6ba6fde73231dc64629`.
Temporary paths are local evidence locations. These bounded checks establish
frontend integration and persistence, not user gameplay/controller acceptance.
The existing runner accepts `--initial-nvram`, `--nvram-settings`, repeatable
`--nvram-setting KEY=VALUE`, and `--nvram-sample` for an isolated existing-save
fixture. User saves and configuration are untouched.
