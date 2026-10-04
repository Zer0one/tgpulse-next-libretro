# Upstream Backport Candidates

## Purpose And Maintenance

Maintain a single register of port changes that could benefit TGPulse-Next
upstream. Update it after each implementation phase and upstream comparison:
add newly identified candidates, amend their evidence and dependencies, and
record when upstream adopts or supersedes them. Keep stable candidate IDs.

This is a proposal register, not another implementation roadmap or approval
to modify/publish upstream. Before preparing a backport, compare the actual
upstream revision, isolate the smallest frontend-neutral change and identify
any standalone behavior requiring user review. Preserve the separate Git
histories. Exclude Libretro callbacks and deployment policy from neutral
machine/helper patches.

Initial comparison: 2026-10-04, local port implementation versus the reviewed
`upstream-next/main` snapshot
`f303b712455571b60cbea2249e1022935d398e22`. This records a fixed source
comparison; it is not a fresh remote-head check.

## Candidate Register

| ID | Candidate | Benefit And Smallest Adaptation | Evidence And Dependencies | Disposition |
| --- | --- | --- | --- | --- |
| B1 | NetMerc Initial Credit Counter Correction | Extend the existing SHA-1-identified factory-seed correction to clear the credit word at `0x18..0x1a`, alongside bookkeeping. Apply only when that seed actually initializes missing persistent data; preserve calibration and unrelated bytes. | Local `crates/tgpulse-core/src/loader.rs`, seed identity/idempotence tests and [native Backup RAM Clear comparison](LIBRETRO_U8_NETMERC_SETTINGS.md). Reviewed upstream corrects bookkeeping but does not clear this credit word. Recheck standalone save precedence before integration. | Candidate; user review required before upstream work. |
| B2 | Optional NetMerc Seed And Explicit Initialization Boundary | Make `netmerc_nvram.bin` optional and separate seed loading from corrective initialization. A valid personal save takes precedence. Patch the supplied seed only when it is actually needed; absence remains silent. A standalone automatic initialization feature would need its own explicit policy and native default/calibration data. | Local loader options and `initialize_netmerc_nvram_seed`, adapter Save RAM precedence and [U8 initialization evidence](LIBRETRO_U8_NETMERC_SETTINGS.md). Depends on standalone load/save lifecycle review; include B1 if adopting seed correction. Libretro option keys and frontend save containers are not transferable. | Candidate requiring standalone policy adaptation. |
| B3 | Configurable MVD Calibration Timing | Backport `OrientationTracker::with_calibration_timing` and preservation of configured timing across reset. Keep the standalone five-second default; optionally offer the tested three-second policy (one-second warmup, two-second measurement) without reducing the sample/movement thresholds. | Local `crates/tgpulse/src/input/motion.rs` and [U5 tests and limitations](LIBRETRO_U5_SENSORS.md). Reviewed upstream hardcodes two/five seconds. A standalone three-second selection requires corresponding GUI/notification updates and sensor-event validation; Libretro polling/axis conversion remains adapter-specific. | Neutral helper candidate; three-second standalone behavior requires separate approval. |
| B4 | Single-System Machine Library Builds | Reuse `model1`/`model2` Cargo feature gates and build-time catalogue selection for optional smaller standalone/library builds. Preserve both systems as the default; keep configuration/state layouts stable. | [Build scope and verification](LIBRETRO_BUILD_SCOPE.md), `crates/tgpulse-core/Cargo.toml`, `build.rs` and scoped machine modules. Reviewed upstream has no equivalent machine features. Standalone CLI/debugger dispatch and packaging must be assessed before selecting a reduced standalone build. | Architectural candidate; broader adaptation than B1/B3. |
| B5 | NetMerc Motor Pulse Observation | Preserve real port-D-bit-2 On activity even when On/Off occurs inside one frontend frame. Add bounded, consumable output observation outside serialized machine state; leave the native latch/timing unchanged and clear observations at reset/restore. Standalone may use it instead of reading only the final latch. | Local `model1io2.rs`, `model1io2/cpu.rs`, `model1board.rs` and [U6 verification](LIBRETRO_U6_RUMBLE.md): native pulse/isolation/state tests and positive synthetic-firmware-to-frontend delivery. Reviewed upstream traces write transitions but its pad dispatch reads the final latch. Physical response remains unverified; upstream output cadence requires review. | Optional observation helper; Libretro output was realigned to standalone final-latch sampling on user request. Equivalent physical response is unproved; no upstream adoption proposed without review. |

| B6 | Ordered And Validated Device BIOS Resources | Reuse the frontend-neutral resource loader with explicit caller-supplied paths, expected chip/size/SHA-1 and fallback to the next valid candidate. Separate strict game-chip identification from separately required I/O firmware. Standalone can adapt its own path order without importing Libretro System callbacks or LCD notification policy. | Local `loader/model1_bios.rs`, `roms_db.rs` and [U7 evidence](LIBRETRO_U7_LCD.md): external I/O loads, missing/wrong resource rejection, optional LCD font locations and invalid-candidate fallback. Preserve existing standalone defaults unless independently reviewed. | Neutral resource-helper candidate; standalone path/strictness policy requires review. |

