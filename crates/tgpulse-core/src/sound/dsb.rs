//! Causal 32 kHz DSB -> 10 MHz / 224 PCM-rate bridge. Like the FM path,
//! integrate held output over each destination interval, not host wall time.
//! Time is in 20 MHz ticks: DSB samples span 625 ticks, PCM spans 448.
use crate::dsbz80::AudioSample;
use std::collections::VecDeque;

#[derive(Clone, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct Conversion {
    time: u64,
    last: [i32; 2], // PCM16 * 128, preserving the DSB volume fraction
    area: [i64; 2],
    pending: VecDeque<(u64, [i32; 2])>,
}

impl Conversion {
    pub fn time(&self) -> u64 {
        self.time
    }
    pub fn latest_sample_time(&self) -> u64 {
        self.pending.back().map_or(0, |p| p.0)
    }
    pub fn push(&mut self, sample: AudioSample) {
        self.pending.push_back((
            sample.clock * 5,
            sample.channels.map(|v| (v * 4194304.0) as i32),
        ));
    }

    pub fn advance(&mut self, clocks: usize, mut emit: impl FnMut([i32; 2])) {
        let end = self.time + clocks as u64 * 2;
        while self.time < end {
            let boundary = self.time + 448 - self.time % 448;
            let next = end
                .min(boundary)
                .min(self.pending.front().map_or(end, |p| p.0));
            for side in 0..2 {
                self.area[side] += i64::from(self.last[side]) * (next - self.time) as i64;
            }
            self.time = next;
            if self.time == boundary {
                emit(self.area.map(|a| (a / (448 * 128)) as i32));
                self.area = [0; 2];
            }
            if self.pending.front().is_some_and(|p| p.0 == self.time) {
                self.last = self.pending.pop_front().unwrap().1;
            }
        }
    }

    pub fn valid(&self) -> bool {
        let limit = 32768 * 128;
        let valid_sample = |s: &[i32; 2]| s.iter().all(|v| v.unsigned_abs() <= limit);
        let mut previous = self.time;
        self.time % 2 == 0
            && valid_sample(&self.last)
            && self
                .area
                .iter()
                .all(|a| a.unsigned_abs() <= u64::from(limit) * (self.time % 448))
            && self.pending.iter().all(|(t, s)| {
                let valid = *t > previous && *t % 625 == 0 && valid_sample(s);
                previous = *t;
                valid
            })
    }
}

/// DSB and conversion history only, not the 68000/PCM/FM or whole machine.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DsbPathState {
    pub(super) board: crate::dsbz80::State,
    pub(super) conversion: Conversion,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_clock_hold_and_serialized_continuation() {
        let mut c = Conversion::default();
        c.push(AudioSample {
            clock: 125,
            channels: [0.5, -0.5],
        });
        let mut out = vec![];
        c.advance(224, |s| out.push(s));
        assert_eq!(out, vec![[0, 0]]);
        c.advance(224, |s| out.push(s));
        assert_eq!(out[1], [16384 * 271 / 448, -16384 * 271 / 448]);
        c.advance(51, |_| {});
        assert!(c.valid());
        let mut restored: Conversion =
            bincode::deserialize(&bincode::serialize(&c).unwrap()).unwrap();
        let mut a = vec![];
        let mut b = vec![];
        c.advance(10000, |s| a.push(s));
        for _ in 0..10000 {
            restored.advance(1, |s| b.push(s));
        }
        assert_eq!(a, b);
        assert_eq!(c, restored);
    }
}
