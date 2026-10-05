// license: BSD-3-Clause
// Copyright (c) 2021, Aaron Giles. All rights reserved.
// Specialized Rust adaptation of YMFM OPN2 operators/channel output.
// Provenance and license: ../ym3438.rs, LICENSES/YMFM-BSD-3-Clause.txt.
use super::tables::{DETUNE, INCREMENT, PM_SHIFTS, POWER, SIN};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Envelope {
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Operator {
    phase: u32,
    attenuation: u16,
    envelope: Envelope,
    inverted: bool,
    key: bool,
}

impl Default for Operator {
    fn default() -> Self {
        Self {
            phase: 0,
            attenuation: 0x3ff,
            envelope: Envelope::Release,
            inverted: false,
            key: false,
        }
    }
}

/// Derived per-operator parameters, rebuilt from registers, never serialized.
#[derive(Clone, Copy)]
pub(super) struct Params {
    frequency: u32,
    detune: i32,
    multiple: u32,
    total_level: u32,
    sustain: u16,
    rates: [u32; 4],
    ssg: u8,
    am: bool,
}

impl Params {
    fn new(regs: &[u8; 512], channel: usize, slot: usize) -> Self {
        let ch = channel % 3 + 0x100 * (channel / 3);
        // Physical register offsets in logical algorithm order (1,2,3,4).
        let op = ch + [0, 8, 4, 12][slot];
        let mut freq = ch;
        if channel == 2 && regs[0x27] & 0xc0 != 0 {
            // Special frequencies for operators 1/2/3; operator 4 uses normal.
            freq = [9, 10, 8, 2][slot];
        }
        let frequency = (u32::from(regs[0xa4 + freq] & 63) << 8) | u32::from(regs[0xa0 + freq]);
        let keycode = ((frequency >> 10) & 15) * 2 | ((0xfe80 >> ((frequency >> 7) & 15)) & 1);
        let dt = (regs[0x30 + op] >> 4) & 7;
        let detune = i32::from(DETUNE[keycode as usize * 4 + usize::from(dt & 3)])
            * if dt & 4 != 0 { -1 } else { 1 };
        let ksr = keycode >> ((regs[0x50 + op] >> 6) ^ 3);
        let rate = |raw: u32| if raw == 0 { 0 } else { (raw + ksr).min(63) };
        let sl = u16::from(regs[0x80 + op] >> 4);
        Self {
            frequency,
            detune,
            multiple: (u32::from(regs[0x30 + op] & 15) * 2).max(1),
            total_level: u32::from(regs[0x40 + op] & 127) << 3,
            sustain: (sl | ((sl + 1) & 16)) << 5,
            rates: [
                rate(u32::from(regs[0x50 + op] & 31) * 2),
                rate(u32::from(regs[0x60 + op] & 31) * 2),
                rate(u32::from(regs[0x70 + op] & 31) * 2),
                rate(u32::from(regs[0x80 + op] & 15) * 4 + 2),
            ],
            ssg: regs[0x90 + op] & 15,
            am: regs[0x60 + op] & 0x80 != 0,
        }
    }

    fn phase_step(&self, sensitivity: u8, pm: i32) -> u32 {
        let mut fnum = (self.frequency & 0x7ff) << 1;
        if sensitivity != 0 {
            let shifts = PM_SHIFTS[usize::from(sensitivity) * 8 + (pm.unsigned_abs() as usize & 7)];
            let bits = (self.frequency >> 4) & 127;
            let mut adjust = (bits >> (shifts & 15)) + (bits >> (shifts >> 4));
            if sensitivity > 5 {
                adjust <<= sensitivity - 5;
            }
            adjust >>= 2;
            fnum = if pm < 0 {
                fnum.wrapping_sub(adjust)
            } else {
                fnum.wrapping_add(adjust)
            } & 0xfff;
        }
        let step = (fnum << ((self.frequency >> 11) & 7)) >> 2;
        ((step.wrapping_add(self.detune as u32) & 0x1ffff) * self.multiple) >> 1
    }
}

pub(super) type ParamsGrid = [[Params; 4]; 6];

pub(super) fn parameters(regs: &[u8; 512]) -> ParamsGrid {
    std::array::from_fn(|ch| std::array::from_fn(|op| Params::new(regs, ch, op)))
}

impl Operator {
    fn attack(&mut self, p: Params, restart: bool) {
        if self.envelope == Envelope::Attack {
            return;
        }
        self.envelope = Envelope::Attack;
        if !restart {
            self.inverted = p.ssg & 12 == 12;
            self.phase = 0;
        }
        if p.rates[0] >= 62 {
            self.attenuation = 0;
        }
    }

    fn prepare(&mut self, p: Params, key: bool) -> bool {
        if self.key != key {
            self.key = key;
            if key {
                self.attack(p, false);
            } else if self.envelope != Envelope::Release {
                self.envelope = Envelope::Release;
                if self.inverted {
                    self.attenuation = 0x200u16.wrapping_sub(self.attenuation) & 0x3ff;
                    self.inverted = false;
                }
            }
        }
        self.envelope != Envelope::Release || self.attenuation < 0x380
    }

    fn clock_ssg(&mut self, p: Params) {
        if p.ssg & 8 == 0 {
            self.inverted = false;
            return;
        }
        if self.attenuation & 0x200 == 0 {
            return;
        }
        if p.ssg & 1 != 0 {
            self.inverted = ((p.ssg >> 2) ^ (p.ssg >> 1)) & 1 != 0;
            if self.envelope != Envelope::Attack {
                self.attenuation = if self.inverted { 0x200 } else { 0x3ff };
            }
        } else {
            self.inverted ^= p.ssg & 2 != 0;
            if matches!(self.envelope, Envelope::Decay | Envelope::Sustain) {
                self.attack(p, true);
            }
            if p.ssg & 2 == 0 {
                self.phase = 0;
            }
        }
        if self.envelope == Envelope::Release {
            self.attenuation = 0x3ff;
        }
    }

    fn clock_envelope(&mut self, p: Params, counter: u32) {
        if self.envelope == Envelope::Attack && self.attenuation == 0 {
            self.envelope = Envelope::Decay;
        }
        if self.envelope == Envelope::Decay && self.attenuation >= p.sustain {
            self.envelope = Envelope::Sustain;
        }
        let rate = p.rates[self.envelope as usize];
        let shift = rate >> 2;
        let counter = counter.wrapping_shl(shift);
        if counter & 0x7ff != 0 {
            return;
        }
        let index = (counter >> shift.max(11)) & 7;
        let increment = (INCREMENT[rate as usize] >> (4 * index)) & 15;
        if self.envelope == Envelope::Attack {
            if rate < 62 {
                // Reference uses two's-complement arithmetic and arithmetic shift.
                self.attenuation = (i32::from(self.attenuation)
                    + ((!i32::from(self.attenuation) * increment as i32) >> 4))
                    as u16;
            }
        } else {
            if p.ssg & 8 == 0 {
                self.attenuation += increment as u16;
            } else if self.attenuation < 0x200 {
                self.attenuation += 4 * increment as u16;
            }
            self.attenuation = self.attenuation.min(0x3ff);
        }
    }

    fn volume(&self, p: Params, modulation: i32, am: u32) -> i32 {
        if self.attenuation > 0x380 {
            return 0;
        }
        let phase = (self.phase >> 10).wrapping_add(modulation as u32);
        let index = if phase & 0x100 != 0 { !phase } else { phase } & 255;
        let mut envelope = u32::from(self.attenuation);
        if self.inverted {
            envelope = 0x200u32.wrapping_sub(envelope) & 0x3ff;
        }
        if p.am {
            envelope += am;
        }
        envelope = (envelope + p.total_level).min(0x3ff);
        let attenuation = u32::from(SIN[index as usize]) + (envelope << 2);
        let volume = (u32::from(POWER[(attenuation & 255) as usize]) >> (attenuation >> 8)) as i32;
        if phase & 0x200 != 0 {
            -volume
        } else {
            volume
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Synthesis {
    operators: [[Operator; 4]; 6],
    feedback: [[i16; 3]; 6],
    envelope_counter: u32,
    lfo_counter: u32,
    lfo_am: u8,
    prepare_count: u16,
    active_channels: u8,
    pub(super) modified: bool,
}

impl Default for Synthesis {
    fn default() -> Self {
        Self {
            operators: [[Operator::default(); 4]; 6],
            feedback: [[0; 3]; 6],
            envelope_counter: 0,
            lfo_counter: 0,
            lfo_am: 0,
            prepare_count: 0,
            active_channels: 63,
            modified: true,
        }
    }
}

impl Synthesis {
    pub(super) fn reset(&mut self) {
        self.operators = [[Operator::default(); 4]; 6];
        self.feedback = [[0; 3]; 6];
        // Reference reset does not reset free-running envelope/LFO counters.
        self.modified = true;
    }

    pub(super) fn valid(&self) -> bool {
        self.lfo_am <= 63
            && self.envelope_counter & 3 != 3
            && self.prepare_count <= 4096
            && self.active_channels & !63 == 0
            && self
                .operators
                .iter()
                .flatten()
                .all(|op| op.attenuation <= 0x3ff)
            && self
                .feedback
                .iter()
                .flatten()
                .all(|&value| (-8192..=8192).contains(&value))
    }

    fn clock_lfo(&mut self, reg: u8) -> i32 {
        if reg & 8 == 0 {
            self.lfo_counter = 0;
            self.lfo_am = 63;
            return 0;
        }
        let subcount = self.lfo_counter & 255;
        self.lfo_counter = self.lfo_counter.wrapping_add(1);
        if subcount >= [109, 78, 72, 68, 63, 45, 9, 6][usize::from(reg & 7)] {
            self.lfo_counter = self.lfo_counter.wrapping_add(0x101 - subcount);
        }
        self.lfo_am = ((self.lfo_counter >> 8) & 63) as u8;
        if self.lfo_counter & (1 << 14) == 0 {
            self.lfo_am ^= 63;
        }
        let mut pm = ((self.lfo_counter >> 10) & 7) as i32;
        if self.lfo_counter & (1 << 13) != 0 {
            pm ^= 7;
        }
        if self.lfo_counter & (1 << 14) != 0 {
            -pm
        } else {
            pm
        }
    }

    pub(super) fn sample(
        &mut self,
        regs: &[u8; 512],
        params: &ParamsGrid,
        keys: &[u8; 6],
        csm: bool,
        dac: u16,
        dac_enabled: bool,
    ) -> [i32; 2] {
        let prepare = self.modified || self.prepare_count >= 4096;
        if prepare {
            self.active_channels = 0;
            for ch in 0..6 {
                for slot in 0..4 {
                    let key = keys[ch] & (1 << slot) != 0 || (ch == 2 && csm);
                    if self.operators[ch][slot].prepare(params[ch][slot], key) {
                        self.active_channels |= 1 << ch;
                    }
                }
            }
            self.modified = false;
            self.prepare_count = 0;
        } else {
            self.prepare_count += 1;
        }
        self.envelope_counter = self.envelope_counter.wrapping_add(1);
        if self.envelope_counter & 3 == 3 {
            self.envelope_counter = self.envelope_counter.wrapping_add(1);
        }
        let pm = self.clock_lfo(regs[0x22]);
        let mut output = [0; 2];
        for (ch, channel_params) in params.iter().enumerate() {
            let offset = ch % 3 + 0x100 * (ch / 3);
            self.feedback[ch][0] = self.feedback[ch][1];
            self.feedback[ch][1] = self.feedback[ch][2];
            for (slot, &p) in channel_params.iter().enumerate() {
                let op = &mut self.operators[ch][slot];
                op.clock_ssg(p);
                if self.envelope_counter & 3 == 0 {
                    op.clock_envelope(p, self.envelope_counter >> 2);
                }
                op.phase = op
                    .phase
                    .wrapping_add(p.phase_step(regs[0xb4 + offset] & 7, pm));
            }
            if ch == 5 && dac_enabled {
                let value = ((dac << 7) as i16 >> 7) as i32;
                if regs[0xb4 + offset] & 128 != 0 {
                    output[0] += value;
                }
                if regs[0xb4 + offset] & 64 != 0 {
                    output[1] += value;
                }
            } else if self.active_channels & (1 << ch) != 0 {
                let value = self.channel_output(ch, regs, channel_params);
                if regs[0xb4 + offset] & 128 != 0 {
                    output[0] += value;
                }
                if regs[0xb4 + offset] & 64 != 0 {
                    output[1] += value;
                }
            }
        }
        // YM3438, not YM2612: no DAC discontinuity. Signed division truncates.
        [output[0] * 128 / 6, output[1] * 128 / 6]
    }

    fn channel_output(&mut self, ch: usize, regs: &[u8; 512], params: &[Params; 4]) -> i32 {
        let offset = ch % 3 + 0x100 * (ch / 3);
        let feedback = (regs[0xb0 + offset] >> 3) & 7;
        let am_shift = (1 << (((regs[0xb4 + offset] >> 4) & 3) ^ 3)) - 1;
        let am = (u32::from(self.lfo_am) << 1) >> am_shift;
        let modulation = if feedback == 0 {
            0
        } else {
            (i32::from(self.feedback[ch][0]) + i32::from(self.feedback[ch][1])) >> (10 - feedback)
        };
        let ops = &self.operators[ch];
        let mut values = [0i32; 8];
        values[1] = ops[0].volume(params[0], modulation, am);
        self.feedback[ch][2] = values[1] as i16;
        if regs[0xb4 + offset] & 0xc0 == 0 {
            return 0;
        }
        // Inputs to operators 2/3/4, then mask of extra carrier outputs 1/2/3.
        let (i2, i3, i4, carriers) = [
            (1, 2, 3, 0),
            (0, 5, 3, 0),
            (0, 2, 6, 0),
            (1, 0, 7, 0),
            (1, 0, 3, 2),
            (1, 1, 1, 6),
            (1, 0, 0, 6),
            (0, 0, 0, 7),
        ][usize::from(regs[0xb0 + offset] & 7)];
        values[2] = ops[1].volume(params[1], values[i2] >> 1, am);
        values[5] = values[1] + values[2];
        values[3] = ops[2].volume(params[2], values[i3] >> 1, am);
        values[6] = values[1] + values[3];
        values[7] = values[2] + values[3];
        let mut result = ops[3].volume(params[3], values[i4] >> 1, am) >> 5;
        for slot in 0..3 {
            if carriers & (1 << slot) != 0 {
                result = (result + (values[slot + 1] >> 5)).clamp(-257, 256);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::super::Ym3438;
    use super::*;

    #[test]
    fn invalid_synthesis_state_is_rejected_before_mutation() {
        let mut chip = Ym3438::new();
        chip.advance(157);
        let valid = chip.snapshot();
        let changes: &[fn(&mut Synthesis)] = &[
            |s| s.operators[5][3].attenuation = 1024,
            |s| s.feedback[2][1] = 8193,
            |s| s.feedback[0][2] = -8193,
            |s| s.envelope_counter = 3,
            |s| s.prepare_count = 4097,
            |s| s.lfo_am = 64,
            |s| s.active_channels = 64,
        ];
        for change in changes {
            let mut invalid = valid.clone();
            change(&mut invalid.synthesis);
            assert!(chip.restore(&invalid).is_err());
            assert_eq!(chip.snapshot(), valid);
        }
    }
}
