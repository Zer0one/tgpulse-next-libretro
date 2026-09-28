// license: BSD-3-Clause
// Reference: MAME src/mame/sega/315_5338a.cpp, copyright-holder Dirk Best.
// See LICENSES/MAME-BSD-3-Clause.txt for the license terms.
//! Sega 315-5338A: seven parallel ports and a serial path to host memory.
//!
//! Cabinet wiring and host address decoding belong to the board, not this
//! chip. Transfers complete immediately, as in MAME's current implementation;
//! this does not model the serial wire protocol or slave mode.

/// A register write can drive parallel ports or transfer one byte to the host.
/// The board applies this effect before executing the next CPU instruction.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WriteEffect {
    None,
    Ports(u8),
    Host { address: u16, data: u8 },
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Chip5338 {
    port_value: [u8; 7],
    /// A set bit marks that port as an input.
    port_config: u8,
    cmd: u8,
    serial_output: u8,
    address: u16,
}

impl Default for Chip5338 {
    fn default() -> Self {
        Self {
            // MAME initializes the output latches high, not to zero.
            port_value: [0xff; 7],
            port_config: 0,
            cmd: 0,
            serial_output: 0,
            address: 0,
        }
    }
}

impl Chip5338 {
    pub(crate) fn read(
        &self,
        reg: u8,
        input_port: impl FnOnce(u8) -> u8,
        read_host: impl FnOnce(u16) -> u8,
    ) -> u8 {
        match reg {
            0x00..=0x06 if self.port_config & (1 << reg) != 0 => input_port(reg),
            0x00..=0x06 => self.port_value[reg as usize],
            0x08 => self.port_config,
            0x0a => self.serial_output,
            0x0b => self.cmd,
            0x0c => read_host(self.address),
            // Transfer finished (bit 3), command acknowledged (bit 0 = 0).
            0x0d => 0x08,
            _ => 0xff,
        }
    }

    pub(crate) fn port_value(&self, port: u8) -> u8 {
        self.port_value[port as usize]
    }

