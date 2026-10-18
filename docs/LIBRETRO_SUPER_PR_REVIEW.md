# Libretro-Super Pull Request Review: TGPulse-Next Model 1

## Submission Target And Scope

The [submitted PR #2132](https://github.com/libretro/libretro-super/pull/2132)
targets `libretro/libretro-super`, base branch `master`. It adds one metadata file:
`dist/info/tgpulse_next_m1_libretro.info`. The prepared patch is
[`docs/pr/libretro-super-tgpulse-next-m1.patch`](pr/libretro-super-tgpulse-next-m1.patch).
The published PR head is `1c1828a893d08d893e42c009c7ca7f30e9997443`
against base `9c08e5e6f33a60dc97ff2d232fe6b938557bebb4`.

[Libretro's core onboarding guide](https://docs.libretro.com/development/cores/developing-cores/#add-your-core-to-libretro-infrastructure)
separates the core `.info` registration in `libretro-super` from the source
repository's `.gitlab-ci.yml`. This source repository has one
[`Makefile.libretro`](../Makefile.libretro) release build path and
[official-template CI definitions](../.gitlab-ci.yml) for Linux x86_64/ARM64,
macOS Intel/Apple Silicon and Windows x86_64. Its published
[v0.1.0.10 release](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.10)
passed the five-platform GitHub matrix and package checks. The Libretro-owned
pipeline has not run, so its platform output remains an onboarding gate.

A previous local proposal also added one Linux x86_64 recipe to the older
`libretro-super` recipe lists. That would have registered only one of the five
verified targets. The current proposal uses the source-side Libretro CI
configuration for platform builds and keeps the upstream patch limited to
metadata. The existing `libretro-super` recipe lists do not expose matching
entries for all five targets.

The [local review checkout](/private/tmp/tgpulse-libretro-super-pr) contains
the submitted metadata change. Its single-file diff was compared with current
upstream `master` before publication. The PR registers identity; distribution through
the Libretro buildbot depends on maintainers connecting and running the source
CI. No ROM, save, personal settings or build product belongs in this patch.

## Submitted PR Title

Register experimental TGPulse-Next Model 1 core for Libretro CI

## Submitted PR Body

### Summary

This PR registers **TGPulse-Next-M1**, an experimental, single-system
Sega Model 1 Libretro core. Its independent source repository is
[`Zer0one/tgpulse-next-libretro`](https://github.com/Zer0one/tgpulse-next-libretro).
The proposed buildbot core is `tgpulse_next_m1_libretro`, displayed as
**Sega - Model 1 (TGPulse-Next)**.

### Core Registration And Build Requirements

- Add `dist/info/tgpulse_next_m1_libretro.info` with public metadata, version
  `0.1.0.10`, ZIP content support and experimental status.
- The source repository provides one `Makefile.libretro` release build path and
  Libretro CI definitions for Linux x86_64/ARM64, macOS Intel/Apple Silicon and
  Windows x86_64. These builds require compatible Rust/Cargo and C++ toolchains.
  Libretro-hosted execution remains to be verified during onboarding.

### Core Highlights

- Extend Model 1 emulation with experimental Sega NetMerc support, including
  music through optional substitute sample banks. Incomplete original ROM
  dumps cannot provide complete original audio. TeknoParrot's
  [publicly reported NetMerc work](https://www.patreon.com/TeknoParrotTeam/posts/teknoparrot-does-168035857)
  inspired this work; no TeknoParrot code or game assets are included.
- Support motion-sensor input and calibration for Sega NetMerc MVD controls.
- Expose reviewed, game-specific NVRAM operator settings directly in
  RetroArch's Core Options, with optional automatic initial setup.
- Simplify linked-cabinet setup with Automatic Network Settings presets for
  supported games, alongside the Linked Cabinets option. The presets manage
  the relevant operator values when NVRAM Settings is enabled.
- Fill Model 1 emulation gaps and improve performance relative to the
  original TGPulse, while retaining its machine implementation and buildable
  standalone application.

### Upstream Integration And Local Adaptations

- This independent repository began with a documented TGPulse-Next source
  snapshot. TGPulse-Next development preceded this Libretro port; the port
  selectively integrated later NetMerc machine, I/O, tracking, audio and
  state changes without merging repository histories.
- The Libretro adapter adds frontend-owned rendering, input, audio, Core
  Options, persistence and Netpacket transport around the reused machine.
  The Model 1-only build is a port integration, separate from TGPulse-Next's
  standalone host code.
- Model 1 machine improvements developed in this port are recorded as
  [upstream backport candidates](https://github.com/Zer0one/tgpulse-next-libretro/blob/main/docs/UPSTREAM_BACKPORT_CANDIDATES.md).
  Their presence in this core does not claim adoption by TGPulse-Next.

### Libretro Integration

- Load complete Model 1 ZIP sets through a narrow adapter that owns Libretro
  callbacks, Core Options v2, frontend input and output, and clean shutdown.
- Expose frontend-managed Save RAM and serialized Save States. The build
  selects Model 1 machine components and their shared devices; Model 2 is
  outside this core.

### Input And Controls

- Provide game-specific RetroPad controls and use Libretro input and rumble
  interfaces where supported.

### NVRAM And Persistence

- Provide Automatic Initial NVRAM Setup for new or invalid Save RAM and
  optional, game-specific NVRAM Settings for reviewed operator fields.
- Keep existing valid Save RAM authoritative until NVRAM Settings is enabled;
  preserve unrelated fields when applying selected overrides.

### Rendering, Timing And Audio

- Provide software output and frontend-owned Vulkan and OpenGL/GLES render
  paths, with widescreen and supersampling options where applicable.
- Default to native Model 1 timing; offer an optional 60 Hz mode. Expose
  Master Volume and per-source Gains for sources in the loaded game.

### Linked Cabinets

- Use Libretro Netpacket transport for supported Model 1 cabinet links while
  retaining the game's native COMM protocol and operator roles.
- Keep Save States unavailable while a linked COMM board is fitted.

### Core Assets

The game catalogue and NVRAM tables are embedded; no separate system
database is required. Users provide game ROMs. Sega NetMerc requires a
user-supplied I/O BIOS; its diagnostic display can use an optional HD44780
BIOS. Neither is distributed by this recipe.

### Tests And Validation

The published [v0.1.0.10 release](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.10)
passed its [five-platform CI run](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/37519007905)
using the shared Makefile:

- Native builds, machine and adapter tests, and Model 1-only scope checks passed
  on Linux x86_64/ARM64, macOS Intel/Apple Silicon and Windows x86_64.
- Each core passed Libretro ABI, dependency and empty-lifecycle checks. All
  published packages passed metadata, license, source-revision and checksum
  verification after download.
- Focused Libretro ABI checks covered the Linked Cabinets option-change Resume
  sequence. Separate two-instance RetroArch runs established game-created
  COMM online on both peers; they started with options already selected.

Detailed evidence and the limits of runtime/controller trials are recorded in
the source repository.

### Limitations And Reviewer Attention

- Sega NetMerc remains experimental. Save States are unavailable while a
  linked COMM board is fitted; hardware rendering requires a compatible
  frontend and driver.
- The Libretro-owned GitLab build has not yet run. The rendering context
  lifecycle, Save State validation, NVRAM overrides and Netpacket transport
  merit independent review.

### Licensing And Attribution

- The project root is MIT licensed; retained third-party source notices and
  packaged dependency licenses remain available in the source repository.
- Thanks to the author of [TGPulse](https://github.com/deepblueworks/TGPulse),
  whose original emulator is the foundation of
  [TGPulse-Next](https://github.com/Zer0one/TGPulse-Next) and this core.
- Thanks to the [TeknoParrot team](https://www.patreon.com/TeknoParrotTeam/posts/teknoparrot-does-168035857)
  for publishing NetMerc research that inspired our independent work on that
  game.

### AI-Assisted Development

This core began as a study of how AI assistance can support the development
of application software. OpenAI Codex assisted substantially with code,
analysis, debugging, documentation and automated checks. The human maintainer
set requirements, reviewed design decisions and performed real-game and
controller trials. It is experimental and may contain bugs or inaccurate
emulation; the published checks do not replace independent review and testing.

## Review And Publication Gates

1. Confirm the single-file `libretro-super` diff, its public identity and
   version, and the source repository's `Makefile.libretro` and `.gitlab-ci.yml`.
2. Recheck the exact source release, five published package manifests and
   matching `.info` metadata. Request Libretro-owned CI execution for all five
   declared targets; local and GitHub runs do not establish that gate.
3. PR #2132 was published from `Zer0one/libretro-super` after user
   authorization. Its sole changed file is the matching `.info`; GitHub
   reported no PR checks immediately after creation.

**Current state:** The registration PR is open. Libretro-owned CI onboarding
and independent reviewer acceptance remain pending.
