//! Clocked MPEG playback, following MAME dsbz80 (BSD-3-Clause).
//! No host queue: the owner consumes output as the emulated clock advances.
use super::{Error, Playback};
use crate::mpeg;
use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: u32 = 32_000;
const CLOCKS_PER_SAMPLE: u8 = (super::CPU_HZ / SAMPLE_RATE) as u8;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct State {
    decoder: mpeg::State,
    pcm: Vec<i16>,
    cursor: usize,
    phase: u8,
    rom_size: usize,
}

pub(super) struct Audio {
    rom: Vec<u8>,
    decoder: mpeg::Decoder,
    pcm: Vec<i16>,
    cursor: usize,
    phase: u8,
}

impl Audio {
    pub fn new(rom: Vec<u8>) -> Result<Self, Error> {
        // 24-bit byte address registers. Region sizes remain the loader's job.
        if rom.is_empty() || rom.len() > 0x1000000 {
            return Err(Error::MpegRegionSize);
        }
        Ok(Self {
            rom,
            decoder: mpeg::Decoder::default(),
            pcm: vec![],
            cursor: 0,
            phase: 0,
        })
    }

    pub fn stop(&mut self) {
        self.pcm.clear();
        self.cursor = 0;
    }

    pub fn reset(&mut self) {
        self.stop();
        self.phase = 0;
        // MAME device_reset does not clear decoder history.
    }

    pub fn tick(&mut self, p: &mut Playback) -> Result<Option<[f32; 2]>, Error> {
        self.phase += 1;
        if self.phase != CLOCKS_PER_SAMPLE {
            return Ok(None);
        }
        self.phase = 0;
        self.sample(p).map(Some)
    }

    fn sample(&mut self, p: &mut Playback) -> Result<[f32; 2], Error> {
        // At most one loop retry per sample: malformed/empty loops cannot spin.
        for attempt in 0..2 {
            if self.cursor < self.pcm.len() {
                let l = self.pcm[self.cursor];
                let r = self.pcm[self.cursor + 1];
                self.cursor += 2;
                let pair = match p.pan {
                    0 => [l, r],
                    1 => [l, l],
                    2 => [r, r],
                    _ => return Err(Error::UnsupportedPan(p.pan)),
                };
                // Same power-of-two normalization as MAME stream.put_int.
                return Ok(
                    pair.map(|s| (i32::from(s) * i32::from(p.volume)) as f32 / (32768.0 * 128.0))
                );
            }
            if p.mode == 0 {
                return Ok([0.0; 2]);
            }
            let start = p.bit_position as usize;
            let limit = p.end as usize * 8;
            if start > limit || limit > self.rom.len() * 8 {
                return Err(Error::Mpeg(mpeg::Error::Bounds));
            }
            match self.decoder.decode(&self.rom, start, limit) {
                Ok(Some(frame)) => {
                    // The reference DSB stream is fixed at 32 kHz and indexes
                    // interleaved stereo. Do not invent conversions for other
                    // decoder formats; the SWA data audited so far is stereo.
                    if frame.channels != 2 || frame.sample_rate != SAMPLE_RATE {
                        return Err(Error::MpegFormat {
                            channels: frame.channels,
                            rate: frame.sample_rate,
                        });
                    }
                    p.bit_position = frame.next_bit as u32;
                    self.pcm = frame.samples;
                    self.cursor = 0;
                    // Consume the newly decoded frame without another decode.
                    return self.sample(p);
                }
                Ok(None) | Err(mpeg::Error::Truncated) => {
                    // Truncated tail/exhaustion follows MAME's decode-false
                    // stop/loop path. The bounded decoder commits no history.
                    if p.mode != 2 {
                        p.mode = 0;
                        return Ok([0.0; 2]);
                    }
                    let loop_bit = p.loop_start * 8;
                    let stuck = p.bit_position == loop_bit;
                    p.bit_position = loop_bit;
                    if p.loop_end != 0 {
                        p.end = p.loop_end;
                    }
                    if stuck || attempt == 1 {
                        p.mode = 0;
                        return Ok([0.0; 2]);
                    }
                }
                Err(e) => return Err(Error::Mpeg(e)),
            }
        }
        unreachable!("both loop attempts return or produce a frame")
    }

    pub fn snapshot(&self) -> State {
        State {
            decoder: self.decoder.snapshot(),
            pcm: self.pcm.clone(),
            cursor: self.cursor,
            phase: self.phase,
            rom_size: self.rom.len(),
        }
    }

    // Prepare before committing any board state; ROM ownership never changes.
    pub fn validate(&self, s: &State) -> Result<mpeg::Decoder, Error> {
        if s.rom_size != self.rom.len()
            || !(s.pcm.is_empty() || s.pcm.len() == 2304)
            || s.cursor > s.pcm.len()
            || s.cursor % 2 != 0
            || s.phase >= CLOCKS_PER_SAMPLE
        {
            return Err(Error::InvalidSnapshot);
        }
        let mut decoder = mpeg::Decoder::default();
        decoder
            .restore(&s.decoder)
            .map_err(|_| Error::InvalidSnapshot)?;
        Ok(decoder)
    }

    pub fn restore_validated(&mut self, s: &State, decoder: mpeg::Decoder) {
        self.decoder = decoder;
        self.pcm = s.pcm.clone();
        self.cursor = s.cursor;
        self.phase = s.phase;
    }
}

#[cfg(test)]
mod tests;
