//! Bounded MPEG-1 Layer II decoder for the Z80 DSB, no host resources.
//! Adapted from MAME mpeg_audio.cpp, BSD-3-Clause, copyright Olivier Galibert.
//! Joint stereo is explicitly unsupported pending a separate reference audit.
mod tables;
use tables::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Error {
    Bounds,
    Truncated,
    InvalidHeader,
    JointStereo,
    InvalidAllocation,
    InvalidState,
}

/// History only. Immutable data and derived cosine coefficients are excluded.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct State {
    history: Vec<f64>,
    cursor: [usize; 2],
}
impl Default for State {
    fn default() -> Self {
        Self {
            history: vec![0.0; 2048],
            cursor: [512; 2],
        }
    }
}
pub struct Decoder {
    state: State,
    cosine: [[f64; 32]; 32],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Interleaved, 1152 samples per channel. Mono remains mono here.
    pub samples: Vec<i16>,
    pub channels: usize,
    pub sample_rate: u32,
    /// First bit after decoded payload, like MAME; padding/ancillary data is
    /// skipped by the next sync search, not assumed to be part of audio data.
    pub next_bit: usize,
}
impl Default for Decoder {
    fn default() -> Self {
        Self {
            state: State::default(),
            cosine: std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    (i as f64 * (2 * j + 1) as f64 * std::f64::consts::PI / 64.0).cos()
                })
            }),
        }
    }
}
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    limit: usize,
}
impl Bits<'_> {
    fn get(&mut self, count: usize) -> Result<usize, Error> {
        if count > 16 || count > self.limit.saturating_sub(self.pos) {
            return Err(Error::Truncated);
        }
        let mut value = 0;
        for _ in 0..count {
            value = value * 2 + usize::from((self.data[self.pos / 8] >> (7 - self.pos % 8)) & 1);
            self.pos += 1;
        }
        Ok(value)
    }
}
impl Decoder {
    pub fn snapshot(&self) -> State {
        self.state.clone()
    }
    pub fn restore(&mut self, state: &State) -> Result<(), Error> {
        if state.history.len() != 2048
            || !state.history.iter().all(|x| x.is_finite())
            || !state.cursor.iter().all(|&x| x <= 512 && x % 32 == 0)
        {
            return Err(Error::InvalidState);
        }
        self.state = state.clone();
        Ok(())
    }
    pub fn clear(&mut self) {
        self.state = State::default();
    }
    /// Decode one complete frame, committing history only on success. Errors
    /// never leave partially updated history or expose partial PCM. `None`
    /// means no accepted Layer II sync before the limit. CRC is consumed, not
    /// checked, matching the reference. Layer I/III/AMM/MPEG-2 are not accepted.
    pub fn decode(
        &mut self,
        data: &[u8],
        start: usize,
        limit: usize,
    ) -> Result<Option<Frame>, Error> {
        if start > limit || limit > data.len().checked_mul(8).ok_or(Error::Bounds)? {
            return Err(Error::Bounds);
        }
        let mut r = Bits {
            data,
            pos: start,
            limit,
        };
        let mut sync = 0;
        let mut count = 0;
        let found = loop {
            if r.pos == r.limit {
                break false;
            }
            sync = ((sync << 1) | r.get(1)?) & 0xfff;
            count += 1;
            if count >= 12 && sync == 0xfff {
                let candidate = r.pos;
                if limit - candidate < 3 {
                    return Err(Error::Truncated);
                }
                if r.get(3)? == 6 {
                    break true;
                }
                r.pos = candidate; // scan rejected variants one bit at a time
            }
        };
        if !found {
            return Ok(None);
        }
        let protected = r.get(1)? == 0;
        let bitrate = r.get(4)?;
        let rate = r.get(2)?;
        r.get(2)?; // padding/private
        let mode = r.get(2)?;
        r.get(6)?; // mode extension, copyright/original, emphasis
        if protected {
            r.get(16)?;
        }
        if mode == 1 {
            return Err(Error::JointStereo);
        }
        let channels = if mode == 3 { 1 } else { 2 };
        let param = PARAM[channels - 1][rate][bitrate];
        if param < 0 {
            return Err(Error::InvalidHeader);
        }
        let param = param as usize;
        let bands = BANDS[param];
        let mut allocation = [[0usize; 32]; 2];
        for band in 0..bands {
            for channel in 0..channels {
                let index = r.get(ALLOC_BITS[param][band])?;
                let value = ALLOC[param][band][index];
                if !(0..18).contains(&value) {
                    return Err(Error::InvalidAllocation);
                }
                allocation[channel][band] = value as usize;
            }
        }
        let mut scfsi = [[0; 32]; 2];
        for band in 0..bands {
            for channel in 0..channels {
                if allocation[channel][band] != 0 {
                    scfsi[channel][band] = r.get(2)?;
                }
            }
        }
        let mut scales = [[[0.0; 32]; 3]; 2];
        for band in 0..bands {
            for channel in 0..channels {
                if allocation[channel][band] == 0 {
                    continue;
                }
                let a = r.get(6)?;
                let values = match scfsi[channel][band] {
                    0 => [a, r.get(6)?, r.get(6)?],
                    1 => [a, a, r.get(6)?],
                    2 => [a, a, a],
                    _ => {
                        let b = r.get(6)?;
                        [a, b, b]
                    }
                };
                for step in 0..3 {
                    scales[channel][step][band] = SCALEFACTORS[values[step]];
                }
            }
        }
        // Transactional working copy: truncated packets must not poison the
        // next song or a retry with more input. This deliberately strengthens
        // MAME's partial-history behavior on a late failed decode.
        let mut state = self.state.clone();
        let mut samples = Vec::with_capacity(1152 * channels);
        for upper in 0..3 {
            for _ in 0..4 {
                let mut sub = [[[0.0; 32]; 3]; 2];
                for band in 0..bands {
                    for channel in 0..channels {
                        let q = allocation[channel][band];
                        if q == 0 {
                            continue;
                        }
                        let (bits, modulus, cube) = quantization(q);
                        let raw = if modulus != 0 {
                            let v = r.get(cube)?;
                            [
                                v % modulus,
                                (v / modulus) % modulus,
                                (v / modulus / modulus) % modulus,
                            ]
                        } else {
                            [r.get(bits)?, r.get(bits)?, r.get(bits)?]
                        };
                        let div = (1usize << (bits - 1)) as f64;
                        let (offset, scale) = if modulus != 0 {
                            (0.5, 1.0 / (modulus as f64 / (1usize << bits) as f64))
                        } else {
                            (1.0 / div, 1.0 / (1.0 - 1.0 / (1usize << bits) as f64))
                        };
                        for step in 0..3 {
                            sub[channel][step][band] = ((raw[step] as f64 - div) / div + offset)
                                * scale
                                * scales[channel][upper][band];
                        }
                    }
                }
                for step in 0..3 {
                    let mut pcm = [[0i16; 32]; 2];
                    for channel in 0..channels {
                        pcm[channel] = self.synthesize(&mut state, channel, &sub[channel][step]);
                    }
                    for i in 0..32 {
                        for channel in 0..channels {
                            samples.push(pcm[channel][i]);
                        }
                    }
                }
            }
        }
        self.state = state;
        Ok(Some(Frame {
            samples,
            channels,
            sample_rate: [44100, 48000, 32000, 0][rate],
            next_bit: r.pos,
        }))
    }
    fn synthesize(&self, state: &mut State, channel: usize, input: &[f64; 32]) -> [i16; 32] {
        let pos = state.cursor[channel];
        let h = &mut state.history[channel * 1024..(channel + 1) * 1024];
        for i in 0..32 {
            let mut sum = 0.0;
            for j in 0..32 {
                sum += input[j] * self.cosine[i][j];
            }
            h[pos + i] = sum;
        }
        let mut out = [0.0; 32];
        let hread = &h[pos + 16..];
        for j in (0..512).step_by(64) {
            for i in 0..16 {
                out[i] += hread[i + j] * FILTER[i + j] - hread[32 - i + j] * FILTER[32 + i + j];
            }
            out[16] -= hread[16 + j] * FILTER[48 + j];
            for i in 17..32 {
                out[i] -= hread[32 - i + j] * FILTER[i + j] + hread[i + j] * FILTER[32 + i + j];
            }
        }
        if pos == 0 {
            h.copy_within(0..480, 544);
            state.cursor[channel] = 512;
        } else {
            state.cursor[channel] -= 32;
        }
        out.map(|v| (v * 32768.0 + 0.5).clamp(-32768.0, 32767.0) as i16)
    }
}
fn quantization(q: usize) -> (usize, usize, usize) {
    match q {
        1 => (2, 3, 5),
        2 => (3, 5, 7),
        3 => (3, 0, 0),
        4 => (4, 9, 10),
        _ => (q - 1, 0, 0),
    }
}
#[cfg(test)]
pub(crate) mod tests;
