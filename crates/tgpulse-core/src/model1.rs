//! Sega Model 1 motherboard: NEC V60 memory map and board-level devices.
//!
//! This first milestone models the complete host-visible map, GLUE interrupts,
//! timers, dual-port I/O RAM, sound UART and V60/TGP host communication.

use std::collections::VecDeque;

use mb86233::{Mb86233, Mb86233Bus};
use v60::{Bus, V60};

use crate::config::{Config, Inputs};
use crate::loader::Model1Roms;
use crate::sound::SoundSystem;

mod state;
pub use state::MotherboardState;
mod save;
pub use save::MAX_STATE_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    IoBoard(crate::model1io2::BusError),
    Dsb(crate::dsbz80::Error),
    Serial(crate::sound::SerialError),
}
impl From<crate::model1io2::BusError> for Error {
    fn from(e: crate::model1io2::BusError) -> Self {
        Self::IoBoard(e)
    }
}
impl From<crate::dsbz80::Error> for Error {
    fn from(e: crate::dsbz80::Error) -> Self {
        Self::Dsb(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoBoard(e) => e.fmt(f),
            Self::Dsb(e) => e.fmt(f),
            Self::Serial(e) => write!(f, "Model 1 serial: {e:?}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<crate::sound::SerialError> for Error {
    fn from(e: crate::sound::SerialError) -> Self {
        Self::Serial(e)
    }
}

pub const CPU_HZ: u32 = 16_000_000;
pub const CYCLES_PER_FRAME: i32 = 656 * 424;

/// V60 clock, which the other boards' rates are expressed against.
pub const V60_HZ: u32 = 16_000_000;

/// Depth of the V60<->TGP hand-off FIFOs (`copro_fifo_in`/`_out`), which the reference
/// configures at 16 words each. The producer is paused once its FIFO is full so
/// the queues stay bounded instead of the V60 flooding the TGP.
pub const COPRO_FIFO_DEPTH: usize = 16;

pub struct Model1System {
    /// Identity of immutable resources supplied at construction, not NVRAM.
    /// Replacing ROMs/devices requires constructing a new machine.
    resource_identity: [u8; 20],
    pub main_cpu: V60,
    pub tgp_cpu: Mb86233,
    /// Fractional 40/16 MHz clock phase (numerator over 2), including HALT time.
    /// Captured with tgp_cpu.icount debt by the motherboard snapshot.
    tgp_clock_remainder: u8,

    pub maincpu_rom: Vec<u8>,
    pub nvram: Vec<u8>,
    pub work_ram: Vec<u8>,
    pub display_list: [Vec<u8>; 2],
    pub(crate) video: crate::model1_video::Model1VideoState,
    pub tile_ram: Vec<u8>,
    pub char_ram: Vec<u8>,
    pub palette_ram: Vec<u8>,
    pub colorxlat_ram: Vec<u8>,
    /// The I/O board: a Z80 with its own firmware, which owns the dual-port
    /// RAM it shares with the V60 and the 93C45 the operator settings live in.
    pub ioboard: crate::model1board::IoBoard,
    pub comm: Option<crate::model1comm::CommBoard>,
    /// Transient CPU-bus context, never charged by debugger/renderer reads.
    v60_access_active: bool,
    v60_wait_cycles: u32,
    /// Empty result FIFO stalls an IN instruction until the TGP supplies data.
    v60_fifo_waiting: bool,
    v60_io_stall: bool,

    pub sound: SoundSystem,
    pub inputs: Inputs,
    pub drive_cmd: u8,
    pub config: Config,

    pub tgp_program: Vec<u32>,
    pub tgp_data: Vec<u32>,
    pub copro_ram: Vec<u32>,
    pub copro_fifo_in: VecDeque<u32>,
    pub copro_fifo_out: VecDeque<u32>,

    /// CPU-board math ROM the TGP's geometry accelerators index into.
    pub copro_tables: Vec<u32>,
    /// Geometry-board polygon/model ROM used by the Model 1 renderer.
    pub polygons: Vec<u32>,
    /// TGP external data ROM selected by `copro_data_base`.
    pub copro_data: Vec<u32>,
    /// The four TGP-side coprocessor-RAM address registers (I/O 0x00/08/10/18),
    /// each auto-incrementing on data access. Distinct from the V60-side
    /// `copro_ram_addr`, but they walk the same `copro_ram`.
    copro_io_ram_adr: [u32; 4],
    /// Latched arguments for the I/O-mapped function units.
    copro_sincos_base: u32,
    copro_inv_base: u32,
    copro_isqrt_base: u32,
    copro_atan_base: [u32; 4],
    copro_data_base: u32,

    pub listctl: [u16; 2],
    pub bank_reg: u16,
    pub bank_base: u32,

    pub irq_status: u8,
    pub irq_mask: u8,
    pub last_irq: u8,

    pub timer_mode: u16,
    pub timer_period: [u16; 2],
    pub timer_remaining: [u32; 2],
    /// Last observed count, retained when software stops a timer (MAME #15715).
    timer_latched: [u16; 2],

    pub frame_num: u64,

    pub copro_ram_addr: u16,
    copro_ram_latch: [u16; 2],
    copro_fifo_read_latch: u32,
    copro_fifo_write_latch: u32,
    copro_stall: bool,
    /// Ring of the last copro FIFO transfers (direction tag, value, in/out
    /// depths after the transfer). Always recording (it is only 128 slots) so
    /// a protocol stall can be autopsied after the fact.
    pub fifo_events: VecDeque<(char, u32, usize, usize)>,
}

/// Read-only NetMerc program state, derived from existing SRAM/work RAM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MvdHolderContext {
    pub latched: bool,
    pub credits: u16,
    pub in_game: bool,
}

impl Model1System {
    /// Frontend-owned preference; changes the next matching TGP conversion,
    /// without rewriting ROM, RAM, ALU pipeline results or persistent data.
    pub fn set_netmerc_city_workaround(&mut self, enabled: bool) {
        self.config.netmerc_city_workaround = enabled;
        self.tgp_cpu.netmerc_city_conversion =
            enabled && self.ioboard.kind() == crate::model1board::Kind::NetMerc;
    }
    /// NetMerc's game-acknowledged MVD Holder latch, not the live switch level.
    /// The supported program stores this word at V60 address 0x52766a and
    /// clears it when starting a game. Work RAM already belongs to save states.
    pub fn mvd_holder_latched(&self) -> Option<bool> {
        self.mvd_holder_context().map(|state| state.latched)
    }

    /// Supported NetMerc program: Holder 0x52766a, session 0x527668,
    /// credits 0x400018. This query does not change input or game memory.
    pub fn mvd_holder_context(&self) -> Option<MvdHolderContext> {
        (self.ioboard.kind() == crate::model1board::Kind::NetMerc).then(|| MvdHolderContext {
            latched: u16::from_le_bytes([self.work_ram[0x2766a], self.work_ram[0x2766b]]) != 0,
            credits: u16::from_le_bytes([self.nvram[0x18], self.nvram[0x19]]),
            in_game: u16::from_le_bytes([self.work_ram[0x27668], self.work_ram[0x27669]]) == 1,
        })
    }

    /// Battery-backed SRAM at 0x400000, plus the I/O board's 93C45 image as
    /// the second block. The Model 1 boards have no EEPROM of their own, but
    /// the I/O board does, and the game keeps its operator settings there.
    pub fn nvram_blocks(&self) -> (Vec<u8>, Vec<u8>) {
        let eeprom = self
            .ioboard
            .eeprom()
            .data
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        (self.nvram.clone(), eeprom)
    }

    pub fn set_nvram_blocks(&mut self, backup: &[u8], eeprom: &[u8]) {
        let n = self.nvram.len().min(backup.len());
        self.nvram[..n].copy_from_slice(&backup[..n]);
        let chip = self.ioboard.eeprom_mut();
        for (word, chunk) in eeprom.chunks_exact(2).enumerate() {
            if word < chip.data.len() {
                chip.data[word] = u16::from_le_bytes([chunk[0], chunk[1]]);
            }
        }
    }

    pub fn nvram_sizes(&self) -> (usize, usize) {
        // Persist the complete 16-bit EEPROM words, not its byte-wide DPRAM mirror.
        (self.nvram.len(), self.ioboard.eeprom().data.len() * 2)
    }

    pub fn new(roms: &Model1Roms) -> Result<Self, Error> {
        Self::with_config(roms, Config::default())
    }

    pub fn with_config(roms: &Model1Roms, config: Config) -> Result<Self, Error> {
        let mut video = crate::model1_video::Model1VideoState::new();
        video.smooth_shadows = config.smooth_shadows;
        // The 93C45 on the I/O board holds the operator settings. The romset
        // carries a dump of a configured one; a blank chip would send the game
        // into its setup menu on a first boot. The dump stores each 16-bit
        // word little-endian, which is also how the saved image keeps it.
        let mut eeprom = crate::eeprom93c46::Eeprom93c46::new();
        for (word, chunk) in roms.ioboard_config.chunks_exact(2).enumerate() {
            if word < eeprom.data.len() {
                eeprom.data[word] = u16::from_le_bytes([chunk[0], chunk[1]]);
            }
        }
        let mut ioboard = crate::model1board::IoBoard::new(roms.ioboard_kind, &roms.iocpu, eeprom)?;
        if roms.ioboard_kind == crate::model1board::Kind::NetMerc {
            // MAME netmerc_state::machine_reset: stationary HMD pose facing
            // forward (12868 ~= 90 degrees, 25736 ~= 180 degrees). This is
            // power-on data, not an i386SX tracking or a ready handshake. The
            // serial peer publishes the same initial pose after firmware setup.
            // Snapshot restore retains the saved DPRAM without reseeding it.
            for (i, value) in crate::model1io2::HmdPose::default()
                .words()
                .into_iter()
                .enumerate()
            {
                ioboard.dpram_mut()[0x80 + i * 2..0x82 + i * 2]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        let mut sound = if let Some(dsb) = &roms.dsb {
            SoundSystem::with_dsb_audio(
                roms.sndcpu.clone(),
                roms.mpcm1.clone(),
                roms.mpcm2.clone(),
                &dsb.firmware,
                dsb.mpeg.clone(),
            )?
        } else {
            SoundSystem::new(roms.sndcpu.clone(), roms.mpcm1.clone(), roms.mpcm2.clone())
        };
        sound.set_mutes(config.audio_mutes);
        sound.enable_model1_serial();
        if roms.ioboard_kind == crate::model1board::Kind::NetMerc && roms.netmerc_procedural_audio {
            sound.enable_netmerc_recovery();
        }

        // Battery-backed work RAM (RAMA). Use a shipped factory image when present
        // (NetMerc); otherwise retain our zero-filled boot state. Games initialise bytes
        // they rely on during boot. We must match that power-on state: a stray
        // 0xff fill leaves flags like vf's 0x40bf00 (a "skip the vblank game
        // logic" gate, read at FE3F21) set, so vf's ISR never builds a display
        // list and the screen stays blank. The couple of bytes we used to seed
        // for vr are unnecessary once the region starts at zero.
        let nvram = if roms.nvram_default.len() == 0x10000 {
            roms.nvram_default.clone()
        } else {
            vec![0x00; 0x10000]
        };

        Ok(Self {
            resource_identity: save::resource_identity(roms),
            main_cpu: V60::new(),
            tgp_cpu: {
                let mut cpu = Mb86233::new();
                if roms.ioboard_kind == crate::model1board::Kind::NetMerc {
                    cpu.float_mode = mb86233::FloatMode::Finite;
                    cpu.netmerc_city_conversion = config.netmerc_city_workaround;
                }
                cpu
            },
            tgp_clock_remainder: 0,

            maincpu_rom: roms.maincpu.clone(),
            nvram,
            work_ram: vec![0; 0x40000],
            display_list: [vec![0; 0x10000], vec![0; 0x10000]],
            video,
            tile_ram: vec![0; 0x10000],
            char_ram: vec![0; 0x80000],
            palette_ram: vec![0; 0x4000],
            colorxlat_ram: vec![0; 0xc000],
            ioboard,
            comm: (roms.comm_board && config.cabinet == crate::config::Cabinet::Twin)
                .then(crate::model1comm::CommBoard::new),
            v60_access_active: false,
            v60_wait_cycles: 0,
            v60_fifo_waiting: false,
            v60_io_stall: false,

            sound,
            inputs: Inputs::default(),
            drive_cmd: 0,
            config,

            tgp_program: roms
                .tgp
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
            tgp_data: vec![0; 0x400],
            copro_ram: vec![0; 0x2000],
            copro_fifo_in: VecDeque::new(),
            copro_fifo_out: VecDeque::new(),

            copro_tables: roms.copro_tables.clone(),
            polygons: roms.polygons.clone(),
            copro_data: roms.copro_data.clone(),
            copro_io_ram_adr: [0; 4],
            copro_sincos_base: 0,
            copro_inv_base: 0,
            copro_isqrt_base: 0,
            copro_atan_base: [0; 4],
            copro_data_base: 0,

            listctl: [0; 2],
            bank_reg: 0x0001,
            bank_base: 0x0100_0000,

            irq_status: 0,
            irq_mask: 0xff,
            last_irq: 0,

            timer_mode: 0,
            timer_period: [0; 2],
            timer_remaining: [0; 2],
            timer_latched: [0; 2],

            frame_num: 0,

            copro_ram_addr: 0,
            copro_ram_latch: [0; 2],
            copro_fifo_read_latch: 0,
            copro_fifo_write_latch: 0,
            copro_stall: false,
            fifo_events: VecDeque::new(),
        })
    }

    pub fn run_slice(&mut self, cycles: i32) -> Result<(), Error> {
        if let Some(error) = self.ioboard.fault() {
            return Err(error.into());
        }
        if let Some(error) = self.sound.serial_fault() {
            return Err(error.into());
        }
        if let Some(error) = self.sound.dsb_fault() {
            return Err(error.into());
        }
        if cycles <= 0 {
            return Ok(());
        }
        // The CPU bus borrows the whole machine. Move the executing CPUs out
        // once per call instead of copying their diagnostic tables and making
        // reset placeholders in every 64-clock quantum. Bus callbacks only
        // inspect the placeholder CPUs for trace messages.
        let mut main_cpu = std::mem::replace(&mut self.main_cpu, V60::new());
        let mut tgp_cpu = std::mem::replace(&mut self.tgp_cpu, Mb86233::new());
        let result = self.run_cpu_slices(&mut main_cpu, &mut tgp_cpu, cycles);
        // Restore the real CPUs even when a board reports a sticky fault.
        self.main_cpu = main_cpu;
        self.tgp_cpu = tgp_cpu;
        result
    }

    fn run_cpu_slices(
        &mut self,
        main_cpu: &mut V60,
        tgp_cpu: &mut Mb86233,
        cycles: i32,
    ) -> Result<(), Error> {
        const QUANTUM: i32 = 64;

        let mut remaining = cycles;
        while remaining > 0 {
            let step = remaining.min(QUANTUM);

            // Pause the V60 while its outbound FIFO is full, so it cannot outrun
            // the TGP and the queue stays near the hardware's 16-word depth.
            if !self.copro_fifo_out.is_empty() {
                self.v60_fifo_waiting = false;
            }
            if self.copro_fifo_in.len() <= COPRO_FIFO_DEPTH && !self.v60_fifo_waiting {
                self.sync_irq(main_cpu);
                self.v60_access_active = true;
                main_cpu.run(self, step);
                self.v60_access_active = false;
                self.v60_wait_cycles = 0;
                self.sync_irq(main_cpu);
            }

            // Each revision retains the fractional ratio of its own board
            // clock to the V60, independently of CPU instruction overshoot.
            self.ioboard.run_main_cycles(step as u32, self.inputs)?;
            self.drive_cmd = self.ioboard.drive_cmd();

            // Step the TGP in the same fine lockstep so the FIFO handshakes
            // resolve instead of deadlocking. The MB86233 runs at 40 MHz against
            // the V60's 16 MHz, so it receives exactly 5/2 clocks per slice; a FIFO
            // read with no data rewinds and retries (take_stall), which is how
            // it waits for the V60 without an event scheduler. It halts itself
            // (halt_requested) once its own output FIFO backs up.
            let tgp_clocks = step * 5 + i32::from(self.tgp_clock_remainder);
            self.tgp_clock_remainder = (tgp_clocks % 2) as u8;
            // Like MAME's scheduler/local CPU time, retain instruction overshoot,
            // not unused time from an empty FIFO or HALT. Clock phase and debt
            // advance even while output overflow prevents CPU execution.
            let tgp_step = tgp_clocks / 2 + tgp_cpu.icount.min(0);
            tgp_cpu.icount = tgp_step.min(0);
            if tgp_step > 0 && self.copro_fifo_out.len() <= COPRO_FIFO_DEPTH {
                tgp_cpu.execute(self, tgp_step);
            }

            self.advance_timers(step as u32);
            self.sound.run(step, CPU_HZ);
            if let Some(error) = self.sound.serial_fault() {
                return Err(error.into());
            }
            if let Some(error) = self.sound.dsb_fault() {
                return Err(error.into());
            }
            // sound_ready_w: the M1 audio UART's ready lines
            // raise IRQ level 3 while unmasked -- vf's sound-queue pump lives
            // in that handler and the game hangs in its boot without it.
            self.sound_ready_irq();

            remaining -= step;
        }
        Ok(())
    }

    pub fn run_frame(&mut self) -> Result<(), Error> {
        self.run_slice(CYCLES_PER_FRAME)?;
        self.trigger_vblank();
        Ok(())
    }

    pub fn trigger_vblank(&mut self) {
        // set_current_render_list: when bit 2 of listctl[0]
        // is clear, the display-buffer select (bit 6) is latched from bit 3 at
        // render time. The V60 reads bit 6 back to learn which buffer to write
        // next (get_list_number), so the latch must be visible to the CPU, not
        // just to the renderer. end_frame: in bit-2 mode the hardware toggles
        // the select itself every other frame.
        if self.listctl[0] & 4 == 0 {
            self.listctl[0] =
                (self.listctl[0] & !0x40) | if self.listctl[0] & 8 != 0 { 0x40 } else { 0 };
        }
        // The reference scans renderer uploads on the rising vblank edge even when the
        // display list was not rasterized during that frame.
        crate::model1_video::scan_uploads(self);
        // Consume the completed list before selecting the following frame's
        // buffer. Switching first can upload data from an unfinished list.
        if self.listctl[0] & 4 != 0 && self.frame_num & 1 != 0 {
            self.listctl[0] ^= 0x40;
        }
        log::trace!(target: "model1_drive", "frame={} sampled={:02X}", self.frame_num, self.drive_cmd);
        self.frame_num = self.frame_num.wrapping_add(1);
        if let Some(comm) = &mut self.comm {
            comm.tick();
        }
        if self.irq_mask & (1 << 1) == 0 {
            self.raise_irq(1);
        }
    }

    pub fn raise_irq(&mut self, level: u8) {
        self.irq_status |= 1 << level;
    }

    fn sync_irq(&self, cpu: &mut V60) {
        if let Some(level) = (0..8).find(|level| self.irq_status & (1 << level) != 0) {
            cpu.assert_irq(level);
        } else {
            cpu.clear_irq();
        }
    }

    fn irq_control_w(&mut self, value: u8) {
        match value {
            0x10 => self.irq_status = 0,
            0x20 => self.irq_status &= !(1 << self.last_irq),
            _ => {}
        }
    }

    fn sound_ready_irq(&mut self) {
        if (self.sound.board.uart_tx_ready() || self.sound.board.uart_rx_ready())
            && self.irq_mask & (1 << 3) == 0
        {
            self.raise_irq(3);
        }
    }

    fn bank_w(&mut self, value: u16) {
        self.bank_reg = value;
        if value & 0x0f == 1 {
            self.bank_base = 0x0100_0000 + 0x0010_0000 * ((value as u32 >> 4) & 7);
        }
    }

    fn set_timer_period(&mut self, index: usize, value: u16) {
        self.timer_period[index] = value;
        self.timer_remaining[index] = u32::from(value) * 0x800;
    }

    fn advance_timers(&mut self, cycles: u32) {
        for index in 0..2 {
            let remaining = self.timer_remaining[index];
            if remaining == 0 {
                continue;
            }

            if cycles < remaining {
                self.timer_remaining[index] -= cycles;
                continue;
            }

            if self.irq_mask & 1 == 0 {
                self.raise_irq(0);
            }
            let period = u32::from(self.timer_period[index]) * 0x800;
            // Reload at the expiry instant, not at the end of this slice.
            // Repeated expiries coalesce into the same pending IRQ0 bit.
            self.timer_remaining[index] = if period == 0 {
                0
            } else {
                period - (cycles - remaining) % period
            };
        }
    }

    fn timer_r(&mut self, index: usize) -> u16 {
        if self.timer_period[index] == 0 {
            return self.timer_latched[index];
        }
        let count = (self.timer_remaining[index] / 0x800) as u16;
        if self.v60_access_active {
            self.timer_latched[index] = count;
        }
        count
    }

    fn dpram_write(&mut self, index: usize, value: u8) {
        if let Some(dst) = self.ioboard.dpram_mut().get_mut(index) {
            *dst = value;
        }
    }

    fn copro_ram_read(&mut self, high: bool) -> u16 {
        let value = self.copro_ram[(self.copro_ram_addr & 0x1fff) as usize];
        let result = if high {
            (value >> 16) as u16
        } else {
            value as u16
        };

        if high && self.copro_ram_addr & 0x8000 != 0 {
            self.copro_ram_addr = self.copro_ram_addr.wrapping_add(1);
        }
        result
    }

    fn copro_ram_write(&mut self, high: bool, value: u16) {
        self.copro_ram_latch[usize::from(high)] = value;
        if high {
            let word =
                u32::from(self.copro_ram_latch[0]) | (u32::from(self.copro_ram_latch[1]) << 16);
            let index = (self.copro_ram_addr & 0x1fff) as usize;
            self.copro_ram[index] = word;
            if self.copro_ram_addr & 0x8000 != 0 {
                self.copro_ram_addr = self.copro_ram_addr.wrapping_add(1);
            }
        }
    }

    /// Records one copro FIFO transfer in the autopsy ring (128 slots).
    /// Empty-pop stall retries are not transfers and are not recorded.
    fn fifo_note(&mut self, dir: char, value: u32) {
        if self.fifo_events.len() >= 128 {
            self.fifo_events.pop_front();
        }
        let depths = (self.copro_fifo_in.len(), self.copro_fifo_out.len());
        self.fifo_events.push_back((dir, value, depths.0, depths.1));
    }

    fn copro_fifo_read(&mut self, high: bool) -> u16 {
        if !high {
            if !self.v60_access_active {
                // Match side-effect-disabled inspection: no pop, no CPU clocks.
                self.copro_fifo_read_latch = self.copro_fifo_out.front().copied().unwrap_or(0);
                return self.copro_fifo_read_latch as u16;
            }
            let Some(value) = self.copro_fifo_out.pop_front() else {
                self.v60_fifo_waiting = true;
                self.v60_io_stall = true;
                return 0; // placeholder only; stalled IN must not commit it
            };
            self.v60_fifo_waiting = false;
            self.copro_fifo_read_latch = value;
            self.fifo_note('R', self.copro_fifo_read_latch);
            log::trace!(target: "fifo",
                "[fifo] V60 {:06X} pop  out={:08X} (out left {})",
                self.main_cpu.pc() & 0x00ff_ffff,
                self.copro_fifo_read_latch,
                self.copro_fifo_out.len()
            );
            self.copro_fifo_read_latch as u16
        } else {
            (self.copro_fifo_read_latch >> 16) as u16
        }
    }

    fn copro_fifo_write(&mut self, high: bool, value: u16) {
        if high {
            self.copro_fifo_write_latch =
                (self.copro_fifo_write_latch & 0x0000_ffff) | (u32::from(value) << 16);
            self.copro_fifo_in.push_back(self.copro_fifo_write_latch);
            self.fifo_note('W', self.copro_fifo_write_latch);
            log::trace!(target: "fifo",
                "[fifo] V60 {:06X} push in={:08X} (in {})",
                self.main_cpu.pc() & 0x00ff_ffff,
                self.copro_fifo_write_latch,
                self.copro_fifo_in.len()
            );
        } else {
            self.copro_fifo_write_latch =
                (self.copro_fifo_write_latch & 0xffff_0000) | u32::from(value);
        }
    }

    // --- TGP coprocessor I/O map
    // model1_m.cpp. The address registers, the RAM data window and the geometry
    // function units (sincos/atan/inv/isqrt) all read `copro_tables`/`copro_ram`.

    /// Advance one of the four RAM address registers after a data access: page 4
    /// (bit 0x40000) strides by 4, everything else by 1.
    fn copro_ramadr_step(&mut self, reg: usize) {
        let adr = self.copro_io_ram_adr[reg];
        self.copro_io_ram_adr[reg] = adr.wrapping_add(if adr & 0x40000 != 0 { 4 } else { 1 });
    }

    fn copro_ramdata_r(&mut self, reg: usize) -> u32 {
        let val = self.copro_ram[(self.copro_io_ram_adr[reg] & 0x1fff) as usize];
        self.copro_ramadr_step(reg);
        val
    }

    fn copro_ramdata_w(&mut self, reg: usize, data: u32) {
        let index = (self.copro_io_ram_adr[reg] & 0x1fff) as usize;
        self.copro_ram[index] = data;
        self.copro_ramadr_step(reg);
    }

    fn copro_sincos_r(&self, offset: u32) -> u32 {
        let ang = self.copro_sincos_base.wrapping_add(offset * 0x4000);
        let mut index = (ang & 0x3fff) as i32;
        if ang & 0x4000 != 0 {
            index = (0x4000 - index).min(0x3fff);
        }
        let mut result = self.copro_tables[index as usize];
        if ang & 0x8000 != 0 {
            result ^= 0x8000_0000;
        }
        result
    }

    fn copro_inv_r(&self, offset: u32) -> u32 {
        let index = ((self.copro_inv_base >> 9) & 0x3ffe) | (offset & 1);
        let result = self.copro_tables[(index | 0x8000) as usize];
        let bexp = (self.copro_inv_base >> 23) & 0xff;
        let exp = (result >> 23).wrapping_add(0x7f).wrapping_sub(bexp) & 0xff;
        let mut result = (result & 0x807f_ffff) | (exp << 23);
        if self.copro_inv_base & 0x8000_0000 != 0 {
            result ^= 0x8000_0000;
        }
        result
    }

    fn copro_isqrt_r(&self, offset: u32) -> u32 {
        let index = 0x2000 ^ (((self.copro_isqrt_base >> 10) & 0x3ffe) | (offset & 1));
        let result = self.copro_tables[(index | 0xc000) as usize];
        let bexp = (self.copro_isqrt_base >> 24) & 0x7f;
        let exp = (result >> 23).wrapping_add(0x3f).wrapping_sub(bexp) & 0xff;
        let mut result = (result & 0x807f_ffff) | (exp << 23);
        if offset & 1 == 0 {
            result &= 0x7fff_ffff;
        }
        result
    }

    fn copro_atan_r(&self) -> u32 {
        let mut idx = self.copro_atan_base[3] & 0xffff;
        if idx & 0xc000 != 0 {
            idx = 0x3fff;
        }
        let mut result = self.copro_tables[(idx | 0x4000) as usize];

        // Correct a known table bug the way the hardware effectively does.
        let dt = (result >> 16).wrapping_add(result) as u16;
        if dt & 0x001 != 0 {
            result = if result & 0x00f == 0x00e {
                result.wrapping_sub(0x0000_0001)
            } else {
                result.wrapping_sub(0x0001_0000)
            };
        }
        if dt & 0x010 != 0 {
            result = if result & 0x0f0 == 0x0e0 {
                result.wrapping_sub(0x0000_0010)
            } else {
                result.wrapping_sub(0x0010_0000)
            };
        }
        if dt & 0x100 != 0 {
            result = if result & 0xf00 == 0xe00 {
                result.wrapping_sub(0x0000_0100)
            } else {
                result.wrapping_sub(0x0100_0000)
            };
        }

        let s0 = self.copro_atan_base[0] & 0x8000_0000 != 0;
        let s1 = self.copro_atan_base[1] & 0x8000_0000 != 0;
        let s2 = self.copro_atan_base[2] & 0x8000_0000 != 0;
        if s0 ^ s1 ^ s2 {
            result >>= 16;
        }
        if s2 {
            result = result.wrapping_add(0x4000);
        }
        if (s0 && !s2) || (s1 && s2) {
            result = result.wrapping_add(0x8000);
        }
        result & 0xffff
    }

    fn copro_io_read(&mut self, address: u32) -> u32 {
        self.copro_io_read_inner(address)
    }

    fn copro_io_read_inner(&mut self, address: u32) -> u32 {
        match address {
            // RAM address / data registers, four banks selected by bits 3-4.
            a if a < 0x20 && a & 1 == 0 => {
                let v = self.copro_io_ram_adr[(a >> 3) as usize & 3];
                // The TGP writes 0xFFFFFFFF here as an "end of results" sentinel
                // (its auto-incrementing RAM pointer then wraps to slot 0). When the
                // microcode's polygon-count read lands on this register it would
                // otherwise take that sentinel as a count of ~4 billion and hang.
                // A sentinel is never a real count, so report it as empty (0), which
                // is what every count-0 object already does via unmapped addresses.
                if v == 0xFFFF_FFFF {
                    0
                } else {
                    v
                }
            }
            a if a < 0x20 => self.copro_ramdata_r((a >> 3) as usize & 3),
            0x20..=0x23 => self.copro_sincos_r(address - 0x20),
            0x24..=0x27 => self.copro_atan_r(),
            0x28..=0x29 => self.copro_inv_r(address - 0x28),
            0x2a..=0x2b => self.copro_isqrt_r(address & 1),
            0x8000..=0xffff => {
                // The reference copro_data_r receives a window-relative offset from
                // 0x0000 through 0x7fff, not the absolute I/O address.
                if self.copro_data.is_empty() {
                    0
                } else {
                    let offset = address & 0x7fff;
                    let index = (self.copro_data_base & !0x7fff) | offset;
                    self.copro_data[index as usize & (self.copro_data.len() - 1)]
                }
            }
            _ => 0,
        }
    }

    fn copro_io_write(&mut self, address: u32, data: u32) {
        match address {
            a if a < 0x20 && a & 1 == 0 => self.copro_io_ram_adr[(a >> 3) as usize & 3] = data,
            a if a < 0x20 => self.copro_ramdata_w((a >> 3) as usize & 3, data),
            0x20..=0x23 => self.copro_sincos_base = data,
            0x24..=0x27 => self.copro_atan_base[(address - 0x24) as usize] = data,
            0x28..=0x29 => self.copro_inv_base = data,
            0x2a..=0x2b => self.copro_isqrt_base = data,
            0x2e => self.copro_data_base = data,
            _ => {}
        }
    }

    fn normal_read(&mut self, address: u32) -> u8 {
        let address = address & 0x00ff_ffff;
        match address {
            0x000000..=0x0fffff | 0x200000..=0x2fffff | 0xf80000..=0xffffff => self
                .maincpu_rom
                .get(address as usize)
                .copied()
                .unwrap_or(0xff),
            0x100000..=0x1fffff => {
                let offset = self.bank_base + address - 0x100000;
                self.maincpu_rom
                    .get(offset as usize)
                    .copied()
                    .unwrap_or(0xff)
            }
            0x400000..=0x40ffff => self.nvram[(address - 0x400000) as usize],
            0x500000..=0x53ffff => self.work_ram[(address - 0x500000) as usize],
            0x600000..=0x60ffff => self.display_list[0][(address - 0x600000) as usize],
            0x610000..=0x61ffff => self.display_list[1][(address - 0x610000) as usize],
            0x680000..=0x680003 => {
                let index = ((address - 0x680000) >> 1) as usize;
                let mut value = self.listctl[index];
                if index == 0 {
                    value |= 0x30;
                }
                (value >> ((address & 1) * 8)) as u8
            }
            0x700000..=0x70ffff => self.tile_ram[(address - 0x700000) as usize],
            0x780000..=0x7fffff => self.char_ram[(address - 0x780000) as usize],
            0x900000..=0x903fff => self.palette_ram[(address - 0x900000) as usize],
            0x910000..=0x91bfff => self.colorxlat_ram[(address - 0x910000) as usize],
            0xb00000..=0xb00fff => self
                .comm
                .as_ref()
                .map_or(0xff, |c| c.shared_read((address - 0xb00000) as usize)),
            0xb01000 => self.comm.as_ref().map_or(0xff, |c| c.cn_read()),
            0xb01002 => self.comm.as_ref().map_or(0xff, |c| c.fg_read()),
            0xc00000..=0xc00fff if address & 1 == 0 => {
                // MAME model1_state::dpram_r charges one V60 cycle per
                // low-byte read on the shared map, independent of I/O-board
                // revision. Debugger/renderer reads have no timing side effects.
                if self.v60_access_active {
                    self.v60_wait_cycles += 1;
                }
                let idx = ((address - 0xc00000) >> 1) as usize & 0x7ff;

                self.ioboard.dpram()[idx]
            }
            0xc40000 => self.sound.board.main_uart_read(),
            0xc40002 => self.sound.board.main_uart_status(),
            0xe00002 => self.irq_mask,
            0xe0000c..=0xe0000f => {
                let index = ((address - 0xe0000c) >> 1) as usize;
                (self.timer_r(index) >> ((address & 1) * 8)) as u8
            }
            _ => 0xff,
        }
    }

    fn normal_write(&mut self, address: u32, value: u8) {
        let address = address & 0x00ff_ffff;
        match address {
            0x400000..=0x40ffff => self.nvram[(address - 0x400000) as usize] = value,
            0x500000..=0x53ffff => self.work_ram[(address - 0x500000) as usize] = value,
            0x600000..=0x60ffff => self.display_list[0][(address - 0x600000) as usize] = value,
            0x610000..=0x61ffff => self.display_list[1][(address - 0x610000) as usize] = value,
            0x680000..=0x680003 => {
                let index = ((address - 0x680000) >> 1) as usize;
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                self.listctl[index] = (self.listctl[index] & !mask) | (u16::from(value) << shift);
            }
            0x700000..=0x70ffff => self.tile_ram[(address - 0x700000) as usize] = value,
            0x780000..=0x7fffff => self.char_ram[(address - 0x780000) as usize] = value,
            0x900000..=0x903fff => self.palette_ram[(address - 0x900000) as usize] = value,
            0x910000..=0x91bfff => self.colorxlat_ram[(address - 0x910000) as usize] = value,
            0xb00000..=0xb00fff => {
                if let Some(c) = &mut self.comm {
                    c.shared_write((address - 0xb00000) as usize, value);
                }
            }
            0xb01000 => {
                if let Some(c) = &mut self.comm {
                    c.cn_write(value);
                }
            }
            0xb01002 => {
                if let Some(c) = &mut self.comm {
                    c.fg_write(value);
                }
            }
            0xc00000..=0xc00fff if address & 1 == 0 => {
                let index = ((address - 0xc00000) >> 1) as usize & 0x7ff;
                self.dpram_write(index, value);
            }
            0xc40000 => self.sound.send(value),
            0xc40002 => self.sound.board.uart_control(value),
            0xe00000 => self.irq_control_w(value),
            0xe00002 => {
                self.irq_mask = value;
                // MAME irq_mask_w re-evaluates an already asserted UART ready.
                self.sound_ready_irq();
            }
            0xe00004..=0xe00005 => {
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                let combined = (self.bank_reg & !mask) | (u16::from(value) << shift);
                self.bank_w(combined);
            }
            0xe00006..=0xe00007 => {
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                self.timer_mode = (self.timer_mode & !mask) | (u16::from(value) << shift);
            }
            0xe00008..=0xe0000b => {
                let index = ((address - 0xe00008) >> 1) as usize;
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                let combined = (self.timer_period[index] & !mask) | (u16::from(value) << shift);
                self.set_timer_period(index, combined);
            }
            _ => {}
        }
    }
}

impl Bus for Model1System {
    fn take_wait_cycles(&mut self) -> u32 {
        std::mem::take(&mut self.v60_wait_cycles)
    }

    fn take_io_stall(&mut self) -> bool {
        std::mem::take(&mut self.v60_io_stall)
    }
    /// Live GLUE interrupt line: asserted while any raised level is still
    /// pending. The ISR's `E00000` acknowledge clears `irq_status` mid-run, so
    /// the CPU consults this instead of re-taking its latched line after `reti`.
    fn irq_active(&self) -> Option<bool> {
        Some(self.irq_status != 0)
    }

    fn irq_acknowledge(&mut self) -> Option<u8> {
        if let Some(level) = (0..8).find(|level| self.irq_status & (1 << level) != 0) {
            self.last_irq = level;
        }
        Some(self.last_irq)
    }

    fn read_u8(&mut self, address: u32) -> u8 {
        let address = address & 0x00ff_ffff;
        let word = match address {
            0xd00000..=0xd1ffff => self.copro_ram_addr,
            0xd20000..=0xd3ffff => self.copro_ram_read(address & 2 != 0),
            0xd80000..=0xd9ffff => self.copro_fifo_read(address & 2 != 0),
            0xdc0000..=0xddffff => 0xffff,
            _ => return self.normal_read(address),
        };
        (word >> ((address & 1) * 8)) as u8
    }

    fn write_u8(&mut self, address: u32, value: u8) {
        let address = address & 0x00ff_ffff;
        match address {
            0xd00000..=0xd1ffff => {
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                self.copro_ram_addr = (self.copro_ram_addr & !mask) | (u16::from(value) << shift);
            }
            0xd20000..=0xd3ffff => {
                let high = address & 2 != 0;
                let index = usize::from(high);
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                let combined = (self.copro_ram_latch[index] & !mask) | (u16::from(value) << shift);
                self.copro_ram_write(high, combined);
            }
            0xd80000..=0xd9ffff => {
                let high = address & 2 != 0;
                let current = if high {
                    (self.copro_fifo_write_latch >> 16) as u16
                } else {
                    self.copro_fifo_write_latch as u16
                };
                let shift = (address & 1) * 8;
                let mask = 0xffu16 << shift;
                self.copro_fifo_write(high, (current & !mask) | (u16::from(value) << shift));
            }
            _ => self.normal_write(address, value),
        }
    }

    fn read_u16(&mut self, address: u32) -> u16 {
        let address = address & 0x00ff_ffff;
        match address {
            0xd00000..=0xd1ffff => self.copro_ram_addr,
            0xd20000..=0xd3ffff => self.copro_ram_read(address & 2 != 0),
            0xd80000..=0xd9ffff => self.copro_fifo_read(address & 2 != 0),
            0xdc0000..=0xddffff => 0xffff,
            _ => u16::from_le_bytes([
                self.normal_read(address),
                self.normal_read(address.wrapping_add(1)),
            ]),
        }
    }

    fn write_u16(&mut self, address: u32, value: u16) {
        let address = address & 0x00ff_ffff;
        match address {
            0xd00000..=0xd1ffff => self.copro_ram_addr = value,
            0xd20000..=0xd3ffff => self.copro_ram_write(address & 2 != 0, value),
            0xd80000..=0xd9ffff => self.copro_fifo_write(address & 2 != 0, value),
            0xe00004..=0xe00005 => self.bank_w(value),
            0xe00006..=0xe00007 => self.timer_mode = value,
            0xe00008..=0xe00009 => self.set_timer_period(0, value),
            0xe0000a..=0xe0000b => self.set_timer_period(1, value),
            _ => {
                let bytes = value.to_le_bytes();
                self.normal_write(address, bytes[0]);
                self.normal_write(address.wrapping_add(1), bytes[1]);
            }
        }
    }

    fn read_u32(&mut self, address: u32) -> u32 {
        let lo = u32::from(self.read_u16(address));
        let hi = u32::from(self.read_u16(address.wrapping_add(2)));
        lo | (hi << 16)
    }

    fn write_u32(&mut self, address: u32, value: u32) {
        self.write_u16(address, value as u16);
        self.write_u16(address.wrapping_add(2), (value >> 16) as u16);
    }

    fn halt_requested(&self) -> bool {
        // The full-post-sync callback
        // asserts INPUT_LINE_HALT on the V60 after accepting the overflow word.
        self.copro_fifo_in.len() > COPRO_FIFO_DEPTH || self.v60_fifo_waiting
    }
}

impl crate::tilemap::TileSource for Model1System {
    fn tile_u16(&self, idx: usize) -> u16 {
        le16(&self.tile_ram, idx)
    }
    fn char_word(&self, idx: usize) -> u32 {
        le32(&self.char_ram, idx)
    }
    fn palette_u16(&self, idx: usize) -> u16 {
        le16(&self.palette_ram, idx)
    }
    fn colorxlat_u16(&self, idx: usize) -> u16 {
        le16(&self.colorxlat_ram, idx)
    }
    fn colorxlat_written(&self) -> bool {
        // VR programs the translation RAM, but until that path is verified the
        // 5->8-bit expansion fallback keeps the picture legible.
        false
    }
    fn monitor_gamma(&self, v: u32) -> u32 {
        v & 0xff
    }
    fn palette_dimmed(&self, colour: u16) -> bool {
        // MAME model1_paletteram_w: expand to 8 bits, then halve each channel.
        colour & 0x8000 == 0
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    fn empty_roms() -> Model1Roms {
        Model1Roms {
            netmerc_procedural_audio: false,
            dsb: None,
            comm_board: false,
            ioboard_kind: crate::model1board::Kind::Original,
            nvram_default: vec![],
            maincpu: vec![],
            tgp: vec![],
            copro_tables: vec![],
            polygons: vec![],
            copro_data: vec![],
            iocpu: vec![],
            sndcpu: vec![],
            mpcm1: vec![],
            mpcm2: vec![],
            ioboard_config: vec![],
        }
    }

    #[test]
    fn comm_bus_lanes_and_masked_vblank() {
        let mut roms = empty_roms();
        let mut absent = Model1System::new(&roms).unwrap();
        absent.write_u32(0xb00000, 0x12345678);
        assert_eq!(absent.read_u32(0xb00000), u32::MAX);
        roms.comm_board = true;
        let mut sys = Model1System::with_config(
            &roms,
            Config {
                cabinet: crate::config::Cabinet::Twin,
                ..Config::default()
            },
        )
        .unwrap();
        sys.write_u32(0xb00008, 0x12345678);
        assert_eq!(sys.read_u32(0xb00008), 0x12345678);
        sys.write_u16(0xb00ffe, 0xbeef);
        assert_eq!(sys.read_u16(0xb00ffe), 0xbeef);
        sys.write_u16(0xb01000, 0xff01);
        assert_eq!(sys.read_u16(0xb01000), 0xffff);
        sys.write_u16(0xb01002, 0xff01);
        assert_eq!(sys.read_u16(0xb01002), 0xffff);
        sys.irq_mask = 0xff;
        sys.trigger_vblank();
        assert_eq!(sys.read_u8(0xb00000), 5);
        sys.write_u8(0xb01000, 0);
        assert_eq!(sys.read_u16(0xb01000), 0xfffe);
        assert_eq!(sys.read_u8(0xb01003), 0xff);
    }

    #[test]
    fn vblank_uploads_completed_list_before_automatic_buffer_switch() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.listctl = [4, 0x1f];
        sys.frame_num = 1;
        for (index, value) in [0x1234u32, 0x5678].into_iter().enumerate() {
            // Polygon upload, destination 0, length 1, then end-of-list.
            for (word, data) in [5u32, 0x800000, 1, value, 0xf].into_iter().enumerate() {
                sys.display_list[index][word * 4..word * 4 + 4]
                    .copy_from_slice(&data.to_le_bytes());
            }
        }
        sys.trigger_vblank();
        assert_eq!(sys.video.poly_ram[0], 0x1234);
        assert_eq!(sys.listctl[0] & 0x40, 0x40);
        sys.trigger_vblank();
        assert_eq!(sys.video.poly_ram[0], 0x5678);
        assert_eq!(sys.listctl[0] & 0x40, 0x40);
    }

    #[test]
    fn cabinet_selects_comm_presence_without_changing_model1_nvram() {
        for supported in [false, true] {
            for cabinet in [crate::config::Cabinet::Single, crate::config::Cabinet::Twin] {
                let mut roms = empty_roms();
                roms.comm_board = supported;
                let mut sys = Model1System::with_config(
                    &roms,
                    Config {
                        cabinet,
                        ..Config::default()
                    },
                )
                .unwrap();
                let fitted = supported && cabinet == crate::config::Cabinet::Twin;
                assert_eq!(sys.comm.is_some(), fitted);
                let (bl, el) = sys.nvram_sizes();
                let backup = vec![0x5a; bl];
                let eeprom = vec![0xa5; el];
                sys.set_nvram_blocks(&backup, &eeprom);
                assert_eq!(sys.nvram_blocks(), (backup, eeprom));
                if !fitted {
                    sys.write_u32(0xb00000, 0);
                    sys.write_u8(0xb01000, 1);
                    assert_eq!(sys.read_u32(0xb00000), u32::MAX);
                    assert_eq!(sys.read_u8(0xb01000), 0xff);
                }
            }
        }
    }

    #[test]
    fn tile_palette_intensity_matches_mame_for_every_colour() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        for colour in 0..=u16::MAX {
            sys.palette_ram[..2].copy_from_slice(&colour.to_le_bytes());
            let channel = |shift: u32| {
                let v = (u32::from(colour) >> shift) & 31u32;
                let expanded = (v << 3) | (v >> 2);
                if colour & 0x8000 == 0 {
                    expanded >> 1
                } else {
                    expanded
                }
            };
            let expected = 0xff00_0000 | channel(0) << 16 | channel(5) << 8 | channel(10);
            assert_eq!(crate::tilemap::pen_color(&sys, 0), expected, "{colour:04x}");
        }
    }

    #[test]
    fn dsb_resources_select_board_and_fault_reaches_machine_caller() {
        let mut roms = empty_roms();
        assert!(Model1System::new(&roms).unwrap().sound.board.dsb.is_none());
        roms.dsb = Some(crate::loader::DsbRoms {
            firmware: vec![0; crate::dsbz80::FIRMWARE_SIZE],
            mpeg: vec![0; 0x400000],
        });
        let mut sys = Model1System::new(&roms).unwrap();
        sys.sound.send(1);
        sys.sound.send(2);
        assert_eq!(
            sys.run_slice(0),
            Err(Error::Serial(crate::sound::SerialError::TransmitFull))
        );
    }

    #[test]
    fn cpu_state_survives_a_board_fault_during_a_slice() {
        let mut roms = empty_roms();
        roms.maincpu = vec![0]; // HALT
        roms.ioboard_kind = crate::model1board::Kind::WingWar;
        roms.iocpu = vec![0; 0x10000];
        roms.iocpu[..3].copy_from_slice(&[0x3a, 0x00, 0x81]); // LD A, (unsupported device)
        let mut sys = Model1System::new(&roms).unwrap();
        sys.main_cpu.reg[v60::cpu::PC] = 0;
        sys.main_cpu.reg[5] = 0x1234_5678;
        sys.tgp_cpu.pc = 0x123;
        sys.tgp_cpu.a = 0x8765_4321;
        let tgp_before = bincode::serialize(&sys.tgp_cpu).unwrap();
        let error = Err(Error::IoBoard(
            crate::model1io2::BusError::UnimplementedMemory(0x8100),
        ));
        assert_eq!(sys.run_slice(64), error);
        assert!(sys.main_cpu.halted, "completed V60 work must be restored");
        assert_eq!(sys.main_cpu.reg[5], 0x1234_5678);
        assert_eq!(sys.main_cpu.op_count[0], 1);
        assert_eq!(bincode::serialize(&sys.tgp_cpu).unwrap(), tgp_before);
        let main_before = bincode::serialize(&sys.main_cpu).unwrap();
        assert_eq!(sys.run_slice(64), error);
        assert_eq!(bincode::serialize(&sys.main_cpu).unwrap(), main_before);
        assert_eq!(bincode::serialize(&sys.tgp_cpu).unwrap(), tgp_before);
    }

    #[test]
    fn timer_reload_preserves_phase_across_slices_and_multiple_expiries() {
        for chunk in [1, 3, 63, 64, 127, 2049, 20000] {
            let mut sys = Model1System::new(&empty_roms()).unwrap();
            sys.irq_mask = 0xfe;
            sys.set_timer_period(0, 1);
            sys.set_timer_period(1, 3);
            let total = 7 * 0x800 + 17;
            let mut left = total;
            while left > 0 {
                let step = left.min(chunk);
                sys.advance_timers(step);
                left -= step;
            }
            assert_eq!(sys.timer_remaining, [0x800 - 17, 2 * 0x800 - 17], "{chunk}");
            assert_eq!(sys.irq_status, 1); // repeated IRQ0s coalesce
        }
    }

    #[test]
    fn timer_masks_stop_restart_and_register_lanes_match_reference() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.write_u8(0xe00008, 2);
        sys.write_u8(0xe00009, 1);
        assert_eq!(sys.timer_period[0], 0x102);
        assert_eq!(sys.timer_remaining[0], 0x102 * 0x800);
        sys.write_u16(0xe00008, 1);
        sys.advance_timers(0x800 - 1);
        assert_eq!(sys.timer_remaining[0], 1);
        assert_eq!(sys.irq_status, 0);
        sys.advance_timers(1); // masked expiry still reloads
        assert_eq!(sys.timer_remaining[0], 0x800);
        assert_eq!(sys.irq_status, 0);
        sys.write_u8(0xe00002, 0xfe); // no retroactive timer IRQ
        assert_eq!(sys.irq_status, 0);
        sys.advance_timers(0x800);
        assert_eq!(sys.irq_status, 1);
        sys.write_u8(0xe00002, 0xff); // masking does not clear pending IRQ
        assert_eq!(sys.irq_status, 1);
        sys.write_u8(0xe00000, 0x10);
        sys.write_u16(0xe00008, 0);
        sys.advance_timers(10000);
        assert_eq!((sys.timer_remaining[0], sys.irq_status), (0, 0));
        sys.write_u16(0xe00008, 2);
        sys.write_u16(0xe0000c, 0); // count writes ignored
        sys.write_u16(0xe00006, 0xffff); // mode stored; no effect in MAME
        assert_eq!((sys.timer_mode, sys.timer_remaining[0]), (0xffff, 0x1000));
    }

    #[test]
    fn timer_debugger_read_does_not_change_stopped_latch() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.set_timer_period(0, 10);
        sys.v60_access_active = true;
        sys.advance_timers(3 * 0x800);
        assert_eq!(sys.read_u16(0xe0000c), 7);
        sys.v60_access_active = false;
        sys.advance_timers(2 * 0x800);
        assert_eq!(sys.read_u16(0xe0000c), 5);
        sys.set_timer_period(0, 0);
        assert_eq!(sys.read_u16(0xe0000c), 7);
    }

    fn irq_test_machine(program: &[u8]) -> Model1System {
        let mut roms = empty_roms();
        roms.maincpu = program.to_vec();
        let mut sys = Model1System::new(&roms).unwrap();
        sys.main_cpu.reg[v60::cpu::PC] = 0;
        sys.main_cpu.reg[v60::cpu::PSW] = 1 << 18;
        sys.main_cpu.reg[v60::cpu::SP] = 0x53e000;
        sys.main_cpu.reg[36] = 0x53f000; // ISP
        sys.main_cpu.reg[v60::cpu::SBR] = 0x500000;
        for level in 0..8 {
            sys.write_u32(0x500000 + (0x40 + level) * 4, 0x501000 + level * 0x100);
        }
        sys
    }

    #[test]
    fn irq_acknowledge_latches_only_the_level_accepted_by_the_cpu() {
        let mut sys = irq_test_machine(&[0xcd; 32]);
        sys.main_cpu.reg[v60::cpu::PSW] = 0; // IE disabled
        sys.raise_irq(3);
        sys.run_slice(8).unwrap();
        assert_eq!((sys.last_irq, sys.main_cpu.irq_taken), (0, 0));
        sys.main_cpu.reg[v60::cpu::PSW] = 1 << 18;
        sys.run_slice(8).unwrap(); // handler HALT, IE cleared on entry
        assert_eq!((sys.last_irq, sys.main_cpu.irq_taken), (3, 1));
        sys.raise_irq(0); // higher-priority pending IRQ during handler
        sys.run_slice(64).unwrap();
        assert_eq!(sys.last_irq, 3);
        sys.write_u8(0xe00000, 0x20); // clears the accepted IRQ3, not pending IRQ0
        assert_eq!(sys.irq_status, 1);
        sys.write_u8(0xe00000, 0x30); // unknown command ignored
        assert_eq!(sys.irq_status, 1);
        sys.write_u8(0xe00000, 0x10);
        assert_eq!(sys.irq_status, 0);
    }

    #[test]
    fn irq_reti_selects_next_pending_vector_in_same_slice() {
        let mut sys = irq_test_machine(&[0xcd, 0]);
        // OUT.B R0,absolute 0xe00000; RETIU #0.
        let handler = [0x21, 0x00, 0xf3, 0, 0, 0xe0, 0, 0xea, 0xe0];
        for level in [1, 3] {
            for (index, byte) in handler.iter().enumerate() {
                sys.write_u8(0x501000 + level * 0x100 + index as u32, *byte);
            }
        }
        sys.main_cpu.reg[0] = 0x20;
        sys.raise_irq(3);
        sys.raise_irq(1);
        sys.run_slice(64).unwrap();
        assert_eq!(sys.main_cpu.irq_taken, 2);
        assert_eq!(
            (sys.last_irq, sys.main_cpu.irq_vector, sys.irq_status),
            (3, 3, 0)
        );
        assert!(sys.main_cpu.halted);
        assert_eq!(sys.main_cpu.pc(), 2);
    }

    #[test]
    fn irq_uart_unmask_asserts_and_vectors_within_current_cpu_slice() {
        // OUT.B R0,absolute 0xe00002; HALT.
        let mut sys = irq_test_machine(&[0x21, 0x00, 0xf3, 2, 0, 0xe0, 0, 0]);
        sys.sound.board.uart_control(0x4e);
        sys.sound.board.uart_control(0x37);
        sys.main_cpu.reg[0] = 0xf7;
        assert!(!sys.main_cpu.irq_line);
        sys.run_slice(8).unwrap();
        assert_eq!(sys.irq_status, 8);
        assert_eq!((sys.last_irq, sys.main_cpu.irq_taken), (3, 1));
        assert_eq!(sys.main_cpu.pc(), 0x501300); // before first handler instruction
        assert_eq!(sys.read_u32(0x53eff8), 7); // interrupted immediately after OUT
    }

    #[test]
    fn timer_retains_last_read_count_when_stopped() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.v60_access_active = true;
        for index in 0..2 {
            let period = 0xe00008 + index as u32 * 2;
            let count = 0xe0000c + index as u32 * 2;
            assert_eq!(sys.read_u16(count), 0);
            sys.write_u16(period, 10);
            sys.advance_timers(3 * 0x800);
            assert_eq!(sys.read_u16(count), 7);
            sys.advance_timers(2 * 0x800);
            sys.write_u16(period, 0);
            sys.advance_timers(100 * 0x800);
            // Retain the last *read*, not the count at the moment of stopping.
            assert_eq!(sys.read_u16(count), 7);
            sys.write_u16(period, 4);
            assert_eq!(sys.read_u16(count), 4);
            sys.advance_timers(4 * 0x800);
            assert_eq!(sys.read_u16(count), 4);
        }
    }

