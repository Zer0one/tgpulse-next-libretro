// license: BSD-3-Clause
// Reference: MAME src/mame/sega/model1io2.cpp, copyright-holder Dirk Best.
// See LICENSES/MAME-BSD-3-Clause.txt.
//! Advanced Model 1 I/O board bus: 837-10859 (Wing War), 837-11659 (NetMerc).
//!
//! Bus and CPU integration, selected for Wing War and R360 by `model1board`.
//! Firmware is supplied in memory; no host resources or
//! frontend bindings live here. Known missing devices return errors rather than
//! invented ready values. Other games retain their existing board selection.

use crate::eeprom93c46::Eeprom93c46;
use crate::msm6253::Adc;
use crate::sega3155338::{Chip5338, WriteEffect};
use crate::tmpz84c015::{InterruptSource, Peripherals};
pub use crate::z80sio::{SerialInputs, SerialOutputs};

mod cpu;
mod r360;
pub use cpu::{BoardState, IoBoard, SerialEvent};

pub const CPU_HZ: u32 = 9_830_400;
const FIRMWARE_SIZE: usize = 0x10000;
const DPRAM_SIZE: usize = 0x800;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BusError {
    FirmwareSize(usize),
    UnimplementedMemory(u16),
    UnimplementedPort(u8),
    InvalidSnapshot,
    InvalidSerialChannel(u8),
    MissingInterruptSource,
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirmwareSize(size) => {
                write!(f, "I/O board 2 firmware must be 65536 bytes, got {size}")
            }
            Self::UnimplementedMemory(addr) => write!(
                f,
                "I/O board 2 device at memory {addr:04X} is not implemented"
            ),
            Self::UnimplementedPort(port) => {
                write!(f, "TMPZ84C015 device at port {port:02X} is not implemented")
            }
            Self::InvalidSnapshot => write!(f, "invalid Model 1 I/O board snapshot"),
            Self::MissingInterruptSource => {
                write!(f, "interrupt acknowledged without a pending source")
            }
            Self::InvalidSerialChannel(channel) => {
                write!(f, "invalid serial channel {channel}, expected 0 or 1")
            }
        }
    }
}
impl std::error::Error for BusError {}

/// Physical board pins, not another frontend signal/binding catalogue. The
/// existing game input translation will supply these when the board is enabled.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Inputs {
    pub digital: [u8; 3],
    pub analog: [u8; 8],
    pub dips: [u8; 3],
    pub drive: u8,
    /// Active-low buttons 0..3 and jumpers 4..5. EEPROM DO/unused bits are not
    /// configurable here; read_memory(0x8040) supplies them from the hardware.
    pub board_switches: u8,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            digital: [0xff; 3],
            analog: [0xff; 8],
            dips: [0xff; 3],
            drive: 0xff,
            board_switches: 0x3f,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Outputs {
    pub lamps: u8,
    pub drive: u8,
    /// Pin to the external MB3773, not an emulated external watchdog timeout.
    pub watchdog_clock: bool,
    pub comm_error: bool,
}

/// Mutable peripheral state only: no firmware, callbacks, CPU or host objects.
/// This is not a versioned machine save-state format. The caller must retain
/// the same firmware. IoBoard's combined snapshot also retains CPU/scheduler state.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BusState {
    #[serde(with = "serde_big_array::BigArray")]
    ram: [u8; 0x2000],
    #[serde(with = "serde_big_array::BigArray")]
    dpram: [u8; DPRAM_SIZE],
    io: Chip5338,
    adc: Adc,
    cpu_peripherals: Peripherals,
    eeprom: Eeprom93c46,
    inputs: Inputs,
    outputs: Outputs,
    secondary: bool,
    lcd_data: u8,
    r360: Option<r360::Cabinet>,
}

pub struct Bus {
    firmware: Box<[u8]>,
    state: BusState,
}

