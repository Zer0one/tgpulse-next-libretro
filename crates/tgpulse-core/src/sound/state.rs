//! In-memory Model 1 audio-board state, separate from ROMs and host output.
use super::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SoundState {
    version: u32,
    cpu: m68000::state::State,
    ram: Vec<u8>,
    pcm: [crate::multipcm::State; 2],
    netmerc: Option<netmerc::Recovery>,
    fm: FmPathState,
    serial: SerialState,
    dsb: Option<DsbPathState>,
    rx: std::collections::VecDeque<u8>,
    tx: Option<u8>,
    rx_count: u64,
    rx_read_count: u64,
    ym_writes: u64,
    remainder: i64,
    main_fraction: u64,
    irq_pending: bool,
    #[serde(with = "serde_big_array::BigArray")]
    exception_counts: [u64; 256],
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(dsb: bool) -> SoundSystem {
        let mut s = super::super::mute_tests::sounding_board(0);
        if dsb {
            let mut firmware = vec![0; crate::dsbz80::FIRMWARE_SIZE];
            // Initialize receiver, then HALT; preserve its in-flight UART state.
            firmware[..9].copy_from_slice(&[0x3e, 0x4e, 0xd3, 0xf1, 0x3e, 0x37, 0xd3, 0xf1, 0x76]);
            s.board.dsb = Some(crate::dsbz80::Board::new(&firmware).unwrap());
        }
        s.enable_model1_serial();
        s.board.uart_control(0x4e);
        s.board.uart_control(0x37);
        s.board.write8(0xc20003, 0x4e);
        s.board.write8(0xc20003, 0x37);
        s.send(0x96);
        s.board.write8(0xc20001, 0x35);
        // DAC output and live FM timers in addition to the active PCM voice.
        for (reg, value) in [(0x2b, 0x80), (0x2a, 200)] {
            s.board.write8(0xd00001, reg);
            s.board.write8(0xd00003, value);
        }
        s
    }
    fn encoded(s: &SoundSystem) -> Vec<u8> {
        bincode::serialize(&s.snapshot_model1().unwrap()).unwrap()
    }

    #[test]
    fn netmerc_recovery_uses_bus_and_native_clock_with_output_only_mutes() {
        let mut a = fixture(false);
        a.enable_netmerc_recovery();
        for base in [0xc40001, 0xc60001] {
            for (reg, value) in [(0, 0), (1, 83), (2, 0), (3, 0x10), (5, 1), (4, 0x80)] {
                a.board.write8(base + 4, reg);
                a.board.write8(base, value);
            }
        }
        a.run(40003, 16_000_000);
        let saved = a.snapshot_model1().unwrap();
        let mut b = fixture(false);
        assert!(b.restore_model1(&saved).is_err(), "audio mode must match");
        b.enable_netmerc_recovery();
        b.restore_model1(&saved).unwrap();
        b.set_mutes(AudioMutes {
            multipcm1: true,
            multipcm2: true,
            ym3438: true,
            ..Default::default()
        });
        a.samples.clear();
        for cycles in [1, 127, 8119, 40003] {
            a.run(cycles, 16_000_000);
            for _ in 0..cycles {
                b.run(1, 16_000_000);
            }
            assert_eq!(encoded(&a), encoded(&b));
        }
        assert_eq!(a.samples.len(), b.samples.len());
        assert!(a.samples.iter().any(|&s| s != (0, 0)));
        assert!(b.samples.iter().all(|&s| s == (0, 0)));
        b.set_mutes(AudioMutes::default());
        a.samples.clear();
        b.samples.clear();
        a.run(8119, 16_000_000);
        b.run(8119, 16_000_000);
        assert_eq!(a.samples, b.samples);
    }

    #[test]
    fn board_round_trip_preserves_audio_serial_cpu_and_fractional_debt() {
        for dsb in [false, true] {
            let mut a = fixture(dsb);
            a.run(1237, 16_000_000);
            a.cpu.exception(Vector::Level2Interrupt.into()); // masked pending CPU state
            a.board.ram[17] = 0xc5;
            assert!(!a.board.uart_rx_ready(), "snapshot during transmission");
            assert_ne!(a.main_fraction, 0);
            let state: SoundState = bincode::deserialize(&encoded(&a)).unwrap();
            let mut b = fixture(dsb);
            b.run(6000, 16_000_000); // discard a different timeline
            assert!(!b.samples.is_empty());
            b.restore_model1(&state).unwrap();
            assert!(b.samples.is_empty());
            assert_eq!(encoded(&a), encoded(&b));
            a.samples.clear();
            for cycles in [1, 3, 64, 127, 8119, 40003] {
                a.run(cycles, 16_000_000);
                b.run(cycles, 16_000_000);
                assert_eq!(a.samples, b.samples);
                assert_eq!(encoded(&a), encoded(&b));
            }
            assert!(a.samples.iter().any(|s| *s != (0, 0)));
            assert_eq!(a.board.main_uart_read(), 0x35);
            assert_eq!(b.board.main_uart_read(), 0x35);
            assert_eq!(a.board.read8(0xc20001), 0x96);
            assert_eq!(b.board.read8(0xc20001), 0x96);
            assert_eq!(a.serial_fault(), None);
            assert_eq!(a.dsb_fault(), None);
        }
    }

    #[test]
    fn stopped_board_restore_preserves_preferences_not_stale_host_audio() {
        let mut a = fixture(false);
        a.cpu.stop = true;
        a.run(1237, 16_000_000);
        let saved = a.snapshot_model1().unwrap();
        let mut b = fixture(false);
        b.set_mutes(AudioMutes {
            multipcm1: true,
            ..Default::default()
        });
        let mut gains = AudioGains::default();
        gains.multipcm1 = 21;
        b.set_gains(gains);
        b.samples.push_back((123, 456));
        b.restore_model1(&saved).unwrap();
        assert!(b.cpu.stop && b.muted[0]);
        assert_eq!(b.gains.multipcm1, 21);
        assert!(b.samples.is_empty());
        a.samples.clear();
        a.set_mutes(AudioMutes {
            multipcm1: true,
            ..Default::default()
        });
        a.set_gains(gains);
        a.run(5003, 16_000_000);
        for _ in 0..5003 {
            b.run(1, 16_000_000);
        }
        assert_eq!(a.samples, b.samples);
        assert_eq!(encoded(&a), encoded(&b));
    }

    #[test]
    fn rejected_board_state_is_atomic_and_checks_board_variant() {
        let mut s = fixture(false);
        s.run(1237, 16_000_000);
        let saved = s.snapshot_model1().unwrap();
        let before = encoded(&s);
        let samples = s.samples.clone();
        for field in 0..6 {
            let mut bad = saved.clone();
            match field {
                0 => bad.version += 1,
                1 => bad.ram.pop().map(|_| ()).unwrap(),
                2 => bad.cpu.pending = vec![26, 26],
                3 => bad.main_fraction = 16_000_000,
                4 => bad.dsb = fixture(true).snapshot_dsb_path(),
                _ => bad.pcm[1] = MultiPcm::new(vec![], 8_000_000.0).snapshot(),
            }
            assert!(s.restore_model1(&bad).is_err());
            assert_eq!(encoded(&s), before);
            assert_eq!(s.samples, samples);
        }
        let mut linked = fixture(true);
        assert!(linked.restore_model1(&saved).is_err());
        linked.run(1237, 16_000_000);
        let before = encoded(&linked);
        let samples = linked.samples.clone();
        let mut bad = linked.snapshot_model1().unwrap();
        bad.dsb.as_mut().unwrap().conversion = dsb::Conversion::default();
        assert!(
            linked.restore_model1(&bad).is_err(),
            "reject inconsistent DSB time"
        );
        assert_eq!(encoded(&linked), before);
        assert_eq!(linked.samples, samples);
        assert!(SoundSystem::new(vec![], vec![], vec![])
            .snapshot_model1()
            .is_err());
    }
}

