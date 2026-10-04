# U1a: Sega NetMerc City Workaround And General Option Visibility

## Reference And Adaptation

The standalone Settings checkbox and `Config` default enable the City workaround.
The existing `Model1System::set_netmerc_city_workaround` API updates the selected
preference live and activates the firmware-specific City conversion only for
`Kind::NetMerc`. SM2's Video correction options provide Enabled/Disabled naming
and category conventions; neither reference core implements this machine fix.
The adapter uses the existing API without adding frontend code to shared hardware.

Key: `tgpulse_next_netmerc_city_workaround`. Category: Video.
Values: Enabled (default), Disabled. The option is always visible and changes
take effect on subsequent matching conversions. This is separate from the
conditional NetMerc finite-arithmetic policy. Reset and state restoration retain
the destination frontend preference; it is not serialized as machine state.

The user's general rule is applied to all general Core Options, including gains,
driving ranges, MVD, rumble, supersampling and widescreen. Registration, load
and unload explicitly publish them as visible. Applicability remains guarded
at runtime; supersampling help states its hardware-rendering requirement.
Reviewed per-set NVRAM fields and per-set Linked Cabinets retain the existing
explicitly agreed filtering. Native input descriptors remain profile-specific.

## Verification (2026-10-04)

- 59 adapter tests pass, including modern/legacy City defaults and general
  option visibility, excluding the agreed per-set selectors.
- The native Model 1 City test passes for Original, Wing War and NetMerc:
  toggling changes only NetMerc's conversion flag, preserves arithmetic policy,
  leaves snapshot bytes unchanged and retains destination preference on restore.
- Existing NetMerc controls ABI runner passes, including always-visible MVD
  controls after unload and native/virtual bindings and lifecycle checks.
- Existing scope runner passes all nine original sets at 120 frames per set:
  identical video, audio, Save RAM, machine snapshots and state continuation.
- Offline locked macOS release build and Model 1 artifact ABI/dependency/empty
  lifecycle gates pass. This is not City gameplay or visual road acceptance.
- Development core and matching info installed in local RetroArch. Installed
  core matches build SHA-256:
  `6b3dad1cb0308df6fdbea0d5ab6ee464af831bf815f74ff010e11e16711ce243`.

Local logs and reports: `/private/tmp/tgpulse-u1a/` (`tests-final.log`,
`city-native.log`, `artifact.log`, `controls.json`, `regression.json`,
`deployment.json`). No ROMs, user configuration or saves are part of delivery.

Required reasoning: High; effort S; estimated account usage 0.5–1 percentage
point. Shared account usage changed from 31% to 32% (rounded; includes other
chats). Reset: 2026-10-10 09:49:17 CEST. No source publication is part of U1a.
