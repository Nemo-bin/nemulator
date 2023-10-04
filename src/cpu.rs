use crate::memory::Memory;
use crate::registers::Registers;

const KIB:usize = 1024;

#[derive(Copy, Clone)]
pub struct CPU {
    pub halted:bool,
    pub ime:bool,
    pub registers:Registers,
    pub memory:Memory,
    pub pc:u16,
    pub sp:u16,
}

impl CPU{
    pub fn new() -> Self {
        CPU{
            halted:false,
            ime:false,
            registers:Registers::new(),
            memory:Memory::new(),
            pc:0x100,
            sp:0xFFFE,
        }
    }

    pub fn fetch(&mut self) -> u8 {
        let addr = self.pc;
        let data = self.memory.read(addr);
        self.pc += 1;
        data
    }

   // izik1.github.io/gbops/index.html
    pub fn execute(&mut self, opcode:u8){}
}