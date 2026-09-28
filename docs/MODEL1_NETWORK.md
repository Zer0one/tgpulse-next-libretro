# Model 1 cabinet link

## Status — desktop TCP/settings checkpoint, 2026-09-27

The M1COMM HLE is connected to the V60 bus/VINT; the desktop now provides an
optional TCP ring and persistent settings/GUI controls. **TCP/board exchange is
tested, including VR boot with operator-configured MASTER/SLAVE/LIVE NVRAM;
an actual synchronized VR race is not yet validated.** Networking defaults off.
An absent peer is not replaced with a fake successful link. No NVRAM role or
cabinet option is overridden.

This follows MAME's **active `M1COMM_SIMULATION` path**, not execution of the
communication board's Z80/MB89237A/MB89374 firmware. Adding a second low-level
implementation would not reproduce the reference currently used to run games.
The I/O board and its Z80 are separate devices; the new I/O Z80 core is not needed
for this HLE implementation. Model 2's existing COMM implementation is unchanged.

## Sources actually inspected

| Reference | Revision | Relevant implementation |
| --- | --- | --- |
| MAME local checkout | `bd7e0b815842ec461e8ad2538d127f3332f5c96c` | `src/mame/sega/m1comm.{h,cpp}`, `model1.cpp`: active HLE, registers, board presence, VINT and memory map |
| SM2-Emu standalone | `af0be801980e40eb1f7eeff7f72ecc2f8ffe1024` | `src/hw/comm_transport.h`, `comm_udp.h`: complete-frame, nonblocking host transport boundary |
| SM2-Emu Libretro | `75ced234792520c325aa63aa7be3dfaf87b07d46` | `src/hw/comm_transport.h`, `src/libretro/netpacket.{h,cpp}`: readiness, frontend-owned connection lifecycle and packet delivery |
| Supermodel standalone sources retained in libretro-supermodel-modern | `4566c9470ab89ec8e2738bfb43c18281df580971` | `Src/OSD/SDL/{Main,Gui}.cpp`, `Src/Network/SimNetBoard.cpp`: Network, incoming/outgoing ports and outgoing address; TCP transport |

The MAME board algorithm is adapted into this implementation. Its Ariane Fugmann
BSD-3-Clause attribution is retained in `model1comm.rs` and
`LICENSES/MAME-BSD-3-Clause.txt`. The other sources inform boundaries and the
frontend; no Model 2/3 protocol, transport implementation or incomplete
Supermodel save-state implementation has been copied into the Model 1 board.

Existing TGPulse components reused: Model 1 byte-bus dispatch, VINT scheduling,
identified ROM-set metadata, `serde`/`bincode` device-state patterns and the
existing workspace tests/debugger. Model 2's COMM was inspected but its 16 KiB
layout, role selection and packet handling differ: reusing that board directly
would be incorrect. No new dependency is introduced.

## Reference contract

- Board eligibility follows MAME: `vr`, `vformula`, `wingwar`, `wingwaru`, `wingwarj`,
  `wingwar360`. Presence does not establish gameplay support (especially R360).
  VF, SWA/SWAJ and NetMerc are not assigned an M1COMM board.
  TGPulse's `cabinet = twin` fits the board for eligible sets; `single` (default)
  leaves it absent. This is an explicit host cabinet selection, not a claim that
  MAME dynamically removes the board. It never rewrites Model 1 operator roles.
- Shared RAM: all 4 KiB at V60 `B00000–B00FFF`, both byte lanes. CN at `B01000`,
  FG at `B01002`; unmapped/odd register bytes return `FF`.
- CN=0 disables the board and clears ZFG, retaining RAM, FG and link staging.
  CN=1 restarts ID allocation with a 232-VINT delay. The delay advances only when
  both host links are ready. FG writes are ignored while CN is disabled.
- Game-written RAM byte 1 selects master=1, slave=2 or relay=0. Frontends must not
  substitute Model 2's FG-based role selection. The game owns its operator setup.
- Wire frames are exactly 453 bytes: ID plus 452-byte payload. `FF` allocates IDs,
  `FE` announces node count, `FD` carries the master's ten additional bytes,
  `FC` carries VINT synchronization. Unused control-frame tails retain the last
  buffer contents, as in MAME.
- The relay forwards traffic but takes no player ID. A node consumes its own
  returning payload; other nodes store it at `0x10 + ID * 0x1C4` and forward it.
  Own outgoing data is at `0x10`; RAM[4] gates sending and is not cleared by COMM.
  RAM[5] is cleared at the end of an online tick.
