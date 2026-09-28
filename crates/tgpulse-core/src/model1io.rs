//! The Model 1 I/O board, 837-8950-01.
//!
//! A Z80 with its own firmware, a Sega 315-5338A I/O chip, a 93C45 serial
//! EEPROM and an MSM6253 ADC, sitting between the main board and the cabinet.
//! The V60 never touches a control directly: it leaves a command in the shared
//! dual-port RAM and the Z80 fills the rest of that RAM in with the state of
//! the panel.
//!
//! It matters for more than inputs. The operator settings a game keeps -- the
//! country, which is what selects the language -- live in the 93C45 on this
//! board, and the only thing that can write them is this Z80. A faked board
//! can poll inputs convincingly and still leave a game unable to remember that
//! it was set to English, because the commit never happens.
//!
//!

use z80::{Z80_io, Z80};

use crate::config::Inputs;
use crate::eeprom93c46::Eeprom93c46;
use crate::msm6253::Adc;
use crate::sega3155338::{Chip5338, WriteEffect};

/// Z80 clock: the board's 32 MHz crystal divided by eight.
pub const Z80_HZ: u32 = 4_000_000;

/// The dual-port RAM shared with the main board (an MB8421).
pub const DPRAM_SIZE: usize = 0x800;

/// Everything the Z80 can reach.
struct Board {
    rom: Vec<u8>,
    /// MB8464, 8KB at 0x4000.
    ram: [u8; 0x2000],
    io: Chip5338,
    adc: Adc,

    /// Shared with the V60. The board lives here rather than in the main
    /// system because the dual-port RAM is physically between the two.
    dpram: Vec<u8>,

    /// Cabinet state, refreshed by the main system each frame.
    inputs: Inputs,
    eeprom: Eeprom93c46,

    /// Last value written to port E, which drives the force-feedback board.
    drive_cmd: u8,
    /// Last value written to port F: lamps and the coin counter.
    outputs: u8,
    /// Port A bit 0 swaps the panel for the second set of controls on a twin
    /// cabinet, and swaps the DIP switches in for the digital inputs.
    secondary_controls: bool,
}

pub struct IoBoard {
    cpu: Z80<Board>,
    /// Z80 cycles owed but not yet run, so a fractional slice is not lost.
    cycle_debt: i64,
}

/// Mutable board state captured between run calls. Firmware is supplied by the
/// owner and must match on restore; this is not a versioned machine/file format.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardState {
    cpu: z80::CpuState,
    #[serde(with = "serde_big_array::BigArray")]
    ram: [u8; 0x2000],
    dpram: Vec<u8>,
    io: Chip5338,
    adc: Adc,
    inputs: Inputs,
    eeprom: Eeprom93c46,
    drive_cmd: u8,
    outputs: u8,
    secondary_controls: bool,
    cycle_debt: i64,
}

impl IoBoard {
    pub fn snapshot(&self) -> BoardState {
        let bus = &self.cpu.io;
        BoardState {
            cpu: self.cpu.snapshot(),
            ram: bus.ram,
            dpram: bus.dpram.clone(),
            io: bus.io.clone(),
            adc: bus.adc.clone(),
            inputs: bus.inputs,
            eeprom: bus.eeprom.clone(),
            drive_cmd: bus.drive_cmd,
            outputs: bus.outputs,
            secondary_controls: bus.secondary_controls,
            cycle_debt: self.cycle_debt,
        }
    }

