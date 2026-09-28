// license: BSD-3-Clause
// Reference: MAME src/devices/machine/z80ctc.cpp, copyright-holder Wilbert Pol,
// based on the original version (c) 1997 Tatsuyuki Satoh.
// See LICENSES/MAME-BSD-3-Clause.txt.
//! Four-channel Z80 CTC, including the fourth ZC output of the TMPZ84C015.
//!
//! Time is measured in input CPU clocks, not host time. External counter inputs
//! are clocked explicitly. No independently clocked counter input is configured
//! on Model 1 I/O board 2. Output callbacks report clock offsets within advance,
//! so a future SIO can consume both edges without losing their order.

const INTERRUPT: u8 = 0x80;
const COUNTER: u8 = 0x40;
const PRESCALER: u8 = 0x20;
const RISING: u8 = 0x10;
const EXTERNAL: u8 = 0x08;
const CONSTANT: u8 = 0x04;
const RESET: u8 = 0x02;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IrqState {
    pub requested: bool,
    pub in_service: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Channel {
    mode: u8,
    constant: u16,
    down: u16,
    external_level: bool,
    waiting: bool,
    remaining: Option<u32>,
    period: u32,
    pulse_remaining: u8,
    pending: bool,
    in_service: bool,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            mode: RESET,
            constant: 256,
            down: 0,
            external_level: false,
            waiting: false,
            remaining: None,
            period: 0,
            pulse_remaining: 0,
            pending: false,
            in_service: false,
        }
    }
}

impl Channel {
    fn divisor(&self) -> u32 {
        if self.mode & PRESCALER == 0 {
            16
        } else {
            256
        }
    }

    fn read(&self) -> u8 {
        match self.remaining {
            // Preserve MAME's remaining/prescaler + 1 readback, including the
            // exact reload boundary. A read does not consume any clocks.
            Some(remaining) if !self.waiting => (remaining / self.divisor() + 1) as u8,
            _ => self.down as u8,
        }
    }

    fn start_timer(&mut self) {
        self.period = self.divisor() * u32::from(self.constant);
        self.remaining = Some(self.period);
    }