    #[test]
    fn factory_nvram_is_overridden_by_saved_nvram() {
        let mut roms = empty_roms();
        roms.nvram_default = vec![0x5a; 0x10000];
        let mut sys = Model1System::new(&roms).unwrap();
        assert_eq!(sys.nvram, roms.nvram_default);
        sys.set_nvram_blocks(&vec![0xa5; 0x10000], &[]);
        assert_eq!(sys.nvram, vec![0xa5; 0x10000]);
    }

    #[test]
    fn netmerc_saved_sram_and_state_restore_are_not_automatically_repaired() {
        let mut roms = empty_roms();
        roms.ioboard_kind = crate::model1board::Kind::NetMerc;
        roms.iocpu = vec![0x76; 0x10000];
        // Loader supplies an initialized default; motherboard copies it as-is.
        roms.nvram_default = vec![0; 0x10000];
        roms.nvram_default[0] = 0x0f;
        let mut sys = Model1System::new(&roms).unwrap();
        assert_eq!(sys.nvram, roms.nvram_default);

        // Even the old uninitialized signature is preserved in personal saves.
        let mut personal = vec![0xff; 0x10000];
        personal[4..8].copy_from_slice(&12345u32.to_le_bytes());
        personal[0x5c..0x60].copy_from_slice(&2u32.to_le_bytes());
        let eeprom: Vec<_> = (0..128).map(|i| (i * 7) as u8).collect();
        sys.set_nvram_blocks(&personal, &eeprom);
        assert_eq!(sys.nvram_blocks(), (personal.clone(), eeprom.clone()));
        assert_eq!(personal[0], 0xff); // Caller-owned SRAM stays unchanged.

        let (backup, stored_eeprom) = sys.nvram_blocks();
        let container = crate::nvram::encode(&backup, &stored_eeprom);
        let (backup, stored_eeprom) = crate::nvram::decode(&container, 65536, 128).unwrap();
        let mut restarted = Model1System::new(&roms).unwrap();
        restarted.set_nvram_blocks(&backup, &stored_eeprom);
        assert_eq!(restarted.nvram_blocks(), (personal.clone(), eeprom));

        // A machine snapshot must retain even the uninitialized signature:
        // restore is exact state replacement, never automatic SRAM repair.
        let saved = sys.save_state().unwrap();
        restarted.load_state(&saved).unwrap();
        assert_eq!(restarted.nvram, personal);
        assert_eq!(restarted.save_state().unwrap(), saved);

        // Partial loads cannot accidentally manufacture a matching signature.
        restarted.set_nvram_blocks(&[0xff], &[]);
        assert_eq!(restarted.nvram, personal);
    }

