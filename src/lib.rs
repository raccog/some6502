use bitflags::bitflags;

/// The negative sign for a 2's complement, 8-bit integer
const NEGATIVE_SIGN_U8: u8 = 0b10000000;

bitflags! {
    /// An 8-bit register that holds all the 6502 flags.
    pub struct StatusRegister: u8 {
        const CARRY = 0b00000001;
        const ZERO = 0b00000010;
        const INTERRUPT = 0b00000100;
        const DECIMAL = 0b00001000;
        const B_FLAG = 0b00110000;
        const OVERFLOW = 0b01000000;
        const NEGATIVE = NEGATIVE_SIGN_U8;
    }
}

/// The necessary registers for any 6502.
///
/// This is a struct, as the registers don't really change between models.
pub struct Registers {
    /// The accumulator
    pub a: u8,
    /// The X index
    pub x: u8,
    /// The Y index
    pub y: u8,
    /// The status register with all flags
    pub flags: StatusRegister,
    /// The program counter or instruction pointer
    pub pc: u16,
}

impl Registers {
    /// Returns an empty set of registers; meaning every register is set to 0.
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            flags: StatusRegister::empty(),
            pc: 0,
        }
    }

    /// Sets every register to 0.
    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.flags = StatusRegister::empty();
        self.pc = 0;
    }
}

/// The 16-bit memory bus used by the 6502 instruction set.
///
/// Each 6502 model usually has a different memory map. Using a trait for this is beneficial because the
/// memory map can be implemented seperately for each model, while using the same API for reading and
/// writing to memory.
pub trait MemoryBus {
    /// Absolute indexed mode.
    /// 
    /// Either register X or Y can be used for `idx`.
    fn abs_idx(&self, address: u16, idx: u8) -> u16 {
        address.overflowing_add(idx as u16).0
    }

    /// Absolute indirect mode.
    /// 
    /// Only used for the JMP instruction.
    fn abs_indirect(&self, address: u16) -> u16 {
        let hi_addr = if address & 0xff == 0xff {
            address & 0xff00
        } else {
            address + 1
        };

        let lo = self.read(address);
        let hi = self.read(hi_addr);

        u16::from_le_bytes([lo, hi])
    }

    /// X-Indexed, Zero-Page Indirect mode.
    fn indirect_x(&self, address: u8, x: u8) -> u16 {
        let address = address.overflowing_add(x).0;
        let lo = self.read(address as u16);
        let hi = self.read(address.overflowing_add(1).0 as u16);
        u16::from_le_bytes([lo, hi])
    }

    /// Zero-Page Indirect, Y-Indexed mode.
    fn indirect_y(&self, address: u8, y: u8) -> u16 {
        let (lo, carry) = self.read(address as u16).overflowing_add(y);
        let hi = {
            let value = self.read(address.overflowing_add(1).0 as u16);
            if carry {
                value.overflowing_add(1).0
            } else {
                value
            }
        };

        u16::from_le_bytes([lo, hi])
    }

    /// Read a byte from the 16-bit address bus.
    fn read(&self, address: u16) -> u8;

    /// Write a byte to the 16-bit address bus.
    fn write(&mut self, address: u16, value: u8);

    /// Zero Page mode.
    fn zero_idx(&self, address: u8, idx: u8) -> u16 {
        address.overflowing_add(idx).0 as u16
    }
}