    fn expire(&mut self) {
        if self.mode & INTERRUPT != 0 {
            self.pending = true;
        }
        self.down = self.constant;
        self.pulse_remaining = 1;
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Ctc {
    channels: [Channel; 4],
    vector: u8,
}

impl Ctc {
    pub(crate) fn valid_state(&self) -> bool {
        self.vector & 7 == 0
            && self.channels.iter().all(|ch| {
                (1..=256).contains(&ch.constant)
                    && ch.pulse_remaining <= 1
                    && match ch.remaining {
                        Some(remaining) => {
                            !ch.waiting
                                && remaining > 0
                                && remaining <= ch.period
                                && ch.period <= 65536
                        }
                        None => true,
                    }
            })
    }

    pub(crate) fn reset(&mut self) {
        for ch in &mut self.channels {
            // MAME's device reset leaves the vector, external level and last
            // down-counter value alone, unlike construction of a fresh device.
            ch.mode = RESET;
            ch.constant = 256;
            ch.remaining = None;
            ch.waiting = false;
            ch.pending = false;
            ch.in_service = false;
        }
    }

    pub(crate) fn read(&self, channel: u8) -> u8 {
        self.channels[(channel & 3) as usize].read()
    }

    pub(crate) fn write(&mut self, channel: u8, data: u8) {
        let index = (channel & 3) as usize;
        let ch = &mut self.channels[index];
        if ch.mode & CONSTANT != 0 {
            ch.constant = if data == 0 { 256 } else { u16::from(data) };
            ch.mode &= !(CONSTANT | RESET);
            ch.down = ch.constant;
            ch.waiting = ch.mode & COUNTER == 0 && ch.mode & EXTERNAL != 0;
            if ch.mode & COUNTER == 0 && !ch.waiting {
                ch.start_timer();
            } else {
                ch.remaining = None;
            }
        } else if data & 1 == 0 && index == 0 {
            self.vector = data & 0xf8;
        } else if data & 1 != 0 {
            if ch.mode & COUNTER == 0 && data & COUNTER != 0 && data & RESET == 0 {
                ch.remaining = None;
            }
            if data & RESET != 0 {
                ch.down = u16::from(ch.read());
                ch.remaining = None;
            }
            ch.mode = data;
            ch.waiting = false;
            // Software reset alone does not acknowledge an IRQ. Disabling
            // interrupts clears a pending request, but not in-service state.
            if data & INTERRUPT == 0 {
                ch.pending = false;
            }
        }
    }

    /// An input edge can start an externally triggered timer or tick a counter.
    /// Returns true when this edge generated a ZC high transition; advance()
    /// will deliver its low transition one CPU clock later.
    pub(crate) fn trigger(&mut self, channel: u8, level: bool) -> bool {
        let ch = &mut self.channels[(channel & 3) as usize];
        if ch.external_level == level {
            return false;
        }
        ch.external_level = level;
        if level != (ch.mode & RISING != 0) {
            return false;
        }
        if ch.waiting && ch.mode & COUNTER == 0 {
            ch.start_timer();
        }
        ch.waiting = false;
        if ch.mode & COUNTER != 0 {
            ch.down = ch.down.wrapping_sub(1);
            if ch.down == 0 {
                ch.expire();
                return true;
            }
        }
        false
    }

    pub(crate) fn advance(&mut self, clocks: u32, mut output: impl FnMut(u32, u8, bool)) {
        let mut elapsed = 0;
        while elapsed < clocks {
            let mut next = clocks - elapsed;
            for ch in &self.channels {
                if let Some(remaining) = ch.remaining {
                    next = next.min(remaining);
                }
                if ch.pulse_remaining != 0 {
                    next = next.min(u32::from(ch.pulse_remaining));
                }
            }
            elapsed += next;
            for (index, ch) in self.channels.iter_mut().enumerate() {
                if ch.pulse_remaining != 0 {
                    ch.pulse_remaining -= next as u8;
                    if ch.pulse_remaining == 0 {
                        output(elapsed, index as u8, false);
                    }
                }
                if let Some(remaining) = &mut ch.remaining {
                    *remaining -= next;
                    if *remaining == 0 {
                        ch.remaining = Some(ch.period);
                        ch.expire();
                        output(elapsed, index as u8, true);
                    }
                }
            }
        }
    }

    pub(crate) fn irq_state(&self) -> IrqState {
        let mut state = IrqState::default();
        for ch in &self.channels {
            if ch.in_service {
                state.in_service = true;
                break;
            }
            state.requested |= ch.pending;
        }
        state
    }

    /// Called on CPU interrupt acknowledge, not when the INT line is asserted.
    pub(crate) fn acknowledge(&mut self) -> Option<u8> {
        for (index, ch) in self.channels.iter_mut().enumerate() {
            if ch.in_service {
                break;
            }
            if ch.pending {
                ch.pending = false;
                ch.in_service = true;
                return Some(self.vector | (index as u8 * 2));
            }
        }
        None
    }

    pub(crate) fn reti(&mut self) {
        if let Some(ch) = self.channels.iter_mut().find(|ch| ch.in_service) {
            ch.in_service = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timer(ctc: &mut Ctc, channel: u8, control: u8, constant: u8) {
        ctc.write(channel, control | CONSTANT | 1);
        ctc.write(channel, constant);
    }

    fn advance(ctc: &mut Ctc, clocks: u32) -> Vec<(u32, u8, bool)> {
        let mut edges = Vec::new();
        ctc.advance(clocks, |t, ch, level| edges.push((t, ch, level)));
        edges
    }

    #[test]
    fn reset_channels_do_not_run() {
        let mut ctc = Ctc::default();
        assert!(advance(&mut ctc, 1_000_000).is_empty());
        assert_eq!(ctc.irq_state(), IrqState::default());
        assert_eq!(ctc.acknowledge(), None);
        for ch in 0..4 {
            assert_eq!(ctc.read(ch), 0);
        }
    }

    #[test]
    fn timer_readback_reload_and_single_clock_pulses() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 2, INTERRUPT, 2);
        assert_eq!(ctc.read(2), 3); // MAME exact reload boundary
        assert!(advance(&mut ctc, 1).is_empty());
        assert_eq!(ctc.read(2), 2);
        assert!(advance(&mut ctc, 30).is_empty());
        assert_eq!(ctc.read(2), 1);
        assert_eq!(advance(&mut ctc, 1), [(1, 2, true)]);
        assert_eq!(ctc.read(2), 3);
        assert!(ctc.irq_state().requested);
        assert_eq!(
            advance(&mut ctc, 33),
            [(1, 2, false), (32, 2, true), (33, 2, false)]
        );
    }

    #[test]
    fn zero_constant_is_256_for_both_prescalers() {
        for (prescaler, clocks) in [(0, 4096), (PRESCALER, 65536)] {
            let mut ctc = Ctc::default();
            timer(&mut ctc, 3, prescaler, 0);
            assert!(advance(&mut ctc, clocks - 1).is_empty());
            assert_eq!(advance(&mut ctc, 1), [(1, 3, true)]);
            assert!(!ctc.irq_state().requested);
        }
    }

    #[test]
    fn constant_load_has_priority_over_vector_and_control_decoding() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 0, INTERRUPT, 0x81);
        assert_eq!(advance(&mut ctc, 0x81 * 16), [(0x81 * 16, 0, true)]);
        ctc.write(1, 0xe0); // vector can only be set on channel 0
        assert_eq!(ctc.acknowledge(), Some(0));
        ctc.reti();
        ctc.write(0, 0xde);
        advance(&mut ctc, 0x81 * 16);
        assert_eq!(ctc.acknowledge(), Some(0xd8));
    }