    #[test]
    fn absent_or_invalid_factory_nvram_preserves_zero_default() {
        let mut roms = empty_roms();
        for size in [0, 1, 0x10001] {
            roms.nvram_default = vec![0xff; size];
            assert_eq!(Model1System::new(&roms).unwrap().nvram, vec![0; 0x10000]);
        }
    }

    #[test]
    fn city_preference_changes_only_netmerc_conversion_and_survives_state_restore() {
        use crate::model1board::Kind;
        for kind in [Kind::Original, Kind::WingWar, Kind::NetMerc] {
            let mut roms = empty_roms();
            roms.ioboard_kind = kind;
            if kind != Kind::Original { roms.iocpu = vec![0x76; 0x10000]; }
            let mut machine = Model1System::new(&roms).unwrap();
            let arithmetic = machine.tgp_cpu.float_mode;
            let saved_enabled = machine.save_state().unwrap();
            machine.set_netmerc_city_workaround(false);
            assert!(!machine.tgp_cpu.netmerc_city_conversion);
            assert!(!machine.config.netmerc_city_workaround);
            assert_eq!(machine.tgp_cpu.float_mode, arithmetic);
            assert_eq!(machine.save_state().unwrap(), saved_enabled);
            machine.load_state(&saved_enabled).unwrap();
            assert!(!machine.tgp_cpu.netmerc_city_conversion);
            let saved_disabled = machine.save_state().unwrap();
            machine.set_netmerc_city_workaround(true);
            assert_eq!(machine.tgp_cpu.netmerc_city_conversion, kind == Kind::NetMerc);
            assert!(machine.config.netmerc_city_workaround);
            machine.load_state(&saved_disabled).unwrap();
            assert_eq!(machine.tgp_cpu.netmerc_city_conversion, kind == Kind::NetMerc);
            assert_eq!(machine.tgp_cpu.float_mode, arithmetic);
        }
    }

