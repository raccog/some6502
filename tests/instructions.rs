use some6502::*;

struct TestBus {
    memory: [u8; u16::MAX as usize],
}

impl TestBus {
    pub fn new() -> Self {
        Self {
            memory: [0; u16::MAX as usize],
        }
    }
}

impl MemoryBus for TestBus {
    fn read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.memory[address as usize] = value;
    }
}

struct TestExecution {
    pub bus: TestBus,
    pub registers: Registers,
}

impl InstructionExecution for TestExecution {
    fn bus(&mut self) -> &mut dyn MemoryBus {
        &mut self.bus
    }

    fn registers(&mut self) -> &mut Registers {
        &mut self.registers
    }
}

#[test]
fn abs_idx() {
    let bus = TestBus::new();

    assert_eq!(bus.abs_idx(0, 0x8), 0x8);
    assert_eq!(bus.abs_idx(0xff00, 0x8), 0xff08);
    assert_eq!(bus.abs_idx(0xffff, 0x2), 0x1);
}

#[test]
fn abs_indirect() {
    let mut bus = TestBus::new();

    assert_eq!(bus.abs_indirect(0), 0);

    bus.memory[0] = 0xef;
    bus.memory[1] = 0xbe;

    assert_eq!(bus.abs_indirect(0), 0xbeef);

    bus.memory[0xff01] = 0xab;
    bus.memory[0xff02] = 0xcd;

    assert_eq!(bus.abs_indirect(0xff01), 0xcdab);

    bus.memory[0xabff] = 0x34;
    bus.memory[0xab00] = 0x12;

    assert_eq!(bus.abs_indirect(0xabff), 0x1234);
}

#[test]
fn zero_idx() {
    let bus = TestBus::new();

    assert_eq!(bus.zero_idx(0, 0x8), 0x8);
    assert_eq!(bus.zero_idx(0xab, 0x1), 0xac);
    assert_eq!(bus.zero_idx(0xff, 0x8), 0x7);
}

#[test]
fn indirect_x() {
    let mut bus = TestBus::new();

    assert_eq!(bus.indirect_x(0, 0), 0);

    bus.memory[0] = 0xef;
    bus.memory[1] = 0xbe;

    assert_eq!(bus.indirect_x(0, 0), 0xbeef);

    bus.memory[0x01] = 0xab;
    bus.memory[0x02] = 0xcd;

    assert_eq!(bus.indirect_x(0x00, 1), 0xcdab);

    bus.memory[0x80] = 0x34;
    bus.memory[0x81] = 0x12;

    assert_eq!(bus.indirect_x(0x48, 0x38), 0x1234);

    bus.memory[0xff] = 0x34;
    bus.memory[0x00] = 0x12;

    assert_eq!(bus.indirect_x(0xff, 0), 0x1234);
}

#[test]
fn indirect_y() {
    let mut bus = TestBus::new();

    assert_eq!(bus.indirect_y(0, 0), 0);

    bus.memory[0] = 0xef;
    bus.memory[1] = 0xbe;

    assert_eq!(bus.indirect_y(0, 0), 0xbeef);

    bus.memory[0x01] = 0xab;
    bus.memory[0x02] = 0xcd;

    assert_eq!(bus.indirect_y(0x01, 1), 0xcdac);

    bus.memory[0x80] = 0xff;
    bus.memory[0x81] = 0x12;

    assert_eq!(bus.indirect_y(0x80, 0x2), 0x1301);

    bus.memory[0xff] = 0x34;
    bus.memory[0x00] = 0x12;

    assert_eq!(bus.indirect_y(0xff, 0x21), 0x1255);
}

#[test]
fn adc_common() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    assert_eq!(execution.registers.a, 0);

    // 0 + 1 = 1
    execution.adc_common(1);
    assert_eq!(execution.registers.a, 1);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );

    // 1 + 1 = 2
    execution.adc_common(1);
    assert_eq!(execution.registers.a, 2);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );

    // 2 + 127 = 129
    execution.adc_common(127);
    assert_eq!(execution.registers.a, 129);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::OVERFLOW
    );

    // 0x81 + 0x80 = 0x01
    execution.adc_common(0x80);
    assert_eq!(execution.registers.a, 01);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::CARRY
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::OVERFLOW
    );

    // 1 + 1 + 1(carry) = 3
    execution.adc_common(1);
    assert_eq!(execution.registers.a, 03);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_imm() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xff;

    execution.adc_imm(1);
    assert_eq!(execution.registers.a, 0);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::CARRY
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_zero() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xf0;
    execution.bus().write(0xaa, 0xf);

    execution.adc_zero(0xaa);
    assert_eq!(execution.registers.a, 0xff);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_zero_x() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xf0;
    execution.registers.x = 0xa;
    execution.bus().write(0xaa, 0xf);

    execution.adc_zero_x(0xa0);
    assert_eq!(execution.registers.a, 0xff);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_abs() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xf0;
    execution.bus().write(0xaabb, 0xf);

    execution.adc_abs(0xaabb);
    assert_eq!(execution.registers.a, 0xff);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_abs_x() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xf0;
    execution.registers.x = 0xbb;
    execution.bus().write(0xaabb, 0xf);

    execution.adc_abs_x(0xaa00);
    assert_eq!(execution.registers.a, 0xff);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_abs_y() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xe0;
    execution.registers.y = 0xdd;
    execution.bus().write(0xccdd, 0xe);

    execution.adc_abs_y(0xcc00);
    assert_eq!(execution.registers.a, 0xee);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_ind_x() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xf0;
    execution.registers.x = 0xb;
    execution.bus().write(0xab, 0xcd);
    execution.bus().write(0xac, 0xab);
    execution.bus().write(0xabcd, 0xf);

    execution.adc_ind_x(0xa0);
    assert_eq!(execution.registers.a, 0xff);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}

#[test]
fn adc_ind_y() {
    let mut execution = TestExecution {
        bus: TestBus::new(),
        registers: Registers::new(),
    };

    execution.registers.a = 0xf0;
    execution.registers.y = 0xe;
    execution.bus().write(0xab, 0xbf);
    execution.bus().write(0xac, 0xab);
    execution.bus().write(0xabcd, 0xf);

    execution.adc_ind_y(0xab);
    assert_eq!(execution.registers.a, 0xff);
    assert_eq!(
        execution.registers.flags & StatusRegister::CARRY,
        StatusRegister::empty()
    );
    assert_eq!(
        execution.registers.flags & StatusRegister::OVERFLOW,
        StatusRegister::empty()
    );
}