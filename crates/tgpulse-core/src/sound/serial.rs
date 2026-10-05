//! Model 1's two 8251s, clocked at 500 kHz from the 10 MHz audio timebase.
//! Explicit emulated state only; no host clock or queued whole-byte delivery.
use crate::i8251::{Error, Uart};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SerialState {
    pub(crate) main: Uart,
    pub(crate) driver: Uart,
    phase: u8,
    pub(crate) fault: Option<Error>,
}
impl SerialState {
    pub(crate) fn to_edge(&self) -> usize {
        20 - usize::from(self.phase)
    }
    pub(crate) fn latch(&mut self, result: Result<(), Error>) {
        if self.fault.is_none() {
            self.fault = result.err();
        }
    }
    pub(crate) fn tick_sound(&mut self) {
        self.phase += 1;
        if self.phase == 20 {
            self.phase = 0;
            // Sample both old pins before advancing either endpoint.
            self.main.rx = self.driver.tx;
            self.driver.rx = self.main.tx;
            self.main.tick();
            self.driver.tick();
        }
    }
    /// Advance up to the next UART edge. The caller renders audio before this
    /// interval, so the endpoints must only tick at its final clock.
    pub(crate) fn advance_sound_clocks(&mut self, clocks: usize) {
        debug_assert!(clocks <= self.to_edge());
        let phase = usize::from(self.phase) + clocks;
        if phase == 20 {
            self.phase = 19;
            self.tick_sound();
        } else {
            self.phase = phase as u8;
        }
    }
    pub(crate) fn valid(&self) -> bool {
        self.phase < 20 && self.main.valid() && self.driver.valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn link() -> SerialState {
        let mut s = SerialState::default();
        for uart in [&mut s.main, &mut s.driver] {
            uart.control(0x4e).unwrap();
            uart.control(0x37).unwrap();
        }
        s
    }
    #[test]
    fn duplex_is_clocked_and_holding_register_has_backpressure() {
        let mut s = link();
        s.main.write(0x96).unwrap();
        s.driver.write(0x35).unwrap();
        assert_eq!(s.main.status() & 1, 0);
        assert_eq!(s.main.write(0), Err(Error::TransmitFull));
        for _ in 0..3000 {
            s.tick_sound();
        }
        assert!(!s.main.irq() && !s.driver.irq());
        for _ in 0..240 {
            s.tick_sound();
        }
        assert!(s.main.irq() && s.driver.irq());
        assert_eq!(s.main.read(), 0x35);
        assert_eq!(s.driver.read(), 0x96);
        assert!(!s.main.irq() && !s.driver.irq());
    }
    #[test]
    fn mid_character_snapshot_continues_identically() {
        let mut s = link();
        s.main.write(0xa5).unwrap();
        for _ in 0..1237 {
            s.tick_sound();
        }
        let bytes = bincode::serialize(&s).unwrap();
        let mut restored: SerialState = bincode::deserialize(&bytes).unwrap();
        assert!(restored.valid());
        for _ in 0..2200 {
            s.tick_sound();
            restored.tick_sound();
            assert_eq!(s, restored);
        }
        assert_eq!(restored.driver.read(), 0xa5);
    }

    #[test]
    fn unread_byte_overrun_is_flagged_not_hidden_in_a_queue() {
        let mut s = link();
        for byte in [0x12, 0x34] {
            s.main.write(byte).unwrap();
            for _ in 0..3240 {
                s.tick_sound();
            }
        }
        assert_eq!(s.driver.status() & 0x12, 0x12);
        assert_eq!(s.driver.read(), 0x34);
        s.driver.control(0x37).unwrap();
        assert_eq!(s.driver.status() & 0x12, 0);
    }
    #[test]
    fn batched_clocks_match_single_clock_uart_edges() {
        let mut reference = link();
        reference.main.write(0x96).unwrap();
        reference.driver.write(0x35).unwrap();
        let mut batched = reference.clone();
        for clocks in [1, 7, 12, 20, 3, 37, 224, 3240] {
            let mut remaining = clocks;
            while remaining > 0 {
                let step = remaining.min(batched.to_edge());
                batched.advance_sound_clocks(step);
                remaining -= step;
            }
            for _ in 0..clocks {
                reference.tick_sound();
            }
            assert_eq!(batched, reference);
        }
    }
}