    #[test]
    fn netmerc_forward_pose_is_cold_boot_data_not_a_restore_side_effect() {
        use crate::model1board::Kind;
        let mut roms = empty_roms();
        roms.iocpu = vec![0x76; 0x10000];
        roms.ioboard_kind = Kind::WingWar;
        assert_eq!(
            &Model1System::new(&roms).unwrap().ioboard.dpram()[0x80..0x8c],
            &[0; 12]
        );
        roms.ioboard_kind = Kind::NetMerc;
        let mut original = Model1System::new(&roms).unwrap();
        let expected: Vec<_> = [0i16, 0, 0, 12868, 25736, 12868]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect();
        assert_eq!(&original.ioboard.dpram()[0x80..0x8c], &expected);
        assert_eq!(original.mvd_holder_latched(), Some(false));
        original.nvram[0x18..0x1a].copy_from_slice(&1u16.to_le_bytes());
        original.work_ram[0x27668..0x2766a].copy_from_slice(&1u16.to_le_bytes());
        original.work_ram[0x2766a..0x2766c].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(original.mvd_holder_latched(), Some(true));
        original.ioboard.dpram_mut()[0x86..0x88].copy_from_slice(&0x1234i16.to_le_bytes());
        let saved = original.save_state().unwrap();
        let mut restored = Model1System::new(&roms).unwrap();
        restored.load_state(&saved).unwrap();
        assert_eq!(restored.mvd_holder_latched(), Some(true));
        assert_eq!(
            restored.mvd_holder_context(),
            Some(MvdHolderContext {
                latched: true,
                credits: 1,
                in_game: true
            })
        );
        assert_eq!(
            &restored.ioboard.dpram()[0x80..0x8c],
            &original.ioboard.dpram()[0x80..0x8c]
        );
        assert_eq!(&restored.ioboard.dpram()[0x86..0x88], &[0x34, 0x12]);
    }