- Status RAM[0]: waiting `05`, online `01`, failed `FF`. Online ID/count are at
  RAM[2]/[3]; ZFG remains asserted, unlike the toggling Model 2 implementation.
- VINT ticks regardless of the V60 IRQ mask. This checkpoint follows MAME's
  default `comm_framesync=0`; it does not introduce a busy-wait into the core.

## Host boundary and state

`CommBoard::receive` accepts a complete incoming frame; `take_transmit` returns
one outgoing frame. A frontend must assemble TCP fragments outside the core,
send queued frames to the next node, and report readiness/loss with
`set_connected`. There is **no automatic local echo**, socket or wall-clock
timeout. The tests connect distinct board instances explicitly in a ring.

Intentional defensive adaptations: bounded 64-frame queues and checked packet
length/node count. The fixed RAM layout accommodates at most eight participant
IDs. Invalid counts/overflow fail explicitly instead of writing past RAM or
growing an unlimited host queue. This is not a hardware queue-depth claim.

The versioned, validated, in-memory `State` includes RAM, registers, link state,
ID/count, timer, retained packet buffer and pending complete frames. Restore is
atomic and leaves the host disconnected; it does not replay register writes,
open sockets or write NVRAM. Host transport resources and partially assembled
TCP frames belong to the frontend. Restoring a live distributed session requires
coordinated peer/transport state, not restoring one cabinet in isolation.
This is device serialization, **not complete Model 1 machine save states** or an
implemented Libretro adapter.

## Verification

- Eight ROM-free board tests cover exact handshake frames/delay, ring ID/count,
  master/slave/relay payload delivery, additional master bytes, CN/FG semantics,
  missing peer, lost connection, malformed frames, queue overflow, node bounds,
  atomic invalid-state rejection and identical continuation after serialization
  during handshake/online traffic, including pending transmission.
- A ninth motherboard test checks 8/16/32-bit mapping, absence of the board,
  register byte lanes, the final RAM word and VINT with V60 interrupts masked.
- `cargo test --offline --workspace`: 302 tests passed. Offline release build
  passed. These are implementation tests, not a MAME/TGPulse interoperability run.
- Real-ROM debugger probe: VR completes 120 and 1,800 frames with the existing
  standalone NVRAM, no saves written. At frame 120 the V60 PC (`00FE1435`) and
  empty TGP FIFOs match the pre-change baseline. CN remains `FE` (disabled), FG
  `FE`; the newly mapped shared RAM reads zero rather than unmapped `FF`.
  At 1,800 frames PC is `00FE13E2`, both FIFOs empty. This checks standalone boot
  execution, not linked gameplay or fresh visual/audio validation.

## Desktop adapter and GUI checkpoint

`crates/tgpulse/src/network.rs` owns the listener, incoming/outgoing TCP streams,
bounded channels and joinable worker. The frame thread only polls complete
frames; partial reads/writes and connection retries stay on the worker. Connect
attempts have a 50 ms bound and retry every 250 ms while waiting. Session close
or reset stops/joins the worker and releases sockets. A failed established link
requires a game reload/reset, rather than silently replacing peers mid-race.
The session's NVRAM is saved **before** constructing a reset machine, so freshly
edited operator options are not discarded by loading the earlier disk image.

On the wire this is MAME's raw 453-byte framing. Supermodel's TCP helpers prepend
a length field; that Model 3 framing is deliberately **not** reused for M1COMM.
No matchmaking, handshake envelope, automatic loopback, authentication or
encryption is added. Use a trusted local network, not an Internet-exposed port.
Network startup is off by default and the default listener is loopback-only.
Numeric IPv4/IPv6 addresses avoid a blocking DNS lookup during session shutdown.

The standalone Supermodel GUI was checked again before settling the controls:
`Gui.cpp` creates its Networking tab dynamically from the Network group in
`Main.cpp`: `Network`, `SimulateNet`, `PortIn`, `PortOut`, `AddressOut`. TGPulse
uses the same endpoint names, adding MAME's local bind address as `AddressIn`;
activation is handled by the shared cabinet selector, not a second Network toggle.
There is no `SimulateNet` selector because only the HLE implementation exists.
The controls remain in the existing TGPulse Settings window, not a new frontend.
An explicit Apply validates/saves the draft; settings take effect on load/reset.
The status distinguishes TCP connectivity from the game-visible COMM state.
The shared `Network board fitted (twin cabinet)` control now applies to both
Model 1 and Model 2. Model 2's existing behavior is unchanged. On Model 1, board
presence and desktop TCP transport share one setting: `cabinet = single` means
no board/no transport; `cabinet = twin` fits the board and starts TCP for supported
games. The former `model1_network` key and separate Network checkbox have been
removed; remove that key from older personal profiles. Addresses and ports remain
configurable and a missing peer remains a real disconnected link, not loopback.
MASTER/SLAVE/LIVE/NO LINK remain operator choices in the game's NVRAM. Selecting
`single` does not repair or overwrite a NVRAM configured to expect a link.

