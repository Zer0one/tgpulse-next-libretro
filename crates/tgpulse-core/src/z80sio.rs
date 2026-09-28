// license: BSD-3-Clause
// Reference: MAME src/devices/machine/z80sio.cpp,
// copyright-holders Curt Coder, Joakim Larsson Edstrom.
// See LICENSES/MAME-BSD-3-Clause.txt.
//! Bounded asynchronous SIO. Sync/SDLC, DMA and five-bit TX are not implemented.
//! Pins are logical electrical levels (CTS/DCD/RTS/DTR are active low).
use crate::z80ctc::IrqState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SerialInputs {
    pub rxd: bool,
    pub cts: bool,
    pub dcd: bool,
}
impl Default for SerialInputs {
    fn default() -> Self {
        Self {
            rxd: true,
            cts: true,
            dcd: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SerialOutputs {
    pub txd: bool,
    pub rts: bool,
    pub dtr: bool,
}

const PRIORITY: [usize; 6] = [2, 0, 1, 5, 3, 4]; // A RX/TX/EXT, B RX/TX/EXT

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Channel {
    wr: [u8; 8],
    rr1_errors: u8,
    inputs: SerialInputs,
    clock: bool,
    ext_latch: Option<u8>,
    ext_changed: bool,
    pending: [bool; 3],
    rx_data: [u8; 3],
    rx_errors: [u8; 3],
    rx_len: u8,
    rx_first: bool,
    rx_bit: u8,
    rx_count: u8,
    rx_shift: u16,
    break_seen: bool,
    tx_hold: Option<u8>,
    tx_frame: u16,
    tx_bits: u8,
    tx_left: u8,
    tx_count: u16,
    tx_stop: u16,
    txd: bool,
    tx_disarm: bool,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            wr: [0; 8],
            rr1_errors: 0,
            inputs: SerialInputs::default(),
            clock: false,
            ext_latch: None,
            ext_changed: false,
            pending: [false; 3],
            rx_data: [0; 3],
            rx_errors: [0; 3],
            rx_len: 0,
            rx_first: false,
            rx_bit: 0,
            rx_count: 0,
            rx_shift: 0,
            break_seen: false,
            tx_hold: None,
            tx_frame: 0,
            tx_bits: 0,
            tx_left: 0,
            tx_count: 0,
            tx_stop: 0,
            txd: true,
            tx_disarm: false,
        }
    }
}
impl Channel {
    fn divisor(&self) -> u16 {
        [1, 16, 32, 64][(self.wr[4] >> 6) as usize]
    }
    fn rx_width(&self) -> u8 {
        [5, 7, 6, 8][(self.wr[3] >> 6) as usize]
    }
    fn tx_width(&self) -> u8 {
        [5, 7, 6, 8][((self.wr[5] >> 5) & 3) as usize]
    }
    fn rx_enabled(&self) -> bool {
        self.wr[3] & 1 != 0 && (self.wr[3] & 0x20 == 0 || !self.inputs.dcd)
    }
    fn tx_enabled(&self) -> bool {
        self.wr[5] & 8 != 0 && (self.wr[3] & 0x20 == 0 || !self.inputs.cts)
    }
    fn all_sent(&self) -> bool {
        self.tx_hold.is_none() && self.tx_bits == 0 && self.tx_stop == 0
    }
    fn supported(&self) -> bool {
        self.wr[1] & 0x80 == 0 // no Wait/Ready/DMA integration
            && ((self.wr[3] & 1 == 0 && self.wr[5] & 8 == 0) || self.wr[4] & 12 != 0)
            && (self.wr[5] & 8 == 0 || (self.tx_width() != 5 && !(self.divisor() == 1 && self.wr[4] & 12 == 8)))
    }
    fn ext_status(&self) -> u8 {
        (u8::from(!self.inputs.cts) << 5)
            | (u8::from(!self.inputs.dcd) << 3)
            | if self.wr[4] & 12 == 0 { 0x10 } else { 0 }
            | (u8::from(self.break_seen) << 7)
    }
    fn rr0(&self) -> u8 {
        0x40 | self.ext_latch.unwrap_or_else(|| self.ext_status())
            | u8::from(self.rx_len != 0)
            | (u8::from(self.tx_hold.is_none()) << 2)
    }
    fn special_mask(&self) -> u8 {
        match self.wr[1] & 0x18 {
            0 => 0,
            0x10 => 0x70,
            _ => 0x60,
        }
    }
    fn update_rx_irq(&mut self) {
        let mode = self.wr[1] & 0x18;
        self.pending[2] = mode != 0
            && self.rx_len != 0
            && (mode != 8
                || self.rx_errors[0] & 1 != 0
                || self.rr1_errors & self.special_mask() != 0);
    }
    fn ext_event(&mut self) {
        if self.ext_latch.is_none() {
            self.ext_latch = Some(self.ext_status());
        } else {
            self.ext_changed = true;
        }
        if self.wr[1] & 1 != 0 {
            self.pending[1] = true;
        }
    }
    fn reset_ext(&mut self) {
        let changed = self.ext_changed;
        self.ext_latch = None;
        self.ext_changed = false;
        self.pending[1] = false;
        if changed {
            self.ext_event();
        }
    }
    fn queue_rx(&mut self, data: u8, mut error: u8) {
        if self.wr[1] & 0x18 == 8 && self.rx_first {
            error |= 1;
            self.rx_first = false;
        }
        let index = if self.rx_len == 3 {
            error |= 0x20;
            2
        } else {
            let i = self.rx_len;
            self.rx_len += 1;
            i as usize
        };
        self.rx_data[index] = data;
        self.rx_errors[index] = error;
        if index == 0 {
            self.rr1_errors = (self.rr1_errors & !0x40) | (error & 0x70);
        }
        self.update_rx_irq();
    }
    fn pop_rx(&mut self) {
        if self.rx_len != 0 {
            self.rx_len -= 1;
            if self.rx_len != 0 {
                self.rx_data.rotate_left(1);
                self.rx_errors.rotate_left(1);
                self.rr1_errors = (self.rr1_errors & !0x40) | (self.rx_errors[0] & 0x70);
            }
        }
        self.update_rx_irq();
    }
    fn receive_clock(&mut self) {
        if !self.rx_enabled() {
            return;
        }
        let clocks = (self.divisor() - 1) as u8;
        if self.break_seen {
            if !self.inputs.rxd {
                return;
            }
            self.break_seen = false;
            self.ext_event();
        }
        if self.rx_bit == 0 {
            if self.inputs.rxd {
                self.rx_count = self.rx_count.max(clocks / 2 + 1) - 1;
            } else if self.rx_count != 0 {
                self.rx_count -= 1;
            } else {
                self.rx_count = clocks;
                self.rx_bit = 1;
                self.rx_shift = 0xffff;
            }
        } else if self.rx_count != 0 {
            self.rx_count -= 1;
        } else {
            if !self.inputs.rxd {
                self.rx_shift &= !(1 << (self.rx_bit - 1));
            }
            let width = self.rx_width();
            let parity = self.wr[4] & 1 != 0;
            let bits = width + u8::from(parity);
            if self.rx_bit == bits + 1 {
                let stop = 1u16 << bits;
                let brk = self.rx_shift & ((stop << 1) - 1) == 0;
                let mut error = if brk || self.inputs.rxd { 0 } else { 0x40 };
                if parity {
                    let odd = (self.rx_shift & ((1 << (width + 1)) - 1)).count_ones() & 1 != 0;
                    if odd == (self.wr[4] & 2 != 0) {
                        error |= 0x10;
                    }
                }
                self.queue_rx((self.rx_shift | stop) as u8, error);
                self.rx_bit = 0;
                self.rx_count = if self.inputs.rxd { clocks / 2 } else { clocks };
                if brk {
                    self.break_seen = true;
                    self.ext_event();
                }
            } else {
                self.rx_count = clocks;
                self.rx_bit += 1;
            }
        }
    }
    fn load_tx(&mut self) {
        if !self.tx_enabled() || self.tx_bits != 0 || self.tx_stop != 0 {
            return;
        }
        let Some(data) = self.tx_hold.take() else {
            return;
        };
        let width = self.tx_width();
        let data = data & ((1u16 << width) - 1) as u8;
        self.tx_frame = u16::from(data) << 1; // start bit, data, optional parity
        self.tx_bits = 1 + width;
        if self.wr[4] & 1 != 0 {
            let bit = (data.count_ones() & 1 != 0) ^ (self.wr[4] & 2 == 0);
            self.tx_frame |= u16::from(bit) << self.tx_bits;
            self.tx_bits += 1;
        }
        self.tx_left = self.tx_bits;
        self.tx_count = 0;
        if self.wr[1] & 2 != 0 && !self.tx_disarm {
            self.pending[0] = true;
        }
    }
    fn transmit_clock(&mut self) {
        if self.wr[5] & 0x10 != 0 {
            return;
        } // break holds TXD low, retains queued state
        if self.tx_stop != 0 {
            self.tx_stop -= 1;
            if self.tx_stop != 0 {
                return;
            }
        }
        self.load_tx();
        if self.tx_count != 0 {
            self.tx_count -= 1;
            if self.tx_count != 0 {
                return;
            }
        }
        if self.tx_left != 0 {
            self.txd = self.tx_frame & 1 != 0;
            self.tx_frame >>= 1;
            self.tx_left -= 1;
            self.tx_count = self.divisor();
        } else if !self.txd || self.tx_count == 0 {
            // The final data/parity interval has elapsed: start stop bits.
            if self.tx_bits != 0 {
                self.txd = true;
                self.tx_bits = 0;
                self.tx_stop = self.divisor() * [0, 2, 3, 4][((self.wr[4] >> 2) & 3) as usize] / 2;
            }
        }
    }
    fn outputs(&self) -> SerialOutputs {
        SerialOutputs {
            txd: self.txd && self.wr[5] & 0x10 == 0,
            rts: self.wr[5] & 2 == 0 && self.all_sent(),
            dtr: self.wr[5] & 0x80 == 0,
        }
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Sio {
    channels: [Channel; 2],
    service: [bool; 6],
}
impl Sio {
    pub fn valid_state(&self) -> bool {
        self.channels.iter().all(|c| {
            c.supported()
                && c.rx_len <= 3
                && c.rx_bit <= 10
                && c.rx_count <= 63
                && c.tx_bits <= 10
                && c.tx_left <= c.tx_bits
                && c.tx_count <= 64
                && c.tx_stop <= 128
        })
    }
    fn reset_channel(&mut self, index: usize) {
        let old = &self.channels[index];
        let mut reset = Channel::default();
        reset.wr = old.wr;
        reset.wr[1] &= !0x9b;
        reset.wr[3] &= !1;
        reset.wr[5] &= !0x8a;
        reset.inputs = old.inputs;
        reset.clock = old.clock;
        self.channels[index] = reset;
        if index == 0 {
            self.service = [false; 6];
            for c in &mut self.channels {
                c.pending = [false; 3];
            }
        }
    }
    pub fn reset(&mut self) {
        self.reset_channel(0);
        self.reset_channel(1);
    }
    pub fn inputs(&mut self, index: usize, inputs: SerialInputs) {
        let c = &mut self.channels[index];
        let old = c.inputs;
        let enabled = c.rx_enabled();
        c.inputs = inputs;
        if !enabled && c.rx_enabled() {
            c.rx_first = true;
            c.rx_bit = 0;
            c.rx_count = 0;
        }
        if old.cts != inputs.cts || old.dcd != inputs.dcd {
            c.ext_event();
        }
    }
    pub fn outputs(&self, index: usize) -> SerialOutputs {
        self.channels[index].outputs()
    }
    pub fn clock(&mut self, index: usize, level: bool) {
        let c = &mut self.channels[index];
        if level && !c.clock {
            c.receive_clock();
        }
        if !level && c.clock {
            c.transmit_clock();
        }
        c.clock = level;
    }
    pub fn read(&mut self, address: u8) -> Option<u8> {
        let index = (address >> 1 & 1) as usize;
        if address & 1 == 0 {
            let c = &mut self.channels[index];
            let data = c.rx_data[0];
            if c.wr[1] & 0x18 != 8 || c.rr1_errors & 0x60 == 0 {
                c.pop_rx();
            }
            return Some(data);
        }
        let reg = self.channels[index].wr[0] & 7;
        self.channels[index].wr[0] &= !7;
        let c = &self.channels[index];
        match reg {
            0 => Some(
                c.rr0()
                    | if index == 0 && self.channels.iter().any(|c| c.pending.iter().any(|v| *v)) {
                        2
                    } else {
                        0
                    },
            ),
            1 => Some(c.rr1_errors | u8::from(c.all_sent())),
            2 => Some(if index == 0 { 0 } else { self.vector() }),
            _ => None,
        }
    }
    pub fn write(&mut self, address: u8, data: u8) -> bool {
        let index = (address >> 1 & 1) as usize;
        let c = &mut self.channels[index];
        if address & 1 == 0 {
            if c.wr[4] & 12 == 0 {
                return false;
            }
            c.tx_hold = Some(data);
            c.tx_disarm = false;
            c.pending[0] = false;
            c.load_tx();
            return true;
        }
        let reg = (c.wr[0] & 7) as usize;
        if reg == 0 {
            // CRC reset commands are inert in async; reject them in sync mode.
            if (data & 0xc0 != 0 || data & 0x38 == 8) && c.wr[4] & 12 == 0 {
                return false;
            }
            c.wr[0] = data;
            match data & 0x38 {
                0x10 => c.reset_ext(),
                0x18 => self.reset_channel(index),
                0x20 => c.rx_first = true,
                0x28 => {
                    c.pending[0] = false;
                    c.tx_disarm = true;
                }
                0x30 => {
                    let pop = c.wr[1] & 0x18 == 8 && c.rr1_errors & 0x60 != 0;
                    c.rr1_errors = 0;
                    if pop {
                        c.pop_rx();
                    } else {
                        c.update_rx_irq();
                    }
                }
                0x38 if index == 0 => self.reti(),
                _ => {}
            }
            return true;
        }
        let old = c.wr[reg];
        let was_rx = c.rx_enabled();
        c.wr[reg] = data;
        if !c.supported()
            || (reg == 2 && index == 0 && data != 0)
            || (matches!(reg, 3..=5) && (!c.all_sent() || c.rx_bit != 0) && data != old)
        {
            c.wr[reg] = old;
            return false;
        }
        c.wr[0] &= !7;
        if !was_rx && c.rx_enabled() {
            c.rx_first = true;
            c.rx_bit = 0;
            c.rx_count = 0;
        }
        if reg == 1 && data & 1 != 0 && c.ext_latch.is_some() {
            c.pending[1] = true;
        }
        if reg == 5 && data & 8 == 0 {
            c.pending[0] = false;
        }
        if reg == 5 {
            c.load_tx();
        }
        c.update_rx_irq();
        true
    }
    pub fn irq_state(&self) -> IrqState {
        let mut state = IrqState::default();
        for src in PRIORITY {
            if self.service[src] {
                state.in_service = true;
                break;
            }
            state.requested |= self.channels[src / 3].pending[src % 3];
        }
        state
    }
    pub fn acknowledge(&mut self) -> Option<u8> {
        for src in PRIORITY {
            if self.service[src] {
                break;
            }
            if self.channels[src / 3].pending[src % 3] {
                let vector = self.vector();
                self.service[src] = true;
                return Some(vector);
            }
        }
        None
    }
    pub fn reti(&mut self) {
        for src in PRIORITY {
            if self.service[src] {
                self.service[src] = false;
                break;
            }
        }
    }
    fn vector(&self) -> u8 {
        let base = self.channels[1].wr[2];
        if self.channels[1].wr[1] & 4 == 0 {
            return base;
        }
        for src in PRIORITY {
            let c = &self.channels[src / 3];
            if c.pending[src % 3] {
                let mut bits = [8, 10, 12, 0, 2, 4][src];
                if src % 3 == 2 && c.rr1_errors & c.special_mask() != 0 {
                    bits |= 2;
                }
                return (base & 0xf1) | bits;
            }
        }
        (base & 0xf1) | 6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wr(s: &mut Sio, ch: u8, reg: u8, data: u8) {
        assert!(s.write(ch * 2 + 1, reg));
        assert!(s.write(ch * 2 + 1, data));
    }
    fn rr(s: &mut Sio, ch: u8, reg: u8) -> u8 {
        assert!(s.write(ch * 2 + 1, reg));
        s.read(ch * 2 + 1).unwrap()
    }
    fn configured() -> Sio {
        let mut s = Sio::default();
        for ch in 0..2 {
            wr(&mut s, ch, 4, 0x44);
            wr(&mut s, ch, 3, 0xc1);
            wr(&mut s, ch, 5, 0xea);
        }
        wr(&mut s, 1, 2, 0x80);
        wr(&mut s, 1, 1, 4);
        s
    }
    fn clocks(s: &mut Sio, ch: usize, count: u32, rxd: bool) {
        let mut inputs = s.channels[ch].inputs;
        inputs.rxd = rxd;
        s.inputs(ch, inputs);
        for _ in 0..count {
            s.clock(ch, true);
            s.clock(ch, false);
        }
    }
    fn receive(s: &mut Sio, ch: usize, data: u8, parity: Option<bool>, stop: bool) {
        clocks(s, ch, 16, true);
        clocks(s, ch, 16, false);
        for bit in 0..8 {
            clocks(s, ch, 16, data & (1 << bit) != 0);
        }
        if let Some(parity) = parity {
            clocks(s, ch, 16, parity);
        }
        clocks(s, ch, 16, stop);
    }
    #[test]
    fn register_pointer_channels_and_unsupported_modes() {
        let mut s = Sio::default();
        assert_eq!(s.read(1), Some(0x54));
        assert_eq!(rr(&mut s, 0, 1), 1);
        assert!(s.write(1, 3));
        assert!(!s.write(1, 1)); // sync RX enable
        assert!(!s.write(1, 0xc1));
        assert_eq!(s.read(1), None); // pointer still RR3
        wr(&mut s, 1, 2, 0xa0);
        assert_eq!(rr(&mut s, 0, 2), 0);
        assert_eq!(rr(&mut s, 1, 2), 0xa0);
        wr(&mut s, 1, 1, 4);
        assert_eq!(rr(&mut s, 1, 2), 0xa6);
        assert!(s.write(1, 1));
        assert!(!s.write(1, 0x80)); // DMA/Wait unsupported
        assert!(s.valid_state());
    }
    #[test]
    fn tx_bits_use_clocks_and_buffer_empty_is_not_all_sent() {
        let mut s = configured();
        wr(&mut s, 0, 1, 2);
        assert!(s.write(0, 0xa5));
        assert!(s.irq_state().requested);
        assert_eq!(rr(&mut s, 0, 0) & 4, 4);
        assert_eq!(rr(&mut s, 0, 1) & 1, 0);
        assert!(s.outputs(0).txd); // no clocks, no bits emitted
        let mut bits = vec![];
        for n in 0..161 {
            clocks(&mut s, 0, 1, true);
            if n % 16 == 0 {
                bits.push(s.outputs(0).txd);
            }
        }
        assert_eq!(
            bits,
            [false, true, false, true, false, false, true, false, true, true, true]
        );
        assert_eq!(rr(&mut s, 0, 1) & 1, 1);
        assert_eq!(s.acknowledge(), Some(0x88));
        s.reti();
        assert!(s.irq_state().requested); // RETI alone does not clear the source
        assert!(s.write(1, 0x28));
        assert!(!s.irq_state().requested);
        assert!(s.write(0, 0x55));
        assert!(s.irq_state().requested);
    }
    #[test]
    fn holding_register_waits_for_complete_first_frame() {
        let mut s = configured();
        s.write(0, 0);
        s.write(0, 0xff);
        assert_eq!(rr(&mut s, 0, 0) & 4, 0);
        clocks(&mut s, 0, 144, true);
        assert!(!s.outputs(0).txd);
        clocks(&mut s, 0, 1, true);
        assert!(s.outputs(0).txd); // stop bit
        assert_eq!(rr(&mut s, 0, 0) & 4, 0);
        clocks(&mut s, 0, 16, true);
        assert!(!s.outputs(0).txd); // next start
        assert_eq!(rr(&mut s, 0, 0) & 4, 4);
        clocks(&mut s, 0, 160, true);
        assert_eq!(rr(&mut s, 0, 1) & 1, 1);
    }
    #[test]
    fn receiver_fifo_overrun_and_error_reset() {
        let mut s = configured();
        wr(&mut s, 0, 1, 0x10);
        for data in [0x12, 0x34, 0x56, 0x78] {
            receive(&mut s, 0, data, None, true);
        }
        assert_eq!(s.acknowledge(), Some(0x8c));
        assert_eq!(s.read(0), Some(0x12));
        assert_eq!(s.read(0), Some(0x34));
        assert_eq!(rr(&mut s, 0, 1) & 0x20, 0x20);
        s.reti();
        assert_eq!(s.acknowledge(), Some(0x8e));
        assert_eq!(s.read(0), Some(0x78)); // fourth overwrites third, with overrun
        assert_eq!(rr(&mut s, 0, 0) & 1, 0);
        s.write(1, 0x30);
        assert_eq!(rr(&mut s, 0, 1) & 0x70, 0);
    }
    #[test]
    fn parity_framing_and_interrupt_on_first_hold_fifo_until_error_reset() {
        let mut s = configured();
        wr(&mut s, 0, 4, 0x47);
        wr(&mut s, 0, 1, 8);
        receive(&mut s, 0, 0x55, Some(true), false); // wrong even parity, missing stop
        assert_eq!(rr(&mut s, 0, 1) & 0x70, 0x50);
        assert_eq!(s.read(0), Some(0x55));
        assert_eq!(s.read(0), Some(0x55));
        assert_eq!(rr(&mut s, 0, 0) & 1, 1);
        s.write(1, 0x30);
        assert_eq!(rr(&mut s, 0, 0) & 1, 0);
        receive(&mut s, 0, 0x33, Some(false), true);
        assert!(!s.irq_state().requested); // not first anymore
        s.read(0);
        s.write(1, 0x20);
        receive(&mut s, 0, 0x33, Some(false), true);
        assert!(s.irq_state().requested);
    }
    #[test]
    fn channel_priority_nested_irq_and_modem_latches() {
        let mut s = configured();
        wr(&mut s, 0, 1, 0x11);
        wr(&mut s, 1, 1, 0x16);
        s.write(2, 0x55);
        assert_eq!(s.acknowledge(), Some(0x80));
        receive(&mut s, 0, 0xab, None, true); // A RX can preempt B TX
        assert_eq!(s.acknowledge(), Some(0x8c));
        s.read(0);
        s.reti();
        assert!(s.irq_state().in_service);
        s.write(3, 0x28);
        s.reti();
        s.inputs(
            0,
            SerialInputs {
                cts: false,
                ..SerialInputs::default()
            },
        );
        assert_eq!(rr(&mut s, 0, 0) & 0x20, 0x20);
        s.inputs(0, SerialInputs::default());
        assert_eq!(rr(&mut s, 0, 0) & 0x20, 0x20); // frozen first edge
        s.write(1, 0x10);
        assert_eq!(rr(&mut s, 0, 0) & 0x20, 0);
        assert!(s.irq_state().requested);
        s.write(1, 0x10);
        assert!(!s.irq_state().requested);
    }
    #[test]
    fn snapshots_continue_mid_rx_tx_and_register_selection() {
        let mut s = configured();
        s.write(0, 0x53);
        clocks(&mut s, 0, 25, false);
        s.write(3, 2); // pending register selection included in snapshot
        let mut copy: Sio = bincode::deserialize(&bincode::serialize(&s).unwrap()).unwrap();
        for dev in [&mut s, &mut copy] {
            assert!(dev.write(3, 0x90));
        }
        for n in 0..300 {
            let rxd = n % 37 < 16;
            clocks(&mut s, 0, 1, rxd);
            clocks(&mut copy, 0, 1, rxd);
            assert_eq!(s.outputs(0), copy.outputs(0));
            assert_eq!(
                bincode::serialize(&s).unwrap(),
                bincode::serialize(&copy).unwrap()
            );
        }
        assert!(copy.valid_state());
        copy.reset();
        assert_eq!(rr(&mut copy, 0, 1), 1);
        assert!(!copy.irq_state().requested);
    }

    #[test]
    fn tx_word_lengths_parity_stop_durations_and_divisors() {
        for (width_bits, width) in [(0x20, 7), (0x40, 6), (0x60, 8)] {
            for (clock_bits, divisor) in [(0, 1), (0x40, 16), (0x80, 32), (0xc0, 64)] {
                for (stop_bits, halves) in [(4, 2), (8, 3), (12, 4)] {
                    if divisor == 1 && halves == 3 {
                        continue;
                    }
                    let mut s = Sio::default();
                    wr(&mut s, 0, 4, clock_bits | stop_bits | 3); // even parity
                    wr(&mut s, 0, 5, width_bits | 8);
                    s.write(0, 0x35);
                    let mut bits = vec![];
                    for _ in 0..(1 + width + 1) {
                        clocks(&mut s, 0, 1, true);
                        bits.push(s.outputs(0).txd);
                        clocks(&mut s, 0, divisor - 1, true);
                    }
                    assert!(!bits[0]);
                    for bit in 0..width {
                        assert_eq!(bits[bit + 1], 0x35 & (1 << bit) != 0);
                    }
                    assert!(!bits[width + 1]); // 0x35 has four set bits
                    clocks(&mut s, 0, 1, true);
                    assert!(s.outputs(0).txd);
                    clocks(&mut s, 0, divisor * halves / 2 - 1, true);
                    assert_eq!(rr(&mut s, 0, 1) & 1, 0);
                    clocks(&mut s, 0, 1, true);
                    assert_eq!(rr(&mut s, 0, 1) & 1, 1);
                }
            }
        }
    }

    #[test]
    fn cts_dcd_auto_enable_and_break_hold() {
        let mut s = configured();
        wr(&mut s, 0, 3, 0xe1); // auto enables
        s.write(0, 0x55);
        clocks(&mut s, 0, 200, true);
        assert_eq!(rr(&mut s, 0, 0) & 4, 0);
        assert!(s.outputs(0).txd);
        s.inputs(
            0,
            SerialInputs {
                cts: false,
                dcd: false,
                rxd: true,
            },
        );
        clocks(&mut s, 0, 161, true);
        assert_eq!(rr(&mut s, 0, 1) & 1, 1);
        wr(&mut s, 0, 5, 0xfa);
        assert!(!s.outputs(0).txd);
        clocks(&mut s, 0, 200, true);
        assert!(!s.outputs(0).txd);
        wr(&mut s, 0, 5, 0xea);
        assert!(s.outputs(0).txd);
        s.write(1, 0x10); // release the earlier CTS/DCD status latch
        receive(&mut s, 0, 0, None, false); // break condition, not repeated zero bytes
        assert_eq!(rr(&mut s, 0, 0) & 0x80, 0x80);
        clocks(&mut s, 0, 500, false);
        assert_eq!(s.channels[0].rx_len, 1);
        clocks(&mut s, 0, 16, true);
        s.write(1, 0x10);
        s.write(1, 0x10);
        assert_eq!(rr(&mut s, 0, 0) & 0x80, 0);
    }
}
