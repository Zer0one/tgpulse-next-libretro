# LCF Controller Profile Paused-Menu Verification

Date: 2026-10-13

## Environment And Isolation

RetroArch 1.22.2 Nightly on macOS ARM64, rendered RGUI menu, Virtua Racing.
The installed public `tgpulse_next_m1_libretro.dylib` was used; SHA-256:
`89195e6d82d1685204ff19a99c39553181eb9b992883510b91bf6559b122e185`.
Configuration and saves were copied from the existing isolated test fixture to
`/private/tmp/tgpulse-lcf-profile-ui-20261013`; personal files were not edited.
The two previous test instances were already closed when inspected.

## Observed Result

Opened the game menu through the native Command → Menu Toggle action, then
Core Options → Input. Controller Profile Type was the first Input entry and
offered Normal / Normal + Test/Service.

Selected Normal while the game remained paused. Immediately opened
Controls → Port 1 Controls → Device Type: the choices were None and
Driving: Sequential + VR4, without the Test/Service suffix.

Returned through the menus without Resume or Restart and selected
Normal + Test/Service. Immediately reopening the same Device Type list showed
None and the same game profile with the Test/Service Slots suffix. The game
background stayed on the same scene throughout this menu-only sequence.
No stale full/reduced alternative remained in either Device Type list.

Exited the sole test instance using the native Quit RetroArch menu action;
the subsequent process check found no running RetroArch instance. No forced
termination was needed. This closes the previously pending E6 visual check.

## Evidence Boundary

This verifies actual visible paused-menu metadata refresh in this frontend,
independently of the existing ABI callback/re-entry tests. Native Service/Test
bit delivery, historical IDs, None and other profiles retain their earlier
adapter/ABI evidence; no new physical-controller, rumble, other-platform or
gameplay acceptance is claimed. Screenshots were inspected through CUA; no
exported screenshot files or process exit status are claimed.
No code change, rebuild, installation, commit, push or release was necessary.
