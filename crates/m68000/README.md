# Local m68000 0.2.3

Source: the crates.io `m68000` 0.2.3 package already used by TGPulse.
Upstream: https://github.com/Stovent/m68000, package VCS revision
`cd4e0b1e02e811660b6ce1876d040f7f79682014`, directory `m68000`.
Original crates.io archive SHA-256 (from the prior Cargo.lock):
`ae608876c3e5b088c53d7b310dda17bae1b5aa33cb61a5824e7bc730a3f9c7fe`.
Author: Stovent. License: MPL-2.0, as declared upstream and retained in source
headers (https://mozilla.org/MPL/2.0/). See UPSTREAM_README.md for upstream docs.

Local delta: add `state.rs` and its module declaration, with explicit
instruction-boundary snapshot/restore including pending exceptions and current
opcode. The manifest uses the workspace's existing serde/bincode, disables
upstream CLI binaries, and retains upstream tests. No opcode, timing, exception
priority or memory-access behavior is changed. In particular the upstream
priority-set behavior is preserved, not silently repaired during serialization.

The upstream `operators.rs` tests depend on the obsolete nightly
`bigint_helper_methods` feature, so they are retained as reference but are not a
stable-workspace test target. Memory, status-register and assembler tests are
enabled along with the new state tests. No nightly toolchain is installed.

States exclude memory and the generic CPU implementation. The containing
machine must check its CPU model, ROM resources, format version and input size.
This API does not itself imply complete sound-board or machine save states.

This replaces the registry dependency rather than creating a second active CPU.
Keep the delta narrow for upstream reapplication; do not edit the Cargo cache.
