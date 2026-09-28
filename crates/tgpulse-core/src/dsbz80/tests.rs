use super::*;

fn board(program: &[u8]) -> Board {
    let mut rom = vec![0; FIRMWARE_SIZE];
    rom[..program.len()].copy_from_slice(program);
    Board::new(&rom).unwrap()
}
fn uart() -> uart::Uart {
    let mut u = uart::Uart::default();
    for byte in [0, 0, 0, 0x40, 0x4e, 0x37] {
        u.control(byte).unwrap();
    }
    u
}
fn bit(u: &mut uart::Uart, value: bool) {
    u.rx = value;
    for _ in 0..16 {
        u.tick();
    }
}
fn frame(u: &mut uart::Uart, byte: u8, stop: bool) {
    bit(u, false);
    for i in 0..8 {
        bit(u, byte & (1 << i) != 0);
    }
    bit(u, stop);
}
fn bytes(b: &Board) -> Vec<u8> {
    bincode::serialize(&b.snapshot()).unwrap()
}

#[test]
fn memory_map_rom_is_read_only_and_ports_decode_low_byte() {
    let mut b = board(&[0x76]);
    b.cpu.io.write_byte(0, 0);
    b.cpu.io.write_byte(0x8000, 0x12);
    b.cpu.io.write_byte(0xffff, 0x34);
    assert_eq!(b.cpu.io.read_byte(0), 0x76);
    assert_eq!(b.cpu.io.read_byte(0x8000), 0x12);
    assert_eq!(b.cpu.io.read_byte(0xffff), 0x34);
    b.cpu.io.port_out(0x12e8, 0xc5);
    assert_eq!(b.playback().volume, 0x3a);
    b.cpu.io.port_out(0xea, 15); // firmware touches these unmapped outputs
    b.cpu.io.port_out(0xeb, 10);
    assert_eq!(b.cpu.io.port_in(0xea), 0xff);
    assert!(b.cpu.io.fault.get().is_none());
    assert!(matches!(Board::new(&[0; 16]), Err(Error::FirmwareSize)));
}

#[test]
fn playback_partial_writes_loop_latches_and_bit_position() {
    let mut p = Playback::default();
    p.write(0xe2, 0x12).unwrap();
    p.write(0xe3, 0x34).unwrap();
    assert_eq!(p.start, 0);
    // Preserve an incomplete 24-bit register write across serialization.
    p = bincode::deserialize(&bincode::serialize(&p).unwrap()).unwrap();
    p.write(0xe4, 0x56).unwrap();
    assert_eq!(p.start, 0x123456);
    p.write(0xe7, 0x80).unwrap();
    p.write(0xe0, 2).unwrap();
    assert_eq!(p.bit_position, 0x123456 * 8);
    p.write(0xe4, 0x78).unwrap();
    p.write(0xe7, 0).unwrap();
    assert_eq!(p.start, 0x123456);
    assert_eq!(p.loop_start, 0x123478);
    assert_eq!(p.end, 0x80);
    assert_eq!(p.loop_end, 0); // decoder must keep prior end at transition
    p.write(0xe0, 0).unwrap();
    assert_eq!(p.mode, 0);
    assert_eq!(p.write(0xe9, 3), Err(Error::UnsupportedPan(3)));
    assert_eq!(p.write(0xe0, 3), Err(Error::UnsupportedPlayback(3)));
    let b = board(&[0x76]);
    b.cpu.io.state.borrow_mut().playback = p;
    assert_eq!(b.cpu.io.port_in(0xe2), 0x12);
    assert_eq!(b.cpu.io.port_in(0xe3), 0x34);
    assert_eq!(b.cpu.io.port_in(0xe4), 0x56);
}

