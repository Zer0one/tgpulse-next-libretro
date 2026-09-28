use super::*;

#[test]
fn timed_driver_pin_fans_out_to_main_and_dsb() {
    for partition in [1, 3, 64, 20000] {
        let mut linked = sound(true);
        linked.enable_model1_serial();
        linked.board.uart_control(0x4e);
        linked.board.uart_control(0x37);
        let mut remaining = 20000;
        while remaining > 0 {
            let n = remaining.min(partition);
            linked.run(n, SND_CPU_HZ);
            remaining -= n;
        }
        assert_eq!(linked.serial_fault(), None);
        assert_eq!(linked.dsb_fault(), None);
        assert!(linked.board.uart_rx_ready());
        assert_eq!(linked.board.main_uart_read(), 0x25);
        let dsb = linked.board.dsb.as_ref().unwrap();
        assert_eq!(dsb.playback().volume, !0x25 & 127);
        assert_eq!(dsb.diagnostics().0, 1);
        assert_eq!(dsb.diagnostics().1, 1);
    }
}

fn sound(linked: bool) -> SoundSystem {
    let mut rom = vec![0; 128];
    rom[..4].copy_from_slice(&0x00f0fff0u32.to_be_bytes());
    rom[4..8].copy_from_slice(&8u32.to_be_bytes());
    // from_board executes reset plus the first instruction. Leave that NOP
    // independent of the serial path selected immediately after construction.
    rom[8..10].copy_from_slice(&[0x4e, 0x71]);
    let mut p = 10;
    for (address, value) in [
        (0x00c20003u32, 0x4eu16),
        (0x00c20003, 0x37),
        (0x00c20001, 0x25),
    ] {
        // MOVE.B #imm,absolute.l through the actual 68000 interpreter/bus.
        for byte in [0x13, 0xfc, value.to_be_bytes()[0], value.to_be_bytes()[1]]
            .into_iter()
            .chain(address.to_be_bytes())
        {
            rom[p] = byte;
            p += 1;
        }
    }
    rom[p..p + 2].copy_from_slice(&[0x60, 0xfe]); // BRA self
    if linked {
        SoundSystem::with_dsb(rom, vec![0; 1024], vec![0; 1024], &firmware()).unwrap()
    } else {
        SoundSystem::new(rom, vec![0; 1024], vec![0; 1024])
    }
}
fn firmware() -> Vec<u8> {
    // Poll UART then use received byte as MPEG volume.
    let p = [
        0x3e, 0x4e, 0xd3, 0xf1, 0x3e, 0x37, 0xd3, 0xf1, 0xdb, 0xf1, 0xe6, 2, 0x28, 0xfa, 0xdb,
        0xf0, 0xd3, 0xe8, 0x76,
    ];
    let mut rom = vec![0; crate::dsbz80::FIRMWARE_SIZE];
    rom[..p.len()].copy_from_slice(&p);
    rom
}

#[test]
fn actual_sound_cpu_output_reaches_dsb_without_consuming_main_reply() {
    let mut linked = sound(true);
    let mut baseline = sound(false);
    linked.run(20_000, SND_CPU_HZ);
    baseline.run(20_000, SND_CPU_HZ);
    let d = linked.board.dsb.as_ref().unwrap();
    assert_eq!(d.fault(), None);
    assert_eq!(d.diagnostics().0, 1);
    assert_eq!(d.diagnostics().1, 1);
    assert_eq!(d.playback().volume, !0x25 & 127);
    assert_eq!(linked.board.tx, Some(0x25));
    assert_eq!(linked.samples, baseline.samples);
    assert_eq!(linked.cpu.regs.pc, baseline.cpu.regs.pc);
}

#[test]
fn incoming_main_commands_are_not_directly_mirrored_to_dsb() {
    let mut s = sound(true);
    s.board.uart_send(0xaa);
    assert_eq!(s.board.dsb.as_ref().unwrap().diagnostics().0, 0);
    assert_eq!(s.board.tx, None);
    assert_eq!(s.board.rx.front(), Some(&0xaa));
}

#[test]
fn source_data_status_does_not_claim_ready_with_full_holding_register() {
    let mut s = sound(true);
    s.board.write8(0xc20003, 0x4e);
    s.board.write8(0xc20003, 0x37);
    s.board.write8(0xc20001, 1);
    assert_eq!(s.board.read8(0xc20003) & UART_TX_RDY, 0);
    s.board.dsb.as_mut().unwrap().run_sound_cycles(100).unwrap();
    assert_ne!(s.board.read8(0xc20003) & UART_TX_RDY, 0);
}

#[test]
fn serial_fault_is_observable_and_stops_the_opt_in_pair() {
    let mut s = sound(true);
    s.board.write8(0xc20003, 0x4e);
    s.board.write8(0xc20003, 0x37);
    s.board.write8(0xc20001, 1);
    s.board.write8(0xc20001, 2);
    assert_eq!(
        s.board.dsb.as_ref().unwrap().fault(),
        Some(crate::dsbz80::Error::TransmitFull)
    );
    let pc = s.cpu.regs.pc;
    let state = bincode::serialize(&s.board.dsb.as_ref().unwrap().snapshot()).unwrap();
    s.run(20_000, SND_CPU_HZ);
    assert_eq!(s.cpu.regs.pc, pc);
    assert_eq!(
        state,
        bincode::serialize(&s.board.dsb.as_ref().unwrap().snapshot()).unwrap()
    );
    assert!(s.samples.is_empty());
}

