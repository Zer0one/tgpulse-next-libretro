//! Synthetic programs only: exercise the production library (not its upstream
//! cfg(test) CP/M console shims). No ROM, filesystem, GUI or wall-clock access.
use z80::{CpuState, Z80_io, Z80};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Clocks(u32),
    Ack(u8),
    Reti,
    Out(u16, u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Bus {
    memory: Vec<u8>,
    events: Vec<Event>,
    clocks: u64,
    irq: bool,
    vector: u8,
    service: bool,
    scheduled_irq: Option<(u64, u8)>,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            memory: vec![0; 65536],
            events: vec![],
            clocks: 0,
            irq: false,
            vector: 0x20,
            service: false,
            scheduled_irq: None,
        }
    }
}

impl Z80_io for Bus {
    fn read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
    fn port_in(&self, _: u16) -> u8 {
        0xa5
    }
    fn port_out(&mut self, addr: u16, value: u8) {
        self.events.push(Event::Out(addr, value));
    }
    fn irq_pending(&self) -> bool {
        self.irq && !self.service
    }
    fn irq_acknowledge(&mut self, _: u8) -> u8 {
        self.events.push(Event::Ack(self.vector));
        self.irq = false;
        self.service = true;
        self.vector
    }
    fn reti(&mut self) {
        self.events.push(Event::Reti);
        self.service = false;
    }
    fn advance(&mut self, clocks: u32) {
        assert!(clocks > 0);
        self.clocks += u64::from(clocks);
        self.events.push(Event::Clocks(clocks));
        if let Some((at, vector)) = self.scheduled_irq {
            if self.clocks >= at {
                self.irq = true;
                self.vector = vector;
                self.scheduled_irq = None;
            }
        }
    }
}

fn cpu(program: &[u8]) -> Z80<Bus> {
    let mut bus = Bus::default();
    bus.memory[..program.len()].copy_from_slice(program);
    let mut cpu = Z80::new(bus);
    cpu.init();
    cpu.sp = 0xf000;
    cpu.i = 0x80;
    cpu.interrupt_mode = 2;
    cpu.io.memory[0x8020..0x8022].copy_from_slice(&0x1000u16.to_le_bytes());
    cpu
}

fn enable(cpu: &mut Z80<Bus>) {
    cpu.iff1 = true;
    cpu.iff2 = true;
}

fn stacked_pc(cpu: &Z80<Bus>) -> u16 {
    let sp = cpu.sp as usize;
    u16::from_le_bytes([cpu.io.memory[sp], cpu.io.memory[sp + 1]])
}

#[test]
fn disabled_irq_and_di_never_acknowledge() {
    let mut cpu = cpu(&[0, 0xf3, 0]); // NOP; DI; NOP
    cpu.io.irq = true;
    assert_eq!(cpu.step(), 4);
    enable(&mut cpu);
    assert_eq!(cpu.step(), 4);
    assert_eq!(cpu.step(), 4);
    assert_eq!(cpu.pc, 3);
    assert!(cpu.io.irq);
    assert!(!cpu.iff1 && !cpu.iff2);
    assert_eq!(cpu.io.events, vec![Event::Clocks(4); 3]);
}

#[test]
fn ei_defers_irq_until_following_instruction() {
    let mut cpu = cpu(&[0xfb, 0]); // EI; NOP
    cpu.io.irq = true;
    assert_eq!(cpu.step(), 4);
    assert_eq!(cpu.pc, 1);
    assert!(!cpu.io.service);
    assert_eq!(cpu.step(), 23); // NOP + IM2 acceptance
    assert_eq!(cpu.pc, 0x1000);
    assert_eq!(stacked_pc(&cpu), 2);
    assert!(!cpu.iff1 && !cpu.iff2);
    assert_eq!(
        cpu.io.events,
        vec![
            Event::Clocks(4),
            Event::Clocks(4),
            Event::Ack(0x20),
            Event::Clocks(19),
        ]
    );
}

#[test]
fn di_after_ei_cancels_acceptance() {
    let mut cpu = cpu(&[0xfb, 0xf3, 0]);
    cpu.io.irq = true;
    for _ in 0..3 {
        assert_eq!(cpu.step(), 4);
    }
    assert!(!cpu.io.service);
    assert_eq!(cpu.pc, 3);
}

