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

`Linked Cabinets (Restart Required)` is registered with an independent key for
each supported set, defaults Disabled and is visible only for the current set.
A selection above Disabled fits the native COMM board and expects the selected
number of RetroArch Netplay participants. If Netpacket is unavailable, the core
reports it and keeps COMM disconnected; no local echo supplies a missing peer.

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

- With Linked Cabinets enabled, apply the selected native role and identity.
- With Linked Cabinets OFF and automation enabled, restore only managed fields
  to reviewed defaults: No Link + Red/No.1 (Red), or Wing War Stand Alone.
- Disabled performs no automatic writes, including when Linked Cabinets goes
  OFF. Disabling automation preserves values already applied.
- Automation is independent of the NVRAM Settings master switch and synchronizes
  its managed option values through Libretro SET_VARIABLE. Disable automation
  for manual control, including any custom native role/color combination.
- Changed persisted values use the existing checksum/mirror and machine-reset
  path. Reload all participants after topology changes. Netplay host/client
  selection never determines Master/Slave/Live.

The reviewed values and encodings are reused from `nvram_data.rs`; no campaign
samples or upstream emulation components are changed. Color identities need
only be unique among playable cabinets; they are not numbered Netplay slots.

### Connect two cabinets

1. Use the same exact ROM set and core build on both frontends. Keep each
   frontend's saves and configuration separate.
2. Select **2 Cabinets** on both, then reload content.
3. Choose Automatic Network Settings presets: Red (Master) and Orange (Slave)
   for VR/VFormula, or Master and Slave for Wing War. For manual NVRAM/service
   configuration, set automation to Disabled and use distinct playable identities. The Netplay host can be either native role; host does not
   automatically mean MASTER.
4. Host a RetroArch Netplay session on one frontend and connect the other to it.
5. Let the games perform their native COMM startup. A transport roster alone
   is not proof that the game has established its link.

For MASTER/SLAVE/LIVE, select **3 Cabinets** everywhere and set LINK ID = LIVE
on the third VR/VFormula instance. Automatic Network Settings can select Master, Slave and Live independently
on each instance. Disable automation before applying custom NVRAM Settings
or using saved service-menu values. No role is assigned by transport.

Restart/reload all cabinets after changing topology or roles during a session.
For standalone VR operation, set **Linked Cabinets = Disabled** and
**NVRAM Settings → Link ID = No Link** (with NVRAM Settings enabled), then
restart/reload. Disabling the transport does not change the saved native role.
A saved Slave role without a fitted COMM board produces the game's horizontal
white-line screen with **CANCELLED**. This was reproduced on macOS/Vulkan on
2026-10-06 using an isolated copy of the affected save: two 900-frame runs
differed only in Link ID (Slave versus No Link). No Link restored the normal
attract scene. Personal configuration and saves were preserved; evidence is
`/private/tmp/tgpulse-vr-stripes-current/` and
`/private/tmp/tgpulse-vr-stripes-no-link/`. The tested core SHA-256 is
`2b87edabdd849c70445a764577d8f9985bf7751a86d93e89e453356a490c25eb`.

Individual resets preserve backup RAM/EEPROM and recreate the native machine;
they do not rewind peers or establish a coordinated session reset. After a lost
participant, close/reload the game and restart Netplay on every cabinet. Start
the host first, then join from the client; reconnecting only one client does
not recover a host transport already marked failed.

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
standalone Save State/reset path.

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

83 adapter tests passed, including adjacency/default checks and every preset
(including Live) on all six native-eligible sets, native integrity and unrelated
operator-field preservation. The existing NVRAM ABI runner's new focused mode
verified VR's eight colors, Live restoring default Red, frontend option synchronization,
Disabled followed by OFF, active automation on OFF, and manual overrides while
automation is Disabled. Native bookkeeping can advance during frames, so the
Disabled equality check covers operator EEPROM rather than all live backup RAM.
This is API/NVRAM evidence, not a new multi-instance gameplay/controller test.

The macOS release build passed native ABI/dependency/lifecycle and Model 1 scope
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
