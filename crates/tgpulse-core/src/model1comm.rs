//! Model 1 837-8842 communication board, following MAME's active M1COMM_SIMULATION.
//! Adapted from MAME m1comm.cpp, copyright Ariane Fugmann (BSD-3-Clause).
//! See LICENSES/MAME-BSD-3-Clause.txt and docs/MODEL1_NETWORK.md.
//!
//! The frontend delivers complete wire frames and drains outgoing frames. No
//! socket, wall clock, automatic loopback or frontend configuration lives here.
//! This first checkpoint implements MAME's default (frame-sync disabled) path.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const RAM_SIZE: usize = 0x1000;
pub const PAYLOAD_SIZE: usize = 0x1c4;
pub const FRAME_SIZE: usize = PAYLOAD_SIZE + 1;
const START: usize = 0x10;
const MAX_NODES: u8 = 8;
const QUEUE_LIMIT: usize = 64;

/// Hardware presence, not a claim that linked gameplay has been validated.
pub fn present_for_set(set: &str) -> bool {
    matches!(
        set,
        "vr" | "vformula" | "wingwar" | "wingwaru" | "wingwarj" | "wingwar360"
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    version: u8,
    ram: Vec<u8>,
    cn: u8,
    fg: u8,
    zfg: u8,
    alive: u8,
    id: u8,
    count: u8,
    timer: u16,
    buffer: Vec<u8>,
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}

pub struct CommBoard {
    state: State,
    /// Host resource status deliberately excluded from snapshots.
    connected: bool,
}

impl Default for CommBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl CommBoard {
    pub fn new() -> Self {
        Self {
            state: State {
                version: 1,
                ram: vec![0; RAM_SIZE],
                cn: 0,
                fg: 0,
                zfg: 0,
                alive: 0,
                id: 0,
                count: 0,
                timer: 0,
                buffer: vec![0; FRAME_SIZE],
                rx: VecDeque::new(),
                tx: VecDeque::new(),
            },
            connected: false,
        }
    }

    pub fn shared_read(&self, offset: usize) -> u8 {
        self.state.ram.get(offset).copied().unwrap_or(0xff)
    }
    pub fn shared_write(&mut self, offset: usize, value: u8) {
        if let Some(byte) = self.state.ram.get_mut(offset) {
            *byte = value;
        }
    }
    pub fn cn_read(&self) -> u8 {
        self.state.cn | 0xfe
    }
    pub fn fg_read(&self) -> u8 {
        self.state.fg | ((!self.state.zfg & 1) << 7) | 0x7e
    }
    pub fn fg_write(&mut self, value: u8) {
        if self.state.cn != 0 {
            self.state.fg = value & 1;
        }
    }
    pub fn cn_write(&mut self, value: u8) {
        let s = &mut self.state;
        s.cn = value & 1;
        // As in MAME, CN does not clear shared RAM/FG or recreate the links.
        if s.cn == 0 {
            s.zfg = 0;
        } else {
            s.id = 0;
            s.alive = 0;
            s.count = 0;
            s.timer = 0xe8;
        }
    }
    /// Both incoming and outgoing host links must be ready. No absent peer is
    /// replaced with a local echo. Reconnection after failure requires CN reset.
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
        if !connected && self.state.cn != 0 && self.state.alive == 1 {
            self.fail();
        }
    }
    pub fn receive(&mut self, frame: &[u8]) -> Result<(), &'static str> {
        if frame.len() != FRAME_SIZE {
            return Err("invalid M1COMM frame length");
        }
        if self.state.rx.len() >= QUEUE_LIMIT {
            self.fail();
            return Err("M1COMM receive queue overflow");
        }
        self.state.rx.push_back(frame.to_vec());
        Ok(())
    }
    /// Host transport may retain a burst until the next VINT consumes RX frames.
    /// Inspecting available slots does not tick the board or change its state.
    pub fn receive_capacity(&self) -> usize {
        QUEUE_LIMIT.saturating_sub(self.state.rx.len())
    }
    pub fn take_transmit(&mut self) -> Option<Vec<u8>> {
        self.state.tx.pop_front()
    }
    pub fn snapshot(&self) -> State {
        self.state.clone()
    }
    /// Restore is in-memory and atomic; reconnect host resources explicitly.
    /// A coordinated peer/transport snapshot is required for network continuation.
    pub fn restore(&mut self, state: &State) -> Result<(), &'static str> {
        if state.version != 1
            || state.ram.len() != RAM_SIZE
            || state.buffer.len() != FRAME_SIZE
            || state.cn > 1
            || state.fg > 1
            || state.zfg > 1
            || state.alive > 2
            || state.id > MAX_NODES
            || state.count > MAX_NODES
            || state.timer > 0xe8
            || (state.alive == 1 && (state.count == 0 || state.id > state.count))
            || [&state.rx, &state.tx]
                .iter()
                .any(|q| q.len() > QUEUE_LIMIT || q.iter().any(|f| f.len() != FRAME_SIZE))
        {
            return Err("invalid M1COMM snapshot");
        }
        self.state = state.clone();
        self.connected = false;
        Ok(())
    }
    fn fail(&mut self) {
        self.state.alive = 2;
        self.state.timer = 0;
        self.state.ram[0] = 0xff;
    }
    fn send(&mut self) {
        if self.state.alive == 2 {
            return;
        }
        if self.state.tx.len() == QUEUE_LIMIT {
            self.fail();
        } else {
            self.state.tx.push_back(self.state.buffer.clone());
        }
    }
    fn read(&mut self) -> bool {
        if let Some(frame) = self.state.rx.pop_front() {
            self.state.buffer = frame;
            true
        } else {
            false
        }
    }
    fn online(&mut self) {
        let s = &mut self.state;
        // Defensive host-input bound: MAME's fixed 4 KiB layout has eight slots.
        if s.count == 0 || s.count > MAX_NODES || s.id > s.count {
            self.fail();
            return;
        }
        s.alive = 1;
        s.zfg = 1;
        s.ram[0] = 1;
        s.ram[2] = s.id;
        s.ram[3] = s.count;
    }
    fn send_data(&mut self, id: u8, offset: usize, size: usize) {
        let s = &mut self.state;
        s.buffer[0] = id;
        s.buffer[1..1 + size].copy_from_slice(&s.ram[offset..offset + size]);
        // Unused control-frame tails retain the previous buffer, as in MAME.
        self.send();
    }

    /// One VINT edge, including when the V60's interrupt is masked.
    pub fn tick(&mut self) {
        if self.state.cn == 0 {
            return;
        }
        if self.state.alive == 2 {
            self.state.ram[0] = 0xff;
            return;
        }
        let master = self.state.ram[1] == 1;
        let slave = self.state.ram[1] == 2;
        let relay = self.state.ram[1] == 0;
        if self.state.alive == 0 {
            self.state.ram[0] = 5;
        }
        if !self.connected {
            return;
        }
        if self.state.alive == 0 {
            while self.state.alive == 0 && self.read() {
                match self.state.buffer[0] {
                    0xff => {
                        let count = self.state.buffer[1];
                        if count == 0 || count > MAX_NODES || (slave && count == MAX_NODES) {
                            self.fail();
                            break;
                        }
                        if master {
                            self.state.id = 1;
                            self.state.count = count;
                            self.state.timer = 0;
                        } else {
                            if slave {
                                self.state.buffer[1] += 1;
                                self.state.id = self.state.buffer[1];
                            }
                            self.send();
                        }
                    }
                    0xfe => {
                        if slave || relay {
                            self.state.count = self.state.buffer[1];
                            self.send();
                        }
                        if self.state.alive != 2 {
                            self.online();
                        }
                    }
                    _ => {}
                }
            }
            if master && self.state.alive == 0 {
                match self.state.timer {
                    1 => {
                        self.state.buffer[0] = 0xff;
                        self.state.buffer[1] = 1;
                        self.send();
                    }
                    0 => {
                        self.state.buffer[0] = 0xfe;
                        self.state.buffer[1] = self.state.count;
                        self.send();
                        if self.state.alive != 2 {
                            self.online();
                        }
                    }
                    _ => self.state.timer -= 1,
                }
            }
        }
        if self.state.alive != 1 {
            return;
        }
        while self.state.alive == 1 && self.read() {
            let id = self.state.buffer[0];
            if id > 0 && id <= self.state.count {
                if id != self.state.id {
                    let start = START + usize::from(id) * PAYLOAD_SIZE;
                    self.state.ram[start..start + PAYLOAD_SIZE]
                        .copy_from_slice(&self.state.buffer[1..]);
                    self.send();
                }
            } else if id == 0xfc {
                self.state.timer = 0;
                if !master {
                    self.send();
                }
            } else if id == 0xfd && !master {
                self.state.ram[6..16].copy_from_slice(&self.state.buffer[1..11]);
                self.send();
            }
        }
        if self.state.alive != 1 {
            return;
        }
        self.state.timer = 0; // MAME comm_framesync=0: never busy-wait the frontend.
        if self.state.id != 0 {
            if self.state.ram[4] != 0 {
                self.send_data(self.state.id, START, PAYLOAD_SIZE);
                let start = START + usize::from(self.state.id) * PAYLOAD_SIZE;
                self.state.ram[start..start + PAYLOAD_SIZE]
                    .copy_from_slice(&self.state.buffer[1..]);
            }
            if master {
                self.send_data(0xfd, 6, 10);
                self.state.buffer[0] = 0xfc;
                self.state.buffer[1] = 1;
                self.send();
            }
        }
        self.state.ram[5] = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(roles: &[u8]) -> Vec<CommBoard> {
        roles
            .iter()
            .map(|&role| {
                let mut b = CommBoard::new();
                b.shared_write(1, role);
                b.cn_write(1);
                b.set_connected(true);
                b
            })
            .collect()
    }
    fn step(boards: &mut [CommBoard]) {
        for board in boards.iter_mut() {
            board.tick();
        }
        for i in 0..boards.len() {
            while let Some(frame) = boards[i].take_transmit() {
                let next = (i + 1) % boards.len();
                boards[next].receive(&frame).unwrap();
            }
        }
    }
    #[test]
    fn registers_and_ram_survive_cn_disable() {
        let mut b = CommBoard::new();
        b.fg_write(1);
        assert_eq!(b.fg_read(), 0xfe);
        b.cn_write(3);
        b.fg_write(3);
        b.shared_write(123, 42);
        b.cn_write(0);
        assert_eq!(b.cn_read(), 0xfe);
        assert_eq!(b.fg_read(), 0xff);
        assert_eq!(b.shared_read(123), 42);
    }
    #[test]
    fn missing_peer_never_becomes_online() {
        let mut b = CommBoard::new();
        b.shared_write(1, 1);
        b.cn_write(1);
        for _ in 0..1000 {
            b.tick();
        }
        assert_eq!(b.shared_read(0), 5);
        assert_eq!(b.state.timer, 0xe8);
        assert!(b.take_transmit().is_none());
    }
    #[test]
    fn ring_handshake_payloads_and_master_status() {
        let mut b = ring(&[1, 2, 0, 2]);
        for _ in 0..250 {
            step(&mut b);
        }
        for (board, id) in b.iter().zip([1, 2, 0, 3]) {
            assert_eq!(&board.state.ram[..4], &[1, board.state.ram[1], id, 3]);
            assert_eq!(board.fg_read(), 0x7e);
        }
        for (i, board) in b.iter_mut().enumerate() {
            board.state.ram[START..START + PAYLOAD_SIZE].fill(10 + i as u8);
            board.shared_write(4, 1);
            board.shared_write(5, 42);
        }
        b[0].state.ram[6..16].fill(0x77);
        for _ in 0..10 {
            step(&mut b);
        }
        for board in &b {
            for (id, expected) in [(1, 10), (2, 11), (3, 13)] {
                let offset = START + id * PAYLOAD_SIZE;
                assert!(board.state.ram[offset..offset + PAYLOAD_SIZE]
                    .iter()
                    .all(|&v| v == expected));
            }
            assert_eq!(&board.state.ram[6..16], &[0x77; 10]);
            assert_eq!(board.shared_read(5), 0);
            assert_eq!(board.shared_read(4), 1);
            assert_eq!(board.fg_read(), 0x7e);
        }
    }
    #[test]
    fn restore_pending_handshake_and_online_traffic() {
        let mut a = ring(&[1, 2]);
        for frame in 0..260 {
            step(&mut a);
            if frame == 231 || frame == 250 {
                let mut b: Vec<_> = a
                    .iter()
                    .map(|board| {
                        let bytes = bincode::serialize(&board.snapshot()).unwrap();
                        let mut copy = CommBoard::new();
                        copy.restore(&bincode::deserialize(&bytes).unwrap())
                            .unwrap();
                        assert!(!copy.connected);
                        copy.set_connected(true);
                        copy
                    })
                    .collect();
                for _ in 0..8 {
                    step(&mut a);
                    step(&mut b);
                    for (original, restored) in a.iter().zip(&b) {
                        assert_eq!(original.snapshot(), restored.snapshot());
                    }
                }
            }
        }
    }
    #[test]
    fn disconnect_overflow_bad_frames_and_atomic_restore() {
        let mut b = ring(&[1, 2]);
        for _ in 0..240 {
            step(&mut b);
        }
        b[0].set_connected(false);
        assert_eq!(b[0].shared_read(0), 0xff);
        b[0].set_connected(true);
        b[0].tick();
        assert_eq!(b[0].shared_read(0), 0xff);
        assert!(b[0].receive(&[0; 2]).is_err());
        let before = b[0].snapshot();
        let mut bad = before.clone();
        bad.ram.clear();
        assert!(b[0].restore(&bad).is_err());
        assert_eq!(before, b[0].snapshot());
        let mut c = CommBoard::new();
        for _ in 0..QUEUE_LIMIT {
            c.receive(&[0; FRAME_SIZE]).unwrap();
        }
        assert!(c.receive(&[0; FRAME_SIZE]).is_err());
    }
    #[test]
    fn reference_handshake_frames_and_pending_transmit_restore() {
        let mut b = ring(&[1]).pop().unwrap();
        for _ in 0..231 {
            b.tick();
        }
        assert!(b.take_transmit().is_none());
        b.tick();
        let snapshot: State =
            bincode::deserialize(&bincode::serialize(&b.snapshot()).unwrap()).unwrap();
        let mut restored = CommBoard::new();
        restored.restore(&snapshot).unwrap();
        restored.set_connected(true);
        let mut expected = vec![0; FRAME_SIZE];
        expected[0] = 0xff;
        expected[1] = 1;
        assert_eq!(b.take_transmit().unwrap(), expected);
        assert_eq!(restored.take_transmit().unwrap(), expected);
        // Explicit peer response, not an implicit echo inside the device.
        expected[1] = 2;
        for board in [&mut b, &mut restored] {
            board.receive(&expected).unwrap();
            board.tick();
            let frames: Vec<_> = std::iter::from_fn(|| board.take_transmit()).collect();
            assert_eq!(frames.len(), 3);
            expected[0] = 0xfe;
            assert_eq!(frames[0], expected);
            expected[0] = 0xfd;
            expected[1] = 0;
            assert_eq!(frames[1], expected);
            expected[0] = 0xfc;
            expected[1] = 1;
            assert_eq!(frames[2], expected);
            expected[0] = 0xff;
            expected[1] = 2;
        }
        assert_eq!(b.snapshot(), restored.snapshot());
    }

    #[test]
    fn excessive_node_count_fails_without_overrunning_shared_ram() {
        let mut b = ring(&[2]).pop().unwrap();
        let mut frame = vec![0; FRAME_SIZE];
        frame[0] = 0xff;
        frame[1] = 8;
        b.receive(&frame).unwrap();
        b.tick();
        assert_eq!(b.shared_read(0), 0xff);
        assert!(b.take_transmit().is_none());
    }

    #[test]
    fn hardware_presence_is_not_all_model1_games() {
        for game in [
            "vr",
            "vformula",
            "wingwar",
            "wingwaru",
            "wingwarj",
            "wingwar360",
        ] {
            assert!(present_for_set(game));
        }
        for game in ["vf", "swa", "swaj", "netmerc"] {
            assert!(!present_for_set(game));
        }
    }
}
