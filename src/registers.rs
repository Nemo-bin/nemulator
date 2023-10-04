#[derive(Copy, Clone)]
pub struct Registers{
    pub  a:u8,
    pub f:u8,
    pub b:u8,
    pub c:u8,
    pub d:u8,
    pub e:u8,
    pub h:u8,
    pub l:u8,
    pub pc:u16,
    pub sp:u16,
}

impl Registers{
    pub fn new() -> Registers{
        Registers{
            a:0x01,
            f:0xB0,
            b:0x00,
            c:0x13,
            d:0x00,
            e:0xD8,
            h:0x01,
            l:0x4D,
            pc:0x0100,
            sp:0xFFFE,
        }
    }

    pub fn af(&self) -> u16{
        ((self.a as u16) << 8) | ((self.f & 0xF0) as u16)
    }
    pub fn bc(&self) -> u16{
        ((self.b as u16) << 8) | (self.c as  u16)
    }
    pub fn de(&self) -> u16{
        ((self.d as u16) << 8) | (self.e as u16)
    }
    pub fn hl(&self) -> u16{
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_af(&mut self, param:u16){
        self.a = (param >> 8) as u8;
        self.f = (param & 0x00F0) as u8;
    }
    pub fn set_bc(&mut self, param:u16){
        self.b = (param >> 8) as u8;
        self.c = (param & 0x00FF) as u8;
    }
    pub fn set_de(&mut self, param:u16){
        self.d = (param >> 8) as u8;
        self.e = (param & 0x00FF) as u8;
    }
    pub fn set_hl(&mut self, param:u16){
        self.h = (param >> 8) as u8;
        self.l = (param & 0x00FF) as u8;
    }

    // f = znhc0000
    pub fn set_z_flag(&mut self, param:u16){
        if param == 0 {self.f |= 0b10000000;}
        else {self.f &= !0b10000000;}
    }
    pub fn set_n_flag(&mut self, param:u8){
        if param == 0 {self.f &= !0b01000000;} 
            else {self.f |= 0b01000000;}
    }
    pub fn set_h_flag(&mut self, param:u8){
        if param == 0 {self.f &= !0b00100000;} 
            else {self.f |= 0b00100000;}
    }
    pub fn set_c_flag(&mut self, param:u8){
        if param == 0 {self.f &= !0b00010000;} 
            else {self.f |= 0b00010000;}
    }

    pub fn get_z_flag(&self) -> u8{
        (self.f & 0b10000000) >> 7
    }
    pub fn get_n_flag(&self) -> u8{
        (self.f & 0b01000000) >> 6
    }
    pub fn get_h_flag(&self) -> u8{
        (self.f & 0b00100000) >> 5
    }
    pub fn get_c_flag(&self) -> u8{
        (self.f & 0b00010000) >> 4
    }
}