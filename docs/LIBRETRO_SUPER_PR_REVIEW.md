# Libretro-Super Pull Request Review: TGPulse-Next Model 1

## Submission Target

The proposed PR targets [`libretro/libretro-super`](https://github.com/libretro/libretro-super),
base branch `master`. The TGPulse proposal is prepared against libretro-super commit
`3c142c68ed6fa509c8b4cdc5ca47877efbf75951` on local review branch
`codex/add-tgpulse-next-m1` at `/private/tmp/tgpulse-libretro-super-pr`.
The exact two-file diff is saved as
[`pr/libretro-super-tgpulse-next-m1.patch`](pr/libretro-super-tgpulse-next-m1.patch).
The upstream submission is prepared locally for review; no Libretro pull
request has been opened. The 0.1.0.6 source release is published independently of this upstream
submission.

| Proposed upstream file | Adaptation |
| --- | --- |
| `dist/info/tgpulse_next_m1_libretro.info` | Use the public Core Name `TGPulse-Next-M1`, Core Label `Sega - Model 1 (TGPulse-Next)` and version `0.1.0.9`. |
| `recipes/linux/cores-linux-x64-generic` | Add one `tgpulse_next_m1` entry pointing to `Zer0one/tgpulse-next-libretro` `main`; use `GENERIC Makefile.libretro .`. |

The new source-side [`Makefile.libretro`](../Makefile.libretro) is required by
the existing buildbot's `GENERIC` contract: it invokes Make with
`platform=unix`, then copies `tgpulse_next_m1_libretro.so` from the source
root. The wrapper builds only the `tgpulse-libretro` package, which selects
the `model1` feature without Model 2 machine dependencies. It accepts the five
existing desktop target triples and produces the public core filename at the
source root. The Linux x86_64 recipe requires compatible Rust/Cargo, GCC and
static `libstdc++.a`. The source-side [`.gitlab-ci.yml`](../.gitlab-ci.yml)
extends Libretro's official Rust templates for Linux x86_64/ARM64, Windows
x86_64 GNU and macOS Intel/Apple Silicon. Those jobs have not run on
Libretro-owned runners; the existing GitHub workflow remains the published
five-platform release baseline. Both CI definitions now invoke the same
Makefile, with Windows GNU/MinGW in the common target policy; the updated
GitHub workflow passed all five native jobs for v0.1.0.6.

The source `.info` now has the public name and label. Following the user's 2026-10-05 naming update, local
`tools/install_dev_core.py` installs `tgpulse_next_m1_libretro` with the same
public metadata; its historical script filename is retained. Prior Development
files are retired into a backup directory. Published v0.1.0.6 packages use the corrected public `.info`;
v0.1.0.3 archives retain their historical metadata.
The earlier isolated local-installer check retained the then-agreed Development name and
label. A separate offline package preflight passed with the public name and
label, including version, licenses, source revision and checksums. The new
macOS arm64 Makefile build also passed artifact checks and was installed under
the Development name with matching SHA-256. Version 0.1.0.6 is now published;
its exact-source package evidence is recorded in `LIBRETRO_CI.md`.

The updated wrapper was built in an isolated Debian 12 Linux x86_64 container
with Rust/Cargo 1.97.1. The test used a ROM-free source snapshot based on local
HEAD `822c73f`, plus the uncommitted wrapper, metadata and FM Auto change. The
output was `tgpulse_next_m1_libretro.so` (SHA-256
`9d369ead94b5e49ce802471063073f915969ced08eb7549481b8de3e0cc39371`).
The existing artifact checker passed ELF64/x86-64 format, 25 Libretro ABI
exports, three empty lifecycle cycles and compiled `model1` scope. `ldd`
found only `libgcc_s`, `libm`, `libc` and the ELF loader; no dependency was
missing. A buildbot-style `clean` then `build` sequence passed. These are
build/ABI checks, not RetroArch gameplay tests. The macOS arm64 public-named
core built through the same Makefile and passed the artifact checker; the
locally installed Development copy has SHA-256
`9cb8f8964dc365f836e1706b3b50f9d33b3bdeb9fedd721652d5b2e43bad98ad`.
The current shared recipe sets the public macOS `@rpath` install name during
linking and signs/verifies the native output. Its 194-file macOS package
preflight passed. GitHub's production build now calls this same recipe and
validates/packages the root artifact; the prior direct Cargo build and
workflow-local linkage/normalization commands have been removed. The Windows
native job follows Supermodel's MSYS2 setup and uses the same GNU target as
Libretro's Rust template. The revised remote matrix passed for v0.1.0.6,
including native GNU Windows loading and lifecycle checks. All five published packages were downloaded and
verified against their manifests, GitHub digests and tagged source revision.
The GitHub macOS arm64 build is installed locally under the Development name.

Model 1 embeds its game catalogue and NVRAM tables, so this PR has no required
`system` asset for the core as a whole. Sega NetMerc requires its I/O BIOS;
its diagnostic display also uses an optional HD44780 BIOS. Neither is
distributed. No `libretro-system-files` contribution is needed for this
initial integration.

## Ecosystem Build Integration Boundary

The two-file proposal registers public metadata and one Linux x86_64
`libretro-super` recipe. It does not by itself establish distribution through
Libretro's multi-platform buildbot. The [official core-development guide](https://docs.libretro.com/development/cores/developing-cores/#add-your-core-to-libretro-infrastructure)
also calls for source-side `.gitlab-ci.yml` integration, and the
[macOS build guide](https://docs.libretro.com/development/retroarch/compilation/osx/#gitlab-ci-mimicry)
identifies GitLab CI as the buildbot path for cores. The five official Rust
template includes and their inherited job names were checked against
`libretro-infrastructure/ci-templates` commit
`96f603ee450eff3e9ad2baeac75b300aa83c1c9e`. The Rust image definition at
`libretro-build-rust` commit `674682c815b69127805c44ec9ddee6d5bb69a509`
contains the target toolchains; actual pipeline execution and packaged output
on Libretro hosts remain unverified.

The source-side `.gitlab-ci.yml` uses the official Rust templates, replacing
their default Cargo command and rename step with this package's explicit
Model 1 Makefile contract. Linux ARM64 adds the cross C++ compiler needed for
static `libstdc++`; Windows uses the GNU cross toolchain in the Rust image.
Android, iOS and webOS are intentionally not registered. Repository ownership
or a Libretro-managed mirror is a maintainer decision, not a prerequisite for
preparing or reviewing the source-side build contract.

## Proposed PR Title

Add experimental TGPulse-Next Model 1 core (info file + Linux x64 recipe)

## Proposed PR Body

### Summary

This PR registers **TGPulse-Next-M1**, an experimental, single-system
Sega Model 1 Libretro core. Its independent source repository is
[`Zer0one/tgpulse-next-libretro`](https://github.com/Zer0one/tgpulse-next-libretro).
The resulting buildbot core is `tgpulse_next_m1_libretro`, displayed as
**Sega - Model 1 (TGPulse-Next)**. The local installation uses the same public
identity.

### Changes And Build Requirements

- Add `dist/info/tgpulse_next_m1_libretro.info` with public metadata, version
  `0.1.0.9`, ZIP content support and experimental status.
- Add a Linux x86_64 recipe that fetches the public source repository's `main`
  branch and invokes its root `Makefile.libretro`.
- The Makefile performs a locked release build of the Model 1 adapter and
  publishes the public `tgpulse_next_m1_libretro` library. The source also
  provides Libretro Rust-template jobs for Linux x86_64/ARM64, macOS
  Intel/Apple Silicon and Windows x86_64. The build hosts need compatible
  Rust/Cargo and the target C++ runtime toolchain.

### Core Scope And Assets

The core loads complete Model 1 ZIP sets. It provides Libretro Core Options
v2, software output plus Vulkan/OpenGL rendering, game-specific controls,
Save RAM and Save States, and linked cabinets for supported sets. It compiles
Model 1 machine components and the shared devices they require. Model 2 is
outside this build.

The game catalogue and NVRAM tables are embedded; no separate system
database is required. Users provide game ROMs. Sega NetMerc requires a
user-supplied I/O BIOS; its diagnostic display can use an optional HD44780
BIOS. Neither is distributed by this recipe.

### Validation

The published [v0.1.0.6 release](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.6)
has a passing [five-platform CI run](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/37351380108)
using the shared Makefile. Published packages were checked for ABI, metadata,
licenses, source revisions and checksums. The Libretro GitLab jobs use the
current official Rust templates but have not yet run on Libretro-owned
infrastructure. Runtime and controller evidence is recorded in the source
repository.

### Limitations And Licensing

- Sega NetMerc remains experimental. Save States are unavailable while a
  linked COMM board is fitted; hardware rendering requires a compatible
  frontend and driver.
- The project root is MIT licensed; retained third-party source notices and
  packaged dependency licenses remain available in the source repository.

### AI-Assisted Development

OpenAI Codex assisted substantially with code, analysis, debugging,
documentation and automated checks. The human maintainer set requirements,
reviewed design decisions and performed real-game and controller trials.

## Review And Publication Gates

1. Confirm the two-file upstream diff, public metadata, local Development
   installer, source-side Makefile and `.gitlab-ci.yml`. The FM Auto Gain
   change belongs to source release 0.1.0.5 and is outside the two-file
   libretro-super registration patch.
2. The exact v0.1.0.6 source commit passed the revised five-platform native
   GitHub matrix and downloaded-package verification. Libretro-owned GitLab
   execution remains a separate onboarding gate.
3. After user review and explicit authorization, create the upstream PR from
   a suitable fork and verify its diff against current `libretro-super/master`.
   The source wrapper and public metadata are already published on `main`.

**Current state:** The upstream patch and reviewer text are prepared locally.
Version 0.1.0.6 and its revised five-platform packages are published and
verified; upstream PR publication still awaits user review.

## 0.1.0.7 Identity And Publication Update

The proposed metadata and reviewer message now use `TGPulse-Next - Model 1`
and version `0.1.0.7`. Core Label and binary filename are unchanged.
[Release 0.1.0.7](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.7)
and all five native jobs passed; downloaded packages and the local GitHub
macOS installation were verified. Earlier publication sections above retain
their historical evidence. No Libretro PR has been opened.

## 0.1.0.8 Identity And Publication Update

Current proposed metadata and reviewer text use `TGPulse-Next-M1`, version
`0.1.0.8`. Core Label and binary filename are unchanged.
[Release 0.1.0.8](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.8)
passed all five native jobs; downloaded packages and the local GitHub macOS
installation are verified. Prior sections retain historical evidence.
No Libretro PR has been opened.
