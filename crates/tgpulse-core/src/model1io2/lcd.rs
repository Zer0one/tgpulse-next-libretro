// license: BSD-3-Clause
// Write-side reference: MAME src/devices/video/hd44780.cpp (Sandro Ronco)
// and src/mame/sega/model1io2.cpp (Dirk Best). See LICENSES/MAME-BSD-3-Clause.txt.
// Command semantics also checked against Hitachi HD44780U, pp. 26-27.
//! The board's write-only HD44780 diagnostic panel (2 x 20 characters).
//! No readback/busy pin or host display is connected. The optional renderer
//! consumes a frontend-owned CGROM without adding resources to device state.
//! Keep command/data state so a frontend can inspect the panel and snapshots
//! resume an incomplete four-bit transfer without replaying board writes.

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Lcd {
    #[serde(with = "serde_big_array::BigArray")]
    ddram: [u8; 128],
    #[serde(with = "serde_big_array::BigArray")]
    cgram: [u8; 64],
    address: u8,
    character_ram: bool,
    increment: bool,
    entry_shift: bool,
    display_shift: u8,
    display_control: u8,
    function: u8,
    first_command: bool,
    high_nibble: Option<(bool, u8)>,
}

impl Default for Lcd {
    fn default() -> Self {
        Self {
            ddram: [b' '; 128],
            cgram: [0; 64],
            address: 0,
            character_ram: false,
            increment: true,
            entry_shift: false,
            display_shift: 0,
            display_control: 0,
            function: 0x10, // eight-bit interface, one internal line, 5x8 font
            first_command: true,
            high_nibble: None,
        }
    }
}

impl Lcd {
    /// MAME board geometry/pens: 121x19, 5x8 dots, one-dot character/line gaps.
    /// CGROM has 16 bytes per character. Codes 0..15 alias eight CGRAM glyphs.
    pub(super) fn pixels(&self, cgrom: &[u8; 4096], blink_on: bool) -> super::DiagnosticPixels {
        let mut pixels = [[0; 121]; 19];
        if self.display_control & 4 == 0 {
            return pixels;
        }
        let lines = if self.function & 8 == 0 { 1 } else { 2 };
        let width = 80 / lines;
        for line in 0..lines {
            for column in 0..20 {
                let address = line * 64 + (column + self.display_shift as usize) % width;
                let code = self.ddram[address] as usize;
                for y in 0..8 {
                    let mut row = if code < 16 {
                        if self.function & 4 == 0 {
                            self.cgram[(code & 7) * 8 + y]
                        } else {
                            self.cgram[((code >> 1) & 3) * 16 + y]
                        }
                    } else {
                        cgrom[code * 16 + y]
                    };
                    if address == self.address as usize {
                        let cursor_row = if self.function & 4 == 0 { 7 } else { 9 };
                        if (self.display_control & 2 != 0 && y == cursor_row)
                            || (self.display_control & 1 != 0 && blink_on)
                        {
                            row = 0x1f;
                        }
                    }
                    for x in 0..5 {
                        pixels[1 + line * 9 + y][1 + column * 6 + x] =
                            if row & (1 << (4 - x)) != 0 { 1 } else { 2 };
                    }
                }
            }
        }
        pixels
    }

    pub(super) fn valid(&self) -> bool {
        self.address < 128
            && (!self.character_ram || self.address < 64)
            && self.display_shift < 80
            && self.display_control < 8
            && self.function < 32
            && self.high_nibble.is_none_or(|(_, value)| value & 15 == 0)
    }

