// CPU state lives in the z80 module so no raw-memory or layout tricks are needed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CpuState {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub af_alt: u16,
    pub bc_alt: u16,
    pub de_alt: u16,
    pub hl_alt: u16,
    pub pc: u16,
    pub sp: u16,
    pub ix: u16,
    pub iy: u16,
    pub mem_ptr: u16,
    pub i: u8,
    pub r: u8,
    pub iff_delay: u8,
    pub interrupt_mode: u8,
    pub irq_data: u8,
    pub irq_pending: u8,
    pub nmi_pending: u8,
    pub halted: bool,
    pub iff1: bool,
    pub iff2: bool,
}

impl CpuState {
    pub fn is_valid(&self) -> bool {
        self.interrupt_mode <= 2
            && self.iff_delay <= 1
            && self.irq_pending & !3 == 0
            && self.nmi_pending & !3 == 0
    }
}

impl<T: Z80_io> Z80<T> {
    pub fn snapshot(&self) -> CpuState {
        // The upstream CPU stores register pairs in unions. Read each declared
        // pair explicitly; never serialize padding, pointers or the I/O object.
        unsafe {
            CpuState {
                af: self.c2rust_unnamed.af,
                bc: self.c2rust_unnamed_0.bc,
                de: self.c2rust_unnamed_1.de,
                hl: self.c2rust_unnamed_2.hl,
                af_alt: self.c2rust_unnamed_3.a_f_,
                bc_alt: self.c2rust_unnamed_4.b_c_,
                de_alt: self.c2rust_unnamed_5.d_e_,
                hl_alt: self.c2rust_unnamed_6.h_l_,
                pc: self.pc,
                sp: self.sp,
                ix: self.ix,
                iy: self.iy,
                mem_ptr: self.mem_ptr,
                i: self.i,
                r: self.r,
                iff_delay: self.iff_delay,
                interrupt_mode: self.interrupt_mode,
                irq_data: self.irq_data,
                irq_pending: self.irq_pending,
                nmi_pending: self.nmi_pending,
                halted: self.halted,
                iff1: self.iff1,
                iff2: self.iff2,
            }
        }
    }

    /// Returns false without changing the CPU if the snapshot is malformed.
    /// Bus restoration is deliberately the owner's responsibility.
    pub fn restore(&mut self, state: &CpuState) -> bool {
        if !state.is_valid() {
            return false;
        }
        self.c2rust_unnamed.af = state.af;
        self.c2rust_unnamed_0.bc = state.bc;
        self.c2rust_unnamed_1.de = state.de;
        self.c2rust_unnamed_2.hl = state.hl;
        self.c2rust_unnamed_3.a_f_ = state.af_alt;
        self.c2rust_unnamed_4.b_c_ = state.bc_alt;
        self.c2rust_unnamed_5.d_e_ = state.de_alt;
        self.c2rust_unnamed_6.h_l_ = state.hl_alt;
        self.pc = state.pc;
        self.sp = state.sp;
        self.ix = state.ix;
        self.iy = state.iy;
        self.mem_ptr = state.mem_ptr;
        self.i = state.i;
        self.r = state.r;
        self.iff_delay = state.iff_delay;
        self.interrupt_mode = state.interrupt_mode;
        self.irq_data = state.irq_data;
        self.irq_pending = state.irq_pending;
        self.nmi_pending = state.nmi_pending;
        self.halted = state.halted;
        self.iff1 = state.iff1;
        self.iff2 = state.iff2;
        true
    }
}
