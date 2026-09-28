//! Frontend-independent CPU/bus owner. Resources and delivered pin events are
//! not serialized. Unsupported accesses latch a fault until reset or restore.
use super::*;
use std::cell::{Cell, Ref, RefCell};
use z80::{CpuState, Z80_io, Z80};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerialEvent {
    pub clock: u64,
    pub channel: u8,
    pub pins: SerialOutputs,
}

struct CpuBus {
    bus: RefCell<Bus>,
    fault: Cell<Option<BusError>>,
    elapsed: u64,
    events: Vec<SerialEvent>,
}
impl CpuBus {
    fn latch<T>(&self, value: Result<T, BusError>, fallback: T) -> T {
        match value {
            Ok(value) => value,
            Err(error) => {
                if self.fault.get().is_none() {
                    self.fault.set(Some(error));
                }
                fallback
            }
        }
    }
}
impl Z80_io for CpuBus {
    fn read_byte(&self, address: u16) -> u8 {
        if self.fault.get().is_some() {
            return 0xff;
        }
        self.latch(self.bus.borrow().read_memory(address), 0xff)
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        if self.fault.get().is_none() {
            self.latch(self.bus.borrow_mut().write_memory(address, value), ());
        }
    }
    fn port_in(&self, address: u16) -> u8 {
        if self.fault.get().is_some() {
            return 0xff;
        }
        self.latch(self.bus.borrow_mut().read_port(address), 0xff)
    }
    fn port_out(&mut self, address: u16, value: u8) {
        if self.fault.get().is_some() {
            return;
        }
        let before = std::array::from_fn::<_, 2, _>(|ch| {
            self.bus.borrow().state.cpu_peripherals.sio.outputs(ch)
        });
        self.latch(self.bus.borrow_mut().write_port(address, value), ());
        for (channel, before) in before.into_iter().enumerate() {
            let after = self.bus.borrow().state.cpu_peripherals.sio.outputs(channel);
            if before != after {
                self.events.push(SerialEvent {
                    clock: self.elapsed,
                    channel: channel as u8,
                    pins: after,
                });
            }
        }
    }
    fn irq_pending(&self) -> bool {
        self.fault.get().is_none() && self.bus.borrow().irq_pending()
    }
    fn irq_acknowledge(&mut self, _: u8) -> u8 {
        let vector = self
            .bus
            .borrow_mut()
            .acknowledge_irq()
            .ok_or(BusError::MissingInterruptSource);
        self.latch(vector, 0xff)
    }
    fn reti(&mut self) {
        if self.fault.get().is_none() {
            self.bus.borrow_mut().reti();
        }
    }
    fn advance(&mut self, clocks: u32) {
        if self.fault.get().is_some() {
            return;
        }
        let elapsed = self.elapsed;
        let events = &mut self.events;
        self.bus
            .borrow_mut()
            .state
            .cpu_peripherals
            .advance_observed(
                clocks,
                |_, _, _| {},
                |offset, channel, pins| {
                    events.push(SerialEvent {
                        clock: elapsed + u64::from(offset),
                        channel,
                        pins,
                    });
                },
            );
        self.elapsed += u64::from(clocks);
    }
}

/// Internal, unversioned state; restore only with the same firmware/cabinet.
/// Pending CPU/peripheral state and fractional instruction debt are included.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardState {
    cpu: CpuState,
    bus: BusState,
    cycle_debt: i64,
    elapsed: u64,
    fault: Option<BusError>,
}