    /// Reject malformed state before mutation. Copy latches directly: replaying
    /// port writes could clock the EEPROM or start a new DPRAM transfer. EEPROM
    /// contents/dirty state are restored in memory only; no persistence callback.
    pub fn restore(&mut self, state: &BoardState) -> Result<(), &'static str> {
        self.validate_state(state)?;
        self.restore_validated(state);
        Ok(())
    }

    pub(crate) fn validate_state(&self, state: &BoardState) -> Result<(), &'static str> {
        if !state.cpu.is_valid()
            || state.dpram.len() != DPRAM_SIZE
            || !(-(u32::MAX as i64)..=0).contains(&state.cycle_debt)
        {
            return Err("invalid original Model 1 I/O board snapshot");
        }
        Ok(())
    }

    fn restore_validated(&mut self, state: &BoardState) {
        assert!(self.cpu.restore(&state.cpu));
        let bus = &mut self.cpu.io;
        bus.ram = state.ram;
        bus.dpram.clone_from(&state.dpram);
        bus.io = state.io.clone();
        bus.adc = state.adc.clone();
        bus.inputs = state.inputs;
        bus.eeprom = state.eeprom.clone();
        bus.drive_cmd = state.drive_cmd;
        bus.outputs = state.outputs;
        bus.secondary_controls = state.secondary_controls;
        self.cycle_debt = state.cycle_debt;
    }

    /// Builds the board. Without firmware there is no board: the caller is
    /// expected to have loaded the romset's `iocpu` region.
    pub fn new(firmware: &[u8], eeprom: Eeprom93c46) -> Self {
        let mut rom = firmware.to_vec();
        // Only the bottom 16KB is mapped; the EPROM is larger than the window.
        rom.resize(0x4000, 0xff);

        let mut cpu = Z80::new(Board {
            rom,
            ram: [0; 0x2000],
            io: Chip5338::default(),
            adc: Adc::default(),
            dpram: vec![0; DPRAM_SIZE],
            inputs: Inputs::default(),
            eeprom,
            drive_cmd: 0xff,
            outputs: 0,
            secondary_controls: false,
        });
        cpu.reset();

        Self { cpu, cycle_debt: 0 }
    }

    /// Runs the board for `cycles` of its own clock.
    pub fn run(&mut self, cycles: i64) {
        self.cycle_debt += cycles;
        while self.cycle_debt > 0 {
            self.cycle_debt -= self.cpu.step() as i64;
        }
    }

    /// Publishes the panel state the firmware will read on its next poll.
    pub fn set_inputs(&mut self, inputs: Inputs) {
        self.cpu.io.inputs = inputs;
    }

    /// The force command the game last sent to the drive board.
    pub fn drive_cmd(&self) -> u8 {
        self.cpu.io.drive_cmd
    }

    pub fn dpram(&self) -> &[u8] {
        &self.cpu.io.dpram
    }

    pub fn dpram_mut(&mut self) -> &mut Vec<u8> {
        &mut self.cpu.io.dpram
    }

    pub fn eeprom(&self) -> &Eeprom93c46 {
        &self.cpu.io.eeprom
    }

    pub fn eeprom_mut(&mut self) -> &mut Eeprom93c46 {
        &mut self.cpu.io.eeprom
    }
}