    #[test]
    fn nvram_container_round_trips_complete_eeprom() {
        for kind in [
            crate::model1board::Kind::Original,
            crate::model1board::Kind::WingWar,
            crate::model1board::Kind::NetMerc,
        ] {
            let mut roms = empty_roms();
            roms.ioboard_kind = kind;
            roms.iocpu = vec![0; 0x10000];
            let mut original = Model1System::new(&roms).unwrap();
            original.nvram[0] = 0x5a;
            original.nvram[0xffff] = 0xa5;
            for (index, word) in original.ioboard.eeprom_mut().data.iter_mut().enumerate() {
                *word = 0xa500 | index as u16;
            }
            original.ioboard.dpram_mut()[0x20] = 0x5a;
            original.ioboard.dpram_mut()[0x80..0x8c].fill(0xa5);
            let (backup, eeprom) = original.nvram_blocks();
            assert_eq!(eeprom.len(), 128);
            let blob = crate::nvram::encode(&backup, &eeprom);
            let mut restored = Model1System::new(&roms).unwrap();
            let cold_pose = restored.ioboard.dpram()[0x80..0x8c].to_vec();
            let (backup_len, eeprom_len) = restored.nvram_sizes();
            let (loaded_backup, loaded_eeprom) =
                crate::nvram::decode(&blob, backup_len, eeprom_len)
                    .expect("a Model 1 save must be accepted by the next instance");
            restored.set_nvram_blocks(&loaded_backup, &loaded_eeprom);
            assert_eq!(restored.nvram_blocks(), (backup, eeprom));
            // Persistent operator data must not import a volatile mailbox or
            // the previous session's HMD pose into a fresh machine.
            assert_eq!(restored.ioboard.dpram()[0x20], 0);
            assert_eq!(&restored.ioboard.dpram()[0x80..0x8c], &cold_pose);
        }
    }

