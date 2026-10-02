# Libretro builds and releases

## Reference adaptation

Inspected SM2-Emu `.github/workflows/libretro-ci.yml`, `CI.md`,
`scripts/check-libretro-artifact.py` and `check-libretro-package.py`, and
Supermodel's workflow, validation script and published `0.3.0-beta` packages.
Retain native platform builds, ROM-free ABI/lifecycle checks, core/info pairing,
licenses, source revisions, ZIPs and checksums. Model 1 embeds its catalogue and
NVRAM tables, so no SM2/Supermodel `system` assets are copied.

## Build matrix

The workflow uses the hosted runners' Rust, Python and C/C++ toolchains.
Cargo.lock fixes crate versions; BUILD_INFO.json records the effective compiler
and Cargo versions. It does not install tools on the developer's machine.

| Platform | Runner | Rust target | Packaged library |
| --- | --- | --- | --- |
| Linux x86_64 | ubuntu-24.04 | x86_64-unknown-linux-gnu | tgpulse_next_m1_libretro.so |
| Linux ARM64 | ubuntu-24.04-arm | aarch64-unknown-linux-gnu | tgpulse_next_m1_libretro.so |
| macOS Apple Silicon | macos-15 | aarch64-apple-darwin | tgpulse_next_m1_libretro.dylib |
| macOS Intel | macos-15-intel | x86_64-apple-darwin | tgpulse_next_m1_libretro.dylib |
| Windows x86_64 | windows-2022 | x86_64-pc-windows-msvc | tgpulse_next_m1_libretro.dll |

Build only `tgpulse-libretro`, preserving standalone defaults. Thin LTO and one
codegen unit already apply. Linux statically links the C++ runtime; system
glibc/libgcc dependencies remain. Windows reuses the existing static-CRT MSVC
configuration rather than installing a separate MinGW toolchain. macOS targets
13.0, replaces the build-local dylib ID with `@rpath`, and signs/verifies the
finished library before checking and packaging it.

CI runs machine/adapter tests and `tools/check_libretro_artifact.py` on every
native runner. Linux also audits the source-only NVRAM inventory. Full sample
audits and `generate_model1_nvram.py --check` require the locally retained raw
campaign and remain separate from ROM-free CI. Existing frontend evidence is
in the renderer and linked-cabinet documents; CI is not gameplay proof.

## Local preflight and packaging

Use the existing cached toolchain/dependencies:

```sh
cargo build --offline --locked --release -p tgpulse-libretro
python3 tools/check_libretro_artifact.py target/release/libtgpulse_next_m1_libretro.dylib --target aarch64-apple-darwin
python3 tools/install_dev_core.py
python3 tools/generate_model1_nvram.py --check
ruby tools/audit_model1_campaign_inventory.rb
```

To exercise the package procedure before publication, use a fresh directory
outside Git:

```sh
python3 tools/package_libretro.py \
  --core target/release/libtgpulse_next_m1_libretro.dylib \
  --target aarch64-apple-darwin --offline \
  --output /private/tmp/model1-package \
  --archive /private/tmp/tgpulse-next-m1-libretro-macos-arm64-0.1.0.0.zip
python3 tools/package_libretro.py --check --output /private/tmp/model1-package
```

The package checker requires the exact declared file inventory, matching
metadata/version, nonempty core, dependency license inventory and complete
SHA256SUMS. The packager normalizes the Rust library's leading `lib` to pair
the published core filename with its `.info`. Archives use the source commit
timestamp and ordered entries. Compiler/host differences may still change
binary bytes; this is not a claim of reproducibility across different hosts.

## Publish a preview

Publication requires explicit user authorization. Prepare English release
notes under `docs/releases/VERSION.md`, keep the runtime/info version
consistent, commit the source and workflow, and create an annotated `vVERSION`
tag at that verified source commit. Push to this repository's `origin`.

The public core version follows the agreed four-component convention:
`UPSTREAM_MAJOR.MINOR.PATCH.PORT_REVISION`, initially **0.1.0.0**. Runtime,
`.info`, tag, release notes and ZIP names use that version. Cargo requires
three-component SemVer; its upstream/workspace version remains `0.1.0` and
is not substituted for the public Libretro version.

A tag push builds the matrix and publishes a prerelease only after every gate
passes. A workflow dispatch with `release_tag=vVERSION` checks out that exact
existing tag and follows the same gates. An empty dispatch builds without
publishing. For the first workflow commit, `[skip ci]` avoids a duplicate
branch build; dispatch the tagged build explicitly.

The release job checks all five ZIPs, internal checksums, metadata version and
SOURCE_COMMIT against the tag checkout, writes archive SHA256SUMS, then uploads
the packages with the English notes. Download the published assets and verify
their archive/internal checksums and source revisions before reporting success.

If a published tag's workflow fails, keep the tag fixed. Correct the workflow
on `main` and dispatch `release_tag` again, following the existing TGPulse-Next
recovery convention. Do not replace the tag or silently publish another source
commit under its name. The workflow checks out its current ABI validation tool
separately; that tool reads the tagged core's `.info`. This permits a checker
fix without modifying the compiled/tagged source. CI artifacts are retained
for 14 days.
