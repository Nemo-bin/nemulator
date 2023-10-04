pub enum Reg {
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
}

pub enum RegW {
    AF,
    BC,
    DE,
    HL,
}

#[derive(Copy, Clone)]
pub struct Registers {
    pub A:u8,
    pub F:u8,
    pub B:u8,
    pub C:u8,
    pub D:u8,
    pub E:u8,
    pub H:u8,
    pub L:u8,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            A:0x01,
            F:0xB0,
            B:0x00,
            C:0x13,
            D:0x00,
            E:0xD8,
            H:0x01,
            L:0x4D,
        }
    }

    pub fn get_reg(&self, src: Reg) -> u8 {
        match src {
            Reg::A => self.A,
            Reg::F => self.F,
            Reg::B => self.B,
            Reg::C => self.C,
            Reg::D => self.D,
            Reg::E => self.E,   
            Reg::H => self.H,
            Reg::L => self.L,       
        }
    }

    pub fn get_regW(&self, src: RegW) -> u16 {
        match src {
            RegW::AF => { (self.A as u16) << 8 | self.F as u16 }
            RegW::BC => { (self.B as u16) << 8 | self.C as u16 }
            RegW::DE => { (self.D as u16) << 8 | self.E as u16 }
            RegW::HL => { (self.H as u16) << 8 | self.L as u16 }
        }
    }

    pub fn set_reg(&mut self, dst: u8, src: Reg) {
        match src {
            Reg::A => { self.A = dst },
            Reg::F => { self.F = dst },
            Reg::B => { self.B = dst },
            Reg::C => { self.C = dst },
            Reg::D => { self.D = dst },
            Reg::E => { self.E = dst },   
            Reg::H => { self.H = dst },
            Reg::L => { self.L = dst },       
        };
    }

    pub fn set_regW(&mut self, dst: u16, src: RegW) {
        match src {
            RegW::AF => { self.A = (dst >> 8) as u8; self.F = (dst << 8) as u8; },
            RegW::BC => { self.B = (dst >> 8) as u8; self.C = (dst << 8) as u8; },
            RegW::DE => { self.D = (dst >> 8) as u8; self.E = (dst << 8) as u8; },
            RegW::HL => { self.H = (dst >> 8) as u8; self.L = (dst << 8) as u8; },
        };
    }
}