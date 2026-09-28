// license: BSD-3-Clause
// Reference: MAME src/devices/machine/msm6253.cpp, copyright-holder AJR.
// See LICENSES/MAME-BSD-3-Clause.txt for the license terms.
//! OKI MSM6253 8-bit ADC serial output, shared by the Model 1 I/O boards.
//!
//! The board decodes the channel and samples it on a write, not on the first
//! read. Conversion timing is not modeled; each read shifts the latched sample
//! out MSB first, with zeros feeding in behind it.

use std::cell::Cell;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Adc {
    shifter: Cell<u8>,
}

impl Adc {
    pub(crate) fn latch(&self, value: u8) {
        self.shifter.set(value);
    }

    /// The next bit of the conversion, most significant first, in bit 0.
    pub(crate) fn shift_out(&self) -> u8 {
        let shifter = self.shifter.get();
        self.shifter.set(shifter << 1);
        shifter >> 7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_shifts_msb_first_and_then_zero() {
        let adc = Adc::default();
        assert_eq!(adc.shift_out(), 0);
        for value in 0..=255u8 {
            adc.latch(value);
            for bit in (0..8).rev() {
                assert_eq!(adc.shift_out(), (value >> bit) & 1);
            }
            for _ in 0..8 {
                assert_eq!(adc.shift_out(), 0);
            }
        }
    }

    #[test]
    fn write_replaces_an_unfinished_conversion() {
        let adc = Adc::default();
        adc.latch(0xff);
        assert_eq!(adc.shift_out(), 1);
        adc.latch(0x53);
        let result = (0..8).fold(0, |value, _| (value << 1) | adc.shift_out());
        assert_eq!(result, 0x53);
    }

    #[test]
    fn snapshot_resumes_in_the_middle_of_a_conversion() {
        let adc = Adc::default();
        adc.latch(0xb6);
        assert_eq!(adc.shift_out(), 1);
        assert_eq!(adc.shift_out(), 0);
        assert_eq!(adc.shift_out(), 1);
        let bytes = bincode::serialize(&adc).unwrap();
        let restored: Adc = bincode::deserialize(&bytes).unwrap();
        for expected in [1, 0, 1, 1, 0, 0, 0, 0] {
            assert_eq!(restored.shift_out(), expected);
            assert_eq!(adc.shift_out(), expected);
        }
        assert_eq!(
            bincode::serialize(&restored).unwrap(),
            bincode::serialize(&adc).unwrap()
        );
    }
}
