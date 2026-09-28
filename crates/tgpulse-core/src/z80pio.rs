// license: BSD-3-Clause
// Reference: MAME src/devices/machine/z80pio.cpp, copyright-holder Curt Coder.
// See LICENSES/MAME-BSD-3-Clause.txt.
//! PIO register/bit-control model. No host objects or wall-clock state.
//! Bidirectional mode is deliberately rejected pending handshake integration.
use crate::z80ctc::IrqState;

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
enum Next {
    #[default]
    Command,
    Direction,
    Mask,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Port {
    mode: u8,
    next: Next,
    input: u8,
    output: u8,
    direction: u8,
    control: u8,
    vector: u8,
    mask: u8,
    enabled: bool,
    pending: bool,
    service: bool,
    matched: bool,
    ready: bool,
    strobe: bool,
}

impl Default for Port {
    fn default() -> Self {
        Self {
            mode: 1,
            next: Next::Command,
            input: 0,
            output: 0,
            direction: 0,
            control: 0,
            vector: 0,
            mask: 0xff,
            enabled: false,
            pending: false,
            service: false,
            matched: false,
            ready: false,
            strobe: false,
        }
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Pio {
    ports: [Port; 2],
}

impl Pio {
    pub fn reset(&mut self) {
        for p in &mut self.ports {
            // Retain vector, sampled pins and ICW configuration, like MAME.
            p.mode = 1;
            p.control &= 0x7f;
            p.enabled = false;
            p.pending = false;
            p.service = false;
            p.matched = false;
            p.direction = 0;
            p.mask = 0xff;
            p.output = 0;
            p.ready = false;
        }
    }

    pub fn valid_state(&self) -> bool {
        self.ports
            .iter()
            .all(|p| matches!(p.mode, 0 | 1 | 3) && p.vector & 1 == 0)
    }

    // Alternate layout: A data, A control, B data, B control.
    pub fn read(&mut self, address: u8, pins: [u8; 2]) -> u8 {
        if address & 1 != 0 {
            return (self.ports[0].control & 0xc0) | (self.ports[1].control >> 4);
        }
        let index = (address >> 1 & 1) as usize;
        let p = &mut self.ports[index];
        match p.mode {
            0 => p.output,
            1 => {
                if !p.strobe {
                    p.input = pins[index];
                }
                p.ready = true;
                p.input
            }
            _ => {
                p.input = pins[index];
                (p.input & p.direction) | (p.output & !p.direction)
            }
        }
    }

    /// false means unsupported control/mode; do not pretend it succeeded.
    pub fn write(&mut self, address: u8, data: u8) -> bool {
        let p = &mut self.ports[(address >> 1 & 1) as usize];
        if address & 1 == 0 {
            p.output = data;
            if p.mode == 0 {
                p.ready = true;
            }
            return true;
        }
        match p.next {
            Next::Direction => {
                p.direction = data;
                p.enabled = p.control & 0x80 != 0;
                p.next = Next::Command;
            }
            Next::Mask => {
                p.mask = data;
                p.enabled = p.control & 0x80 != 0;
                p.next = Next::Command;
            }
            Next::Command if data & 1 == 0 => p.vector = data,
            Next::Command => match data & 15 {
                15 => {
                    let mode = data >> 6;
                    if mode == 2 {
                        return false;
                    }
                    p.mode = mode;
                    if mode == 0 {
                        p.ready = true;
                    }
                    if mode == 3 {
                        p.ready = false;
                        p.enabled = false;
                        p.matched = false;
                        p.next = Next::Direction;
                    }
                }
                7 => {
                    p.control = data;
                    if data & 0x10 != 0 {
                        p.enabled = false;
                        p.pending = false;
                        p.matched = false;
                        p.next = Next::Mask;
                    } else {
                        p.enabled = data & 0x80 != 0;
                    }
                }
                3 => {
                    p.control = (p.control & 0x7f) | (data & 0x80);
                    p.enabled = data & 0x80 != 0;
                }
                _ => return false,
            },
        }
        true
    }

    pub fn pins(&mut self, index: usize, pins: u8) {
        let p = &mut self.ports[index];
        if p.mode != 3 {
            return;
        }
        p.input = pins;
        let mask = !p.mask;
        let data = ((pins & p.direction) | (p.output & !p.direction)) & mask;
        let matched = match p.control & 0x60 {
            0 => data != mask,
            0x20 => data != 0,
            0x40 => data == 0,
            _ => data == mask,
        };
        if !p.matched && matched && !p.service {
            p.pending = true;
        }
        p.matched = matched;
    }

    /// Handshake pins for modes 0/1. No strobes are synthesized for board DIP switches.
    #[allow(dead_code)] // Not wired on this board; covered independently for reuse.
    pub fn strobe(&mut self, index: usize, level: bool, pins: u8) {
        let p = &mut self.ports[index];
        if p.mode == 1 && !level {
            p.input = pins;
        }
        if (p.mode == 1 || (p.mode == 0 && p.ready)) && !p.strobe && level {
            p.pending = true;
            p.ready = false;
        }
        p.strobe = level;
    }

    pub fn irq_state(&self) -> IrqState {
        // Match the reference's PIO policy: any in-service port blocks requests.
        if self.ports.iter().any(|p| p.service) {
            return IrqState {
                requested: false,
                in_service: true,
            };
        }
        IrqState {
            requested: self.ports.iter().any(|p| p.enabled && p.pending),
            in_service: false,
        }
    }
    pub fn acknowledge(&mut self) -> Option<u8> {
        if !self.irq_state().requested {
            return None;
        }
        let p = self.ports.iter_mut().find(|p| p.enabled && p.pending)?;
        p.pending = false;
        p.service = true;
        Some(p.vector)
    }
    pub fn reti(&mut self) {
        if let Some(p) = self.ports.iter_mut().find(|p| p.service) {
            p.service = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ports_directions_and_control_readback() {
        let mut p = Pio::default();
        assert_eq!(p.read(0, [0x12, 0x34]), 0x12);
        assert_eq!(p.read(2, [0x12, 0x34]), 0x34);
        assert!(p.write(1, 0xcf));
        assert!(p.write(1, 0xf0));
        assert!(p.write(0, 0x5a));
        assert_eq!(p.read(0, [0xc3, 0]), 0xca);
        assert!(p.write(3, 0x0f));
        assert!(p.write(2, 0x56));
        assert_eq!(p.read(2, [0, 0]), 0x56);
        assert!(p.write(1, 0xc7));
        assert!(p.write(3, 0x87));
        assert_eq!(p.read(1, [0, 0]), 0xc8);
        assert!(!p.write(3, 0x8f)); // unsupported bidirectional
    }
    fn bit_irq(p: &mut Pio, port: u8, vector: u8) {
        for data in [vector, 0xcf, 0xff, 0xb7, 0xfe] {
            assert!(p.write(port, data));
        }
    }
    #[test]
    fn masked_bit_interrupts_ack_reti_and_priority() {
        let mut p = Pio::default();
        bit_irq(&mut p, 1, 0x40);
        bit_irq(&mut p, 3, 0x42);
        p.pins(0, 2);
        assert!(!p.irq_state().requested);
        p.pins(0, 1);
        p.pins(1, 1);
        assert_eq!(p.acknowledge(), Some(0x40));
        assert!(!p.irq_state().requested);
        p.reti();
        assert_eq!(p.acknowledge(), Some(0x42));
        p.reti();
        p.pins(0, 1);
        assert!(!p.irq_state().requested);
        p.pins(0, 0);
        p.pins(0, 1);
        assert!(p.irq_state().requested);
    }
    #[test]
    fn disabled_pending_port_cannot_steal_enabled_vector() {
        let mut p = Pio::default();
        bit_irq(&mut p, 1, 0x40);
        bit_irq(&mut p, 3, 0x42);
        p.pins(0, 1);
        p.write(1, 3);
        p.pins(1, 1);
        assert_eq!(p.acknowledge(), Some(0x42));
    }
    #[test]
    fn input_strobe_latches_and_snapshot_preserves_next_control_word() {
        let mut p = Pio::default();
        p.write(1, 0x20);
        p.write(1, 0x87);
        p.strobe(0, false, 0x12);
        p.strobe(0, true, 0x34);
        assert_eq!(p.read(0, [0xff; 2]), 0x12);
        assert_eq!(p.acknowledge(), Some(0x20));
        p.reti();
        p.write(1, 0xcf);
        let mut restored: Pio = bincode::deserialize(&bincode::serialize(&p).unwrap()).unwrap();
        for dev in [&mut p, &mut restored] {
            dev.write(1, 0xf0);
            dev.write(0, 0x0a);
        }
        assert_eq!(p.read(0, [0x50; 2]), restored.read(0, [0x50; 2]));
        assert_eq!(
            bincode::serialize(&p).unwrap(),
            bincode::serialize(&restored).unwrap()
        );
        p.reset();
        assert!(!p.irq_state().requested);
        assert!(p.valid_state());
    }
}
