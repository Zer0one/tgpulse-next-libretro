# Model 1 single system build

## Reference and adaptation

Reviewed SM2-Emu Libretro `PORTING_PLAN.md` phase 4, its native CI and packaging
workflow, and Supermodel `Docs/ROADMAP.md`. Both reference cores are already
system-specific; neither supplies a multi-system Cargo split to copy. Reuse
their ABI, core/info, license, package and Development-installation gates.
The smallest TGPulse adaptation is explicit machine features and build-time
catalogue selection. "Single system build" is the user's chosen replacement
for the earlier "Tiny" term. Implementation status lives only in
[the roadmap](LIBRETRO_ROADMAP.md).

## Machine boundaries

`tgpulse-core` declares `model1` and `model2`, with both enabled by default.
The standalone dependency retains those defaults. The M1 Libretro adapter
requests `model1` with default features disabled.

| Library selection | Main CPU / extra coprocessors | Catalogue |
| --- | --- | --- |
| `model1` | V60; shared MB86233, 68000 and Z80 | 10 Model 1 sets |
| `model2` | i960, MB86233, MB86235 and SHARC; shared 68000 and Z80 | 90 Model 2 sets |
| Default, both | Both machine families | All 100 sets |

Model 1 includes its machines, I/O/COMM/drive boards, native rendering, sound
and persistence. Model 2 machine/memory/geometry, SCSP and state modules are
excluded from the M1 build. The existing debugger dispatches both machines
and is retained in complete builds. Shared sound, tilemap, loader helpers and
configuration types remain reusable. Configuration/state layouts were not
reduced or reordered to shrink the artifact.

The feature boundary prepares the later Model 2-only and combined machine
libraries. It does not implement a Model 2 Libretro adapter or claim those
future cores are ready. The M1 artifact and installed Development filenames
retain their established `m1` suffix.

`build.rs` derives the selected records from the canonical upstream
`src/roms_db.dat` into Cargo's output directory. Chosen records remain byte
identical, including resource layout and parent/clone metadata. No second
maintained database or runtime-only catalogue filter is introduced. The M1
catalogue retains NetMerc; its existing boot problem still excludes it from
runtime comparison and NVRAM management.

## Build and verification procedure

Cargo unifies dependency features within one invocation. Build the dedicated
M1 core separately from the default full library/standalone. A workspace-wide
build can re-enable Model 2 through the standalone dependency; its M1-named
artifact must not be installed as a single-system core. The scope gate below
rejects that artifact using a compiled marker independently of `.info`.

```sh
cargo build --offline --locked --release -p tgpulse-libretro
python3 tools/check_libretro_artifact.py \
  target/release/libtgpulse_next_m1_libretro.dylib \
  --target aarch64-apple-darwin --machine-scope model1
python3 tools/install_dev_core.py
```

Test each library/adapter configuration separately, as reflected in CI:

```sh
cargo test --offline --locked -p tgpulse-core --no-default-features --features model1
cargo test --offline --locked -p tgpulse-libretro
cargo test --offline --locked -p tgpulse-core --no-default-features --features model2
cargo test --offline --locked -p tgpulse-core
cargo check --offline --locked -p tgpulse --lib --bin tgpulse
```

The canonical catalogue regression checks selected families and unchanged
record bytes. `cargo tree -p tgpulse-libretro` supplies the actual dependency
selection for package licensing; workspace-wide `cargo metadata` unifies
standalone features and therefore overstates this dependency set. Package
BUILD_INFO records `machine_scope: model1`; its checker validates the binary
marker and excludes Model 2 CPU dependencies from the license inventory.

`tools/test_libretro_model1_scope.py` reuses callback types from the existing
ABI acquisition host. With a retained pre-split core and user-owned complete
ROMs, it compares software frames, PCM, Save RAM and complete machine snapshots
for all nine runnable Model 1 sets. It also imports a pre-split state into the
new core and compares a five-frame continuation. It reads no personal settings
or saves; its JSON contains hashes and results, not ROM or state contents.

```sh
python3 tools/test_libretro_model1_scope.py \
  --baseline-core /absolute/path/pre-split-m1-core.dylib \
  --core target/release/libtgpulse_next_m1_libretro.dylib \
  --rom-dir /absolute/path/model1-roms \
  --output /private/tmp/new-model1-scope-comparison.json
```

## Local evidence — 2026-10-02

macOS Apple Silicon, existing compiler/dependencies, offline builds:

| Measurement | Previous / combined | Model 1 only |
| --- | --- | --- |
| Release dylib | 3,557,184 bytes | 3,422,352 bytes |
| Embedded catalogue | 145,807 bytes / 100 records | 18,877 bytes / 10 records |
| Scoped warm rebuild | 18.540 s | 13.097 s |

Binary size decreases by **134,832 bytes (3.79%)**. Thin LTO already eliminated
Model 2 CPU/machine symbols from the previous M1 adapter, so this is not a
claim of a large runtime speedup. The change removes unused catalogue data
and excludes Model 2 compilation/dependencies explicitly.

Build times are one observation per configuration: core and adapter rebuilt,
shared dependencies already cached, thin LTO and one codegen unit. They are
not clean-build timings or a performance benchmark.

Verified evidence:

- 214 Model 1 library tests, 94 Model 2-only tests, 227 complete-library tests,
  and 47 M1 adapter tests pass. The existing ROM-dependent repair test remains
  ignored where applicable.
- Standalone library/binary type-checks with both default machine families.
- The M1 dependency graph includes V60, MB86233, 68000 and Z80; it excludes
  i960, MB86235 and SHARC. The binary scope gate rejects a combined-feature
  reference build and accepts the new M1 artifact.
- All nine runnable sets produce identical software frame and audio hashes,
  Save RAM and machine state after 120 frames. Pre-split state restore and
  five-frame continuation also match for every set.
- Native artifact format, 25 ABI exports, system dependencies and three empty
  lifecycle cycles pass. The scoped macOS package passes exact inventory,
  metadata/version, source-base record, licenses and internal checksums.
- Existing isolated RetroArch Vulkan runner: 120 frames, image delivered,
  normal exit, 2.392 s, using the existing MoltenVK runtime.
- Development core installed and SHA-256 checked against the build:
  `ecc165692634d04f2c9ecb86ac2895c9f504ab4896e28d2bf59b46575440d48a`.
  Matching installed metadata:
  `536649062c567fae166f3091664a92c9a87a37a8711f7d4ed1d78f769ef698ed`.

The five-platform workflow is adapted but has not run for this local change.
The published 0.1.0.1 archives remain the previous interim build. Local package
preflight records the base Git commit; publication requires committing the
change and building the new immutable tag. Evidence above is distinct from
listening, gameplay and physical-controller acceptance.
Required reasoning: High. Account usage during this phase: 55% to 56%; this
is a rounded shared-account reading, not a per-task bill. Reset:
2026-10-07 15:58:51 CEST.
