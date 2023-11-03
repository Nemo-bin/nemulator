mod cpu;
mod memory;
mod registers;

use std::env;
use std::thread;
use std::time::Duration;
use cpu::CPU;

const GB_WIDTH:usize = 160;
const GB_HEIGHT:usize = 144;

fn main() {

    let args:Vec<String> = env::args().collect();
    let filename = &args[1];

    let mut cpu = CPU::new();
    cpu.memory.load_rom(filename);

    let running = true;
    while running {
        if cpu.memory.read(0xff02) == 0x81 {
            println!("{:x}", cpu.memory.read(0xff01));
        }
        println!("{:x}:{:x} - {:x?}", cpu.pc, cpu.sp, cpu.registers);

        thread::sleep(Duration::from_millis(0));
        let opcode = cpu.fetch();
        cpu.execute(opcode);
    }
}
