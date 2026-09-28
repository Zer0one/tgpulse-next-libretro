//! CPU-board state at an execution boundary, independent of host resources.
use super::*;

/// Internal, unversioned snapshot of the motherboard only. The enclosing
/// machine must restore matching I/O, sound, video and optional COMM states,
/// and enforce ROM identity and bounded decoding. No ROMs/config/host handles.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MotherboardState {
    main_cpu: V60,
    tgp_cpu: Mb86233,
    tgp_clock_remainder: u8,
    nvram: Vec<u8>,
    work_ram: Vec<u8>,
    tgp_data: Vec<u32>,
    copro_ram: Vec<u32>,
    copro_fifo_in: VecDeque<u32>,
    copro_fifo_out: VecDeque<u32>,
    v60_fifo_waiting: bool,
    v60_io_stall: bool,
    copro_stall: bool,
    inputs: Inputs,
    drive_cmd: u8,
    copro_io_ram_adr: [u32; 4],
    copro_sincos_base: u32,
    copro_inv_base: u32,
    copro_isqrt_base: u32,
    copro_atan_base: [u32; 4],
    copro_data_base: u32,
    bank_reg: u16,
    bank_base: u32,
    irq_status: u8,
    irq_mask: u8,
    last_irq: u8,
    timer_mode: u16,
    timer_period: [u16; 2],
    timer_remaining: [u32; 2],
    timer_latched: [u16; 2],
    frame_num: u64,
    copro_ram_addr: u16,
    copro_ram_latch: [u16; 2],
    copro_fifo_read_latch: u32,
    copro_fifo_write_latch: u32,
    fifo_events: VecDeque<(char, u32, usize, usize)>,
}

impl MotherboardState {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.nvram.len() != 0x10000 || self.work_ram.len() != 0x40000
            || self.tgp_data.len() != 0x400 || self.copro_ram.len() != 0x2000
            // Producers accept the seventeenth word, then HALT. Do not trim it.
            || self.copro_fifo_in.len() > COPRO_FIFO_DEPTH + 1
            || self.copro_fifo_out.len() > COPRO_FIFO_DEPTH + 1
            || self.fifo_events.len() > 128
        {
            return Err("invalid Model 1 motherboard memory/queue dimensions");
        }
        if self.tgp_clock_remainder > 1 || self.last_irq > 7
            || self.main_cpu.irq_vector > 7 || self.main_cpu.icount > 0
            // Positive TGP count is legitimate: a FIFO retry abandons the rest
            // of a <=64-main-clock quantum. The scheduler only carries debt.
            || self.tgp_cpu.icount > 160
            || self.tgp_cpu.vsm > 7
            || self.tgp_cpu.vsmr != (8u16 << self.tgp_cpu.vsm) - 1
            || !(0x0100_0000..=0x0170_0000).contains(&self.bank_base)
            || self.bank_base & 0xfffff != 0
            || self.timer_remaining.iter().zip(self.timer_period)
                .any(|(&remaining, period)| remaining > u32::from(period) * 0x800)
            || !self.main_cpu.trace.is_empty() || self.main_cpu.trace_cap != 0
        {
            return Err("invalid Model 1 motherboard CPU/timing state");
        }
        Ok(())
    }
}