#[test]
fn uart_8n1_transmission_occupies_ten_bits_of_sixteen_wire_ticks() {
    let mut u = uart();
    u.write(0xa5).unwrap();
    // Independent framing oracle: start, eight LSB-first data bits, stop.
    // 160 ticks at the configured 500 kHz wire clock = 320 microseconds.
    let bits = [
        false, true, false, true, false, false, true, false, true, true,
    ];
    for expected in bits {
        for _ in 0..16 {
            u.tick();
            assert_eq!(u.tx, expected);
            assert_eq!(u.status() & 4, 0); // TXEMPTY waits for the stop bit to end
        }
    }
    u.tick();
    assert_eq!(u.status() & 5, 5);
    assert!(u.tx);
}

#[test]
fn uart_firmware_preamble_rx_ready_and_read_clear() {
    let mut u = uart();
    assert_eq!(u.mode, 0x4e);
    assert_eq!(u.status(), 5);
    bit(&mut u, false);
    assert!(!u.irq());
    for i in 0..8 {
        bit(&mut u, 0xa5 & (1 << i) != 0);
    }
    assert!(!u.irq());
    bit(&mut u, true);
    assert!(u.irq());
    assert_eq!(u.read(), 0xa5);
    assert!(!u.irq());
    assert_eq!(u.status(), 5);
}

#[test]
fn uart_overrun_framing_error_reset_and_disabled_receiver() {
    let mut u = uart();
    frame(&mut u, 0x55, true);
    frame(&mut u, 0x66, false);
    assert_eq!(u.status() & 0x32, 0x32);
    assert_eq!(u.read(), 0x66); // MAME retains newest byte on overrun
    u.control(0x37).unwrap();
    assert_eq!(u.status() & 0x38, 0);
    u.control(0x31).unwrap();
    frame(&mut u, 0xff, true);
    assert!(!u.irq());
    assert_eq!(u.status() & 2, 0);
}

#[test]
fn uart_false_start_and_mid_character_roundtrip() {
    let mut u = uart();
    u.rx = false;
    u.tick();
    u.rx = true;
    for _ in 0..200 {
        u.tick();
    }
    assert!(!u.irq());
    bit(&mut u, false);
    bit(&mut u, true);
    bit(&mut u, false);
    let mut restored: uart::Uart = bincode::deserialize(&bincode::serialize(&u).unwrap()).unwrap();
    for level in [true, false, false, true, false, true, true] {
        bit(&mut u, level);
        bit(&mut restored, level);
    }
    assert_eq!(u, restored);
    assert!(u.irq());
    assert_eq!(u.read(), 0xa5);
}

#[test]
fn uart_internal_reset_is_not_hardware_reset_and_modes_fail_explicitly() {
    let mut u = uart();
    frame(&mut u, 7, true);
    u.control(0x40).unwrap();
    assert_eq!(u.status() & 2, 2);
    assert_eq!(u.control(0x5e), Err(uart::Error::UnsupportedUartMode(0x5e)));
    u.control(0x4e).unwrap();
    u.control(0x37).unwrap();
    assert_eq!(u.read(), 7);
    assert_eq!(
        u.control(0x3f),
        Err(uart::Error::UnsupportedUartCommand(0x3f))
    );
}

#[test]
fn transmit_has_holding_register_timed_bits_and_roundtrip() {
    let mut u = uart();
    u.write(0xa5).unwrap();
    assert_eq!(u.status() & 1, 0);
    u.tick(); // start bit enters shift register
    assert!(!u.tx);
    assert_eq!(u.status() & 5, 1);
    u.write(0x55).unwrap();
    assert_eq!(u.write(0x33), Err(uart::Error::TransmitFull));
    let mut restored: uart::Uart = bincode::deserialize(&bincode::serialize(&u).unwrap()).unwrap();
    for i in 1..=320 {
        u.tick();
        restored.tick();
        assert_eq!(u, restored);
        if i % 16 == 0 && i <= 128 {
            assert_eq!(u.tx, 0xa5 & (1 << (i / 16 - 1)) != 0);
        }
    }
    assert!(u.tx);
    assert_eq!(u.status() & 5, 5);
}

