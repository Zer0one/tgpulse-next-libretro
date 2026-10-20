use super::*;

fn machine() -> Model1System {
    let mut sys = Model1System::new(&crate::loader::Model1Roms {
        netmerc_procedural_audio: false, dsb: None, comm_board: false,
        ioboard_kind: crate::model1board::Kind::Original,
        nvram_default: vec![], maincpu: vec![], tgp: vec![], copro_tables: vec![],
        polygons: vec![], copro_data: vec![], iocpu: vec![0x76], sndcpu: vec![],
        mpcm1: vec![], mpcm2: vec![], ioboard_config: vec![],
    }).unwrap();
    sys.listctl[1] = 0x1f;
    sys.video.tgp_ram[0] = 1;
    set16(&mut sys.palette_ram, 0x1001, 0x801f);
    // Identity channel translation: isolate geometry/order from lighting.
    for i in 0..0x6000 { set16(&mut sys.colorxlat_ram, i, (((i >> 8) & 31) << 3) as u16); }
    sys
}

fn set16(bytes: &mut [u8], word: usize, value: u16) {
    bytes[word * 2..word * 2 + 2].copy_from_slice(&value.to_le_bytes());
}
fn set32(bytes: &mut [u8], word: usize, value: u32) {
    bytes[word * 2..word * 2 + 4].copy_from_slice(&value.to_le_bytes());
}
fn vertex(bytes: &mut [u8], at: usize, xyz: [f32; 3]) {
    for (i, f) in xyz.into_iter().enumerate() { set32(bytes, at + 2 * i, f.to_bits()); }
}

fn direct(sys: &mut Model1System, flags: u32, depth: f32) {
    let list = &mut sys.display_list[0];
    set32(list, 0, 2); set32(list, 2, TGP_RAM_BASE);
    vertex(list, 6, [10.0, -10.0, -1.0]);
    vertex(list, 14, [20.0, -10.0, 0.0]);
    set32(list, 20, flags); set32(list, 22, 0x8000_0000);
    vertex(list, 26, [10.0, -20.0, 0.0]);
    set32(list, 32, depth.to_bits());
    vertex(list, 34, [20.0, -20.0, -1.0]);
    set32(list, 42, 0xf);
}

#[test]
fn zero_and_negative_z_direct_polygons_are_projected_and_sorted_below_hud() {
    let mut sys = machine(); direct(&mut sys, 0x203, 7.0);
    let state = sys.save_state().unwrap();
    let quads = gpu_quads(&mut sys);
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].xs, [20, 10, 10, 20]);
    assert_eq!(quads[0].ys, [10, 10, 20, 20]);
    assert_eq!(quads[0].color & 0xffffff, 0xff0000);
    assert_eq!(quads[0].pad[0], 0);
    let mut frame = vec![0; SCREEN_W * SCREEN_H];
    let stats = render_below_hud(&mut sys, &mut frame);
    assert!(!stats.above_hud);
    assert_eq!(frame[15 * SCREEN_W + 15] & 0xffffff, 0xff0000);
    assert_eq!(sys.save_state().unwrap(), state);
    // Primitive projection is centered, not perspective-expanded, in wide mode.
    assert_eq!(gpu_quads_ws(&mut sys, 683.0)[0].xs, [113, 103, 103, 113]);
}

#[test]
fn transparent_pen_moire_uses_backdrop_and_degenerate_records_are_dropped() {
    let mut sys = machine(); direct(&mut sys, 0x2203, 0.0);
    sys.video.tgp_ram[0] = 0;
    set16(&mut sys.palette_ram, 0, 0x83e0);
    let quads = gpu_quads(&mut sys);
    assert_eq!(quads[0].moire, 1);
    assert_eq!(quads[0].color & 0xffffff, 0x00ff00);
    for at in [6, 14, 26, 34] {
        vertex(&mut sys.display_list[0], at, [0.0; 3]);
    }
    assert!(gpu_quads(&mut sys).is_empty());
}

#[test]
fn above_hud_geometry_is_emitted_in_a_separate_pass_with_bright_pixel_stencil() {
    let mut sys = machine();
    // One positive-z, front-facing object, translated by viewport center.
    sys.polygons = [
        10.0f32.to_bits(), (-10.0f32).to_bits(), 1.0f32.to_bits(),
        20.0f32.to_bits(), (-10.0f32).to_bits(), 1.0f32.to_bits(),
        0x4203, 0, 0, 1.0f32.to_bits(),
        10.0f32.to_bits(), (-20.0f32).to_bits(), 1.0f32.to_bits(),
        20.0f32.to_bits(), (-20.0f32).to_bits(), 1.0f32.to_bits(),
    ].to_vec();
    set32(&mut sys.display_list[0], 0, 0x41);
    set32(&mut sys.display_list[0], 2, TGP_RAM_BASE);
    set32(&mut sys.display_list[0], 4, 0);
    set32(&mut sys.display_list[0], 6, 1);
    set32(&mut sys.display_list[0], 8, 0xf);
    // Unlit UI bypasses the deliberately zero default light parameters.
    sys.video.tgp_ram[0] = 0x401;
    let mut frame = vec![0xff00_0000; SCREEN_W * SCREEN_H];
    let stats = render_below_hud(&mut sys, &mut frame);
    assert!(stats.above_hud);
    assert!(frame.iter().all(|&p| p == 0xff00_0000));
    let quads = gpu_quads(&mut sys);
    assert!(!quads.is_empty());
    assert!(quads.iter().all(|q| q.pad[0] == 1));
    let mut hud = vec![0; frame.len()];
    hud[15 * SCREEN_W + 15] = 0xfe00_0008; // threshold is strictly greater than 8
    hud[15 * SCREEN_W + 16] = 0xfe00_0009;
    hud[15 * SCREEN_W + 17] = 0xfe00_0000;
    frame[15 * SCREEN_W + 18] = 0xff00_0900; // bright pre-HUD scene also blocks
    compose_hud(&mut sys, &mut frame, &hud, true);
    assert_eq!(frame[15 * SCREEN_W + 15] & 0xffffff, 0xff0000);
    assert_eq!(frame[15 * SCREEN_W + 16], 0xfe00_0009);
    assert_eq!(frame[15 * SCREEN_W + 17] & 0xffffff, 0xff0000);
    assert_eq!(frame[15 * SCREEN_W + 18], 0xff00_0900);
}

#[test]
fn truncated_direct_records_stop_at_the_list_boundary() {
    let sys = machine();
    for size in [0, 8, 36, 40, 64] {
        let mut queue = vec![]; let mut stats = Model1RenderStats::default();
        let list = vec![0xff; size];
        assert!(direct::push_direct(&sys, &View::new(sys.video.lightparams),
            &list, 0, &mut queue, &mut stats) <= size / 2);
    }
}
