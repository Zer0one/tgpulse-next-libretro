# Framework Network Integration

User authorized production integration on 2026-10-14 after the
[isolated feasibility study](NETWORK_LOOPBACK_FEASIBILITY_2026-10-14.md).
Reference: Framework 0.1.18 F14, SM2 current ANS reference alignment and
`M2Comm::set_transport(nullptr)`, Supermodel's current Network Board / Linked
Cabinets roadmap and adapter implementation. This report supersedes the
feasibility report's pending-integration status; its historical results remain.

## Delivered Contract

- Network Board is a global System option, Enabled / Disabled, default Enabled
  like Supermodel. Existing Linked Cabinets/ANS keys, choices, value labels and
  defaults are unchanged. V2 and legacy frontends register the new selector.
- Disabled removes COMM from supported Model 1 machines; Linked Cabinets and
  ANS become hidden without altering their saved selections. Active ANS still
  prepares reviewed offline native settings with NVRAM Settings enabled.
- Enabled with Linked Cabinets Disabled uses explicit complete-frame local
  loopback. This does not invoke frontend network send/poll or synthesize native
  protocol success. Missing peers in an external topology do not trigger echo.
- Supported totals >= 2 retain the existing native ring and selected ANS role.
  Offline topology overrides that role without changing the saved ANS choice.
  ANS Disabled preserves manual authority; NVRAM Settings Disabled prevents
  automatic NVRAM writes. Native game writes remain independent.
- Board and topology changes recreate/reset the machine automatically on Resume,
  before managed NVRAM changes and the next native frame. No manual reload is
  required. Open frontend lobby callbacks are retained.
- Paused visibility uses the existing update callback, including a cached board
  state synchronized at load and resumed updates. An integration probe exposed
  an initially stale cache across content loads; synchronization fixes the
  callback's changed return value, not only the emitted display flags.

The native CommBoard, firmware simulation, CPUs and standalone are unchanged.
Save States retain the existing board-present restriction, including local
loopback. Because the new board defaults Enabled, users of supported titles
must disable Network Board to use Save States. No new state format is introduced.

## Verification And Delivery

93 adapter tests passed. Full registration comparison confirms existing choices,
labels, defaults and categories preserved; new Network Board registration matches
in V2 and legacy modes. Linked Cabinets remains directly before ANS.

The final six-set ABI probe covers `vr`, `vformula`, `wingwar`, `wingwaru`,
`wingwarj`, `wingwar360`: present local board, NoLink despite retained ANS Slave,
automatic board removal/restoration on Resume, hidden/restored dependent controls,
paused visibility change signaling, retained selections, linked Slave restoration,
return to offline defaults and NVRAM-off EEPROM preservation. Save RAM container
integrity passes. Device echo is independently tested with client ID 5 and
forbidden frontend send/poll callbacks; existing native ring tests remain passing.

A 1,200-frame isolated VR RetroArch run exits normally and saves an attract
image. It used the integrated candidate before the final cache-synchronization
fix; the final build is covered by the complete ABI probe and artifact checks.
The earlier feasibility study separately covers offline Virtua Formula and
Wing War in actual RetroArch. No new interactive menu, physical-controller,
full topology/multiplayer or Windows/Linux qualification is claimed.

macOS ARM64 release, 25-export ABI/dependency/scope checks and three empty
lifecycle cycles pass. Installed public core and matching metadata with backup;
build and installed-core SHA-256:
`ebcdee0e157cb2bee931749878e58eba6233f9b3b759fa38f1f143610c96aab3`.
The standalone build was not modified. No commit, push or release was made.

Generated evidence: `outputs/network-integration-20261014/`, including final
ABI probe/log, registration snapshot, artifact results, test log, installation
manifest and isolated frontend result. Runner paths/configurations are isolated;
only its owned process was launched and awaited. No frontend belonging to
another chat was controlled or closed.