impl SoundSystem {
    /// Capture between run calls. Caller owns matching ROM identity/CPU clock.
    /// Gains/mutes and the already produced host sample queue are not hardware.
    pub fn snapshot_model1(&self) -> Result<SoundState, &'static str> {
        let serial = self
            .board
            .serial
            .clone()
            .ok_or("not a Model 1 sound board")?;
        Ok(SoundState {
            version: 2,
            cpu: self.cpu.snapshot(),
            ram: self.board.ram.clone(),
            pcm: [self.board.pcm[0].snapshot(), self.board.pcm[1].snapshot()],
            netmerc: self.board.netmerc.clone(),
            fm: self.snapshot_fm_path(),
            serial,
            dsb: self.snapshot_dsb_path(),
            rx: self.board.rx.clone(),
            tx: self.board.tx,
            rx_count: self.board.rx_count,
            rx_read_count: self.board.rx_read_count,
            ym_writes: self.board.ym_writes,
            remainder: self.remainder,
            main_fraction: self.main_fraction,
            irq_pending: self.irq_pending,
            exception_counts: self.exception_counts,
        })
    }

    /// Validate all fallible components before committing. This neither writes
    /// NVRAM nor replays bus writes; ROMs and frontend preferences stay owned by
    /// the current machine. Serialized bytes still need bounded outer decoding.
    pub fn restore_model1(&mut self, state: &SoundState) -> Result<(), &'static str> {
        if state.version != 2
            || self.board.serial.is_none()
            || !state.serial.valid()
            || state.ram.len() != SND_RAM_SIZE
            || state.rx.len() > 8
            || state.main_fraction >= 16_000_000
            || state.dsb.is_some() != self.board.dsb.is_some()
            || state.netmerc.is_some() != self.board.netmerc.is_some()
            || state.netmerc.as_ref().is_some_and(|r| !r.valid())
        {
            return Err("invalid Model 1 sound state");
        }
        let mut cpu = M68000::<Mc68000>::new_no_reset();
        cpu.restore(&state.cpu)?;
        let mut fm = FmPath::new();
        fm.restore(&state.fm)?;
        for (chip, saved) in self.board.pcm.iter().zip(&state.pcm) {
            chip.validate_state(saved)?;
        }
        // DSB restore itself is atomic and is the last fallible operation.
        // No preceding validation changes the live board or its output queue.
        if let Some(dsb) = &state.dsb {
            self.restore_dsb_path(dsb)
                .map_err(|_| "invalid DSB sound state")?;
        }
        self.cpu = cpu;
        self.board.ram.clone_from(&state.ram);
        for (chip, saved) in self.board.pcm.iter_mut().zip(&state.pcm) {
            chip.restore(saved).expect("validated PCM state");
        }
        self.board.ym = fm;
        self.board.netmerc.clone_from(&state.netmerc);
        self.board.serial = Some(state.serial.clone());
        self.board.rx.clone_from(&state.rx);
        self.board.tx = state.tx;
        self.board.rx_count = state.rx_count;
        self.board.rx_read_count = state.rx_read_count;
        self.board.ym_writes = state.ym_writes;
        self.remainder = state.remainder;
        self.main_fraction = state.main_fraction;
        self.irq_pending = state.irq_pending;
        self.exception_counts = state.exception_counts;
        self.samples.clear();
        Ok(())
    }
}