impl Bus {
    pub fn new(firmware: &[u8], eeprom: Eeprom93c46) -> Result<Self, BusError> {
        if firmware.len() != FIRMWARE_SIZE {
            return Err(BusError::FirmwareSize(firmware.len()));
        }
        Ok(Self {
            firmware: firmware.into(),
            state: BusState {
                ram: [0; 0x2000],
                dpram: [0; DPRAM_SIZE],
                io: Chip5338::default(),
                adc: Adc::default(),
                cpu_peripherals: Peripherals::default(),
                eeprom,
                inputs: Inputs::default(),
                outputs: Outputs {
                    drive: 0xff,
                    ..Outputs::default()
                },
                secondary: false,
                lcd_data: 0,
                r360: None,
            },
        })
    }

    pub fn set_inputs(&mut self, mut inputs: Inputs) {
        if let Some(cabinet) = &mut self.state.r360 {
            cabinet.throttle = inputs.analog[2];
            inputs.analog[2] = 0; // MAME R360: throttle travels via drive commands.
        }
        self.state
            .cpu_peripherals
            .set_pio_inputs([inputs.dips[1], inputs.dips[2]]);
        self.state.inputs = inputs;
    }
    pub fn set_serial_inputs(&mut self, channel: u8, inputs: SerialInputs) -> Result<(), BusError> {
        if channel > 1 {
            return Err(BusError::InvalidSerialChannel(channel));
        }
        self.state
            .cpu_peripherals
            .sio
            .inputs(channel as usize, inputs);
        Ok(())
    }
    pub fn serial_outputs(&self, channel: u8) -> Result<SerialOutputs, BusError> {
        if channel > 1 {
            return Err(BusError::InvalidSerialChannel(channel));
        }
        Ok(self.state.cpu_peripherals.sio.outputs(channel as usize))
    }
    pub fn outputs(&self) -> Outputs {
        self.state.outputs
    }
    pub fn dpram(&self) -> &[u8; DPRAM_SIZE] {
        &self.state.dpram
    }
    pub fn dpram_mut(&mut self) -> &mut [u8; DPRAM_SIZE] {
        &mut self.state.dpram
    }
    pub fn eeprom(&self) -> &Eeprom93c46 {
        &self.state.eeprom
    }
    pub fn snapshot(&self) -> BusState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: &BusState) -> Result<(), BusError> {
        self.validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    pub(crate) fn validate_state(&self, state: &BusState) -> Result<(), BusError> {
        if !state.cpu_peripherals.valid_state() || state.r360.is_some() != self.state.r360.is_some()
        {
            return Err(BusError::InvalidSnapshot);
        }
        Ok(())
    }

    /// Only resets the implemented CPU peripherals. It does not erase RAM,
    /// EEPROM, chip output latches or cabinet state, nor does it reset a CPU.
    pub fn reset_cpu_peripherals(&mut self) {
        self.state.cpu_peripherals.reset();
    }

    pub fn read_memory(&self, address: u16) -> Result<u8, BusError> {
        let state = &self.state;
        Ok(match address {
            0x0000..=0x7fff => self.firmware[address as usize],
            0x8000..=0x800f => state.io.read(
                (address & 15) as u8,
                |port| match port {
                    2 if state.r360.is_some() => state.r360.as_ref().unwrap().response,
                    0..=2 => state.inputs.digital[port as usize],
                    4 => state.inputs.drive,
                    _ => 0xff,
                },
                |addr| state.dpram[addr as usize & (DPRAM_SIZE - 1)],
            ),
            0x8040 => {
                0x80 | (state.inputs.board_switches & 0x3f)
                    | (u8::from(state.eeprom.do_read()) << 6)
            }
            0x8080 => state.inputs.dips[0],
            0x8100..=0x810f => return Err(BusError::UnimplementedMemory(address)),
            0x8200..=0x8207 => state.adc.shift_out(),
            0xe000..=0xffff => state.ram[(address - 0xe000) as usize],
            _ => 0xff,
        })
    }

    pub fn write_memory(&mut self, address: u16, data: u8) -> Result<(), BusError> {
        match address {
            0x8000..=0x800f => match self.state.io.write((address & 15) as u8, data) {
                WriteEffect::None => {}
                WriteEffect::Host { address, data } => {
                    self.state.dpram[address as usize & (DPRAM_SIZE - 1)] = data
                }
                WriteEffect::Ports(mask) => {
                    for port in 0..7 {
                        if mask & (1 << port) != 0 {
                            self.output_port(port, self.state.io.port_value(port))?;
                        }
                    }
                }
            },
            0x8100..=0x810f => return Err(BusError::UnimplementedMemory(address)),
            0x8200..=0x8207 => {
                let channel = (address & 3) as usize + usize::from(self.state.secondary) * 4;
                self.state.adc.latch(self.state.inputs.analog[channel]);
            }
            0xe000..=0xffff => self.state.ram[(address - 0xe000) as usize] = data,
            _ => {}
        }
        Ok(())
    }

    fn output_port(&mut self, port: u8, data: u8) -> Result<(), BusError> {
        let state = &mut self.state;
        match port {
            3 => state.outputs.lamps = data,
            4 => {
                state.outputs.drive = data;
                if let Some(cabinet) = &mut state.r360 {
                    cabinet.write(data);
                }
                state.lcd_data = data;
            }
            5 => {
                // Preserve the reference's clock-before-data/select ordering.
                // Firmware must establish DI/CS before the rising clock write.
                state.eeprom.clk_write(data & 0x20 != 0);
                state.eeprom.di_write(data & 0x40 != 0);
                state.eeprom.cs_write(data & 0x10 != 0);
                if data & 0x0e == 0x04 {
                    // EEPROM pins/latch have already changed. Stop here rather
                    // than pretending the diagnostic LCD accepted the write.
                    return Err(BusError::UnimplementedMemory(0x8005));
                }
            }
            6 => {
                state.outputs.watchdog_clock = data & 0x80 != 0;
                state.secondary = data & 0x40 != 0;
                state.outputs.comm_error = data & 0x20 == 0;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn read_port(&mut self, address: u16) -> Result<u8, BusError> {
        self.state
            .cpu_peripherals
            .read(address)
            .ok_or(BusError::UnimplementedPort(address as u8))
    }

    pub fn write_port(&mut self, address: u16, data: u8) -> Result<(), BusError> {
        self.state
            .cpu_peripherals
            .write(address, data)
            .ok_or(BusError::UnimplementedPort(address as u8))
    }

    /// Peripheral-only advancement. IoBoard supplies instruction/interrupt
    /// clocks and additionally delivers timestamped serial output transitions.
    pub fn advance(&mut self, clocks: u32, output: impl FnMut(u32, u8, bool)) {
        self.state.cpu_peripherals.advance(clocks, output);
    }

    pub fn trigger_ctc(&mut self, channel: u8, level: bool) -> bool {
        let pulse = self.state.cpu_peripherals.ctc.trigger(channel, level);
        if pulse && (2..=3).contains(&channel) {
            self.state
                .cpu_peripherals
                .sio
                .clock((channel - 2) as usize, true);
        }
        pulse
    }

    pub fn watchdog_output(&self) -> bool {
        self.state.cpu_peripherals.watchdog_output()
    }

    pub fn irq_pending(&self) -> bool {
        self.state
            .cpu_peripherals
            .interrupt_source(
                self.state.cpu_peripherals.sio.irq_state(),
                self.state.cpu_peripherals.pio.irq_state(),
            )
            .is_some()
    }

    pub fn acknowledge_irq(&mut self) -> Option<u8> {
        let p = &mut self.state.cpu_peripherals;
        match p.interrupt_source(p.sio.irq_state(), p.pio.irq_state())? {
            InterruptSource::Ctc => p.ctc.acknowledge(),
            InterruptSource::Sio => p.sio.acknowledge(),
            InterruptSource::Pio => p.pio.acknowledge(),
        }
    }

    pub fn reti(&mut self) {
        let p = &mut self.state.cpu_peripherals;
        match p.reti_source(p.sio.irq_state(), p.pio.irq_state()) {
            Some(InterruptSource::Ctc) => p.ctc.reti(),
            Some(InterruptSource::Sio) => p.sio.reti(),
            Some(InterruptSource::Pio) => p.pio.reti(),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus() -> Bus {
        Bus::new(&vec![0xa5; FIRMWARE_SIZE], Eeprom93c46::new()).unwrap()
    }

    #[test]
    fn firmware_window_ram_banks_and_unmapped_addresses() {
        assert!(matches!(
            Bus::new(&[], Eeprom93c46::new()),
            Err(BusError::FirmwareSize(0))
        ));
        let mut bus = bus();
        for addr in [0, 0x4000, 0x7fff] {
            bus.write_memory(addr, 0).unwrap();
            assert_eq!(bus.read_memory(addr), Ok(0xa5));
        }
        for (addr, data) in [(0xe000, 1), (0xefff, 2), (0xf000, 3), (0xffff, 4)] {
            bus.write_memory(addr, data).unwrap();
        }
        for (addr, data) in [(0xe000, 1), (0xefff, 2), (0xf000, 3), (0xffff, 4)] {
            assert_eq!(bus.read_memory(addr), Ok(data));
        }
        for addr in [0x8010, 0x8041, 0x8180, 0x8208, 0xc000] {
            bus.write_memory(addr, 0).unwrap();
            assert_eq!(bus.read_memory(addr), Ok(0xff));
        }
    }

    #[test]
    fn advanced_ports_do_not_reuse_first_generation_digital_bank_wiring() {
        let mut bus = bus();
        bus.set_inputs(Inputs {
            digital: [0x12, 0x34, 0x56],
            dips: [0xab, 0xcd, 0xef],
            board_switches: 0x25,
            drive: 0x78,
            ..Inputs::default()
        });
        bus.write_memory(0x8008, 0x17).unwrap();
        for bank in [0, 0x40] {
            bus.write_memory(0x8006, bank).unwrap();
            assert_eq!(bus.read_memory(0x8000), Ok(0x12));
            assert_eq!(bus.read_memory(0x8001), Ok(0x34));
            assert_eq!(bus.read_memory(0x8002), Ok(0x56));
        }
        assert_eq!(bus.read_memory(0x8004), Ok(0x78));
        assert_eq!(bus.read_memory(0x8080), Ok(0xab));
        assert_eq!(bus.read_memory(0x8040), Ok(0xe5));
        bus.write_memory(0x8003, 0x97).unwrap();
        bus.write_memory(0x8004, 0x68).unwrap();
        bus.write_memory(0x8006, 0xa0).unwrap();
        assert_eq!(
            bus.outputs(),
            Outputs {
                lamps: 0x97,
                drive: 0x68,
                watchdog_clock: true,
                comm_error: false
            }
        );
    }

    #[test]
    fn r360_feedback_matches_mame_and_preserves_latched_reply_on_restore() {
        let mut original = bus();
        original.state.r360 = Some(Default::default());
        original.write_memory(0x8008, 4).unwrap(); // IN2 reads external cabinet.
        assert_eq!(original.read_memory(0x8002), Ok(0));
        for (cmd, response) in [
            (0xbf, 0xbf),
            (0xbe, 0xbf),
            (0xbd, 0xbb),
            (0xbc, 0xba),
            (0xbb, 0xb9),
            (0xba, 0xbf),
            (0xb9, 0xbf),
            (0x99, 0xbf),
        ] {
            original.write_memory(0x8004, cmd).unwrap();
            assert_eq!(original.read_memory(0x8002), Ok(response));
        }
        for throttle in 0..=255 {
            let mut inputs = Inputs::default();
            inputs.analog[2] = throttle;
            original.set_inputs(inputs);
            assert_eq!(original.state.inputs.analog[2], 0);
            original.write_memory(0x8004, 0xaf).unwrap();
            assert_eq!(original.read_memory(0x8002), Ok(!throttle));
        }
        let saved = bincode::serialize(&original.snapshot()).unwrap();
        let state = bincode::deserialize(&saved).unwrap();
        let mut restored = bus();
        assert_eq!(restored.restore(&state), Err(BusError::InvalidSnapshot));
        restored.state.r360 = Some(Default::default());
        restored.restore(&state).unwrap();
        for cmd in [0xbd, 0xbc, 0xbb, 0xaf, 0x00, 0xb9] {
            for b in [&mut original, &mut restored] {
                b.write_memory(0x8004, cmd).unwrap();
            }
            assert_eq!(original.read_memory(0x8002), restored.read_memory(0x8002));
            assert_eq!(
                bincode::serialize(&original.snapshot()).unwrap(),
                bincode::serialize(&restored.snapshot()).unwrap()
            );
        }
    }

    #[test]
    fn adc_mirror_channel_selection_and_secondary_bank() {
        let mut bus = bus();
        let values = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1];
        for mirror in [0, 4] {
            for (channel, expected) in values.into_iter().enumerate() {
                bus.set_inputs(Inputs {
                    analog: values,
                    ..Inputs::default()
                });
                bus.write_memory(0x8006, if channel >= 4 { 0x40 } else { 0 })
                    .unwrap();
                bus.write_memory(0x8200 + (channel as u16 & 3) + mirror, 0)
                    .unwrap();
                bus.set_inputs(Inputs::default());
                let value = (0..8).fold(0, |v, bit| {
                    (v << 1) | bus.read_memory(0x8200 + bit).unwrap()
                });
                assert_eq!(value, expected);
            }
        }
    }

    fn eeprom_bits(bus: &mut Bus, value: u32, bits: u32) {
        for bit in (0..bits).rev() {
            let pins = 0x18 | (((value >> bit) as u8 & 1) << 6); // CS, CN6 disabled
            bus.write_memory(0x8005, pins).unwrap();
            bus.write_memory(0x8005, pins | 0x20).unwrap();
        }
    }

    #[test]
    fn eeprom_uses_port_f_and_returns_data_at_8040_bit6() {
        let mut bus = bus();
        eeprom_bits(&mut bus, 0x130, 9);
        bus.write_memory(0x8005, 8).unwrap();
        eeprom_bits(&mut bus, 0x14a, 9);
        eeprom_bits(&mut bus, 0x583a, 16);
        bus.write_memory(0x8005, 8).unwrap();
        assert_eq!(bus.eeprom().data[10], 0x583a);
        assert!(bus.eeprom().dirty);
        eeprom_bits(&mut bus, 0x18a, 9);
        assert_eq!(bus.read_memory(0x8040), Ok(0xbf));
        let mut value = 0u16;
        for _ in 0..16 {
            eeprom_bits(&mut bus, 0, 1);
            let pins = bus.read_memory(0x8040).unwrap();
            assert_eq!(pins & 0xbf, 0xbf);
            value = (value << 1) | u16::from((pins >> 6) & 1);
        }
        assert_eq!(value, 0x583a);
    }

    #[test]
    fn serial_host_transfer_wraps_dpram_and_reads_main_board_updates() {
        let mut bus = bus();
        for (reg, value) in [(10, 0x67), (9, 0), (10, 0xf9), (9, 1), (10, 0x4a), (9, 7)] {
            bus.write_memory(0x8000 + reg, value).unwrap();
        }
        assert_eq!(bus.dpram()[0x167], 0x4a);
        bus.dpram_mut()[0x167] = 0x98;
        assert_eq!(bus.read_memory(0x800c), Ok(0x98));
    }

    #[test]
    fn known_missing_devices_are_not_reported_as_working() {
        let mut bus = bus();
        for addr in 0x8100..=0x810f {
            assert_eq!(
                bus.read_memory(addr),
                Err(BusError::UnimplementedMemory(addr))
            );
            assert_eq!(
                bus.write_memory(addr, 0),
                Err(BusError::UnimplementedMemory(addr))
            );
        }
        assert_eq!(
            bus.write_port(0x1d, 0x8f),
            Err(BusError::UnimplementedPort(0x1d))
        );
        bus.write_port(0x19, 3).unwrap();
        assert_eq!(
            bus.write_port(0x19, 1),
            Err(BusError::UnimplementedPort(0x19))
        );
        assert_eq!(
            bus.read_port(0xab19),
            Err(BusError::UnimplementedPort(0x19))
        ); // RR3
        assert_eq!(
            bus.write_memory(0x8005, 4),
            Err(BusError::UnimplementedMemory(0x8005))
        );
    }

    #[test]
    fn bus_ports_deliver_ctc_irq_only_until_ack_and_reti() {
        let mut bus = bus();
        bus.write_port(0xa010, 0xd8).unwrap();
        bus.write_port(0xb013, 0x85).unwrap();
        bus.write_port(0x13, 1).unwrap();
        let mut edges = Vec::new();
        bus.advance(16, |t, ch, high| edges.push((t, ch, high)));
        assert_eq!(edges, [(16, 3, true)]);
        assert!(bus.irq_pending());
        assert_eq!(bus.acknowledge_irq(), Some(0xde));
        assert!(!bus.irq_pending());
        bus.advance(16, |_, _, _| {});
        assert!(!bus.irq_pending());
        bus.reti();
        assert!(bus.irq_pending());
        bus.reset_cpu_peripherals();
        assert!(!bus.irq_pending());
        assert_eq!(bus.acknowledge_irq(), None);
    }

    #[test]
    fn real_ctc_sio_pio_sources_obey_priority_and_nested_reti() {
        let mut bus = bus();
        // A bit-mode PIO interrupt on DIP2 bit 0, vector 40.
        for data in [0x40, 0xcf, 0xff, 0xb7, 0xfe] {
            bus.write_port(0x1d, data).unwrap();
        }
        bus.set_inputs(Inputs {
            dips: [0xff, 0xfe, 0xff],
            ..Inputs::default()
        });
        bus.set_inputs(Inputs::default());
        // SIO A TX empty interrupt; fixed shared vector 80 from channel B.
        for (port, data) in [
            (0x1b, 2),
            (0x1b, 0x80),
            (0x19, 4),
            (0x19, 0x44),
            (0x19, 5),
            (0x19, 0x68),
            (0x19, 1),
            (0x19, 2),
            (0x18, 0x55),
        ] {
            bus.write_port(port, data).unwrap();
        }
        bus.write_port(0x10, 0x20).unwrap();
        bus.write_port(0x10, 0x85).unwrap();
        bus.write_port(0x10, 1).unwrap();
        bus.advance(16, |_, _, _| {});
        bus.write_port(0x10, 0x83).unwrap(); // stop timer without disabling its IRQ
        assert_eq!(bus.acknowledge_irq(), Some(0x20));
        assert!(!bus.irq_pending());
        bus.reti();
        assert_eq!(bus.acknowledge_irq(), Some(0x80));
        bus.write_port(0x19, 0x28).unwrap(); // clear SIO source, retain service latch
        assert!(!bus.irq_pending());
        bus.write_port(0xf4, 4).unwrap(); // PIO > CTC > SIO, permits PIO preemption
        assert_eq!(bus.acknowledge_irq(), Some(0x40));
        bus.reti();
        assert!(!bus.irq_pending());
        bus.reti();
        assert!(!bus.irq_pending());
        assert_eq!(bus.acknowledge_irq(), None);
    }

    #[test]
    fn bus_snapshot_continues_adc_ctc_and_memory_without_firmware_or_host_state() {
        let mut bus = bus();
        bus.write_memory(0xe023, 0x37).unwrap();
        bus.dpram_mut()[25] = 0x58;
        bus.set_inputs(Inputs {
            analog: [0x56; 8],
            ..Inputs::default()
        });
        bus.write_memory(0x8200, 0).unwrap();
        bus.read_memory(0x8200).unwrap();
        bus.write_port(0x12, 0x85).unwrap();
        bus.write_port(0x12, 3).unwrap();
        bus.advance(23, |_, _, _| {});
        let encoded = bincode::serialize(&bus.snapshot()).unwrap();
        assert!(encoded.len() < 0x10000); // immutable firmware is not included
        let snapshot = bincode::deserialize(&encoded).unwrap();
        let mut restored = Bus::new(&vec![0xa5; FIRMWARE_SIZE], Eeprom93c46::new()).unwrap();
        restored.restore(&snapshot).unwrap();
        for _ in 0..10 {
            assert_eq!(bus.read_memory(0x8200), restored.read_memory(0x8200));
        }
        let mut edges = Vec::new();
        let mut restored_edges = Vec::new();
        bus.advance(100, |t, ch, high| edges.push((t, ch, high)));
        restored.advance(100, |t, ch, high| restored_edges.push((t, ch, high)));
        assert_eq!(edges, restored_edges);
        assert_eq!(bus.acknowledge_irq(), restored.acknowledge_irq());
        assert_eq!(restored.read_memory(0xe023), Ok(0x37));
        assert_eq!(restored.dpram()[25], 0x58);
        assert_eq!(
            bincode::serialize(&bus.snapshot()).unwrap(),
            bincode::serialize(&restored.snapshot()).unwrap()
        );
    }
}
