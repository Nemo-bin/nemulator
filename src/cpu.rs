use crate::memory::Memory;
use crate::registers::Registers;

const KIB:usize = 1024;

#[derive(Copy, Clone)]
pub struct CPU {
    pub halted:bool,
    pub ime:bool,
    pub reg:Registers,
    pub memory:Memory,
}

impl CPU{
    pub fn new() -> Self {
        CPU{
            halted:false,
            ime:false,
            reg:Registers::new(),
            memory:Memory::new(),
        }
    }

    pub fn stack_push(mut self, num:u16){
        self.memory.write(self.reg.sp.into(), (num >> 8) as u8);
        self.reg.sp = self.reg.sp.wrapping_sub(1);
        self.memory.write(self.reg.sp.into(), (num & 0xFF) as u8);
        self.reg.sp = self.reg.sp.wrapping_sub(1);
    }

    pub fn stack_pop(mut self) -> u16{
        let lower = self.memory.read(self.reg.sp.into()) as u16;
        self.reg.sp = self.reg.sp.wrapping_add(1);
        let upper = self.memory.read(self.reg.sp.into()) as u16;
        self.reg.sp = self.reg.sp.wrapping_add(1);
        ((upper << 8) | lower)
    }

    pub fn fetch(&self) -> u8 {
        let addr = self.reg.pc;
        let opcode = self.memory.read(addr);
        opcode
    }

   // izik1.github.io/gbops/index.html
    // Pls write func for get u16 from mem
    pub fn execute(&mut self, opcode:u8){}
}