fn playing_sound() -> SoundSystem {
    let mpeg = crate::mpeg::tests::stream(6);
    let mut firmware = vec![0; crate::dsbz80::FIRMWARE_SIZE];
    let size = mpeg.len() as u32;
    let mut p = 0;
    for (port, value) in [
        (0xe5, (size >> 16) as u8),
        (0xe6, (size >> 8) as u8),
        (0xe7, size as u8),
        (0xe0, 2),
    ] {
        firmware[p..p + 4].copy_from_slice(&[0x3e, value, 0xd3, port]);
        p += 4;
    }
    firmware[p] = 0x76;
    SoundSystem::with_dsb_audio(
        sound(false).board.rom,
        vec![0; 1024],
        vec![0; 1024],
        &firmware,
        mpeg,
    )
    .unwrap()
}

#[test]
fn dsb_gain_changes_output_not_decoder_state() {
    let mut reference = playing_sound();
    let mut adjusted = playing_sound();
    adjusted.set_gains(AudioGains {
        dsb: 25,
        ..AudioGains::default()
    });
    for _ in 0..1000 {
        reference.run(1000, SND_CPU_HZ);
        adjusted.run(1000, SND_CPU_HZ);
    }
    assert!(reference.samples.iter().any(|s| *s != (0, 0)));
    for (a, b) in reference.samples.iter().zip(&adjusted.samples) {
        assert_eq!(*b, (a.0 / 4, a.1 / 4));
    }
    assert_eq!(
        bincode::serialize(&reference.snapshot_dsb_path()).unwrap(),
        bincode::serialize(&adjusted.snapshot_dsb_path()).unwrap()
    );
    adjusted.set_gains(AudioGains::default());
    reference.samples.clear();
    reference.run(100_003, SND_CPU_HZ);
    adjusted.run(100_003, SND_CPU_HZ);
    assert_eq!(reference.samples, adjusted.samples);
}

#[test]
fn dsb_mute_is_output_only_and_unmute_has_no_stale_queue() {
    let mut audible = playing_sound();
    let mut muted = playing_sound();
    muted.set_mutes(AudioMutes {
        dsb: true,
        ..AudioMutes::default()
    });
    for _ in 0..1000 {
        audible.run(1000, SND_CPU_HZ);
        muted.run(1000, SND_CPU_HZ);
    }
    assert!(audible.samples.iter().any(|s| *s != (0, 0)));
    assert!(muted.samples.iter().all(|s| *s == (0, 0)));
    assert_eq!(audible.dsb_fault(), None);
    assert_eq!(
        bincode::serialize(&audible.snapshot_dsb_path()).unwrap(),
        bincode::serialize(&muted.snapshot_dsb_path()).unwrap()
    );
    assert_eq!(audible.cpu.regs.pc, muted.cpu.regs.pc);
    audible.samples.clear();
    muted.set_mutes(AudioMutes::default());
    assert!(muted.samples.is_empty());
    audible.run(100_000, SND_CPU_HZ);
    muted.run(100_000, SND_CPU_HZ);
    assert_eq!(audible.samples, muted.samples);
    assert_eq!(audible.sources(), DSB_SOURCES);
    assert_eq!(sound(false).sources(), MULTIPCM_SOURCES);
}

#[test]
fn dsb_conversion_snapshot_continues_mid_interval_and_rejects_bad_phase_atomically() {
    let mut a = playing_sound();
    a.render_cycles(123_457);
    let snapshot = a.snapshot_dsb_path().unwrap();
    let mut b = playing_sound();
    b.restore_fm_path(&a.snapshot_fm_path()).unwrap();
    b.restore_dsb_path(&bincode::deserialize(&bincode::serialize(&snapshot).unwrap()).unwrap())
        .unwrap();
    a.samples.clear();
    a.render_cycles(800_000);
    b.render_cycles(800_000);
    assert_eq!(a.samples, b.samples);
    assert_eq!(
        bincode::serialize(&a.snapshot_dsb_path()).unwrap(),
        bincode::serialize(&b.snapshot_dsb_path()).unwrap()
    );
    let before = bincode::serialize(&b.snapshot_dsb_path()).unwrap();
    let mut bad = b.snapshot_dsb_path().unwrap();
    bad.conversion = dsb::Conversion::default();
    assert_eq!(
        b.restore_dsb_path(&bad),
        Err(crate::dsbz80::Error::InvalidSnapshot)
    );
    assert_eq!(bincode::serialize(&b.snapshot_dsb_path()).unwrap(), before);
}

#[test]
fn dsb_is_added_before_final_clipping_without_changing_other_board_gains() {
    assert_eq!(
        mix_dsb(
            (32767, 32767),
            (32767, 32767),
            [30000; 2],
            [false; 3],
            [-20000; 2]
        ),
        (21767, 21767)
    );
    assert_eq!(
        mix_dsb(
            (32767, 32767),
            (32767, 32767),
            [30000; 2],
            [false; 3],
            [20000; 2]
        ),
        (32767, 32767)
    );
}
