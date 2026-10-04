// license: BSD-3-Clause
// Reference: MAME src/mame/sega/model1io2.cpp, copyright-holder Dirk Best.
// See LICENSES/MAME-BSD-3-Clause.txt.
//! Advanced Model 1 I/O board bus: 837-10859 (Wing War), 837-11659 (NetMerc).
//!
//! Bus and CPU integration, selected for Wing War, R360 and NetMerc by `model1board`.
//! Firmware is supplied in memory; no host resources or
//! frontend bindings live here. Known missing devices return errors rather than
//! invented ready values. Other games retain their existing board selection.

use crate::eeprom93c46::Eeprom93c46;
use crate::msm6253::Adc;
use crate::sega3155338::{Chip5338, WriteEffect};
use crate::tmpz84c015::{InterruptSource, Peripherals};
pub use crate::z80sio::{SerialInputs, SerialOutputs};

mod cpu;
mod lcd;
mod r360;
mod tracking;
pub use cpu::{BoardState, IoBoard, SerialEvent};
pub use tracking::{HmdPose, TrackingStatus};

pub const CPU_HZ: u32 = 9_830_400;
/// Palette indexes: 0 panel background, 1 lit dot, 2 unlit dot.
pub type DiagnosticPixels = [[u8; 121]; 19];
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
    lcd: lcd::Lcd,
    r360: Option<r360::Cabinet>,
    tracking: Option<tracking::Tracker>,
}

pub struct Bus {
    firmware: Box<[u8]>,
    state: BusState,
    // Delivered output observation, not native state. Bounded and drained by
    // frontends so an On/Off pulse within one frame is not lost.
    motor_seen_on: bool,
}

