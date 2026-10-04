//! Bounded i8251 subset: async 8N1 x16, plus sync initialization bytes.
//! Register semantics reference MAME i8251 (BSD-3-Clause, smf).
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    UnsupportedUartMode(u8),
    UnsupportedUartCommand(u8),
    TransmitFull,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Uart {
    pub mode: u8,
    pub command: u8,
    // 0 command, 1 mode, 2/3 remaining sync characters.
    phase: u8,
    status: u8,
    data: u8,
    pub rx: bool,
    pub tx: bool,
    rx_step: u8,
    rx_wait: u8,
    rx_shift: u8,
    tx_hold: Option<u8>,
    tx_shift: u16,
    tx_bits: u8,
    tx_wait: u8,
}
impl Default for Uart {
    fn default() -> Self {
        Self {
            mode: 0,
            command: 0,
            phase: 1,
            status: 5,
            data: 0,
            rx: true,
            tx: true,
            rx_step: 0,
            rx_wait: 0,
            rx_shift: 0,
            tx_hold: None,
            tx_shift: 0,
            tx_bits: 0,
            tx_wait: 0,
        }
    }
}
impl Uart {
    pub fn control(&mut self, value: u8) -> Result<(), Error> {
        match self.phase {
            1 => {
                // Permit the firmware's synchronous reset preamble without
                // claiming synchronous data transfers. Async parity-off bit 5
                // is immaterial. Other framing needs a separate implementation.
                if value & 3 != 0 && value & !0x20 != 0x4e {
                    return Err(Error::UnsupportedUartMode(value));
                }
                self.mode = value;
                self.phase = if value & 3 != 0 || value & 0x40 != 0 {
                    0
                } else if value & 0x80 != 0 {
                    2
                } else {
                    3
                };
            }
            2 | 3 => self.phase = if self.phase == 3 { 2 } else { 0 },
            _ => {
                if value & 0x40 == 0
                    && (value & 0x88 != 0 || (value & 5 != 0 && self.mode & 3 == 0))
                {
                    return Err(Error::UnsupportedUartCommand(value));
                }
                self.command = value;
                if value & 0x10 != 0 {
                    self.status &= !0x38;
                }
                // Internal reset returns to mode format, not a hardware reset.
                if value & 0x40 != 0 {
                    self.phase = 1;
                }
                if value & 4 == 0 {
                    self.rx_step = 0;
                    self.rx_wait = 0;
                }
            }
        }
        Ok(())
    }
    pub fn irq(&self) -> bool {
        self.command & 4 != 0 && self.status & 2 != 0
    }
    pub fn status(&self) -> u8 {
        self.status
    }
    pub fn read(&mut self) -> u8 {
        self.status &= !2;
        self.data
    }
    pub fn write(&mut self, value: u8) -> Result<(), Error> {
        if self.tx_hold.is_some() {
            return Err(Error::TransmitFull);
        }
        self.tx_hold = Some(value);
        self.status &= !1;
        Ok(())
    }
    /// One external x16 clock (500 kHz on DSB). The caller owns clock timing;
    /// CTS is permanently asserted by the connected emulated device.
    pub fn tick(&mut self) {
        if self.mode & 3 == 0 {
            return;
        }
        if self.command & 4 != 0 {
            if self.rx_step == 0 {
                if !self.rx {
                    self.rx_step = 1;
                    self.rx_wait = 8;
                    self.rx_shift = 0;
                }
            } else {
                self.rx_wait -= 1;
                if self.rx_wait == 0 {
                    match self.rx_step {
                        1 if self.rx => self.rx_step = 0, // false start
                        1 => {
                            self.rx_step = 2;
                            self.rx_wait = 16;
                        }
                        2..=9 => {
                            self.rx_shift |= (self.rx as u8) << (self.rx_step - 2);
                            self.rx_step += 1;
                            self.rx_wait = 16;
                        }
                        _ => {
                            if !self.rx {
                                self.status |= 0x20;
                            }
                            if self.status & 2 != 0 {
                                self.status |= 0x10;
                            }
                            self.data = self.rx_shift;
                            self.status |= 2;
                            self.rx_step = 0;
                        }
                    }
                }
            }
        }
        // Finish an in-flight frame even if transmission is then disabled.
        if self.tx_wait > 0 {
            self.tx_wait -= 1;
        }
        if self.tx_wait == 0 {
            if self.tx_bits == 0 && self.command & 1 != 0 {
                if let Some(byte) = self.tx_hold.take() {
                    self.tx_shift = 0x200 | (u16::from(byte) << 1);
                    self.tx_bits = 10;
                    self.status = (self.status | 1) & !4;
                }
            }
            if self.tx_bits != 0 {
                self.tx = self.tx_shift & 1 != 0;
                self.tx_shift >>= 1;
                self.tx_bits -= 1;
                self.tx_wait = 16;
            } else {
                self.tx = true;
                if self.tx_hold.is_none() {
                    self.status |= 4;
                }
            }
        }
    }
    pub fn valid(&self) -> bool {
        self.phase <= 3
            && self.status & !0x3f == 0
            && (self.mode & 3 == 0 || self.mode & !0x20 == 0x4e)
            && self.rx_step <= 10
            && self.rx_wait <= 16
            && (self.rx_step == 0 || self.rx_wait != 0)
            && self.tx_bits <= 10
            && self.tx_wait <= 16
            && self.tx_shift <= 0x3ff
    }
}
