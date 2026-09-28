//! Sega Model 1 and Model 2 hardware emulation.
//!
//! Everything here is the machine itself -- processors, memory maps, the
//! geometry engine, the sound boards -- with no window, no GPU and no input
//! device. A front end drives it by stepping the system a frame at a time and
//! reading back the framebuffer; the `tgpulse` crate is one such front end.

pub mod config;
pub mod debugger;
pub mod dsbz80;
pub mod eeprom93c46;
pub mod geometry;
mod i8251;
pub mod library;
pub mod loader;
pub mod memory;
pub mod model1;
pub mod model1_drive;
pub mod model1_video;
pub mod model1board;
pub mod model1comm;
pub mod model1io;
pub mod model1io2;
pub mod model2_drive;
pub mod mpeg;
mod msm6253;
pub mod multipcm;
pub mod nvram;
pub mod roms_db;
pub mod savestate;
pub mod scsp;
mod sega3155338;
pub mod sound;
pub mod sound2a;
pub mod system;
pub mod tilemap;
mod tmpz84c015;
pub mod ym3438;
mod z80ctc;
mod z80pio;
mod z80sio;