impl Bus {
    pub fn new(firmware: &[u8], eeprom: Eeprom93c46) -> Result<Self, BusError> {
        if firmware.len() != FIRMWARE_SIZE {
            return Err(BusError::FirmwareSize(firmware.len()));
        }
        Ok(Self {
            firmware: firmware.into(),
            motor_seen_on: false,
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
                lcd: lcd::Lcd::default(),
                r360: None,
                tracking: None,
            },
        })
    }

    pub fn set_inputs(&mut self, mut inputs: Inputs) {
        if let Some(tracker) = &mut self.state.tracking {
            // JP4 ROM_EMU on, JP3 MODE off: the firmware's Polhemus branch.
            inputs.board_switches = (inputs.board_switches & 0x0f) | 0x20;
            tracker.set_baud(inputs.dips[0]);
        }
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
    pub(super) fn take_netmerc_motor_activity(&mut self) -> bool {
        let observed = std::mem::take(&mut self.motor_seen_on);
        observed || self.state.outputs.lamps & 0x04 != 0
    }
    /// Visible character codes of the write-only diagnostic LCD, without a
    /// font renderer or GUI. Display-off returns spaces; custom glyphs are codes.
    pub fn diagnostic_lines(&self) -> [[u8; 20]; 2] {
        self.state.lcd.lines()
    }
    pub fn diagnostic_pixels(&self, cgrom: &[u8; 4096], blink_on: bool) -> DiagnosticPixels {
        self.state.lcd.pixels(cgrom, blink_on)
    }
    pub fn tracking_status(&self) -> Option<TrackingStatus> {
        self.state.tracking.as_ref().map(tracking::Tracker::status)
    }
    pub fn set_hmd_pose(&mut self, pose: HmdPose) -> bool {
        if let Some(tracker) = &mut self.state.tracking {
            tracker.set_pose(pose);
            true
        } else {
            false
        }
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
        self.motor_seen_on = false;
        Ok(())
    }

    pub(crate) fn validate_state(&self, state: &BusState) -> Result<(), BusError> {
        if !state.cpu_peripherals.valid_state()
            || !state.lcd.valid()
            || state.r360.is_some() != self.state.r360.is_some()
            || state.tracking.is_some() != self.state.tracking.is_some()
            || state
                .tracking
                .as_ref()
                .is_some_and(|tracker| !tracker.valid())
        {
            return Err(BusError::InvalidSnapshot);
        }
        Ok(())
    }

    /// Only resets the implemented CPU peripherals. It does not erase RAM,
    /// EEPROM, chip output latches or cabinet state, nor does it reset a CPU.
    pub fn reset_cpu_peripherals(&mut self) {
        self.state.cpu_peripherals.reset();
        self.motor_seen_on = false;
        if let Some(tracker) = &mut self.state.tracking {
            tracker.reset_measurement();
        }
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
                    let address = address as usize & (DPRAM_SIZE - 1);
                    // Keep the neutral startup pose until the NetMerc I/O
                    // firmware has decoded its first complete serial record.
                    let data = match (&self.state.tracking, address) {
                        (Some(tracker), 0x80..=0x8b) => {
                            tracker.publication_byte(address - 0x80, data)
                        }
                        _ => data,
                    };
                    self.state.dpram[address] = data
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
            0xe000..=0xffff => {
                let offset = (address - 0xe000) as usize;
                // A new record sync makes the binary decoder clear F08C
                // before decoding the preceding receive buffer at F400.
                if address == 0xf08c && data == 0 {
                    if let Some(tracker) = &mut self.state.tracking {
                        tracker.receive_boundary(self.state.ram[offset]);
                    }
                }
                // EPR-18021's two decoders commit by incrementing F096 after
                // writing F480..F48B. RAM clearing is not a record completion.
                if address == 0xf096 && data != 0 && data == self.state.ram[offset].wrapping_add(1)
                {
                    if let Some(tracker) = &mut self.state.tracking {
                        tracker.decoded_record(
                            &self.state.ram[0x1400..0x1414],
                            &self.state.ram[0x1480..0x148c],
                        );
                    }
                }
                self.state.ram[offset] = data;
            }
            _ => {}
        }
        Ok(())
    }

    fn output_port(&mut self, port: u8, data: u8) -> Result<(), BusError> {
        let state = &mut self.state;
        match port {
            3 => {
                if state.tracking.is_some() {
                    self.motor_seen_on |= (state.outputs.lamps | data) & 0x04 != 0;
                }
                state.outputs.lamps = data;
            }
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
                    // Same CN6 enable/E/RW qualification as MAME model1io2.
                    // The board has no LCD readback path; EEPROM pins still
                    // receive every port-F write independently of the panel.
                    state.lcd.write(data & 1 != 0, state.lcd_data);
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
        if self.state.tracking.is_none() {
            self.state.cpu_peripherals.advance(clocks, output);
            return;
        }
        self.advance_observed(clocks, output, |_, _, _| {});
    }

    fn advance_observed(
        &mut self,
        clocks: u32,
        mut output: impl FnMut(u32, u8, bool),
        mut serial: impl FnMut(u32, u8, SerialOutputs),
    ) {
        let Some(tracker) = &mut self.state.tracking else {
            self.state
                .cpu_peripherals
                .advance_observed(clocks, output, serial);
            return;
        };
        let peripherals = &mut self.state.cpu_peripherals;
        let mut offset = 0;
        while offset < clocks {
            let step = (clocks - offset).min(tracker.next_tick());
            peripherals.sio.inputs(
                0,
                SerialInputs {
                    rxd: tracker.tx(),
                    cts: false,
                    dcd: false,
                },
            );
            peripherals.advance_observed(
                step,
                |t, ch, high| output(offset + t, ch, high),
                |t, ch, pins| serial(offset + t, ch, pins),
            );
            tracker.advance(step, peripherals.sio.outputs(0).txd);
            offset += step;
        }
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
    fn netmerc_startup_publication_requires_complete_decoded_record_and_survives_state() {
        fn publish(bus: &mut Bus, bytes: [u8; 12]) -> [u8; 12] {
            for (offset, data) in bytes.into_iter().enumerate() {
                let address = 0x80 + offset;
                bus.write_memory(0x800a, (address >> 8) as u8).unwrap();
                bus.write_memory(0x8009, 1).unwrap();
                bus.write_memory(0x800a, address as u8).unwrap();
                bus.write_memory(0x8009, 0).unwrap();
                bus.write_memory(0x800a, data).unwrap();
                bus.write_memory(0x8009, 7).unwrap();
            }
            bus.dpram()[0x80..0x8c].try_into().unwrap()
        }
        fn bytes(pose: HmdPose) -> [u8; 12] {
            let words = pose.words();
            std::array::from_fn(|i| words[i / 2].to_le_bytes()[i % 2])
        }
        let mut bus = bus();
        bus.state.tracking = Some(Default::default());
        let neutral = bytes(HmdPose::default());
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        let waiting = bus.snapshot();
        // A first sync decodes an empty receive buffer: reject that completion.
        bus.write_memory(0xf096, 1).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        let zero = HmdPose {
            position: [0; 3],
            orientation: [0; 3],
        };
        let packet = tracking::encode(zero);
        // Only 19 bytes received: the decoded counter alone cannot release it.
        bus.state.ram[0x1400..0x1413].copy_from_slice(&packet[..19]);
        bus.state.ram[0x108c] = 19;
        bus.write_memory(0xf08c, 0).unwrap();
        bus.write_memory(0xf096, 2).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        bus.state.ram[0x1413] = packet[19];
        bus.state.ram[0x108c] = 20;
        bus.write_memory(0xf08c, 0).unwrap();
        let partial = bus.snapshot();
        assert_eq!(publish(&mut bus, [0; 12]), neutral); // Complete but not decoded.
        bus.write_memory(0xf096, 3).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), [0; 12]); // Legitimate zero pose.
        let ready = bus.snapshot();
        // Readiness is latched even if the firmware's byte counter wraps.
        bus.state.ram[0x1096] = 255;
        bus.write_memory(0xf096, 0).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), [0; 12]);
        bus.restore(&waiting).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        bus.restore(&partial).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        bus.write_memory(0xf096, 3).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), [0; 12]);
        bus.restore(&ready).unwrap();
        assert_eq!(publish(&mut bus, [0; 12]), [0; 12]);
        bus.reset_cpu_peripherals();
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        bus.write_memory(0xf096, 0).unwrap(); // Reset clearing retains fallback.
        assert_eq!(publish(&mut bus, [0; 12]), neutral);
        let mut ordinary = self::bus();
        assert_eq!(publish(&mut ordinary, [0; 12]), [0; 12]);
    }

    #[test]
    fn netmerc_motor_observation_ignores_lcd_and_reset_clears_pending_pulses() {
        let mut bus = bus();
        bus.state.tracking = Some(Default::default());
        bus.write_memory(0x8004, 0xff).unwrap(); // LCD data includes bit 2.
        bus.write_memory(0x8005, 0x10).unwrap(); // LCD select/enable traffic.
        assert!(!bus.take_netmerc_motor_activity());
        bus.write_memory(0x8003, 4).unwrap();
        assert!(bus.take_netmerc_motor_activity());
        assert!(bus.take_netmerc_motor_activity()); // Native held On latch.
        bus.write_memory(0x8003, 0).unwrap();
        assert!(bus.take_netmerc_motor_activity()); // On at start of this interval.
        assert!(!bus.take_netmerc_motor_activity());
        let off = bus.snapshot();
        bus.write_memory(0x8003, 4).unwrap();
        bus.write_memory(0x8003, 0).unwrap();
        bus.restore(&off).unwrap();
        assert!(!bus.take_netmerc_motor_activity());
        bus.write_memory(0x8003, 4).unwrap();
        bus.write_memory(0x8003, 0).unwrap();
        bus.reset_cpu_peripherals();
        assert!(!bus.take_netmerc_motor_activity());
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
    }

    #[test]
    fn diagnostic_lcd_qualifies_cn6_writes_and_preserves_eeprom_pins() {
        let mut bus = bus();
        let write = |bus: &mut Bus, rs: bool, value| {
            bus.write_memory(0x8004, value).unwrap();
            bus.write_memory(0x8005, 4 | u8::from(rs)).unwrap();
        };
        write(&mut bus, false, 0x38);
        write(&mut bus, false, 0x0c);
        for value in b"NETMERC" {
            write(&mut bus, true, *value);
        }
        for disabled in [0x0d, 0x07, 0x01] {
            bus.write_memory(0x8004, b'!').unwrap();
            bus.write_memory(0x8005, disabled).unwrap();
        }
        assert_eq!(&bus.diagnostic_lines()[0][..8], b"NETMERC ");
        let snapshot = bincode::deserialize(&bincode::serialize(&bus.snapshot()).unwrap()).unwrap();
        let mut restored = self::bus();
        restored.restore(&snapshot).unwrap();
        write(&mut bus, true, b'!');
        write(&mut restored, true, b'!');
        assert_eq!(
            bincode::serialize(&bus.snapshot()).unwrap(),
            bincode::serialize(&restored.snapshot()).unwrap()
        );
        // LCD support must not steal the shared EEPROM DI/CLK/CS pins.
        let mut expected = bus.state.eeprom.clone();
        for pins in [0x55, 0x75] {
            bus.write_memory(0x8005, pins).unwrap();
            expected.clk_write(pins & 0x20 != 0);
            expected.di_write(pins & 0x40 != 0);
            expected.cs_write(pins & 0x10 != 0);
        }
        assert_eq!(
            bincode::serialize(&bus.state.eeprom).unwrap(),
            bincode::serialize(&expected).unwrap()
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
