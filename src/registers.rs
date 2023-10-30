#[derive(Copy, Clone)]
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

#[derive(Copy, Clone)]
pub enum RegW {
    AF,
    BC,
    DE,
    HL,
}

pub enum Flag {
    Z,
    N,
    C,
    H,
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

trait SupportedDataType {}
impl SupportedDataType for u8 {}
impl SupportedDataType for u16 {}

trait Eval {
    type GenericInteger: SupportedDataType;
    fn eval_z(&self) -> bool;
    fn eval_c(&self, a: Self::GenericInteger, b: Self::GenericInteger) -> bool;
    fn eval_h(&self, a: Self::GenericInteger, b: Self::GenericInteger) -> bool;
}

impl Eval for u8 {
    type GenericInteger = u8;

    fn eval_z(&self) -> bool {
        if *self == 0 {
            true
        } else { false }
    }

    fn eval_c(&self, a: u8, b: u8) -> bool {
        if (a & 0xeF == 0x80) && (b & 0xeF == 0x80) {
            true
        } else { false }
    }

    fn eval_h(&self, a: u8, b: u8) -> bool {
        let sum = (a & 0xf) + (b & 0xf);
        if (sum & 0x10 == 0x10) {
            true
        } else { false }
    }

}

impl Eval for u16 {
    type GenericInteger = u16;

    fn eval_z(&self) -> bool {
        if *self == 0 {
            true
        } else { false }
    }

    fn eval_c(&self, a: u16, b: u16) -> bool {
        if (a & 0xeFFF == 0x8000) && (b & 0xe7FFF == 0x8000) {
            true
        } else { false }
    }

    fn eval_h(&self, a: u16, b: u16) -> bool {
        let sum = (a & 0xfff) + (b & 0xfff);
        if (sum & 0x1000 == 0x1000) {
            true
        } else { false }
    }
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
            RegW::AF => { (self.A as u16) << 8 | self.F as u16 },
            RegW::BC => { (self.B as u16) << 8 | self.C as u16 },
            RegW::DE => { (self.D as u16) << 8 | self.E as u16 },
            RegW::HL => { (self.H as u16) << 8 | self.L as u16 },
        }
    }

    pub fn set_reg(&mut self, dst: Reg, src: u8) {
        match dst {
            Reg::A => { self.A = src },
            Reg::F => { self.F = src },
            Reg::B => { self.B = src },
            Reg::C => { self.C = src },
            Reg::D => { self.D = src },
            Reg::E => { self.E = src },   
            Reg::H => { self.H = src },
            Reg::L => { self.L = src },       
        };
    }

    pub fn set_regW(&mut self, dst: RegW, src: u16) {
        match dst {
            RegW::AF => { self.A = (src >> 8) as u8; self.F = src as u8; },
            RegW::BC => { self.B = (src >> 8) as u8; self.C = src as u8; },
            RegW::DE => { self.D = (src >> 8) as u8; self.E = src as u8; },
            RegW::HL => { self.H = (src >> 8) as u8; self.L = src as u8; },
        };
    }

    pub fn get_flag(&self, f: Flag) -> bool {
        let src = match f {
            Flag::Z => 0b10000000,
            Flag::N => 0b01000000,
            Flag::C => 0b00100000,
            Flag::H => 0b00010000,
        };

        if self.F & src != 0 {
            true
        } else { false }
    }

    pub fn set_flag(&mut self, f: Flag, set: bool) {
        let src = match f {
            Flag::Z => 0b10000000,
            Flag::N => 0b01000000,
            Flag::H => 0b00100000,
            Flag::C => 0b00010000,
        };

        if set {
            self.F |= src;
        } else { self.F &= !src; }
    }
}