# Libretro builds and releases

## Reference adaptation

Inspected SM2-Emu `.github/workflows/libretro-ci.yml`, `CI.md`,
`scripts/check-libretro-artifact.py` and `check-libretro-package.py`, and
Supermodel's workflow, validation script and published `0.3.0-beta` packages.
Retain native platform builds, ROM-free ABI/lifecycle checks, core/info pairing,
licenses, source revisions, ZIPs and checksums. Model 1 embeds its catalogue and
NVRAM tables, so no SM2/Supermodel `system` assets are copied.

## Build matrix

The GitHub release workflow uses the hosted runners' Rust, Python and C/C++ toolchains.
Cargo.lock fixes crate versions; BUILD_INFO.json records the effective compiler
and Cargo versions. It does not install tools on the developer's machine.

| Platform | Runner | Rust target | Packaged library |
| --- | --- | --- | --- |
| Linux x86_64 | ubuntu-24.04 | x86_64-unknown-linux-gnu | tgpulse_next_m1_libretro.so |
| Linux ARM64 | ubuntu-24.04-arm | aarch64-unknown-linux-gnu | tgpulse_next_m1_libretro.so |
| macOS Apple Silicon | macos-15 | aarch64-apple-darwin | tgpulse_next_m1_libretro.dylib |
| macOS Intel | macos-15-intel | x86_64-apple-darwin | tgpulse_next_m1_libretro.dylib |
| Windows x86_64 | windows-2022 | x86_64-pc-windows-gnu | tgpulse_next_m1_libretro.dll |

Build only `tgpulse-libretro`, selecting its Model 1 machine feature and
preserving both standalone defaults. Check library configurations in separate
Cargo invocations to avoid feature unification. Require the compiled
`model1` scope marker independently of metadata; package dependency/license
inventory follows the selected core graph. Scope design and local evidence
are in [LIBRETRO_BUILD_SCOPE.md](LIBRETRO_BUILD_SCOPE.md). Thin LTO and one
codegen unit already apply. Linux statically links the C++ runtime; system
glibc/libgcc dependencies remain. Windows uses GNU/MinGW and the existing
static C++ runtime configuration; its ephemeral GitHub runner provisions
MSYS2 Make/GCC and the GNU Rust target, following Supermodel's Windows setup.
No developer-host dependency installation is needed for this change. The
Makefile sets macOS 13.0 and the public `@rpath` dylib ID at link time, then
signs/verifies the output when building on a native macOS host.

CI runs machine/adapter tests and `tools/check_libretro_artifact.py` on every
native runner. Linux also audits the source-only NVRAM inventory. Full sample
audits and `generate_model1_nvram.py --check` require the locally retained raw
campaign and remain separate from ROM-free CI. Existing frontend evidence is
in the renderer and linked-cabinet documents; CI is not gameplay proof.

## Shared Libretro Build Recipe

The root [`Makefile.libretro`](../Makefile.libretro) is the sole release build
recipe for local builds, GitHub release CI and Libretro GitLab jobs. It builds only the
`tgpulse-libretro` package and copies its library to the public
`tgpulse_next_m1_libretro` filename. It accepts explicit Linux x86_64/ARM64,
macOS Intel/Apple Silicon and Windows x86_64 target triples. Linux links the
target's static C++ runtime and still uses system glibc/libgcc. The proposed
Linux x86_64 `libretro-super` recipe invokes `platform=unix`; the
source-side [`.gitlab-ci.yml`](../.gitlab-ci.yml) extends official Libretro
Rust templates for all five desktop targets. Both CI definitions call the
same Makefile; neither contains an independent Cargo release command or
platform linkage policy. Windows uses GNU/MinGW in both. GitHub additionally
runs native tests, artifact checks and packaging; GitLab supplies Libretro's
toolchain image and artifact collection.

The current five-platform GitHub workflow and Libretro GitLab jobs both use
this recipe; earlier releases used direct Cargo commands and Windows MSVC.
Their recorded results below describe those historical builds, not validation
of the revised GNU Windows job. The new GitHub matrix has not run remotely.

The template definitions were inspected at
`libretro-infrastructure/ci-templates` commit
`96f603ee450eff3e9ad2baeac75b300aa83c1c9e`, and the official Rust image
definition at `libretro-build-rust` commit
`674682c815b69127805c44ec9ddee6d5bb69a509`. The Makefile produced a
Model 1-scoped Linux x86_64 ELF in an isolated Rust builder and a macOS arm64
dylib locally; both passed the artifact checker. The macOS result was installed
as the Development core with a matching SHA-256. These checks validate the
source-side build contract on those two hosts. The Libretro-owned GitLab jobs,
its cross builds and distribution remain unverified until the source is
published and their pipeline runs. [Submission details and PR text](LIBRETRO_SUPER_PR_REVIEW.md)
are prepared for review; no Libretro PR has been opened.