impl Model1System {
    /// Capture after a run/bus call, never while the V60 has been temporarily
    /// moved out of the motherboard to execute against its bus. Temporary bus
    /// accounting is deliberately not serialized as an executable CPU context.
    pub fn snapshot_motherboard(&self) -> Result<MotherboardState, &'static str> {
        if self.v60_access_active || self.v60_wait_cycles != 0 {
            return Err("Model 1 motherboard is not at a stable execution boundary");
        }
        let mut main_cpu = self.main_cpu.clone();
        // Tracing policy belongs to the debugger. Match the CPU serde policy
        // for coverage in both direct and encoded snapshots, not just on decode.
        main_cpu.trace.clear();
        main_cpu.trace_cap = 0;
        main_cpu.trace_lo = 0;
        main_cpu.trace_hi = 0;
        main_cpu.op_count.fill(0);
        main_cpu.op_unimpl.fill(0);
        let mut tgp_cpu = self.tgp_cpu.clone();
        tgp_cpu.cov = Default::default();
        let state = MotherboardState {
            main_cpu,
            tgp_cpu,
            tgp_clock_remainder: self.tgp_clock_remainder,
            nvram: self.nvram.clone(),
            work_ram: self.work_ram.clone(),
            tgp_data: self.tgp_data.clone(),
            copro_ram: self.copro_ram.clone(),
            copro_fifo_in: self.copro_fifo_in.clone(),
            copro_fifo_out: self.copro_fifo_out.clone(),
            v60_fifo_waiting: self.v60_fifo_waiting,
            v60_io_stall: self.v60_io_stall,
            copro_stall: self.copro_stall,
            inputs: self.inputs,
            drive_cmd: self.drive_cmd,
            copro_io_ram_adr: self.copro_io_ram_adr,
            copro_sincos_base: self.copro_sincos_base,
            copro_inv_base: self.copro_inv_base,
            copro_isqrt_base: self.copro_isqrt_base,
            copro_atan_base: self.copro_atan_base,
            copro_data_base: self.copro_data_base,
            bank_reg: self.bank_reg,
            bank_base: self.bank_base,
            irq_status: self.irq_status,
            irq_mask: self.irq_mask,
            last_irq: self.last_irq,
            timer_mode: self.timer_mode,
            timer_period: self.timer_period,
            timer_remaining: self.timer_remaining,
            timer_latched: self.timer_latched,
            frame_num: self.frame_num,
            copro_ram_addr: self.copro_ram_addr,
            copro_ram_latch: self.copro_ram_latch,
            copro_fifo_read_latch: self.copro_fifo_read_latch,
            copro_fifo_write_latch: self.copro_fifo_write_latch,
            fifo_events: self.fifo_events.clone(),
        };
        state.validate()?;
        Ok(state)
    }

    /// Restore only this board, without reset, bus writes, IRQ synchronization
    /// or NVRAM persistence. Other devices and frontend preferences are untouched.
    /// Clear stale CPU trace/coverage, retaining the destination trace policy.
    pub fn restore_motherboard(&mut self, state: &MotherboardState) -> Result<(), &'static str> {
        self.validate_motherboard_restore(state)?;
        self.restore_motherboard_validated(state);
        Ok(())
    }

    pub(crate) fn validate_motherboard_restore(
        &self,
        state: &MotherboardState,
    ) -> Result<(), &'static str> {
        if self.v60_access_active || self.v60_wait_cycles != 0 {
            return Err("Model 1 motherboard is not at a stable execution boundary");
        }
        state.validate()
    }

    fn restore_motherboard_validated(&mut self, state: &MotherboardState) {
        let mut main_cpu = state.main_cpu.clone();
        main_cpu.trace_lo = self.main_cpu.trace_lo;
        main_cpu.trace_hi = self.main_cpu.trace_hi;
        main_cpu.trace_cap = self.main_cpu.trace_cap;
        main_cpu.op_count.fill(0);
        main_cpu.op_unimpl.fill(0);
        let mut tgp_cpu = state.tgp_cpu.clone();
        tgp_cpu.cov = Default::default();
        self.main_cpu = main_cpu;
        self.tgp_cpu = tgp_cpu;
        self.tgp_clock_remainder = state.tgp_clock_remainder;
        self.nvram.clone_from(&state.nvram);
        self.work_ram.clone_from(&state.work_ram);
        self.tgp_data.clone_from(&state.tgp_data);
        self.copro_ram.clone_from(&state.copro_ram);
        self.copro_fifo_in.clone_from(&state.copro_fifo_in);
        self.copro_fifo_out.clone_from(&state.copro_fifo_out);
        self.v60_fifo_waiting = state.v60_fifo_waiting;
        self.v60_io_stall = state.v60_io_stall;
        self.copro_stall = state.copro_stall;
        self.inputs = state.inputs;
        self.drive_cmd = state.drive_cmd;
        self.copro_io_ram_adr = state.copro_io_ram_adr;
        self.copro_sincos_base = state.copro_sincos_base;
        self.copro_inv_base = state.copro_inv_base;
        self.copro_isqrt_base = state.copro_isqrt_base;
        self.copro_atan_base = state.copro_atan_base;
        self.copro_data_base = state.copro_data_base;
        self.bank_reg = state.bank_reg;
        // This is a latch: some bank_reg writes intentionally leave it unchanged.
        self.bank_base = state.bank_base;
        self.irq_status = state.irq_status;
        self.irq_mask = state.irq_mask;
        self.last_irq = state.last_irq;
        self.timer_mode = state.timer_mode;
        self.timer_period = state.timer_period;
        self.timer_remaining = state.timer_remaining;
        self.timer_latched = state.timer_latched;
        self.frame_num = state.frame_num;
        self.copro_ram_addr = state.copro_ram_addr;
        self.copro_ram_latch = state.copro_ram_latch;
        self.copro_fifo_read_latch = state.copro_fifo_read_latch;
        self.copro_fifo_write_latch = state.copro_fifo_write_latch;
        self.fifo_events.clone_from(&state.fifo_events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roms(program: &[u8]) -> Model1Roms {
        Model1Roms {
            dsb: None,
            comm_board: false,
            ioboard_kind: crate::model1board::Kind::Original,
            nvram_default: vec![],
            maincpu: program.to_vec(),
            tgp: vec![],
            copro_tables: vec![0; 0x10000],
            polygons: vec![],
            copro_data: vec![0x1234; 0x10000],
            iocpu: vec![0x76],
            sndcpu: vec![],
            mpcm1: vec![],
            mpcm2: vec![],
            ioboard_config: vec![],
        }
    }
    fn machine(roms: &Model1Roms) -> Model1System {
        let mut s = Model1System::new(roms).unwrap();
        s.main_cpu.reg[v60::cpu::PC] = 0;
        s
    }
    fn encoded(s: &Model1System) -> Vec<u8> {
        bincode::serialize(&s.snapshot_motherboard().unwrap()).unwrap()
    }
    fn restored(s: &mut Model1System, roms: &Model1Roms) -> Model1System {
        let saved: MotherboardState = bincode::deserialize(&encoded(s)).unwrap();
        let mut other = machine(roms);
        other.restore_motherboard(&saved).unwrap();
        // Test harness only: peers must be at the same time for run_slice.
        // This is not the future atomic whole-machine restore transaction.
        other.ioboard.restore(&s.ioboard.snapshot()).unwrap();
        other
            .sound
            .restore_model1(&s.sound.snapshot_model1().unwrap())
            .unwrap();
        s.sound.samples.clear();
        assert_eq!(encoded(s), encoded(&other));
        other
    }
    fn same(a: &Model1System, b: &Model1System) {
        assert_eq!(encoded(a), encoded(b));
        assert_eq!(a.sound.samples, b.sound.samples);
        assert_eq!(
            bincode::serialize(&a.ioboard.snapshot()).unwrap(),
            bincode::serialize(&b.ioboard.snapshot()).unwrap()
        );
        assert_eq!(
            bincode::serialize(&a.sound.snapshot_model1().unwrap()).unwrap(),
            bincode::serialize(&b.sound.snapshot_model1().unwrap()).unwrap()
        );
    }

    #[test]
    fn v60_empty_fifo_retry_restores_without_duplicate_pop_or_destination_write() {
        let roms = roms(&[0x24, 0x20, 0xf3, 0, 0, 0xd8, 0, 0]); // IN.W fifo,R0; HALT
        let mut a = machine(&roms);
        a.main_cpu.reg[0] = 0xaabbccdd;
        a.run_slice(65).unwrap();
        assert!(a.v60_fifo_waiting);
        assert_eq!(a.main_cpu.pc(), 0);
        assert_eq!(a.main_cpu.reg[0], 0xaabbccdd);
        let mut b = restored(&mut a, &roms);
        for s in [&mut a, &mut b] {
            s.run_slice(99).unwrap(); // still blocked
            assert_eq!(s.main_cpu.reg[0], 0xaabbccdd);
            Mb86233Bus::write_data(s, 0x400, 0x12345678);
            s.run_slice(8).unwrap();
            assert_eq!(s.main_cpu.reg[0], 0x12345678);
            assert_eq!(s.main_cpu.pc(), 7);
            assert_eq!(s.fifo_events.iter().filter(|e| e.0 == 'R').count(), 1);
            assert!(s.copro_fifo_out.is_empty());
        }
        same(&a, &b);
    }

    #[test]
    fn tgp_retry_overflow_and_fractional_instruction_debt_continue_identically() {
        for mode in 0..3 {
            let mut roms = roms(&[0]); // V60 HALT
            let code = match mode {
                0 => 0x100u32,                                    // LAB command FIFO, empty => retry
                1 => (7 << 26) | (7 << 18) | (0x10 << 9) | 0x180, // MOV A,out FIFO
                _ => 8 << 21,                                     // two-cycle FML
            };
            let mut program = vec![code; 1024];
            if mode == 2 {
                program[0] = 0;
            } // odd instruction count -> debt
            roms.tgp = program.iter().flat_map(|v| v.to_le_bytes()).collect();
            let mut a = machine(&roms);
            a.tgp_cpu.b0 = 0x100;
            a.tgp_cpu.b1 = 0x400;
            a.tgp_cpu.i0 = 0;
            a.tgp_cpu.i1 = 0;
            a.tgp_cpu.a = 0x12345678;
            if mode == 2 {
                a.tgp_cpu.b0 = 0;
            }
            a.run_slice(1).unwrap();
            if mode != 2 {
                a.run_slice(64).unwrap();
            }
            assert_eq!(a.tgp_clock_remainder, 1);
            match mode {
                0 => {
                    assert_eq!(a.tgp_cpu.pc, 0);
                    assert!(a.tgp_cpu.icount > 0);
                }
                1 => assert_eq!(a.copro_fifo_out.len(), 17),
                _ => assert_eq!(a.tgp_cpu.icount, -1),
            }
            let mut b = restored(&mut a, &roms);
            for s in [&mut a, &mut b] {
                if mode == 0 {
                    s.copro_fifo_in.extend(1..=16);
                } else if mode == 1 {
                    // Actual V60 low-word pop, not a fake queue reset.
                    s.v60_access_active = true;
                    assert_eq!(s.copro_fifo_read(false), 0x5678);
                    s.v60_access_active = false;
                }
            }
            for cycles in [1, 3, 64, 127] {
                a.run_slice(cycles).unwrap();
                b.run_slice(cycles).unwrap();
                same(&a, &b);
            }
            if mode == 0 {
                assert!(a.copro_fifo_in.is_empty());
            }
            if mode == 1 {
                assert_eq!(a.copro_fifo_out.len(), 17);
            }
        }
    }

    #[test]
    fn latched_halves_bank_math_addresses_and_nvram_are_not_recomputed() {
        let roms = roms(&[0]);
        let mut a = machine(&roms);
        a.bank_w(0x51);
        a.bank_w(0x02); // register changes, mapping latch does not
        a.write_u16(0xd00000, 0x8007);
        a.write_u16(0xd20000, 0x5678); // low half only; not yet in RAM
        a.write_u16(0xd80000, 0xabcd); // low FIFO half, not yet enqueued
        a.copro_fifo_read_latch = 0xdeadbeef;
        for (address, value) in [
            (0, 0x40009),
            (8, u32::MAX),
            (0x20, 0x1234),
            (0x24, 1),
            (0x25, 2),
            (0x26, 3),
            (0x27, 4),
            (0x28, 0x3f800000),
            (0x2a, 0x40800000),
            (0x2e, 0x8000),
        ] {
            a.copro_io_write(address, value);
        }
        a.nvram[23] = 0x45;
        a.work_ram[32] = 0x76;
        a.tgp_data[43] = 0x87654321;
        a.inputs.in0 = 0x53;
        a.drive_cmd = 0x12;
        a.frame_num = 123;
        // Pending bus retry flags are copied, not consumed during restoration.
        a.v60_io_stall = true;
        a.copro_stall = true;
        let mut b = restored(&mut a, &roms);
        assert_eq!((b.bank_reg, b.bank_base), (2, 0x01500000));
        assert_eq!(b.nvram[23], 0x45);
        assert_eq!(b.inputs.in0, 0x53);
        assert_eq!((b.drive_cmd, b.frame_num), (0x12, 123));
        for s in [&mut a, &mut b] {
            assert!(Bus::take_io_stall(s));
            assert!(Mb86233Bus::take_stall(s));
            assert_eq!(s.copro_fifo_read(true), 0xdead);
            s.write_u16(0xd20002, 0x1234);
            assert_eq!(s.copro_ram[7], 0x12345678);
            assert_eq!(s.copro_ram_addr, 0x8008);
            s.write_u16(0xd80002, 0x9876);
            assert_eq!(s.copro_fifo_in.front(), Some(&0x9876abcd));
            s.copro_io_write(1, 0xcafe);
            assert_eq!(s.copro_io_ram_adr[0], 0x4000d);
            assert_eq!(s.copro_ram[9], 0xcafe);
            assert_eq!(s.copro_io_read(0x8000), 0x1234);
        }
        same(&a, &b);
    }

    #[test]
    fn timer_expiry_halt_irq_acknowledgement_and_cpu_debt_survive_restore() {
        let roms = roms(&[0xcd, 0xcd, 0]); // NOP, NOP, HALT
        let mut a = machine(&roms);
        a.main_cpu.reg[v60::cpu::SP] = 0x53e000;
        a.main_cpu.reg[36] = 0x53f000;
        a.main_cpu.reg[v60::cpu::SBR] = 0x500000;
        a.write_u32(0x500100, 0x501000); // IRQ0 -> RAM HALT
        a.main_cpu.reg[v60::cpu::PSW] = 1 << 18;
        a.irq_mask = 0xfe;
        a.set_timer_period(0, 1);
        a.timer_remaining[0] = 10;
        a.timer_latched[1] = 7; // stopped timer retains last observed count
        a.run_slice(1).unwrap();
        assert_eq!(a.main_cpu.icount, -7);
        let mut b = restored(&mut a, &roms);
        for cycles in [8, 1, 8, 64, 4096] {
            a.run_slice(cycles).unwrap();
            b.run_slice(cycles).unwrap();
            same(&a, &b);
        }
        assert!(a.main_cpu.halted);
        assert_eq!(a.main_cpu.irq_taken, 1);
        assert_eq!(a.last_irq, 0);
        assert_eq!(a.timer_r(1), 7);
        // Save with an already-pending IRQ while halted/IE disabled.
        let mut c = restored(&mut a, &roms);
        assert!(c.irq_status & 1 != 0);
        for s in [&mut a, &mut c] {
            s.irq_control_w(0x20);
            assert_eq!(s.irq_status, 0);
            s.run_slice(17).unwrap();
        }
        same(&a, &c);
    }

    #[test]
    fn invalid_or_in_instruction_state_is_rejected_atomically_and_debug_policy_is_local() {
        let roms = roms(&[0xcd, 0]);
        let mut s = machine(&roms);
        s.run_slice(1).unwrap();
        s.main_cpu.trace_cap = 64;
        s.main_cpu.trace_lo = 5;
        s.main_cpu.trace_hi = 100;
        s.main_cpu.trace.push((5, 0xcd));
        s.main_cpu.op_count[0xcd] = 123;
        s.tgp_cpu.cov.stall_retries = 456;
        let saved = s.snapshot_motherboard().unwrap();
        let before = encoded(&s);
        for case in 0..13 {
            let mut bad = saved.clone();
            match case {
                0 => {
                    bad.nvram.pop();
                }
                1 => {
                    bad.work_ram.pop();
                }
                2 => {
                    bad.tgp_data.pop();
                }
                3 => {
                    bad.copro_ram.pop();
                }
                4 => bad.copro_fifo_in.resize(18, 0),
                5 => bad.copro_fifo_out.resize(18, 0),
                6 => bad.tgp_clock_remainder = 2,
                7 => bad.last_irq = 8,
                8 => bad.timer_remaining[0] = u32::MAX,
                9 => bad.bank_base = 3,
                10 => bad.tgp_cpu.vsm = 8,
                11 => bad.main_cpu.icount = 1,
                _ => bad.fifo_events.resize(129, ('W', 0, 0, 0)),
            }
            assert!(s.restore_motherboard(&bad).is_err());
            assert_eq!(encoded(&s), before);
            assert_eq!(s.main_cpu.trace, [(5, 0xcd)]);
            assert_eq!(s.main_cpu.op_count[0xcd], 123);
        }
        s.v60_access_active = true;
        assert!(s.snapshot_motherboard().is_err());
        assert!(s.restore_motherboard(&saved).is_err());
        s.v60_access_active = false;
        s.v60_wait_cycles = 1;
        assert!(s.snapshot_motherboard().is_err());
        assert!(s.restore_motherboard(&saved).is_err());
        s.v60_wait_cycles = 0;
        s.restore_motherboard(&saved).unwrap();
        assert_eq!(encoded(&s), before);
        assert_eq!(
            (
                s.main_cpu.trace_cap,
                s.main_cpu.trace_lo,
                s.main_cpu.trace_hi
            ),
            (64, 5, 100)
        );
        assert!(s.main_cpu.trace.is_empty());
        assert_eq!(s.main_cpu.op_count[0xcd], 0);
        assert_eq!(s.tgp_cpu.cov.stall_retries, 0);
        assert_eq!(s.maincpu_rom, roms.maincpu);
    }
}