impl Z80_io for Board {
    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3fff => self.rom[addr as usize],
            0x4000..=0x5fff => self.ram[(addr - 0x4000) as usize],
            0x8000..=0x800f => self.io.read(
                (addr & 0x0f) as u8,
                |port| self.input_port(port),
                |address| {
                    self.dpram
                        .get(address as usize & (DPRAM_SIZE - 1))
                        .copied()
                        .unwrap_or(0xff)
                },
            ),
            0xc000..=0xc003 => self.adc.shift_out(),
            _ => 0xff,
        }
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000..=0x5fff => self.ram[(addr - 0x4000) as usize] = value,
            0x8000..=0x800f => {
                let reg = (addr & 0x0f) as u8;
                self.io_write(reg, value);
            }
            // Writing picks a channel and latches its reading in one action.
            0xc000..=0xc003 => {
                let value = self.analog((addr & 3) as usize);
                self.adc.latch(value);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(board: &IoBoard) -> Vec<u8> {
        bincode::serialize(&board.snapshot()).unwrap()
    }

    fn restored(board: &IoBoard, firmware: &[u8]) -> IoBoard {
        let saved = bincode::deserialize(&encoded(board)).unwrap();
        let mut other = IoBoard::new(firmware, Eeprom93c46::new());
        other.restore(&saved).unwrap();
        assert_eq!(encoded(board), encoded(&other));
        other
    }

    #[test]
    fn snapshot_preserves_cpu_bus_and_instruction_debt_across_run_partitions() {
        let program = [
            0x21, 0, 0x40, // HL = RAM
            0x34, // INC (HL)
            0x3a, 1, 0x80, // IN0
            0x32, 1, 0x40, // -> RAM
            0x32, 4, 0x80, // -> drive
            0xc3, 3, 0, // repeat
        ];
        let mut first = IoBoard::new(&program, Eeprom93c46::new());
        first.set_inputs(Inputs {
            in0: 0xa5,
            ..Inputs::default()
        });
        first.cpu.io.write_byte(0x8008, 2);
        first.dpram_mut()[0x21] = 0x92;
        first.run(25);
        assert!(first.cycle_debt < 0);
        let mut second = restored(&first, &program);
        first.run(10_003);
        for _ in 0..10_003 {
            second.run(1);
        }
        assert_eq!(encoded(&first), encoded(&second));
        assert_eq!(second.cpu.io.ram[1], 0xa5);
        assert_eq!(second.drive_cmd(), 0xa5);
        assert_eq!(second.dpram()[0x21], 0x92);
        // Restoring the device does not replace the owner's immutable firmware.
        let mut different = IoBoard::new(&[0xc9], Eeprom93c46::new());
        different.restore(&first.snapshot()).unwrap();
        assert_eq!(different.cpu.io.read_byte(0), 0xc9);
    }

    #[test]
    fn snapshot_resumes_adc_eeprom_and_pending_host_transfer_without_port_replay() {
        let mut first = IoBoard::new(&[0x76], Eeprom93c46::new());
        first.set_inputs(Inputs {
            steer: 0xb6,
            ..Inputs::default()
        });
        first.cpu.io.write_byte(0xc000, 0);
        for expected in [1, 0, 1] {
            assert_eq!(first.cpu.io.read_byte(0xc000), expected);
        }
        first.eeprom_mut().data[7] = 0xa635;
        first.cpu.io.write_byte(0x8008, 0x40);
        eeprom_bits(&mut first.cpu.io, 0x187, 9);
        for _ in 0..5 {
            eeprom_bits(&mut first.cpu.io, 0, 1);
        }
        // Stage a host address and data byte, but do not issue the transfer yet.
        for (reg, value) in [(10, 0x23), (9, 0), (10, 0xf9), (9, 1), (10, 0x5e)] {
            first.cpu.io.write_byte(0x8000 + reg, value);
        }
        first.dpram_mut()[0x123] = 0xa9;
        first.run(1); // halted CPU with instruction overshoot
        let mut second = restored(&first, &[0x76]);
        for expected in [1, 0, 1, 1, 0, 0] {
            assert_eq!(first.cpu.io.read_byte(0xc000), expected);
            assert_eq!(second.cpu.io.read_byte(0xc000), expected);
        }
        for bit in (0..11).rev() {
            for b in [&mut first, &mut second] {
                eeprom_bits(&mut b.cpu.io, 0, 1);
                assert_eq!(
                    b.cpu.io.read_byte(0x8006) >> 7,
                    ((0xa635u16 >> bit) & 1) as u8
                );
            }
        }
        for b in [&mut first, &mut second] {
            assert!(!b.eeprom().dirty);
            assert_eq!(b.dpram()[0x123], 0xa9);
            b.cpu.io.write_byte(0x8009, 7);
            assert_eq!(b.dpram()[0x123], 0x5e);
            b.run(133);
        }
        assert_eq!(encoded(&first), encoded(&second));
    }

    #[test]
    fn snapshot_resumes_partial_eeprom_command_and_write_in_memory() {
        let mut first = IoBoard::new(&[0x76], Eeprom93c46::new());
        eeprom_bits(&mut first.cpu.io, 0x130, 9); // unlock
        first.cpu.io.write_byte(0x8000, 0);
        eeprom_bits(&mut first.cpu.io, 0x147 >> 4, 5); // partial WRITE command
        let mut second = restored(&first, &[0x76]);
        for b in [&mut first, &mut second] {
            eeprom_bits(&mut b.cpu.io, 0x147 & 0xf, 4);
            eeprom_bits(&mut b.cpu.io, 0xa635 >> 9, 7); // partial data
            assert_eq!(b.eeprom().data[7], 0xffff);
            assert!(!b.eeprom().dirty);
        }
        assert_eq!(encoded(&first), encoded(&second));
        let saved = first.snapshot();
        second.run(100); // discard a different CPU timeline on restore
        second.eeprom_mut().data[7] = 0;
        second.eeprom_mut().dirty = true;
        second.restore(&saved).unwrap();
        assert_eq!(encoded(&first), encoded(&second));
        for b in [&mut first, &mut second] {
            eeprom_bits(&mut b.cpu.io, 0xa635 & 0x1ff, 9);
            assert_eq!(b.eeprom().data[7], 0xa635);
            assert!(b.eeprom().dirty);
        }
        assert_eq!(encoded(&first), encoded(&second));
    }

    #[test]
    fn malformed_snapshots_do_not_change_cpu_bus_or_outputs() {
        let mut board = IoBoard::new(&[0x76], Eeprom93c46::new());
        board.run(1);
        board.cpu.io.write_byte(0x8004, 0x12);
        board.cpu.io.write_byte(0x8005, 0x34);
        board.cpu.io.write_byte(0x8000, 1);
        board.dpram_mut()[0x21] = 0x92;
        let before = encoded(&board);
        for case in 0..5 {
            let mut saved = board.snapshot();
            match case {
                0 => saved.cpu.interrupt_mode = 3,
                1 => {
                    saved.dpram.pop();
                }
                2 => saved.dpram.push(0),
                3 => saved.cycle_debt = 1,
                _ => saved.cycle_debt = -(u32::MAX as i64) - 1,
            }
            assert!(board.restore(&saved).is_err());
            assert_eq!(encoded(&board), before);
        }
        let second = restored(&board, &[0x76]);
        assert_eq!(second.drive_cmd(), 0x12);
        assert_eq!(second.cpu.io.outputs, 0x34);
        assert!(second.cpu.io.secondary_controls);
    }

    #[test]
    fn shared_z80_preserves_board_io_and_fractional_run_budget() {
        // Execute real memory-mapped I/O: read IN0, store it in RAM, latch
        // drive/lamp outputs, then loop. No external IRQ source on this board.
        let program = [
            0x3e, 0x02, 0x32, 0x08, 0x80, // port B input
            0x3a, 0x01, 0x80, 0x32, 0x00, 0x40, // IN0 -> RAM
            0x3e, 0x12, 0x32, 0x04, 0x80, // drive
            0x3e, 0x34, 0x32, 0x05, 0x80, // lamps
            0xc3, 0x05, 0x00, // repeat input/output loop
        ];
        let make = || {
            let mut board = IoBoard::new(&program, Eeprom93c46::new());
            board.set_inputs(Inputs {
                in0: 0xa5,
                ..Inputs::default()
            });
            board
        };
        let mut whole = make();
        let mut sliced = make();
        whole.run(100_003);
        for _ in 0..100_003 {
            sliced.run(1);
        }
        assert_eq!(whole.cpu.snapshot(), sliced.cpu.snapshot());
        assert_eq!(whole.cycle_debt, sliced.cycle_debt);
        assert_eq!(whole.cpu.io.ram, sliced.cpu.io.ram);
        assert_eq!(whole.cpu.io.ram[0], 0xa5);
        for board in [&whole, &sliced] {
            assert_eq!(board.drive_cmd(), 0x12);
            assert_eq!(board.cpu.io.outputs, 0x34);
            assert!(board.cycle_debt <= 0);
        }
        // Constructing another board does not reset the running instance.
        let before = whole.cpu.snapshot();
        let fresh = make();
        assert_eq!(fresh.cpu.pc, 0);
        assert!(!fresh.cpu.halted);
        assert!(!fresh.cpu.iff1);
        assert_eq!(fresh.drive_cmd(), 0xff);
        assert_eq!(whole.cpu.snapshot(), before);
    }

    #[test]
    fn shared_z80_halt_consumes_budget_without_replaying_io() {
        let mut board = IoBoard::new(&[0x3e, 0x56, 0x32, 0x04, 0x80, 0x76], Eeprom93c46::new());
        board.run(100);
        assert!(board.cpu.halted);
        assert_eq!(board.drive_cmd(), 0x56);
        let pc = board.cpu.pc;
        board.cpu.io.drive_cmd = 0x78;
        board.run(101);
        assert_eq!(board.cpu.pc, pc);
        assert_eq!(board.drive_cmd(), 0x78);
        assert!(board.cycle_debt <= 0);
    }

    #[test]
    fn first_generation_memory_map_stays_separate_from_board2() {
        let mut board = IoBoard::new(&vec![0x5a; 0x8000], Eeprom93c46::new()).cpu.io;
        for address in [0, 0x3fff] {
            board.write_byte(address, 0xa5);
            assert_eq!(board.read_byte(address), 0x5a);
        }
        for address in [0x4000, 0x5fff] {
            board.write_byte(address, 0x23);
            assert_eq!(board.read_byte(address), 0x23);
        }
        // These are not RAM/ADC locations on this first-generation board.
        for address in [0x6000, 0x8040, 0x8080, 0x8200, 0xe000, 0xffff] {
            board.write_byte(address, 0x12);
            assert_eq!(board.read_byte(address), 0xff);
        }
    }

    #[test]
    fn parallel_inputs_and_dips_keep_first_generation_port_wiring() {
        let mut io = IoBoard::new(&[], Eeprom93c46::new());
        io.set_inputs(Inputs {
            in0: 0x12,
            in1: 0x34,
            in2: 0x56,
            dsw: [0x78, 0x9a, 0xbc],
            ..Inputs::default()
        });
        let board = &mut io.cpu.io;
        board.write_byte(0x8008, 0x0e);
        for (port, expected) in [(1, 0x12), (2, 0x34), (3, 0x56)] {
            assert_eq!(board.read_byte(0x8000 + port), expected);
        }
        board.write_byte(0x8000, 1);
        for (port, expected) in [(1, 0x78), (2, 0x9a), (3, 0xbc)] {
            assert_eq!(board.read_byte(0x8000 + port), expected);
        }
        board.write_byte(0x8000, 0);
        assert_eq!(board.read_byte(0x8001), 0x12);
    }

    #[test]
    fn adc_samples_on_write_and_selects_both_analog_banks() {
        let mut io = IoBoard::new(&[], Eeprom93c46::new());
        let values = [0x15, 0x2a, 0x43, 0x68, 0x87, 0xab, 0xce, 0xf1];
        let inputs = Inputs {
            steer: values[0],
            accel: values[1],
            brake: values[2],
            analog: values,
            ..Inputs::default()
        };
        for (channel, expected) in values.into_iter().enumerate() {
            io.set_inputs(inputs);
            io.cpu.io.write_byte(0x8000, (channel / 4) as u8);
            io.cpu.io.write_byte(0xc000 + (channel & 3) as u16, 0xff);
            // Later input changes must not alter a conversion already latched.
            io.set_inputs(Inputs::default());
            let value = (0..8).fold(0, |value, bit| {
                (value << 1) | io.cpu.io.read_byte(0xc000 + (bit & 3))
            });
            assert_eq!(value, expected, "channel {channel}");
            assert_eq!(io.cpu.io.read_byte(0xc000), 0);
        }
    }

    #[test]
    fn host_addresses_wrap_at_the_board_dual_port_ram_not_in_the_chip() {
        let mut io = IoBoard::new(&[], Eeprom93c46::new());
        for (reg, value) in [
            (0x0a, 0x23),
            (9, 0),
            (0x0a, 0xf9),
            (9, 1),
            (0x0a, 0x5e),
            (9, 7),
        ] {
            io.cpu.io.write_byte(0x8000 + reg, value);
        }
        assert_eq!(io.dpram()[0x123], 0x5e);
        io.dpram_mut()[0x123] = 0xa9;
        assert_eq!(io.cpu.io.read_byte(0x800c), 0xa9);
        io.cpu.io.write_byte(0x8009, 0x73);
        assert_eq!(io.dpram()[3], 0x5e);
        assert_eq!(io.dpram()[0x123], 0xa9);
    }

    #[test]
    fn output_effects_are_applied_immediately_even_for_input_ports() {
        let mut io = IoBoard::new(&[], Eeprom93c46::new());
        io.cpu.io.write_byte(0x8008, 0x30);
        io.cpu.io.write_byte(0x8004, 0x12);
        io.cpu.io.write_byte(0x8005, 0x34);
        assert_eq!(io.drive_cmd(), 0x12);
        assert_eq!(io.cpu.io.outputs, 0x34);
        // Input reads still use external wiring rather than the output latch.
        assert_eq!(io.cpu.io.read_byte(0x8004), 0xff);
        io.cpu.io.drive_cmd = 0;
        io.cpu.io.outputs = 0;
        io.cpu.io.write_byte(0x8008, 0);
        assert_eq!(io.drive_cmd(), 0x12);
        assert_eq!(io.cpu.io.outputs, 0x34);
    }

    fn eeprom_bits(board: &mut Board, value: u32, count: u32) {
        for bit in (0..count).rev() {
            let lines = 0x40 | (((value >> bit) as u8 & 1) << 5);
            board.write_byte(0x8000, lines);
            board.write_byte(0x8000, lines | 0x80);
        }
    }

    #[test]
    fn eeprom_commands_survive_the_shared_chip_boundary() {
        let mut io = IoBoard::new(&[], Eeprom93c46::new());
        // Enable writes: start, opcode 00, address 11xxxx.
        eeprom_bits(&mut io.cpu.io, 0x130, 9);
        io.cpu.io.write_byte(0x8000, 0);
        // Write word 7: start, opcode 01, six-bit address, sixteen data bits.
        eeprom_bits(&mut io.cpu.io, 0x147, 9);
        eeprom_bits(&mut io.cpu.io, 0xa635, 16);
        io.cpu.io.write_byte(0x8000, 0);
        assert_eq!(io.eeprom().data[7], 0xa635);
        assert!(io.eeprom().dirty);
        assert_eq!(io.eeprom().data[6], 0xffff);
        // Read through port G bit 7, including the initial dummy zero.
        io.cpu.io.write_byte(0x8008, 0x40);
        eeprom_bits(&mut io.cpu.io, 0x187, 9);
        assert_eq!(io.cpu.io.read_byte(0x8006), 0x7f);
        let mut result = 0u16;
        for _ in 0..16 {
            eeprom_bits(&mut io.cpu.io, 0, 1);
            let port = io.cpu.io.read_byte(0x8006);
            assert_eq!(port & 0x7f, 0x7f);
            result = (result << 1) | (port >> 7) as u16;
        }
        assert_eq!(result, 0xa635);
        io.cpu.io.write_byte(0x8000, 0);
        assert_eq!(io.cpu.io.read_byte(0x8006), 0xff);
    }
}

