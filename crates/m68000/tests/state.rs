// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use m68000::{cpu_details::Mc68000, exception::Vector, M68000};

#[test]
fn pending_irq_and_stop_survive_serialization_without_delivery() {
    let mut cpu = M68000::<Mc68000>::new_no_reset();
    cpu.regs.pc.0 = 0x100;
    cpu.regs.ssp.0 = 0x380;
    cpu.regs.usp.0 = 0x300;
    cpu.exception(Vector::Level2Interrupt.into());
    cpu.stop = true;
    let state = cpu.snapshot();
    assert_eq!(state.pending, vec![26]);
    let decoded = bincode::deserialize(&bincode::serialize(&state).unwrap()).unwrap();
    let mut restored = M68000::<Mc68000>::new(); // discard pending reset
    restored.restore(&decoded).unwrap();
    assert_eq!(restored.snapshot(), state);
    let mut mem = [0u16; 512];
    mem[52] = 0;
    mem[53] = 0x200; // IRQ2 vector
    mem[256] = 0x7007; // MOVEQ #7,D0
    mem[257] = 0x4e72;
    mem[258] = 0x2700; // STOP
    let mut other = mem.clone();
    // A stopped masked CPU remains stopped, with IRQ still pending.
    assert_eq!(
        cpu.interpreter(&mut mem[..]),
        restored.interpreter(&mut other[..])
    );
    assert_eq!(cpu.snapshot(), restored.snapshot());
    // Wake as the board does and permit the pending IRQ.
    for c in [&mut cpu, &mut restored] {
        c.stop = false;
        c.regs.sr.interrupt_mask = 0;
    }
    for _ in 0..3 {
        assert_eq!(
            cpu.interpreter(&mut mem[..]),
            restored.interpreter(&mut other[..])
        );
        assert_eq!(cpu.snapshot(), restored.snapshot());
        assert_eq!(mem, other);
    }
    assert_eq!(restored.regs.d[0].0, 7);
    assert!(restored.stop);
}

#[test]
fn opcode_and_exception_frame_continue_identically() {
    let mut cpu = M68000::<Mc68000>::new_no_reset();
    cpu.regs.pc.0 = 0x100;
    cpu.regs.ssp.0 = 0x380;
    let mut mem = [0u16; 512];
    mem[128] = 0x7012; // populate private current opcode
    cpu.interpreter(&mut mem[..]);
    cpu.exception(Vector::AddressError.into());
    mem[6] = 0;
    mem[7] = 0x200;
    mem[256] = 0x4e71;
    let state = cpu.snapshot();
    assert_eq!(state.current_opcode, 0x7012);
    let mut restored = M68000::<Mc68000>::new();
    restored.restore(&state).unwrap();
    let mut other = mem.clone();
    assert_eq!(
        cpu.interpreter(&mut mem[..]),
        restored.interpreter(&mut other[..])
    );
    assert_eq!(mem, other, "exception stack must retain current opcode");
    assert_eq!(cpu.snapshot(), restored.snapshot());
}

#[test]
fn invalid_state_does_not_modify_cpu() {
    let mut cpu = M68000::<Mc68000>::new();
    let before = cpu.snapshot();
    for pending in [vec![26, 26], vec![26, 27], vec![3, 2]] {
        let mut bad = before.clone();
        bad.pending = pending;
        assert!(cpu.restore(&bad).is_err(), "accepted {:?}", bad.pending);
        assert_eq!(cpu.snapshot(), before);
    }
    let mut bad = before.clone();
    bad.status |= 0x4000;
    assert!(cpu.restore(&bad).is_err());
    assert_eq!(cpu.snapshot(), before);
}