    pub(crate) fn write(&mut self, reg: u8, value: u8) -> WriteEffect {
        match reg {
            0x00..=0x06 => {
                self.port_value[reg as usize] = value;
                // MAME drives the output callback even for an input port.
                return WriteEffect::Ports(1 << reg);
            }
            0x08 => {
                let becoming_output = self.port_config & !value & 0x7f;
                self.port_config = value;
                return WriteEffect::Ports(becoming_output);
            }
            0x09 => {
                self.cmd = value;
                match value {
                    0x00 => self.address = (self.address & 0xff00) | self.serial_output as u16,
                    0x01 => {
                        self.address = (self.address & 0x00ff) | ((self.serial_output as u16) << 8)
                    }
                    0x07 => {
                        return WriteEffect::Host {
                            address: self.address,
                            data: self.serial_output,
                        };
                    }
                    0x70..=0x77 => {
                        return WriteEffect::Host {
                            address: (value & 7) as u16,
                            data: self.serial_output,
                        };
                    }
                    0x87 => {}
                    other => {
                        log::debug!(target: "ioboard", "unknown 315-5338A command {other:02X}")
                    }
                }
            }
            0x0a => self.serial_output = value,
            _ => {}
        }
        WriteEffect::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_local(chip: &Chip5338, reg: u8) -> u8 {
        chip.read(
            reg,
            |_| panic!("unexpected input read"),
            |_| panic!("unexpected host read"),
        )
    }

    #[test]
    fn power_on_latches_and_status_match_reference() {
        let chip = Chip5338::default();
        for port in 0..7 {
            assert_eq!(read_local(&chip, port), 0xff);
        }
        for reg in [0x08, 0x0a, 0x0b] {
            assert_eq!(read_local(&chip, reg), 0);
        }
        assert_eq!(read_local(&chip, 0x0d), 0x08);
        for reg in [0x07, 0x09, 0x0e, 0x0f] {
            assert_eq!(read_local(&chip, reg), 0xff);
        }
    }

    #[test]
    fn direction_selects_input_without_discarding_output_latch() {
        for port in 0..7 {
            let mut chip = Chip5338::default();
            assert_eq!(chip.write(8, 1 << port), WriteEffect::Ports(0));
            assert_eq!(chip.write(port, 0x52), WriteEffect::Ports(1 << port));
            assert_eq!(
                chip.read(
                    port,
                    |p| {
                        assert_eq!(p, port);
                        0xa7
                    },
                    |_| panic!()
                ),
                0xa7
            );
            assert_eq!(chip.write(8, 0), WriteEffect::Ports(1 << port));
            assert_eq!(chip.port_value(port), 0x52);
            assert_eq!(read_local(&chip, port), 0x52);
            assert_eq!(chip.write(8, 0), WriteEffect::Ports(0));
        }
    }

    #[test]
    fn direction_change_only_replays_ports_becoming_outputs() {
        let mut chip = Chip5338::default();
        chip.write(8, 0xff);
        assert_eq!(chip.write(8, 0x82), WriteEffect::Ports(0x7d));
        assert_eq!(read_local(&chip, 8), 0x82);
        // Bit 7 is readable but there is no eighth parallel port.
        assert_eq!(chip.write(8, 2), WriteEffect::Ports(0));
    }

    #[test]
    fn host_address_is_16_bit_and_does_not_auto_increment() {
        let mut chip = Chip5338::default();
        chip.write(0x0a, 0x34);
        chip.write(9, 0);
        chip.write(0x0a, 0xf2);
        chip.write(9, 1);
        chip.write(0x0a, 0xab);
        for _ in 0..2 {
            assert_eq!(
                chip.write(9, 7),
                WriteEffect::Host {
                    address: 0xf234,
                    data: 0xab
                }
            );
        }
        assert_eq!(read_local(&chip, 0x0a), 0xab);
        assert_eq!(read_local(&chip, 0x0b), 7);
        assert_eq!(chip.write(9, 0x87), WriteEffect::None);
        for _ in 0..2 {
            assert_eq!(
                chip.read(
                    0x0c,
                    |_| panic!(),
                    |address| {
                        assert_eq!(address, 0xf234);
                        0x6e
                    }
                ),
                0x6e
            );
        }
        // Loading the low byte must leave the high byte intact.
        chip.write(9, 0);
        assert_eq!(
            chip.write(9, 7),
            WriteEffect::Host {
                address: 0xf2ab,
                data: 0xab
            }
        );
    }

    #[test]
    fn short_commands_write_fixed_host_addresses() {
        let mut chip = Chip5338::default();
        chip.write(0x0a, 0xef);
        chip.write(9, 0);
        for command in 0x70..=0x77 {
            assert_eq!(
                chip.write(9, command),
                WriteEffect::Host {
                    address: (command & 7) as u16,
                    data: 0xef
                }
            );
        }
        assert_eq!(
            chip.write(9, 7),
            WriteEffect::Host {
                address: 0xef,
                data: 0xef
            }
        );
    }

    #[test]
    fn unsupported_writes_do_not_transfer_or_drive_ports() {
        let mut chip = Chip5338::default();
        for reg in [7, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f] {
            assert_eq!(chip.write(reg, 0x55), WriteEffect::None);
        }
        assert_eq!(chip.write(9, 0x56), WriteEffect::None);
        assert_eq!(read_local(&chip, 0x0b), 0x56);
        assert_eq!(read_local(&chip, 0x0d), 8);
        assert_eq!(
            chip.write(9, 7),
            WriteEffect::Host {
                address: 0,
                data: 0
            }
        );
    }

    #[test]
    fn snapshot_resumes_a_configured_host_transfer_and_preserves_port_latches() {
        let mut chip = Chip5338::default();
        for port in 0..7 {
            chip.write(port, 0x30 + port);
        }
        chip.write(8, 0x7f);
        chip.write(0x0a, 0xcd);
        chip.write(9, 0);
        chip.write(0x0a, 0xab);
        chip.write(9, 1);
        chip.write(0x0a, 0x62);
        chip.write(9, 0x87);

        let bytes = bincode::serialize(&chip).unwrap();
        let mut restored: Chip5338 = bincode::deserialize(&bytes).unwrap();
        assert_eq!(read_local(&restored, 8), 0x7f);
        assert_eq!(read_local(&restored, 0x0a), 0x62);
        assert_eq!(read_local(&restored, 0x0b), 0x87);
        assert_eq!(restored.write(8, 0), chip.write(8, 0));
        for port in 0..7 {
            assert_eq!(read_local(&restored, port), 0x30 + port);
        }
        assert_eq!(restored.write(9, 7), chip.write(9, 7));
        assert_eq!(
            restored.write(9, 7),
            WriteEffect::Host {
                address: 0xabcd,
                data: 0x62
            }
        );
        assert_eq!(
            bincode::serialize(&restored).unwrap(),
            bincode::serialize(&chip).unwrap()
        );
    }
}