#[test]
fn cpu_programs_uart_and_slices_preserve_clock_debt_and_tx_edges() {
    let program = [
        0x3e, 0x4e, 0xd3, 0xf1, 0x3e, 0x37, 0xd3, 0xf1, 0x3e, 0x55, 0xd3, 0xf0, 0x76,
    ];
    let mut whole = board(&program);
    let mut sliced = board(&program);
    let mut a = vec![];
    let mut b = vec![];
    whole.run(3000, |e| a.push(e)).unwrap();
    for _ in 0..3000 {
        sliced.run(1, |e| b.push(e)).unwrap();
    }
    assert_eq!(a, b);
    assert_eq!(bytes(&whole), bytes(&sliced));
    assert!(a.len() >= 8);
    assert!(a.windows(2).all(|w| w[1].clock - w[0].clock == 128));
    let mut original = board(&program);
    original.run(501, |_| {}).unwrap();
    let mut restored = board(&program);
    restored
        .restore(&bincode::deserialize(&bytes(&original)).unwrap())
        .unwrap();
    a.clear();
    b.clear();
    original.run(2499, |e| a.push(e)).unwrap();
    restored.run(2499, |e| b.push(e)).unwrap();
    assert!(!a.is_empty());
    assert_eq!(a, b);
    assert_eq!(bytes(&original), bytes(&restored));
}

#[test]
fn im1_receives_live_uart_irq_and_read_deasserts_it() {
    let mut program = vec![0; 0x50];
    let init = [
        0x31, 0xfe, 0xff, 0xed, 0x56, 0x3e, 0x4e, 0xd3, 0xf1, 0x3e, 0x37, 0xd3, 0xf1, 0xfb, 0x76,
        0x18, 0xfd,
    ];
    program[..init.len()].copy_from_slice(&init);
    program[0x38..0x40].copy_from_slice(&[0xdb, 0xf0, 0x32, 0, 0x80, 0xfb, 0xed, 0x4d]);
    let mut b = board(&program);
    b.run(200, |_| {}).unwrap();
    for level in [
        false, true, false, true, false, false, true, false, true, true,
    ] {
        b.set_rx(level);
        b.run(128, |_| {}).unwrap();
    }
    b.run(200, |_| {}).unwrap();
    assert_eq!(b.cpu.io.read_byte(0x8000), 0xa5);
    assert!(!b.cpu.io.irq_pending());
    assert!(b.cpu_state().halted);
}

#[test]
fn fault_is_sticky_and_invalid_restore_is_atomic() {
    let mut b = board(&[0x3e, 0x5e, 0xd3, 0xf1, 0x32, 0, 0x80]);
    assert_eq!(b.run(100, |_| {}), Err(Error::UnsupportedUartMode(0x5e)));
    let before = bytes(&b);
    assert!(b.run(100, |_| {}).is_err());
    assert_eq!(before, bytes(&b));
    assert_eq!(b.cpu.io.read_byte(0x8000), 0);
    let mut invalid = b.snapshot();
    invalid.bus.ram.pop();
    assert_eq!(b.restore(&invalid), Err(Error::InvalidSnapshot));
    assert_eq!(before, bytes(&b));
    let saved = b.snapshot();
    b.reset();
    assert_eq!(b.cpu_state().pc, 0);
    b.restore(&saved).unwrap();
    assert_eq!(bytes(&b), before);
}

#[test]
fn combined_snapshot_during_receive_continues_identically() {
    let program = [0x3e, 0x4e, 0xd3, 0xf1, 0x3e, 0x37, 0xd3, 0xf1, 0x76];
    let mut a = board(&program);
    a.run(200, |_| {}).unwrap();
    for level in [false, true, false] {
        a.set_rx(level);
        a.run(128, |_| {}).unwrap();
    }
    a.set_rx(true);
    a.run(31, |_| {}).unwrap();
    let mut b = board(&program);
    b.restore(&bincode::deserialize(&bytes(&a)).unwrap())
        .unwrap();
    a.run(97, |_| {}).unwrap();
    b.run(97, |_| {}).unwrap();
    for level in [false, false, true, false, true, true] {
        for board in [&mut a, &mut b] {
            board.set_rx(level);
            board.run(128, |_| {}).unwrap();
        }
    }
    assert_eq!(bytes(&a), bytes(&b));
    assert!(a.cpu.io.irq_pending());
    assert_eq!(a.cpu.io.port_in(0xf0), 0xa5);
    assert_eq!(b.cpu.io.port_in(0xf0), 0xa5);
}

