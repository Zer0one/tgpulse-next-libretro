# Model 1 operator settings catalogue

This is the acquisition authority for campaign recipes and sample names. It
records what the emulated game's Test menu actually displayed. A value enters
the catalogue only with a screenshot; a numerical pattern or a parent menu is
not enough. After a set's menu/value cycle is complete, first transfer the
observed list to its diagnostic YAML and the Core Options review workbook.
Then generate or revise its TOML recipes and acquire isolated saved samples.
Finally update YAML and Excel with byte mappings and verification results.
Option selection is a separate review decision. [Capture procedure](NVRAM_CAPTURE.md).

Each screenshot under `diagnostic-evidence/` is a full 496 × 384 rendered
service-menu frame. The contact sheet for each VR field crops those same
frames around the selected row and numbers the change steps for rapid review.
The corresponding raw `.srm` files and manifests remain outside Git at
`/private/tmp/tgpulse-vr-cycle-20261001` and
`/private/tmp/tgpulse-vr-coin-full-cycle-20261001`.

## Coverage

Network-related operator entries are part of the inventory, even when linked
cabinet transport is not yet implemented in this core. Record their visible
labels, defaults, full value cycles, dependencies and screenshots per set.
`vr` LINK ID and Wing War NETWORK are documented below; inspect each
set's operator-setting pages independently for link role, cabinet ID or
communication settings. Do not equate an
operator menu value with verified network gameplay.