    #[test]
    fn tgp_clock_ratio_is_independent_of_slice_partition() {
        for chunk in [1, 2, 3, 7, 63, 64, 65, 127, 257] {
            let mut sys = Model1System::new(&empty_roms()).unwrap();
            sys.main_cpu.reg[v60::cpu::PC] = 0; // zero ROM: HALT
            assert_eq!(sys.tgp_clock_remainder, 0);
            let mut remaining = 257;
            while remaining > 0 {
                let step = chunk.min(remaining);
                sys.run_slice(step).unwrap();
                remaining -= step;
            }
            // Default LAB without ALU work costs one TGP cycle.
            assert_eq!(sys.tgp_cpu.pc, 642, "chunk {chunk}");
            assert_eq!(sys.tgp_clock_remainder, 1, "chunk {chunk}");
            sys.run_slice(1).unwrap();
            assert_eq!(sys.tgp_cpu.pc, 645);
            assert_eq!(sys.tgp_clock_remainder, 0);
        }
    }

    #[test]
    fn tgp_two_cycle_alu_debt_survives_slice_boundaries() {
        // All two-cycle ALU families from MAME mb86233::alu_post_2.
        for alu in [5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 17, 19, 20] {
            let machine = || {
                let mut sys = Model1System::new(&empty_roms()).unwrap();
                sys.main_cpu.reg[v60::cpu::PC] = 0;
                sys.tgp_program = vec![alu << 21; 1024];
                sys
            };
            let mut whole = machine();
            whole.run_slice(257).unwrap();
            for chunk in [1, 3, 7, 63, 65, 127] {
                let mut split = machine();
                let mut remaining = 257;
                while remaining > 0 {
                    let step = chunk.min(remaining);
                    split.run_slice(step).unwrap();
                    remaining -= step;
                }
                assert_eq!(split.tgp_cpu.pc, 321, "ALU {alu}, chunk {chunk}");
                assert_eq!(split.tgp_clock_remainder, whole.tgp_clock_remainder);
                assert_eq!(
                    bincode::serialize(&split.tgp_cpu).unwrap(),
                    bincode::serialize(&whole.tgp_cpu).unwrap(),
                    "ALU {alu}, chunk {chunk}"
                );
                // Three more TGP clocks execute two instructions, carrying -1.
                split.run_slice(1).unwrap();
                assert_eq!((split.tgp_cpu.pc, split.tgp_cpu.icount), (323, -1));
                let state = bincode::serialize(&split.tgp_cpu).unwrap();
                split.run_slice(0).unwrap();
                split.run_slice(-1).unwrap();
                assert_eq!(bincode::serialize(&split.tgp_cpu).unwrap(), state);
                split.run_slice(1).unwrap();
                assert_eq!((split.tgp_cpu.pc, split.tgp_cpu.icount), (324, -1));
            }
        }
    }

