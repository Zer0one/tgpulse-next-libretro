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
