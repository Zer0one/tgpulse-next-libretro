//! Small motherboard boundary for the two Model 1 I/O-board revisions.
//! Cabinet signals already translated by the frontend remain unchanged here.
use std::{cell::Ref, ops::Deref};

use crate::{config::Inputs, eeprom93c46::Eeprom93c46, model1io, model1io2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Original,
    WingWar,
    WingWarR360,
}

impl Kind {
    pub fn for_set(name: &str) -> Self {
        match name {
            "wingwar" | "wingwaru" | "wingwarj" => Self::WingWar,
            "wingwar360" => Self::WingWarR360,
            // NetMerc remains a separate, unvalidated path.
            _ => Self::Original,
        }
    }
    pub fn clock_hz(self) -> u32 {
        match self {
            Self::Original => model1io::Z80_HZ,
            Self::WingWar | Self::WingWarR360 => model1io2::CPU_HZ,
        }
    }
}

enum Device {
    Original(model1io::IoBoard),
    Advanced(Box<model1io2::IoBoard>),
}

/// A view into either the original board or the advanced CPU's interior bus.
/// No EEPROM/DPRAM copies are made for motherboard reads.
pub enum BoardRead<'a, T: ?Sized> {
    Direct(&'a T),
    Guard(Ref<'a, T>),
}
impl<T: ?Sized> Deref for BoardRead<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            Self::Direct(value) => value,
            Self::Guard(value) => value,
        }
    }
}

pub struct IoBoard {
    device: Device,
    /// Fractional board clocks, expressed in V60-clock denominator units.
    /// Keep this separate from each CPU's instruction-overshoot debt.
    clock_remainder: u64,
}

/// Board state at the motherboard boundary, including the fractional V60 clock
/// conversion. Restore with the same firmware/cabinet. No host resources or ROM;
/// this is not a versioned format or a complete Model 1 machine save state.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardState {
    board: DeviceState,
    clock_remainder: u64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum DeviceState {
    Original(model1io::BoardState),
    Advanced(model1io2::BoardState),
}

impl IoBoard {
    pub fn new(
        kind: Kind,
        firmware: &[u8],
        eeprom: Eeprom93c46,
    ) -> Result<Self, model1io2::BusError> {
        let device = match kind {
            Kind::Original => {
                let mut board = model1io::IoBoard::new(firmware, eeprom);
                // Preserve the existing board-1 boot behavior. Board 2 must
                // produce this status itself after its real EEPROM transfer.
                board.dpram_mut()[0x21] = 0x40;
                Device::Original(board)
            }
            Kind::WingWar => Device::Advanced(Box::new(model1io2::IoBoard::new(firmware, eeprom)?)),
            Kind::WingWarR360 => {
                Device::Advanced(Box::new(model1io2::IoBoard::new_r360(firmware, eeprom)?))
            }
        };
        Ok(Self {
            device,
            clock_remainder: 0,
        })
    }

