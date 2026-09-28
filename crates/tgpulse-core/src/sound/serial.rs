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
}
