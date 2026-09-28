//! Desktop TCP ring for MAME's M1COMM fixed-size wire frames.
//! Host resources stay here, never in tgpulse-core or its snapshots.
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tgpulse_core::model1comm::{CommBoard, FRAME_SIZE};

const CAPACITY: usize = 64;
type Frame = [u8; FRAME_SIZE];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub address_in: String,
    pub port_in: u16,
    pub address_out: String,
    pub port_out: u16,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            address_in: "127.0.0.1".into(),
            port_in: 15112,
            address_out: "127.0.0.1".into(),
            port_out: 15113,
        }
    }
}
impl Config {
    pub fn endpoints(&self) -> Result<(SocketAddr, SocketAddr), String> {
        let local = SocketAddr::new(
            self.address_in
                .parse()
                .map_err(|_| "AddressIn: enter a numeric IPv4/IPv6 address")?,
            self.port_in,
        );
        let remote = SocketAddr::new(
            self.address_out
                .parse()
                .map_err(|_| "AddressOut: enter a numeric IPv4/IPv6 address")?,
            self.port_out,
        );
        if local.port() == 0
            || remote.port() == 0
            || remote.ip().is_unspecified()
            || remote.ip().is_multicast()
        {
            return Err("Use nonzero ports and a unicast outgoing address".into());
        }
        Ok((local, remote))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Status {
    Waiting,
    Connected,
    Failed(String),
}

pub struct Network {
    outgoing: mpsc::SyncSender<Frame>,
    incoming: mpsc::Receiver<Frame>,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<Status>>,
    worker: Option<JoinHandle<()>>,
    reported: bool,
}
impl Network {
    pub fn open(config: &Config) -> Result<Self, String> {
        let (local, remote) = config.endpoints()?;
        let listener =
            TcpListener::bind(local).map_err(|e| format!("Cannot listen on {local}: {e}"))?;
        Self::start(listener, remote)
    }
    fn start(listener: TcpListener, remote: SocketAddr) -> Result<Self, String> {
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let (outgoing, tx) = mpsc::sync_channel(CAPACITY);
        let (rx, incoming) = mpsc::sync_channel(CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(Status::Waiting));
        let worker_stop = stop.clone();
        let worker_status = status.clone();
        let worker = thread::Builder::new()
            .name("m1comm-tcp".into())
            .spawn(move || {
                if let Err(error) = run(listener, remote, tx, rx, &worker_stop, &worker_status) {
                    *worker_status.lock().unwrap() = Status::Failed(error.to_string());
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            outgoing,
            incoming,
            stop,
            status,
            worker: Some(worker),
            reported: false,
        })
    }
    pub fn description(&self) -> String {
        match &*self.status.lock().unwrap() {
            Status::Waiting => "TCP: waiting for incoming/outgoing connections".into(),
            Status::Connected => "TCP: connected (not proof of a game link)".into(),
            Status::Failed(e) => format!("TCP: {e}. Reload the game to reconnect."),
        }
    }
    /// Called before and after VINT. Never waits for the worker or a peer.
    pub fn poll(&mut self, board: &mut CommBoard) -> Result<(), String> {
        let status = self.status.lock().unwrap().clone();
        board.set_connected(status == Status::Connected);
        if let Status::Failed(error) = status {
            if !self.reported {
                self.reported = true;
                return Err(error);
            }
            return Ok(());
        }
        for _ in 0..CAPACITY {
            let Ok(frame) = self.incoming.try_recv() else {
                break;
            };
            if let Err(e) = board.receive(&frame) {
                return self.fail(board, e);
            }
        }
        while let Some(frame) = board.take_transmit() {
            let frame: Frame = frame.try_into().map_err(|_| "Invalid board frame length")?;
            if self.outgoing.try_send(frame).is_err() {
                return self.fail(board, "Outgoing queue full or disconnected");
            }
        }
        Ok(())
    }
    fn fail(&mut self, board: &mut CommBoard, error: &str) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        *self.status.lock().unwrap() = Status::Failed(error.into());
        self.reported = true;
        board.set_connected(false);
        Err(error.into())
    }
}
impl Drop for Network {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // At most the bounded connect timeout plus a short worker sleep.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Default)]
struct Framing {
    read: Vec<u8>,
    write: Option<(Frame, usize)>,
}
impl Framing {
    fn receive(&mut self, reader: &mut impl Read) -> io::Result<Option<Frame>> {
        let mut bytes = [0; FRAME_SIZE];
        match reader.read(&mut bytes[..FRAME_SIZE - self.read.len()]) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => self.read.extend_from_slice(&bytes[..n]),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(None)
            }
            Err(e) => return Err(e),
        }
        if self.read.len() == FRAME_SIZE {
            let frame = self.read.as_slice().try_into().unwrap();
            self.read.clear();
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }
    fn flush(&mut self, writer: &mut impl Write) -> io::Result<bool> {
        let Some((frame, offset)) = &mut self.write else {
            return Ok(true);
        };
        match writer.write(&frame[*offset..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => *offset += n,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(false)
            }
            Err(e) => return Err(e),
        }
        if *offset == FRAME_SIZE {
            self.write = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn prepare(stream: TcpStream) -> io::Result<TcpStream> {
    stream.set_nonblocking(true)?;
    stream.set_nodelay(true)?;
    Ok(stream)
}
fn run(
    listener: TcpListener,
    remote: SocketAddr,
    outgoing: mpsc::Receiver<Frame>,
    incoming: mpsc::SyncSender<Frame>,
    stop: &AtomicBool,
    status: &Mutex<Status>,
) -> io::Result<()> {
    let mut rx = None;
    let mut tx = None;
    let mut retry = Instant::now();
    let mut framing = Framing::default();
    while !stop.load(Ordering::Acquire) {
        if rx.is_none() {
            match listener.accept() {
                Ok((s, _)) => rx = Some(prepare(s)?),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
        }
        if tx.is_none() && Instant::now() >= retry {
            // DNS and blocking connection attempts must never run on the frame thread.
            if let Ok(s) = TcpStream::connect_timeout(&remote, Duration::from_millis(50)) {
                tx = Some(prepare(s)?);
            }
            retry = Instant::now() + Duration::from_millis(250);
        }
        if let (Some(rx), Some(tx)) = (&mut rx, &mut tx) {
            {
                let mut status = status.lock().unwrap();
                if *status == Status::Waiting {
                    *status = Status::Connected;
                }
            }
            for _ in 0..CAPACITY {
                match framing.receive(rx)? {
                    Some(frame) => incoming
                        .try_send(frame)
                        .map_err(|_| io::Error::other("Incoming queue full or closed"))?,
                    None => break,
                }
            }
            for _ in 0..CAPACITY {
                if framing.write.is_none() {
                    framing.write = outgoing.try_recv().ok().map(|frame| (frame, 0));
                    if framing.write.is_none() {
                        break;
                    }
                }
                if !framing.flush(tx)? {
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_reads_writes_and_backpressure_preserve_wire_bytes() {
        struct Chunk {
            bytes: Vec<u8>,
            blocked: bool,
        }
        impl Write for Chunk {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.blocked = !self.blocked;
                if self.blocked {
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                let n = bytes.len().min(7);
                self.bytes.extend_from_slice(&bytes[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let frame = std::array::from_fn(|i| i as u8);
        let mut codec = Framing::default();
        codec.write = Some((frame, 0));
        let mut sink = Chunk {
            bytes: vec![],
            blocked: false,
        };
        while !codec.flush(&mut sink).unwrap() {}
        assert_eq!(sink.bytes, frame);
        for (i, chunk) in sink.bytes.chunks(3).enumerate() {
            let result = codec.receive(&mut &chunk[..]).unwrap();
            assert_eq!(
                result,
                if i == FRAME_SIZE / 3 - 1 {
                    Some(frame)
                } else {
                    None
                }
            );
        }
        assert!(codec.receive(&mut &[][..]).is_err());
    }
    #[test]
    fn tcp_ring_handshake_disconnect_and_rebind() {
        let a = TcpListener::bind("127.0.0.1:0").unwrap();
        let b = TcpListener::bind("127.0.0.1:0").unwrap();
        let aa = a.local_addr().unwrap();
        let ba = b.local_addr().unwrap();
        let mut a = Network::start(a, ba).unwrap();
        let mut b = Network::start(b, aa).unwrap();
        let mut boards = [CommBoard::new(), CommBoard::new()];
        for (i, board) in boards.iter_mut().enumerate() {
            board.shared_write(1, i as u8 + 1);
            board.cn_write(1);
            board.shared_write(4, 1);
            board.shared_write(0x10, 0x40 + i as u8);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            for (net, board) in [(&mut a, &mut boards[0])] {
                net.poll(board).unwrap();
                board.tick();
                net.poll(board).unwrap();
            }
            b.poll(&mut boards[1]).unwrap();
            boards[1].tick();
            b.poll(&mut boards[1]).unwrap();
            if boards[0].shared_read(0x10 + 2 * 452) == 0x41
                && boards[1].shared_read(0x10 + 452) == 0x40
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "{} / {}",
                a.description(),
                b.description()
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(boards[0].shared_read(3), 2);
        assert_eq!(boards[1].shared_read(3), 2);
        drop(b);
        while boards[0].shared_read(0) != 0xff {
            let _ = a.poll(&mut boards[0]);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(2));
        }
        drop(a);
        // Closing/resetting a session releases listeners and joins workers.
        let _a = TcpListener::bind(aa).unwrap();
        let _b = TcpListener::bind(ba).unwrap();
    }
    #[test]
    fn absent_peer_waits_and_drop_releases_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let unused = TcpListener::bind("127.0.0.1:0").unwrap();
        let remote = unused.local_addr().unwrap();
        drop(unused);
        let mut net = Network::start(listener, remote).unwrap();
        let mut board = CommBoard::new();
        board.cn_write(1);
        board.shared_write(1, 1);
        for _ in 0..300 {
            net.poll(&mut board).unwrap();
            board.tick();
        }
        assert_eq!(board.shared_read(0), 5);
        assert_eq!(*net.status.lock().unwrap(), Status::Waiting);
        assert!(TcpListener::bind(address).is_err());
        let start = Instant::now();
        drop(net);
        assert!(start.elapsed() < Duration::from_secs(1));
        let _rebound = TcpListener::bind(address).unwrap();
    }

    #[test]
    fn invalid_endpoints_rejected() {
        for address in ["localhost", "0.0.0.0", "bad", "224.0.0.1"] {
            let c = Config {
                address_out: address.into(),
                ..Config::default()
            };
            assert!(c.endpoints().is_err());
        }
        assert!(Config {
            port_in: 0,
            ..Config::default()
        }
        .endpoints()
        .is_err());
    }

    /// Explicit real-ROM probe with operator-configured fixtures; never pokes
    /// COMM registers or writes NVRAM. Fixture preparation is a separate check.
    #[test]
    #[ignore = "requires Model 1 ROM plus MASTER/SLAVE NVRAM fixtures; see MODEL1_NETWORK.md"]
    fn model1_preconfigured_cabinets_tcp_link() {
        use tgpulse_core::{loader, model1::Model1System, nvram};
        let rom = std::env::var("TGPULSE_MODEL1_ROM").expect("TGPULSE_MODEL1_ROM");
        eprintln!("Model 1 link probe ROM={rom}");
        let roms = loader::load_model1_zip(&rom).unwrap();
        let mut fixtures = vec![
            ("TGPULSE_MODEL1_MASTER_NVRAM", 1),
            ("TGPULSE_MODEL1_SLAVE_NVRAM", 2),
        ];
        if std::env::var_os("TGPULSE_MODEL1_LIVE_NVRAM").is_some() {
            // MAME's relay participates in the ring but takes no player ID
            // and does not increment the participant count.
            fixtures.push(("TGPULSE_MODEL1_LIVE_NVRAM", 0));
        }
        let mut systems: Vec<_> = fixtures
            .iter()
            .map(|(variable, _)| {
                let nv = std::fs::read(std::env::var(variable).expect(variable)).unwrap();
                let mut s = Model1System::with_config(
                    &roms,
                    tgpulse_core::config::Config {
                        cabinet: tgpulse_core::config::Cabinet::Twin,
                        ..Default::default()
                    },
                )
                .unwrap();
                let (a, b) = s.nvram_sizes();
                let (a, b) = nvram::decode(&nv, a, b).unwrap();
                s.set_nvram_blocks(&a, &b);
                s
            })
            .collect();
        let listeners: Vec<_> = (0..systems.len())
            .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
            .collect();
        let addresses: Vec<_> = listeners.iter().map(|s| s.local_addr().unwrap()).collect();
        let mut nets: Vec<_> = listeners
            .into_iter()
            .enumerate()
            .map(|(i, listener)| {
                Network::start(listener, addresses[(i + 1) % addresses.len()]).unwrap()
            })
            .collect();
        let mut linked_frames = 0;
        for frame in 0..2400 {
            for (net, sys) in nets.iter_mut().zip(&mut systems) {
                net.poll(sys.comm.as_mut().unwrap()).unwrap();
                sys.run_frame().unwrap();
                sys.sound.samples.clear();
                net.poll(sys.comm.as_mut().unwrap()).unwrap();
            }
            if frame % 300 == 0 {
                eprintln!(
                    "V60 PCs={:x?}",
                    systems.iter().map(|s| s.main_cpu.ppc).collect::<Vec<_>>()
                );
                eprintln!(
                    "frame={frame} COMM={:?}",
                    systems
                        .iter()
                        .map(|s| (0..6)
                            .map(|i| s.comm.as_ref().unwrap().shared_read(i))
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                );
            }
            if systems
                .iter()
                .all(|s| s.comm.as_ref().unwrap().shared_read(0) == 1)
            {
                linked_frames += 1;
            } else {
                linked_frames = 0;
            }
            thread::sleep(Duration::from_millis(1));
        }
        if linked_frames < 600 {
            for ((variable, _), sys) in fixtures.iter().zip(&systems) {
                let board = sys.comm.as_ref().unwrap();
                eprintln!(
                    "{variable}: CN={:02x} FG={:02x} EEPROM={:02x?}",
                    board.cn_read(),
                    board.fg_read(),
                    sys.nvram_blocks().1
                );
            }
        }
        assert!(
            linked_frames >= 600,
            "game-created link must remain online for at least 600 frames"
        );
        for ((variable, role), s) in fixtures.iter().zip(&systems) {
            let board = s.comm.as_ref().unwrap();
            assert_eq!(board.cn_read() & 1, 1);
            assert_eq!(board.shared_read(0), 1);
            assert_eq!(board.shared_read(1), *role, "role from {variable}");
            assert_eq!(board.shared_read(2), *role, "ID from {variable}");
            assert_eq!(board.shared_read(3), 2);
        }
    }
}
