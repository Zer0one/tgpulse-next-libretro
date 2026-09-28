use v60::{cpu::PC, Bus, V60};

struct Io {
    memory: Vec<u8>,
    ready: bool,
    stalled: bool,
    addresses: Vec<u32>,
    writes: usize,
    value: u32,
}

impl Io {
    fn new(program: &[u8], value: u32) -> Self {
        let mut memory = vec![0; 0x2000];
        memory[..program.len()].copy_from_slice(program);
        Self {
            memory,
            ready: false,
            stalled: false,
            addresses: vec![],
            writes: 0,
            value,
        }
    }
    fn input(&mut self, addr: u32) -> u32 {
        self.addresses.push(addr);
        self.stalled = !self.ready;
        if self.ready {
            self.value
        } else {
            0
        }
    }
}

impl Bus for Io {
    fn read_u8(&mut self, addr: u32) -> u8 {
        self.memory.get(addr as usize).copied().unwrap_or(0)
    }
    fn write_u8(&mut self, addr: u32, value: u8) {
        self.memory[addr as usize] = value;
        self.writes += 1;
    }
    fn read_io8(&mut self, addr: u32) -> u8 {
        self.input(addr) as u8
    }
    fn read_io16(&mut self, addr: u32) -> u16 {
        self.input(addr) as u16
    }
    fn read_io32(&mut self, addr: u32) -> u32 {
        self.input(addr)
    }
    fn take_io_stall(&mut self) -> bool {
        std::mem::take(&mut self.stalled)
    }
}

#[test]
fn stalled_in_preserves_auto_address_registers_for_every_width() {
    for (opcode, step, result) in [
        (0x20, 1, 0xaabb_cc78),
        (0x22, 2, 0xaabb_5678),
        (0x24, 4, 0x1234_5678),
    ] {
        for decrement in [false, true] {
            // Includes SP; only the first operand's register update is rolled back.
            for source in [1, 31] {
                let mode = if decrement { 0xa0 } else { 0x80 } | source;
                let mut bus = Io::new(&[opcode, 0x60, mode, 0], 0x1234_5678);
                let mut cpu = V60::new();
                cpu.reg[PC] = 0;
                cpu.reg[0] = 0xaabb_ccdd;
                cpu.reg[source as usize] = 0x8000;
                let before = cpu.reg;
                for _ in 0..3 {
                    cpu.run(&mut bus, 8);
                    assert_eq!(cpu.reg, before);
                    assert_eq!(bus.writes, 0);
                }
                bus.ready = true;
                cpu.run(&mut bus, 8);
                let address = if decrement { 0x8000 - step } else { 0x8000 };
                assert_eq!(bus.addresses, vec![address; 4]);
                assert_eq!(
                    cpu.reg[source as usize],
                    if decrement {
                        0x8000 - step
                    } else {
                        0x8000 + step
                    }
                );
                assert_eq!(cpu.reg[0], result);
                assert_eq!(cpu.pc(), 3);
            }
        }
    }
}

#[test]
fn stalled_in_does_not_decode_or_write_auto_increment_destination() {
    // IN.W [R1+],[R2+]. A legitimate zero must complete just like other data.
    for value in [0, 0x1234_5678] {
        let mut bus = Io::new(&[0x24, 0xe0, 0x81, 0x82, 0], value);
        let mut cpu = V60::new();
        cpu.reg[PC] = 0;
        cpu.reg[1] = 0x8000;
        cpu.reg[2] = 0x1000;
        let before = cpu.reg;
        for _ in 0..3 {
            cpu.run(&mut bus, 8);
            assert_eq!(cpu.reg, before);
            assert_eq!(bus.writes, 0);
        }
        bus.ready = true;
        cpu.run(&mut bus, 8);
        assert_eq!((cpu.reg[1], cpu.reg[2], cpu.pc()), (0x8004, 0x1004, 4));
        assert_eq!(bus.addresses, vec![0x8000; 4]);
        assert_eq!(bus.writes, 4);
        assert_eq!(bus.read_u32(0x1000), value);
    }
}
