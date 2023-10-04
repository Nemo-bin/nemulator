use crate::memory::Memory;
use crate::registers::*;

const KIB:usize = 1024;

#[derive(Copy, Clone)]
pub struct CPU {
    pub halted: bool,
    pub ime: bool,
    pub registers: Registers,
    pub memory: Memory,
    pub pc: u16,
    pub sp: u8,
}

impl CPU{
    pub fn new() -> Self {
        CPU {
            halted: false,
            ime: false,
            registers: Registers::new(),
            memory: Memory::new(),
            pc: 0x100,
            sp: 0x0,
        }
    }

    pub fn fetch(&mut self) -> u8 {
        let addr = self.pc;
        let data = self.memory.read(addr);
        self.pc += 1;
        data
    }

    pub fn fetchW(&mut self) -> u16 {
        let upper_byte = self.fetch() as u16;
        let lower_byte = self.fetch() as u16;
        (upper_byte << 8) | lower_byte 
    }

    pub fn execute(&mut self, opcode:u8) {
    }

   // izik1.github.io/gbops/index.html
    // LD
    // Load a register with another register
    pub fn reg_ld_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        self.registers.set_reg(dst, src);
    }

    // Load a register with the operand
    pub fn reg_ld_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        self.registers.set_reg(dst, src);
    }

    pub fn regW_ld_operand(&mut self, dst: RegW) {
        let src = self.fetchW();
        self.registers.set_regW(dst, src);
    }

    // Load an address with a register value
    pub fn regWaddr_ld_reg(&mut self, dst: RegW, src: Reg) {
        let dst = self.registers.get_regW(dst);
        let src = self.registers.get_reg(src);
        self.memory.write(dst, src);
    }

    pub fn addr_ld_sp(&mut self) {
        let src = self.sp;
        let dst = self.fetchW();
        self.memory.write(dst, src);
    }

    // Load a register with the value at an address
    pub fn reg_ld_addr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        self.registers.set_reg(dst, src);
    }

    // INC / DEC
    // Increment a register
    pub fn inc_reg(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst).wrapping_add(1);
        self.registers.set_reg(dst, src);
    }

    pub fn inc_regW(&mut self, dst: RegW) {
        let src = self.registers.get_regW(dst).wrapping_add(1);
        self.registers.set_regW(dst, src);
    }

    // Decrement a register
    pub fn dec_reg(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst).wrapping_sub(1);
        self.registers.set_reg(dst, src);
    }

    pub fn dec_regW(&mut self, dst: RegW) {
        let src = self.registers.get_regW(dst).wrapping_sub(1);
        self.registers.set_regW(dst, src);
    }

    // ADD / SUB / ADC / SBC
    // Add two registers
    pub fn reg_add_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum); 
    }

    pub fn regW_add_regW(&mut self, dst: RegW, src: RegW) {
        let src = self.registers.get_regW(src);
        let sum = self.registers.get_regW(dst).wrapping_add(src);
        self.registers.set_regW(dst, sum); 
    }

    // Add value at address to register 
    pub fn reg_add_addr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum);
    }
    
    // Adc two registers
    pub fn reg_adc_reg(&mut self, dst: Reg, src: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let src = self.registers.get_reg(src).wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum); 
    }

    // Adc value at address to register
    pub fn reg_adc_addr(&mut self, dst: Reg, src: RegW) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr).wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum);
    }

    // Sub two registers
    pub fn reg_sub_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum); 
    }
    // Sub value at address from register 
    pub fn reg_sub_addr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum); 
    }
    // Sbc two registers
    pub fn reg_sbc_reg(&mut self, dst: Reg, src: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let src = self.registers.get_reg(src).wrapping_sub(cy);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum); 
    }

    // Sbc value at address from register
    pub fn reg_sbc_addr(&mut self, dst: Reg, src: RegW) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr).wrapping_sub(cy);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum);
    }
}