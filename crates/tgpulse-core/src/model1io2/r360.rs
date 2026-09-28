// license: BSD-3-Clause
// Adapted from MAME model1.cpp r360_r/r360_w, copyright Olivier Galibert.
//! R360 cabinet feedback protocol, not mechanical motion simulation.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Cabinet {
    pub response: u8,
    pub throttle: u8,
}
impl Cabinet {
    pub fn write(&mut self, command: u8) {
        self.response = match command {
            0xbf | 0xbe | 0xba | 0xb9 => !0x40,
            0xbd => !0x44,
            0xbc => !0x45,
            0xbb => !0x46,
            0xaf => !self.throttle,
            _ => self.response,
        };
    }
}
