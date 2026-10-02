# Model 1 binding audit

Date: 2026-10-02. Scope: all ten database sets, seven adapter profiles, both
controller ports. This is a source-level audit of the current working tree,
including uncommitted code. Initial findings and subsequent implemented corrections are recorded below.

## Method and references

Compare the complete path: default physical binding, player assignment, sampling,
label/descriptor and native I/O/ADC encoding. Standalone custom user bindings and
keyboard bindings do not transfer into Libretro; the frontend owns those.

Sources inspected:

- Current adapter: `crates/tgpulse-libretro/src/lib.rs` (profile selection,
  controller names, descriptors, polling) and `model1_controls.rs` (native ports).
- Standalone: `input/signals/mod.rs`, `sampling.rs`, `sdl_pads.rs`, `cabinet.rs`,
  `cabinet/analog.rs`, `player_tests.rs`, `signals/audit.rs`, and
  `docs/INPUT_AUDIT.md` (including the corrected VF semantics).
- SM2: `src/libretro/input.cpp` physical defaults, fighting native bits,
  descriptors, common controls and driving polling. Do not transfer Model 2
  native bits to unrelated Model 1 cabinets.
- Supermodel: `Src/OSD/Libretro/libretro.cpp` view descriptors; its single-view
  Up and four-view ordering differ from SM2. The existing standalone ordering
  is the chosen contract for Model 1.

Physical equivalence: RetroPad B/A/Y/X are South/East/West/North. L/R are
shoulder buttons, not L2/R2. TGPulse's SDL translation confirms that its gilrs
LeftTrigger/RightTrigger button names mean shoulders, whereas LeftZ/RightZ
are analog triggers. Standalone stick Y is positive up; Libretro is positive
 down, so the adapter inversion must be preserved.

## Findings at audit time (resolved below)

| ID | Severity | Affected sets/ports | Expected from reference | Current core | Required correction |
| --- | --- | --- | --- | --- | --- |
| B1 | High: wrong native action | `vf`, P1/P2 | B/L = Kick -> native 0x02; A/R = Punch -> 0x01; Y = Guard -> 0x04. Standalone and SM2 agree. | Polling constructs Kick/Punch/Guard correctly, but `VfCabinet::into_native` maps the first action to 0x01 and the second to 0x02. Actual Kick and Punch are swapped. | Swap the first two native masks, retaining physical defaults and labels. Verify face and shoulder alternatives on each port using independent expected bits. |
| B2 | Missing alternative bindings | `wingwar`, `wingwarj`, `wingwaru`, `wingwar360`, `swa`, `swaj`, `netmerc`; P1, plus SWA Gunner P2 | L duplicates B for Machine Gun/Laser/Button 1; R duplicates A for Missile/Torpedo/Button 2. | Flight polling reads only B/A. Shoulder alternatives are absent. | Add the standalone OR alternatives and matching descriptors; preserve Pilot/Gunner isolation. |
| B3 | Missing throttle source | Four Wing War sets and two SWA sets, P1 | Right stick Up = Throttle Up; Down = Throttle Down, alongside L2/R2. | Only L2/R2 are sampled and described. Right stick does nothing. | Restore the standalone half-axis alternatives with correct Libretro Y sign and standalone combination rule. Gunner has no throttle. |
| B4 | Different analog response | All flight profiles, SWA P1/P2 | Standalone removes values inside the 0.15 deadzone and preserves values outside it before cabinet calibration. | `stick()` additionally rescales surviving travel by `(abs(v)-0.15)/0.85`. Mid-travel ADC values differ. | Match standalone flight sampling. Keep VR's separate steering curve, which already matches its standalone racing path. |
| B5 | Incomplete descriptors | `vf`, P1/P2 | L/R action alternatives are named in the reference controller UI. | VF runtime already accepts L/R, but descriptors list only B/A/Y. | Add Kick on L and Punch on R to descriptors. This does not require adding runtime aliases: they already exist. |

