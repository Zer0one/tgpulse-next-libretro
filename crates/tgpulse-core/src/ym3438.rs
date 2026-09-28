// license: BSD-3-Clause
// Copyright (c) 2021, Aaron Giles. All rights reserved.
// Rust adaptation of YMFM's OPN register/interface, timers and FM synthesis.
// Reference: MAME bd7e0b815842ec461e8ad2538d127f3332f5c96c,
// 3rdparty/ymfm/src/{ymfm_opn.cpp,ymfm_opn.h,ymfm_fm.ipp}.
// See LICENSES/YMFM-BSD-3-Clause.txt and docs/MODEL1_AUDIO.md.
//! Isolated YM3438 register/timing and native-rate FM/DAC synthesis.
//!
//! `SoundSystem` connects this chip through its clock/conversion bridge.
//! Callers advance integer input-chip clocks before reads/writes. At coincident
//! boundaries timers expire A then B, then the FM sample clock ticks, then the
//! caller's bus operation occurs. This is the explicit standalone reference
//! harness convention, not a claim of sub-cycle hardware/MAME scheduler fidelity.
//! No CPU clock conversion, host resources or persistent storage live here.

use serde::{Deserialize, Serialize};
mod synthesis;
mod tables;
use synthesis::Synthesis;

const SAMPLE_CLOCKS: u16 = 144;
const BUSY_CLOCKS: u16 = 192;

/// Mutable chip state, not a whole machine state or a stable disk format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(with = "serde_big_array::BigArray")]
    registers: [u8; 512],
    address: u16,
    dac_data: u16,
    dac_enabled: bool,
    status: u8,
    busy_remaining: u16,
    timer_remaining: [Option<u32>; 2],
    sample_remaining: u16,
    total_samples: u8,
    /// Normal key requests in logical four-operator order.
    key_on: [u8; 6],
    /// Timer-A CSM request for channel 3; consumed at the next sample clock.
    csm_pending: bool,
    synthesis: Synthesis,
}

/// Native-rate device, independent of the sound board and desktop frontend.
pub struct Ym3438 {
    state: State,
}

impl Default for Ym3438 {
    fn default() -> Self {
        Self::new()
    }
}

impl Ym3438 {
    /// Construct a powered-on, reset device. Unlike reset(), starts chip time
    /// at zero and clears the address/DAC/Busy and free-running divider state.
    pub fn new() -> Self {
        let mut chip = Self {
            state: State {
                registers: [0; 512],
                address: 0,
                dac_data: 0,
                dac_enabled: false,
                status: 0,
                busy_remaining: 0,
                timer_remaining: [None; 2],
                sample_remaining: SAMPLE_CLOCKS,
                total_samples: 0,
                key_on: [0; 6],
                csm_pending: false,
                synthesis: Synthesis::default(),
            },
        };
        chip.reset();
        chip
    }

    /// YMFM reset semantics: clear FM registers/status/key requests and cancel
    /// timers. Address, DAC, host Busy deadline and free-running clock survive.
    pub fn reset(&mut self) {
        self.state.registers.fill(0);
        for channel in [0xb4, 0xb5, 0xb6, 0x1b4, 0x1b5, 0x1b6] {
            self.state.registers[channel] = 0xc0;
        }
        self.state.status = 0;
        self.state.timer_remaining = [None; 2];
        self.state.key_on = [0; 6];
        self.state.csm_pending = false;
        self.state.synthesis.reset();
    }

    /// Four mirrored bus ports: address/data, high address/high data.
    pub fn write(&mut self, port: u8, data: u8) {
        match port & 3 {
            0 => self.state.address = u16::from(data),
            2 => self.state.address = 0x100 | u16::from(data),
            bank => {
                if (self.state.address >> 8) as u8 != bank >> 1 {
                    return; // Wrong-bank data does not start/extend Busy either.
                }
                log::trace!(target: "ym3438", "{:03x}={:02x}", self.state.address, data);
                match self.state.address {
                    0x2a => {
                        self.state.dac_data =
                            (self.state.dac_data & 1) | (u16::from(data ^ 0x80) << 1);
                    }
                    0x2b => self.state.dac_enabled = data & 0x80 != 0,
                    0x2c => {
                        self.state.dac_data =
                            (self.state.dac_data & !1) | u16::from((data >> 3) & 1);
                    }
                    _ => self.write_register(self.state.address as usize, data),
                }
                self.state.busy_remaining = BUSY_CLOCKS;
            }
        }
    }

    pub fn read(&self, port: u8) -> u8 {
        if port & 3 == 0 {
            self.state.status
                | if self.state.busy_remaining != 0 {
                    0x80
                } else {
                    0
                }
        } else {
            0
        }
    }

    /// Device IRQ level, deliberately not wired to any CPU in this checkpoint.
    pub fn irq(&self) -> bool {
        self.state.status != 0
    }