    #[test]
    fn tgp_halt_advances_clock_phase_and_retires_debt_without_banking_time() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.main_cpu.reg[v60::cpu::PC] = 0;
        sys.tgp_program = vec![8 << 21; 1024]; // two-cycle FML
        sys.tgp_program[0] = 0; // one-cycle LAB, so the first slice overshoots
        sys.run_slice(1).unwrap();
        assert_eq!(
            (sys.tgp_cpu.pc, sys.tgp_cpu.icount, sys.tgp_clock_remainder),
            (2, -1, 1)
        );
        sys.copro_fifo_out.extend([0; 17]); // external HALT
        sys.run_slice(1).unwrap();
        assert_eq!(
            (sys.tgp_cpu.pc, sys.tgp_cpu.icount, sys.tgp_clock_remainder),
            (2, 0, 0)
        );
        sys.run_slice(65).unwrap();
        assert_eq!(
            (sys.tgp_cpu.pc, sys.tgp_cpu.icount, sys.tgp_clock_remainder),
            (2, 0, 1)
        );
        sys.copro_fifo_out.clear();
        sys.run_slice(1).unwrap(); // only the new three clocks, no HALT-time burst
        assert_eq!(
            (sys.tgp_cpu.pc, sys.tgp_cpu.icount, sys.tgp_clock_remainder),
            (4, -1, 0)
        );
    }

    #[test]
    fn tgp_empty_fifo_does_not_bank_unused_instruction_budget() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.main_cpu.reg[v60::cpu::PC] = 0;
        sys.tgp_program = vec![0x100; 1024]; // LAB from command FIFO
        sys.tgp_cpu.b0 = 0x100;
        sys.tgp_cpu.i0 = 0;
        sys.run_slice(128).unwrap();
        assert_eq!(sys.tgp_cpu.pc, 0);
        assert!(sys.tgp_cpu.icount > 0); // abandoned idle budget, not CPU debt
        sys.copro_fifo_in.extend(0..16);
        sys.run_slice(1).unwrap();
        assert_eq!(sys.tgp_cpu.pc, 2);
        assert_eq!(sys.copro_fifo_in.len(), 14);
        assert_eq!(sys.tgp_cpu.icount, 0);
    }

    #[test]
    fn fifo_empty_in_retries_without_committing_zero_and_resumes_on_data() {
        for (opcode, expected) in [
            (0x20, 0xaabb_cc78),
            (0x22, 0xaabb_5678),
            (0x24, 0x1234_5678),
        ] {
            let mut roms = empty_roms();
            // IN.B/H/W absolute 0xd80000,R0, followed by HALT.
            roms.maincpu = vec![opcode, 0x20, 0xf3, 0, 0, 0xd8, 0, 0];
            let mut sys = Model1System::new(&roms).unwrap();
            sys.main_cpu.reg[v60::cpu::PC] = 0;
            sys.main_cpu.reg[0] = 0xaabb_ccdd;
            sys.run_slice(64).unwrap();
            assert!(sys.v60_fifo_waiting);
            assert_eq!(sys.main_cpu.pc(), 0);
            assert_eq!(sys.main_cpu.reg[0], 0xaabb_ccdd);
            assert!(sys.fifo_events.is_empty()); // no fictitious transfer
            let attempts = sys.main_cpu.op_count[opcode as usize];
            sys.run_slice(256).unwrap();
            assert_eq!(sys.main_cpu.op_count[opcode as usize], attempts);
            Mb86233Bus::write_data(&mut sys, 0x400, 0x1234_5678);
            sys.run_slice(8).unwrap();
            assert!(!sys.v60_fifo_waiting);
            assert_eq!(sys.main_cpu.pc(), 7);
            assert_eq!(sys.main_cpu.reg[0], expected);
            assert!(sys.copro_fifo_out.is_empty());
            assert_eq!(sys.fifo_events.iter().filter(|e| e.0 == 'R').count(), 1);
        }
    }

    #[test]
    fn fifo_tgp_empty_read_retries_before_registers_or_address_increment() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.tgp_program = vec![0x100]; // LAB [B0+X0++],io[0]
        let mut tgp = Mb86233::new();
        tgp.b0 = 0x100;
        tgp.i0 = 1;
        tgp.a = 0xfeed;
        tgp.b = 0xbeef;
        for _ in 0..3 {
            tgp.execute(&mut sys, 64);
            assert_eq!(tgp.pc, 0);
            assert_eq!((tgp.a, tgp.b, tgp.x0), (0xfeed, 0xbeef, 0));
        }
        assert!(sys.fifo_events.is_empty());
        sys.copro_fifo_in.push_back(0x1234_5678);
        tgp.execute(&mut sys, 1);
        assert_eq!(tgp.pc, 1);
        assert_eq!((tgp.a, tgp.x0), (0x1234_5678, 1));
        assert!(sys.copro_fifo_in.is_empty());
    }

    #[test]
    fn fifo_producers_finish_overflow_instruction_then_stop_and_resume() {
        let mut roms = empty_roms();
        // Repeated OUT.W R0,absolute 0xd80000.
        roms.maincpu = [0x25, 0x00, 0xf3, 0, 0, 0xd8, 0].repeat(20);
        let mut sys = Model1System::new(&roms).unwrap();
        let mut cpu = V60::new();
        cpu.reg[v60::cpu::PC] = 0;
        cpu.reg[0] = 0x1234_5678;
        sys.v60_access_active = true;
        cpu.run(&mut sys, 1000);
        assert_eq!(cpu.pc(), 17 * 7);
        assert_eq!(sys.copro_fifo_in.len(), 17);
        assert_eq!(Mb86233Bus::read_data(&mut sys, 0x100), 0x1234_5678);
        cpu.run(&mut sys, 8);
        assert_eq!(cpu.pc(), 18 * 7);
        assert_eq!(sys.copro_fifo_in.len(), 17);

        // MOV A,[B1+X1], same overflow boundary for the opposite producer.
        sys.tgp_program = vec![(7 << 26) | (7 << 18) | (0x10 << 9) | 0x180; 20];
        let mut tgp = Mb86233::new();
        tgp.b1 = 0x400;
        tgp.i1 = 0;
        tgp.a = 0xabcd_1234;
        tgp.execute(&mut sys, 100);
        assert_eq!(tgp.pc, 17);
        assert_eq!(sys.copro_fifo_out.len(), 17);
        assert_eq!(sys.read_u32(0xd80000), 0xabcd_1234);
        tgp.execute(&mut sys, 1);
        assert_eq!(tgp.pc, 18);
        assert_eq!(sys.copro_fifo_out.len(), 17);
    }

    #[test]
    fn fifo_overflow_preserves_order_and_unblocks_at_sixteen() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        for word in 0..17 {
            sys.write_u32(0xd80000, 0x1234_0000 | word);
            assert_eq!(Bus::halt_requested(&sys), word == 16);
            Mb86233Bus::write_data(&mut sys, 0x400, 0xabcd_0000 | word);
            assert_eq!(Mb86233Bus::halt_requested(&sys), word == 16);
        }
        sys.v60_access_active = true;
        for word in 0..17 {
            assert_eq!(Mb86233Bus::read_data(&mut sys, 0x100), 0x1234_0000 | word);
            assert!(!Bus::halt_requested(&sys));
            assert_eq!(sys.read_u32(0xd80000), 0xabcd_0000 | word);
            assert!(!Mb86233Bus::halt_requested(&sys));
        }
        assert_eq!(Mb86233Bus::read_data(&mut sys, 0x100), 0);
        assert!(Mb86233Bus::take_stall(&mut sys));
        assert!(!Mb86233Bus::take_stall(&mut sys));
    }

    #[test]
    fn fifo_halfword_latches_and_debugger_peek_do_not_advance_tgp() {
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.write_u16(0xd80000, 0x5678);
        assert!(sys.copro_fifo_in.is_empty());
        sys.write_u16(0xd80002, 0x1234);
        assert_eq!(sys.copro_fifo_in.pop_front(), Some(0x1234_5678));
        assert_eq!(sys.read_u32(0xd80000), 0);
        assert!(!sys.v60_fifo_waiting);
        assert_eq!(sys.tgp_cpu.pc, 0);
        sys.copro_fifo_out.push_back(0xdead_beef);
        assert_eq!(sys.read_u32(0xd80000), 0xdead_beef);
        assert_eq!(sys.copro_fifo_out.len(), 1);
        sys.v60_access_active = true;
        assert_eq!(sys.read_u16(0xd80000), 0xbeef);
        assert_eq!(sys.read_u16(0xd80002), 0xdead);
        assert_eq!(sys.read_u16(0xd80002), 0xdead);
        assert!(sys.copro_fifo_out.is_empty());
    }

    #[test]
    fn all_dpram_boards_charge_only_cpu_low_byte_reads() {
        let mut roms = empty_roms();
        for kind in [
            crate::model1board::Kind::Original,
            crate::model1board::Kind::WingWar,
            crate::model1board::Kind::WingWarR360,
        ] {
            roms.ioboard_kind = kind;
            roms.iocpu = vec![0; 0x10000];
            let mut sys = Model1System::new(&roms).unwrap();
            sys.read_u32(0xc00000);
            assert_eq!(sys.take_wait_cycles(), 0); // debugger/host inspection
            sys.v60_access_active = true;
            sys.write_u8(0xc00042, 0x5a);
            sys.write_u16(0xc00044, 0xabcd);
            sys.write_u32(0xc00046, 0x12345678);
            assert_eq!(sys.take_wait_cycles(), 0); // writes have no extra wait
            assert_eq!(sys.read_u8(0xc00042), 0x5a);
            assert_eq!(sys.take_wait_cycles(), 1);
            assert_eq!(sys.read_u8(0xc00043), 0xff);
            assert_eq!(sys.take_wait_cycles(), 0);
            sys.read_u16(0xc00042);
            assert_eq!(sys.take_wait_cycles(), 1);
            sys.read_u32(0xc00042);
            assert_eq!(sys.take_wait_cycles(), 2);
            sys.read_u32(0xc00043); // unaligned: still two connected lanes
            assert_eq!(sys.take_wait_cycles(), 2);
            sys.read_u32(0xc00ffe); // only one connected lane before map end
            assert_eq!(sys.take_wait_cycles(), 1);
            sys.read_u8(0xc01000);
            assert_eq!(sys.take_wait_cycles(), 0);
            sys.v60_access_active = false;
            sys.read_u32(0xc00042);
            assert_eq!(sys.take_wait_cycles(), 0);
        }
    }

    #[test]
    fn original_dpram_wait_reaches_v60_budget_and_carries_instruction_debt() {
        // Fetch a NOP from the low DPRAM lane: 8 base clocks + 1 bus wait.
        // Running an 8-clock slice must not lose that final clock.
        let mut sys = Model1System::new(&empty_roms()).unwrap();
        sys.write_u8(0xc00000, 0xcd);
        let mut cpu = V60::new();
        cpu.reg[v60::cpu::PC] = 0xc00000;
        sys.v60_access_active = true;
        cpu.run(&mut sys, 8);
        assert_eq!(cpu.pc(), 0xc00001);
        assert_eq!(cpu.icount, -1);
        assert_eq!(sys.take_wait_cycles(), 0); // drained by CPU, not twice
        cpu.run(&mut sys, 1);
        assert_eq!(cpu.pc(), 0xc00001); // pays debt without another fetch
        assert_eq!(cpu.icount, 0);
    }
}