B4 example: SWA positive horizontal input 0.5 produces calibrated ADC 77 in
standalone, approximately 86 in the core, despite the same centre, polarity and
endpoints. This is a response-curve difference rather than reversed direction.

## Complete profile coverage

| Profile and sets | Main binding/port result | Remaining findings |
| --- | --- | --- |
| Driving: `vr`, `vformula` | Left stick steering, R2 accelerator, L2 brake, L/R sequential down/up, Down/Left/Right/Up views, native view/gear masks and ADC order match standalone. Both Coin slots and shared Test/Service match. | None found in this scope. VR does not use a 4-speed H-gate in the standalone, so its absence is correct. |
| Fighting: `vf` | D-pad direction masks, Guard, independent coins/starts and shared operators match. Face/shoulder polling is present, but native action order and labels need separate fixes. | B1, B5. |
| Flight: `wingwar`, `wingwarj`, `wingwaru` | B/A/Y weapons, Down/Left/Right/Up views, native weapon/view bits, left-stick axis polarity, throttle polarity, ADC order/rest/travel match. No native Coin 2 or P2 gameplay is invented. | B2, B3, B4. |
| Flight R360: `wingwar360` | Same weapon positions, intentionally no view switches, distinct stick polarity, native Coin 2, throttle polarity and channel order match. | B2, B3, B4. |
| SWA: `swa`, `swaj` | Independent Pilot/Gunner sticks and weapons, calibrated ADC 0/1 and 4/5, Pilot VR1 Down/Up alias, coins and shared operators match. No Gunner Start/View/throttle is added. Both requested profile names include VR1; this name does not imply a separate Gunner View switch. | B2, B3, B4. |
| NetMerc: `netmerc` | B/A/Y actions, no Start or Coin 2, distinct X/Y polarity and ADC channels 0/2 match standalone. | B2, B4. Source inspected only; boot/gameplay remains blocked and NetMerc is excluded from the NVRAM campaign. |

## Existing deliberate adaptations and limits

- Test=L3 and Service=R3 match standalone and the user preference. SM2 currently
  reverses these physical positions; that divergence is deliberate.
- P2 operator controls exist even for single-player profiles. Coin 2 is exposed
  only where the standalone has its native slot. Removing a port disables its
  contributions. SWA alone keeps distinct role names.
- VF's unapproved analog direction extension was removed in the follow-up
  correction below; its native joystick remains digital, as in both references.
- Libretro digital trigger fallback and frontend keyboard remapping differ from
  standalone host input delivery. They are not additional arcade switches.
- VR has software ramping for steering recentering and pedal release, matching
  the standalone racing sampler's deadzones and step sizes (10/20).

No claim of exhaustive physical-device/gameplay correctness follows from this
source audit. Existing adapter tests validate their own current expectations;
B1 demonstrates why those expectations must be checked against the independent
standalone/SM2 native-port contract.

## Correction following user review (2026-10-02)

B1: VF native masks now map B to Kick 0x02, A to Punch 0x01 and Y to Guard
0x04, independently for P1/P2. At the user's explicit request, L/R alternatives
are removed from VF rather than restored/described. This supersedes B5's
original recommendation and is a deliberate difference from standalone/SM2.

B2: Flight profiles now OR L/R with B/A for the first two weapons. Descriptors
name those alternatives. SWA Gunner receives the same alternatives on its own
native bits, without affecting Pilot or disabled ports.

B3: Right stick Up/Down now supplements L2/R2 throttle for Wing War/R360 and
SWA Pilot. Each half-axis takes the strongest source before subtraction,
matching standalone signal alternatives; Gunner has no throttle input.

B4: Flight sticks retain the original magnitude outside the 0.15 deadzone and
use standalone positive/negative normalization. VR steering remains separate; VF's unsupported analog extension was removed
in the follow-up correction below. Midpoint ADC values match the
standalone calibration (including SWA Pilot/Gunner 0.5 -> 77).