    fn write_register(&mut self, index: usize, data: u8) {
        self.state.synthesis.modified = true;
        // YMFM shares frequency high-byte latches across channels AND banks.
        // The unused low-bank B8/B9 slots hold ordinary/special frequency data.
        if index & 0xf0 == 0xa0 {
            if index & 3 == 3 {
                return;
            }
            let latch = 0xb8 | ((index >> 3) & 1);
            if index & 4 != 0 {
                self.state.registers[latch] = data & 0x3f;
            } else {
                self.state.registers[index] = data;
                self.state.registers[index | 4] = self.state.registers[latch];
            }
            return;
        }
        if index & 0xf8 == 0xb8 {
            return; // Internal latch slots, including upper-bank aliases.
        }
        self.state.registers[index] = data;
        if index == 0x28 && data & 3 != 3 {
            let channel = usize::from((data & 3) + ((data >> 2) & 1) * 3);
            self.state.key_on[channel] = data >> 4;
        }
        if index == 0x27 {
            self.state.status &= !((data >> 4) & 3);
            for timer in 0..2 {
                if data & (1 << timer) == 0 {
                    self.state.timer_remaining[timer] = None;
                } else if self.state.timer_remaining[timer].is_none() {
                    let phase = if timer == 1 {
                        u32::from(self.state.total_samples & 15) * u32::from(SAMPLE_CLOCKS)
                    } else {
                        0
                    };
                    self.state.timer_remaining[timer] = Some(self.timer_period(timer) - phase);
                }
            }
        }
    }

    fn timer_period(&self, timer: usize) -> u32 {
        let regs = &self.state.registers;
        if timer == 0 {
            (1024 - ((u32::from(regs[0x24]) << 2) | u32::from(regs[0x25] & 3))) * 144
        } else {
            (256 - u32::from(regs[0x26])) * 2304
        }
    }

    /// Advance with output discarded; oscillator/envelope/feedback still run.
    pub fn advance(&mut self, clocks: u32) {
        self.advance_with_output(clocks, |_| {});
    }

    /// Advance integer input-chip clocks and deliver each native stereo frame
    /// (input_clock / 144 Hz) to the caller. No resampling, queue, host device or
    /// board gain is implicit. Call slicing does not change the emitted stream.
    pub fn advance_with_output(&mut self, mut clocks: u32, mut emit: impl FnMut([i32; 2])) {
        while clocks != 0 {
            let mut step = clocks.min(u32::from(self.state.sample_remaining));
            for remaining in self.state.timer_remaining.into_iter().flatten() {
                step = step.min(remaining);
            }
            clocks -= step;
            self.state.sample_remaining -= step as u16;
            self.state.busy_remaining = self.state.busy_remaining.saturating_sub(step as u16);
            for timer in 0..2 {
                if let Some(remaining) = &mut self.state.timer_remaining[timer] {
                    *remaining -= step;
                    if *remaining == 0 {
                        let mode = self.state.registers[0x27];
                        if mode & (4 << timer) != 0 {
                            self.state.status |= 1 << timer;
                        }
                        if timer == 0 && mode & 0xc0 == 0x80 {
                            self.state.csm_pending = true;
                            self.state.synthesis.modified = true;
                        }
                        // Period writes affect the next reload, not this expiry.
                        self.state.timer_remaining[timer] = Some(self.timer_period(timer));
                    }
                }
            }
            if self.state.sample_remaining == 0 {
                self.state.sample_remaining = SAMPLE_CLOCKS;
                self.state.total_samples = self.state.total_samples.wrapping_add(1);
                let sample = self.state.synthesis.sample(
                    &self.state.registers,
                    &self.state.key_on,
                    self.state.csm_pending,
                    self.state.dac_data,
                    self.state.dac_enabled,
                );
                self.state.csm_pending = false;
                emit(sample);
            }
        }
    }

    pub fn snapshot(&self) -> State {
        self.state.clone()
    }

    /// For the board scheduler; does not expose a host sample rate or wall time.
    pub(crate) fn clocks_until_sample(&self) -> u16 {
        self.state.sample_remaining
    }

    /// Validate before changing the device. This typed in-memory format has no
    /// disk I/O and is separate from NVRAM; callers own encoding/versioning.
    pub fn restore(&mut self, state: &State) -> Result<(), &'static str> {
        if state.address >= 512
            || state.dac_data >= 512
            || state.status & !3 != 0
            || state.busy_remaining > BUSY_CLOCKS
            || !(1..=SAMPLE_CLOCKS).contains(&state.sample_remaining)
            || state.key_on.iter().any(|&mask| mask > 15)
            || [0xb8, 0xb9].iter().any(|&i| state.registers[i] > 63)
            || !state.synthesis.valid()
            || (state.csm_pending && !state.synthesis.modified)
        {
            return Err("invalid YM3438 register/timing state");
        }
        for timer in 0..2 {
            let loaded = state.registers[0x27] & (1 << timer) != 0;
            if loaded != state.timer_remaining[timer].is_some()
                || state.timer_remaining[timer].is_some_and(|remaining| {
                    remaining == 0 || remaining > [1024 * 144, 256 * 2304][timer]
                })
            {
                return Err("invalid YM3438 timer state");
            }
        }
        self.state = state.clone();
        Ok(())
    }
}

#[cfg(test)]
mod audio_tests;
#[cfg(test)]
mod tests;
