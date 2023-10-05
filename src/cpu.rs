use crate::memory::Memory;
use crate::registers::*;

const KIB:usize = 1024;

#[derive(Copy, Clone)]
pub struct CPU {
    pub halted: bool,
    pub ime: bool,
    pub ime_waiting: bool,
    pub registers: Registers,
    pub memory: Memory,
    pub pc: u16,
    pub sp: u16,
}

impl CPU{
    pub fn new() -> Self {
        CPU {
            halted: false,
            ime: false,
            ime_waiting: false,
            registers: Registers::new(),
            memory: Memory::new(),
            pc: 0x100,
            sp: 0x0,
        }
    }

    pub fn fetch(&mut self) -> u8 {
        let addr = self.pc;
        let data = self.memory.read(addr);
        self.pc = self.pc.wrapping_add(1);
        data
    }

    pub fn fetchW(&mut self) -> u16 {
        let upper_byte = self.fetch() as u16;
        let lower_byte = self.fetch() as u16;
        (upper_byte << 8) | lower_byte 
    }

    pub fn stack_push(mut self, num:u16){
        self.memory.write(self.sp.into(), (num >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.memory.write(self.sp.into(), (num & 0xFF) as u8);
        self.sp = self.sp.wrapping_sub(1);
    }

    pub fn stack_pop(mut self) -> u16{
        let lower = self.memory.read(self.sp.into()) as u16;
        self.sp = self.sp.wrapping_add(1);
        let upper = self.memory.read(self.sp.into()) as u16;
        self.sp = self.sp.wrapping_add(1);
        ((upper << 8) | lower)
    }

    pub fn execute(&mut self, opcode:u8) {

        if self.ime_waiting && opcode != 0xFB {
            self.ime = true;
            self.ime_waiting = false;
        }
    }

   // izik1.github.io/gbops/index.html
    // LD
    // Load a register with another register
    pub fn reg_ld_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        self.registers.set_reg(dst, src);
    }

    pub fn sp_ld_hl(&mut self) {
        let src = self.registers.get_regW(RegW::HL);
        self.sp = src;
    }

    pub fn hl_ld_spi8(&mut self) {
        self.sp.wrapping_add_signed((self.fetch() as i16));
        self.registers.set_regW(RegW::HL, self.sp);
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

    pub fn addr_ld_regA(&mut self) {
        let dst = self.fetchW();
        let src = self.registers.get_reg(Reg::A);
        self.memory.write(dst, src);
    }

    pub fn addr_ld_sp(&mut self) {
        let src = self.sp;
        let upper = (src >> 8) as u8;
        let lower = (src << 8) as u8;
        let dst = self.fetchW();
        self.memory.write(dst, upper);
        self.memory.write(dst.wrapping_add(1), lower);
    }

    // Load a register with the value at an address
    pub fn reg_ld_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        self.registers.set_reg(dst, src);
    }

    pub fn regA_ld_addr(&mut self) {
        let dst = self.fetchW();
        let src = self.memory.read(dst);
        self.registers.set_reg(Reg::A, src);
    }

    // LD (ff00+u8)
    pub fn reg_ld_u8ff00(&mut self) {
        let dst = self.fetch() as u16 + 0xFF00;
        let src = self.memory.read(dst);
        self.registers.set_reg(Reg::A, src);
    }

    pub fn u8ff00_ld_reg(&mut self) {
        let dst = self.fetch() as u16 + 0xFF00;
        let src = self.registers.get_reg(Reg::A);
        self.memory.write(dst, src);
    }

    pub fn reg_ld_regff00(&mut self) {
        let dst = self.registers.get_reg(Reg::C) as u16 + 0xFF00;
        let src = self.memory.read(dst);
        self.registers.set_reg(Reg::A, src);
    }

    pub fn regff00_ld_reg(&mut self) {
        let dst = self.registers.get_reg(Reg::C) as u16 + 0xFF00;
        let src = self.registers.get_reg(Reg::A);
        self.memory.write(dst, src);
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

    // Add value at regW address to register 
    pub fn reg_add_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum);
    }