The single-recipe follow-up on 2026-10-05 verified Linux x86_64 and macOS
arm64 production outputs, all 25 ABI exports, empty lifecycle cycles and
Model 1 scope. The macOS public `@rpath` ID, signature and 194-file package
preflight passed. Its Development installation matches SHA-256
`9cb8f8964dc365f836e1706b3b50f9d33b3bdeb9fedd721652d5b2e43bad98ad`.
Linux SHA-256 remains
`9d369ead94b5e49ce802471063073f915969ced08eb7549481b8de3e0cc39371`.
Both CI YAML files parse, and the GNU Windows PE export parser recognizes
all 25 exports in an existing published DLL. That parser check is not a
Windows GNU build or native execution result. Required reasoning: Medium;
account usage 48%; reset 2026-10-10 09:49:17 CEST.

## Local preflight and packaging

Use the existing cached toolchain/dependencies:

```sh
CARGO_NET_OFFLINE=true make -f Makefile.libretro
python3 tools/check_libretro_artifact.py tgpulse_next_m1_libretro.dylib --target aarch64-apple-darwin --machine-scope model1
python3 tools/install_dev_core.py
python3 tools/generate_model1_nvram.py --check
ruby tools/audit_model1_campaign_inventory.rb
```

To exercise the package procedure before publication, use a fresh directory
outside Git:

```sh
python3 tools/package_libretro.py \
  --core tgpulse_next_m1_libretro.dylib \
  --target aarch64-apple-darwin --offline \
  --output /private/tmp/model1-package \
  --archive /private/tmp/tgpulse-next-m1-libretro-macos-arm64-0.1.0.3.zip
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

For a tag containing `Makefile.libretro`, if its workflow fails, keep the tag
fixed. Correct the workflow
on `main` and dispatch `release_tag` again, following the existing TGPulse-Next
recovery convention. Do not replace the tag or silently publish another source
commit under its name. Older tags without the shared recipe require their
historical workflow; the current workflow does not inject a new build recipe
into an old source tag. The workflow checks out its current ABI validation tool
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


## Verified 0.1.0.1 publication (2026-10-02)

- Release: [0.1.0.1 Model 1 Preview](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.1), marked prerelease.
- Fixed annotated tag: `v0.1.0.1`; source commit `a2eba83f812f5710771b5ca641d37e647cc1cc04`.
- All five native build/check/package jobs and the publication job passed:
  [CI 36978325511](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/36978325511).
- Restores global source Gains, default Auto, with common load/live parsing and
  four modern/legacy source keys. English release notes include saved-option
  cleanup guidance; the local user-authorized cleanup is documented separately.
- Local verification: 47 adapter tests, offline locked release build, native
  ABI/dependency/empty-lifecycle gate, NVRAM generator and full campaign audit,
  and the 196-file macOS package preflight passed.
- Downloaded all five published archives plus SHA256SUMS. Verified GitHub
  asset digests, archive/internal checksums, exact package inventories,
  licenses, version 0.1.0.1 and the tagged source revision. Linux/macOS
  packages contain 196 files; Windows contains 206.

| Platform ZIP | SHA-256 |
| --- | --- |
| Linux ARM64 | `f1522c4831d8e239b537177db3bd52fd231277ddadeccb1aecd8553beae5d026` |
| Linux x86_64 | `c4a69b4df8881ec519b4f2c0626d4e94e56bd071e8537600ba9dcc8a59588735` |
| macOS Apple Silicon | `4e2a880475732fae6bb3f64403c4c4e772e77c88848b8c940029a5d0e214660c` |
| macOS Intel | `ab413ac3b901806c872b833758b9cd7fa4f91eedf0213faa72c1252b53c995c3` |
| Windows x86_64 | `e8633d3dda0411e27ff0a5aa87401af31fc54cb5360d6e170f72c37296224320` |

Development core 0.1.0.1 and metadata were installed and hash-checked locally:
core `76017083ee2f164d4e4e4f1837f5ff8cdca8b4609d1a5aabdbc85ef07537fbbe`,
info `68d8ea89d185ecac4a00f867f47f9cb08387f2699747100d82dc4cdc91174209`.
Publication and package verification are separate from listening, gameplay
and controller acceptance. Required reasoning: Medium; account usage 49%;
reset 2026-10-07 15:58:51 CEST.

## Verified 0.1.0.2 publication (2026-10-02)

- Release: [0.1.0.2 Model 1 Preview](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.2), marked prerelease.
- Fixed annotated tag: `v0.1.0.2`; source commit `6946b876803782d9549c61a7b11e0ae45d9efec3`.
- All five native build/check/package jobs and the publication job passed:
  [CI 37007851965](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/37007851965).
- The Model 1 single system build excludes Model 2 machine/device dependencies.
  Each native job independently verifies M1-only, M2-only and combined library
  configurations, adapter tests and the compiled scope marker. Standalone
  defaults retain both systems.
- Reference: SM2 `PORTING_PLAN.md` phase 4 native matrix, ABI, core/info,
  license, source revision and checksum distribution gates, rechecked for this
  publication. Adaptation: Rust machine-feature checks and scoped dependency
  license collection; Model 2 and combined Libretro adapters remain future work.
- Local release verification: 47 adapter tests, offline locked release build,
  native ABI/dependency/empty-lifecycle gate and 194-file macOS package preflight.
  Earlier functional and state-compatibility evidence is in `LIBRETRO_BUILD_SCOPE.md`.
- Downloaded all five published archives plus SHA256SUMS. Verified GitHub
  asset digests, archive/internal checksums, exact inventories, Model 1 scope,
  licenses, version 0.1.0.2 and the tagged source revision. Linux/macOS packages
  contain 194 files; Windows contains 202.

| Platform ZIP | SHA-256 |
| --- | --- |
| Linux ARM64 | `94e2942b0580ddc4a4cdcfb113fef45d12f08be312207422ffde2471e3a8e433` |
| Linux x86_64 | `cb075c3301f29e151ee98a081dd935264a79a3e90e42cab90357c72535d78abf` |
| macOS Apple Silicon | `613a2c5d1a1e9e3b6b12e343722639dfb90eb21c6a96613eaf8affa0e41c6948` |
| macOS Intel | `c63436ef99d5005981e287adf2541e0e4f2bb21a78c936c9f0632be1740723df` |
| Windows x86_64 | `b8629643018efdaca1056f649ea1af1185c09c3d9ffbe171077fe679bb0198a8` |

Development core 0.1.0.2 and matching metadata were installed and hash-checked:
core `ead4a5dc151d96bb0cc750bb5a0cce99364e5452445e4a2d7b4d841aca82b2c7`,
info `91ff93c2f5f1bd468bf88d66b73e3fe87066bbb596bbe37b41bc135f0021fdee`.
Publication/package checks are separate from listening, gameplay and physical
controller acceptance. Required reasoning: High; account usage 56% before and
after publication (rounded shared-account reading). Reset:
2026-10-07 15:58:51 CEST.

## Verified 0.1.0.3 publication (2026-10-04)

- Release: [0.1.0.3 Model 1 Preview](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.3), marked prerelease.
- Fixed annotated tag: `v0.1.0.3`; source commit
  `90a65102fc76f170f7e0c130f734d8259a2f0937`.
- All five native build/check/package jobs and the publication job passed:
  [CI 37234302083](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/37234302083).
- This release includes NetMerc U1–U8 integration, the Wing War throttle
  correction and the user-requested Gain visibility rule. The adapter compares
  registered Gains with the loaded machine's `SoundSystem::sources()`; global
  Gain values remain stored across games.
- Local Model 1 and adapter tests, NVRAM campaign checks, macOS release build,
  native ABI gate and Development core/info installation were completed before
  source publication. At the user's request, host/gameplay tests were not
  repeated for this release.
- Downloaded all five published ZIPs and `SHA256SUMS`. Each archive matches the
  published manifest and GitHub asset digest. Embedded `SOURCE_COMMIT.txt`
  and `BUILD_INFO.json` identify the fixed tag commit, version 0.1.0.3 and
  Model 1-only scope. The release workflow also checked each package's internal
  manifest, inventory and licenses before uploading it.

| Platform ZIP | SHA-256 |
| --- | --- |
| Linux ARM64 | `d42d4d2771d427b2ae70e9453edd41facbe3aa0f1be67912ad7919765f383873` |
| Linux x86_64 | `9b4005dd83219fbac7c08d2baf915e30be48caf36b24bf61e943cab5b28a800b` |
| macOS Apple Silicon | `b89844f94945753b9bc41d95f2bba07c2a823beffff72bf9817475b548e418bf` |
| macOS Intel | `c74cd583d527c1bbe7693daa1cebe30b7b4ff8edd4be0fef8d3380fa9b216a7a` |
| Windows x86_64 | `011ac9c6f0f1a2160a924f0153cf5dd0de68138d8773335a0d8c4f5ce518621e` |

The local Development core SHA-256 is
`7e238aafb5d3a17ce72fa09bf495e9c425b0d263d44ddd8713e0bafcce6f62b9`;
matching `.info` SHA-256 is
`956e7f1b8098ecf36ae21b865e2363461d0e0083b661698cc7458e03b014a659`.
Package verification does not establish physical controller feel or additional
gameplay acceptance.
