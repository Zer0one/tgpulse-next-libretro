# TGPulse-Next Libretro working notes

This is an independent Libretro-port repository, not a GitHub fork. The initial
commit imports a documented TGPulse-Next source snapshot. `origin` is this
repository; `upstream-next` is Zer0one/TGPulse-Next; `upstream-original` is
deepblueworks/TGPulse. Keep those projects and their Git histories separate.

The initial porting scope is Model 1. Use SM2-Emu Libretro as a reference for
adapter structure, frontend lifecycle and verification discipline, not as a
source-code dependency. Reuse the imported TGPulse-Next machine implementation;
keep Libretro ABI, callbacks and frontend settings in a narrow adapter rather
than spreading frontend dependencies through emulated hardware. Preserve the
standalone build unless an explicit milestone changes that requirement.

Keep emulated time, frame and sample production, input delivery, ROM/resource
provision, NVRAM and full machine state explicit. The future frontend should
own host devices and paths. Do not claim Libretro support from the source
snapshot: an adapter, builds and real frontend tests are still required.

Before installing software or dependencies, ask the user. Never commit ROMs,
personal settings, saves or generated build products. Keep tests and publication
evidence separate from actual game or controller testing. Commit and push future
porting work only when requested for that work.

Write project documentation and workbook labels in English unless the user
explicitly requests another language. Follow `docs/REFERENCE_WORKFLOW.md` for
each phase: inspect the current reference implementation, reuse its established
workflow and deliverables, and document the smallest necessary adaptation.
Ask about genuine adaptation decisions; routine implementation choices can
proceed under the existing authorization. Report the required model/reasoning
level before substantial work and account usage/reset after verified phases.
Keep `docs/LIBRETRO_ROADMAP.md` as the sole active implementation roadmap for
this port. Supporting documents record design, procedures and evidence without
parallel status plans. User-run game and controller trials are outside that roadmap.
Before creating or changing a roadmap phase, read the corresponding current
sections of SM2-Emu Libretro `PORTING_PLAN.md` and, when relevant, Supermodel
`Docs/ROADMAP.md`. Record the matching reference phase, the Model 1 adaptation
and any deliberate exclusion in the single port roadmap before setting its
priority or effort estimate.
Reviewing applicable TGPulse-Next upstream changes is routine maintenance,
not a port roadmap milestone. Preserve the separate Git histories and keep
desktop-only host code outside the Libretro adapter.

After each verified macOS release build of the Libretro core, install that build
in local RetroArch as `tgpulse_next_m1_libretro.dylib`, install its matching
`tgpulse_next_m1_libretro.info` with the public name and label, and verify
the installed core's SHA-256 against the build. This core deployment is
authorized by the user for subsequent phases; it does not authorize installing
dependencies or replacing updater-managed cores. Report any deployment failure
as an incomplete phase, rather than reporting the build alone as delivered.
This follows the user's 2026-10-05 instruction to keep local names non-Development.
Keep runtime library_name and metadata corename identical for Netplay discovery;
back up replaced local files and retire the former Development copy.
