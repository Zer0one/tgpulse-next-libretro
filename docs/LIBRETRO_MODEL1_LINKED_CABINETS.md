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

### Connect two cabinets

1. Use the same exact ROM set and core build on both frontends. Keep each
   frontend's saves and configuration separate.
2. Select **2 Cabinets** on both, then reload content.
3. Configure MASTER on one cabinet and SLAVE on the other through the existing
   NVRAM Settings fields or native service menu. For VR/VFormula choose distinct
   car identities. The Netplay host can be either native role; host does not
   automatically mean MASTER.
4. Host a RetroArch Netplay session on one frontend and connect the other to it.
5. Let the games perform their native COMM startup. A transport roster alone
   is not proof that the game has established its link.

For MASTER/SLAVE/LIVE, select **3 Cabinets** everywhere and set LINK ID = LIVE
on the third VR/VFormula instance. Keep automatic setup enabled if desired:
it initializes new saves with its reviewed offline policy; explicit NVRAM
Settings then apply the selected operator roles. With NVRAM Settings disabled,
a valid saved service-menu role is preserved. No role is assigned by transport.

Restart/reload all cabinets after changing topology or roles during a session.
Individual resets preserve backup RAM/EEPROM and recreate the native machine;
they do not rewind peers or establish a coordinated session reset. After a lost
participant, reload the linked session rather than replacing it silently.

## Transport and persistence contract

The official environment callback is 78. The protocol identity is
`TGPulse-Next Model 1 M1COMM v1`. Its envelope is `TGMN`, version/type, little-endian
cabinet count, exact-set FNV-1a identity, payload length and the untouched native
frame. HELLO packets establish a complete sorted roster containing frontend
host ID 0. Native frames use reliable delivery plus flush hint to the successor;
only predecessor frames are admitted. Counts/sets must match. Unknown/extra
participants, malformed lengths, incompatible packets and bounded-queue
failures cannot fabricate an online board. Incoming queues are limited to 64
frames, matching the native host boundary. Callbacks use a transport mutex
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