pub struct IoBoard {
    cpu: Z80<CpuBus>,
    cycle_debt: i64,
}
impl IoBoard {
    pub fn new(firmware: &[u8], eeprom: Eeprom93c46) -> Result<Self, BusError> {
        let bus = Bus::new(firmware, eeprom)?;
        let mut cpu = Z80::new(CpuBus {
            bus: RefCell::new(bus),
            fault: Cell::new(None),
            elapsed: 0,
            events: vec![],
        });
        cpu.init();
        Ok(Self { cpu, cycle_debt: 0 })
    }
    /// Same board and firmware, with MAME's R360 cabinet feedback wiring.
    pub fn new_r360(firmware: &[u8], eeprom: Eeprom93c46) -> Result<Self, BusError> {
        let mut board = Self::new(firmware, eeprom)?;
        board.cpu.io.bus.get_mut().state.r360 = Some(Default::default());
        Ok(board)
    }
    pub fn is_r360(&self) -> bool {
        self.cpu.io.bus.borrow().state.r360.is_some()
    }
    /// Run a bounded number of requested board clocks, carrying overshoot to
    /// the next call. Pin events are delivered synchronously in clock order.
    /// An unsupported access may leave effects from its partial instruction;
    /// no further instructions run after the fault, including on later calls.
    pub fn run(
        &mut self,
        clocks: u32,
        mut serial: impl FnMut(SerialEvent),
    ) -> Result<(), BusError> {
        if let Some(error) = self.cpu.io.fault.get() {
            return Err(error);
        }
        self.cycle_debt += i64::from(clocks);
        while self.cycle_debt > 0 {
            self.cycle_debt -= i64::from(self.cpu.step());
            for event in self.cpu.io.events.drain(..) {
                serial(event);
            }
            if let Some(error) = self.cpu.io.fault.get() {
                return Err(error);
            }
        }
        Ok(())
    }
    /// Soft CPU/peripheral reset. RAM, EEPROM, input pins and elapsed time
    /// survive. Callers may query pin levels afterwards; no writes are replayed.
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cpu.clr_irq();
        self.cpu.clr_nmi();
        self.cpu.io.bus.borrow_mut().reset_cpu_peripherals();
        self.cpu.io.fault.set(None);
        self.cpu.io.events.clear();
        self.cycle_debt = 0;
    }
    pub fn cpu_state(&self) -> CpuState {
        self.cpu.snapshot()
    }
    pub fn elapsed_clocks(&self) -> u64 {
        self.cpu.io.elapsed
    }
    pub fn bus(&self) -> Ref<'_, Bus> {
        self.cpu.io.bus.borrow()
    }
    pub fn set_inputs(&mut self, inputs: Inputs) {
        self.cpu.io.bus.borrow_mut().set_inputs(inputs);
    }
    pub fn set_serial_inputs(&mut self, channel: u8, inputs: SerialInputs) -> Result<(), BusError> {
        self.cpu
            .io
            .bus
            .borrow_mut()
            .set_serial_inputs(channel, inputs)
    }
    pub fn dpram(&self) -> Ref<'_, [u8]> {
        Ref::map(self.cpu.io.bus.borrow(), |bus| &bus.state.dpram[..])
    }
    pub fn dpram_mut(&mut self) -> &mut [u8] {
        &mut self.cpu.io.bus.get_mut().state.dpram
    }
    pub fn eeprom(&self) -> Ref<'_, Eeprom93c46> {
        Ref::map(self.cpu.io.bus.borrow(), |bus| &bus.state.eeprom)
    }
    pub fn eeprom_mut(&mut self) -> &mut Eeprom93c46 {
        &mut self.cpu.io.bus.get_mut().state.eeprom
    }
    pub fn fault(&self) -> Option<BusError> {
        self.cpu.io.fault.get()
    }
    pub fn snapshot(&self) -> BoardState {
        debug_assert!(self.cpu.io.events.is_empty());
        BoardState {
            cpu: self.cpu.snapshot(),
            bus: self.cpu.io.bus.borrow().snapshot(),
            cycle_debt: self.cycle_debt,
            elapsed: self.cpu.io.elapsed,
            fault: self.cpu.io.fault.get(),
        }
    }
    pub(crate) fn validate_state(&self, state: &BoardState) -> Result<(), BusError> {
        // Validate every component before changing anything. Callbacks and
        // firmware are owned by this instance, not deserialized from the state.
        if !state.cpu.is_valid()
            || !state.bus.cpu_peripherals.valid_state()
            || !(-(u32::MAX as i64)..=i64::from(u32::MAX)).contains(&state.cycle_debt)
            || (state.fault.is_none() && state.cycle_debt > 0)
        {
            return Err(BusError::InvalidSnapshot);
        }
        self.cpu.io.bus.borrow().validate_state(&state.bus)
    }
    pub fn restore(&mut self, state: &BoardState) -> Result<(), BusError> {
        self.validate_state(state)?;
        self.cpu.io.bus.borrow_mut().restore(&state.bus)?;
        assert!(self.cpu.restore(&state.cpu));
        self.cycle_debt = state.cycle_debt;
        self.cpu.io.elapsed = state.elapsed;
        self.cpu.io.fault.set(state.fault);
        self.cpu.io.events.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r360_cpu_command_and_reply_continue_identically_after_restore() {
        let mut firmware = vec![0; FIRMWARE_SIZE];
        let program = [
            0x3e, 4, 0x32, 8, 0x80, // IN2 direction
            0x3e, 0xbd, 0x32, 4, 0x80, // setup command
            0x3a, 2, 0x80, 0x32, 0, 0xe0, // read reply into RAM
            0x76,
        ];
        firmware[..program.len()].copy_from_slice(&program);
        let mut first = IoBoard::new_r360(&firmware, Eeprom93c46::new()).unwrap();
        first.run(25, |_| {}).unwrap(); // mid-program, with CPU cycle debt
        let snapshot =
            bincode::deserialize(&bincode::serialize(&first.snapshot()).unwrap()).unwrap();
        let mut second = IoBoard::new_r360(&firmware, Eeprom93c46::new()).unwrap();
        second.restore(&snapshot).unwrap();
        first.run(150, |_| {}).unwrap();
        second.run(150, |_| {}).unwrap();
        assert_eq!(first.bus().read_memory(0xe000), Ok(0xbb));
        assert_eq!(
            bincode::serialize(&first.snapshot()).unwrap(),
            bincode::serialize(&second.snapshot()).unwrap()
        );
        let mut wrong = IoBoard::new(&firmware, Eeprom93c46::new()).unwrap();
        let before = bincode::serialize(&wrong.snapshot()).unwrap();
        assert_eq!(wrong.restore(&snapshot), Err(BusError::InvalidSnapshot));
        assert_eq!(bincode::serialize(&wrong.snapshot()).unwrap(), before);
    }
    fn board(program: &[u8]) -> IoBoard {
        let mut firmware = vec![0; FIRMWARE_SIZE];
        firmware[..program.len()].copy_from_slice(program);
        IoBoard::new(&firmware, Eeprom93c46::new()).unwrap()
    }
    #[test]
    fn cpu_reads_pio_dips_and_memory_using_actual_bus() {
        let mut b = board(&[
            0xdb, 0x1c, 0x32, 0x00, 0xe0, 0xdb, 0x1e, 0x32, 0x01, 0xe0, 0x76,
        ]);
        b.set_inputs(Inputs {
            dips: [0x12, 0x34, 0x56],
            ..Inputs::default()
        });
        b.run(100, |_| {}).unwrap();
        assert_eq!(b.bus().read_memory(0xe000), Ok(0x34));
        assert_eq!(b.bus().read_memory(0xe001), Ok(0x56));
        assert!(b.cpu_state().halted);
    }
    fn irq_board() -> IoBoard {
        // IM2 vector in RAM E020 -> firmware ISR 0040. Periodic CTC0 IRQ.
        let mut program = vec![0; 128];
        let init = [
            0x31, 0xf0, 0xff, 0x3e, 0xe0, 0xed, 0x47, 0xed, 0x5e, 0x21, 0x40, 0, 0x22, 0x20, 0xe0,
            0x3e, 0x20, 0xd3, 0x10, 0x3e, 0x85, 0xd3, 0x10, 0x3e, 8, 0xd3, 0x10, 0xfb, 0x76, 0x18,
            0xfd,
        ];
        program[..init.len()].copy_from_slice(&init);
        program[0x40..0x47].copy_from_slice(&[0x21, 0x00, 0xe1, 0x34, 0xfb, 0xed, 0x4d]);
        board(&program)
    }
    #[test]
    fn ctc_cpu_ack_reti_and_sliced_run_are_equivalent() {
        let mut whole = irq_board();
        let mut sliced = irq_board();
        whole.run(10_000, |_| {}).unwrap();
        for _ in 0..10_000 {
            sliced.run(1, |_| {}).unwrap();
        }
        assert!(whole.bus().read_memory(0xe100).unwrap() > 10);
        assert_eq!(
            bincode::serialize(&whole.snapshot()).unwrap(),
            bincode::serialize(&sliced.snapshot()).unwrap()
        );
    }
    #[test]
    fn board_snapshot_resumes_inside_irq_with_cycle_debt_and_no_replayed_events() {
        let mut original = irq_board();
        for _ in 0..1000 {
            original.run(1, |_| {}).unwrap();
            if original.cpu_state().pc == 0x40 {
                break;
            }
        }
        assert_eq!(original.cpu_state().pc, 0x40);
        let encoded = bincode::serialize(&original.snapshot()).unwrap();
        let mut restored = irq_board();
        restored
            .restore(&bincode::deserialize(&encoded).unwrap())
            .unwrap();
        let mut first = vec![];
        let mut second = vec![];
        original.run(5555, |e| first.push(e)).unwrap();
        restored.run(5555, |e| second.push(e)).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            bincode::serialize(&original.snapshot()).unwrap(),
            bincode::serialize(&restored.snapshot()).unwrap()
        );
        let before = bincode::serialize(&restored.snapshot()).unwrap();
        let mut invalid = restored.snapshot();
        invalid.cpu.interrupt_mode = 3;
        assert_eq!(restored.restore(&invalid), Err(BusError::InvalidSnapshot));
        assert_eq!(bincode::serialize(&restored.snapshot()).unwrap(), before);
    }
    #[test]
    fn unsupported_access_fault_is_sticky_across_runs_and_snapshot() {
        let mut b = board(&[0x3a, 0x00, 0x81, 0x32, 0x00, 0xe0]);
        let expected = Err(BusError::UnimplementedMemory(0x8100));
        assert_eq!(b.run(1000, |_| {}), expected);
        assert_eq!(b.bus().read_memory(0xe000), Ok(0));
        let snapshot = b.snapshot();
        let before = bincode::serialize(&snapshot).unwrap();
        assert_eq!(b.run(1000, |_| {}), expected);
        assert_eq!(bincode::serialize(&b.snapshot()).unwrap(), before);
        b.reset();
        assert_eq!(b.cpu_state().pc, 0);
        b.restore(&snapshot).unwrap();
        assert_eq!(b.run(1, |_| {}), expected);
    }
    #[test]
    fn reset_preserves_ram_eeprom_inputs_and_host_shared_memory() {
        let mut b = irq_board();
        b.run(1000, |_| {}).unwrap();
        b.dpram_mut()[5] = 0x42;
        let ram = b.bus().read_memory(0xe100).unwrap();
        let elapsed = b.elapsed_clocks();
        let eeprom = bincode::serialize(b.bus().eeprom()).unwrap();
        b.reset();
        assert_eq!(b.bus().read_memory(0xe100), Ok(ram));
        assert_eq!(b.bus().dpram()[5], 0x42);
        assert_eq!(b.elapsed_clocks(), elapsed);
        assert_eq!(bincode::serialize(b.bus().eeprom()).unwrap(), eeprom);
        assert!(!b.bus().irq_pending());
        assert_eq!(b.cpu_state().pc, 0);
    }

    fn serial_board() -> IoBoard {
        // Configure both channels 8N1 x16, clocked independently by CTC2/3.
        let mut program = vec![];
        let mut out = |port, data| program.extend_from_slice(&[0x3e, data, 0xd3, port]);
        for ctrl in [0x19, 0x1b] {
            for (reg, value) in [(4, 0x44), (3, 0xc1), (5, 0xea)] {
                out(ctrl, reg);
                out(ctrl, value);
            }
        }
        for port in [0x12, 0x13] {
            out(port, 5);
            out(port, 1);
        }
        out(0x18, 0x55);
        out(0x1a, 0xa5);
        program.push(0x76);
        board(&program)
    }

    #[test]
    fn ctc_edges_clock_both_sio_channels_with_ordered_snapshot_stable_events() {
        let mut whole = serial_board();
        let mut split = serial_board();
        let mut first = vec![];
        let mut second = vec![];
        whole.run(6000, |e| first.push(e)).unwrap();
        for _ in 0..6000 {
            split.run(1, |e| second.push(e)).unwrap();
        }
        assert_eq!(first, second);
        assert!(first.windows(2).all(|pair| pair[0].clock <= pair[1].clock));
        for channel in 0..2 {
            let events: Vec<_> = first
                .iter()
                .filter(|e| e.channel == channel && !e.pins.rts)
                .collect();
            assert!(events.len() >= 7);
            assert!(events.windows(2).any(|p| p[1].clock - p[0].clock == 256)); // x16 CTC /16
        }
        assert_eq!(
            bincode::serialize(&whole.snapshot()).unwrap(),
            bincode::serialize(&split.snapshot()).unwrap()
        );
        let mut original = serial_board();
        original.run(1200, |_| {}).unwrap();
        let mut restored = serial_board();
        restored.restore(&original.snapshot()).unwrap();
        let mut a = vec![];
        let mut b = vec![];
        original.run(4800, |e| a.push(e)).unwrap();
        restored.run(4800, |e| b.push(e)).unwrap();
        assert!(!a.is_empty());
        assert_eq!(a, b);
        assert_eq!(
            bincode::serialize(&original.snapshot()).unwrap(),
            bincode::serialize(&restored.snapshot()).unwrap()
        );
    }

    #[test]
    fn unsupported_port_write_stops_before_following_instruction() {
        let mut b = board(&[0x3e, 0x8f, 0xd3, 0x1d, 0x32, 0x00, 0xe0]);
        assert_eq!(b.run(100, |_| {}), Err(BusError::UnimplementedPort(0x1d)));
        assert_eq!(b.bus().read_memory(0xe000), Ok(0));
        assert_eq!(b.cpu_state().pc, 4);
    }
}
