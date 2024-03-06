// Code for managing timer registers etc.
pub struct Timer {
    sysclk: u16,

    tima: u8,
    tma: u8,
    tac: u8,

    last_bit: u8,

    tima_reload_cycle: bool,
    tima_cycles_to_irq: u8,
    pub tima_overflow_irq: bool,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            sysclk: 0,

            tima: 0,
            tma: 0,
            tac: 0,

            last_bit: 0,

            tima_reload_cycle: false,
            tima_cycles_to_irq: 0,
            tima_overflow_irq: false,
        }
    }

    
    pub fn sysclk_change(&mut self, new_sysclk: u16) {
        self.sysclk = new_sysclk;
        let clock_speed = self.tac & 0b0000_0011;

        let mut current_bit = match clock_speed {
            0 => (self.sysclk >> 9) & 1,
            3 => (self.sysclk >> 7) & 1,
            2 => (self.sysclk >> 5) & 1,
            1 => (self.sysclk >> 3) & 1,
            _ => unreachable!(),
        } as u8; // get bit of sysclk

        current_bit = current_bit & ((self.tac & 4) >> 2); // AND with TIMA Enabled bit in TAC
        self.fallen_edge(self.last_bit, current_bit);
        self.last_bit = current_bit;
    }

    fn fallen_edge(&mut self, before: u8, after: u8) {
        if (before == 1) && (after == 0) {
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0 {
                self.tima_cycles_to_irq = 1;
            }
        }
    }

    pub fn inc_sysclk(&mut self) {
        self.tima_reload_cycle = false;
        if self.tima_cycles_to_irq > 0 {
            self.tima_cycles_to_irq -= 1;
            if self.tima_cycles_to_irq == 0 {
                self.tima_overflow_irq = true;
                self.tima = self.tma;
                self.tima_reload_cycle = true;
            }
        } // upon overflow, there is a 1 m-cycle delay till the irq is made

        self.sysclk_change(self.sysclk.wrapping_add(4)); // 4 t-cycles as my cpu stores t-cycles but incs at each M 
    }

    pub fn write_io(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF04 => { // rDIV
                self.sysclk_change(0); 
            }, // writes to rDIV reset sysclk

            0xFF05 => { // TIMA
                if !self.tima_reload_cycle { self.tima = val };
                if self.tima_cycles_to_irq == 1 { self.tima_cycles_to_irq = 0 }; // during the strange cycle, writes
            }, // writes to TIMA reset the irq and prevent it from being called

            0xFF06 => { // TMA
                if self.tima_reload_cycle { self.tima = val; };
                self.tma = val;
            },
            0xFF07 => { // TAC
                let last_bit = self.last_bit;
                self.last_bit = self.last_bit & ((val & 4) >> 2);

                self.fallen_edge(last_bit, self.last_bit);
                self.tac = val;
            }
            _ => unreachable!(),
        }
    }

    pub fn read_io(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF04 => (self.sysclk >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => unreachable!(),
        }
    }
}