### Configuration / two local cabinets

Each process needs **its own profile and NVRAM file**. `--config <file>` loads
that profile and GUI adjustments save back to it, not `config/settings.conf`.
The optional `nvram = <file>` selects an existing compatible TGPulse container
for both load and save, including periodic flush, reset and clean exit. Missing
or incompatible explicit files report an error; they never fall back to default
NVRAM. Empty/omitted `nvram` preserves the existing `nvram/<set>.nv` behavior.
Profiles with explicit NVRAM require a CLI romset; switching to another title
is refused to prevent reusing a cabinet file for a different game.

Explicit CLI paths (`--config`, `--roms`, ROM ZIP, debugger `-f`) and the profile's
NVRAM path resolve against the **invocation directory**, not the profile's parent.
Absolute paths stay absolute. The toolkit launchers export `TGPULSE_LAUNCH_DIR`
before changing cwd; direct binary calls use their current directory. Default
library/settings/bindings/state paths retain the runtime cwd. CLI settings take
precedence over profile values regardless of option order.

Examples are in `docs/examples/vr-master.conf` and `vr-slave.conf`. From the
development checkout, after copying a known-good VR NVRAM into each of the two
distinct paths named in these profiles:

```sh
tgpulse.dev --config docs/examples/vr-master.conf vr
tgpulse.dev --config docs/examples/vr-slave.conf vr
```

These are example profiles, not preconfigured MASTER/SLAVE NVRAM images. Copy
the profiles to a personal location before changing them in the GUI. Never run
two processes with the same NVRAM or writable settings file. Bindings and save
states remain shared runtime resources; do not edit/write those concurrently.

These are the new settings (defaults shown):

```ini
cabinet = single
model1_address_in = 127.0.0.1
model1_port_in = 15112
model1_address_out = 127.0.0.1
model1_port_out = 15113
```

For local tests set `cabinet = twin` in all profiles.
Cabinet A listens on
15112 and sends to 15113; B listens on 15113 and sends to 15112. These defaults
use MAME's base port and distinct in/out ports as in Supermodel, so enabling
networking alone does not silently connect the cabinet to itself. For two LAN
hosts, bind to each host's LAN address (or `0.0.0.0` for IPv4) and set AddressOut
to the next host. The outgoing port must equal that host's incoming port.

GUI settings do not alter the game's LINK ID. In VR's GAME
SYSTEM menu, verified choices include NO LINK, LIVE, SLAVE and MASTER; select
MASTER on one cabinet and SLAVE on the other, exit/save and confirm after reload.
Do not confuse LIVE/relay with a second player. Keep normal speed; coordinated
pause/fast-forward and resuming a distributed save state are not implemented.
The standalone debugger still ignores personal settings and does not open LAN;
`--debug --config` is explicitly rejected. The GUI's independent debug machine
is unavailable with custom NVRAM, rather than silently using the default save.

### Verification and limits

- Four desktop transport tests cover fragmented I/O and backpressure without
  changing wire bytes, invalid endpoints, missing peer, two-board handshake/data
  transfer over real loopback TCP, lost peer, bounded teardown and listener reuse.
- Settings round-trip includes non-default network values. A headless ImGui
  render test checks that drawing the draft does not silently apply/save it.
  This is not a manual UI acceptance test.
- The VR operator-menu experiment did **not** produce validated persistent
  MASTER/SLAVE fixtures: after its scripted menu navigation the EEPROM remained
  unchanged and the cold-booted games never enabled COMM. The sequence is not
  used as proof of a transport or game-link failure, nor retained as an automatic
  operator-configuration feature. Existing user NVRAM was read only.
- That experiment exposed two debug-profile V60 overflow traps: downward
  halfword-copy final cursors and absolute scaled negative indexes. Both now
  wrap at 32 bits, matching MAME `opMOVSTRDH` and `am2DirectAddressIndexed` and the
  existing release arithmetic; two focused ROM-free regression tests cover them.
- `cargo test --offline --workspace`: **309 passed**; offline release build
  passes. TCP tests require loopback socket permission. This is not a claim of
  MAME interoperability, physical LAN operation or a synchronized VR race.

An opt-in acceptance probe uses independent operator-configured fixtures;
it does not patch roles/status or save files:

