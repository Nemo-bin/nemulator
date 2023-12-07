mod cpu;
mod memory;
mod registers;
mod ppu;

use std::{
    io::{
        BufReader,
        prelude::*,
        Write,
    },
    fs::{
        File,
        OpenOptions,
    },
    env,
    thread,
    time::Duration,
};

use sdl2::keyboard::Keycode;
use sdl2::event::Event;

use cpu::CPU;
use registers::Reg;
use ppu::PPU;

const GB_WIDTH:u32 = 160;
const GB_HEIGHT:u32 = 144;
const SCALE:u32 = 3;

fn main() {

    /////////////////////////////// ARGUMENTS ///////////////////////////////

    let args:Vec<String> = env::args().collect();
    let filename = &args[1];
    if args.len() == 3 {
        let blargg_log_number = format!("blarggs_logs/{}", &args[2]);
    }

    ///////////////////////////////// "MAIN" /////////////////////////////////

    let mut cpu = CPU::new();
    cpu.memory.load_rom(filename);

    let mut ppu = PPU::new(&cpu);

    /*
    fs::remove_file("logfiles/logfile.log").expect("removal failed");
    let mut logfile = File::create("logfiles/logfile.log").expect("creation failed");  

    let mut logfile = OpenOptions::new()
        .append(true)
        .open("logfiles/logfile.log")
        .expect("cannot open file");

    let mut blargg_log = File::open(blargg_log_number).expect("Failed to open file");
    let blargg_log_content = BufReader::new(blargg_log);

    let blargg_log_lines: Vec<String> = blargg_log_content
        .lines()
        .map(|line| line.expect("Something went wrong"))
        .collect();

    let mut line_index:usize = 0;
    */

    let mut running = true;
    while running {
        /* if cpu.memory.read(0xff02) == 0x81 {
            println!("{:x}", cpu.memory.read(0xff01));
        }*/

        /*
        thread::sleep(Duration::from_millis(0));
        let log_line = format!(
                "A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X} ({:02X} {:02X} {:02X} {:02X})\n",
                cpu.registers.get_reg(Reg::A),
                cpu.registers.get_reg(Reg::F),
                cpu.registers.get_reg(Reg::B),
                cpu.registers.get_reg(Reg::C),
                cpu.registers.get_reg(Reg::D),
                cpu.registers.get_reg(Reg::E),
                cpu.registers.get_reg(Reg::H),
                cpu.registers.get_reg(Reg::L),
                cpu.sp,
                cpu.pc,
                cpu.memory.read(cpu.pc),
                cpu.memory.read(cpu.pc + 1),
                cpu.memory.read(cpu.pc + 2),
                cpu.memory.read(cpu.pc + 3)
            );

        if cpu.pc > 100 {
            if (log_line.trim() != blargg_log_lines[line_index].trim()) {
                println!("{}", log_line);
                println!("{}", blargg_log_lines[line_index]);
                println!("Opcode {:x} is potentially erroneous", cpu.memory.read(cpu.pc.wrapping_sub(1)));
                panic!("Logs are not equal on line {}", line_index);
            }
            logfile.write(log_line.as_bytes());
            line_index += 1;
        }
        */

        // println!("{}", log_line);

        for event in ppu.renderer.event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => { running = false; },
                _ => {},
            }
        }

        let opcode = cpu.fetch();
        cpu.execute(opcode);
        ppu.draw_frame(&cpu);
        // print!("{:?}", ppu.renderer.displaybuffer);
    }
}
