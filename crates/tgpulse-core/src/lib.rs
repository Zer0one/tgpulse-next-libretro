//! Sega Model 1 and Model 2 hardware emulation.
//!
//! Everything here is the machine itself -- processors, memory maps, the
//! geometry engine, the sound boards -- with no window, no GPU and no input
//! device. A front end drives it by stepping the system a frame at a time and
//! reading back the framebuffer; the `tgpulse` crate is one such front end.

#[cfg(not(any(feature = "model1", feature = "model2")))]
compile_error!("enable at least one machine feature: model1 or model2");

#[cfg(all(feature = "model1", feature = "model2"))]
pub const MACHINE_SCOPE: &str = "TGPulse compiled machines: model1+model2";
#[cfg(all(feature = "model1", not(feature = "model2")))]
pub const MACHINE_SCOPE: &str = "TGPulse compiled machines: model1";
#[cfg(all(feature = "model2", not(feature = "model1")))]
pub const MACHINE_SCOPE: &str = "TGPulse compiled machines: model2";

pub mod config;
// The existing debugger dispatches both families; keep it in full builds.
#[cfg(all(feature = "model1", feature = "model2"))]
pub mod debugger;
pub mod dsbz80;
pub mod eeprom93c46;
#[cfg(feature = "model2")]
pub mod geometry;
mod i8251;
pub mod library;
pub mod loader;
#[cfg(feature = "model2")]
pub mod memory;
#[cfg(feature = "model1")]
pub mod model1;
#[cfg(feature = "model1")]
pub mod model1_drive;
#[cfg(feature = "model1")]
pub mod model1_video;
#[cfg(feature = "model1")]
pub mod model1board;
#[cfg(feature = "model1")]
pub mod model1comm;
#[cfg(feature = "model1")]
pub mod model1io;
#[cfg(feature = "model1")]
pub mod model1io2;
#[cfg(feature = "model2")]
pub mod model2_drive;
pub mod mpeg;
#[cfg(feature = "model1")]
mod msm6253;
pub mod multipcm;
pub mod nvram;
pub mod roms_db;
#[cfg(feature = "model2")]
pub mod savestate;
#[cfg(feature = "model2")]
pub mod scsp;
#[cfg(feature = "model1")]
mod sega3155338;
pub mod sound;
#[cfg(feature = "model2")]
pub mod sound2a;
#[cfg(feature = "model2")]
pub mod system;
pub mod tilemap;
#[cfg(feature = "model1")]
mod tmpz84c015;
pub mod ym3438;
#[cfg(feature = "model1")]
mod z80ctc;
#[cfg(feature = "model1")]
mod z80pio;
#[cfg(feature = "model1")]
mod z80sio;