impl Board {
    /// The digital input ports, in the order the 315-5338A reads them. On a
    /// twin cabinet port A bit 0 swaps in the DIP switches instead.
    fn input_port(&self, port: u8) -> u8 {
        let i = &self.inputs;
        match (port, self.secondary_controls) {
            (1, false) => i.in0,
            (2, false) => i.in1,
            (3, false) => i.in2,
            (1, true) => i.dsw[0],
            (2, true) => i.dsw[1],
            (3, true) => i.dsw[2],
            // Port E reads the drive board back; nothing here models one, and
            // the firmware only checks that it answers.
            (4, _) => 0xff,
            // Port G: the EEPROM's data line, then the four board buttons,
            // which are not wired to anything a player can press.
            (6, _) => (u8::from(self.eeprom.do_read()) << 7) | 0x7f,
            _ => 0xff,
        }
    }

    /// One ADC channel's reading. Channels 4 to 7 are the second set of
    /// controls on a twin cabinet, which port A selects between.
    fn analog(&self, channel: usize) -> u8 {
        let i = &self.inputs;
        match channel + usize::from(self.secondary_controls) * 4 {
            0 => i.steer,
            1 => i.accel,
            2 => i.brake,
            other => i.analog.get(other).copied().unwrap_or(0),
        }
    }

