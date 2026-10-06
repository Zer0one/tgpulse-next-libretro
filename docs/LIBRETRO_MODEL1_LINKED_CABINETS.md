# Model 1 linked cabinets

## Reference and minimum adaptation

The current SM2-Emu Libretro `src/libretro/netpacket.{h,cpp}`, linked-cabinet
Core Options and `scripts/test-retroarch-netpacket.py` were inspected before
implementation. Supermodel's `LibretroNetPacket.h`, `LibretroNetBoard.cpp` and
Core Options provide the second reference. Both use frontend-owned Netpacket
sessions, reliable packets, an explicit cabinet total and independent operator
roles. The adapter follows SM2's nonblocking sorted-roster ring procedure.

The existing TGPulse `model1comm::CommBoard` remains unchanged. It receives and
transmits its native complete 453-byte frames before/after the machine's VINT.
The desktop TCP adapter supplies the corresponding native polling boundary.
Libretro replaces only the host transport: no addresses/ports are core options,
no socket/thread/dependency is introduced, and the game still writes COMM roles.
The standalone build and protocol are preserved.

## Options and native roles

`Linked Cabinets` is registered with an independent key for
each supported set, defaults Disabled and is visible only for the current set.
A selection above Disabled fits the native COMM board and expects the selected
number of RetroArch Netplay participants. Without a real Netpacket session,
COMM stays disconnected; no local echo supplies a missing peer.

| Sets | Selector | Operator roles |
| --- | --- | --- |
| `vr`, `vformula` | Disabled; 2–9 Cabinets | Exactly one MASTER, remaining playable cabinets SLAVE, at most one optional LIVE relay. At most eight playable cabinets; the total includes LIVE. Each playable cabinet needs a unique CAR COLOR / CAR NUMBER. |
| `wingwar`, `wingwaru`, `wingwarj`, `wingwar360` | Disabled; 2 Cabinets | One MASTER and one SLAVE in NETWORK. |