/// Little-endian u16 at u16-index `idx` in a byte-addressed RAM.
fn le16(mem: &[u8], idx: usize) -> u16 {
    let b = idx * 2;
    u16::from_le_bytes([
        mem.get(b).copied().unwrap_or(0),
        mem.get(b + 1).copied().unwrap_or(0),
    ])
}

/// Little-endian u32 at u32-index `idx` in a byte-addressed RAM.
fn le32(mem: &[u8], idx: usize) -> u32 {
    let b = idx * 4;
    u32::from_le_bytes([
        mem.get(b).copied().unwrap_or(0),
        mem.get(b + 1).copied().unwrap_or(0),
        mem.get(b + 2).copied().unwrap_or(0),
        mem.get(b + 3).copied().unwrap_or(0),
    ])
}

impl Mb86233Bus for Model1System {
    fn read_program(&mut self, address: u32) -> u32 {
        self.tgp_program.get(address as usize).copied().unwrap_or(0)
    }

    fn read_data(&mut self, address: u32) -> u32 {
        if address == 0x100 {
            return match self.copro_fifo_in.pop_front() {
                Some(value) => {
                    self.fifo_note('r', value);
                    log::trace!(target: "fifo",
                        "[fifo] TGP {:04X} pop  in={:08X} (in left {})",
                        self.tgp_cpu.pc, value, self.copro_fifo_in.len()
                    );
                    value
                }
                None => {
                    self.copro_stall = true;
                    0
                }
            };
        }
        self.tgp_data.get(address as usize).copied().unwrap_or(0)
    }

    fn write_data(&mut self, address: u32, value: u32) {
        if address == 0x400 {
            self.fifo_note('w', value);
            log::trace!(target: "fifo",
                "[fifo] TGP {:04X} push out={:08X} (out {})",
                self.tgp_cpu.pc, value, self.copro_fifo_out.len()
            );
            self.copro_fifo_out.push_back(value);
        } else if let Some(dst) = self.tgp_data.get_mut(address as usize) {
            *dst = value;
        }
    }

    fn read_io(&mut self, address: u32) -> u32 {
        self.copro_io_read(address)
    }

    fn write_io(&mut self, address: u32, value: u32) {
        self.copro_io_write(address, value);
    }

    /// The TGP fetches host commands through register-file port 1, the same
    /// wiring as Model 2: an empty FIFO stalls the instruction so it retries
    /// until the V60 supplies a word, rather than running on with a bogus 0.
    fn read_rf(&mut self, address: u32) -> u32 {
        match address {
            1 => match self.copro_fifo_in.pop_front() {
                Some(v) => {
                    self.fifo_note('r', v);
                    log::trace!(target: "fifo",
                        "[fifo] TGP {:04X} pop  in={:08X} (in left {}) [rf1]",
                        self.tgp_cpu.pc, v, self.copro_fifo_in.len()
                    );
                    v
                }
                None => {
                    self.copro_stall = true;
                    0
                }
            },
            _ => 0,
        }
    }

    /// Port 0 is the LED/busy latch (ignored); port 2 pushes a result word back
    /// to the V60 through the output FIFO.
    fn write_rf(&mut self, address: u32, value: u32) {
        if address == 2 {
            self.fifo_note('w', value);
            log::trace!(target: "fifo",
                "[fifo] TGP {:04X} push out={:08X} (out {}) [rf2]",
                self.tgp_cpu.pc, value, self.copro_fifo_out.len()
            );
            self.copro_fifo_out.push_back(value);
        }
    }

    fn take_stall(&mut self) -> bool {
        std::mem::take(&mut self.copro_stall)
    }

    fn halt_requested(&self) -> bool {
        // Match the FIFO semantics: accept the overflow word, then halt
        // before the TGP fetches and executes another instruction.
        self.copro_fifo_out.len() > COPRO_FIFO_DEPTH
    }
}