    pub fn kind(&self) -> Kind {
        match &self.device {
            Device::Original(_) => Kind::Original,
            Device::Advanced(board) => {
                if board.is_r360() {
                    Kind::WingWarR360
                } else {
                    Kind::WingWar
                }
            }
        }
    }
    pub fn fault(&self) -> Option<model1io2::BusError> {
        match &self.device {
            Device::Original(_) => None,
            Device::Advanced(board) => board.fault(),
        }
    }
    pub fn run_main_cycles(
        &mut self,
        cycles: u32,
        inputs: Inputs,
    ) -> Result<(), model1io2::BusError> {
        if let Some(error) = self.fault() {
            return Err(error);
        }
        let scaled = self.clock_remainder + u64::from(cycles) * u64::from(self.kind().clock_hz());
        let clocks = scaled / u64::from(crate::model1::V60_HZ);
        self.clock_remainder = scaled % u64::from(crate::model1::V60_HZ);
        match &mut self.device {
            Device::Original(board) => {
                board.set_inputs(inputs);
                board.run(clocks as i64);
            }
            Device::Advanced(board) => {
                board.set_inputs(model1io2::Inputs {
                    digital: [inputs.in0, inputs.in1, inputs.in2],
                    analog: inputs.analog,
                    dips: inputs.dsw,
                    ..model1io2::Inputs::default()
                });
                // CN7/CN8 have no peer on this base cabinet. The standalone
                // board API retains its timestamped serial outputs for future
                // device integration; do not synthesize a loopback here.
                board.run(clocks as u32, |_| {})?;
            }
        }
        Ok(())
    }
    pub fn drive_cmd(&self) -> u8 {
        match &self.device {
            Device::Original(board) => board.drive_cmd(),
            Device::Advanced(board) => board.bus().outputs().drive,
        }
    }
    pub fn dpram(&self) -> BoardRead<'_, [u8]> {
        match &self.device {
            Device::Original(board) => BoardRead::Direct(board.dpram()),
            Device::Advanced(board) => BoardRead::Guard(board.dpram()),
        }
    }
    pub fn dpram_mut(&mut self) -> &mut [u8] {
        match &mut self.device {
            Device::Original(board) => board.dpram_mut(),
            Device::Advanced(board) => board.dpram_mut(),
        }
    }
    pub fn eeprom(&self) -> BoardRead<'_, Eeprom93c46> {
        match &self.device {
            Device::Original(board) => BoardRead::Direct(board.eeprom()),
            Device::Advanced(board) => BoardRead::Guard(board.eeprom()),
        }
    }
    pub fn eeprom_mut(&mut self) -> &mut Eeprom93c46 {
        match &mut self.device {
            Device::Original(board) => board.eeprom_mut(),
            Device::Advanced(board) => board.eeprom_mut(),
        }
    }
    pub fn snapshot(&self) -> BoardState {
        BoardState {
            board: match &self.device {
                Device::Original(board) => DeviceState::Original(board.snapshot()),
                Device::Advanced(board) => DeviceState::Advanced(board.snapshot()),
            },
            clock_remainder: self.clock_remainder,
        }
    }
    pub(crate) fn validate_state(&self, state: &BoardState) -> Result<(), model1io2::BusError> {
        if state.clock_remainder >= u64::from(crate::model1::V60_HZ) {
            return Err(model1io2::BusError::InvalidSnapshot);
        }
        match (&self.device, &state.board) {
            (Device::Original(board), DeviceState::Original(saved)) => board
                .validate_state(saved)
                .map_err(|_| model1io2::BusError::InvalidSnapshot),
            (Device::Advanced(board), DeviceState::Advanced(saved)) => board.validate_state(saved),
            _ => Err(model1io2::BusError::InvalidSnapshot),
        }
    }
    pub fn restore(&mut self, state: &BoardState) -> Result<(), model1io2::BusError> {
        self.validate_state(state)?;
        // Both board restores validate before mutation, including the advanced
        // board's R360 variant. Never reconstruct/reset a board to restore it.
        match (&mut self.device, &state.board) {
            (Device::Original(board), DeviceState::Original(saved)) => board
                .restore(saved)
                .map_err(|_| model1io2::BusError::InvalidSnapshot)?,
            (Device::Advanced(board), DeviceState::Advanced(saved)) => board.restore(saved)?,
            _ => return Err(model1io2::BusError::InvalidSnapshot),
        }
        self.clock_remainder = state.clock_remainder;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn advanced(program: &[u8]) -> IoBoard {
        let mut rom = vec![0; 0x10000];
        rom[..program.len()].copy_from_slice(program);
        IoBoard::new(Kind::WingWar, &rom, Eeprom93c46::new()).unwrap()
    }
    #[test]
    fn wingwar_variants_select_their_advanced_board_wiring() {
        for set in ["wingwar", "wingwaru", "wingwarj"] {
            assert_eq!(Kind::for_set(set), Kind::WingWar);
        }
        assert_eq!(Kind::for_set("wingwar360"), Kind::WingWarR360);
        for set in ["vr", "vf", "swa", "netmerc", "unknown"] {
            assert_eq!(Kind::for_set(set), Kind::Original);
        }
        assert_eq!(Kind::Original.clock_hz(), 4_000_000);
        assert_eq!(Kind::WingWar.clock_hz(), 9_830_400);
    }
    #[test]
    fn advanced_status_is_not_seeded_and_firmware_size_is_checked() {
        assert_eq!(advanced(&[0x76]).dpram()[0x21], 0);
        assert_eq!(
            IoBoard::new(Kind::Original, &[], Eeprom93c46::new())
                .unwrap()
                .dpram()[0x21],
            0x40
        );
        assert!(matches!(
            IoBoard::new(Kind::WingWar, &[], Eeprom93c46::new()),
            Err(model1io2::BusError::FirmwareSize(0))
        ));
    }
    #[test]
    fn fractional_clocks_and_restore_preserve_equal_continuation() {
        for kind in [Kind::Original, Kind::WingWar, Kind::WingWarR360] {
            let ram_hi = if kind == Kind::Original { 0x40 } else { 0xe0 };
            let program = [0x21, 0, ram_hi, 0x34, 0xc3, 3, 0]; // INC (RAM), JP
            let mut firmware = vec![0; 0x10000];
            firmware[..program.len()].copy_from_slice(&program);
            let make = || IoBoard::new(kind, &firmware, Eeprom93c46::new()).unwrap();
            let mut whole = make();
            let mut sliced = make();
            // Constructor boot seeds are not reapplied on restore.
            whole.dpram_mut()[0x21] = 0x92;
            sliced.dpram_mut()[0x21] = 0x92;
            whole.run_main_cycles(20_001, Inputs::default()).unwrap();
            for _ in 0..20_001 {
                sliced.run_main_cycles(1, Inputs::default()).unwrap();
            }
            let encode = |b: &IoBoard| bincode::serialize(&b.snapshot()).unwrap();
            assert_eq!(encode(&whole), encode(&sliced));
            let state: BoardState = bincode::deserialize(&encode(&whole)).unwrap();
            assert_ne!(state.clock_remainder, 0);
            let mut restored = make();
            restored.run_main_cycles(7, Inputs::default()).unwrap();
            restored.restore(&state).unwrap();
            assert_eq!(encode(&whole), encode(&restored));
            assert_eq!(restored.dpram()[0x21], 0x92);
            whole.run_main_cycles(31_337, Inputs::default()).unwrap();
            for _ in 0..31_337 {
                restored.run_main_cycles(1, Inputs::default()).unwrap();
            }
            assert_eq!(encode(&whole), encode(&restored));
            let before = encode(&restored);
            let mut invalid = state;
            invalid.clock_remainder = u64::from(crate::model1::V60_HZ);
            assert!(restored.restore(&invalid).is_err());
            assert_eq!(encode(&restored), before);
        }
    }
    #[test]
    fn restore_rejects_other_board_revisions_without_mutation() {
        let firmware = vec![0x76; 0x10000];
        for kind in [Kind::Original, Kind::WingWar, Kind::WingWarR360] {
            let mut board = IoBoard::new(kind, &firmware, Eeprom93c46::new()).unwrap();
            board.run_main_cycles(1337, Inputs::default()).unwrap();
            let before = bincode::serialize(&board.snapshot()).unwrap();
            for other in [Kind::Original, Kind::WingWar, Kind::WingWarR360] {
                if kind == other {
                    continue;
                }
                let wrong = IoBoard::new(other, &firmware, Eeprom93c46::new()).unwrap();
                assert_eq!(
                    board.restore(&wrong.snapshot()),
                    Err(model1io2::BusError::InvalidSnapshot)
                );
                assert_eq!(bincode::serialize(&board.snapshot()).unwrap(), before);
            }
        }
    }
    #[test]
    fn panel_channels_reach_board_without_new_bindings() {
        let mut program = vec![0x3e, 7, 0x32, 8, 0x80];
        for port in 0..3 {
            program.extend_from_slice(&[0x3a, port, 0x80, 0x32, port, 0xe0]);
        }
        program.extend_from_slice(&[0x3a, 0x80, 0x80, 0x32, 3, 0xe0, 0x32, 0, 0x82]);
        for bit in 0..8 {
            program.extend_from_slice(&[0x3a, 0, 0x82, 0x32, 4 + bit, 0xe0]);
        }
        program.push(0x76);
        let mut board = advanced(&program);
        let mut inputs = Inputs::default();
        inputs.in0 = 0x12;
        inputs.in1 = 0x34;
        inputs.in2 = 0x56;
        inputs.dsw[0] = 0x78;
        inputs.analog[0] = 0xa6;
        board.run_main_cycles(4000, inputs).unwrap();
        let Device::Advanced(board) = &board.device else {
            unreachable!()
        };
        for (offset, value) in [0x12, 0x34, 0x56, 0x78].into_iter().enumerate() {
            assert_eq!(board.bus().read_memory(0xe000 + offset as u16), Ok(value));
        }
        let sample = (0..8).fold(0, |a, i| {
            (a << 1) | board.bus().read_memory(0xe004 + i).unwrap()
        });
        assert_eq!(sample, 0xa6);
    }
    #[test]
    fn board_fault_stops_later_clock_and_bus_effects() {
        let mut board = advanced(&[0x3e, 0x8f, 0xd3, 0x1d]); // unsupported PIO mode 2
        let error = board.run_main_cycles(1000, Inputs::default()).unwrap_err();
        let before = bincode::serialize(&board.snapshot()).unwrap();
        assert_eq!(board.run_main_cycles(1000, Inputs::default()), Err(error));
        assert_eq!(bincode::serialize(&board.snapshot()).unwrap(), before);
    }
}
