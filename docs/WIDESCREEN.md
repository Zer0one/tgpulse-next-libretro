# Widescreen modes

Settings → Widescreen and `--widescreen off|on|auto` select the same persisted
setting (`widescreen = auto` in `config/settings.conf`). Existing on/off files
remain valid. Off remains the default; no user configuration is rewritten by
this change. The touch settings menu cycles through all three choices.

- **Off** retains the previous native framing.
- **On** retains the existing forced, window-shaped 3D field-of-view expansion
  and optional stretched 2D layers.
- **Auto** displays the native framebuffer at the cabinet's **4:3 or 16:9**
  aspect, with bars as needed. It does not expand the game's FOV a second time.
  Mouse/lightgun mapping uses that same presentation rectangle. The stretch-2D
  option belongs to forced On; Auto displays the entire native image together.

Auto reads the live machine EEPROM, never changes operator settings, and is
re-evaluated on redraw (including after a service-menu save or state load).
Unknown games and unrecognized/blank values fall back to 4:3.

| Sets | EEPROM byte offset | 16:9 condition |
| --- | --- | --- |
| vr | 0x0A | value = 1 (0 = 4:3) |
| indy500, indy500d, indy500to | 0x17 | value = 1 |
| stcc, stcca, stccb, stcco | 0x10 | bit 0x08 set (blank 0xFF excluded) |

Offsets refer to little-endian EEPROM bytes, not the header of the combined
TGPULSE1 save file. Model 2 mappings follow `automatic_widescreen()` in
SM2-Emu Libretro (`src/libretro/core.cpp`, inspected at f4d9051). MAME's
`src/mame/layout/vr.lay` documents VR's Monitor and STCC's Deluxe setting;
Model 2's driver attaches that layout to Indy 500 and STCC families.

## Model 1 investigation (2026-09-26)

VR was tested using the release debugger and isolated temporary NVRAM, not the
user's save. Game System initially showed `16:9 WIDE`; changing only Monitor
to `4:3 NORMAL`, confirming Yes and saving changed EEPROM byte 0x0A from 1 to
0 (plus checksum/write bookkeeping bytes). A fresh emulator instance loaded
the resulting file and showed `4:3 NORMAL` again. This is a native operator
setting, not a renderer preference.

Loaded program images for vf, vformula, swa/swaj and wingwar/wingwarj/wingwaru/
wingwar360 were also searched for monitor/aspect menu strings. VR contains
explicit 4:3/16:9 choices. VF has a Monitor label but no corresponding aspect
strings found; Virtua Formula has camera monitor labels, not proof of a
selectable aspect setting. MAME's Model 1 driver explicitly assigns layout_vr
to VR, not those other sets. This is not proof that their hardware could never
use another display: no unverified EEPROM mapping is inferred for them.
NetMerc's program scan was blocked by the existing loader error
`chip at 0x80000 overruns its region`; its aspect capability remains unverified.

Model 2 offsets are source-backed, not newly tested in each game here. Automated
tests cover the field/byte interpretation, clones, blank/short data, mode
parsing/persistence, and distinguishing native Auto from forced On. Actual
desktop presentation and live service-menu transitions still need gameplay QA.

## Fullscreen startup and shared settings

`fullscreen = on` in `config/settings.conf` enters fullscreen after a game
loads successfully, including a game supplied on the command line. The library
always starts windowed; closing a game (or a failed load) returns to windowed
mode without clearing the preference. Pausing is not closing a game.
The GUI checkbox is remembered. In-game F11 only toggles the current window:
it does not change or save the startup preference. Unrelated GUI settings do
not undo that temporary toggle. The next game uses the saved preference again.
F11 in the library is ignored.
`--fullscreen on|off` overrides the loaded preference
for that invocation without itself writing the file. Missing/invalid values
keep the default (off).

The development launcher and the toolkit's installed `tgpulse.my` launcher
both use `~/dev/TGPulse` as runtime cwd by default, hence share settings,
bindings, ROMs and NVRAM. `TGPULSE_RUNTIME_DIR` can override the installed
launcher's cwd. Running a binary directly uses the caller's cwd instead.
Sharing the file does not update the installed binary: an older executable
can ignore new settings or rewrite them using its older format.
