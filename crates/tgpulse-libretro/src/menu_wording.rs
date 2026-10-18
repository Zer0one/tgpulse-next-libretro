//! Reviewed frontend metadata only; emulation and stored option values are unchanged.
//! Source: Framework Menu Review, 203-row revision, SHA-256
//! d691cc31ebb32b1e666c962629f83357370e80b002bb5acc5de32242e1f09f91.
//! Native per-game NVRAM operator fields are deliberately excluded.

use std::ffi::CStr;

pub fn find(key: &CStr) -> Option<(&'static CStr, &'static CStr, &'static CStr)> {
    match key.to_bytes() {
        // Menu Review row 6, Order 7.
        b"tgpulse_next_av_timing" =>
            Some((c"A/V Timing (Restart Required)", c"Select the video cadence and corresponding audio timing.", c"video")),
        // Menu Review row 9, Order 8.
        b"tgpulse_next_aspect_ratio" =>
            Some((c"Aspect Ratio", c"Select Auto, 4:3 or 16:9. Auto follows native presentation and applicable Widescreen Mode. Explicit 4:3 suppresses expansion; 16:9 with Widescreen Mode Disabled stretches the original image. Native widescreen settings are preserved.", c"video")),
        // Menu Review row 12, Order 40.
        b"tgpulse_next_initial_nvram_setup" =>
            Some((c"Automatic Initial NVRAM Setup (Restart Required)", c"Initialize missing save data from a validated NVRAM template and apply approved initial settings while preserving existing valid saves.", c"system")),
        // Menu Review row 13, Order 34.
        b"tgpulse_next_automatic_network_vr" | b"tgpulse_next_automatic_network_vformula" | b"tgpulse_next_automatic_network_wingwar" | b"tgpulse_next_automatic_network_wingwaru" | b"tgpulse_next_automatic_network_wingwarj" | b"tgpulse_next_automatic_network_wingwar360" =>
            Some((c"Auto Network Role Settings", crate::automatic_network::ROLE_HELP, c"system")),
        // Menu Review row 46, Order 1.
        b"tgpulse_next_controller_profile_type" =>
            Some((c"Controller Profile Type", c"Select whether controller profiles include Test/Service inputs while retaining gameplay controls and remapping.", c"input")),
        // Menu Review row 75, Order 3.
        b"tgpulse_next_accelerator_output_range" =>
            Some((c"Driving Accelerator Output Range", c"Scale accelerator output from its released position to adjust the input travel needed for full acceleration.", c"input")),
        // Menu Review row 76, Order 4.
        b"tgpulse_next_brake_output_range" =>
            Some((c"Driving Brake Output Range", c"Scale brake output from its released position to adjust the input travel needed for full braking.", c"input")),
        // Menu Review row 77, Order 5.
        b"tgpulse_next_steering_output_range" =>
            Some((c"Driving Steering Output Range", c"Scale steering output around its center to adjust the input travel needed for full lock.", c"input")),
        // Menu Review row 78, Order 6.
        b"tgpulse_next_steering_response" =>
            Some((c"Driving Steering Response", c"Select the steering response curve to adjust sensitivity around the center while retaining full travel.", c"input")),
        // Menu Review row 79, Order 13.
        b"tgpulse_next_dsb_gain" =>
            Some((c"DSB (MPEG) Gain", c"Adjust this sound source's output gain without stopping its emulation when muted.", c"audio")),
        // Menu Review row 89, Order 14.
        b"tgpulse_next_ym3438_gain" =>
            Some((c"FM (YM3438) Gain", c"Adjust this sound source's output gain without stopping its emulation when muted.", c"audio")),
        // Menu Review row 98, Order 71.
        b"tgpulse_next_gamepad_rumble" =>
            Some((c"Gamepad Rumble", c"Enable gamepad vibration using the game's force-feedback or rumble effects.", c"input")),
        // Menu Review row 108, Order 33.
        b"tgpulse_next_rom_repairs" =>
            Some((c"Known Bad Dump ROM Repairs (Restart Required)", c"Repair a recognized bad ROM dump in memory after verifying its identity, without changing archive files.", c"system")),
        // Menu Review row 121, Order 35.
        b"tgpulse_next_linked_cabinets_vr" | b"tgpulse_next_linked_cabinets_vformula" | b"tgpulse_next_linked_cabinets_wingwar" | b"tgpulse_next_linked_cabinets_wingwaru" | b"tgpulse_next_linked_cabinets_wingwarj" | b"tgpulse_next_linked_cabinets_wingwar360" =>
            Some((c"Linked Cabinets", crate::automatic_network::LINK_HELP, c"system")),
        // Menu Review row 125, Order 9.
        b"tgpulse_next_volume" =>
            Some((c"Master Volume", c"Adjust the overall mixed audio output level.", c"audio")),
        // Menu Review row 132, Order 15.
        b"tgpulse_next_multipcm1_gain" =>
            Some((c"MultiPCM 1 Gain", c"Adjust this sound source's output gain without stopping its emulation when muted.", c"audio")),
        // Menu Review row 133, Order 16.
        b"tgpulse_next_multipcm2_gain" =>
            Some((c"MultiPCM 2 Gain", c"Adjust this sound source's output gain without stopping its emulation when muted.", c"audio")),
        // Menu Review row 140, Order 2.
        b"tgpulse_next_nvram_settings" =>
            Some((c"NVRAM Settings", c"Apply reviewed operator overrides to saved NVRAM while preserving unrelated fields and calibration.", c"system")),
        // Menu Review row 159, Order 10.
        b"tgpulse_next_renderer" =>
            Some((c"Renderer (Restart Required)", c"Select hardware-accelerated or software rendering.", c"video")),
        // Menu Review row 168, Order 22.
        b"tgpulse_next_netmerc_audio_donor" =>
            Some((c"Sega NetMerc: Audio Donor (Restart Required)", c"Use substitute audio samples when original samples are unavailable.", c"audio")),
        // Menu Review row 169, Order 23.
        b"tgpulse_next_netmerc_city_workaround" =>
            Some((c"Sega NetMerc: City Workaround", c"Correct road geometry through a TGP conversion workaround.", c"video")),
        // Menu Review row 170, Order 24.
        b"tgpulse_next_netmerc_diagnostic_opacity" =>
            Some((c"Sega NetMerc: Diagnostic Background Opacity", c"Adjust the diagnostic LCD background opacity while keeping its glyphs opaque.", c"video")),
        // Menu Review row 171, Order 25.
        b"tgpulse_next_netmerc_diagnostic_display" =>
            Some((c"Sega NetMerc: Diagnostic Display", c"Display the cabinet's diagnostic LCD within the game image.", c"video")),
        // Menu Review row 172, Order 26.
        b"tgpulse_next_netmerc_diagnostic_position" =>
            Some((c"Sega NetMerc: Diagnostic Position", c"Select the screen corner used for the diagnostic LCD.", c"video")),
        // Menu Review row 173, Order 27.
        b"tgpulse_next_netmerc_mvd_drift_compensation" =>
            Some((c"Sega NetMerc: MVD Drift Compensation", c"Refine gyroscope bias during detected stillness to reduce tracking drift without recentering the view.", c"input")),
        // Menu Review row 174, Order 28.
        b"tgpulse_next_netmerc_mvd_gravity_stabilization" =>
            Some((c"Sega NetMerc: MVD Gravity Stabilization", c"Use accelerometer gravity to stabilize head-tracking pitch and roll.", c"input")),
        // Menu Review row 175, Order 29.
        b"tgpulse_next_netmerc_mvd_horizontal_range" =>
            Some((c"Sega NetMerc: MVD Horizontal Range", c"Set the horizontal head-tracking range controlled by the analog stick.", c"input")),
        // Menu Review row 176, Order 30.
        b"tgpulse_next_netmerc_mvd_input" =>
            Some((c"Sega NetMerc: MVD Input", c"Select motion sensors, analog-stick input or a fixed camera for head tracking.", c"input")),
        // Menu Review row 177, Order 31.
        b"tgpulse_next_netmerc_mvd_sensor_diagnostics" =>
            Some((c"Sega NetMerc: MVD Sensor Diagnostics", c"Record motion input, calibration and orientation data for head-tracking diagnostics.", c"input")),
        // Menu Review row 178, Order 32.
        b"tgpulse_next_netmerc_mvd_vertical_range" =>
            Some((c"Sega NetMerc: MVD Vertical Range", c"Set the vertical head-tracking range controlled by the analog stick.", c"input")),
        // Menu Review row 181, Order 11.
        b"tgpulse_next_smooth_shadows" =>
            Some((c"Smooth Shadows (Restart Required)", c"Blend stippled shadows instead of preserving the original dither pattern.", c"video")),
        // Menu Review row 188, Order 56.
        b"tgpulse_next_supersampling" =>
            Some((c"Supersampling (Restart Required)", c"Render additional internal samples per output pixel to improve image quality. Applies only to Vulkan / OpenGL / GLES renderers.", c"video")),
        // Menu Review row 190, Order 12.
        b"tgpulse_next_timing_overlay" =>
            Some((c"Timing / FPS Overlay", c"Display timing and frame-rate measurements to inspect emulation and rendering performance.", c"video")),
        // Menu Review row 203, Order 207.
        b"tgpulse_next_widescreen_mode" =>
            Some((c"Widescreen Mode", c"Expand the 3D field of view and report 16:9 automatically, optionally stretching both 2D layers. Applies only to games without native widescreen support, with hardware rendering and Aspect Ratio set to Auto or 16:9. Explicit 4:3 suppresses expansion. Disabled preserves Aspect Ratio and native monitor settings. Native widescreen is unchanged. Changes apply when returning to the game without a reset.", c"video")),
        _ => None,
    }
}