#[test]
fn soft_reset_retains_reference_playback_latches_and_ram() {
    let mut b = board(&[0x76]);
    b.cpu.io.write_byte(0x8123, 0x55);
    for (p, v) in [(0xe4, 10), (0xe7, 20), (0xe0, 2), (0xe4, 12), (0xe9, 2)] {
        b.cpu.io.port_out(p, v);
    }
    b.run(101, |_| {}).unwrap();
    let elapsed = b.cpu.io.elapsed;
    b.reset();
    let p = b.playback();
    assert_eq!(
        (p.start, p.end, p.loop_start, p.pan, p.bit_position),
        (10, 20, 12, 2, 80)
    );
    assert_eq!(
        (p.mode, p.volume, p.start_latch, p.end_latch),
        (0, 127, 0, 0)
    );
    assert_eq!(b.cpu.io.read_byte(0x8123), 0x55);
    assert_eq!(b.cpu.io.elapsed, elapsed);
    assert!(!b.cpu.io.irq_pending());
    assert!(b.tx());
}

fn connected_board() -> Board {
    // Set UART, then poll RX and append bytes to RAM (no IRQ dependence).
    let mut b = board(&[
        0x3e, 0x4e, 0xd3, 0xf1, 0x3e, 0x37, 0xd3, 0xf1, 0x21, 0, 0x80, 0xdb, 0xf1, 0xe6, 2, 0x28,
        0xfa, 0xdb, 0xf0, 0x77, 0x23, 0x18, 0xf4,
    ]);
    b.connect_sender();
    b.sender_control(0x4e);
    b.sender_control(0x37);
    b.run_sound_cycles(1000).unwrap();
    b
}

#[test]
fn filtered_sender_has_backpressure_and_preserves_order() {
    let mut b = connected_board();
    b.sender_write(0x55);
    assert_eq!(b.sender_status() & 1, 0);
    b.run_sound_cycles(100).unwrap();
    assert_eq!(b.sender_status() & 1, 1);
    b.sender_write(0xa5);
    b.run_sound_cycles(7000).unwrap();
    assert_eq!(b.diagnostics().0, 2);
    assert_eq!(b.diagnostics().1, 2);
    assert_eq!(b.cpu.io.read_byte(0x8000), 0x55);
    assert_eq!(b.cpu.io.read_byte(0x8001), 0xa5);
    assert_eq!(b.diagnostics().2 & 0x38, 0);
}

#[test]
fn linked_pair_fractional_clock_and_inflight_snapshot_continue_identically() {
    let mut whole = connected_board();
    let mut sliced = connected_board();
    whole.sender_write(0x39);
    sliced.sender_write(0x39);
    whole.run_sound_cycles(503).unwrap();
    for _ in 0..503 {
        sliced.run_sound_cycles(1).unwrap();
    }
    assert_eq!(bytes(&whole), bytes(&sliced));
    let mut restored = connected_board();
    restored
        .restore(&bincode::deserialize(&bytes(&whole)).unwrap())
        .unwrap();
    whole.run_sound_cycles(5000).unwrap();
    restored.run_sound_cycles(5000).unwrap();
    assert_eq!(bytes(&whole), bytes(&restored));
    assert_eq!(whole.cpu.io.read_byte(0x8000), 0x39);
}

#[test]
fn linked_sender_overflow_is_explicit_and_sticky() {
    let mut b = connected_board();
    b.sender_write(1);
    b.sender_write(2);
    assert_eq!(b.fault(), Some(Error::TransmitFull));
    let before = bytes(&b);
    assert_eq!(b.run_sound_cycles(100), Err(Error::TransmitFull));
    assert_eq!(bytes(&b), before);
}
