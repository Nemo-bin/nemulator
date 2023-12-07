use std::fs::File;
use std::io::prelude::*;

const KIB:usize = 1024;

#[derive(Copy, Clone)]
pub struct Memory{
    // MBC registers
    // pub mbc:u8,
    // pub ram_enabled:bool,
    // pub rom_bank_number:u8,
    // pub ram_bank_number:u8,
    // pub banking_mode_select:u8,
    // Memory 
    pub rom_bank_0:[u8; 16*KIB], // 0000 -> 3FFF | From cartridge, fixed
    pub rom_bank_n:[u8; 496*KIB], // 4000 -> 7FFF | From cartridge, switchable
    pub vram:[u8; 8*KIB], // 8000 -> 9FFF | VRAM
    pub extern_ram:[u8; 8*KIB], // A000 -> BFFF | In cartridge, switchable if any
    pub ram_bank_0:[u8; 4*KIB], // C000 -> CFFF | Work ram
    pub ram_bank_1:[u8; 4*KIB], // D000 -> DFFF | Work ram, bank 1 (switchable in CGB)
    pub mirror:[u8; 0xFDFF- 0xE000 + 1], // E000 -> FDFF | Mirror of C000 -> DDFF | Echo RAM, typically unused
    pub oam:[u8; 0xFE9F - 0xFE00 + 1], // FE00 -> FE9F | Sprite attribute table (OAM)
    // FEA0 -> FEFF Unusable
    pub io_registers:[u8; 0xFF7F - 0xFF00 + 1], // FF00 -> FF7F | I/O Registers
    pub hram:[u8; 0xFFFE - 0xFF80 + 1], // FF80 -> FFFE | High RAM
    pub ie_register:[u8; 1] // FFFF -> FFFF | Interrupt enable register (IE)
}

impl Memory{
    pub fn new() -> Memory{
        Memory{
            // mbc:0,
            // ram_enabled:false,
            // rom_bank_number:1,
            // ram_bank_number:0,
            // banking_mode_select:0,
            rom_bank_0:[0; 16*KIB],
            rom_bank_n:[0; 496*KIB], 
            vram:[0; 8*KIB], 
            extern_ram:[0; 8*KIB], 
            ram_bank_0:[0; 4*KIB], 
            ram_bank_1:[0; 4*KIB], 
            mirror:[0; 0xFDFF- 0xE000 + 1],
            oam:[0; 0xFE9F - 0xFE00 + 1],
            io_registers:[0; 0xFF7F - 0xFF00 + 1], // Might need to un array this as io registers can have special behaviour
            hram:[0; 0xFFFE - 0xFF80 + 1],
            ie_register:[0; 1] 
        }
    }

    pub fn load_rom(&mut self, filename:&str){
        let mut f = File::open(filename).expect("Unable to open file!");
        let mut buffer = [0u8; 512*KIB];

        f.read(&mut buffer);
        for i in 0..(32*KIB){
            if i < (16*KIB){
                self.rom_bank_0[i] = buffer[i]
            } else { self.rom_bank_n[i - 16*KIB] = buffer[i]}
        }
    }

    // pub fn set_mbc(&mut self) -> u8{
    //     self.read(0x147)
    // }

    pub fn write(&mut self, address:u16, data:u8) {
        // println!("WRITING @ {:x}", address);
        let location = match address {
            // MBC registers
            // 0x0000..=0x1FFF => { if data == 0x0a { self.ram_enabled = true } },
            // 0x2000..=0x3FFF => { self.rom_bank_number = (data & 0x1F); println!("ROM CHANGED - {}", self.rom_bank_number); },
            // 0x4000..=0x5FFF => {},
            // 0x6000..=0x7FFF => {},
            // Memory writes
            0x8000..=0x9FFF => { self.vram[address as usize - 0x8000] = data },
            0xA000..=0xBFFF => { self.extern_ram[address as usize - 0xA000] = data },
            0xC000..=0xCFFF => { self.ram_bank_0[address as usize - 0xC000] = data },
            0xD000..=0xDFFF => { self.ram_bank_1[address as usize - 0xD000] = data },
            0xE000..=0xFDFF => { self.mirror[address as usize - 0xE000] = data },
            0xFE00..=0xFE9F => { self.oam[address as usize - 0xFE00] = data },
            0xFF01 => { if self.read(0xff02) == 0x81 {
                print!("{}", (data as u8) as char)
            } else { self.io_registers[address as usize - 0xFF00] = data; } },
            0xFF00..=0xFF7F => { self.io_registers[address as usize - 0xFF00] = data; },
            0xFF80..=0xFFFE => { self.hram[address as usize - 0xFF80] = data },
            0xFFFF => { self.ie_register[0] = data },
            _ => { println!("INVALID ADDRESS WRITE @ {:x}",address) }
        };
    }

    pub fn read(&self, address:u16) -> u8 {
        //let offset = if (self.rom_bank_number > 0) && (self.mbc != 0) { 
        //    0x3FFF*((self.rom_bank_number-1) as u16)
        // } 
        //else { 0 };
        if address == 0xFF44 {
            let data = 0x90;
            return data;
        }
        else {
            let data = match address {
                0..=0x3FFF => self.rom_bank_0[address as usize],
                0x4000..=0x7FFF => self.rom_bank_n[(address as usize - 0x4000)],
                0x8000..=0x9FFF => self.vram[address as usize - 0x8000],
                0xA000..=0xBFFF => self.extern_ram[address as usize - 0xA000],
                0xC000..=0xCFFF => self.ram_bank_0[address as usize - 0xC000],
                0xD000..=0xDFFF => self.ram_bank_1[address as usize - 0xD000],
                0xE000..=0xFDFF => self.mirror[address as usize - 0xE000],
                0xFE00..=0xFE9F => self.oam[address as usize - 0xFE00],
                0xFF00..=0xFF7F => self.io_registers[address as usize - 0xFF00],
                0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80],
                0xFFFF => self.ie_register[0],
                _ => { println!("INVALID ADDRESS READ @ {:x}",address); 0u8 }
            };
            return data;
        }
    }
}