/// This trait defines the default 6502 instruction set.
///
/// It includes default implementations for each instruction, but they can also be overridden if needed. For
/// example, on the NES, there is no BCD mode, so the decimal flag should be overridden to do what the
/// NES does. This hopefully allows the instruction set to conform to any model of 6502.
///
/// # Registers and Memory Bus
///
/// This trait also includes 2 special functions for retrieving the 6502 models and memory bus.
///
/// These functions need to be implemented for each 6502 model.
///
/// # Instruction Modes
///
/// Each function is formatted with a naming scheme similar to the following examples of the ADC instruction:
///
/// * `adc_imm` - ADC immediate
/// * `adc_zero` - ADC zero page
/// * `adc_zero_x` - ADC zero page,X
/// * `adc_abs` - ADC absolute
/// * `adc_abs_x` - ADC absolute,X
/// * `adc_abs_y` - ADC absolute,Y
/// * `adc_ind_x` - ADC indirect,X
/// * `adc_ind_y` - ADC indirect,Y
///
/// As shown above, each function starts with its shortened name and ends with the addressing mode.
///
/// Functions that end in common, (`adc_common`) do not use any addressing mode. The common functions are
/// used to share implementions between an instruction with different addressing modes.
pub trait InstructionExecution {
    /// Returns the memory bus connected to this execution engine.
    fn bus(&mut self) -> &mut dyn MemoryBus;

    /// Returns the registers connected to this execution engine.
    fn registers(&mut self) -> &mut Registers;

    /// Common implementation for the ADC instruction.
    fn adc_common(&mut self, value: u8) {
        let registers = self.registers();
        // TODO: Check decimal flag

        // Perform addition as unsigned 32-bit integers
        let result = registers.a as u32
            + value as u32
            + (registers.flags & StatusRegister::CARRY).bits() as u32;

        // Carry flag if result does not fit in an unsigned 8-bit integer
        registers
            .flags
            .set(StatusRegister::CARRY, result > u8::MAX as u32);

        // Truncate result to an unsigned 8-bit integer
        let result = (result & u8::MAX as u32) as u8;

        // Overflow flag
        // This one is kinda complicated.
        // It is set if both operands have the same sign, but the result has the opposite or "incorrect" sign.
        // This could be (positive + positive = negative) or (negative + negative = positive).
        let overflow = ((registers.a ^ value) & NEGATIVE_SIGN_U8 == 0)
            && ((registers.a ^ result) & NEGATIVE_SIGN_U8 == NEGATIVE_SIGN_U8);
        registers.flags.set(StatusRegister::OVERFLOW, overflow);

        // Negative flag if the result is negative (interpreted as 2's complement)
        registers.flags.set(
            StatusRegister::NEGATIVE,
            result & NEGATIVE_SIGN_U8 == NEGATIVE_SIGN_U8,
        );

        // Zero flag if the result is 0
        registers.flags.set(StatusRegister::ZERO, result == 0);

        // Set accumulator to result
        registers.a = result;
    }

    fn adc_imm(&mut self, value: u8) {
        self.adc_common(value);
    }

    fn adc_zero(&mut self, address: u8) {
        let value = self.bus().read(address as u16);
        self.adc_common(value);
    }

    fn adc_zero_x(&mut self, address: u8) {
        let x = self.registers().x;
        let zero_addr = self.bus().zero_idx(address, x);
        let value = self.bus().read(zero_addr as u16);
        self.adc_common(value);
    }

    fn adc_abs(&mut self, address: u16) {
        let value = self.bus().read(address);
        self.adc_common(value);
    }

    fn adc_abs_x(&mut self, address: u16) {
        let x = self.registers().x;
        let abs_addr = self.bus().abs_idx(address, x);
        let value = self.bus().read(abs_addr);
        self.adc_common(value);
    }

    fn adc_abs_y(&mut self, address: u16) {
        let y = self.registers().y;
        let abs_addr = self.bus().abs_idx(address, y);
        let value = self.bus().read(abs_addr);
        self.adc_common(value);
    }

    fn adc_ind_x(&mut self, address: u8) {
        let x = self.registers().x;
        let indirect_addr = self.bus().indirect_x(address, x);
        let value = self.bus().read(indirect_addr);
        self.adc_common(value);
    }

    fn adc_ind_y(&mut self, address: u8) {
        let y = self.registers().y;
        let indirect_addr = self.bus().indirect_y(address, y);
        let value = self.bus().read(indirect_addr);
        self.adc_common(value);
    }
}
