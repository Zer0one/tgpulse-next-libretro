// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Instruction-boundary CPU state. No memory, CPU implementation, or host data.
//! Restore into the same CPU model; the containing machine owns format/version
//! and resource identity. Do not reconstruct via exception(), which wakes STOP.
use crate::{exception::Exception, CpuDetails, M68000};
use std::{collections::BTreeSet, num::Wrapping};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub data: [u32; 8],
    pub address: [u32; 7],
    pub usp: u32,
    pub ssp: u32,
    pub status: u16,
    pub pc: u32,
    pub current_opcode: u16,
    pub stopped: bool,
    pub pending: Vec<u8>,
}

impl State {
    fn exceptions(&self) -> BTreeSet<Exception> {
        // Use the same insertion semantics as exception(). Bulk collection
        // may use derived Eq while this upstream type orders by priority only.
        let mut set = BTreeSet::new();
        for vector in &self.pending {
            set.insert(Exception::from(*vector));
        }
        set
    }
    pub fn valid(&self) -> bool {
        // Match the existing core's priority-ordered set exactly, including its
        // one-entry-per-priority semantics. Do not silently deduplicate a save.
        let exceptions = self.exceptions();
        self.pending.len() <= 256
            && self.status & !0xa71f == 0
            && exceptions
                .iter()
                .map(|e| e.vector)
                .eq(self.pending.iter().copied())
    }
}

impl<CPU: CpuDetails> M68000<CPU> {
    pub fn snapshot(&self) -> State {
        State {
            data: self.regs.d.map(|r| r.0),
            address: self.regs.a.map(|r| r.0),
            usp: self.regs.usp.0,
            ssp: self.regs.ssp.0,
            status: self.regs.sr.into(),
            pc: self.regs.pc.0,
            current_opcode: self.current_opcode,
            stopped: self.stop,
            pending: self.exceptions.iter().map(|e| e.vector).collect(),
        }
    }

    /// Validate before mutation; no bus access, reset, or exception delivery.
    pub fn restore(&mut self, state: &State) -> Result<(), &'static str> {
        if !state.valid() {
            return Err("invalid 68000 state");
        }
        self.regs.d = state.data.map(Wrapping);
        self.regs.a = state.address.map(Wrapping);
        self.regs.usp = Wrapping(state.usp);
        self.regs.ssp = Wrapping(state.ssp);
        self.regs.sr = state.status.into();
        self.regs.pc = Wrapping(state.pc);
        self.current_opcode = state.current_opcode;
        self.stop = state.stopped;
        self.exceptions = state.exceptions();
        Ok(())
    }
}