#[test]
fn vector_is_sampled_at_acceptance_after_elapsed_instruction() {
    let mut cpu = cpu(&[0]);
    enable(&mut cpu);
    cpu.irq_data = 0x20; // stale vector must not win over the live source
    cpu.io.scheduled_irq = Some((4, 0x23)); // odd IM2 vector is legal
    cpu.io.memory[0x8023..0x8025].copy_from_slice(&0x1234u16.to_le_bytes());
    assert_eq!(cpu.step(), 23);
    assert_eq!(cpu.pc, 0x1234);
    assert_eq!(cpu.irq_data, 0x23);
    assert_eq!(stacked_pc(&cpu), 1);
    assert_eq!(
        cpu.io.events,
        vec![Event::Clocks(4), Event::Ack(0x23), Event::Clocks(19)]
    );
}

#[test]
fn im1_acknowledges_but_uses_fixed_vector() {
    let mut cpu = cpu(&[0]);
    cpu.interrupt_mode = 1;
    enable(&mut cpu);
    cpu.io.irq = true;
    assert_eq!(cpu.step(), 17);
    assert_eq!(cpu.pc, 0x38);
    assert!(cpu.io.service);
    assert_eq!(cpu.io.clocks, 17);
}

#[test]
fn nmi_precedes_irq_without_acknowledging_it() {
    let mut cpu = cpu(&[0]);
    enable(&mut cpu);
    cpu.io.irq = true;
    cpu.pulse_nmi();
    assert_eq!(cpu.step(), 15);
    assert_eq!(cpu.pc, 0x66);
    assert_eq!(stacked_pc(&cpu), 1);
    assert!(!cpu.iff1 && cpu.iff2);
    assert!(!cpu.io.service && cpu.io.irq);
    assert_eq!(cpu.nmi_pending, 0);
    assert_eq!(cpu.io.events, vec![Event::Clocks(4), Event::Clocks(11)]);
}

#[test]
fn ei_does_not_delay_nmi_and_retn_preserves_irq_request() {
    let mut cpu = cpu(&[0xfb, 0]);
    cpu.io.memory[0x66..0x68].copy_from_slice(&[0xed, 0x45]);
    cpu.io.irq = true;
    cpu.pulse_nmi();
    assert_eq!(cpu.step(), 15); // EI + NMI, no maskable acknowledgement
    assert_eq!(cpu.pc, 0x66);
    assert!(!cpu.iff1 && cpu.iff2);
    assert_eq!(stacked_pc(&cpu), 1);
    assert_eq!(cpu.io.events, vec![Event::Clocks(4), Event::Clocks(11)]);
    assert_eq!(cpu.step(), 33); // RETN restores IFF1; pending IM2 can now enter
    assert_eq!(cpu.pc, 0x1000);
    assert_eq!(stacked_pc(&cpu), 1);
    assert!(!cpu.io.events.contains(&Event::Reti));
}

#[test]
fn nmi_wakes_halt_even_when_maskable_interrupts_are_disabled() {
    let mut cpu = cpu(&[0x76]);
    assert_eq!(cpu.step(), 4);
    cpu.pulse_nmi();
    assert_eq!(cpu.step(), 15);
    assert!(!cpu.halted);
    assert_eq!(cpu.pc, 0x66);
    assert_eq!(stacked_pc(&cpu), 1);
    assert!(!cpu.io.service);
}

#[test]
fn halt_advances_time_and_exits_on_live_irq() {
    let mut cpu = cpu(&[0x76]);
    enable(&mut cpu);
    assert_eq!(cpu.step(), 4);
    assert!(cpu.halted);
    assert_eq!(cpu.step(), 4);
    assert_eq!(cpu.pc, 1);
    cpu.io.scheduled_irq = Some((12, 0x20));
    assert_eq!(cpu.step(), 23);
    assert!(!cpu.halted);
    assert_eq!(stacked_pc(&cpu), 1);
    assert_eq!(cpu.io.clocks, 31);
}