    fn io_write(&mut self, reg: u8, value: u8) {
        match self.io.write(reg, value) {
            WriteEffect::None => {}
            WriteEffect::Ports(mask) => {
                for port in 0..7u8 {
                    if mask & (1 << port) != 0 {
                        self.output_port(port, self.io.port_value(port));
                    }
                }
            }
            WriteEffect::Host { address, data } => self.dpram_write(address, data),
        }
    }

    fn dpram_write(&mut self, address: u16, data: u8) {
        if let Some(slot) = self.dpram.get_mut(address as usize & (DPRAM_SIZE - 1)) {
            *slot = data;
        }
    }

    fn output_port(&mut self, port: u8, value: u8) {
        match port {
            0 => {
                // 7 eeprom clk, 6 eeprom cs, 5 eeprom di, 4 eeprom pe,
                // 1 led, 0 which set of controls the panel presents.
                self.eeprom.cs_write(value & 0x40 != 0);
                self.eeprom.di_write(value & 0x20 != 0);
                self.eeprom.clk_write(value & 0x80 != 0);
                self.secondary_controls = value & 0x01 != 0;
            }
            4 => {
                // Trace each port-E output, including commands later replaced
                // in this frame. The normal drive_cmd latch keeps only the last.
                log::trace!(target: "model1_drive", "port_e={value:02X}");
                self.drive_cmd = value;
            }
            5 => self.outputs = value,
            _ => {}
        }
    }
}
