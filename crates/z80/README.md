# Z80 integration adapter

Based on the already-used crates.io `z80` **1.0.2** by Kirjava:
`https://github.com/kirjavascript/z80`. `src/z80.rs` retains the original MIT
license and authors. It was copied from the existing Cargo cache, not downloaded
or installed. Keep unrelated opcode code unformatted to make the delta readable.

After consolidation the local package is named `z80` (formerly `tgpulse-z80`).
Dependencies use an explicit path to this directory, not the registry package.
This is now the shared CPU implementation for `model1io::IoBoard`,
`model1io2::IoBoard` (Wing War and R360 wiring) and the Star Wars DSB. The
registry `z80` dependency was removed during the user-requested consolidation;
the original board retains its existing bus and cycle-debt scheduler and uses
the optional hooks' defaults. No opcode or timing rewrite accompanies migration.
Model 2 currently uses high-level I/O/drive handling, not either Z80 package;
its sound CPUs are 68000. No Model 2 CPU implementation was replaced.
See [the consolidation checkpoint](../../docs/MODEL1_ROADMAP.md#consolidate-the-z80-implementations)
and board integration contract for verification boundaries. NetMerc remains a
separate milestone. Unifying CPU code does not complete machine save states.

Original `z80.rs` SHA-256:
`e95042b2c07cadab457ffef155f87df29499bfb9cb96dc386623002b4602dc75`.
Only trait hooks, interrupt/step handling, canonical RETI and the state-module
include differ in that file; snapshot code is separate in `src/state.rs`.

Local changes:

- Live IRQ line and acknowledge callback, invoked only on CPU acceptance.
- RETI notification and IFF2 -> IFF1 restoration for canonical ED 4D.
- EI defers maskable IRQ only, without suppressing a pending NMI. The installed
  version returned before checking NMI when processing the EI delay.
- Emulated clock callback at instruction/interrupt boundaries, not host time.
- Explicit typed CPU snapshot/restore (including alternate registers, flags,
  pending interrupts, EI delay, HALT and internal memory pointer), excluding bus
  and host resources. Not a versioned whole-machine state format.
- Isolated integration tests; upstream optional CP/M exerciser assets are not
  bundled or required by the workspace test suite.

Run `cargo test --offline -p z80`. The 18 synthetic tests cover disabled
IRQ, DI/EI, live IM2 vector selection, IM1 acknowledgement, NMI priority, HALT,
canonical RETI versus RETN, typed state validation and continued execution
across 16 snapshot points in a block-copy/interrupt/I/O program. They run as
integration tests, so they exercise production code without the original
library's `cfg(test)` CP/M console handling.

The `advance` hook receives positive instruction clocks before interrupt
sampling and interrupt-entry clocks afterwards. RETI notification occurs during
instruction execution, before its aggregate clocks are delivered. Peripheral
side effects are therefore not positioned at individual bus T-states. Restore
does not touch the I/O object; its state and scheduler debt belong to the caller.

This remains an instruction-stepped core, not a T-state-accurate bus. No general
opcode/timing accuracy audit is implied. Snapshot validation is structural, not
a guarantee of compatibility with other emulator versions or games.
IM0 timing, undocumented RETI aliases and the inherited asserted-NMI API have
not been revised or certified by these tests; the canonical IM1/IM2 and pulsed
NMI paths above are the covered boundary. The original register-pair unions
also retain their little-endian-host assumption; no big-endian port is claimed.