The requested Wing War name is corrected to `Flight: Wing War + VR4` on both
ports; the previous V4 spelling was a typo. All original findings are addressed.

Verification: 31 adapter tests passed, including both VF ports, removed shoulder
bindings, flight shoulder alternatives, disabled-port isolation, calibrated
midpoints and right-stick throttle direction/role isolation. These checks do
not constitute physical-controller/gameplay acceptance.


## Audited source snapshots

- `crates/tgpulse-libretro/src/lib.rs`: `1993736fe4b96174f51418a00a684d6b655611433e9e716caf3659670eaa8fb4`
- `crates/tgpulse-libretro/src/model1_controls.rs`: `9b5be256b37cc341f1b14ad7754005ff99a7e5ca73f2787b1d1c80e1a884d530`
- `crates/tgpulse/src/input/signals/mod.rs`: `b11cde574aa7d1b8ec2603895f0fcac0229e14dedc5a54ca18288e1fc74590b2`
- `crates/tgpulse/src/input/cabinet.rs`: `b166b24517be30d8581465bee10b8d1bbec01de4d1e1ea2ebbef8644651a0698`
- `crates/tgpulse/src/input/cabinet/analog.rs`: `fbbe270c0c748680195a4ca8f81495c5e56daf63ffec2578e7983325fb443d81`
- `crates/tgpulse/src/input/sampling.rs`: `a538b117bb07cf7a10682efc1defb5750fbaaa4edd309bdcf59fb7bd719cef96`

Correction delivery: offline macOS release build and ABI load passed; the
Development core and matching `.info` were installed in RetroArch. Built and
installed SHA-256 match:
`de500afa386320624170cff6297400c69f85f7605c48e030dd78ae0dd36f8e55`.

## Follow-up: one action, multiple bindings (2026-10-02)

The complete descriptor and polling paths were rechecked for all ten sets and
both ports against current standalone signals/sampling/cabinet encoding and
SM2 descriptor/polling code. The previous test expecting `VR1 (Alternate)` was
incorrect and is corrected along with the implementation.

| Finding | Correction | Reference evidence |
| --- | --- | --- |
| SWA Pilot Down and Up were named as separate VR1 inputs. | Both descriptors now read `VR1`; both retain the same native IN1 bit 0x10. No new switch is introduced. | Standalone `sampling.rs::signal` aliases assignable pad View4 to View1 on single-view cabinets; `cabinet.rs` exposes just View1 for SWA. SM2 uses identical action labels for multiple bindings, e.g. Desert Tank Machine Gun/Cannon and dual Elevation sources. |
| VF exposed Horizontal/Vertical Movement and internally synthesized digital directions from analog axes. | Removed both axis descriptors and analog polling for both players. D-pad directions, Kick/Punch/Guard, coins, starts and shared operators remain. | Standalone Up/Down/Left/Right signals use digital pad directions; SM2 fighting descriptors and polling expose digital directions only. VF has no native analog channel. |

Flight shoulder alternatives already share each weapon's existing label and
native action; throttle trigger/right-stick sources resolve existing throttle
signals. Four-view cabinets retain four real view bits. SWA Gunner has no
View/Start/throttle switch added, regardless of its user-requested profile name.
Single-player ports have only the agreed shared Test/Service and native Coin 2
where applicable. NetMerc remains a source-only audit because its boot is blocked.
No additional unsupported native gameplay controls were found in this scope.

Verification: 34 offline adapter checks pass. Both SWA bindings assert the same
VR1 label and native bit; VF full-scale analog axes produce no direction bits,
while each player's digital direction path remains active. Existing weapon,
throttle, disabled-port, operator and native coin-slot checks still pass.
The release build is followed by Development installation and SHA-256 matching.
Physical controller/gameplay acceptance remains separate.

Follow-up deployment: macOS release build and installed Development core match
SHA-256 `86c51cfb5904175a5f7a8b73138d6d0efd2e204b243f1dd5fd4e2db7c883840c`.