```sh
TGPULSE_MODEL1_ROM=/absolute/path/vr.zip \
TGPULSE_MODEL1_MASTER_NVRAM=/absolute/path/master/vr.nv \
TGPULSE_MODEL1_SLAVE_NVRAM=/absolute/path/slave/vr.nv \
cargo test --offline -p tgpulse model1_preconfigured_cabinets_tcp_link -- --ignored --nocapture
```

It requires the game-created link to remain online for the final 600 frames,
checks roles, IDs and two-node count, and supplies no synthetic COMM success.
The same probe accepts Wing War/R360 ROMs with matching operator NVRAM files.
Optionally add `TGPULSE_MODEL1_LIVE_NVRAM=/absolute/path/live/vr.nv` to include LIVE
in the ring: MASTER -> SLAVE -> LIVE -> MASTER. Ports are dynamically allocated
on loopback, independent of the desktop profiles, and all machines run in one
test process through the real TCP adapter. No GUI is opened.

**2026-09-27 operator-fixture acceptance:** the user-provided, distinct NVRAM
files pass the two-cabinet test and the three-instance LIVE variant, each over
2,400 emulated frames with all links online for the final 600 frames. Observed
COMM role/ID/count: MASTER `1/1/2`, SLAVE `2/2/2`, LIVE `0/0/2`. As in MAME,
LIVE is a relay, not a third numbered participant. Tests use temporary copies;
original SHA-256 values remain unchanged. These are game-created links, not
synthetic status writes. This supersedes the earlier unsuccessful automated
operator-menu preparation; it does not prove a synchronized race, separate
desktop-process timing, physical LAN or MAME interoperability.

Cabinet-selection follow-up: 312 workspace tests pass, including the new
supported/unsupported-set x single/twin presence matrix, absent-board bus reads
and NVRAM retention. The three-instance real-ROM acceptance probe passes again
with explicit `Cabinet::Twin`; original operator NVRAM hashes are unchanged.
Offline release build passes. A headless VR cold-start probe with `single` and
no saved NVRAM runs 1,800 frames to V60 PC `00FE13DB`. The existing default user
NVRAM instead reaches `00FE6E28` with the board absent; this is not treated as
standalone gameplay proof or repaired by overriding operator settings. Configure
NO LINK in the game for standalone operation. No visual/gameplay acceptance is
claimed by these headless probes.

## Next bounded checkpoint

1. Check a synchronized race in separate desktop instances (including LIVE's
   spectator behavior), then on physical LAN. Operator fixtures and boot-time
   game-created links have passed the bounded acceptance probe above.
2. Check MAME interoperability separately. Frame-sync is still MAME's default
   disabled path; assess cooperative synchronization only against demonstrated
   game needs, never by blocking the frontend indefinitely.
3. Diagnose the R360 link-start path described below; validate Virtua Formula later.

### Wing War operator-fixture tests — 2026-09-27

The same opt-in probe is now named `model1_preconfigured_cabinets_tcp_link` and
uses `TGPULSE_MODEL1_*` inputs instead of VR-specific names. Temporary copies of
the user's independent operator NVRAM files are used; no roles/COMM status are
injected and no original file is saved.

- Wing War World: **passed**, 2,400 frames and at least the last 600 online;
  MASTER role/ID/count `1/1/2`, SLAVE `2/2/2`. This validates a game-created
  two-node link through real loopback TCP, not synchronized human gameplay.
- Wing War R360: **failed the same acceptance gate**. Both machines advance
  into their usual loop, but COMM bytes 0..5 remain zero through frame 2,100
  and the final online assertion fails. Do not classify this as a proven TCP
  transport bug or claim R360 networking works. A repeated diagnostic shows CN/FG
  `FE/FE` and EEPROM rewritten by the game in memory: input header bytes
  `53 33 32 41 46 30` become `53 33 32 41 63 04`; multiple settings are reset,
  including the original differing bytes at offsets 24/25 (hex). No disk save
  occurs. First diagnose why R360 reinitializes these fixtures; do not assume
  either an operator mistake or a transport defect. Do not bypass it with fake
  ready flags. Both failures are reproducible over 2,400 frames; original file
  hashes remain unchanged.

Profile checkpoint: 311 workspace tests pass, including isolated NVRAM read/write,
invalid/missing file rejection, profile round-trip, CLI precedence and explicit
path resolution. Offline release build passes. A CLI-only smoke through installed
`tgpulse.dev` from an unrelated temporary directory verifies relative profile and
ROM-directory resolution and missing-profile diagnostics without accessing saves.
The toolkit launchers are maintained/deployed in their separate
repository; no current/rolling binary, dependency or user NVRAM is changed.
These checks do not establish a real two-cabinet VR race.
