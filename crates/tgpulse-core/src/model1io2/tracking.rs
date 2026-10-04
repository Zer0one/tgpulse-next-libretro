//! NetMerc's Polhemus-compatible serial measurement endpoint. This is a
//! protocol peer driven by frontend-provided raw poses, not an emulation of
//! the i386SX firmware or magnetic sensor physics. See MODEL1_NETMERC.md.
use crate::i8251::Uart;

/// Signed raw words in the order consumed by NetMerc: XYZ, then three angles.
/// These are game/controller units, not SDL sensor readings or physical units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HmdPose {
    pub position: [i16; 3],
    pub orientation: [i16; 3],
}

impl Default for HmdPose {
    fn default() -> Self {
        Self {
            position: [0; 3],
            orientation: [12868, 25736, 12868],
        }
    }
}

impl HmdPose {
    pub fn words(self) -> [i16; 6] {
        [
            self.position[0],
            self.position[1],
            self.position[2],
            self.orientation[0],
            self.orientation[1],
            self.orientation[2],
        ]
    }
}

/// Public diagnostics, independent of any desktop UI or host sensor backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackingStatus {
    pub pose: HmdPose,
    pub streaming: bool,
    pub records: u64,
    pub last_command: u8,
    pub unsupported_commands: u64,
    pub uart_errors: u8,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Tracker {
    uart: Uart,
    pose: HmdPose,
    phase: u32,
    divider: u32,
    sample_remaining: u32,
    continuous: bool,
    binary: bool,
    metric: bool,
    packet: [u8; 20],
    cursor: usize,
    records: u64,
    last_command: u8,
    unsupported_commands: u64,
    measurement_ready: bool,
    complete_packet: bool,
}

const SAMPLE_CLOCKS: u32 = super::CPU_HZ / 60;

impl Default for Tracker {
    fn default() -> Self {
        let mut uart = Uart::default();
        uart.control(0x4e).unwrap(); // async 8N1 x16
        uart.control(0x37).unwrap(); // enable TX/RX, clear errors, assert modem pins
        Self {
            uart,
            pose: HmdPose::default(),
            phase: 0,
            divider: 16,
            sample_remaining: SAMPLE_CLOCKS,
            continuous: false,
            binary: false,
            metric: false,
            packet: [0; 20],
            cursor: 20,
            records: 0,
            last_command: 0,
            unsupported_commands: 0,
            measurement_ready: false,
            complete_packet: false,
        }
    }
}

impl Tracker {
    pub(super) fn reset_measurement(&mut self) {
        self.measurement_ready = false;
        self.complete_packet = false;
    }
    pub(super) fn receive_boundary(&mut self, length: u8) {
        self.complete_packet = length == 20;
    }
    /// EPR-18021 increments its decoded-record counter after rebuilding all
    /// six words. Check the complete station-1 record as well, since the first
    /// sync byte can make the firmware decode its still-empty receive buffer.
    pub(super) fn decoded_record(&mut self, packet: &[u8], words: &[u8]) {
        let decoded: [i16; 6] =
            std::array::from_fn(|i| i16::from_le_bytes([words[i * 2], words[i * 2 + 1]]));
        let pose = HmdPose {
            position: decoded[..3].try_into().unwrap(),
            orientation: decoded[3..].try_into().unwrap(),
        };
        self.measurement_ready |= self.complete_packet && packet == encode(pose);
        self.complete_packet = false;
    }
    pub(super) fn publication_byte(&self, offset: usize, data: u8) -> u8 {
        if self.measurement_ready {
            data
        } else {
            HmdPose::default().words()[offset / 2].to_le_bytes()[offset % 2]
        }
    }
    pub(super) fn status(&self) -> TrackingStatus {
        TrackingStatus {
            pose: self.pose,
            streaming: self.continuous,
            records: self.records,
            last_command: self.last_command,
            unsupported_commands: self.unsupported_commands,
            uart_errors: self.uart.status() & 0x38,
        }
    }
    pub(super) fn set_pose(&mut self, pose: HmdPose) {
        self.pose = pose;
    }
    pub(super) fn tx(&self) -> bool {
        self.uart.tx
    }
    pub(super) fn next_tick(&self) -> u32 {
        self.divider - self.phase
    }
    pub(super) fn set_baud(&mut self, dips: u8) {
        // Cabinet baud selection: 38400, 19200, 9600 or 4800 (DSW1 low bits).
        self.divider = 16 << ((!dips) & 3);
        self.phase %= self.divider;
    }
    pub(super) fn valid(&self) -> bool {
        self.uart.valid()
            && self.uart.mode == 0x4e
            && self.uart.command == 0x37
            && [16, 32, 64, 128].contains(&self.divider)
            && self.phase < self.divider
            && self.cursor <= 20
            // Zero may await the next UART edge after a baud change.
            && self.sample_remaining <= SAMPLE_CLOCKS
    }
    /// The bus calls this at or before the next external UART clock boundary.
    pub(super) fn advance(&mut self, clocks: u32, board_tx: bool) {
        self.phase += clocks;
        self.sample_remaining = self.sample_remaining.saturating_sub(clocks);
        if self.phase == self.divider {
            self.phase = 0;
            self.uart.rx = board_tx;
            self.uart.tick();
            if self.uart.status() & 2 != 0 {
                let command = self.uart.read();
                self.command(command);
            }
            // Latch a whole record, never change it halfway through transmission.
            // A slower UART discards intervening samples instead of queueing them.
            if self.sample_remaining == 0 {
                self.sample_remaining = SAMPLE_CLOCKS;
                if self.continuous
                    && self.binary
                    && self.metric
                    && self.cursor == 20
                    && self.uart.status() & 4 != 0
                {
                    self.packet = encode(self.pose);
                    self.cursor = 0;
                    self.records += 1;
                }
            }
            if self.cursor < 20 && self.uart.status() & 1 != 0 {
                self.uart.write(self.packet[self.cursor]).unwrap();
                self.cursor += 1;
            }
        }
    }
    fn command(&mut self, command: u8) {
        self.last_command = command;
        match command {
            b'c' => {
                self.continuous = false;
                self.cursor = 20;
            }
            b'C' => {
                self.continuous = true;
                self.sample_remaining = SAMPLE_CLOCKS;
            }
            b'f' => self.binary = true,
            b'u' => self.metric = true,
            // Quiet mode filters physical measurements; supplied poses are already
            // measurements. S asks for firmware/BIT identity, which is unknown:
            // record the request, but do not invent an identity or a healthy BIT.
            b'm' | b'K' => {}
            b'S' => self.unsupported_commands += 1,
            _ => self.unsupported_commands += 1,
        }
    }
}