    pub(super) fn write(&mut self, data_register: bool, mut value: u8) {
        if self.function & 0x10 == 0 {
            // Changing RS restarts the transfer, like HD44780::update_nibble.
            match self.high_nibble.take() {
                Some((rs, high)) if rs == data_register => value = high | (value >> 4),
                _ => {
                    self.high_nibble = Some((data_register, value & 0xf0));
                    return;
                }
            }
        }
        if data_register {
            if self.character_ram {
                self.cgram[self.address as usize] = value;
            } else {
                self.ddram[self.address as usize] = value;
            }
            self.step_address(self.increment);
            if self.entry_shift && !self.character_ram {
                self.shift(self.increment);
            }
        } else {
            self.command(value);
        }
    }

    fn command(&mut self, value: u8) {
        match value {
            0x80..=0xff => {
                self.character_ram = false;
                self.address = value & 0x7f;
                self.correct_address();
                return;
            }
            0x40..=0x7f => {
                self.character_ram = true;
                self.address = value & 0x3f;
                return;
            }
            0x20..=0x3f => {
                let function = value & 0x1f;
                if !self.first_command
                    && (self.function ^ function) & 0x10 == 0
                    && (self.function ^ function) & 0x0c != 0
                {
                    return;
                }
                self.function = if function & 8 != 0 {
                    function & !4
                } else {
                    function
                };
                self.first_command = true;
                self.correct_address();
                return;
            }
            0x10..=0x1f => {
                if value & 8 != 0 {
                    // Shift right moves the visible window towards lower addresses.
                    self.shift(value & 4 == 0);
                } else {
                    self.step_address(value & 4 != 0);
                }
            }
            0x08..=0x0f => self.display_control = value & 7,
            0x04..=0x07 => {
                self.increment = value & 2 != 0;
                self.entry_shift = value & 1 != 0;
            }
            0x02..=0x03 => self.home(),
            0x01 => {
                self.ddram.fill(b' ');
                self.home();
                self.increment = true;
                return;
            }
            _ => {}
        }
        self.first_command = false;
    }

    fn home(&mut self) {
        self.address = 0;
        self.character_ram = false;
        self.display_shift = 0;
    }

    fn correct_address(&mut self) {
        if self.character_ram {
            self.address &= 63;
        } else {
            let maximum = if self.function & 8 == 0 { 0x4f } else { 0x67 };
            if self.address > maximum {
                self.address -= maximum + 1;
            } else if self.function & 8 != 0 && (0x28..0x40).contains(&self.address) {
                self.address += 0x18;
            }
        }
    }

    fn step_address(&mut self, increment: bool) {
        self.address = if increment {
            self.address + 1
        } else if self.address == 0 {
            if self.character_ram {
                63
            } else if self.function & 8 == 0 {
                0x4f
            } else {
                0x67
            }
        } else if !self.character_ram && self.function & 8 != 0 && self.address == 0x40 {
            0x27
        } else {
            self.address - 1
        };
        self.correct_address();
    }

    fn shift(&mut self, forward: bool) {
        self.display_shift = (self.display_shift + if forward { 1 } else { 79 }) % 80;
    }

