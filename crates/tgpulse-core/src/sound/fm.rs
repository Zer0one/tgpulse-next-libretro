//! YM3438 + causal native-to-MultiPCM-rate bridge, driven by 10 MHz CPU clocks.
//! Each 224-clock output integrates the held FM output over that interval.
//! Native FM frames arrive every 180 CPU clocks (8 MHz / 144). This box filter
//! is deterministic and causal, not MAME's resampler or a high-order sinc filter.
use crate::ym3438::{State as ChipState, Ym3438};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Conversion {
    /// Fractional 8 MHz clocks in fifths of a 10 MHz CPU clock conversion.
    fifths: u8,
    pcm_remaining: u16,
    last: [i32; 2],
    area: [i64; 2],
}

/// FM chip plus converter only, NOT CPU/PCM/UART or a complete sound-board save.
/// No ROMs, output preferences, pending host samples, paths or devices included.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FmPathState {
    chip: ChipState,
    conversion: Conversion,
}

pub(super) struct FmPath {
    chip: Ym3438,
    conversion: Conversion,
}

impl FmPath {
    pub fn new() -> Self {
        Self {
            chip: Ym3438::new(),
            conversion: Conversion {
                fifths: 0,
                pcm_remaining: 224,
                last: [0; 2],
                area: [0; 2],
            },
        }
    }

    pub fn read(&self, port: u8) -> u8 {
        self.chip.read(port)
    }
    pub fn write(&mut self, port: u8, value: u8) {
        self.chip.write(port, value);
    }

    pub fn advance(&mut self, mut cycles: usize, mut emit: impl FnMut([i32; 2])) {
        while cycles != 0 {
            // The native-frame boundary is an exact CPU clock for this ratio.
            let to_fm = (usize::from(self.chip.clocks_until_sample()) * 5
                - usize::from(self.conversion.fifths))
                / 4;
            let step = cycles
                .min(to_fm)
                .min(usize::from(self.conversion.pcm_remaining));
            for side in 0..2 {
                self.conversion.area[side] += i64::from(self.conversion.last[side]) * step as i64;
            }
            let fifths = step * 4 + usize::from(self.conversion.fifths);
            self.conversion.fifths = (fifths % 5) as u8;
            self.chip
                .advance_with_output((fifths / 5) as u32, |frame| self.conversion.last = frame);
            cycles -= step;
            self.conversion.pcm_remaining -= step as u16;
            if self.conversion.pcm_remaining == 0 {
                emit(self.conversion.area.map(|sum| (sum / 224) as i32));
                self.conversion.area = [0; 2];
                self.conversion.pcm_remaining = 224;
            }
        }
    }

    pub fn snapshot(&self) -> FmPathState {
        FmPathState {
            chip: self.chip.snapshot(),
            conversion: self.conversion.clone(),
        }
    }

    pub fn restore(&mut self, state: &FmPathState) -> Result<(), &'static str> {
        let c = &state.conversion;
        if c.fifths >= 5
            || !(1..=224).contains(&c.pcm_remaining)
            || c.last.iter().any(|v| !(-32896..=32768).contains(v))
            || c.area
                .iter()
                .any(|v| v.unsigned_abs() > 32896 * u64::from(224 - c.pcm_remaining))
        {
            return Err("invalid FM conversion state");
        }
        let mut chip = Ym3438::new();
        chip.restore(&state.chip)?;
        let next = u32::from(chip.clocks_until_sample()) * 5 - u32::from(c.fifths);
        if next % 4 != 0 || (next / 4) % 4 != u32::from(c.pcm_remaining) % 4 {
            return Err("inconsistent FM/PCM clock phases");
        }
        self.chip = chip;
        self.conversion = c.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dac() -> FmPath {
        let mut fm = FmPath::new();
        for (r, v) in [(0x2b, 0x80), (0x2a, 255)] {
            fm.write(0, r);
            fm.write(1, v);
        }
        fm
    }
    #[test]
    fn native_rate_bridge_integrates_edges_and_restores_mid_interval() {
        let mut fm = dac();
        let mut samples = Vec::new();
        fm.advance(224, |frame| samples.push(frame));
        assert_eq!(samples, [[5418 * 44 / 224; 2]]);
        fm.advance(224, |frame| samples.push(frame));
        assert_eq!(samples[1], [5418; 2]);
        fm.advance(113, |_| {});
        let state: FmPathState =
            bincode::deserialize(&bincode::serialize(&fm.snapshot()).unwrap()).unwrap();
        let mut restored = FmPath::new();
        restored.restore(&state).unwrap();
        for target in [&mut fm, &mut restored] {
            target.write(0, 0x2a);
            target.write(1, 0);
        }
        let mut a = Vec::new();
        let mut b = Vec::new();
        fm.advance(10081, |f| a.push(f));
        for _ in 0..10081 {
            restored.advance(1, |f| b.push(f));
        }
        assert_eq!(a, b);
        assert_eq!(fm.snapshot(), restored.snapshot());
        assert!(a.iter().any(|s| s[0] < 0));
    }

    #[test]
    fn bridge_rejects_invalid_state_atomically() {
        let mut fm = dac();
        fm.advance(113, |_| {});
        let good = fm.snapshot();
        let changes: &[fn(&mut Conversion)] = &[
            |s| s.fifths = 5,
            |s| s.pcm_remaining = 0,
            |s| s.pcm_remaining = 225,
            |s| s.last[0] = 40000,
            |s| s.area[0] = i64::MAX,
            |s| s.fifths = (s.fifths + 1) % 5,
            |s| s.pcm_remaining -= 1,
        ];
        for change in changes {
            let mut bad = good.clone();
            change(&mut bad.conversion);
            assert!(fm.restore(&bad).is_err());
            assert_eq!(fm.snapshot(), good);
        }
    }
}
