//! Arithmetic policy. IEEE remains the default for every existing board.
//! Finite is the independently implemented, NetMerc-tested compatibility path;
//! it is not a claim of complete MB86233 hardware floating-point accuracy.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FloatMode {
    #[default]
    Ieee,
    Finite,
}

#[derive(Clone, Copy)]
enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
}

fn decode_finite(bits: u32) -> f64 {
    let exponent = (bits >> 23) & 255;
    let sign = if bits >> 31 == 0 { 1.0 } else { -1.0 };
    if exponent == 0 {
        return 0.0 * sign;
    }
    sign * (1.0 + f64::from(bits & 0x7fffff) / 8388608.0) * 2.0_f64.powi(exponent as i32 - 127)
}

fn encode_finite(value: f64) -> u32 {
    let sign = if value.is_sign_negative() {
        0x80000000
    } else {
        0
    };
    let magnitude = value.abs();
    if magnitude < 2.0_f64.powi(-126) {
        return sign;
    }
    if !magnitude.is_finite() {
        return sign | 0x7fffffff;
    }
    let mut exponent = ((magnitude.to_bits() >> 52) & 2047) as i32 - 1023;
    let mut significand = (magnitude / 2.0_f64.powi(exponent) * 8388608.0).round_ties_even() as u32;
    if significand >= 0x1000000 {
        exponent += 1;
        significand >>= 1;
    }
    if exponent > 128 {
        return sign | 0x7fffffff;
    }
    sign | (((exponent + 127) as u32) << 23) | (significand & 0x7fffff)
}

impl FloatMode {
    fn binary(self, lhs: u32, rhs: u32, op: Op) -> u32 {
        match self {
            Self::Ieee => {
                // Keep the original f32 operations; widening IEEE intermediates
                // would change rounding and potentially NaN payloads.
                let (a, b) = (f32::from_bits(lhs), f32::from_bits(rhs));
                match op {
                    Op::Add => a + b,
                    Op::Subtract => a - b,
                    Op::Multiply => a * b,
                    Op::Divide => a / b,
                }
                .to_bits()
            }
            Self::Finite => {
                let (a, b) = (decode_finite(lhs), decode_finite(rhs));
                encode_finite(match op {
                    Op::Add => a + b,
                    Op::Subtract => a - b,
                    Op::Multiply => a * b,
                    Op::Divide => a / b,
                })
            }
        }
    }
    pub(crate) fn add(self, a: u32, b: u32) -> u32 {
        self.binary(a, b, Op::Add)
    }
    pub(crate) fn subtract(self, a: u32, b: u32) -> u32 {
        self.binary(a, b, Op::Subtract)
    }
    pub(crate) fn multiply(self, a: u32, b: u32) -> u32 {
        self.binary(a, b, Op::Multiply)
    }
    pub(crate) fn divide(self, a: u32, b: u32) -> u32 {
        self.binary(a, b, Op::Divide)
    }
    pub(crate) fn encode_int(self, value: i32) -> u32 {
        match self {
            Self::Ieee => (value as f32).to_bits(),
            Self::Finite => encode_finite(value as f64),
        }
    }
    pub(crate) fn to_int(self, value: u32, mode: u16) -> i32 {
        match self {
            Self::Ieee => {
                let f = f32::from_bits(value);
                match mode {
                    0 => f.round() as i32,
                    1 => f.ceil() as i32,
                    2 => f.floor() as i32,
                    _ => f as i32,
                }
            }
            Self::Finite => {
                let f = decode_finite(value);
                match mode {
                    0 => f.round() as i32,
                    1 => f.ceil() as i32,
                    2 => f.floor() as i32,
                    _ => f as i32,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_operations_retain_original_ieee_results() {
        let mode = FloatMode::default();
        let values = [
            0, 0x80000000, 0x00000007, 0x00800000, 0x3f800000, 0xbf800000, 0x3fcccccd, 0x442719aa,
            0x7f7fffff, 0x7f800000, 0x7fc00000,
        ];
        for a in values {
            for b in values {
                let (af, bf) = (f32::from_bits(a), f32::from_bits(b));
                for (actual, expected) in [
                    (mode.add(a, b), af + bf),
                    (mode.subtract(a, b), af - bf),
                    (mode.multiply(a, b), af * bf),
                    (mode.divide(a, b), af / bf),
                ] {
                    if expected.is_nan() {
                        assert!(f32::from_bits(actual).is_nan());
                    } else {
                        assert_eq!(actual, expected.to_bits());
                    }
                }
            }
        }
    }
    #[test]
    fn netmerc_boundary_division_saturates_and_zero_product_remains_zero() {
        let finite = FloatMode::Finite;
        let quotient = finite.divide(0x442719aa, 0x00000007);
        assert_eq!(quotient, 0x7fffffff);
        assert_eq!(finite.multiply(0, quotient), 0);
        assert_eq!(
            FloatMode::Ieee.divide(0x442719aa, 0x00000007),
            f32::INFINITY.to_bits()
        );
        assert!(f32::from_bits(FloatMode::Ieee.multiply(0, f32::INFINITY.to_bits())).is_nan());
    }
    #[test]
    fn finite_codec_boundaries_rounding_and_conversions() {
        for bits in [
            0, 0x80000000, 0x00800000, 0x3f800000, 0xbf800000, 0x7f7fffff, 0x7fffffff, 0xffffffff,
        ] {
            assert_eq!(encode_finite(decode_finite(bits)), bits);
        }
        assert_eq!(encode_finite(decode_finite(0x007fffff)), 0);
        assert_eq!(encode_finite(f64::INFINITY), 0x7fffffff);
        assert_eq!(encode_finite(f64::NEG_INFINITY), 0xffffffff);
        assert_eq!(encode_finite(1.0 + 2.0_f64.powi(-24)), 1.0f32.to_bits());
        assert_eq!(
            encode_finite(1.0 + 3.0 * 2.0_f64.powi(-24)),
            1.0f32.to_bits() + 2
        );
        assert_eq!(FloatMode::Finite.to_int(1.6f32.to_bits(), 0), 2);
        assert_eq!(FloatMode::Finite.to_int(1.6f32.to_bits(), 3), 1);
    }
    #[test]
    fn city_uses_executing_pc_and_retains_firmware_rounding_when_disabled() {
        let mut cpu = crate::Mb86233::new();
        cpu.float_mode = FloatMode::Finite;
        cpu.d = 1.6f32.to_bits();
        cpu.m = 7;
        cpu.ppc = 0x02e1;
        cpu.pc = 0x02e2;
        cpu.alu_pre(15);
        assert_eq!(cpu.alu.r1, 1);
        cpu.netmerc_city_conversion = true;
        cpu.alu_pre(15);
        assert_eq!(cpu.alu.r1, 2);
        cpu.ppc = 0x02e0;
        cpu.pc = 0x02e1;
        cpu.alu_pre(15);
        assert_eq!(cpu.alu.r1, 1);
    }
}
