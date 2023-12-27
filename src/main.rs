mod cpu;
mod memory;
mod registers;
mod ppu;
mod timer;

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

use backtrace::*;

use cpu::CPU;
use registers::Reg;
use ppu::PPU;
use memory::Memory;

const GB_WIDTH:u32 = 160;
const GB_HEIGHT:u32 = 144;
const SCALE:u32 = 3;

macro_rules! box_arr {
    ($t:expr; $size:expr) => {
        vec![$t; $size].into_boxed_slice().try_into().unwrap()
    };
}

// let arr: Box<[u8; 512]> = box_arr![0; 512];

fn main() {

    /////////////////////////////// BACKTRACE ///////////////////////////////

    /*backtrace::trace(|frame| {
        let ip = frame.ip();
        let symbol_address = frame.symbol_address();

        // Resolve this instruction pointer to a symbol name
        backtrace::resolve_frame(frame, |symbol| {
            if let Some(name) = symbol.name() {
                println!("{} \n", name);
            }
            if let Some(filename) = symbol.filename() {
                println!("{} \n", filename.display());
            }
        });

        true // keep going to the next frame
    });*/

    println!("Size of memory: {}", std::mem::size_of::<Memory>());
    println!("Size of ppu: {}", std::mem::size_of::<PPU>());
    println!("Size of cpu: {}", std::mem::size_of::<CPU>());

    /////////////////////////////// ARGUMENTS ///////////////////////////////

    let args:Vec<String> = env::args().collect();
    let filename = &args[1];
    if args.len() == 3 {
        let blargg_log_number = format!("blarggs_logs/{}", &args[2]);
    }

    ///////////////////////////////// "MAIN" /////////////////////////////////

    let mut cpu = CPU::new();
    println!("CREATED CPU");
    cpu.memory.load_rom(filename);
    println!("LOADED ROM");

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

        for event in cpu.ppu.renderer.event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => { running = false; },
                Event::KeyDown { keycode: Some(Keycode::T), .. } => { 
                    let mut vram_pointer = 0x9800;
                    while vram_pointer <= 0x9FFF {
                        print!("{:x} / ", cpu.memory.read(vram_pointer));
                        vram_pointer += 1;
                    }
                },
                Event::KeyDown { keycode: Some(Keycode::Y), .. } => { 
                    let mut vram_pointer = 0x8000;
                    while vram_pointer <= 0x97FF {
                        print!("{:x} / ", cpu.memory.read(vram_pointer));
                        vram_pointer += 1;
                    }
                },
                _ => {},
            }
        }

        let opcode = cpu.fetch();
        cpu.execute(opcode);
        cpu.interrupt_poll();
    }
}
