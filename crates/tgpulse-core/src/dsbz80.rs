//! Sega Z80 DSB bus/CPU and clocked MPEG output, used by Model 1 SWA/SWAJ.
//! Map/register reference: MAME dsbz80, BSD-3-Clause, R. Belmont / O. Galibert.
//! CPU clock is MAME's estimate, not a verified oscillator measurement.
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use z80::{CpuState, Z80_io, Z80};
mod audio;
use crate::i8251 as uart;
pub use audio::SAMPLE_RATE;

pub const CPU_HZ: u32 = 4_000_000;
pub const FIRMWARE_SIZE: usize = 0x20000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    FirmwareSize,
    UnsupportedUartMode(u8),
    UnsupportedUartCommand(u8),
    TransmitFull,
    UnsupportedPlayback(u8),
    UnsupportedPan(u8),
    InvalidSnapshot,
    MpegRegionSize,
    Mpeg(crate::mpeg::Error),
    MpegFormat { channels: usize, rate: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DSB: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<uart::Error> for Error {
    fn from(error: uart::Error) -> Self {
        match error {
            uart::Error::UnsupportedUartMode(v) => Self::UnsupportedUartMode(v),
            uart::Error::UnsupportedUartCommand(v) => Self::UnsupportedUartCommand(v),
            uart::Error::TransmitFull => Self::TransmitFull,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playback {
    pub start: u32,
    pub end: u32,
    pub loop_start: u32,
    pub loop_end: u32,
    pub bit_position: u32,
    pub mode: u8,
    pub volume: u8,
    pub pan: u8,
    start_latch: u32,
    end_latch: u32,
}
impl Default for Playback {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            loop_start: 0,
            loop_end: 0,
            bit_position: 0,
            mode: 0,
            volume: 127,
            pan: 0,
            start_latch: 0,
            end_latch: 0,
        }
    }
}
impl Playback {
    fn write(&mut self, port: u8, value: u8) -> Result<(), Error> {
        match port {
            0xe0 => {
                if value > 2 {
                    return Err(Error::UnsupportedPlayback(value));
                }
                self.mode = value;
                if value != 0 {
                    self.bit_position = self.start * 8;
                }
            }
            0xe2..=0xe7 => {
                let start = port <= 0xe4;
                let offset = if start { port - 0xe2 } else { port - 0xe5 };
                let latch = if start {
                    &mut self.start_latch
                } else {
                    &mut self.end_latch
                };
                let shift = (2 - offset) * 8;
                *latch = (*latch & !(0xff << shift)) | (u32::from(value) << shift);
                if offset == 2 {
                    match (start, self.mode == 0) {
                        (true, true) => self.start = *latch,
                        (false, true) => self.end = *latch,
                        (true, false) => self.loop_start = *latch,
                        (false, false) => self.loop_end = *latch,
                    }
                }
            }
            0xe8 => self.volume = !value & 0x7f,
            0xe9 => {
                if value & 3 == 3 {
                    return Err(Error::UnsupportedPan(value));
                }
                self.pan = value & 3;
            }
            _ => {}
        }
        Ok(())
    }
    fn valid(&self) -> bool {
        [
            self.start,
            self.end,
            self.loop_start,
            self.loop_end,
            self.start_latch,
            self.end_latch,
        ]
        .iter()
        .all(|&x| x <= 0xffffff)
            && self.bit_position <= 0x7ffffff
            && self.mode <= 2
            && self.pan <= 2
            && self.volume <= 127
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct BusState {
    ram: Vec<u8>,
    uart: uart::Uart,
    playback: Playback,
    clock_phase: u8,
    /// Optional 68000-board transmitter, sharing the 500 kHz wire clock.
    sender: Option<uart::Uart>,
    sound_fraction: u8,
    transmitted: u64,
    received: u64,
}
struct Bus {
    rom: Vec<u8>,
    state: RefCell<BusState>,
    fault: Cell<Option<Error>>,
    elapsed: u64,
    events: Vec<TxEvent>,
    audio: Option<audio::Audio>,
    samples: Vec<AudioSample>,
}
/// Normalized stereo output at the emulated 32 kHz clock. Delivery is immediate
/// after each CPU instruction; the receiver may mix, capture or discard it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioSample {
    pub clock: u64,
    pub channels: [f32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TxEvent {
    pub clock: u64,
    pub level: bool,
}
impl Bus {
    fn latch(&self, result: Result<(), Error>) {
        if self.fault.get().is_none() {
            self.fault.set(result.err());
        }
    }
}
impl Z80_io for Bus {
    fn read_byte(&self, address: u16) -> u8 {
        if address < 0x8000 {
            self.rom[address as usize]
        } else {
            self.state.borrow().ram[address as usize - 0x8000]
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        if address >= 0x8000 && self.fault.get().is_none() {
            self.state.get_mut().ram[address as usize - 0x8000] = value;
        }
    }
    fn port_in(&self, address: u16) -> u8 {
        let mut s = self.state.borrow_mut();
        match address as u8 {
            0xe2..=0xe4 => ((s.playback.bit_position >> 3) >> ((0xe4 - address as u8) * 8)) as u8,
            0xf0 => {
                if s.uart.status() & 2 != 0 {
                    s.received += 1;
                }
                s.uart.read()
            }
            0xf1 => s.uart.status(),
            _ => 0xff, // unmapped MAME reads
        }
    }
    fn port_out(&mut self, address: u16, value: u8) {
        if self.fault.get().is_some() {
            return;
        }
        let mut s = self.state.borrow_mut();
        let result = match address as u8 {
            0xf0 => s.uart.write(value).map_err(Error::from),
            0xf1 => s.uart.control(value).map_err(Error::from),
            port => s.playback.write(port, value),
        };
        if address as u8 == 0xe0 && value == 0 {
            if let Some(audio) = &mut self.audio {
                audio.stop();
            }
        }
        self.latch(result);
    }
    fn irq_pending(&self) -> bool {
        self.fault.get().is_none() && self.state.borrow().uart.irq()
    }
    fn irq_acknowledge(&mut self, _: u8) -> u8 {
        0xff
    }
    fn advance(&mut self, clocks: u32) {
        if self.fault.get().is_some() {
            return;
        }
        let s = self.state.get_mut();
        for _ in 0..clocks {
            self.elapsed += 1;
            if let Some(audio) = &mut self.audio {
                match audio.tick(&mut s.playback) {
                    Ok(Some(channels)) => self.samples.push(AudioSample {
                        clock: self.elapsed,
                        channels,
                    }),
                    Ok(None) => {}
                    Err(e) => {
                        self.fault.set(Some(e));
                        return;
                    }
                }
            }
            s.clock_phase += 1;
            if s.clock_phase == 8 {
                s.clock_phase = 0;
                if let Some(sender) = &mut s.sender {
                    sender.tick();
                    s.uart.rx = sender.tx;
                }
                let before = s.uart.tx;
                s.uart.tick();
                if s.uart.tx != before {
                    self.events.push(TxEvent {
                        clock: self.elapsed,
                        level: s.uart.tx,
                    });
                }
            }
        }
    }
}

/// Internal state, same firmware required; not a full-machine save format.
#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    cpu: CpuState,
    bus: BusState,
    debt: i64,
    elapsed: u64,
    fault: Option<Error>,
    audio: Option<audio::State>,
}
impl State {
    pub(crate) fn executed_ticks(&self) -> Option<u64> {
        self.elapsed.checked_mul(5)
    }
    /// Requested sound time in 20 MHz ticks, including fractional conversion
    /// and subtracting CPU instruction overshoot. Not wall-clock time.
    pub(crate) fn sound_ticks(&self) -> Option<u64> {
        u64::try_from(
            (i128::from(self.elapsed) + i128::from(self.debt)) * 5
                + i128::from(self.bus.sound_fraction),
        )
        .ok()
    }
}
pub struct Board {
    cpu: Z80<Bus>,
    debt: i64,
}
impl Board {
    pub fn new(firmware: &[u8]) -> Result<Self, Error> {
        if firmware.len() != FIRMWARE_SIZE {
            return Err(Error::FirmwareSize);
        }
        let mut cpu = Z80::new(Bus {
            rom: firmware.to_vec(),
            state: RefCell::new(BusState {
                ram: vec![0; 0x8000],
                uart: uart::Uart::default(),
                playback: Playback::default(),
                clock_phase: 0,
                sender: None,
                sound_fraction: 0,
                transmitted: 0,
                received: 0,
            }),
            fault: Cell::new(None),
            elapsed: 0,
            events: vec![],
            audio: None,
            samples: vec![],
        });
        cpu.init();
        Ok(Self { cpu, debt: 0 })
    }
    /// Own immutable MPEG resources. No loader, filesystem or host audio device.
    pub fn with_mpeg(firmware: &[u8], mpeg: Vec<u8>) -> Result<Self, Error> {
        let mut board = Self::new(firmware)?;
        board.cpu.io.audio = Some(audio::Audio::new(mpeg)?);
        Ok(board)
    }
    /// Instruction-boundary pin changes; the future producer scheduler must
    /// account for carried instruction overshoot, not deliver whole-frame bursts.
    pub fn set_rx(&mut self, level: bool) {
        self.cpu.io.state.get_mut().uart.rx = level;
    }
    pub fn tx(&self) -> bool {
        self.cpu.io.state.borrow().uart.tx
    }
    pub fn cpu_state(&self) -> CpuState {
        self.cpu.snapshot()
    }
    pub fn playback(&self) -> Playback {
        self.cpu.io.state.borrow().playback.clone()
    }
    /// Filtered wire from the 68000 board, connected before its first instruction.
    pub fn connect_sender(&mut self) {
        self.cpu.io.state.get_mut().sender = Some(uart::Uart::default());
    }
    /// Use an externally clocked physical transmitter instead of the fixture sender.
    pub(crate) fn disconnect_sender(&mut self) {
        self.cpu.io.state.get_mut().sender = None;
    }
    pub(crate) fn note_external_transmit(&mut self) {
        self.cpu.io.state.get_mut().transmitted += 1;
    }
    pub fn sender_control(&mut self, value: u8) {
        if self.fault().is_some() {
            return;
        }
        let result = self
            .cpu
            .io
            .state
            .get_mut()
            .sender
            .as_mut()
            .map_or(Ok(()), |s| s.control(value));
        self.cpu.io.latch(result.map_err(Error::from));
    }
    pub fn sender_write(&mut self, value: u8) {
        if self.fault().is_some() {
            return;
        }
        let s = self.cpu.io.state.get_mut();
        let result = if let Some(sender) = &mut s.sender {
            let result = sender.write(value);
            if result.is_ok() {
                s.transmitted += 1;
            }
            result
        } else {
            Ok(())
        };
        log::trace!(target: "dsb", "68000 TX at DSB clock {}: {value:02x}", self.cpu.io.elapsed);
        self.cpu.io.latch(result.map_err(Error::from));
    }
    pub fn sender_status(&self) -> u8 {
        self.cpu
            .io
            .state
            .borrow()
            .sender
            .as_ref()
            .map_or(5, |s| s.status() & 5)
    }
    pub fn diagnostics(&self) -> (u64, u64, u8) {
        let s = self.cpu.io.state.borrow();
        (s.transmitted, s.received, s.uart.status())
    }
    pub fn fault(&self) -> Option<Error> {
        self.cpu.io.fault.get()
    }
    /// Advance from 10 MHz sound-CPU instruction clocks. Retain the exact 2/5
    /// ratio and Z80 instruction debt; bus writes occur at instruction boundaries.
    pub fn run_sound_cycles(&mut self, clocks: u32) -> Result<(), Error> {
        self.run_sound_cycles_with_audio(clocks, |_| {})
    }
    pub fn run_sound_cycles_with_audio(
        &mut self,
        clocks: u32,
        audio_output: impl FnMut(AudioSample),
    ) -> Result<(), Error> {
        if let Some(e) = self.fault() {
            return Err(e);
        }
        let s = self.cpu.io.state.get_mut();
        let scaled = u64::from(clocks) * 2 + u64::from(s.sound_fraction);
        s.sound_fraction = (scaled % 5) as u8;
        self.run_with_audio((scaled / 5) as u32, |_| {}, audio_output)
    }
    pub fn run(&mut self, clocks: u32, mut output: impl FnMut(TxEvent)) -> Result<(), Error> {
        self.run_with_audio(clocks, &mut output, |_| {})
    }
    /// Advance hardware and deliver both wire edges and audio in memory. Calling
    /// `run` instead discards output, but does not stop decoding or clocks.
    pub fn run_with_audio(
        &mut self,
        clocks: u32,
        mut output: impl FnMut(TxEvent),
        mut audio_output: impl FnMut(AudioSample),
    ) -> Result<(), Error> {
        if let Some(e) = self.cpu.io.fault.get() {
            return Err(e);
        }
        self.debt += i64::from(clocks);
        while self.debt > 0 {
            self.debt -= i64::from(self.cpu.step());
            for e in self.cpu.io.events.drain(..) {
                output(e);
            }
            for sample in self.cpu.io.samples.drain(..) {
                audio_output(sample);
            }
            if let Some(e) = self.cpu.io.fault.get() {
                return Err(e);
            }
        }
        Ok(())
    }
    /// Soft reset preserves RAM, elapsed clock and the reference's retained
    /// playback addresses/pan, but stops playback and drops pending pin events.
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cpu.clr_irq();
        self.cpu.clr_nmi();
        let s = self.cpu.io.state.get_mut();
        s.uart = uart::Uart::default();
        s.playback.start_latch = 0;
        s.playback.end_latch = 0;
        s.playback.volume = 127;
        s.playback.mode = 0;
        s.clock_phase = 0;
        s.sound_fraction = 0;
        if s.sender.is_some() {
            s.sender = Some(uart::Uart::default());
        }
        s.transmitted = 0;
        s.received = 0;
        self.cpu.io.events.clear();
        self.cpu.io.samples.clear();
        if let Some(audio) = &mut self.cpu.io.audio {
            audio.reset();
        }
        self.cpu.io.fault.set(None);
        self.debt = 0;
    }
    pub fn snapshot(&self) -> State {
        State {
            cpu: self.cpu.snapshot(),
            bus: self.cpu.io.state.borrow().clone(),
            debt: self.debt,
            elapsed: self.cpu.io.elapsed,
            fault: self.cpu.io.fault.get(),
            audio: self.cpu.io.audio.as_ref().map(|a| a.snapshot()),
        }
    }
    pub fn restore(&mut self, state: &State) -> Result<(), Error> {
        if !state.cpu.is_valid()
            || state.bus.ram.len() != 0x8000
            || !state.bus.uart.valid()
            || !state.bus.playback.valid()
            || state.bus.clock_phase >= 8
            || state.bus.sound_fraction >= 5
            || state.bus.sender.as_ref().is_some_and(|s| !s.valid())
            || state.debt < -i64::from(u32::MAX)
            || state.debt > i64::from(u32::MAX)
            || (state.fault.is_none() && state.debt > 0)
        {
            return Err(Error::InvalidSnapshot);
        }
        let decoder = match (&self.cpu.io.audio, &state.audio) {
            (Some(audio), Some(s)) => Some(audio.validate(s)?),
            (None, None) => None,
            _ => return Err(Error::InvalidSnapshot),
        };
        self.cpu.restore(&state.cpu);
        *self.cpu.io.state.get_mut() = state.bus.clone();
        self.debt = state.debt;
        self.cpu.io.elapsed = state.elapsed;
        self.cpu.io.fault.set(state.fault);
        self.cpu.io.events.clear();
        self.cpu.io.samples.clear();
        if let (Some(audio), Some(s), Some(decoder)) =
            (&mut self.cpu.io.audio, &state.audio, decoder)
        {
            audio.restore_validated(s, decoder);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
