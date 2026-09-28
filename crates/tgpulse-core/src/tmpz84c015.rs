// license: BSD-3-Clause
// Reference: MAME src/devices/cpu/z80/tmpz84c015.cpp, copyright-holder hap.
// Daisy-chain arbitration reference: src/devices/machine/z80daisy.cpp,
// copyright-holder Juergen Buchmueller.
// See LICENSES/MAME-BSD-3-Clause.txt.
//! Implemented TMPZ84C015 peripherals, not a substitute CPU core.
//! Unsupported SIO/PIO modes fail explicitly at the register boundary.

use crate::z80ctc::{Ctc, IrqState};
use crate::z80pio::Pio;
use crate::z80sio::{SerialOutputs, Sio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InterruptSource {
    Ctc,
    Sio,
    Pio,
}

const ORDERS: [[InterruptSource; 3]; 6] = {
    use InterruptSource::*;
    [
        [Ctc, Sio, Pio],
        [Sio, Ctc, Pio],
        [Ctc, Pio, Sio],
        [Pio, Sio, Ctc],
        [Pio, Ctc, Sio],
        [Sio, Pio, Ctc],
    ]
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Peripherals {
    pub ctc: Ctc,
    pub sio: Sio,
    pub pio: Pio,
    pio_pins: [u8; 2],
    priority: u8,
    watchdog_mode: u8,
    watchdog_remaining: Option<u32>,
    watchdog_output: bool,
}

impl Default for Peripherals {
    fn default() -> Self {
        let mut this = Self {
            ctc: Ctc::default(),
            sio: Sio::default(),
            pio: Pio::default(),
            pio_pins: [0xff; 2],
            priority: 0,
            watchdog_mode: 0xfb,
            watchdog_remaining: None,
            watchdog_output: false,
        };
        this.clear_watchdog();
        this
    }
}

impl Peripherals {
    pub(crate) fn valid_state(&self) -> bool {
        self.priority < 6
            && self.ctc.valid_state()
            && self.sio.valid_state()
            && self.pio.valid_state()
            && self
                .watchdog_remaining
                .is_none_or(|remaining| (1..=0x400000).contains(&remaining))
    }

    pub(crate) fn reset(&mut self) {
        self.ctc.reset();
        self.sio.reset();
        self.pio.reset();
        self.priority = 0;
        self.watchdog_mode = 0xfb;
        self.clear_watchdog();
    }

    /// None means an unsupported SIO/PIO operation. Unmapped
    /// reads are open bus, not an invented peripheral-ready status.
    pub(crate) fn read(&mut self, address: u16) -> Option<u8> {
        match address as u8 {
            0x10..=0x13 => Some(self.ctc.read(address as u8)),
            0x18..=0x1b => self.sio.read(address as u8),
            0x1c..=0x1f => Some(self.pio.read(address as u8, self.pio_pins)),
            0xf0 => Some(self.watchdog_mode),
            _ => Some(0xff),
        }
    }

    pub(crate) fn write(&mut self, address: u16, data: u8) -> Option<()> {
        match address as u8 {
            0x10..=0x13 => self.ctc.write(address as u8, data),
            0x18..=0x1b => {
                if !self.sio.write(address as u8, data) {
                    return None;
                }
            }
            0x1c..=0x1f => {
                if !self.pio.write(address as u8, data) {
                    return None;
                }
            }
            0xf0 => {
                let old = self.watchdog_mode;
                self.watchdog_mode = data;
                if old & 0x80 == 0 && data & 0x80 != 0 {
                    self.clear_watchdog();
                }
            }
            0xf1 => match data {
                0x4e => self.clear_watchdog(),
                0xb1 if self.watchdog_mode & 0x80 == 0 => self.watchdog_remaining = None,
                _ => {}
            },
            0xf4 => {
                let value = data & 7;
                // MAME's documented guess for reserved priorities 6 and 7.
                self.priority = if value > 5 { value & 3 } else { value };
            }
            _ => {}
        }
        Some(())
    }

    fn clear_watchdog(&mut self) {
        self.watchdog_output = false;
        if self.watchdog_mode & 0x80 != 0 {
            self.watchdog_remaining = Some(0x10000 << (((self.watchdog_mode >> 5) & 3) * 2));
        }
    }

    pub(crate) fn set_pio_inputs(&mut self, pins: [u8; 2]) {
        for (index, value) in pins.into_iter().enumerate() {
            if self.pio_pins[index] != value {
                self.pio.pins(index, value);
            }
        }
        self.pio_pins = pins;
    }

    pub(crate) fn advance(&mut self, clocks: u32, mut output: impl FnMut(u32, u8, bool)) {
        self.advance_observed(clocks, &mut output, |_, _, _| {});
    }

    pub(crate) fn advance_observed(
        &mut self,
        clocks: u32,
        mut output: impl FnMut(u32, u8, bool),
        mut serial: impl FnMut(u32, u8, SerialOutputs),
    ) {
        let sio = &mut self.sio;
        self.ctc.advance(clocks, |offset, channel, level| {
            // Preserve chronological edges, including the one-clock low edge.
            if channel >= 2 {
                let index = (channel - 2) as usize;
                let before = sio.outputs(index);
                sio.clock(index, level);
                let after = sio.outputs(index);
                if after != before {
                    serial(offset, channel - 2, after);
                }
            }
            output(offset, channel, level);
        });
        if let Some(remaining) = self.watchdog_remaining {
            if clocks >= remaining {
                self.watchdog_remaining = None;
                self.watchdog_output = true;
            } else {
                self.watchdog_remaining = Some(remaining - clocks);
            }
        }
    }

    /// WDTOUT is a pin, not an unconditional CPU reset/NMI.
    pub(crate) fn watchdog_output(&self) -> bool {
        self.watchdog_output
    }

    fn state(&self, source: InterruptSource, sio: IrqState, pio: IrqState) -> IrqState {
        match source {
            InterruptSource::Ctc => self.ctc.irq_state(),
            InterruptSource::Sio => sio,
            InterruptSource::Pio => pio,
        }
    }

    /// SIO/PIO states will be supplied by their devices, not stored as fake
    /// ready flags here. Higher-priority INT wins over a lower device's IEO.
    pub(crate) fn interrupt_source(&self, sio: IrqState, pio: IrqState) -> Option<InterruptSource> {
        for source in ORDERS[self.priority as usize] {
            let state = self.state(source, sio, pio);
            if state.requested {
                return Some(source);
            }
            if state.in_service {
                break;
            }
        }
        None
    }

    pub(crate) fn reti_source(&self, sio: IrqState, pio: IrqState) -> Option<InterruptSource> {
        ORDERS[self.priority as usize]
            .into_iter()
            .find(|&source| self.state(source, sio, pio).in_service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_ctc(p: &mut Peripherals) {
        p.write(0x10, 0x85).unwrap();
        p.write(0x10, 1).unwrap();
        p.advance(16, |_, _, _| {});
    }

    #[test]
    fn io_decode_mirrors_high_byte_without_aliasing_other_low_ports() {
        let mut p = Peripherals::default();
        p.write(0xab10, 0x85).unwrap();
        p.write(0xcd10, 2).unwrap();
        assert_eq!(p.read(0x0010), Some(3));
        assert_eq!(p.read(0xff10), Some(3));
        assert_eq!(p.read(0xff14), Some(0xff));
        assert_eq!(p.read(0xa019), Some(0x54)); // reset SIO status, not fake-ready
        assert_eq!(p.read(0xa01c), Some(0xff)); // DIP pins
        assert_eq!(p.write(0xb01d, 0x8f), None); // unsupported PIO mode
        assert_eq!(p.write(0xb019, 3), Some(()));
        assert_eq!(p.write(0xb019, 1), None); // synchronous RX enable
        assert_eq!(p.read(0xfff0), Some(0xfb));
        assert_eq!(p.read(0xfff1), Some(0xff));
        assert_eq!(p.read(0xfff4), Some(0xff));
    }

    #[test]
    fn all_six_irq_orders_and_reserved_values_match_reference() {
        use InterruptSource::*;
        let requested = IrqState {
            requested: true,
            in_service: false,
        };
        for (priority, expected) in [Ctc, Sio, Ctc, Pio, Pio, Sio, Ctc, Pio]
            .into_iter()
            .enumerate()
        {
            let mut p = Peripherals::default();
            request_ctc(&mut p);
            p.write(0xf4, priority as u8).unwrap();
            assert_eq!(p.interrupt_source(requested, requested), Some(expected));
        }
    }

    #[test]
    fn higher_priority_service_blocks_lower_requests_and_reti_targets_service() {
        let mut p = Peripherals::default();
        request_ctc(&mut p);
        p.write(0xf4, 1).unwrap(); // SIO > CTC > PIO
        let service = IrqState {
            requested: false,
            in_service: true,
        };
        assert_eq!(p.interrupt_source(service, IrqState::default()), None);
        assert_eq!(
            p.reti_source(service, IrqState::default()),
            Some(InterruptSource::Sio)
        );
        p.write(0xf4, 0).unwrap(); // CTC can now preempt SIO
        assert_eq!(
            p.interrupt_source(service, IrqState::default()),
            Some(InterruptSource::Ctc)
        );
        p.ctc.acknowledge();
        assert_eq!(
            p.reti_source(service, IrqState::default()),
            Some(InterruptSource::Ctc)
        );
        p.ctc.reti();
        assert_eq!(
            p.reti_source(service, IrqState::default()),
            Some(InterruptSource::Sio)
        );
    }

    #[test]
    fn watchdog_timeout_is_one_shot_and_requires_enable_then_clear_sequence() {
        let mut p = Peripherals::default();
        p.advance(0x400000 - 1, |_, _, _| {});
        assert!(!p.watchdog_output());
        p.advance(1, |_, _, _| {});
        assert!(p.watchdog_output());
        p.write(0xf1, 0x4e).unwrap();
        assert!(!p.watchdog_output());
        p.write(0xf1, 0xb1).unwrap(); // ignored while enabled
        p.advance(0x400000, |_, _, _| {});
        assert!(p.watchdog_output());
        p.write(0xf0, 0x7b).unwrap();
        p.write(0xf1, 0xb1).unwrap();
        p.write(0xf1, 0x4e).unwrap();
        p.advance(0x800000, |_, _, _| {});
        assert!(!p.watchdog_output());
        p.write(0xf0, 0x83).unwrap(); // 0 -> 1 enable, shortest period
        p.advance(0xffff, |_, _, _| {});
        assert!(!p.watchdog_output());
        p.advance(1, |_, _, _| {});
        assert!(p.watchdog_output());
        p.reset();
        assert!(!p.watchdog_output());
        assert_eq!(p.read(0xf0), Some(0xfb));
    }

    #[test]
    fn watchdog_disable_bit_alone_does_not_cancel_running_timer() {
        let mut p = Peripherals::default();
        p.write(0xf0, 0x7b).unwrap();
        p.advance(0x400000, |_, _, _| {});
        assert!(p.watchdog_output());
    }

    #[test]
    fn snapshot_preserves_priority_watchdog_deadline_and_ctc_phase() {
        let mut p = Peripherals::default();
        request_ctc(&mut p);
        p.write(0x10, 3).unwrap(); // stop CTC, no long event stream
        p.write(0xf4, 5).unwrap();
        p.advance(999, |_, _, _| {});
        let mut copy: Peripherals = bincode::deserialize(&bincode::serialize(&p).unwrap()).unwrap();
        p.advance(0x400000 - 1015, |_, _, _| {});
        copy.advance(0x400000 - 1015, |_, _, _| {});
        assert!(copy.watchdog_output());
        assert_eq!(
            bincode::serialize(&p).unwrap(),
            bincode::serialize(&copy).unwrap()
        );
    }

    #[test]
    fn invalid_priority_or_watchdog_deadline_is_rejected_before_restore() {
        let mut p = Peripherals::default();
        assert!(p.valid_state());
        p.priority = 6;
        assert!(!p.valid_state());
        p.priority = 0;
        p.watchdog_remaining = Some(0);
        assert!(!p.valid_state());
        p.watchdog_remaining = Some(0x400001);
        assert!(!p.valid_state());
    }
}