    // Add operand to register
    pub fn reg_add_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum);
    }

    pub fn sp_add_operand(&mut self) {
        self.sp.wrapping_add_signed(self.fetch() as i16);
    }
    
    // Adc two registers
    pub fn reg_adc_reg(&mut self, dst: Reg, src: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let src = self.registers.get_reg(src).wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum); 
    }

    // Adc value at regW address to register
    pub fn reg_adc_regWaddr(&mut self, dst: Reg, src: RegW) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr).wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum);
    }

    // Adc operand to register
    pub fn reg_adc_operand(&mut self, dst: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let src = self.fetch().wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_add(src);
        self.registers.set_reg(dst, sum);
    }

    // Sub two registers
    pub fn reg_sub_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum); 
    }

    // Sub value at regW address from register 
    pub fn reg_sub_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum); 
    }

    // Sub operand from register
    pub fn reg_sub_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum);
    }

    // Sbc two registers
    pub fn reg_sbc_reg(&mut self, dst: Reg, src: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let src = self.registers.get_reg(src).wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum); 
    }

    // Sbc value at regW address from register
    pub fn reg_sbc_regWaddr(&mut self, dst: Reg, src: RegW) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr).wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum);
    }

    // Sbc operand from register
    pub fn reg_sbc_operand(&mut self, dst: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let src = self.fetch().wrapping_add(cy);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
        self.registers.set_reg(dst, sum);
    }

    // AND / OR / XOR / CP
    // And two registers
    pub fn reg_and_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst) & src;
        self.registers.set_reg(dst, sum);
    }

    // And register with regW address value
    pub fn reg_and_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst) & src;
        self.registers.set_reg(dst, sum);
    }

    // And operand with register
    pub fn reg_and_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst) & src;
        self.registers.set_reg(dst, sum);
    }

    // Xor two registers
    pub fn reg_xor_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst) ^ src;
        self.registers.set_reg(dst, sum);
    } 

    // Xor register with regW address value
    pub fn reg_xor_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst) ^ src;
        self.registers.set_reg(dst, sum);
    }

    // Xor operand with register
    pub fn reg_xor_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst) ^ src;
        self.registers.set_reg(dst, sum);
    }

    // Or two registers
    pub fn reg_or_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst) | src;
        self.registers.set_reg(dst, sum);
    }

    // Or register with regW address value
    pub fn reg_or_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst) | src;
        self.registers.set_reg(dst, sum);
    }

    // Or operand with register
    pub fn reg_or_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst) | src;
        self.registers.set_reg(dst, sum);
    }

    // CP two registers
    pub fn reg_cp_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
    }

    // CP value at regW address with register 
    pub fn reg_cp_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.memory.read(addr);
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
    }

    // CP operand with register
    pub fn reg_cp_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst).wrapping_sub(src);
    }

    // SP POP / PUSH to register
    pub fn regW_pop_sp(&mut self, dst: RegW) {
        let src = self.stack_pop();
        self.registers.set_regW(dst, src);
    }

    pub fn regW_push_sp(&mut self, src: RegW) {
        let src = self.registers.get_regW(src);
        self.stack_push(src);
    }

    // Rotates
    // RLC
    pub fn rlc(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src >> 7;
        src <<= 1;
        self.registers.set_reg(dst, (src | cy));
    }

    // RL
    pub fn rl(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src >> 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        src <<= 1;
        self.registers.set_reg(dst, (src | cf));
    }

    // RRC
    pub fn rrc(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src << 7;
        src >>= 1;
        self.registers.set_reg(dst, (src | cy));
    }

    // RR
    pub fn rr(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src << 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        src >>= 1;
        self.registers.set_reg(dst, (src | cf));
    }
    // Misc
    // CPL / Compliment register A
    pub fn cpl(&mut self) {
        let src = self.registers.get_reg(Reg::A);
        self.registers.set_reg(Reg::A, !src);
    }

    // CCF / Compliment carry flag
    pub fn ccf(&mut self) {
        if self.registers.get_flag(Flag::Z) {
            self.registers.set_flag(Flag::Z, false);
        } else { self.registers.set_flag(Flag::Z, true); }
    }

    // SCF / Set carry flag
    pub fn scf(&mut self) {
        self.registers.set_flag(Flag::Z, true);
    }

    // DAA / Decimal encoding
    pub fn daa(&mut self) {
        let mut correction = 0;
        let src = self.registers.get_reg(Reg::A);
        if (self.registers.get_flag(Flag::H)) || ((src & 0xF) > 0x9) {
            correction |= 0x06;
        }
        if (self.registers.get_flag(Flag::C)) || (src > 0x9) {
            correction |= 0x60;
        }
        if !self.registers.get_flag(Flag::N) {
            self.registers.set_reg(Reg::A, src.wrapping_add(correction));
        }
    }

    // JP / JR
    // Relative Jumps
    // jr
    pub fn jr(&mut self) {
        let r_pos = self.fetch() as i16;
        self.pc = self.pc.wrapping_add_signed(r_pos);
    }

    // If not flag
    pub fn jr_nf(&mut self, f: Flag) {
        let r_pos = self.fetch() as i16;
        if !self.registers.get_flag(f) {
            self.pc = self.pc.wrapping_add_signed(r_pos);
        }
    }

    // If flag
    pub fn jr_f(&mut self, f: Flag) {
        let r_pos = self.fetch() as i16;
        if self.registers.get_flag(f) {
            self.pc = self.pc.wrapping_add_signed(r_pos);
        }
    }

    // Jumps
    // jp
    pub fn jp(&mut self) {
        let pos = self.fetchW();
        self.pc = pos;
    }

    // jp hl
    pub fn jp_hl(&mut self) {
        let pos = self.registers.get_regW(RegW::HL);
        self.pc = pos;
    }

    // If not flag
    pub fn jp_nf(&mut self, f: Flag) {
        let pos = self.fetchW();
        if !self.registers.get_flag(f) {
            self.pc = pos;
        }
    }

    // If flag
    pub fn jp_f(&mut self, f: Flag) {
        let pos = self.fetchW();
        if self.registers.get_flag(f) {
            self.pc = pos;
        }
    }

    // RET / Return
    pub fn ret(&mut self) {
        self.pc = self.stack_pop();
    }

    // If not flag
    pub fn ret_nf(&mut self, f: Flag) {
        if !self.registers.get_flag(f) {
            self.pc = self.stack_pop();
        }
    }

    // If flag
    pub fn ret_f(&mut self, f: Flag) {
        if self.registers.get_flag(f) {
            self.pc = self.stack_pop();
        }
    }

    pub fn reti(&mut self) {
        self.pc = self.stack_pop();
        self.ime = true;
    }
    
    // Calls
    // CALL
    pub fn call(&mut self) {
        let pos = self.fetchW();
        self.stack_push(self.pc);
        self.pc = pos;
    }

    // If not flag
    pub fn call_nf(&mut self, f: Flag) {
        let pos = self.fetchW();
        if !self.registers.get_flag(f) {
            self.stack_push(self.pc);
            self.pc = pos;
        }
    }

    // If flag
    pub fn call_f(&mut self, f: Flag) {
        let pos = self.fetchW();
        if self.registers.get_flag(f) {
            self.stack_push(self.pc);
            self.pc = pos;
        }
    }

    // RST - Reset
    pub fn rst(&mut self, rst: u16) {
        self.stack_push(self.pc);
        self.pc = rst;
    }

    // DI / EI
    pub fn ei(&mut self) {
        self.ime_waiting = true;
    }

    pub fn di(&mut self) {
        self.ime = false;
        self.ime_waiting = false;
    }
}