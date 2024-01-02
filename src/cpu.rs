use crate::memory::Memory;
use crate::registers::*;
use crate::ppu::*;
use crate::timer::*;

use std::borrow::BorrowMut;
use std::{thread, time};

/////////////////////////////// INTERRUPT PRIORITY QUEUE ////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Interrupt {
    VBlank,
    STAT,
    Timer,
    Serial,
    Joypad,
}

// A vec can be used to represent a binary tree, using vec[0] as the root, [1], [2], as its children etc.
pub struct BinaryHeap {
    nodes: Vec<Interrupt>,
}

impl BinaryHeap {
    pub fn new() -> Self {
        BinaryHeap {
            nodes: Vec::new(),
        }
    }

    // functions to index parents and children of a node n:
    pub fn parent_index(n: usize) -> usize {
        (n - 1) / 2
    }
    
    pub fn left_child(n: usize) -> usize {
        n * 2
    }

    pub fn right_child(n: usize) -> usize {
        n * 2 + 1
    }

    pub fn is_empty(&self) -> bool {
        if self.nodes.len() == 0 {
            true
        } else { false }
    }

    pub fn get_interrupt_priority(int: Interrupt) -> usize {
        let priority = match int {
            Interrupt::VBlank => 4,
            Interrupt::STAT => 3,
            Interrupt::Timer => 2,
            Interrupt::Serial => 1,
            Interrupt::Joypad => 0,
        };
        priority
    }

    pub fn push(&mut self, int: Interrupt) {
        self.nodes.push(int);
        self.shift_up(self.nodes.len() - 1);
    }

    fn shift_up(&mut self, i: usize) {
        if i == 0 {
            return;
        }
        let int = self.nodes[i];
        let pushed_priority = Self::get_interrupt_priority(int);
        let parent = Self::parent_index(i);
        if Self::get_interrupt_priority(self.nodes[parent]) <= pushed_priority {
            self.nodes.swap(parent, i);
            self.shift_up(parent);
        }
    }

    pub fn pop(&mut self) -> Option<Interrupt> {
        if self.nodes.is_empty() {
            None
        } else {
            let i = self.nodes.len() - 1;
            self.nodes.swap(0, i);
            self.shift_down(0, i);

            self.nodes.pop()
        }
    }

    fn shift_down(&mut self, i: usize, len: usize) {
        let left_child = Self::left_child(i);
        let right_child = Self::right_child(i);

        let highest_priority = i;

    if self.nodes.len() > left_child {
        let left_child_priority = Self::get_interrupt_priority(self.nodes[left_child]);
        if left_child_priority < len && Self::get_interrupt_priority(self.nodes[i]) <= left_child_priority {
            let highest_priority = left_child;
        }
    }

    if self.nodes.len() > right_child {
        let right_child_priority = Self::get_interrupt_priority(self.nodes[right_child]);
        if right_child_priority < len && Self::get_interrupt_priority(self.nodes[i]) <= right_child_priority {
            let highest_priority = right_child;
        }
    }

        if highest_priority != i {
            self.nodes.swap(i, highest_priority);
            self.shift_down(highest_priority, len)
        }
    }
}
////////////////////////////// INPUTS ///////////////////////////////
pub struct InputStates {
    pub down: bool,
    pub up: bool,
    pub left: bool,
    pub right: bool,

    pub start: bool,
    pub select: bool,
    pub b: bool, 
    pub a: bool,
}

impl InputStates {
    pub fn new() -> Self {
        InputStates {
            down: false,
            up: false,
            left: false,
            right: false,

            start: false,
            select: false,
            b: false,
            a: false,
        }
    }

    pub fn get_states(&mut self, joyp: u8) -> u8 {
        if joyp & 0b0010_0000 != 0 && joyp & 0b0001_0000 == 0 { // Dpad selected
            let states = 0b0001_0000 | (self.down as u8) << 3 | (self.up as u8) << 2 | (self.left as u8) << 1 | (self.right as u8); 
            !states
        } else if joyp & 0b0010_0000 == 0 && joyp & 0b0001_0000 != 0 {
            let states = 0b0010_0000 | (self.start as u8) << 3 | (self.select as u8) << 2 | (self.b as u8) << 1 | (self.a as u8); 
            !states
        } else { 0xFF }
    }
}


/////////////////////////////// CPU ////////////////////////////////

trait SupportedDataType {}
impl SupportedDataType for u8 {}
impl SupportedDataType for u16 {}
impl SupportedDataType for i16 {}

trait Eval {
    type GenericInteger: SupportedDataType;
    fn eval_z(&self) -> bool;
    fn eval_c_add(&self, a: Self::GenericInteger, b: Self::GenericInteger) -> bool;
    fn eval_h_add(&self, a: Self::GenericInteger, b: Self::GenericInteger) -> bool;
    fn eval_c_sub(&self, a: Self::GenericInteger, b: Self::GenericInteger) -> bool;
    fn eval_h_sub(&self, a: Self::GenericInteger, b: Self::GenericInteger) -> bool;
}

impl Eval for u8 {
    type GenericInteger = u8;

    fn eval_z(&self) -> bool {
        *self == 0
    }

    fn eval_c_add(&self, a: u8, b: u8) -> bool {
        (a.wrapping_add(b)) < a || (a.wrapping_add(b)) < b
    }

    fn eval_h_add(&self, a: u8, b: u8) -> bool {
        let sum = (a & 0xf) + (b & 0xf);
        (sum & 0x10) == 0x10
    }

    fn eval_c_sub(&self, a: u8, b: u8) -> bool {
        (a as i16 - b as i16) < 0
    }

    fn eval_h_sub(&self, a: u8, b: u8) -> bool {
        (a & 0x0F) < (b & 0x0F)
    }
}

impl Eval for u16 {
    type GenericInteger = u16;

    fn eval_z(&self) -> bool {
        *self == 0
    }

    fn eval_c_add(&self, a: u16, b: u16) -> bool {
        (a.wrapping_add(b)) < a || (a.wrapping_add(b)) < b
    }

    fn eval_h_add(&self, a: u16, b: u16) -> bool {
        let sum = (a & 0xfff) + (b & 0xfff);
        (sum & 0x1000) == 0x1000
    }

    fn eval_c_sub(&self, a: u16, b: u16) -> bool {
        (a as i16 - b as i16) < 0
    }

    fn eval_h_sub(&self, a: u16, b: u16) -> bool {
        (a & 0xFFF) < (b & 0xFFF)
    }
}

impl Eval for i16 {
    type GenericInteger = i16;

    fn eval_z(&self) -> bool {
        *self == 0
    }

    fn eval_c_add(&self, a: i16, b: i16) -> bool {
        (a.wrapping_add(b)) < a || (a.wrapping_add(b)) < b
    }

    fn eval_h_add(&self, a: i16, b: i16) -> bool {
        let sum = (a & 0xfff) + (b & 0xfff);
        (sum & 0x1000) == 0x1000
    }

    fn eval_c_sub(&self, a: i16, b: i16) -> bool {
        (a as i16 - b as i16) < 0
    }