#[test]
fn canonical_reti_restores_iff_and_releases_service_once() {
    for iff2 in [false, true] {
        let mut cpu = cpu(&[0xed, 0x4d]);
        cpu.io.memory[0xf000..0xf002].copy_from_slice(&0x3456u16.to_le_bytes());
        cpu.iff1 = !iff2;
        cpu.iff2 = iff2;
        cpu.io.service = true;
        assert_eq!(cpu.step(), 14);
        assert_eq!(cpu.pc, 0x3456);
        assert_eq!(cpu.sp, 0xf002);
        assert_eq!(cpu.mem_ptr, 0x3456);
        assert_eq!(cpu.iff1, iff2);
        assert!(!cpu.io.service);
        assert_eq!(cpu.io.events, vec![Event::Reti, Event::Clocks(14)]);
    }
}

#[test]
fn retn_does_not_release_daisy_service() {
    let mut cpu = cpu(&[0xed, 0x45]);
    cpu.iff2 = true;
    cpu.io.service = true;
    assert_eq!(cpu.step(), 14);
    assert!(cpu.iff1 && cpu.io.service);
    assert_eq!(cpu.io.events, vec![Event::Clocks(14)]);
}

#[test]
fn reti_allows_next_pending_source_at_instruction_boundary() {
    let mut cpu = cpu(&[0xed, 0x4d]);
    cpu.iff2 = true;
    cpu.io.service = true;
    cpu.io.irq = true;
    cpu.io.memory[0xf000..0xf002].copy_from_slice(&0x3456u16.to_le_bytes());
    assert_eq!(cpu.step(), 33);
    assert_eq!(cpu.pc, 0x1000);
    assert_eq!(stacked_pc(&cpu), 0x3456);
    assert_eq!(
        cpu.io.events,
        vec![
            Event::Reti,
            Event::Clocks(14),
            Event::Ack(0x20),
            Event::Clocks(19),
        ]
    );
}

#[test]
fn typed_state_covers_all_registers_without_touching_bus() {
    let mut cpu = cpu(&[]);
    let state = CpuState {
        af: 0x1234,
        bc: 0x5678,
        de: 0x9abc,
        hl: 0xdef0,
        af_alt: 0x1020,
        bc_alt: 0x3040,
        de_alt: 0x5060,
        hl_alt: 0x7080,
        pc: 0x1111,
        sp: 0x2222,
        ix: 0x3333,
        iy: 0x4444,
        mem_ptr: 0x5555,
        i: 0x66,
        r: 0xf7,
        iff_delay: 1,
        interrupt_mode: 2,
        irq_data: 0x88,
        irq_pending: 3,
        nmi_pending: 3,
        halted: true,
        iff1: false,
        iff2: true,
    };
    let encoded = bincode::serialize(&state).unwrap();
    let decoded: CpuState = bincode::deserialize(&encoded).unwrap();
    let bus_before = cpu.io.clone();
    assert!(cpu.restore(&decoded));
    assert_eq!(cpu.snapshot(), state);
    assert_eq!(cpu.io, bus_before);
}

#[test]
fn invalid_state_is_rejected_atomically() {
    let mut cpu = cpu(&[]);
    let before = cpu.snapshot();
    for field in 0..4 {
        let mut invalid = before.clone();
        invalid.pc = 0x9876;
        match field {
            0 => invalid.interrupt_mode = 3,
            1 => invalid.iff_delay = 2,
            2 => invalid.irq_pending = 4,
            _ => invalid.nmi_pending = 4,
        }
        assert!(!cpu.restore(&invalid));
        assert_eq!(cpu.snapshot(), before);
        assert!(cpu.io.events.is_empty());
    }
}

