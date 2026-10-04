//! Best-effort replacement for NetMerc's undumped PCM descriptors/samples.
//! Independently expressed register-driven synthesis, informed by the observed
//! TeknoModel1 fallback. NOT restored ROM data or a physical sound-chip model.
//! See docs/MODEL1_NETMERC.md for provenance, rules and deliberate differences.
//! No host resources, wall clock or filesystem: one call = one native sample.

use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

const RATE: f64 = super::SND_CPU_HZ as f64 / 224.0;

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
enum Family {
    #[default]
    Tone,
    Percussion,
    Effect,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
struct Voice {
    regs: [u8; 8],
    sample: u16,
    family: Family,
    tone: u8,
    effect: u8,
    active: bool,
    releasing: bool,
    phase: [f64; 2],
    increment: f64,
    envelope: f64,
    attack: f64,
    decay: f64,
    release: f64,
    filter_amount: f64,
    filtered: f64,
    level: f64,
    pan: [f64; 2],
    noise: u32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Chip {
    voices: [Voice; 28],
    slot: usize,
    address: usize,
    dc_input: [f64; 2],
    dc_output: [f64; 2],
}

/// Mutable recovery state is also its in-memory snapshot. No ROM or callbacks.
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Recovery {
    chips: [Chip; 2],
}

fn blep(phase: f64, step: f64) -> f64 {
    if phase < step {
        let t = phase / step;
        2.0 * t - t * t - 1.0
    } else if phase > 1.0 - step {
        let t = (phase - 1.0) / step;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

fn saw(phase: f64, step: f64) -> f64 {
    2.0 * phase - 1.0 - blep(phase, step)
}

impl Voice {
    fn pitch(&mut self) {
        let mut octave = i32::from(self.regs[3] >> 4);
        if octave >= 8 {
            octave -= 16;
        }
        let fraction = u16::from(self.regs[3] & 15) * 64 + u16::from(self.regs[2] >> 2);
        let ratio = (1.0 + f64::from(fraction) / 1024.0) * 2f64.powi(octave - 1);
        let frequency = match self.family {
            Family::Tone => 523.2511306011972 * ratio,
            _ => (90.0 + f64::from((self.sample * 37) % 420)) * (ratio * 2.0).max(0.5),
        };
        self.increment = (frequency / RATE).clamp(0.00001, 0.45);
    }

    fn level(&mut self) {
        self.level = 10f64.powf(f64::from(self.regs[5] >> 1) * -0.375 / 20.0);
    }

    fn pan(&mut self) {
        let pan = self.regs[0] >> 4;
        let attenuation = |p: u8| {
            if p & 7 == 7 {
                0.0
            } else {
                10f64.powf(f64::from(p) * -3.0 / 20.0)
            }
        };
        self.pan = match pan {
            0 => [1.0; 2],
            8 => [0.0; 2],
            1..=7 => [attenuation(pan), 1.0],
            _ => [1.0, attenuation(16 - pan)],
        };
    }

    fn key_on(&mut self, chip: usize) {
        self.family = if chip == 1 {
            Family::Effect
        } else if self.sample < 73 {
            Family::Percussion
        } else {
            Family::Tone
        };
        self.active = true;
        self.releasing = false;
        self.phase = [0.0, 0.31];
        self.filtered = 0.0;
        match self.family {
            Family::Tone => {
                self.tone = match self.sample {
                    83..=85 => 1,
                    105..=106 => 2,
                    124 => 3,
                    135.. => 4,
                    _ => 0,
                };
                (self.attack, self.release, self.filter_amount) = match self.tone {
                    1 => (0.0015, 0.9988, 0.12),
                    2 => (0.001, 0.999, 0.36),
                    3 => (0.00008, 0.9997, 0.16),
                    4 => (0.0018, 0.9987, 0.28),
                    _ => (0.0005, 0.9995, 0.22),
                };
                self.envelope = 0.0;
                self.decay = 1.0;
            }
            Family::Percussion => {
                self.effect = (self.sample & 3) as u8;
                (self.decay, self.filter_amount) = match self.effect {
                    0 => (0.99825, 0.18),
                    1 => (0.9979, 0.24),
                    2 => (0.9962, 0.07),
                    _ => (0.9986, 0.18),
                };
                self.envelope = 1.0;
            }
            Family::Effect => {
                self.effect = 4 + (self.sample & 1) as u8;
                self.decay = 0.99905 + f64::from(self.sample % 5) * 0.00012;
                self.filter_amount = if self.effect == 4 { 0.15 } else { 0.08 };
                self.envelope = 1.0;
            }
        }
        self.noise = (u32::from(self.sample).wrapping_mul(0x85eb_ca6b)
            ^ (chip as u32).wrapping_mul(0xc2b2_ae35)
            ^ 0x9e37_79b9)
            .max(1);
        self.pitch();
        self.level();
        self.pan();
    }

    fn generate(&mut self) -> f64 {
        if !self.active {
            return 0.0;
        }
        let step = match (self.family, self.effect) {
            (Family::Percussion, 0) => (48.0 + self.envelope * 115.0) / RATE,
            (Family::Percussion, 3) => (85.0 + f64::from(self.sample % 7) * 24.0) / RATE,
            _ => self.increment,
        };
        let second_step = (step
            * match self.tone {
                1 => 0.5,
                3 => 0.997,
                _ => 1.003,
            })
        .clamp(0.000005, 0.45);
        self.phase[0] = (self.phase[0] + step).fract();
        self.phase[1] = (self.phase[1] + second_step).fract();
        let phase = self.phase[0];
        let sine = (phase * TAU).sin();
        let second_sine = (self.phase[1] * TAU).sin();
        let triangle = 1.0 - 4.0 * (phase - 0.5).abs();
        let ramp = saw(phase, step);
        let square = (if phase < 0.5 { 1.0 } else { -1.0 }) + blep(phase, step)
            - blep((phase + 0.5).fract(), step);
        let wave = match self.family {
            Family::Tone => {
                let wave = match self.tone {
                    1 => 0.62 * sine + 0.28 * triangle + 0.1 * second_sine,
                    2 => 0.45 * ramp + 0.3 * sine + 0.25 * square,
                    3 => 0.44 * sine + 0.44 * second_sine + 0.12 * triangle,
                    4 => {
                        0.78 * (phase * TAU + 0.85 * (2.0 * phase * TAU).sin()).sin()
                            + 0.22 * (3.0 * phase * TAU).sin()
                    }
                    _ => 0.46 * sine + 0.27 * ramp + 0.27 * saw(self.phase[1], second_step),
                };
                self.filtered += (wave - self.filtered) * self.filter_amount;
                self.envelope = if self.releasing {
                    self.envelope * self.release
                } else {
                    (self.envelope + self.attack).min(1.0)
                };
                self.filtered
            }
            _ => {
                self.noise ^= self.noise << 13;
                self.noise ^= self.noise >> 17;
                self.noise ^= self.noise << 5;
                let noise = f64::from(self.noise as i32) / 2_147_483_648.0;
                let wave = match self.effect {
                    0 => 0.92 * sine + 0.08 * noise * self.envelope,
                    1 => {
                        self.filtered += (noise - self.filtered) * self.filter_amount;
                        0.48 * noise + 0.34 * self.filtered + 0.18 * (phase * 1.73 * TAU).sin()
                    }
                    2 => {
                        self.filtered += (noise - self.filtered) * self.filter_amount;
                        0.88 * (noise - self.filtered) + 0.12 * square
                    }
                    3 => 0.82 * sine + 0.18 * (phase * 2.0 * TAU).sin(),
                    4 => {
                        self.filtered += (noise - self.filtered) * self.filter_amount;
                        0.42 * sine + 0.58 * self.filtered
                    }
                    _ => {
                        self.filtered += (noise - self.filtered) * self.filter_amount;
                        0.46 * (noise - self.filtered) + 0.34 * ramp + 0.2 * sine
                    }
                };
                self.envelope *= if self.releasing { 0.9945 } else { self.decay };
                wave
            }
        };
        // Silence threshold belongs to a decaying/releasing voice, not the
        // first attack sample (sample 124's attack is below that threshold).
        if (self.releasing || !matches!(self.family, Family::Tone)) && self.envelope < 0.0001 {
            self.active = false;
            self.envelope = 0.0;
            return 0.0;
        }
        wave * self.envelope * self.level
    }
}

impl Recovery {
    pub(super) fn write(&mut self, chip: usize, offset: u32, data: u8) {
        let c = &mut self.chips[chip];
        match offset & 3 {
            1 => {
                if let Some(slot) = crate::multipcm::voice_slot(data) {
                    c.slot = slot;
                }
            }
            2 => c.address = usize::from(data.min(7)),
            0 => {
                let v = &mut c.voices[c.slot];
                v.regs[c.address] = data;
                match c.address {
                    0 => v.pan(),
                    1..=2 => {
                        v.sample = u16::from(v.regs[1]) | (u16::from(v.regs[2] & 1) << 8);
                        if c.address == 2 {
                            v.pitch();
                        }
                    }
                    3 => v.pitch(),
                    4 if data & 0x80 != 0 => v.key_on(chip),
                    4 if v.active => v.releasing = true,
                    5 => v.level(),
                    _ => {} // LFO writes retained but not synthesized by this fallback.
                }
            }
            _ => {}
        }
    }

    pub(super) fn generate(&mut self) -> [(i32, i32); 2] {
        std::array::from_fn(|chip| {
            let c = &mut self.chips[chip];
            let mut stereo = [0.0; 2];
            for voice in &mut c.voices {
                let value = voice.generate();
                for (side, pan) in stereo.iter_mut().zip(voice.pan) {
                    *side += value * pan;
                }
            }
            let output: [i32; 2] = std::array::from_fn(|side| {
                let input = stereo[side] * 0.105;
                let filtered = input - c.dc_input[side] + 0.9985 * c.dc_output[side];
                c.dc_input[side] = input;
                c.dc_output[side] = filtered;
                // Existing absolute route gain is 50%: compensate once, then
                // let the existing channel gain/mute/mixer own output levels.
                (filtered * 2.0 * 32767.0).round() as i32
            });
            (output[0], output[1])
        })
    }

    pub(super) fn valid(&self) -> bool {
        self.chips.iter().all(|c| {
            c.slot < 28
                && c.address < 8
                && c.dc_input.iter().chain(&c.dc_output).all(|x| x.is_finite())
                && c.voices.iter().all(|v| {
                    v.sample < 512
                        && v.tone <= 4
                        && v.effect <= 5
                        && v.phase
                            .iter()
                            .all(|x| x.is_finite() && (0.0..1.0).contains(x))
                        && [
                            v.increment,
                            v.envelope,
                            v.attack,
                            v.decay,
                            v.release,
                            v.filter_amount,
                            v.level,
                            v.pan[0],
                            v.pan[1],
                        ]
                        .iter()
                        .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
                        && v.filtered.is_finite()
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(r: &mut Recovery, chip: usize, sample: u16) {
        for (reg, value) in [
            (0, 0),
            (1, sample as u8),
            (2, (sample >> 8) as u8),
            (3, 0x10),
            (5, 1),
            (4, 0x80),
        ] {
            r.write(chip, 2, reg);
            r.write(chip, 0, value);
        }
    }

    #[test]
    fn register_classification_pitch_pan_and_release_follow_observed_rules() {
        let mut r = Recovery::default();
        for sample in [73, 83, 105, 124, 135] {
            program(&mut r, 0, sample);
            let v = &r.chips[0].voices[0];
            assert!(matches!(v.family, Family::Tone));
            assert!((v.increment - 523.2511306011972 / RATE).abs() < 1e-12);
        }
        for sample in 0..4 {
            program(&mut r, 0, sample);
            assert!(matches!(r.chips[0].voices[0].family, Family::Percussion));
            assert_eq!(r.chips[0].voices[0].effect, sample as u8);
            program(&mut r, 1, sample);
            assert!(matches!(r.chips[1].voices[0].family, Family::Effect));
            assert_eq!(r.chips[1].voices[0].effect, 4 + (sample & 1) as u8);
        }
        program(&mut r, 0, 83);
        r.write(0, 2, 0);
        for (pan, expected) in [
            (0, [1.0, 1.0]),
            (0x70, [0.0, 1.0]),
            (0x80, [0.0, 0.0]),
            (0x90, [1.0, 0.0]),
        ] {
            r.write(0, 0, pan);
            assert_eq!(r.chips[0].voices[0].pan, expected);
        }
        r.write(0, 0, 0);
        for _ in 0..1000 {
            r.generate();
        }
        r.write(0, 2, 4);
        r.write(0, 0, 0);
        for _ in 0..20000 {
            r.generate();
        }
        assert!(!r.chips[0].voices[0].active);
        assert!(r.valid());
    }

    #[test]
    fn slow_attack_does_not_get_cut_off_by_release_silence_threshold() {
        let mut r = Recovery::default();
        program(&mut r, 0, 124);
        r.generate();
        assert!(r.chips[0].voices[0].active);
        assert_eq!(r.chips[0].voices[0].envelope, 0.00008);
        assert!((0..1000).any(|_| r.generate()[0] != (0, 0)));
    }

    #[test]
    fn mid_voice_snapshot_preserves_both_chips_noise_filters_and_release() {
        for sample in [0, 1, 2, 3, 73, 83, 105, 124, 135] {
            let mut original = Recovery::default();
            program(&mut original, 0, sample);
            program(&mut original, 1, sample);
            for _ in 0..731 {
                original.generate();
            }
            original.write(1, 2, 4);
            original.write(1, 0, 0);
            let bytes = bincode::serialize(&original).unwrap();
            let mut restored: Recovery = bincode::deserialize(&bytes).unwrap();
            assert!(restored.valid());
            for _ in 0..1000 {
                assert_eq!(original.generate(), restored.generate());
            }
            assert_eq!(
                bincode::serialize(&original).unwrap(),
                bincode::serialize(&restored).unwrap()
            );
        }
    }
}