Eligibility reuses the native `present_for_set` contract. VF, SWA/SWAJ and
NetMerc have no M1COMM option. The Wing War two-player limit also agrees with
[SEGA's official product record](https://www.sega.jp/history/arcade/product/15974/).
VR/VFormula's eight participant identities are documented by the independently
acquired CAR COLOR / NUMBER settings and the native board's RAM/node bound.
LIVE consumes a transport slot, forwards the ring, and has native ID 0 without
increasing the playable participant count.

### Automatic Network Settings

Immediately below each loaded set's **Linked Cabinets** selector, choose
**Automatic Network Settings**. VR/VFormula default to **Red (Master)**;
Orange, Skyblue, Pink, Black, Green, Yellow and Blue select Slave and the matching
reviewed car identity. **Live** selects the native relay role and resets
car color/number to its reviewed Red default. Wing War variants default to Master and offer Slave.

- With NVRAM Settings enabled and Linked Cabinets enabled, set and maintain the
  selected native role and identity through the corresponding NVRAM selectors.
- With Linked Cabinets OFF and automation enabled, set only managed selectors
  to reviewed defaults: No Link + Red/No.1 (Red), or Wing War Stand Alone.
- NVRAM Settings is the master switch: when Disabled, automation is hidden and
  does not write operator settings. When Enabled, its displayed selectors are
  authoritative over saved NVRAM. An active network preset is authoritative
  for its managed selectors, including after restart; disable the preset for
  manual role/color control.
- Disabled performs no automatic writes, including when Linked Cabinets goes
  OFF. Disabling automation preserves values already applied.
- Changed persisted values use the existing checksum/mirror and automatic
  machine-reset path. Returning to the game applies a topology change before
  the next emulated frame. Netplay host/client selection never determines
  Master/Slave/Live.

The reviewed values and encodings are reused from `nvram_data.rs`; no campaign
samples or upstream emulation components are changed. Color identities need
only be unique among playable cabinets; they are not numbered Netplay slots.

### Connect two cabinets

1. Use the same exact ROM set and core build on both frontends. Keep each
   frontend's saves and configuration separate.
2. Select **2 Cabinets** on both.
3. Choose Automatic Network Settings presets: Red (Master) and Orange (Slave)
   for VR/VFormula, or Master and Slave for Wing War, with NVRAM Settings
   enabled. For manual NVRAM/service configuration, set automation to Disabled
   and use distinct playable identities. The Netplay host can be either native role; host does not
   automatically mean MASTER.
4. Return to each game to apply the options and automatic NVRAM reset. Host a
   RetroArch Netplay session on one frontend and connect the other to it. A
   lobby may also be open before selecting the cabinet count.
5. Let the games perform their native COMM startup. A transport roster alone
   is not proof that the game has established its link.

For MASTER/SLAVE/LIVE, select **3 Cabinets** everywhere and set LINK ID = LIVE
on the third VR/VFormula instance. Automatic Network Settings can select Master, Slave and Live independently
on each instance while NVRAM Settings is enabled. Disable automation before applying custom NVRAM Settings
or using saved service-menu values. No role is assigned by transport.

Return to the game on each cabinet after changing topology or roles. The
frontends may resume at different times; packets carrying the old cabinet
count are ignored while the native ring forms.
For standalone VR operation, set **Linked Cabinets = Disabled** and
return to the game. With NVRAM Settings and Automatic Network Settings enabled,
the latter selects **Link ID = No Link**. With automation Disabled, set this
selector manually. Disabling the transport alone does not change the saved
native role.
A saved Slave role without a fitted COMM board produces the game's horizontal
white-line screen with **CANCELLED**. This was reproduced on macOS/Vulkan on
2026-10-06 using an isolated copy of the affected save: two 900-frame runs
differed only in Link ID (Slave versus No Link). No Link restored the normal
attract scene. Personal configuration and saves were preserved; evidence is
`/private/tmp/tgpulse-vr-stripes-current/` and
`/private/tmp/tgpulse-vr-stripes-no-link/`. The tested core SHA-256 is
`2b87edabdd849c70445a764577d8f9985bf7751a86d93e89e453356a490c25eb`.

Resume or Restart preserves backup RAM/EEPROM, rereads the selected cabinet total and
recreates the native machine only when the total changes. It resets the
transport handshake while keeping an open Netplay lobby's callbacks and host
client slots. After a lost participant, Restart can clear the core's failed
COMM state; a disconnected frontend still has to reconnect to the lobby.
Start the host first, then join from the client.

## Transport and persistence contract

The official environment callback is 78. The protocol identity is
`TGPulse-Next Model 1 M1COMM v1`. Its envelope is `TGMN`, version/type, little-endian
cabinet count, exact-set FNV-1a identity, payload length and the untouched native
frame. HELLO packets establish a complete sorted roster containing frontend
host ID 0. Native frames use reliable delivery plus flush hint to the successor;
only predecessor frames are admitted. Counts/sets must match. Unknown/extra
participants, malformed lengths, incompatible packets and bounded-queue
failures cannot fabricate an online board. The transport FIFO retains up to 1,024 native frames (less than 512 KiB of
payload), separate from the board's unchanged 64 RX slots. Each pump transfers
only available native slots, retaining the remaining frames in FIFO order. Callbacks use a transport mutex
separate from the adapter, released before frontend poll/send calls.

Save RAM remains frontend-owned. **Save States are unavailable whenever COMM
is fitted**, even before a peer connects; `retro_serialize_size()` reports zero
and the native machine's save/load refusal remains intact. Cold reset uses the
native constructor and retains immutable loaded ROM resources only for linked
machines, preserving current audio gains and all NVRAM blocks. It does not
remove COMM to bypass snapshot restrictions. Disabled mode retains the existing
standalone Save State/reset path. A topology change reloads the Model 1 ZIP on
Restart, so standalone play does not retain duplicate ROM resources.

## Reusable verification

`tools/test_retroarch_linked.py` adapts SM2's isolated multi-instance runner and
reuses this project's existing bounded RetroArch launcher. It allocates a local
port, creates separate save/state/system/config directories per cabinet,
records owned process IDs immediately and cleans up only its process groups.
Operator choices use the reviewed per-set NVRAM fields; no synthetic COMM
register/status write prepares real-game tests. Reports record the actual core
SHA-256, native role/ID/count transitions and 600 consecutive online frames.

```sh
python3 tools/test_retroarch_linked.py \
  --retroarch /path/to/RetroArch --core /path/to/core \
  --rom /path/to/vr.zip --output /path/to/new/evidence-directory \
  --moltenvk /path/to/existing/libMoltenVK.dylib
```

Add `--cabinets 3 --live` for VR/VFormula's relay. The isolated base runner also
accepts `--linked-cabinets`, `--netplay-role`, `--netplay-host`, `--netplay-port`
and explicit operator `--nvram-setting` values. Do not reuse an output directory.

On 2026-10-02, 47 adapter checks pass, including native two/three-board handshake,
relay frame routing, missing-roster behavior, predecessor-only delivery,
malformed/mismatched packet refusal, receive-queue bounds and option eligibility.
The macOS ARM64 release build passes. All nine single-cabinet ABI sets also
pass NVRAM seeding, save precedence, manual options, reset and exact Save State
restoration at `/private/tmp/tgpulse-netpacket-unlinked-regression`. Save RAM
containers and native EEPROM integrity were checked for all six linked sets.
Real local RetroArch pairs reached
native COMM online for at least 600 consecutive frames for all six eligible
sets; VR additionally passed the three-instance MASTER/SLAVE/LIVE case.
Observed role/ID/count: MASTER `1/1/2`, SLAVE `2/2/2`, LIVE `0/0/2`.
Some later-closing peers explicitly reported native failed status `255` after
another participant exited. Normal exit and screenshots were collected.

Evidence is retained under `/private/tmp/tgpulse-netpacket-` followed by
`vr-two`, `vr-live`, `vformula-two`, `wingwar-two`, `wingwaru-two`,
`wingwarj-two` and `wingwar360-two`. `linked-report.json` summarizes each run;
per-cabinet `result.json`, `run.log`, config, Save RAM and PNG retain details.
Temporary paths are local evidence, not a permanent archive.
VR two/LIVE and Wing War used SHA-256
`f03dc110c37b6e2298fba7ec041cdaa362b217e6bf3878e7239fdcb55b23546a`.
Remaining sets used the final build, which additionally handles native receive
errors as terminal session failures:
`c30b8558328611326793ec811e2aab9556845bad1dffcf7a2b32a0372d1b77c9`.
The Development installer verifies this final core and matching info SHA-256
`e500861081e28bc565fc3ed4772e6153743f993b012de9374c16cb0ce6295d8b`.

These checks establish actual frontend transport and game-created links. They
are not acceptance of a synchronized race/dogfight, physical controllers,
eight-player operation, mixed ROM revisions or a remote Batocera/LAN session.
Implementation status lives only in the [single roadmap](LIBRETRO_ROADMAP.md).

## GitHub 0.1.0.6 Distributed Queue Failure — 2026-10-05

A real user VR session between the Batocera host and macOS client connected,
then the adapter invalidated the linked session. Read-only inspection confirmed
matching 0.1.0.6 cores, the same ZIP CRC, two cabinets, 60 Hz timing, MASTER/RED
and SLAVE/ORANGE. Batocera used Ethernet (`eth0`); `wlan0` was down.

The failure was reproduced with the existing GitHub cores in isolated frontend
runs, Vulkan, VSync enabled and no inactive-window pause. Roster and native
MASTER/SLAVE IDs became valid, then the Mac client reported session failure.
A diagnostic-only Mac build with unchanged protocol/runtime identity and extra
failure logging confirmed **64 pending native frames in the adapter receive
queue**, followed by its overflow branch and COMM status `255`.
The diagnostic dylib remained in `/private/tmp`; installed cores were unchanged.
The host's sustained-online marker alone did not establish both cabinets online.
Successful screenshot/process exits also did not constitute a linked-session pass.

This identifies the failure branch, not why deliveries accumulate. Wi-Fi jitter,
frontend stalls and burst polling still need separation before selecting a fix.
SM2's `src/libretro/netpacket.cpp` stages incoming packets and drains them at the
core boundary; its queue handling was inspected for the next adaptation.
Retaining native frame order and preventing overflow at the machine queue are
required; increasing only the adapter limit would not establish correctness.

Evidence:

- Existing cores: `/private/tmp/tgpulse-netplay-macos-0106/run.log` and
  `/private/tmp/tgpulse-netplay-batocera-results-b/host/run.log`.
- Diagnostic client: `/private/tmp/tgpulse-netplay-macos-0106-diag/run.log`.
- Corresponding host: `/private/tmp/tgpulse-netplay-batocera-results-c/host/run.log`.
- First remote launch lacked the established graphical environment and failed
  before connection. Subsequent launches used `DISPLAY=:0.0` and
  `XDG_RUNTIME_DIR=/run/user/0`; that infrastructure failure is separate.

The existing frontend runner now accepts `--vsync` for bounded distributed
runs. Persistent settings, content and saves remain outside these fixtures.
No commit/push or replacement of installed cores was performed.

## Burst Queue Correction And Verification — 2026-10-05

SM2 `src/libretro/netpacket.cpp` retains reliable incoming packets separately
and lets the board consume them through `recv()`. Supermodel
`Src/OSD/libretro/LibretroNetBoard.cpp` likewise stages frontend delivery before
board consumption. Model 1 retains its existing wire protocol and VINT timing,
with a bounded 1,024-frame transport FIFO and capacity-aware transfer into the
unchanged 64-slot native RX queue. `CommBoard::receive_capacity()` is a pure
host-boundary query; it does not tick, consume or rewrite emulated state.
Frames are neither discarded nor reordered. A genuinely exhausted bounded
transport queue still fails explicitly rather than silently corrupting traffic.

Verification completed:

- Three adapter tests: 300-frame burst delivered in exact order, unchanged ring
  behavior including LIVE, admission/protocol/overflow boundaries.
- Eight native COMM tests passed; serialized state layout is unchanged.
- Existing macOS native ABI/dependency/lifecycle gate passed. Its name check now
  compares the runtime name with actual package `corename` instead of enforcing
  the former incorrect hardcoded name.
- Offline VR, 600 frames: video/PCM/full state/Save RAM identical to GitHub .6.
- Real macOS/Batocera VR: Vulkan, VSync, 60 Hz, two cabinets with MASTER/RED and
  SLAVE/ORANGE; both reached 600 consecutive online frames. Mac ran 5,700 frames
  (92.364 s), Batocera 6,600 (110.583 s). Neither reported queue failure or lost
  the native link before the bounded host shutdown. The final client disconnect
  and subsequent `255` status follow that intentional shutdown.

These are linked-transport/COMM checks, not synchronized racing or controller
acceptance across every title. No Wi-Fi causality claim follows from them.
The Linux artifact was compiled in the existing amd64 builder with the same
Makefile recipe; dependencies are libc, libm and libgcc_s, without missing libs.

Artifacts/evidence:

- Mac core SHA-256: `2b87edabdd849c70445a764577d8f9985bf7751a86d93e89e453356a490c25eb`.
- Linux core SHA-256: `9b822d9ad33b2d261ac0011bd4da1a81101864958edda36e3361f37a93529bcb`.
- `/private/tmp/tgpulse-netplay-fixed-comparison.json`
- `/private/tmp/tgpulse-netplay-fixed-macos-results/`
- `/private/tmp/tgpulse-netplay-fixed-batocera-results/`
- `/private/tmp/tgpulse-netplay-fixed-development-install.json`

The macOS Development core and matching info are installed and hash-verified.
Its display label retains Development; technical `corename` matches runtime
`TGPulse-Next: Model 1` so Netplay can discover it. The installer backs up the
metadata cache for regeneration on the next full launch. Batocera's corrected
build remains staged at
`/userdata/system/codex-core-tests/tgpulse-netplay-fixed-0106/payload/tgpulse_next_m1_libretro.so`;
the installed addon/core, user configuration and user saves were not replaced.
At verification these were unpublished local fixes with version 0.1.0.6;
source commit/push was subsequently authorized. No new release was requested.
Both participants need consistent runtime identities when using
the corrected name; the previous .6 runtime advertises `TGPulse-Next`.

## Public Local Naming Update — 2026-10-05

At the user's request, the corrected macOS build is now installed under
`tgpulse_next_m1_libretro.dylib`, with matching public `.info`.
Core Name: `TGPulse-Next: Model 1`.
Core Label: `Sega - Model 1 (TGPulse-Next)`.
Description is copied unchanged from public source metadata, with no Development
suffix. The previous public files and former Development pair were preserved
under the local RetroArch `backups/tgpulse-public-install-*` directory; only
one public copy remains in the active core/info directories.
The installer and AGENTS.md now preserve this policy for subsequent local builds.
Runtime/info identity and build/installed SHA-256 were verified.
Receipt: `/private/tmp/tgpulse-netplay-fixed-public-install.json`.
This naming update does not publish a release or update the Batocera addon.
The concrete distributed frontend verification uses RetroArch 1.22.2 with
matching corrected core identities; compatibility with another unspecified
RetroArch version cannot be asserted until that version is identified.

## Live Session Recovery — 2026-10-06

Read-only inspection confirmed the installed runtime name and version match:
`TGPulse-Next: Model 1`, `0.1.0.6`. Both selected two cabinets, with Batocera
Master/Red and macOS Slave/Orange. TCP was connected, but that alone did not
establish the game's COMM connection. The installed Batocera binary now comes
from GitHub source `52b74e9`; it contains the queue/discovery fixes above.

After closing the original Mac client, a bounded fresh client connected to the
still-running host but never formed a complete roster: native status remained
`[5, 2, 0, 0]`. The host's original adapter messages were not retained and the
Mac adapter stderr went to `/dev/null`; the exact first failure branch cannot
be recovered from those logs. Source inspection confirms a participant loss
latches the transport failure until a new session. Reloading only the native
machine does not clear that host transport latch.

With authorization, both original games were closed and the existing installed
cores were exercised through the established isolated runner. Each case used
fresh frontend sessions and copies of the actual same-set NVRAM:

| Case | macOS Timing | Batocera Timing | Result On Both Cabinets |
| --- | --- | --- | --- |
| Mixed | Native | 60 Hz | Complete roster; Master `[1, 1, 1, 2]`, Slave `[1, 2, 2, 2]`; at least 600 consecutive online frames |
| Matched | 60 Hz | 60 Hz | Same complete roster and sustained online status |

No queue overflow occurred. Both cases ran 1,200 client frames and 2,400 host
frames and exited cleanly. Loss/status `255` after an intentionally bounded
peer exit is expected; it is distinct from failure to establish the link.
The mixed case excludes a timing mismatch as a necessary cause of this startup
failure; these bounded runs do not establish long-term mixed-cadence gameplay.
Captured images show normal attract rendering. This is COMM/transport evidence,
not acceptance of a synchronized race or physical controller inputs.

The old session's failed reconnection and both successful fresh pairs support
coordinated session reopening as the recovery for this incident. Installed
cores and personal settings were not replaced; test outputs remained isolated.
No emulator source change or release was required for this diagnosis.

- Mac core SHA-256: `2b87edabdd849c70445a764577d8f9985bf7751a86d93e89e453356a490c25eb`.
- Batocera core SHA-256: `0d177c686abe8a0e22423ef4fdfd4517131ab95fcf186588ebab748909121534`.
- Failed existing-host reconnection: `/private/tmp/tgpulse-netplay-reconnect-native-20261006/`.
- Both fresh pairs: `/private/tmp/tgpulse-netplay-recovery-20261006/comparison.json`
  and its `mixed/` and `matched/` evidence directories.

Required reasoning: High. Shared account usage: 70%; reset 2026-10-10 09:49 CEST.

## Automatic Network Settings Verification — 2026-10-06

References inspected: SM2 `src/libretro/core_options.h`, `core.cpp` and
`PORTING_PLAN.md` 3.7–3.8; Supermodel `Src/OSD/libretro/libretro.cpp`,
`LibretroNvramSettings.h` and `Docs/ROADMAP.md` default/override policy.
Reuse per-set frontend option registries, native NVRAM patches and reset after
persisted changes. The smallest adaptation adds user-approved presets in the
adapter; no equivalent automatic Master/color preset was found in those paths.

The published 0.1.0.9 baseline passed 83 adapter tests, including adjacency/default checks and every preset
(including Live) on all six native-eligible sets, native integrity and unrelated
operator-field preservation. The existing NVRAM ABI runner's new focused mode
verified VR's eight colors, Live restoring default Red, frontend option synchronization,
Disabled followed by OFF, active automation on OFF, and manual overrides while
automation is Disabled. Native bookkeeping can advance during frames, so the
Disabled equality check covers operator EEPROM rather than all live backup RAM.
This is API/NVRAM evidence, not a new multi-instance gameplay/controller test.

The published macOS release build passed native ABI/dependency/lifecycle and Model 1 scope
checks and was installed locally with matching SHA-256. Evidence:
`/private/tmp/tgpulse-automatic-network-tests.log`,
`/private/tmp/tgpulse-automatic-network-live-red-20261006/result.json`,
`/private/tmp/tgpulse-automatic-network-artifact.log`,
`/private/tmp/tgpulse-automatic-network-install.json`.
Release 0.1.0.9 is published; see LIBRETRO_CI.md for CI and package evidence.

```sh
python3 tools/test_libretro_nvram_settings.py \
  --core tgpulse_next_m1_libretro.dylib --rom-dir /path/to/model1/roms \
  --automatic-network-only --output /path/to/new-isolated-results
```

## macOS Startup and NVRAM Authority Correction (Local 0.1.0.10 Candidate)

Release 0.1.0.9 can stall when an existing VR operator option differs from
the active Automatic Network Settings preset. RetroArch's `SET_VARIABLE` can
synchronously call the core's option-display callback. Publishing a managed
option while `retro_run` holds the core mutex then re-enters that mutex. Queue
managed option updates during NVRAM application and publish them only after
`retro_run` releases the lock.

NVRAM Settings is now the master switch for both manual selectors and
Automatic Network Settings. When Disabled, the automatic option is hidden and
does not write. When Enabled, an active preset first sets and maintains its
managed NVRAM Settings selectors; the existing NVRAM Settings path applies
their displayed values to Save RAM. This happens on startup and option changes,
including Linked Cabinets changes. With automation Disabled, no automatic
selector changes occur, including when Linked Cabinets goes OFF; manual NVRAM
selectors remain authoritative. Automatic Initial NVRAM Setup is a separate
first-save feature and is unchanged.

On macOS ARM, an isolated 30-frame VR launch with NVRAM Settings enabled and
`vr_link_id=SLAVE` timed out using the published 0.1.0.9 core. The same launch
completed in 1.39 seconds with the fix. A 60-frame Vulkan launch using a copy
of the user's VR Save RAM completed in 1.59 seconds. The copy was isolated and
the original save was not modified. All 83 adapter tests passed. This is
startup regression evidence, not a full linked-cabinet gameplay test.

An earlier local 0.1.0.10 candidate passed 84 adapter tests and focused ABI
checks for master-switch gating, preset authority, Disabled preservation, all
eight VR colors and Live. A 60-frame Vulkan launch using an isolated copy of
the user's VR Save RAM completed in 1.54 seconds. The macOS artifact gate
passed and that candidate was installed in local RetroArch with matching SHA-256
`0ac84114724a1b7073e8c9707df99a2a574c9aa51b7e196904cdf7d565753001`;
the metadata SHA-256 is
`e7ddd4c8355b22f2c6c2810c27443eae9b0b29a2984cc2f67ddd2912ba5cab00`.
These are startup and option regression checks, not a linked-cabinet gameplay test. Evidence:
`/private/tmp/tgpulse-0110-authority-tests-final.log`,
`/private/tmp/tgpulse-0110-authority-abi-2/` and
`/private/tmp/tgpulse-0110-master-automatic-vulkan/`.

## Linked Cabinets on Restart (Local 0.1.0.10 Candidate)

RetroArch Restart now rereads the current set's Linked Cabinets option. When
the total changes, the adapter rebuilds the machine with or without M1COMM,
retains its Save RAM and updates the Netpacket packet count. The transport
keeps the active frontend callbacks and host client slots, then starts a fresh
HELLO handshake. A same-total Restart also clears a failed core transport
state. Automatic Network Settings continues to select the matching role through
NVRAM Settings.

The VR ABI test opened a mock lobby before changing the option and verified
Restart transitions 1 → 2 → 3 → 1, live callback delivery after each linked
transition, COMM insertion/removal, Master/Red while linked and No Link/Red
after returning offline. 85 adapter tests pass. A separate real two-instance
RetroArch regression established the VR COMM roster and sustained game-created
online state for more than 600 frames on each participant. The real two-instance
run started with the linked option already selected; the option-change Restart
sequence was exercised through the ABI with a simulated open lobby. Evidence:
`/private/tmp/tgpulse-linked-restart-20261006-final/` and
`/private/tmp/tgpulse-linked-restart-baseline-20261006-c/`.

The earlier local macOS build passed the artifact gate and was installed in
RetroArch with matching core SHA-256
`cbb98bf2e2031ff745edace605900fef1ec25b784771517e180a955c3c0c99c1`.
The installed `.info` SHA-256 is
`e7ddd4c8355b22f2c6c2810c27443eae9b0b29a2984cc2f67ddd2912ba5cab00`.

## First Restart Role and Netpacket Notice Correction

On a topology Restart, an active Automatic Network Settings preset now applies
its managed NVRAM selector values before the first emulated frame. The adapter
publishes matching Core Option values after releasing its mutex. This prevents
VR from booting one frame with the previous role and then resetting again as
the frontend catches up. NVRAM Settings remains the master switch; Disabled
automation preserves manual selectors.

The early "frontend has no Netpacket support" notice was removed. The
registration response is not a reliable indication that a subsequent Netplay
session cannot start; an actual session and game-created COMM state provide
the useful evidence. Transport failures still report through the existing
runtime error path.

The focused ABI regression verified Slave/Pink NVRAM after the first frame of
Restart from standalone, 1 → 2 → 3 → 1 topology changes, callback preservation
and offline restoration. A separate real two-instance RetroArch test using
Automatic Network Settings reached sustained game-created COMM online on both
cabinets, including a run with the Slave joining ten seconds after the host.
The exact in-menu option-change Restart sequence remains for a frontend trial.
Evidence: `/private/tmp/tgpulse-linked-restart-atomic-20261006/`,
`/private/tmp/tgpulse-linked-automatic-20261006/` and
`/private/tmp/tgpulse-linked-automatic-delay-20261006/`.

The corrected local build passed the macOS artifact gate and is installed in
RetroArch with matching core SHA-256
`4c88ed34e2d351e1bd3703120d10fbd5a9a9809bd5bb454cc370c3140d037b26`.
Its installed `.info` SHA-256 is
`e7ddd4c8355b22f2c6c2810c27443eae9b0b29a2984cc2f67ddd2912ba5cab00`.

## Resume Before the First Linked Frame (Local 0.1.0.10 Candidate)

The reported white-line sequence was **2 Cabinets → change Automatic Network
Settings → Resume → automatic reset** with both frontends already in a lobby.
Previously, Resume applied the linked NVRAM role and reset the machine while
COMM was still absent; fitting COMM required a separate Restart. Resume now
rebuilds the machine with COMM before applying the role and advancing the next
native frame. It preserves backup RAM and EEPROM across that rebuild.

A lobby can accept the peer before the host selects its cabinet count. During
staggered setup, packets with the previous count are ignored until both peers
match; a different game/set hash remains a failure. The focused ABI test
reproduces the exact Resume sequence, checks COMM and Slave/Pink NVRAM before
the first frame, and verifies a staggered peer joins the retained lobby.
Evidence: `/private/tmp/tgpulse-linked-resume-final-20261006/`. All 86 adapter
tests and the macOS artifact gate pass. A separate real two-instance RetroArch
regression reached game-created COMM online for more than 600 frames on both
participants: `/private/tmp/tgpulse-linked-resume-regression-20261006/`.
That run started with options already selected; the exact in-menu interaction
was exercised by the focused ABI test, not the two-instance frontend run.
The verified core was installed locally with matching SHA-256
`e1c33a2ab27f2001faddab1212c832cfc965ca6d9482a7e490f979253e12b720`.
The matching `.info` SHA-256 is
`e7ddd4c8355b22f2c6c2810c27443eae9b0b29a2984cc2f67ddd2912ba5cab00`.