| B7 | NetMerc Neutral Startup Publication Until First Decoded Measurement | Guard shared pose publication with the existing neutral pose until EPR-18021 completes and structurally validates its first 20-byte station-1 measurement. Serialize readiness/pending completion; clear on Reset; accept zero poses and retain normal subsequent firmware writes. | Local `model1io2.rs`, `model1io2/tracking.rs`, state format 5 and [U5 startup correction](LIBRETRO_U5_SENSORS.md#approved-startup-publication-correction--2026-10-04). Real-ROM port/standalone baseline traces matched the premature zero publication; the corrected port passes incomplete/zero/Reset/state checks and 1,000-frame native replay. Recheck the standalone state version and other firmware/packet formats before adoption; no physical-hardware behavior claim. | Candidate; upstream adaptation/review required. No upstream files changed. |

| B8 | Stationary Gyroscope Bias Refinement | Optional extra host bias, using the Fusion stationary-period/filter approach. Percentages scale learning speed; conservative gyro/gravity gates limit slow-turn absorption. Preserve orientation, manual calibration, time-gap/lifecycle reset and host/machine boundaries. | [U5 delivery and parameter sources](LIBRETRO_U5_SENSORS.md#adjustable-drift-compensation--2026-10-04), Fusion 0.02 Hz / 3-second dwell and GamepadMotionHelpers comparison. Local neutral helper tests prove synthetic reduction, 1°/s motion preservation, acceleration rejection, gyro-only operation and 60/120 Hz equivalence; 79 adapter tests and eight mock-sensor ABI groups pass. Optional sensor CSV evidence exposes read-only neutral-helper diagnostics; filesystem/frontend integration remains adapter-only. Libretro default 50% is user-approved initial moderate tuning, not multi-device validation. | Implemented locally; candidate for separate upstream review. Preserve standalone Off policy unless independently approved. Very slow yaw remains ambiguous. First user-provided DualSense Bluetooth capture and exact replay support the local moderate default; see `MVD_DUALSENSE_BLUETOOTH_2026-10-04.md`. Cross-device and external-reference validation remain absent. |
| B9 | Wing War Throttle Polarity Review | Compare in-game power direction with the standalone's current rising-ADC Throttle Up. The Libretro adapter now lowers the Wing War ADC for Throttle Up, preserves right-stick Up/Down bindings and swaps the trigger action labels while keeping their physical ADC responses. Review the standalone signal and UI semantics before considering a backport; do not copy Libretro button IDs into the standalone. | Local `crates/tgpulse-libretro/src/lib.rs`, `model1_controls.rs` and [Wing War adapter correction](LIBRETRO_MODEL1_FRONTEND_MAP.md#wing-war-throttle-polarity-correction-2026-10-04). Focused adapter tests prove ADC direction and trigger preservation; current-build in-game power direction remains user-run evidence. | Candidate for source comparison and standalone gameplay review; no upstream changes made. |
| B10 | Static MinGW Native TLS Linkage | Include `-lmingw32` inside the existing Windows GNU static-runtime link group, after C++/winpthread archives. Rust's earlier scan of mingw32 can precede the new native-TLS references, leaving `_tls_used` and `_tls_index` unresolved. Keep the existing static-runtime policy and apply the smallest toolchain-neutral configuration fix. | `.cargo/config.toml` in source commit `6d37b34`. The 0.1.0.4 candidate failed linking on MSYS2 GCC 16.2.0; the 0.1.0.5 Windows GNU production build links successfully with the same shared recipe. Full job/publication evidence is recorded in `LIBRETRO_CI.md`. The inherited standalone configuration used the same earlier link group; upstream compatibility and standalone builds still need separate review. | Candidate for upstream toolchain review; no upstream changes made. |

Existing upstream emulation fixes imported by U1 are not new backport
candidates. Automatic Holder and Alternative Audio Gains are existing
upstream features awaiting optional port adoption, not changes to send back.


## Required Record Updates

For each candidate retain the local source paths, comparison revision,
benefit, verification limits and necessary upstream adaptation. When a patch
is prepared, add its source commit and review link; when adopted, record the
upstream commit and close the candidate instead of deleting its history.
Candidate registration alone does not claim standalone acceptance.

## B3 Calibration Validation Follow-Up — 2026-10-04

Stationary-pad rejection was reported around the second/third attempt,
followed by successful retries; the rejection reason is not recalled. [The tolerance review](LIBRETRO_U5_SENSORS.md#calibration-tolerance-review--2026-10-04)
reproduces both insufficient polling and single-transient rejection with the
actual helper. The physical cause is unconfirmed pending the reported
rejection category. B3 remains a configurable-timing candidate; a standalone
three-second default or tolerance change is not yet justified by this evidence.