    pub(super) fn lines(&self) -> [[u8; 20]; 2] {
        if self.display_control & 4 == 0 {
            return [[b' '; 20]; 2];
        }
        std::array::from_fn(|line| {
            std::array::from_fn(|column| {
                let width = if self.function & 8 == 0 { 80 } else { 40 };
                let address = (column + self.display_shift as usize) % width + line * 64;
                if address < self.ddram.len() {
                    self.ddram[address]
                } else {
                    b' '
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_use_cgrom_cgram_cursor_shift_and_display_enable() {
        let mut font = [0; 4096];
        font[b'A' as usize * 16] = 0x11;
        let mut single_line = Lcd::default();
        single_line.write(false, 0x0c);
        single_line.display_shift = 79;
        assert_eq!(single_line.pixels(&font, false)[10], [0; 121]);
        let mut lcd = Lcd::default();
        lcd.write(false, 0x38);
        lcd.write(false, 0x0c);
        lcd.write(true, b'A');
        assert_eq!(&lcd.pixels(&font, false)[1][1..6], &[1, 2, 2, 2, 1]);
        lcd.write(false, 0x40);
        lcd.write(true, 0x0a);
        lcd.write(false, 0x81);
        lcd.write(true, 0);
        lcd.write(true, 8); // Alias of the same CGRAM glyph.
        assert_eq!(&lcd.pixels(&font, false)[1][7..12], &[2, 1, 2, 1, 2]);
        assert_eq!(&lcd.pixels(&font, false)[1][13..18], &[2, 1, 2, 1, 2]);
        lcd.write(false, 0x81);
        lcd.write(false, 0x0f);
        assert_eq!(&lcd.pixels(&font, false)[8][7..12], &[1; 5]);
        assert_eq!(&lcd.pixels(&font, true)[1][7..12], &[1; 5]);
        lcd.write(false, 0x18);
        assert_eq!(&lcd.pixels(&font, false)[1][1..6], &[2, 1, 2, 1, 2]);
        lcd.write(false, 0x08);
        assert_eq!(lcd.pixels(&font, true), [[0; 121]; 19]);
    }

    #[test]
    fn diagnostic_text_address_wrap_shift_and_clear() {
        let mut lcd = Lcd::default();
        lcd.write(false, 0x38);
        lcd.write(false, 0x0c);
        lcd.write(false, 0x06);
        for ch in b"NETMERC" {
            lcd.write(true, *ch);
        }
        lcd.write(false, 0xc0);
        for ch in b"I/O CHECK" {
            lcd.write(true, *ch);
        }
        assert_eq!(&lcd.lines()[0][..7], b"NETMERC");
        assert_eq!(&lcd.lines()[1][..9], b"I/O CHECK");
        lcd.write(false, 0xa7); // internal first line's last address, wraps to line 2
        lcd.write(true, b'X');
        lcd.write(true, b'Y');
        assert_eq!(lcd.lines()[1][0], b'Y');
        lcd.write(false, 0x18);
        assert_eq!(&lcd.lines()[0][..6], b"ETMERC");
        lcd.write(false, 0x02);
        assert_eq!(&lcd.lines()[0][..7], b"NETMERC");
        lcd.write(false, 0x01);
        assert_eq!(lcd.lines(), [[b' '; 20]; 2]);
        lcd.write(false, 0x05); // decrement + entry shift
        lcd.write(false, 0x02); // home preserves entry direction
        assert!(!lcd.increment);
        lcd.write(false, 0x40);
        lcd.write(true, 0x1f); // CGRAM writes do not shift the display
        assert_eq!(lcd.display_shift, 0);
        lcd.write(false, 0x01); // clear restores increment, preserves S
        assert!(lcd.increment && lcd.entry_shift);
        assert!(lcd.valid());
    }

    #[test]
    fn four_bit_mid_transfer_restore_and_cgram_continue_identically() {
        let mut original = Lcd::default();
        original.write(false, 0x28); // switch to four-bit, two-line interface
        for value in [0, 0xc0, 0x80, 0] {
            original.write(false, value);
        }
        original.write(true, 0x40); // first half of 'A'
        let mut restored: Lcd =
            bincode::deserialize(&bincode::serialize(&original).unwrap()).unwrap();
        for lcd in [&mut original, &mut restored] {
            lcd.write(true, 0x10);
            assert_eq!(lcd.lines()[0][0], b'A');
            lcd.write(false, 0x40);
            lcd.write(false, 0); // CGRAM address 0
            lcd.write(true, 0x10);
            lcd.write(true, 0xf0);
            assert_eq!(lcd.cgram[0], 0x1f);
            assert!(lcd.valid());
        }
        assert_eq!(
            bincode::serialize(&original).unwrap(),
            bincode::serialize(&restored).unwrap()
        );
        let mut font = [0; 4096];
        font[b'A' as usize * 16] = 0x15;
        assert_eq!(original.pixels(&font, true), restored.pixels(&font, true));
    }
}
