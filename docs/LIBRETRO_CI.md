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

## Verified first publication (2026-10-02)

- Release: [0.1.0.0 Model 1 Preview](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.0), marked prerelease.
- Fixed source tag: `v0.1.0.0`, commit `f1ddd0e958908aa54470f0a6492b3c4eb8034928`.
- Successful five-platform build/publication: [CI 36976129395](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/36976129395).
- Validation-tool/workflow recovery commit: `aad108b`, recognizing MSVC's
  folded-function aliases without changing tagged machine/adapter source.
- Local offline build, 25 ABI exports/empty lifecycle, Development installation
  and complete 953-value generator/campaign audit passed. All native CI
  machine/adapter test jobs passed; the ROM-dependent repair test stays ignored.
- Five published archives were downloaded, rechecked against SHA256SUMS and
  GitHub asset digests, unpacked and passed the exact inventory/internal
  checksums/version/source-revision gate. Linux/macOS packages have 196 files;
  Windows has 206 due to its target-specific dependency license closure.
- The unused `v0.1.0` tag was withdrawn before any release publication. The
  public four-component version is verified in every 0.1.0.0 package.

| Platform ZIP | SHA-256 |
| --- | --- |
| Linux ARM64 | `9c7dca4cbfcafa484155d261db149118330a985408f0c28f29f44f0fbb748763` |
| Linux x86_64 | `ee926e325905b2ad0a1442ca1036c7999aee32339632253993670ee1a1ec8bd0` |
| macOS Apple Silicon | `5f9a223f1cbe5203296454b5eb7eb6abbf4225d22f8ec3bc5d9944f46bfe2868` |
| macOS Intel | `10c92095fbe929222d794c6608b10030b142f98bd349f081fe71e28fa2088749` |
| Windows x86_64 | `b28c05fc8d21f41206ed6367156cd45528c79bddb52602a90969e82fd6c066d7` |

Local Development core SHA-256:
`c99e0d2e496776a11b783358b3403ee182cb051360e91f611deafc0874ee1a1b`.
Matching installed `.info` SHA-256:
`35e97e9499bbad25c2edb7fb3a15bea46691f015e8ba9417089349fd48cf423b`.
These gates establish publication/build/package evidence; user-run gameplay,
physical controller and distributed race/dogfight acceptance remain separate.
