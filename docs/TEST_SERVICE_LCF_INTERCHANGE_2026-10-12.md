# Shared Test And Service Slot Order

Date: 2026-10-12
Status: User-approved shared convention; TGPulse descriptors/polling implemented and locally delivered.

## Approved Contract

Use the current SM2 physical slot order across equivalent Libretro profiles:
**L3 = Service; R3 = Test**. Match descriptors and actual input polling.
This resolves the previously open L3/R3 difference in LCF F09. Preserve native
bit polarity, gameplay bindings, explicit cabinet-specific exceptions and
None/device disable behavior. Global Controller Profile Type controls whether
the supported Test/Service slots are enabled; it does not exchange native bits.

## Source And Delivery Boundary

Reference: SM2 `src/libretro/input.cpp`, general Service A on L3 and Test A
on R3; `PORTING_PLAN.md` phase 3.7 and Controller Profile Type follow-ups.
TGPulse previously had the opposite order; both descriptors and polling are
now aligned under the user's 2026-10-12 authorization. All 92 adapter tests pass;
the macOS core is built, ABI/scope checked and installed with verified hash.
Callback-level global profile refresh is checked separately from actual visible
paused-menu acceptance, which remains pending. Previously saved physical remaps may need user
review after this intentional default-binding change. Do not rewrite remaps.

The shared convention is approved; implementation and verification in each
destination remain separate. No Supermodel implementation is implied.
