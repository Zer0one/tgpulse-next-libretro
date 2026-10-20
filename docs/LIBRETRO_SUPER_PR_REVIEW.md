# Libretro-Super Update Pull Request Review

## Previous Registration

[PR #2132](https://github.com/libretro/libretro-super/pull/2132) was merged on
2026-10-08, merge commit `22d49cb96fda22f05955efea57d0782124c47027`.
It registered the Model 1 core at version 0.1.0.10. This update is a new PR,
not an edit of that merged request.

## Proposed Scope

Target: `libretro/libretro-super`, base `master`.
Inspected upstream revision: `f0308e8b876547006a9c4ebc820a57b039b4cf59`.
Exactly one metadata line changes: `display_version` 0.1.0.10 → 0.1.0.12.
All other upstream metadata matches the released core's `.info` byte for byte.

Prepared [patch](pr/libretro-super-tgpulse-next-m1.patch) and
[PR body](pr/libretro-super-tgpulse-next-m1-update-body.md).
The patch passes `git apply --check` against the downloaded upstream file.
Source implementation changes are already published in the core repository;
they are not additional files in this metadata PR.

## Proposed Title

Update TGPulse-Next Model 1 core metadata to 0.1.0.12

## Proposed Description

## Summary

Update `dist/info/tgpulse_next_m1_libretro.info` from **0.1.0.10** to **0.1.0.12**, following the initial registration merged in #2132.

This PR changes only the advertised version. The core implementation is already published in [TGPulse-Next Libretro 0.1.0.12](https://github.com/Zer0one/tgpulse-next-libretro/releases/tag/v0.1.0.12).

## Core Updates Since 0.1.0.10

- Separate **Linked Cabinets** from **Auto Network Role Settings**. Role automation operates independently of the NVRAM Settings switch, synchronizes managed selectors and preserves the frontend lobby through machine reconfiguration.
- Fix pending option updates so effective NVRAM, network-role and topology changes trigger the intended automatic reset.
- Default new configurations to **60 Hz**, retaining selectable native timing. This mode advances a new machine frame per callback and runs game/audio approximately 4.3% faster.
- Make **Widescreen Mode** Disabled by default and apply changes when returning to the game without reset or reload. Hardware expansion follows Auto/16:9; explicit 4:3 suppresses it. Native widescreen games and Software retain their existing behavior.
- Align controls/menu conventions and source Gain values; refine Sega NetMerc sensor Roll polarity and driving rumble behavior.

## Build And Validation

One shared `Makefile.libretro` remains the build entry point. The source repository provides Libretro CI definitions for Linux x86_64/ARM64, macOS Intel/Apple Silicon and Windows x86_64.

The [0.1.0.12 five-platform CI run](https://github.com/Zer0one/tgpulse-next-libretro/actions/runs/39162481874) passed native builds/tests and ABI, dependency, Model 1 scope and package checks. Downloaded release archives passed source-revision, license and checksum verification. Focused reset/audio, network lifecycle and macOS Vulkan Widescreen checks are documented in the source repository; they do not imply exhaustive gameplay or controller validation.

## Compatibility

Core Name **TGPulse-Next-M1**, display label, library basename, Save RAM layout and machine Save State format remain unchanged. The core remains experimental; Save States are unavailable while COMM is fitted. Review saved network-role selections when upgrading from 0.1.0.10.

## Published Update — 2026-10-19

[PR #2152](https://github.com/libretro/libretro-super/pull/2152) is open,
with the approved title and body. Head: `Zer0one:master`, commit
`da7ef39ff6dfe23136224af4e54db49897b44da9`; base: `libretro:master`.

The fork's existing default branch was 35 commits behind upstream with no
unique commits. It was fast-forwarded before adding the one-line metadata
update; no working branch was created and no unrelated fork changes enter
the PR. GitHub confirms exactly one modified file, one insertion and one
deletion. No PR checks were listed immediately after publication. The approved
body remains in `docs/pr/libretro-super-tgpulse-next-m1-update-body.md`.

The core repository remains on `main`. These review/publication documents
are updated locally; this PR publication does not change or republish core
source or binary releases. Libretro reviewer acceptance remains pending.