/// Default station-1 binary record: ASCII header, six LE words, CR/LF.
/// Each group of seven bytes is followed by a bitmap of the original MSBs;
/// bit 7 of the first encoded byte marks the start of the record.
pub(super) fn encode(pose: HmdPose) -> [u8; 20] {
    let mut raw = [0u8; 17];
    raw[..3].copy_from_slice(b"01 ");
    for (i, word) in pose.words().iter().enumerate() {
        raw[3 + 2 * i..5 + 2 * i].copy_from_slice(&word.to_le_bytes());
    }
    raw[15..].copy_from_slice(b"\r\n");
    let mut packet = [0u8; 20];
    let mut out = 0;
    for group in raw.chunks(7) {
        let mut high = 0;
        for (i, byte) in group.iter().enumerate() {
            packet[out] = byte & 0x7f;
            high |= (byte >> 7) << i;
            out += 1;
        }
        packet[out] = high;
        out += 1;
    }
    packet[0] |= 0x80;
    packet
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_record_matches_firmware_byte_positions_and_high_bit_maps() {
        let packet = encode(HmdPose {
            position: [4, -8, 120],
            orientation: [12868, 25736, -12868],
        });
        assert_eq!(
            packet,
            [
                0xb0, 0x31, 0x20, 4, 0, 0x78, 0x7f, 0x60, 0x78, 0, 0x44, 0x32, 8, 0x64, 0x3c, 0x50,
                0x4d, 13, 10, 1
            ]
        );
    }
    #[test]
    fn clocked_commands_and_mid_record_restore_continue_identically() {
        let mut tracker = Tracker::default();
        let mut host = Uart::default();
        host.control(0x4e).unwrap();
        host.control(0x37).unwrap();
        let tick = |tracker: &mut Tracker, host: &mut Uart| {
            let (a, b) = (tracker.tx(), host.tx);
            tracker.advance(16, b);
            host.rx = a;
            host.tick();
        };
        for command in b"cSufm" {
            host.write(*command).unwrap();
            for _ in 0..180 {
                tick(&mut tracker, &mut host);
            }
        }
        assert_eq!(tracker.records, 0);
        host.write(b'C').unwrap();
        for _ in 0..10500 {
            tick(&mut tracker, &mut host);
        }
        assert!(tracker.continuous && tracker.cursor > 0 && tracker.cursor < 20);
        let encoded = bincode::serialize(&tracker).unwrap();
        let mut restored: Tracker = bincode::deserialize(&encoded).unwrap();
        assert!(restored.valid());
        let mut other_host = host.clone();
        for _ in 0..20000 {
            tick(&mut tracker, &mut host);
            tick(&mut restored, &mut other_host);
            assert_eq!(tracker.tx(), restored.tx());
        }
        assert_eq!(
            bincode::serialize(&tracker).unwrap(),
            bincode::serialize(&restored).unwrap()
        );
        assert_eq!(host, other_host);
        host.write(b'c').unwrap();
        for _ in 0..180 {
            tick(&mut tracker, &mut host);
        }
        assert!(!tracker.continuous);
        let records = tracker.records;
        for _ in 0..20000 {
            tick(&mut tracker, &mut host);
        }
        assert_eq!(tracker.records, records);
    }
}