    #[test]
    fn external_timer_waits_for_selected_edge_then_free_runs() {
        for rising in [false, true] {
            let mut ctc = Ctc::default();
            timer(&mut ctc, 0, EXTERNAL | if rising { RISING } else { 0 }, 2);
            assert_eq!(ctc.read(0), 2);
            assert!(advance(&mut ctc, 5000).is_empty());
            if !rising {
                ctc.trigger(0, true);
            }
            assert!(!ctc.trigger(0, rising));
            assert_eq!(advance(&mut ctc, 32), [(32, 0, true)]);
            // An additional active edge does not restart a running timer.
            ctc.trigger(0, !rising);
            ctc.trigger(0, rising);
            assert_eq!(advance(&mut ctc, 32), [(1, 0, false), (32, 0, true)]);
        }
    }

    #[test]
    fn counters_use_edges_not_elapsed_clocks_or_repeated_levels() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 1, COUNTER | RISING | INTERRUPT, 2);
        assert!(advance(&mut ctc, 1_000_000).is_empty());
        assert!(!ctc.trigger(1, true));
        assert!(!ctc.trigger(1, true));
        assert_eq!(ctc.read(1), 1);
        assert!(!ctc.trigger(1, false));
        assert!(ctc.trigger(1, true));
        assert_eq!(ctc.read(1), 2);
        assert_eq!(ctc.acknowledge(), Some(2));
        assert_eq!(advance(&mut ctc, 1), [(1, 1, false)]);
    }

    #[test]
    fn reset_stops_timer_without_acknowledging_pending_irq() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 0, INTERRUPT, 2);
        advance(&mut ctc, 40);
        let count = ctc.read(0);
        ctc.write(0, INTERRUPT | RESET | 1);
        assert!(advance(&mut ctc, 500).is_empty());
        assert_eq!(ctc.read(0), count);
        assert!(ctc.irq_state().requested);
        ctc.write(0, RESET | 1);
        assert!(!ctc.irq_state().requested);
    }

    #[test]
    fn in_service_blocks_lower_priority_but_allows_higher_priority() {
        let mut ctc = Ctc::default();
        ctc.write(0, 0xe0);
        timer(&mut ctc, 2, INTERRUPT, 1);
        advance(&mut ctc, 16);
        assert_eq!(ctc.acknowledge(), Some(0xe4));
        timer(&mut ctc, 3, INTERRUPT, 1);
        advance(&mut ctc, 16);
        assert_eq!(
            ctc.irq_state(),
            IrqState {
                requested: false,
                in_service: true
            }
        );
        assert_eq!(ctc.acknowledge(), None);
        timer(&mut ctc, 0, INTERRUPT, 1);
        advance(&mut ctc, 16);
        assert_eq!(
            ctc.irq_state(),
            IrqState {
                requested: true,
                in_service: true
            }
        );
        assert_eq!(ctc.acknowledge(), Some(0xe0));
        ctc.write(0, 3); // disabling interrupt must not end service
        assert!(ctc.irq_state().in_service);
        ctc.reti();
        assert!(!ctc.irq_state().requested); // channel 2 still blocks 3
        ctc.write(2, 3); // clear channel 2's new pending request, not its IEO
        ctc.reti();
        assert_eq!(ctc.acknowledge(), Some(0xe6));
    }

    #[test]
    fn pending_during_service_is_delivered_after_reti() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 1, INTERRUPT, 1);
        advance(&mut ctc, 16);
        assert_eq!(ctc.acknowledge(), Some(2));
        advance(&mut ctc, 16);
        assert!(!ctc.irq_state().requested);
        ctc.reti();
        assert_eq!(ctc.acknowledge(), Some(2));
        ctc.reset();
        assert_eq!(ctc.irq_state(), IrqState::default());
    }

    #[test]
    fn slice_size_and_snapshot_do_not_change_output_timing() {
        let mut whole = Ctc::default();
        timer(&mut whole, 0, INTERRUPT, 3);
        timer(&mut whole, 3, INTERRUPT, 2);
        advance(&mut whole, 17);
        let bytes = bincode::serialize(&whole).unwrap();
        let mut sliced: Ctc = bincode::deserialize(&bytes).unwrap();
        let expected = advance(&mut whole, 211);
        let mut actual = Vec::new();
        for offset in 0..211 {
            sliced.advance(1, |t, ch, high| actual.push((offset + t, ch, high)));
        }
        assert_eq!(actual, expected);
        assert_eq!(
            bincode::serialize(&whole).unwrap(),
            bincode::serialize(&sliced).unwrap()
        );
    }

    #[test]
    fn snapshot_preserves_active_pulse_and_nested_interrupt_service() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 3, INTERRUPT, 1);
        advance(&mut ctc, 16);
        ctc.acknowledge();
        timer(&mut ctc, 0, INTERRUPT, 1);
        advance(&mut ctc, 16);
        ctc.acknowledge();
        let mut restored: Ctc = bincode::deserialize(&bincode::serialize(&ctc).unwrap()).unwrap();
        assert_eq!(advance(&mut ctc, 7), advance(&mut restored, 7));
        for _ in 0..2 {
            ctc.reti();
            restored.reti();
            assert_eq!(ctc.irq_state(), restored.irq_state());
            assert_eq!(ctc.acknowledge(), restored.acknowledge());
        }
    }

    #[test]
    fn timer_to_counter_transition_stops_internal_clock() {
        let mut ctc = Ctc::default();
        timer(&mut ctc, 0, INTERRUPT, 4);
        advance(&mut ctc, 17);
        ctc.write(0, COUNTER | RISING | INTERRUPT | 1);
        assert!(advance(&mut ctc, 1000).is_empty());
        // MAME keeps m_down at the last reload value on this transition.
        assert_eq!(ctc.read(0), 4);
        for _ in 0..3 {
            assert!(!ctc.trigger(0, true));
            ctc.trigger(0, false);
        }
        assert!(ctc.trigger(0, true));
        assert_eq!(ctc.acknowledge(), Some(0));
    }

    #[test]
    fn snapshot_rejects_zero_deadlines_and_invalid_time_constants() {
        let mut ctc = Ctc::default();
        assert!(ctc.valid_state());
        timer(&mut ctc, 0, 0, 4);
        assert!(ctc.valid_state());
        let valid = ctc.clone();
        ctc.channels[0].remaining = Some(0);
        assert!(!ctc.valid_state());
        ctc = valid.clone();
        ctc.channels[0].constant = 0;
        assert!(!ctc.valid_state());
        ctc = valid.clone();
        ctc.channels[0].remaining = Some(65);
        assert!(!ctc.valid_state());
        ctc = valid.clone();
        ctc.channels[0].pulse_remaining = 2;
        assert!(!ctc.valid_state());
        ctc = valid;
        ctc.vector = 1;
        assert!(!ctc.valid_state());
    }
}