#[test]
fn restored_primary_and_alternate_registers_are_used_by_instructions() {
    let mut cpu = cpu(&[0x08, 0xd9, 0xf5, 0xc5, 0xd5, 0xe5]); // EX AF; EXX; PUSH pairs
    let mut state = cpu.snapshot();
    state.af = 0x1234;
    state.bc = 0x5678;
    state.de = 0x9abc;
    state.hl = 0xdef0;
    state.af_alt = 0x1020;
    state.bc_alt = 0x3040;
    state.de_alt = 0x5060;
    state.hl_alt = 0x7080;
    assert!(cpu.restore(&state));
    cpu.step();
    cpu.step();
    let swapped = cpu.snapshot();
    assert_eq!(
        (swapped.af, swapped.bc, swapped.de, swapped.hl),
        (0x1020, 0x3040, 0x5060, 0x7080)
    );
    assert_eq!(
        (
            swapped.af_alt,
            swapped.bc_alt,
            swapped.de_alt,
            swapped.hl_alt
        ),
        (0x1234, 0x5678, 0x9abc, 0xdef0)
    );
    for word in [0x1020, 0x3040, 0x5060, 0x7080] {
        assert_eq!(cpu.step(), 11);
        assert_eq!(stacked_pc(&cpu), word);
    }
}

#[test]
fn restored_cpu_and_bus_continue_identically_during_transfer_irq_and_halt() {
    // LDIR performs one byte per step, then EI/HALT with an IM2 interrupt.
    // ISR: IN A,(42); OUT (43),A; EI; RETI. No firmware needed.
    let mut initial = cpu(&[0xed, 0xb0, 0xfb, 0x76, 0x76]);
    let mut state = initial.snapshot();
    state.hl = 0x2000;
    state.de = 0x3000;
    state.bc = 4;
    assert!(initial.restore(&state));
    initial.io.memory[0x2000..0x2004].copy_from_slice(&[1, 2, 3, 4]);
    initial.io.memory[0x1000..0x1007].copy_from_slice(&[0xdb, 0x42, 0xd3, 0x43, 0xfb, 0xed, 0x4d]);
    initial.io.scheduled_irq = Some((90, 0x20));
    for cut in 0..16 {
        let mut original = cpu(&[]);
        original.io = initial.io.clone();
        assert!(original.restore(&initial.snapshot()));
        for _ in 0..cut {
            original.step();
        }
        let encoded = bincode::serialize(&original.snapshot()).unwrap();
        let mut restored = cpu(&[]);
        restored.io = original.io.clone(); // bus state is owned/restored separately
        assert!(restored.restore(&bincode::deserialize(&encoded).unwrap()));
        for _ in 0..24 {
            assert_eq!(original.step(), restored.step(), "cut={cut}");
            assert_eq!(original.snapshot(), restored.snapshot(), "cut={cut}");
            assert_eq!(original.io, restored.io, "cut={cut}");
        }
        assert_eq!(&original.io.memory[0x3000..0x3004], &[1, 2, 3, 4]);
        assert!(original.io.events.contains(&Event::Ack(0x20)));
        assert!(original.io.events.contains(&Event::Reti));
        assert!(original.halted);
    }
}

#[test]
fn default_hooks_preserve_fixed_vector_api() {
    struct PlainBus(Vec<u8>);
    impl Z80_io for PlainBus {
        fn read_byte(&self, addr: u16) -> u8 {
            self.0[addr as usize]
        }
        fn write_byte(&mut self, addr: u16, value: u8) {
            self.0[addr as usize] = value;
        }
    }
    let mut cpu = Z80::new(PlainBus(vec![0; 65536]));
    cpu.init();
    cpu.interrupt_mode = 2;
    cpu.i = 0x80;
    cpu.iff1 = true;
    cpu.iff2 = true;
    cpu.io.0[0x8042..0x8044].copy_from_slice(&0x5678u16.to_le_bytes());
    cpu.pulse_irq(0x42);
    assert_eq!(cpu.step(), 23);
    assert_eq!(cpu.pc, 0x5678);
    assert_eq!(cpu.irq_pending, 0);
}

#[test]
fn init_resets_cpu_state_without_resetting_host_resources() {
    let mut cpu = cpu(&[0]);
    enable(&mut cpu);
    cpu.pulse_nmi();
    cpu.assert_irq(0xaa);
    cpu.step();
    let bus_before = cpu.io.clone();
    cpu.init();
    let mut fresh = Z80::new(Bus::default());
    fresh.init();
    assert_eq!(cpu.snapshot(), fresh.snapshot());
    assert_eq!(cpu.io, bus_before);
}