| Family | Set | Menu inventory | Value cycles | Documentary screenshots |
| --- | --- | --- | --- | --- |
| Virtua Fighter | `vf` | Root, Game, Coin and Manual operator pages complete | 13 closed cycles, 98/98 values saved and reloaded | [Root](diagnostic-evidence/vf/test-root.png), [Game Assignment](diagnostic-evidence/vf/game-assignment-default.png) |
| Virtua Racing | `vr` | Game System page fully sampled/reloaded; other Test entries are diagnostic/actions | Game System: 10/10 closed cycles, 63/63 values persistent | [Root](diagnostic-evidence/vr/test-root.png), [Default page](diagnostic-evidence/vr/game-system-default.png) |
| Virtua Formula | `vformula` | Game System operator page fully sampled/reloaded | Game System: 9/9 closed cycles, 59/59 values persistent | [Root](diagnostic-evidence/vformula/test-root.png), [Game System](diagnostic-evidence/vformula/game-system-default.png) |
| Star Wars Arcade | `swa` | Root, Game, Coin and Manual operator pages complete | 11 closed cycles, 92/92 values saved and reloaded | [Root](diagnostic-evidence/swa/test-root.png), [Game Assignment](diagnostic-evidence/swa/game-assignment-default.png) |
| Star Wars Arcade | `swaj` | Root, Game, Coin and Manual operator pages complete | 11 closed cycles, 90/90 values saved and reloaded | [Root](diagnostic-evidence/swaj/test-root.png), [Game Assignment](diagnostic-evidence/swaj/game-assignment-default.png) |
| Wing War | `wingwar` | Root, Game, Coin, Volume and Communication pages observed | 14 closed cycles, 134/134 values saved and reloaded; full-range calibration sampled separately | [Root](diagnostic-evidence/wingwar/test-root.png), [network values](#wing-war-communication-setting) |
| Wing War | `wingwaru` | Root, Game, Coin, Volume and Communication pages observed | 14 closed cycles, 134/134 values saved and reloaded; calibration sampled separately | [Root](diagnostic-evidence/wingwaru/test-root.png), [network values](#wing-war-communication-setting) |
| Wing War | `wingwarj` | Root, Game, Coin, Volume and Communication pages observed | 14 closed cycles, 134/134 values saved and reloaded; calibration sampled separately | [Root](diagnostic-evidence/wingwarj/test-root.png), [network values](#wing-war-communication-setting) |
| Wing War R360 | `wingwar360` | Root, Game, Coin, Volume and Communication pages observed | 14 closed cycles, 149/149 values saved and reloaded; calibration sampled separately | [Root](diagnostic-evidence/wingwar360/test-root.png), [network values](#wing-war-communication-setting) |
| Sega NetMerc | `netmerc` | Root, Game, Coin and Controller pages observed | 4 closed cycles, 20/20 values saved and reloaded; controller calibration verified separately | [Root](diagnostic-evidence/netmerc/test-menu.png), [campaign](LIBRETRO_U3_NETMERC_CAMPAIGN.md) |

## Virtua Racing (`vr`): Game System

At boot, hold L3 (Test) for 300 frames. On the root Test menu, press red/B
twice to select GAME SYSTEM and green/X to enter. From initially selected EXIT,
red/B advances to the first setting and through the list below; blue/A changes
the selected value. Each button press in this survey was ten frames followed
by 20 frames with only Test held. The screenshot series includes the default
as step 0 and one blue/A press for each following step. In every field the next
press after the final listed value returned to step 0, including Coin Setting
after its 27th press. This proves the observed blue/A value cycle closes.

| Position | Setting | Values in observed blue/A order, starting with native default | Evidence |
| ---: | --- | --- | --- |
| 1 | START CREDIT | 1CREDIT START; 5CREDITS START; 4CREDITS START; 3CREDITS START; 2CREDITS START | [All values](diagnostic-evidence/vr/start-credit-all-values.png) |
| 2 | COIN SETTING | 1; 27; 26; 25; 24; 23; 22; 21; 20; 19; 18; 17; 16; 15; 14; 13; 12; 11; 10; 9; 8; 7; 6; 5; 4; 3; 2 | [All values](diagnostic-evidence/vr/coin-setting-all-values.png) |
| 3 | CAR COLOR | RED; ORANGE; SKYBLUE; PINK; BLACK; GREEN; YELLOW; BLUE | [All values](diagnostic-evidence/vr/car-color-all-values.png) |
| 4 | COURSE SELECT | MAJORITY VOTE; EXPERT; MIDDLE; BEGINNER | [All values](diagnostic-evidence/vr/course-select-all-values.png) |
| 5 | LINK ID | NO LINK; LIVE; SLAVE; MASTER | [All values](diagnostic-evidence/vr/link-id-all-values.png) |
| 6 | RACE MODE | NORMAL; GRANDPRIX | [All values](diagnostic-evidence/vr/race-mode-all-values.png) |
| 7 | COUNTRY | EXPORT; U.S; JAPAN | [All values](diagnostic-evidence/vr/country-all-values.png) |
| 8 | MONITOR | 16:9 WIDE; 4:3 NORMAL | [All values](diagnostic-evidence/vr/monitor-all-values.png) |
| 9 | CABINET | STANDARD; SPECIAL; 2P LINK; UPRIGHT | [All values](diagnostic-evidence/vr/cabinet-all-values.png) |
| 10 | DIFFICULTY | NORMAL; EASY; HARDEST; HARD | [All values](diagnostic-evidence/vr/difficulty-all-values.png) |

CHUTE1 and CHUTE2 are display rows under COIN SETTING, not independently
selected fields. Both display `1COIN 1CREDIT` at the native default; the
complete Coin Setting screenshots document their dependent display for each
setting value. EXIT is the next selectable item after DIFFICULTY. The other root entries are service tests or actions; they do not provide
operator values for NVRAM Settings.

All 63 values on this page have isolated saved samples, native EEPROM CRC
checks and field-specific fresh-load menu comparisons. The encoded bytes and
CRC are recorded in [`vr.yaml`](../data/diagnostic-menus/vr.yaml). LINK ID =
LIVE persists in the Test menu, but a normal boot without a linked peer reaches
a communication cancellation screen. Linked gameplay is unverified; it is not
an approved automatic preset. Real frontend behavior remains outside this verification.

## Virtua Fighter (`vf`)

Enter Test/L3 after the attract sequence. The root Test menu selects GAME
ASSIGNMENT after five Service/R3 pulses and COIN ASSIGNMENT after six. Test/L3
enters the page. The older first-boot screenshot showed only the assignment
fields; the postboot route exposes the complete menu. The observed Game page
starts on MATCH COUNT(1P); Service/R3 advances to the next item, and Test/L3
changes its value. All seven value cycles close in a fresh-run survey.

| Position | Game Assignment setting | Values from native default | Screenshots |
| ---: | --- | --- | --- |
| 1 | MATCH COUNT(1P) | 2; 3; 4; 5 | [All values](diagnostic-evidence/vf/match-count-1p-all-values.png) |
| 2 | MATCH COUNT(VS) | 2; 3; 4; 5 | [All values](diagnostic-evidence/vf/match-count-vs-all-values.png) |
| 3 | DIFFICULTY | NORMAL; HARD; HARDEST; EASY | [All values and dependent Stage Width/Energy](diagnostic-evidence/vf/difficulty-all-values.png) |
| 4 | ADVERTISE SOUND | ON; OFF | [All values](diagnostic-evidence/vf/advertise-sound-all-values.png) |
| 5 | CONTINUE | ON; OFF | [All values](diagnostic-evidence/vf/continue-all-values.png) |
| 6 | COUNTRY | JAPAN; USA; EXPORT | [All values](diagnostic-evidence/vf/country-all-values.png) |
| 7 | CABINET TYPE | SUPER MEGALO; ASTRO CITY2 | [All values](diagnostic-evidence/vf/cabinet-type-all-values.png) |

INITIALIZE and EXIT are actions after the seven fields. INITIALIZE has not
been executed or classified as an option. The full difficulty screenshots
show that Stage Width and Energy displays change with DIFFICULTY.

On the [Coin Assignment default page](diagnostic-evidence/vf/coin-assignment-default.png),
Credit to Start and Credit to Continue are one selected field. Test/L3 cycles
15 ordered pairs: `(1,1)`, then `(2,1)` to `(2,2)`, `(3,1)` to `(3,3)`,
`(4,1)` to `(4,4)`, and `(5,1)` to `(5,5)`; the next press returns to `(1,1)`.
The [full frames](diagnostic-evidence/vf/credit-start-continue-all-values.png)
retain both displayed credit rows. COIN/CREDIT SETTING is the second field:
`1` through `26` in ascending order and back to `1`. Its
[full frames](diagnostic-evidence/vf/coin-credit-setting-all-values.png)
include the dependent COIN CHUTE #1/#2 ratios and FREE PLAY at `26`.

The nested [Manual Setting default page](diagnostic-evidence/vf/manual-setting-default.png)
starts on EXIT. Service/R3 selects each of four fields. Each Test/L3 value
cycle closes after nine changes:

| Field | Values from native default | Screenshots |
| --- | --- | --- |
| COIN TO CREDIT | 1 through 9 COIN(S) 1 CREDIT(S) | [All values](diagnostic-evidence/vf/coin-to-credit-all-values.png) |
| BONUS ADDER | NO BONUS ADDER; 2 through 9 COIN(S) GIVE 1 EXTRA COIN | [All values](diagnostic-evidence/vf/bonus-adder-all-values.png) |
| COIN CHUTE #1 MULTIPLIER | 1 COIN COUNTS AS 1 through 9 COIN(S) | [All values](diagnostic-evidence/vf/coin-chute-1-multiplier-all-values.png) |
| COIN CHUTE #2 MULTIPLIER | 1 COIN COUNTS AS 1 through 9 COIN(S) | [All values](diagnostic-evidence/vf/coin-chute-2-multiplier-all-values.png) |

All 98 values on the Game, Coin and Manual pages have isolated committed
samples, native EEPROM CRC checks, and field-specific fresh-load menu matches.
Manual settings require exiting both Manual and Coin to commit. The encoded
bytes and CRC are recorded in [`vf.yaml`](../data/diagnostic-menus/vf.yaml).
`coin_credit_setting` changes five EEPROM bytes because the displayed chute
ratios depend on its selected value. Those bytes overlap the Manual fields;
the order and precedence of future automatic overrides need a separate review
before implementing Core Options. Game Assignment INITIALIZE was not used.
Real frontend behavior remains pending.

## Virtua Formula (`vformula`): Game System

Enter Test/L3 after the attract sequence, select GAME SYSTEM with two red/B
presses from EXIT, and enter with green/X. Red/B selects a field from EXIT;
blue/A changes its value. The nine cycles below were captured directly from
`vformula.zip`, including the press that returns each field to its native
default. This page differs from the Virtua Racing parent: it has CAR NUMBER
and ADVERTISE SOUND, and no MONITOR or CABINET field in this menu.

| Position | Setting | Values in observed blue/A order from the native default | Evidence |
| ---: | --- | --- | --- |
| 1 | START CREDIT | 1CREDIT START; 5CREDITS START; 4CREDITS START; 3CREDITS START; 2CREDITS START | [All values](diagnostic-evidence/vformula/start-credit-all-values.png) |
| 2 | COIN SETTING | 1; 27; 26; 25; 24; 23; 22; 21; 20; 19; 18; 17; 16; 15; 14; 13; 12; 11; 10; 9; 8; 7; 6; 5; 4; 3; 2 | [All values](diagnostic-evidence/vformula/coin-setting-all-values.png) |
| 3 | CAR NUMBER | NO.1 (RED); NO.8 (ORANGE); NO.7 (SKYBLUE); NO.6 (PINK); NO.5 (BLACK); NO.4 (GREEN); NO.3 (YELLOW); NO.2 (BLUE) | [All values](diagnostic-evidence/vformula/car-number-all-values.png) |
| 4 | COURSE SELECT | MAJORITY VOTE; EXPERT; MIDDLE; BEGINNER | [All values](diagnostic-evidence/vformula/course-select-all-values.png) |
| 5 | LINK ID | NO LINK; LIVE; SLAVE; MASTER | [All values](diagnostic-evidence/vformula/link-id-all-values.png) |
| 6 | RACE MODE | NORMAL; GRANDPRIX | [All values](diagnostic-evidence/vformula/race-mode-all-values.png) |
| 7 | COUNTRY | JAPAN; EXPORT; U.S | [All values](diagnostic-evidence/vformula/country-all-values.png) |
| 8 | ADVERTISE SOUND | ON; OFF | [All values](diagnostic-evidence/vformula/advertise-sound-all-values.png) |
| 9 | DIFFICULTY | NORMAL; EASY; HARDEST; HARD | [All values](diagnostic-evidence/vformula/difficulty-all-values.png) |

CHUTE1 and CHUTE2 are display rows controlled by COIN SETTING. Its full
screenshots include their changing ratios and FREEPLAY at setting 27. LINK ID
is an observed operator value only; linked gameplay has not been verified.
All 59 values on this clone's Game System page have isolated saved samples,
native EEPROM CRC checks and field-specific fresh-load menu comparisons. The
encoded bytes are recorded in [`vformula.yaml`](../data/diagnostic-menus/vformula.yaml).
Its shared field encodings agree with VR, but the complete default EEPROM
image differs; parent templates cannot be substituted for clone samples.
Real frontend behavior remains pending.

## Star Wars Arcade (`swa`, `swaj`)

The Test root appears after the game reaches its attract sequence and Test/L3
is pressed. Pressing Test continuously from the first frame did not open it.
The observed root has MEMORY TEST, INPUT TEST, OUTPUT TEST, SOUND TEST,
C.R.T. TEST, GAME ASSIGNMENTS, COIN ASSIGNMENTS, BOOKKEEPING and BACKUP DATA
CLEAR. INDIVIDUAL is a section label and EXIT is selected initially.

For both `swa` and `swaj`, the Game Assignment page has four selectable
fields. After the game reaches attract, pressing Test/L3 opens the root menu.
Seven Service/R3 pulses from the initially selected EXIT reached GAME
ASSIGNMENTS in the observed runs; Test/L3 entered it. From EXIT on that page,
Service/R3 selects the first field and advances through the fields. Test/L3
changes the selected value. Every listed cycle was captured independently in
each set, including the return to its native default.

| Position | Setting | Native default and complete Test/L3 cycle | `swa` evidence | `swaj` evidence |
| ---: | --- | --- | --- | --- |
| 1 | GAME DIFFICULTY #1 (PLAYER) | 3/5; 4/5; 5/5; 1/5; 2/5; return to 3/5 | [All values](diagnostic-evidence/swa/difficulty-player-all-values.png) | [All values](diagnostic-evidence/swaj/difficulty-player-all-values.png) |
| 2 | GAME DIFFICULTY #2 (ENEMY) | 3/5; 4/5; 5/5; 1/5; 2/5; return to 3/5 | [All values](diagnostic-evidence/swa/difficulty-enemy-all-values.png) | [All values](diagnostic-evidence/swaj/difficulty-enemy-all-values.png) |
| 3 | GAME DIFFICULTY #3 (TIMER) | 3/5; 4/5; 5/5; 1/5; 2/5; return to 3/5 | [All values](diagnostic-evidence/swa/difficulty-timer-all-values.png) | [All values](diagnostic-evidence/swaj/difficulty-timer-all-values.png) |
| 4 | ADVERTISE SOUND | OFF; ON; return to OFF | [All values](diagnostic-evidence/swa/advertise-sound-all-values.png) | [All values](diagnostic-evidence/swaj/advertise-sound-all-values.png) |

The source files beside each contact sheet preserve each full 496 × 384 frame.
The two sets' visible cycles agree. Their NVRAM layouts and all saved
values were checked independently.

## Star Wars Arcade Coin Assignments

The Coin Assignment page groups START and CONTINUE under PILOT WITH GUNNER.
Service/R3 selects START, CONTINUE, COIN/CREDIT SETTING and MANUAL SETTING
from EXIT. Test/L3 changes the selected value. Parent and clone were acquired
independently. The notation below is `Pilot + Gunner` credits, matching the
column headers on each set's [default page](diagnostic-evidence/swa/coin-assignment-default.png)
and [Japan default page](diagnostic-evidence/swaj/coin-assignment-default.png).

| Set | START, native default first | CONTINUE, native default first | Screenshots |
| --- | --- | --- | --- |
| `swa` | 2+1; 2+2; 3+0; 3+1; 3+2; 3+3; 1+0; 1+1; 2+0 | 1+1; 2+0; 2+1; 1+0 | [START](diagnostic-evidence/swa/credit-to-start-pilot-gunner-all-values.png), [CONTINUE](diagnostic-evidence/swa/credit-to-continue-pilot-gunner-all-values.png) |
| `swaj` | 1+1; 2+0; 2+1; 2+2; 3+0; 3+1; 3+2; 3+3; 1+0 | 1+1; 1+0 | [START](diagnostic-evidence/swaj/credit-to-start-pilot-gunner-all-values.png), [CONTINUE](diagnostic-evidence/swaj/credit-to-continue-pilot-gunner-all-values.png) |

Each selected START row returns after nine changes, but changing START also
forces the displayed CONTINUE Gunner count from 1 to 0 during the cycle. The
whole page therefore differs after the START row returns. This dependent
change is visible in the full-frame screenshots beside each contact sheet.
An isolated CONTINUE cycle closes after four values in `swa` and two in
`swaj`. These clone-specific settings cannot use an assumed shared recipe.

COIN/CREDIT SETTING has values `1` through `26` in ascending Test/L3 order
and returns to `1` in both sets. The full screenshots document the dependent
COIN CHUTE #1 and #2 ratios for every number, including FREE PLAY at `26`:
[`swa` values](diagnostic-evidence/swa/coin-credit-setting-all-values.png) and
[`swaj` values](diagnostic-evidence/swaj/coin-credit-setting-all-values.png).

### Manual Setting

The nested manual page starts with COIN TO CREDIT selected. Both sets have
four selectable fields. Each has nine values and the entire page returns to
its default after the ninth Test/L3 change. The order below starts with the
native default.

| Field | Observed values | `swa` screenshots | `swaj` screenshots |
| --- | --- | --- | --- |
| COIN TO CREDIT | 1 through 9 COINS 1 CREDIT | [All values](diagnostic-evidence/swa/coin-to-credit-all-values.png) | [All values](diagnostic-evidence/swaj/coin-to-credit-all-values.png) |
| BONUS ADDER | NO BONUS ADDER; 2 through 9 COINS GIVE 1 EXTRA COIN | [All values](diagnostic-evidence/swa/bonus-adder-all-values.png) | [All values](diagnostic-evidence/swaj/bonus-adder-all-values.png) |
| COIN CHUTE #1 MULTIPLIER | 1 COIN COUNTS AS 1 through 9 COINS | [All values](diagnostic-evidence/swa/coin-chute-1-multiplier-all-values.png) | [All values](diagnostic-evidence/swaj/coin-chute-1-multiplier-all-values.png) |
| COIN CHUTE #2 MULTIPLIER | 1 COIN COUNTS AS 1 through 9 COINS | [All values](diagnostic-evidence/swa/coin-chute-2-multiplier-all-values.png) | [All values](diagnostic-evidence/swaj/coin-chute-2-multiplier-all-values.png) |

The [US manual default](diagnostic-evidence/swa/manual-setting-default.png)
and [Japan manual default](diagnostic-evidence/swaj/manual-setting-default.png)
show the same selected text. For multiplier values above the default, their
credit tables differ visibly between sets. Capture and mapping must remain
per set; the screenshots do not establish compatible NVRAM templates.

All 92 `swa` and 90 `swaj` values on these pages now have isolated committed
samples, native EEPROM CRC and mirror checks, and field-specific fresh-load
menu matches. The field encodings are recorded independently in
[`swa.yaml`](../data/diagnostic-menus/swa.yaml) and
[`swaj.yaml`](../data/diagnostic-menus/swaj.yaml). For Manual Setting, returning
to Coin leaves MANUAL SETTING selected; one Service/R3 press selects Coin EXIT
before Test/L3 commits. Initial attempts to save an unchanged default left
Star Wars EEPROM uninitialized. Each native default sample was therefore
generated by saving one changed ADVERTISE SOUND value, loading it in a fresh
core, and saving the return to the native value. The resulting initialized
template was imported for every isolated default capture. The START and
CONTINUE Pilot/Gunner mappings differ between the parent and clone, and START
can modify CONTINUE Gunner as documented above. Real frontend behavior remains pending.

## Wing War Communication Setting

Each of `wingwar`, `wingwaru`, `wingwarj` and `wingwar360` has a Test root with
GAME ASSIGNMENTS, COIN ASSIGNMENTS, VOLUME SETTING and COMMUNICATION SETTING,
as well as diagnostic and bookkeeping pages. The Communication Setting page
has one selectable field, NETWORK. Starting from each set's own default,
pressing Test/L3 while NETWORK is selected produced the same closed cycle in
all four direct ABI surveys:

| Set | Native default | First change | Second change | Return | Screenshots |
| --- | --- | --- | --- | --- | --- |
| `wingwar` | STAND ALONE | MASTER | SLAVE | STAND ALONE | [initial page](diagnostic-evidence/wingwar/network-standalone.png), [selected default](diagnostic-evidence/wingwar/network-0.png), [master](diagnostic-evidence/wingwar/network-master.png), [slave](diagnostic-evidence/wingwar/network-slave.png) |
| `wingwaru` | STAND ALONE | MASTER | SLAVE | STAND ALONE | [initial page](diagnostic-evidence/wingwaru/network-standalone.png), [selected default](diagnostic-evidence/wingwaru/network-0.png), [master](diagnostic-evidence/wingwaru/network-master.png), [slave](diagnostic-evidence/wingwaru/network-slave.png) |
| `wingwarj` | STAND ALONE | MASTER | SLAVE | STAND ALONE | [initial page](diagnostic-evidence/wingwarj/network-standalone.png), [selected default](diagnostic-evidence/wingwarj/network-0.png), [master](diagnostic-evidence/wingwarj/network-master.png), [slave](diagnostic-evidence/wingwarj/network-slave.png) |
| `wingwar360` | STAND ALONE | MASTER | SLAVE | STAND ALONE | [initial page](diagnostic-evidence/wingwar360/network-standalone.png), [selected default](diagnostic-evidence/wingwar360/network-0.png), [master](diagnostic-evidence/wingwar360/network-master.png), [slave](diagnostic-evidence/wingwar360/network-slave.png) |

This records the operator menu only. It does not establish linked gameplay,
transport or safe automatic NVRAM overrides. The remaining Wing War operator setting cycles and nested pages are
covered in the sections below.

## Wing War: additional default pages

From the root Test menu, Service/R3 pulses 6, 7 and 11 selected GAME
ASSIGNMENTS, COIN ASSIGNMENTS and VOLUME SETTING, respectively, in each set.
Test/L3 entered the selected page. The four Game Assignment default frames
match visually: GAME DIFFICULTY = 3, EXPERT MODE GAME TIME = 120,
DOG FIGHT MODE LIFE = 36, EXPERT MODE LIFE = 24, DOG FIGHT MODE LIFE (US) = 32,
EXPERT MODE LIFE (US) = 20 and ADVERTISE SOUND = ON. These are initial
observations; all seven Game Assignment fields now have closed cycles.

| Set | Game defaults | Coin defaults | Volume defaults |
| --- | --- | --- | --- |
| `wingwar` | [Game Assignment](diagnostic-evidence/wingwar/game-assignment-default.png) | [Coin Assignment](diagnostic-evidence/wingwar/coin-assignment-default.png) | [Volume Setting](diagnostic-evidence/wingwar/volume-setting-default.png) |
| `wingwaru` | [Game Assignment](diagnostic-evidence/wingwaru/game-assignment-default.png) | [Coin Assignment](diagnostic-evidence/wingwaru/coin-assignment-default.png) | [Volume Setting](diagnostic-evidence/wingwaru/volume-setting-default.png) |
| `wingwarj` | [Game Assignment](diagnostic-evidence/wingwarj/game-assignment-default.png) | [Coin Assignment](diagnostic-evidence/wingwarj/coin-assignment-default.png) | [Volume Setting](diagnostic-evidence/wingwarj/volume-setting-default.png) |
| `wingwar360` | [Game Assignment](diagnostic-evidence/wingwar360/game-assignment-default.png) | [Coin Assignment](diagnostic-evidence/wingwar360/coin-assignment-default.png) | [Volume Setting](diagnostic-evidence/wingwar360/volume-setting-default.png) |

The `wingwar`, `wingwaru` and `wingwarj` Coin Assignment defaults match:
CREDIT TO START = CREDIT 2, CREDIT TO CONTINUE = CREDIT 2, COIN/CREDIT
SETTING = 1, and COIN CHUTE #1 displays 1 COIN 1 CREDIT. The `wingwar360`
page instead shows CREDIT TO START = CREDIT 5 and COIN/CREDIT SETTING = 5,
does not display CREDIT TO CONTINUE, and adds COIN CHUTE #2 with 1 COIN 5
CREDITS. MANUAL SETTING is a selectable nested page; its fields and effects are
documented below. This clone difference rules out assuming a shared coin
recipe or NVRAM template.

VOLUME SETTING is a three-axis calibration page, with STICK LR, STICK FR and
THROTTLE maximum/minimum readings and CANCEL & EXIT and SET & EXIT actions.
The initial R360 THROTTLE endpoints are `A0–AF`; the other three sets start
at `00–FF`. A direct ABI probe with neutral axes wrote invalid `80–80`
endpoints that the game rejected on fresh load. A full sweep of each analog
axis followed by a longer Test press on SET & EXIT and exit from the root
Test menu saved `00–FF`, `00–FF`, `01–FF` for the three axes in every set.
The complete 128-byte EEPROM image then survived a fresh import unchanged,
including R360's changed throttle range. Per-set recipes, sample manifests
and reloads are preserved locally under
`validation/nvram-campaigns/2026-10-01/<set>/calibration/`, and
`audit_model1_wingwar_calibration.py` checks them. This is an observed
calibration procedure; its values depend on an actual cabinet/controller and
are not transferable automatic presets or discrete Core Options.

The first fields of GAME ASSIGNMENTS and COIN ASSIGNMENTS have been cycled
independently for all four sets. Each full screenshot is beside its contact
sheet. The next Test/L3 press after the final listed value returned to the
initial screenshot in every run.

| Set | GAME DIFFICULTY values from default | CREDIT TO START values from default | Evidence |
| --- | --- | --- | --- |
| `wingwar` | 3; 4; 5; 6; 7; 8; 1; 2 | 2; 3; 4; 5; 1 | [difficulty](diagnostic-evidence/wingwar/game-difficulty-all-values.png), [credit](diagnostic-evidence/wingwar/credit-to-start-all-values.png) |
| `wingwaru` | 3; 4; 5; 6; 7; 8; 1; 2 | 2; 3; 4; 5; 1 | [difficulty](diagnostic-evidence/wingwaru/game-difficulty-all-values.png), [credit](diagnostic-evidence/wingwaru/credit-to-start-all-values.png) |
| `wingwarj` | 3; 4; 5; 6; 7; 8; 1; 2 | 2; 3; 4; 5; 1 | [difficulty](diagnostic-evidence/wingwarj/game-difficulty-all-values.png), [credit](diagnostic-evidence/wingwarj/credit-to-start-all-values.png) |
| `wingwar360` | 3; 4; 5; 6; 7; 8; 1; 2 | 5; 1; 2; 3; 4 | [difficulty](diagnostic-evidence/wingwar360/game-difficulty-all-values.png), [credit](diagnostic-evidence/wingwar360/credit-to-start-all-values.png) |

The six remaining GAME ASSIGNMENTS fields also have complete Test/L3
cycles. The order below starts with the native default and the next press
after the final value returns to that default in each set. The displayed
ranges are 60–300 in steps of 15 for game time and 16–64 in steps of 4 for
the four life fields; the screenshots establish the wrap order.

| Position | Setting | Values from native default | `wingwar` | `wingwaru` | `wingwarj` | `wingwar360` |
| ---: | --- | --- | --- | --- | --- | --- |
| 2 | EXPERT MODE GAME TIME | 120; 135; 150; 165; 180; 195; 210; 225; 240; 255; 270; 285; 300; 60; 75; 90; 105 | [Values](diagnostic-evidence/wingwar/expert-mode-game-time-all-values.png) | [Values](diagnostic-evidence/wingwaru/expert-mode-game-time-all-values.png) | [Values](diagnostic-evidence/wingwarj/expert-mode-game-time-all-values.png) | [Values](diagnostic-evidence/wingwar360/expert-mode-game-time-all-values.png) |
| 3 | DOG FIGHT MODE LIFE | 36; 40; 44; 48; 52; 56; 60; 64; 16; 20; 24; 28; 32 | [Values](diagnostic-evidence/wingwar/dog-fight-mode-life-all-values.png) | [Values](diagnostic-evidence/wingwaru/dog-fight-mode-life-all-values.png) | [Values](diagnostic-evidence/wingwarj/dog-fight-mode-life-all-values.png) | [Values](diagnostic-evidence/wingwar360/dog-fight-mode-life-all-values.png) |
| 4 | EXPERT MODE LIFE | 24; 28; 32; 36; 40; 44; 48; 52; 56; 60; 64; 16; 20 | [Values](diagnostic-evidence/wingwar/expert-mode-life-all-values.png) | [Values](diagnostic-evidence/wingwaru/expert-mode-life-all-values.png) | [Values](diagnostic-evidence/wingwarj/expert-mode-life-all-values.png) | [Values](diagnostic-evidence/wingwar360/expert-mode-life-all-values.png) |
| 5 | DOG FIGHT MODE LIFE (US) | 32; 36; 40; 44; 48; 52; 56; 60; 64; 16; 20; 24; 28 | [Values](diagnostic-evidence/wingwar/dog-fight-mode-life-us-all-values.png) | [Values](diagnostic-evidence/wingwaru/dog-fight-mode-life-us-all-values.png) | [Values](diagnostic-evidence/wingwarj/dog-fight-mode-life-us-all-values.png) | [Values](diagnostic-evidence/wingwar360/dog-fight-mode-life-us-all-values.png) |
| 6 | EXPERT MODE LIFE (US) | 20; 24; 28; 32; 36; 40; 44; 48; 52; 56; 60; 64; 16 | [Values](diagnostic-evidence/wingwar/expert-mode-life-us-all-values.png) | [Values](diagnostic-evidence/wingwaru/expert-mode-life-us-all-values.png) | [Values](diagnostic-evidence/wingwarj/expert-mode-life-us-all-values.png) | [Values](diagnostic-evidence/wingwar360/expert-mode-life-us-all-values.png) |
| 7 | ADVERTISE SOUND | ON; OFF | [Values](diagnostic-evidence/wingwar/advertise-sound-all-values.png) | [Values](diagnostic-evidence/wingwaru/advertise-sound-all-values.png) | [Values](diagnostic-evidence/wingwarj/advertise-sound-all-values.png) | [Values](diagnostic-evidence/wingwar360/advertise-sound-all-values.png) |

## Wing War Coin Assignment cycles

All visible Coin Assignment value fields have closed Test/L3 cycles in their
respective set. The full screenshot for each value includes the dependent
COIN CHUTE display, including FREE PLAY and multi-line ratios. The rows below
show the selectable setting values in change order, beginning at the native
default. MANUAL SETTING is a nested page documented below.

| Set | CREDIT TO CONTINUE | COIN/CREDIT SETTING | Documentary screenshots |
| --- | --- | --- | --- |
| `wingwar` | 2; 3; 4; 5; 1 | 1; 6; 8; 9; 10; 11; 12; 15; 17; 18; 19; 21; 22; 24; 26 | [continue](diagnostic-evidence/wingwar/credit-to-continue-all-values.png), [coin/credit and chute](diagnostic-evidence/wingwar/coin-credit-setting-all-values.png) |
| `wingwaru` | 2; 3; 4; 5; 1 | 1; 6; 8; 9; 10; 11; 12; 15; 17; 18; 19; 21; 22; 24; 26 | [continue](diagnostic-evidence/wingwaru/credit-to-continue-all-values.png), [coin/credit and chute](diagnostic-evidence/wingwaru/coin-credit-setting-all-values.png) |
| `wingwarj` | 2; 3; 4; 5; 1 | 1; 6; 8; 9; 10; 11; 12; 15; 17; 18; 19; 21; 22; 24; 26 | [continue](diagnostic-evidence/wingwarj/credit-to-continue-all-values.png), [coin/credit and chute](diagnostic-evidence/wingwarj/coin-credit-setting-all-values.png) |
| `wingwar360` | Not displayed | 5; 6; 7; 8; 9; 10; 11; 12; 13; 14; 15; 16; 17; 18; 19; 20; 21; 22; 23; 24; 25; 26; 1; 2; 3; 4 | [coin/credit and both chutes](diagnostic-evidence/wingwar360/coin-credit-setting-all-values.png) |

CREDIT TO START also has a five-value cycle for each set, documented
above. R360 has a different default and no CREDIT TO CONTINUE row; its menu
must not inherit the standard Wing War coin recipe. The setting number and
chute ratios are kept as distinct observed facts until their NVRAM mapping is
measured.

## Wing War Coin Assignment: Manual Setting

The Manual Setting page has three selectable fields in `wingwar`, `wingwaru`
and `wingwarj`, and a fourth COIN CHUTE #2 MULTIPLIER field in `wingwar360`.
Service/R3 selects a field from EXIT; Test/L3 advances its value. Every field
returned to its own default after nine changes in an independent acquisition.
The displayed tables are dependent output and remain in the full screenshots.

| Field | Values in observed Test/L3 order from the default |
| --- | --- |
| COIN TO CREDIT | 1 COIN 1 CREDIT; 2 through 9 COINS 1 CREDIT |
| BONUS ADDER | NO BONUS ADDER; 2 through 9 COINS GIVE 1 EXTRA COIN |
| COIN CHUTE #1 MULTIPLIER | 1 COIN COUNT AS 1 through 9 COINS |
| COIN CHUTE #2 MULTIPLIER (`wingwar360` only) | 1 COIN COUNT AS 5, 6, 7, 8, 9, 1, 2, 3, 4 COINS |

| Set | Default page | COIN TO CREDIT | BONUS ADDER | CHUTE #1 MULTIPLIER | CHUTE #2 MULTIPLIER |
| --- | --- | --- | --- | --- | --- |
| `wingwar` | [Default](diagnostic-evidence/wingwar/manual-setting-default.png) | [Values](diagnostic-evidence/wingwar/coin-to-credit-all-values.png) | [Values](diagnostic-evidence/wingwar/bonus-adder-all-values.png) | [Values](diagnostic-evidence/wingwar/coin-chute-1-multiplier-all-values.png) | Not displayed |
| `wingwaru` | [Default](diagnostic-evidence/wingwaru/manual-setting-default.png) | [Values](diagnostic-evidence/wingwaru/coin-to-credit-all-values.png) | [Values](diagnostic-evidence/wingwaru/bonus-adder-all-values.png) | [Values](diagnostic-evidence/wingwaru/coin-chute-1-multiplier-all-values.png) | Not displayed |
| `wingwarj` | [Default](diagnostic-evidence/wingwarj/manual-setting-default.png) | [Values](diagnostic-evidence/wingwarj/coin-to-credit-all-values.png) | [Values](diagnostic-evidence/wingwarj/bonus-adder-all-values.png) | [Values](diagnostic-evidence/wingwarj/coin-chute-1-multiplier-all-values.png) | Not displayed |
| `wingwar360` | [Default](diagnostic-evidence/wingwar360/manual-setting-default.png) | [Values](diagnostic-evidence/wingwar360/coin-to-credit-all-values.png) | [Values](diagnostic-evidence/wingwar360/bonus-adder-all-values.png) | [Values](diagnostic-evidence/wingwar360/coin-chute-1-multiplier-all-values.png) | [Values](diagnostic-evidence/wingwar360/coin-chute-2-multiplier-all-values.png) |

The `wingwaru` Bonus Adder and chute #1 multiplier tables change differently
from the parent and `wingwarj` for the same selected text. A fresh-run repeat
confirmed this is set-specific visible behavior, rather than a carry-over
from navigating another field. R360 has its own two-chute matrix with displayed
credit counts capped at 24. The stored byte mappings are measured independently
for all four sets and checked against every saved value. Across the family,
all 551 selected and fresh-load screens match their set's documentary images,
and all samples pass native CRC and mirror checks. R360's 149 values and
two-chute page have their own recipes and YAML. The full-range VOLUME SETTING
calibration procedure is documented above and remains separate from automatic
presets. NETWORK values persist in the Test menu; linked gameplay is a
separate runtime gate.

## Remaining games and completion rule

`netmerc` was excluded from the initial nine-set campaign by user direction
because its earlier core startup remained on an orange screen. U1 now verifies
its title screen in RetroArch Vulkan. U3 now completes all four operator fields and 20 values, with full-range
controller calibration saved and reloaded; the approved initial controller calibration is documented
in [U1 integration](LIBRETRO_U1_INTEGRATION.md). A clone inherits no parent default,
value list, recipe or NVRAM template until its own Test menu and samples agree.

The catalogue is complete for a set only after every page, setting, reachable
value and conditional/dependent display is documented with screenshots and
the cycle returns to its initial value. Only then can the corresponding
per-set TOML variation campaign and complete YAML/workbook rows be generated.

## Sega NetMerc (`netmerc`)

The [U3 campaign](LIBRETRO_U3_NETMERC_CAMPAIGN.md) documents Game Difficulty,
Country, Advertise Sound and Coin Chute #1 in native menu order. All cycles
close and all 20 values persist after native Exit and fresh import. No networking
selector is present. Controller endpoint calibration is separate from the MVD
diagnostic display and is also saved/reloaded. The YAML records backup RAM
selector/dependent bytes; the EEPROM CRC used by other sets does not apply.
The user approved the workbook on 2026-10-04. The three selected fields and
automatic policy are implemented in [U8](LIBRETRO_U8_NETMERC_SETTINGS.md).
