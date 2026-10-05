//! Frontend-owned Model 1 ring; adapts SM2 Netpacket session/roster procedure.
//! M1COMM owns protocol and roles. No sockets, local echo or synthetic success.
use crate::ffi;
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Mutex, OnceLock};
use tgpulse_core::model1comm::{CommBoard, FRAME_SIZE};
const HEADER: usize = 14;
// The frontend can deliver many reliable frames in one poll after a stall.
// Keep a bounded transport FIFO separate from the native board's 64 RX slots.
// 1,024 native payloads occupy less than 512 KiB per local cabinet.
const LIMIT: usize = 1024;
pub const GAMES: [(&str, &std::ffi::CStr, u16); 6] = [
    ("vr", c"tgpulse_next_linked_cabinets_vr", 9),
    ("vformula", c"tgpulse_next_linked_cabinets_vformula", 9),
    ("wingwar", c"tgpulse_next_linked_cabinets_wingwar", 2),
    ("wingwaru", c"tgpulse_next_linked_cabinets_wingwaru", 2),
    ("wingwarj", c"tgpulse_next_linked_cabinets_wingwarj", 2),
    ("wingwar360", c"tgpulse_next_linked_cabinets_wingwar360", 2),
];
#[derive(Default)]
struct Session {
    reported_failure: bool,
    supported: bool,
    active: bool,
    failed: bool,
    logged: bool,
    local: u16,
    total: u16,
    hash: u32,
    send: Option<ffi::NetSend>,
    poll: Option<ffi::NetPoll>,
    clients: BTreeSet<u16>,
    peers: BTreeSet<u16>,
    roster: Vec<u16>,
    frames: VecDeque<Vec<u8>>,
}
impl Session {
    fn packet(&self, kind: u8, payload: &[u8]) -> Vec<u8> {
        let mut packet = b"TGMN".to_vec();
        packet.extend([1, kind]);
        packet.extend(self.total.to_le_bytes());
        packet.extend(self.hash.to_le_bytes());
        packet.extend((payload.len() as u16).to_le_bytes());
        packet.extend(payload);
        packet
    }
    fn receive(&mut self, bytes: &[u8], sender: u16) -> bool {
        if !self.active
            || sender == self.local
            || sender == u16::MAX
            || (self.local == 0 && !self.clients.contains(&sender))
            || bytes.len() < HEADER
            || &bytes[..4] != b"TGMN"
            || bytes[4] != 1
        {
            return false;
        }
        let size = u16::from_le_bytes(bytes[12..14].try_into().unwrap()) as usize;
        if bytes.len() != HEADER + size || !matches!((bytes[5], size), (1, 0) | (2, FRAME_SIZE)) {
            return false;
        }
        if u16::from_le_bytes(bytes[6..8].try_into().unwrap()) != self.total
            || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != self.hash
        {
            eprintln!("[TGPulse-Next Libretro] [NetBoard] Incompatible packet: participant {}, sender {}, cabinet total {} vs {}, set hash {} vs {}", self.local, sender, self.total, u16::from_le_bytes(bytes[6..8].try_into().unwrap()), self.hash, u32::from_le_bytes(bytes[8..12].try_into().unwrap()));
            self.failed = true;
            self.roster.clear();
            self.frames.clear();
            return false;
        }
        if self.failed {
            return false;
        }
        if bytes[5] == 1 {
            if self.peers.len() >= self.total.saturating_sub(1) as usize
                && !self.peers.contains(&sender)
            {
                self.failed = true;
                self.roster.clear();
                self.frames.clear();
                return false;
            }
            let inserted = self.peers.insert(sender);
            self.roster = self.peers.iter().copied().chain([self.local]).collect();
            self.roster.sort_unstable();
            if self.roster.len() != self.total as usize || self.roster.first() != Some(&0) {
                self.roster.clear();
            }
            inserted
        } else {
            if self.ready() {
                let i = self.roster.iter().position(|id| *id == self.local).unwrap();
                if sender == self.roster[(i + self.roster.len() - 1) % self.roster.len()] {
                    if self.frames.len() == LIMIT {
                        eprintln!("[TGPulse-Next Libretro] [NetBoard] Receive queue overflow: participant {}, sender {}, {} pending frames", self.local, sender, self.frames.len());
                        self.failed = true;
                        self.roster.clear();
                        self.frames.clear();
                    } else {
                        self.frames.push_back(bytes[HEADER..].to_vec());
                    }
                }
            }
            false
        }
    }
    fn ready(&self) -> bool {
        self.active && !self.failed && self.total >= 2 && self.roster.len() == self.total as usize
    }
    fn take_frames(&mut self, capacity: usize) -> Vec<Vec<u8>> {
        let count = self.frames.len().min(capacity);
        self.frames.drain(..count).collect()
    }
}
fn session() -> &'static Mutex<Session> {
    static S: OnceLock<Mutex<Session>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Session::default()))
}
pub fn configure(set: &str, total: u16) {
    let mut s = session().lock().unwrap();
    let supported = s.supported;
    *s = Session {
        supported,
        total,
        hash: set.bytes().fold(2166136261u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(16777619)
        }),
        ..Session::default()
    };
}
pub fn clear_frames() {
    session().lock().unwrap().frames.clear();
}
unsafe extern "C" fn start(local: u16, send: Option<ffi::NetSend>, poll: Option<ffi::NetPoll>) {
    let mut s = session().lock().unwrap();
    s.active = true;
    s.failed = false;
    s.reported_failure = false;
    s.logged = false;
    s.local = local;
    s.send = send;
    s.poll = poll;
    s.clients.clear();
    s.peers.clear();
    s.roster.clear();
    s.frames.clear();
    eprintln!("[TGPulse-Next Libretro] [NetBoard] Session started: participant {local}");
}
unsafe extern "C" fn receive(data: *const std::ffi::c_void, size: usize, sender: u16) {
    if data.is_null() || !(HEADER..=HEADER + FRAME_SIZE).contains(&size) {
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    let reply = session().lock().unwrap().receive(bytes, sender);
    if reply {
        send_hello();
    }
}
unsafe extern "C" fn stop() {
    let mut s = session().lock().unwrap();
    let supported = s.supported;
    let total = s.total;
    let hash = s.hash;
    *s = Session {
        supported,
        total,
        hash,
        ..Session::default()
    };
    eprintln!("[TGPulse-Next Libretro] [NetBoard] Session stopped");
}
unsafe extern "C" fn connected(id: u16) -> bool {
    let mut s = session().lock().unwrap();
    let accepted = s.active
        && s.local == 0
        && id != 0
        && id != u16::MAX
        && (s.clients.contains(&id) || s.clients.len() + 1 < (s.total as usize));
    if accepted {
        s.clients.insert(id);
    }
    accepted
}
unsafe extern "C" fn disconnected(id: u16) {
    let mut s = session().lock().unwrap();
    s.clients.remove(&id);
    s.peers.remove(&id);
    // Once a session loses a member, require reload/reset of the linked game.
    s.failed = true;
    s.roster.clear();
    s.frames.clear();
    eprintln!("[TGPulse-Next Libretro] [NetBoard] Cabinet disconnected: {id}");
}
pub fn shutdown() {
    unsafe { stop() };
}
pub fn register(env: ffi::Environment) {
    let mut callbacks = ffi::NetCallbacks {
        start,
        receive,
        stop: Some(stop),
        poll: None,
        connected: Some(connected),
        disconnected: Some(disconnected),
        protocol_version: c"TGPulse-Next Model 1 M1COMM v1".as_ptr(),
    };
    let supported = unsafe {
        env(
            ffi::SET_NETPACKET_INTERFACE,
            (&mut callbacks as *mut ffi::NetCallbacks).cast(),
        )
    };
    session().lock().unwrap().supported = supported;
}
pub fn supported() -> bool {
    session().lock().unwrap().supported
}
fn send_hello() {
    let outgoing = {
        let s = session().lock().unwrap();
        if !s.active || s.failed || s.total < 2 {
            return;
        }
        (s.send, s.packet(1, &[]))
    };
    if let (Some(send), packet) = outgoing {
        unsafe { send(5, packet.as_ptr().cast(), packet.len(), u16::MAX) };
    }
}
pub fn pump(board: &mut CommBoard) -> Result<(), &'static str> {
    let poll = {
        let s = session().lock().unwrap();
        if s.active {
            s.poll
        } else {
            None
        }
    };
    if let Some(poll) = poll {
        unsafe { poll() };
    }
    if !session().lock().unwrap().ready() {
        send_hello();
    }
    let (ready, frames, send, successor, hash, total, failed) = {
        let mut s = session().lock().unwrap();
        let ready = s.ready() && s.send.is_some();
        let next = if ready {
            let i = s.roster.iter().position(|id| *id == s.local).unwrap();
            s.roster[(i + 1) % s.roster.len()]
        } else {
            0
        };
        if ready && !s.logged {
            s.logged = true;
            eprintln!(
                "[TGPulse-Next Libretro] [NetBoard] Roster ready: participant {}, {} cabinets",
                s.local, s.total
            );
        }
        (
            ready,
            s.take_frames(board.receive_capacity()),
            s.send,
            next,
            s.hash,
            s.total,
            s.failed,
        )
    };
    board.set_connected(ready);
    for frame in frames {
        if let Err(error) = board.receive(&frame) {
            let mut s = session().lock().unwrap();
            s.failed = true;
            s.roster.clear();
            s.frames.clear();
            s.reported_failure = true;
            board.set_connected(false);
            return Err(error);
        }
    }
    while let Some(frame) = board.take_transmit() {
        if !ready {
            continue;
        }
        let state = Session {
            hash,
            total,
            ..Session::default()
        };
        let packet = state.packet(2, &frame);
        if let Some(send) = send {
            unsafe { send(5, packet.as_ptr().cast(), packet.len(), successor) };
        }
    }
    if failed && !session().lock().unwrap().reported_failure {
        session().lock().unwrap().reported_failure = true;
        Err("Linked cabinet session lost, incompatible or overflowed; reload all cabinets")
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: u16, total: u16) -> Session {
        Session {
            active: true,
            local: id,
            total,
            hash: 42,
            clients: (1..total).collect(),
            ..Session::default()
        }
    }
    #[test]
    fn roster_admission_framing_mismatch_and_disconnect_boundaries() {
        let mut a = node(0, 3);
        let mut b = node(1, 3);
        let c = node(2, 3);
        a.receive(&b.packet(1, &[]), 1);
        assert!(!a.ready());
        a.receive(&c.packet(1, &[]), 2);
        assert!(a.ready());
        let frame = [7; FRAME_SIZE];
        a.receive(&b.packet(2, &frame), 1);
        assert!(a.frames.is_empty());
        a.receive(&c.packet(2, &frame), 2);
        assert_eq!(a.frames.pop_front().unwrap(), frame);
        b.hash = 43;
        a.receive(&b.packet(1, &[]), 1);
        assert!(a.failed && !a.ready());
        let mut a = node(0, 2);
        let mut bad = node(1, 3);
        a.receive(&bad.packet(1, &[]), 1);
        assert!(a.failed);
        bad.total = 2;
        bad.hash = 42;
        a = node(0, 2);
        a.receive(&bad.packet(1, &[]), 9);
        assert!(!a.ready());
        let mut malformed = bad.packet(2, &frame);
        malformed.pop();
        a.receive(&malformed, 1);
        assert!(!a.failed);
        a.receive(&bad.packet(1, &[]), 1);
        assert!(a.ready());
        for _ in 0..=LIMIT {
            a.receive(&bad.packet(2, &frame), 1);
        }
        assert!(a.failed && a.frames.is_empty());
    }
    #[test]
    fn burst_is_retained_and_delivered_in_order_without_overfilling_native_rx() {
        let mut receiver = node(1, 2);
        let sender = node(0, 2);
        receiver.receive(&sender.packet(1, &[]), 0);
        let mut board = CommBoard::new();
        board.cn_write(1);
        board.shared_write(1, 2);
        board.set_connected(true);
        let mut frame = vec![0; FRAME_SIZE];
        frame[0] = 0xff;
        frame[1] = 1;
        board.receive(&frame).unwrap();
        frame[0] = 0xfe;
        frame[1] = 2;
        board.receive(&frame).unwrap();
        board.tick();
        while board.take_transmit().is_some() {}
        assert_eq!(board.shared_read(0), 1);
        // One delayed frontend poll receives far more than the native RX bound.
        for index in 0..300u16 {
            frame[0] = 1;
            frame[1..3].copy_from_slice(&index.to_le_bytes());
            receiver.receive(&sender.packet(2, &frame), 0);
        }
        assert!(receiver.ready());
        let mut delivered = Vec::new();
        while !receiver.frames.is_empty() {
            let batch = receiver.take_frames(board.receive_capacity());
            assert!(!batch.is_empty());
            for frame in batch { board.receive(&frame).unwrap(); }
            assert!(receiver.take_frames(board.receive_capacity()).is_empty());
            board.tick();
            while let Some(frame) = board.take_transmit() {
                if frame[0] == 1 {
                    delivered.push(u16::from_le_bytes([frame[1], frame[2]]));
                }
            }
            assert_eq!(board.shared_read(0), 1);
        }
        assert_eq!(delivered, (0..300u16).collect::<Vec<_>>());
        assert!(receiver.frames.is_empty());
    }
    #[test]
    fn real_comm_ring_uses_exact_frames_and_game_roles_including_live() {
        for roles in [vec![1, 2], vec![1, 2, 0]] {
            let total = roles.len() as u16;
            let mut nodes: Vec<_> = (0..total).map(|id| node(id, total)).collect();
            let hellos: Vec<_> = nodes.iter().map(|s| s.packet(1, &[])).collect();
            for (id, s) in nodes.iter_mut().enumerate() {
                for (peer, p) in hellos.iter().enumerate() {
                    s.receive(p, peer as u16);
                }
                assert!(s.ready(), "{id}");
            }
            let mut boards: Vec<_> = roles
                .iter()
                .map(|role| {
                    let mut b = CommBoard::new();
                    b.cn_write(1);
                    b.shared_write(1, *role);
                    b.set_connected(true);
                    b
                })
                .collect();
            for _ in 0..600 {
                for i in 0..boards.len() {
                    while let Some(frame) = nodes[i].frames.pop_front() {
                        boards[i].receive(&frame).unwrap();
                    }
                    boards[i].tick();
                    while let Some(frame) = boards[i].take_transmit() {
                        let p = nodes[i].packet(2, &frame);
                        let next = (i + 1) % boards.len();
                        nodes[next].receive(&p, i as u16);
                    }
                }
            }
            for (board, role) in boards.iter().zip(&roles) {
                assert_eq!(board.shared_read(0), 1);
                assert_eq!(board.shared_read(2), if *role == 0 { 0 } else { *role });
                assert_eq!(board.shared_read(3), 2);
            }
            boards[0].set_connected(false);
            assert_eq!(boards[0].shared_read(0), 0xff);
        }
    }
}