    fn eval_h_sub(&self, a: i16, b: i16) -> bool {
        (a & 0x0F) < (b & 0x0F)
    }
}

const KIB:usize = 1024;

pub struct CPU {
    pub halted: bool,
    pub ime: bool,
    ime_waiting: bool,
    pub registers: Registers,
    pub memory: Memory,
    pub pc: u16,
    pub sp: u16,

    pub t_cycles: u16,
    pub timer: Timer,

    pub ppu: PPU,

    pub interrupt_queue: BinaryHeap,
    interrupt_queue_bitflags: u8,

    pub input_states: InputStates,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            halted: false,
            ime: false,
            ime_waiting: false,
            registers: Registers::new(),
            memory: Memory::new(),
            pc: 0x100,
            sp: 0xFFFE,

            t_cycles: 0,
            timer: Timer::new(),

            ppu: PPU::new(),

            interrupt_queue: BinaryHeap::new(),
            interrupt_queue_bitflags: 0,
            input_states: InputStates::new(),
        }
    }

    pub fn m_cycle(&mut self) {
        self.t_cycles = self.t_cycles.wrapping_add(4);
        let memory_ref = &mut self.memory;
        self.ppu.tick(memory_ref);
        self.timer.inc_sysclk();
        // thread::sleep(time::Duration::from_nanos(1));
    }

    pub fn set_vblank_flag(&mut self) {
        if self.ppu.entered_vblank {
            self.ppu.entered_vblank = false;
            let interrupt_flags = self.memory.read(0xFF0F);
            self.memory.write(0xFF0F, interrupt_flags | 0b0000_0001);
            // println!("VBLANK FLAG SET - {:x}", self.memory.read(0xFF0F));
        }
    }

    pub fn set_stat_flag(&mut self) {
        if self.ppu.stat_irq {
            self.ppu.stat_irq = false;
            let interrupt_flags = self.memory.read(0xFF0F);
            self.memory.write(0xFF0F, interrupt_flags | 0b0000_0010);
            // println!("STAT FLAG SET - {:x}", self.memory.read(0xFF0F));
        }
    }

    pub fn set_tima_flag(&mut self) {
        if self.timer.tima_overflow_irq {
            self.timer.tima_overflow_irq = false;
            let interrupt_flags = self.memory.read(0xFF0F);
            self.memory.write(0xFF0F, interrupt_flags | 0b0000_0100);
            // println!("TIMER FLAG SET - {:x}", self.memory.read(0xFF0F));
        }
    }

    pub fn set_interrupt_queue_bitflag(&mut self, int: Interrupt) {
        self.interrupt_queue_bitflags |= match int {
            Interrupt::VBlank => 1,
            Interrupt::STAT => 2,
            Interrupt::Timer => 4,
            Interrupt::Serial => 8,
            Interrupt::Joypad => 16,
        };
    }

    pub fn get_interrupt_queue_bitflag(&mut self, int: Interrupt) -> bool {
        if (self.interrupt_queue_bitflags & match int {
            Interrupt::VBlank => 1,
            Interrupt::STAT => 2,
            Interrupt::Timer => 4,
            Interrupt::Serial => 8,
            Interrupt::Joypad => 16,
        }) == 0 { false } else { true }
    }

    pub fn clear_interrupt_queue_bitflag(&mut self, int: Interrupt) {
        self.interrupt_queue_bitflags &= match int {
            Interrupt::VBlank => !1,
            Interrupt::STAT => !2,
            Interrupt::Timer => !4,
            Interrupt::Serial => !8,
            Interrupt::Joypad => !16,
        };
    }

    pub fn interrupt_poll(&mut self) { // rewrite this to pop off queue too
        self.set_vblank_flag();
        self.set_tima_flag();
        self.set_stat_flag();
        let interrupt_enable = self.memory.read(0xFFFF);
        let interrupt_flags = self.memory.read(0xFF0F);
        for flag in 0..5 {
            let flag_and = 1 << flag;
            let interrupt = match flag {
                0 => Interrupt::VBlank,
                1 => Interrupt::STAT,
                2 => Interrupt::Timer,
                3 => Interrupt::Serial,
                4 => Interrupt::Joypad,
                _ => unreachable!(),
            };

            if !self.get_interrupt_queue_bitflag(interrupt) && (flag_and & interrupt_flags != 0) { 
                self.interrupt_queue.push(interrupt);
                self.set_interrupt_queue_bitflag(interrupt);
                // println!("INTERRUPT PUSHED");
                self.halted = false;
            }
        }

        let mut ephemeral_buffer: Vec<Interrupt> = Vec::new();
        if self.interrupt_queue.nodes.len() != 0 {
            for flag in 0..self.interrupt_queue.nodes.len() {
                let interrupt = self.interrupt_queue.pop().unwrap(); // pops each one off
                let interrupt_flag = match interrupt {
                    Interrupt::VBlank => 1,
                    Interrupt::STAT => 2,
                    Interrupt::Timer => 4,
                    Interrupt::Serial => 8,
                    Interrupt::Joypad => 16,
                };
                if self.ime && ((interrupt_flag & interrupt_enable) != 0) { // tries it
                    self.handle_interrupt(interrupt); // if allowed, do it
                    self.memory.write(0xFF0F, interrupt_flags & !interrupt_flag);
                } else { ephemeral_buffer.push(interrupt); } // if not allowed, push it into buffer
            }
            for interrupt in ephemeral_buffer.iter() {
                self.interrupt_queue.push(*interrupt); // all disallowed interrupts put back into queue
            }
        }
    }

    pub fn handle_interrupt(&mut self, int: Interrupt) {
        self.m_cycle();
        self.m_cycle(); // 2 wait cycles while control transferred
        self.stack_push(self.pc);
        self.ime = false; // disables interrupts
        self.pc = match int {
            Interrupt::VBlank => 0x40,
            Interrupt::STAT => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        };
        // println!{"HANDLED INTERRUPT - PC = {:x}", self.pc};
        self.clear_interrupt_queue_bitflag(int);
    }

    pub fn fetch(&mut self) -> u8 {
        let addr = self.pc;
        let data = self.read(addr);
        self.pc = self.pc.wrapping_add(1);
        self.m_cycle();
        data
    }

    pub fn fetchW(&mut self) -> u16 {
        let lower_byte = self.fetch() as u16;
        let upper_byte = self.fetch() as u16;
        (upper_byte << 8) | lower_byte 
    }

    pub fn write(&mut self, address: u16, data: u8, ) {
        match address {
            0xFF04..=0xFF07 => {
                self.timer.write_io(address, data);
            },
            _ => { 
                self.memory.write(address, data); 
            },
        }
        if address == 0xFF0F { println!("@ {:x}", self.pc); }
        self.m_cycle();
    }

    pub fn read(&mut self, address: u16) -> u8 {
        let data = match address {
            0xFF00 => {
                let joyp = self.memory.read(0xFF00);
                self.input_states.get_states(joyp)
            },
            0xFF04..=0xFF07 => {
                self.timer.read_io(address)
            },
            _ => { 
                self.memory.read(address)
            },
        };
        self.m_cycle();
        data
    }

    pub fn stack_push(&mut self, num:u16){
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp.into(), (num >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp.into(), (num & 0xFF) as u8);
    }

    pub fn stack_pop(&mut self) -> u16{
        let lower = self.read(self.sp.into()) as u16;
        self.m_cycle();
        self.sp = self.sp.wrapping_add(1);
        let upper = self.read(self.sp.into()) as u16;
        self.m_cycle();
        self.sp = self.sp.wrapping_add(1);
        ((upper << 8) | lower)
    }

    pub fn execute(&mut self, mut opcode:u8) {
        // println!("EXECUTING OPCODE => {:x} @ PC => {}", opcode, self.pc);
        if self.ime_waiting && opcode != 0xFB {
            self.ime = true;
            self.ime_waiting = false;
        }

        if opcode != 0xCB {
            match opcode { // replace with function pointer array
                0x0 => {  },
                0x1 => { self.regW_ld_operand(RegW::BC); },
                0x2 => { self.regWaddr_ld_reg(RegW::BC, Reg::A); },
                0x3 => { self.inc_regW(RegW::BC); },
                0x4 => { self.inc_reg(Reg::B); },
                0x5 => { self.dec_reg(Reg::B); },
                0x6 => { self.reg_ld_operand(Reg::B); },
                0x7 => { self.rlca(); },
                0x8 => { self.addr_ld_sp(); },
                0x9 => { self.regW_add_regW(RegW::HL, RegW::BC); },
                0xa => { self.reg_ld_regWaddr(Reg::A, RegW::BC); },
                0xb => { self.dec_regW(RegW::BC); },
                0xc => { self.inc_reg(Reg::C); },
                0xd => { self.dec_reg(Reg::C); },
                0xe => { self.reg_ld_operand(Reg::C); },
                0xf => { self.rrca(); },
                0x10 => {  },
                0x11 => { self.regW_ld_operand(RegW::DE); },
                0x12 => { self.regWaddr_ld_reg(RegW::DE, Reg::A); },
                0x13 => { self.inc_regW(RegW::DE); },
                0x14 => { self.inc_reg(Reg::D); },
                0x15 => { self.dec_reg(Reg::D); },
                0x16 => { self.reg_ld_operand(Reg::D); },
                0x17 => { self.rla(); },
                0x18 => { self.jr(); },
                0x19 => { self.regW_add_regW(RegW::HL, RegW::DE); },
                0x1a => { self.reg_ld_regWaddr(Reg::A, RegW::DE); },
                0x1b => { self.dec_regW(RegW::DE); },
                0x1c => { self.inc_reg(Reg::E); },
                0x1d => { self.dec_reg(Reg::E); },
                0x1e => { self.reg_ld_operand(Reg::E); },
                0x1f => { self.rra(); },
                0x20 => { self.jr_nf(Flag::Z); },
                0x21 => { self.regW_ld_operand(RegW::HL); },
                0x22 => { self.regWaddr_ld_reg(RegW::HL, Reg::A); self.inc_regW(RegW::HL); },
                0x23 => { self.inc_regW(RegW::HL); },
                0x24 => { self.inc_reg(Reg::H); },
                0x25 => { self.dec_reg(Reg::H); },
                0x26 => { self.reg_ld_operand(Reg::H); },
                0x27 => { self.daa(); },
                0x28 => { self.jr_f(Flag::Z); },
                0x29 => { self.regW_add_regW(RegW::HL, RegW::HL); },
                0x2a => { self.reg_ld_regWaddr(Reg::A, RegW::HL); self.inc_regW(RegW::HL); },
                0x2b => { self.dec_regW(RegW::HL); },
                0x2c => { self.inc_reg(Reg::L); },
                0x2d => { self.dec_reg(Reg::L); },
                0x2e => { self.reg_ld_operand(Reg::L); },
                0x2f => { self.cpl(); },
                0x30 => { self.jr_nf(Flag::C); },
                0x31 => { self.sp_ld_operand(); },
                0x32 => { self.regWaddr_ld_reg(RegW::HL, Reg::A); self.dec_regW(RegW::HL); },
                0x33 => { self.sp = self.sp.wrapping_add(1); },
                0x34 => { self.inc_addr(RegW::HL); },
                0x35 => { self.dec_addr(RegW::HL); },
                0x36 => { self.regWaddr_ld_operand(RegW::HL); },
                0x37 => { self.scf(); },
                0x38 => { self.jr_f(Flag::C); },
                0x39 => { self.regW_add_sp(RegW::HL); },
                0x3a => { self.reg_ld_regWaddr(Reg::A, RegW::HL); self.dec_regW(RegW::HL); },
                0x3b => { self.sp = self.sp.wrapping_sub(1); },
                0x3c => { self.inc_reg(Reg::A); },
                0x3d => { self.dec_reg(Reg::A); },
                0x3e => { self.reg_ld_operand(Reg::A); },
                0x3f => { self.ccf(); },
                0x40 => { self.reg_ld_reg(Reg::B, Reg::B); },
                0x41 => { self.reg_ld_reg(Reg::B, Reg::C); },
                0x42 => { self.reg_ld_reg(Reg::B, Reg::D); },
                0x43 => { self.reg_ld_reg(Reg::B, Reg::E); },
                0x44 => { self.reg_ld_reg(Reg::B, Reg::H); },
                0x45 => { self.reg_ld_reg(Reg::B, Reg::L); },
                0x46 => { self.reg_ld_regWaddr(Reg::B, RegW::HL); },
                0x47 => { self.reg_ld_reg(Reg::B, Reg::A); },
                0x48 => { self.reg_ld_reg(Reg::C, Reg::B); },
                0x49 => { self.reg_ld_reg(Reg::C, Reg::C); },
                0x4a => { self.reg_ld_reg(Reg::C, Reg::D); },
                0x4b => { self.reg_ld_reg(Reg::C, Reg::E); },
                0x4c => { self.reg_ld_reg(Reg::C, Reg::H); },
                0x4d => { self.reg_ld_reg(Reg::C, Reg::L); },
                0x4e => { self.reg_ld_regWaddr(Reg::C, RegW::HL); },
                0x4f => { self.reg_ld_reg(Reg::C, Reg::A); },
                0x50 => { self.reg_ld_reg(Reg::D, Reg::B); },
                0x51 => { self.reg_ld_reg(Reg::D, Reg::C); },
                0x52 => { self.reg_ld_reg(Reg::D, Reg::D); },
                0x53 => { self.reg_ld_reg(Reg::D, Reg::E); },
                0x54 => { self.reg_ld_reg(Reg::D, Reg::H); },
                0x55 => { self.reg_ld_reg(Reg::D, Reg::L); },
                0x56 => { self.reg_ld_regWaddr(Reg::D, RegW::HL); },
                0x57 => { self.reg_ld_reg(Reg::D, Reg::A); },
                0x58 => { self.reg_ld_reg(Reg::E, Reg::B); },
                0x59 => { self.reg_ld_reg(Reg::E, Reg::C); },
                0x5a => { self.reg_ld_reg(Reg::E, Reg::D); },
                0x5b => { self.reg_ld_reg(Reg::E, Reg::E); },
                0x5c => { self.reg_ld_reg(Reg::E, Reg::H); },
                0x5d => { self.reg_ld_reg(Reg::E, Reg::L); },
                0x5e => { self.reg_ld_regWaddr(Reg::E, RegW::HL); },
                0x5f => { self.reg_ld_reg(Reg::E, Reg::A); },
                0x60 => { self.reg_ld_reg(Reg::H, Reg::B); },
                0x61 => { self.reg_ld_reg(Reg::H, Reg::C); },
                0x62 => { self.reg_ld_reg(Reg::H, Reg::D); },
                0x63 => { self.reg_ld_reg(Reg::H, Reg::E); },
                0x64 => { self.reg_ld_reg(Reg::H, Reg::H); },
                0x65 => { self.reg_ld_reg(Reg::H, Reg::L); },
                0x66 => { self.reg_ld_regWaddr(Reg::H, RegW::HL); },
                0x67 => { self.reg_ld_reg(Reg::H, Reg::A); },
                0x68 => { self.reg_ld_reg(Reg::L, Reg::B); },
                0x69 => { self.reg_ld_reg(Reg::L, Reg::C); },
                0x6a => { self.reg_ld_reg(Reg::L, Reg::D); },
                0x6b => { self.reg_ld_reg(Reg::L, Reg::E); },
                0x6c => { self.reg_ld_reg(Reg::L, Reg::H); },
                0x6d => { self.reg_ld_reg(Reg::L, Reg::L); },
                0x6e => { self.reg_ld_regWaddr(Reg::L, RegW::HL); },
                0x6f => { self.reg_ld_reg(Reg::L, Reg::A); },
                0x70 => { self.regWaddr_ld_reg(RegW::HL, Reg::B); },
                0x71 => { self.regWaddr_ld_reg(RegW::HL, Reg::C); },
                0x72 => { self.regWaddr_ld_reg(RegW::HL, Reg::D); },
                0x73 => { self.regWaddr_ld_reg(RegW::HL, Reg::E); },
                0x74 => { self.regWaddr_ld_reg(RegW::HL, Reg::H); },
                0x75 => { self.regWaddr_ld_reg(RegW::HL, Reg::L); },
                0x76 => { self.halted = true; },
                0x77 => { self.regWaddr_ld_reg(RegW::HL, Reg::A); },
                0x78 => { self.reg_ld_reg(Reg::A, Reg::B); },
                0x79 => { self.reg_ld_reg(Reg::A, Reg::C); },
                0x7a => { self.reg_ld_reg(Reg::A, Reg::D); },
                0x7b => { self.reg_ld_reg(Reg::A, Reg::E); },
                0x7c => { self.reg_ld_reg(Reg::A, Reg::H); },
                0x7d => { self.reg_ld_reg(Reg::A, Reg::L); },
                0x7e => { self.reg_ld_regWaddr(Reg::A, RegW::HL); },
                0x7f => { self.reg_ld_reg(Reg::A, Reg::A); },
                0x80 => { self.reg_add_reg(Reg::A, Reg::B); },
                0x81 => { self.reg_add_reg(Reg::A, Reg::C); },
                0x82 => { self.reg_add_reg(Reg::A, Reg::D); },
                0x83 => { self.reg_add_reg(Reg::A, Reg::E); },
                0x84 => { self.reg_add_reg(Reg::A, Reg::H); },
                0x85 => { self.reg_add_reg(Reg::A, Reg::L); },
                0x86 => { self.reg_add_regWaddr(Reg::A, RegW::HL); },
                0x87 => { self.reg_add_reg(Reg::A, Reg::A); },
                0x88 => { self.reg_adc_reg(Reg::A, Reg::B); },
                0x89 => { self.reg_adc_reg(Reg::A, Reg::C); },
                0x8a => { self.reg_adc_reg(Reg::A, Reg::D); },
                0x8b => { self.reg_adc_reg(Reg::A, Reg::E); },
                0x8c => { self.reg_adc_reg(Reg::A, Reg::H); },
                0x8d => { self.reg_adc_reg(Reg::A, Reg::L); },
                0x8e => { self.reg_adc_regWaddr(Reg::A, RegW::HL); },
                0x8f => { self.reg_adc_reg(Reg::A, Reg::A); },
                0x90 => { self.reg_sub_reg(Reg::A, Reg::B); },
                0x91 => { self.reg_sub_reg(Reg::A, Reg::C); },
                0x92 => { self.reg_sub_reg(Reg::A, Reg::D); },
                0x93 => { self.reg_sub_reg(Reg::A, Reg::E); },
                0x94 => { self.reg_sub_reg(Reg::A, Reg::H); },
                0x95 => { self.reg_sub_reg(Reg::A, Reg::L); },
                0x96 => { self.reg_sub_regWaddr(Reg::A, RegW::HL); },
                0x97 => { self.reg_sub_reg(Reg::A, Reg::A); },
                0x98 => { self.reg_sbc_reg(Reg::A, Reg::B); },
                0x99 => { self.reg_sbc_reg(Reg::A, Reg::C); },
                0x9a => { self.reg_sbc_reg(Reg::A, Reg::D); },
                0x9b => { self.reg_sbc_reg(Reg::A, Reg::E); },
                0x9c => { self.reg_sbc_reg(Reg::A, Reg::H); },
                0x9d => { self.reg_sbc_reg(Reg::A, Reg::L); },
                0x9e => { self.reg_sbc_regWaddr(Reg::A, RegW::HL); },
                0x9f => { self.reg_sbc_reg(Reg::A, Reg::A); },
                0xa0 => { self.reg_and_reg(Reg::A, Reg::B); },
                0xa1 => { self.reg_and_reg(Reg::A, Reg::C); },
                0xa2 => { self.reg_and_reg(Reg::A, Reg::D); },
                0xa3 => { self.reg_and_reg(Reg::A, Reg::E); },
                0xa4 => { self.reg_and_reg(Reg::A, Reg::H); },
                0xa5 => { self.reg_and_reg(Reg::A, Reg::L); },
                0xa6 => { self.reg_and_regWaddr(Reg::A, RegW::HL); },
                0xa7 => { self.reg_and_reg(Reg::A, Reg::A); },
                0xa8 => { self.reg_xor_reg(Reg::A, Reg::B); },
                0xa9 => { self.reg_xor_reg(Reg::A, Reg::C); },
                0xaa => { self.reg_xor_reg(Reg::A, Reg::D); },
                0xab => { self.reg_xor_reg(Reg::A, Reg::E); },
                0xac => { self.reg_xor_reg(Reg::A, Reg::H); },
                0xad => { self.reg_xor_reg(Reg::A, Reg::L); },
                0xae => { self.reg_xor_regWaddr(Reg::A, RegW::HL); },
                0xaf => { self.reg_xor_reg(Reg::A, Reg::A); },
                0xb0 => { self.reg_or_reg(Reg::A, Reg::B); },
                0xb1 => { self.reg_or_reg(Reg::A, Reg::C); },
                0xb2 => { self.reg_or_reg(Reg::A, Reg::D); },
                0xb3 => { self.reg_or_reg(Reg::A, Reg::E); },
                0xb4 => { self.reg_or_reg(Reg::A, Reg::H); },
                0xb5 => { self.reg_or_reg(Reg::A, Reg::L); },
                0xb6 => { self.reg_or_regWaddr(Reg::A, RegW::HL); },
                0xb7 => { self.reg_or_reg(Reg::A, Reg::A); },
                0xb8 => { self.reg_cp_reg(Reg::A, Reg::B); },
                0xb9 => { self.reg_cp_reg(Reg::A, Reg::C); },
                0xba => { self.reg_cp_reg(Reg::A, Reg::D); },
                0xbb => { self.reg_cp_reg(Reg::A, Reg::E); },
                0xbc => { self.reg_cp_reg(Reg::A, Reg::H); },
                0xbd => { self.reg_cp_reg(Reg::A, Reg::L); },
                0xbe => { self.reg_cp_regWaddr(Reg::A, RegW::HL); },
                0xbf => { self.reg_cp_reg(Reg::A, Reg::A); },
                0xc0 => { self.ret_nf(Flag::Z); },
                0xc1 => { self.regW_pop_sp(RegW::BC); },
                0xc2 => { self.jp_nf(Flag::Z); },
                0xc3 => { self.jp(); },
                0xc4 => { self.call_nf(Flag::Z); },
                0xc5 => { self.regW_push_sp(RegW::BC); },
                0xc6 => { self.reg_add_operand(Reg::A); },
                0xc7 => { self.rst(0x00); },
                0xc8 => { self.ret_f(Flag::Z); },
                0xc9 => { self.ret(); },
                0xca => { self.jp_f(Flag::Z); },
                0xcb => {  },
                0xcc => { self.call_f(Flag::Z); },
                0xcd => { self.call(); },
                0xce => { self.reg_adc_operand(Reg::A); },
                0xcf => { self.rst(0x08); },
                0xd0 => { self.ret_nf(Flag::C); },
                0xd1 => { self.regW_pop_sp(RegW::DE); },
                0xd2 => { self.jp_nf(Flag::C); },
                0xd3 => {  },
                0xd4 => { self.call_nf(Flag::C); },
                0xd5 => { self.regW_push_sp(RegW::DE); },
                0xd6 => { self.reg_sub_operand(Reg::A); },
                0xd7 => { self.rst(0x10); },
                0xd8 => { self.ret_f(Flag::C); },
                0xd9 => { self.reti(); },
                0xda => { self.jp_f(Flag::C); },
                0xdb => {  },
                0xdc => { self.call_f(Flag::C); },
                0xdd => {  },
                0xde => { self.reg_sbc_operand(Reg::A); },
                0xdf => { self.rst(0x18); },
                0xe0 => { self.u8ff00_ld_reg(); },
                0xe1 => { self.regW_pop_sp(RegW::HL); },
                0xe2 => { self.regff00_ld_reg(); },
                0xe3 => {  },
                0xe4 => {  },
                0xe5 => { self.regW_push_sp(RegW::HL); },
                0xe6 => { self.reg_and_operand(Reg::A); },
                0xe7 => { self.rst(0x20); },
                0xe8 => { self.sp_add_operand(); },
                0xe9 => { self.jp_hl(); },
                0xea => { self.addr_ld_regA(); },
                0xeb => {  },
                0xec => {  },
                0xed => {  },
                0xee => { self.reg_xor_operand(Reg::A); },
                0xef => { self.rst(0x28); },
                0xf0 => { self.reg_ld_u8ff00(); },
                0xf1 => { self.regW_pop_sp(RegW::AF); },
                0xf2 => { self.reg_ld_regff00(); },
                0xf3 => { self.di(); },
                0xf4 => {  },
                0xf5 => { self.regW_push_sp(RegW::AF); },
                0xf6 => { self.reg_or_operand(Reg::A); },
                0xf7 => { self.rst(0x30); },
                0xf8 => { self.hl_ld_spi8(); },
                0xf9 => { self.sp_ld_hl(); },
                0xfa => { self.regA_ld_addr(); },
                0xfb => { self.ei(); },
                0xfc => {  },
                0xfd => {  },
                0xfe => { self.reg_cp_operand(Reg::A); },
                0xff => { self.rst(0x38); },
                _ => {}
            }
        } else if opcode == 0xCB {
            opcode = self.fetch();
            let r8 = opcode & 0b00000111;
            let upper_bits = opcode & 0b11000000;
            if r8 == 6 {
                let src = RegW::HL;
                if upper_bits == 0 {
                    match (opcode & 0b00111000) >> 3 {
                        0 => self.rlc_hl(),
                        1 => self.rrc_hl(),
                        2 => self.rl_hl(),
                        3 => self.rr_hl(),
                        4 => self.sla_hl(),
                        5 => self.sra_hl(),
                        6 => self.swap_hl(),
                        7 => self.srl_hl(),
                        _ => {}
                    }
                } else {
                    let bit = (opcode & 0b00111000) >> 3;
                    match upper_bits {
                        0b01000000 => self.bit_hl(bit),
                        0b10000000 => self.res_hl(bit),
                        0b11000000 => self.set_hl(bit),
                        _ => {}
                    }
                }
            } else {
                let src = match r8 {
                    0 => Reg::B,
                    1 => Reg::C,
                    2 => Reg::D,
                    3 => Reg::E,
                    4 => Reg::H,
                    5 => Reg::L,
                    7 => Reg::A,
                    _ => unreachable!()
                };
                if upper_bits == 0 {
                    match (opcode & 0b00111000) >> 3 {
                        0 => self.rlc(src),
                        1 => self.rrc(src),
                        2 => self.rl(src),
                        3 => self.rr(src),
                        4 => self.sla(src),
                        5 => self.sra(src),
                        6 => self.swap(src),
                        7 => self.srl(src),
                        _ => {}
                    }
                } else {
                    let bit = (opcode & 0b00111000) >> 3;
                    match upper_bits {
                        0b01000000 => self.bit(bit, src),
                        0b10000000 => self.res(bit, src),
                        0b11000000 => self.set(bit, src),
                        _ => {}
                    }
                }
            }
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
        self.m_cycle();
    }

    pub fn hl_ld_spi8(&mut self) {
        let src = self.fetch() as i8 as i16;
        let temp = src as u8;
        let sum = self.sp.wrapping_add_signed((src));
        self.m_cycle();
        self.registers.set_regW(RegW::HL, sum);
        self.registers.set_flag(Flag::Z, false);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, temp.eval_h_add(self.sp as u8, temp));
        self.registers.set_flag(Flag::C, temp.eval_c_add(self.sp as u8, temp));
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

    pub fn sp_ld_operand(&mut self) {
        self.sp = self.fetchW();
    }

    // Load an address with a register value
    pub fn regWaddr_ld_reg(&mut self, dst: RegW, src: Reg) {
        let dst = self.registers.get_regW(dst);
        let src = self.registers.get_reg(src);
        self.write(dst, src);
    }

    pub fn addr_ld_regA(&mut self) {
        let dst = self.fetchW();
        let src = self.registers.get_reg(Reg::A);
        self.write(dst, src);
    }

    pub fn addr_ld_sp(&mut self) {
        let src = self.sp;
        let upper = (src >> 8) as u8;
        let lower = src as u8;
        let dst = self.fetchW();
        self.write(dst, lower);
        self.write(dst.wrapping_add(1), upper);
    }

    // Load an address with operand
    pub fn regWaddr_ld_operand(&mut self, dst: RegW) {
        let addr = self.registers.get_regW(dst);
        let src = self.fetch();
        self.write(addr, src);
    }

    // Load a register with the value at an address
    pub fn reg_ld_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.read(addr);
        self.registers.set_reg(dst, src);
    }

    pub fn regA_ld_addr(&mut self) {
        let dst = self.fetchW();
        let src = self.read(dst);
        self.registers.set_reg(Reg::A, src);
    }

    // LD (ff00+u8)
    pub fn reg_ld_u8ff00(&mut self) {
        let dst = self.fetch() as u16 + 0xFF00;
        let src = self.read(dst);
        // println!("{:x} LOADED TO REG FROM => {:x}", src, dst);
        self.registers.set_reg(Reg::A, src);
    }

    pub fn u8ff00_ld_reg(&mut self) {
        let dst = self.fetch() as u16 + 0xFF00;
        let src = self.registers.get_reg(Reg::A);
        self.write(dst, src);
    }

    pub fn reg_ld_regff00(&mut self) {
        let dst = self.registers.get_reg(Reg::C) as u16 + 0xFF00;
        let src = self.read(dst);
        self.registers.set_reg(Reg::A, src);
    }

    pub fn regff00_ld_reg(&mut self) {
        let dst = self.registers.get_reg(Reg::C) as u16 + 0xFF00;
        let src = self.registers.get_reg(Reg::A);
        self.write(dst, src);
    }

    // INC / DEC
    // Increment a register
    pub fn inc_reg(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_add(1);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum.eval_z());
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, sum.eval_h_add(src, 1));
    }

    pub fn inc_regW(&mut self, dst: RegW) {
        let src = self.registers.get_regW(dst).wrapping_add(1);
        self.registers.set_regW(dst, src);
        self.m_cycle();
    }

    // Decrement a register
    pub fn dec_reg(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(1);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum.eval_z());
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, 1)); // might not work for sub
    }

    pub fn dec_regW(&mut self, dst: RegW) {
        let src = self.registers.get_regW(dst).wrapping_sub(1);
        self.registers.set_regW(dst, src);
    }

    // Inc / Dec a register address
    pub fn inc_addr(&mut self, dst: RegW) {
        let addr = self.registers.get_regW(dst);
        let src = self.read(addr);
        self.write(addr, src.wrapping_add(1));
        self.registers.set_flag(Flag::Z, src.wrapping_add(1).eval_z());
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, src.eval_h_add(src, 1));
    }

    pub fn dec_addr(&mut self, dst: RegW) {
        let addr = self.registers.get_regW(dst);
        let src = self.read(addr);
        self.write(addr, src.wrapping_sub(1));
        self.registers.set_flag(Flag::Z, src.wrapping_sub(1).eval_z());
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, 1)); // might not  work for sub
    }

    // ADD / SUB / ADC / SBC
    // Add two registers
    pub fn reg_add_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let add = self.registers.get_reg(dst);
        let sum = add.wrapping_add(src);
        self.registers.set_reg(dst, sum); 
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, src.eval_h_add(src, add));
        self.registers.set_flag(Flag::C, src.eval_c_add(src, add));
    }

    pub fn regW_add_regW(&mut self, dst: RegW, src: RegW) {
        let src = self.registers.get_regW(src);
        let add = self.registers.get_regW(dst);
        let sum = add.wrapping_add(src);
        self.m_cycle();
        self.registers.set_regW(dst, sum); 
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, src.eval_h_add(src, add));
        self.registers.set_flag(Flag::C, src.eval_c_add(src, add));
    }

    pub fn regW_add_sp(&mut self, dst: RegW) {
        let src = self.registers.get_regW(dst);
        let sum = src.wrapping_add(self.sp);
        self.m_cycle();
        self.registers.set_regW(dst, sum);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, sum.eval_h_add(src, self.sp));
        self.registers.set_flag(Flag::C, sum.eval_c_add(src, self.sp));
    }

    // Add value at regW address to register 
    pub fn reg_add_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let add = self.read(addr);
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_add(add);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, src.eval_h_add(src, add));
        self.registers.set_flag(Flag::C, src.eval_c_add(src, add));
    }

    // Add operand to register
    pub fn reg_add_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let add = self.registers.get_reg(dst);
        let sum = add.wrapping_add(src);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, src.eval_h_add(src, add));
        self.registers.set_flag(Flag::C, src.eval_c_add(src, add));
    }

    pub fn sp_add_operand(&mut self) {
        let add = self.fetch() as i8 as i16;
        let src = self.sp as u8;
        self.sp = self.sp.wrapping_add_signed(add);
        self.m_cycle();
        self.m_cycle();
        self.registers.set_flag(Flag::Z, false);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, src.eval_h_add(src, add as u8));
        self.registers.set_flag(Flag::C, src.eval_c_add(src, add as u8));
    }

    // Sub two registers
    pub fn reg_sub_reg(&mut self, dst: Reg, src: Reg) {
        let sub = self.registers.get_reg(src);
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub);
        self.registers.set_reg(dst, sum); 
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, sub));
        self.registers.set_flag(Flag::C, src.eval_c_sub(src, sub));
    }

    // Sub value at regW address from register 
    pub fn reg_sub_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let sub = self.read(addr);
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub);
        self.registers.set_reg(dst, sum); 
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, sub));
        self.registers.set_flag(Flag::C, src.eval_c_sub(src, sub));
    }

    // Sub operand from register
    pub fn reg_sub_operand(&mut self, dst: Reg) {
        let sub = self.fetch();
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, sub));
        self.registers.set_flag(Flag::C, src.eval_c_sub(src, sub));
    }

    // Adc two registers
    pub fn reg_adc_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let cy = self.registers.get_flag(Flag::C) as u8;
        let add = self.registers.get_reg(dst);
        let sum = add.wrapping_add(src).wrapping_add(cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, (((src & 0x0F) + (add & 0x0F) + cy) & 0x10) != 0);
        self.registers.set_flag(Flag::C, ((src as u16 + add as u16 + cy as u16) & 0x0100) != 0);
    }

    // Adc value at regW address to register
    pub fn reg_adc_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.read(addr);
        let cy = self.registers.get_flag(Flag::C) as u8;
        let add = self.registers.get_reg(dst);
        let sum = add.wrapping_add(src).wrapping_add(cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, (((src & 0x0F) + (add & 0x0F) + cy) & 0x10) != 0);
        self.registers.set_flag(Flag::C, ((src as u16 + add as u16 + cy as u16) & 0x0100) != 0);
    }

    // Adc operand to register
    pub fn reg_adc_operand(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let cy = self.registers.get_flag(Flag::C) as u8;
        let add = self.fetch();
        let sum = src.wrapping_add(add).wrapping_add(cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, (((src & 0x0F) + (add & 0x0F) + cy) & 0x10) != 0);
        self.registers.set_flag(Flag::C, ((src as u16 + add as u16 + cy as u16) & 0x0100) != 0);
    }

    // Sbc two registers
    pub fn reg_sbc_reg(&mut self, dst: Reg, src: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let sub = self.registers.get_reg(src);
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub).wrapping_sub(cy);
        self.registers.set_reg(dst, sum); 
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, (((src & 0x0F).wrapping_sub((sub & 0x0F)).wrapping_sub(cy)) & 0x10) != 0);
        self.registers.set_flag(Flag::C, (((src as u16).wrapping_sub(sub as u16).wrapping_sub(cy as u16)) & 0x0100) != 0);
    }

    // Sbc value at regW address from register
    pub fn reg_sbc_regWaddr(&mut self, dst: Reg, src: RegW) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let addr = self.registers.get_regW(src);
        let sub = self.read(addr);
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub).wrapping_sub(cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, (((src & 0x0F).wrapping_sub((sub & 0x0F)).wrapping_sub(cy)) & 0x10) != 0);
        self.registers.set_flag(Flag::C, (((src as u16).wrapping_sub(sub as u16).wrapping_sub(cy as u16)) & 0x0100) != 0);
    }

    // Sbc operand from register
    pub fn reg_sbc_operand(&mut self, dst: Reg) {
        let cy = self.registers.get_flag(Flag::C) as u8;
        let sub = self.fetch();
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub).wrapping_sub(cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, (((src & 0x0F).wrapping_sub((sub & 0x0F)).wrapping_sub(cy)) & 0x10) != 0);
        self.registers.set_flag(Flag::C, (((src as u16).wrapping_sub(sub as u16).wrapping_sub(cy as u16)) & 0x0100) != 0);
    }

    // AND / OR / XOR / CP
    // And two registers
    pub fn reg_and_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst) & src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, true);
        self.registers.set_flag(Flag::C, false);
    }

    // And register with regW address value
    pub fn reg_and_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.read(addr);
        let sum = self.registers.get_reg(dst) & src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, true);
        self.registers.set_flag(Flag::C, false);
    }

    // And operand with register
    pub fn reg_and_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst) & src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, true);
        self.registers.set_flag(Flag::C, false);
    }

    // Xor two registers
    pub fn reg_xor_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst) ^ src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    } 

    // Xor register with regW address value
    pub fn reg_xor_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.read(addr);
        let sum = self.registers.get_reg(dst) ^ src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    // Xor operand with register
    pub fn reg_xor_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst) ^ src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    // Or two registers
    pub fn reg_or_reg(&mut self, dst: Reg, src: Reg) {
        let src = self.registers.get_reg(src);
        let sum = self.registers.get_reg(dst) | src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    // Or register with regW address value
    pub fn reg_or_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let src = self.read(addr);
        let sum = self.registers.get_reg(dst) | src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    // Or operand with register
    pub fn reg_or_operand(&mut self, dst: Reg) {
        let src = self.fetch();
        let sum = self.registers.get_reg(dst) | src;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    // CP two registers
    pub fn reg_cp_reg(&mut self, dst: Reg, src: Reg) {
        let sub = self.registers.get_reg(src);
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, sub));
        self.registers.set_flag(Flag::C, src < sub);
    }

    // CP value at regW address with register 
    pub fn reg_cp_regWaddr(&mut self, dst: Reg, src: RegW) {
        let addr = self.registers.get_regW(src);
        let sub = self.read(addr);
        let src = self.registers.get_reg(dst);
        let sum = sub.wrapping_sub(src);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, sub));
        self.registers.set_flag(Flag::C, src < sub);
    }

    // CP operand with register
    pub fn reg_cp_operand(&mut self, dst: Reg) {
        let sub = self.fetch();
        let src = self.registers.get_reg(dst);
        let sum = src.wrapping_sub(sub);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, src.eval_h_sub(src, sub));
        self.registers.set_flag(Flag::C, src < sub);
    }

    // SP POP / PUSH to register
    pub fn regW_pop_sp(&mut self, dst: RegW) {
        let src = self.stack_pop();
        // println!("{:x} has been POPPED", src);
        self.registers.set_regW(dst, src);
    }

    pub fn regW_push_sp(&mut self, src: RegW) {
        let src = self.registers.get_regW(src);
        // println!("{:x} has been PUSHED", src);
        self.stack_push(src);
    }

    // Rotates
    // RLC
    pub fn rlc(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src >> 7;
        let sum = ((src << 1) | cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RL
    pub fn rl(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src >> 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        let sum = ((src << 1) | cf);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RLCA
    pub fn rlca(&mut self) {
        let mut src = self.registers.get_reg(Reg::A);
        let cy = src >> 7;
        src <<= 1;
        self.registers.set_reg(Reg::A, (src | cy));
        self.registers.set_flag(Flag::Z, false);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy == 1);
    }

    // RLA
    pub fn rla(&mut self) {
        let mut src = self.registers.get_reg(Reg::A);
        let cy = src >> 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        src <<= 1;
        self.registers.set_reg(Reg::A, (src | cf));
        self.registers.set_flag(Flag::Z, false);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy == 1);
    }

    // RLC (HL)
    pub fn rlc_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let mut src = self.read(addr);
        let cy = src >> 7;
        let sum = ((src << 1) | cy);
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RL (HL)
    pub fn rl_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let mut src = self.read(addr);
        let cy = src >> 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        let sum = ((src << 1) | cf);
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RRC
    pub fn rrc(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src << 7;
        let sum = ((src >> 1) | cy);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RR
    pub fn rr(&mut self, dst: Reg) {
        let mut src = self.registers.get_reg(dst);
        let cy = src << 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        let sum = ((src >> 1) | (cf << 7));
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RRCA
    pub fn rrca(&mut self) {
        let mut src = self.registers.get_reg(Reg::A);
        let cy = src << 7;
        src >>= 1;
        self.registers.set_reg(Reg::A, (src | cy));
        self.registers.set_flag(Flag::Z, false);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RRA
    pub fn rra(&mut self) {
        let mut src = self.registers.get_reg(Reg::A);
        let cy = src << 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        let sum = (src >> 1 | cf << 7);
        self.registers.set_reg(Reg::A, sum);
        self.registers.set_flag(Flag::Z, false);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RRC (HL)
    pub fn rrc_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let mut src = self.read(addr);
        let cy = src << 7;
        let sum = ((src >> 1) | cy);
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // RR (HL)
    pub fn rr_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let mut src = self.read(addr);
        let cy = src << 7;
        let cf = self.registers.get_flag(Flag::C) as u8;
        let sum = ((src >> 1) | (cf << 7));
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // Shifts
    // SLA
    pub fn sla(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let cy = src >> 7;
        let sum = src << 1;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    pub fn sla_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let src = self.read(addr);
        let cy = src >> 7;
        let sum = src << 1;
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // SRA
    pub fn sra(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let msb = src >> 7;
        let cy = src & 0x1;
        let sum = (src >> 1 | msb << 7);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    pub fn sra_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let src = self.read(addr);
        let msb = src >> 7;
        let cy = src & 0x1;
        let sum = (src >> 1 | msb << 7);
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    // SRL
    pub fn srl(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let cy = src & 0x1;
        let sum = src >> 1;
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);
    }

    pub fn srl_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let src = self.read(addr);
        let cy = src << 7;
        let sum = src >> 1;
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, cy != 0);     
    }

    // Swap
    pub fn swap(&mut self, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let sum = (src >> 4) | (src << 4);
        self.registers.set_reg(dst, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    pub fn swap_hl(&mut self) {
        let addr = self.registers.get_regW(RegW::HL);
        let src = self.read(addr);
        let sum = (src >> 4) | (src << 4);
        self.write(addr, sum);
        self.registers.set_flag(Flag::Z, sum == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, false);
    }

    pub fn bit(&mut self, pos: u8, dst: Reg) {
        let src = self.registers.get_reg(dst);
        let bit = (src >> pos) & 0b00000001;
        self.registers.set_flag(Flag::Z, bit == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, true);
    }

    pub fn bit_hl(&mut self, pos: u8) {
        let addr = self.registers.get_regW(RegW::HL);
        let src = self.read(addr);
        let bit = (src >> pos) & 0b00000001;
        self.registers.set_flag(Flag::Z, bit == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, true);
    }

    pub fn res(&mut self, pos: u8, dst: Reg) {
        let mask = !(0x01 << pos);
        let src = self.registers.get_reg(dst);
        self.registers.set_reg(dst, (src & mask));
    }

    pub fn res_hl(&mut self, pos: u8) {
        let addr = self.registers.get_regW(RegW::HL);
        let mask = !(0x01 << pos);
        let src = self.read(addr);
        self.write(addr, (src & mask));
    }

    pub fn set(&mut self, pos: u8, dst: Reg) {
        let mask = 0x01 << pos;
        let src = self.registers.get_reg(dst);
        self.registers.set_reg(dst, (src | mask));
    }

    pub fn set_hl(&mut self, pos: u8) {
        let addr = self.registers.get_regW(RegW::HL);
        let mask = 0x01 << pos;
        let src = self.read(addr);
        self.write(addr, (src | mask));
    }    

    // Misc
    // CPL / Compliment register A
    pub fn cpl(&mut self) {
        let src = self.registers.get_reg(Reg::A);
        self.registers.set_reg(Reg::A, !src);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, true);
    }

    // CCF / Compliment carry flag
    pub fn ccf(&mut self) {
        if self.registers.get_flag(Flag::C) {
            self.registers.set_flag(Flag::C, false);
        } else { self.registers.set_flag(Flag::C, true); }
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
    }

    // SCF / Set carry flag
    pub fn scf(&mut self) {
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, false);
        self.registers.set_flag(Flag::C, true);
    }

    // DAA / Decimal encoding
    pub fn daa(&mut self) {
        let mut correction = 0;
        let src = self.registers.get_reg(Reg::A);
        if (self.registers.get_flag(Flag::H)) || 
        !self.registers.get_flag(Flag::N) && ((src & 0x0F) > 0x09) {
            correction |= 0x06;
        }
        if (self.registers.get_flag(Flag::C)) || 
        !self.registers.get_flag(Flag::N) && ((src & 0xFF) > 0x99) {
            correction |= 0x60;
            self.registers.set_flag(Flag::C, true);
        }
        if !self.registers.get_flag(Flag::N) {
            self.registers.set_reg(Reg::A, src.wrapping_add(correction));
            self.registers.set_flag(Flag::Z, src.wrapping_add(correction) == 0);
        } else { 
            self.registers.set_reg(Reg::A, src.wrapping_sub(correction));
            self.registers.set_flag(Flag::Z, src.wrapping_sub(correction) == 0);
        }
        self.registers.set_flag(Flag::H, false);
    }

    // JP / JR
    // Relative Jumps
    // jr
    pub fn jr(&mut self) {
        let r_pos = self.fetch() as i8;
        self.pc = self.pc.wrapping_add_signed(r_pos.into());
        self.m_cycle();
    }

    // If not flag
    pub fn jr_nf(&mut self, f: Flag) {
        let r_pos = self.fetch() as i8;
        if !self.registers.get_flag(f) {
            self.pc = self.pc.wrapping_add_signed(r_pos.into());
            self.m_cycle();
        }
    }

    // If flag
    pub fn jr_f(&mut self, f: Flag) {
        let r_pos = self.fetch() as i8;
        if self.registers.get_flag(f) {
            self.pc = self.pc.wrapping_add_signed(r_pos.into());
            self.m_cycle();
        }
    }

    // Jumps
    // jp
    pub fn jp(&mut self) {
        let pos = self.fetchW();
        self.m_cycle();
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
            self.m_cycle();
        }
    }

    // If flag
    pub fn jp_f(&mut self, f: Flag) {
        let pos = self.fetchW();
        if self.registers.get_flag(f) {
            self.pc = pos;
            self.m_cycle();
        }
    }

    // RET / Return
    pub fn ret(&mut self) {
        self.pc = self.stack_pop();
        self.m_cycle();
    }

    // If not flag
    pub fn ret_nf(&mut self, f: Flag) {
        self.m_cycle();
        if !self.registers.get_flag(f) {
            self.pc = self.stack_pop();
        }
    }

    // If flag
    pub fn ret_f(&mut self, f: Flag) {
        self.m_cycle();
        if self.registers.get_flag(f) {
            self.pc = self.stack_pop();
        }
    }

    pub fn reti(&mut self) {
        self.pc = self.stack_pop();
        self.m_cycle();
        self.ime = true;
    }
    
    // Calls
    // CALL
    pub fn call(&mut self) {
        let pos = self.fetchW();
        self.stack_push(self.pc);
        self.pc = pos;
        self.m_cycle();
    }

    // If not flag
    pub fn call_nf(&mut self, f: Flag) {
        let pos = self.fetchW();
        if !self.registers.get_flag(f) {
            self.stack_push(self.pc);
            self.pc = pos;
            self.m_cycle();
        }
    }

    // If flag
    pub fn call_f(&mut self, f: Flag) {
        let pos = self.fetchW();
        if self.registers.get_flag(f) {
            self.stack_push(self.pc);
            self.pc = pos;
            self.m_cycle();
        }
    }

    // RST - Reset
    pub fn rst(&mut self, rst: u16) {
        self.stack_push(self.pc);
        self.m_cycle();
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