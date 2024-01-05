mod cpu;
mod memory;
mod registers;
mod ppu;
mod timer;

use std::{
    io,
    fs,
    env,
    thread,
    time::Duration,
};
use fs::File;
use io::Read;

use sdl2::keyboard::Keycode;
use sdl2::event::Event;

use backtrace::*;

use tui::{
    backend::CrosstermBackend,
    widgets::{Tabs, Widget, Block, Borders, Paragraph, List, ListItem, ListState},
    layout::{Alignment, Layout, Constraint, Direction},
    style::{Color, Style},
    symbols::DOT,
    text::{Spans, Span},
    Terminal
};
use crossterm::{
    event::{self, poll, read, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

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

struct RomList {
    items: Vec<String>,
    state: ListState,
}

impl RomList {
    pub fn new(items: Vec<String>) -> Self {
        RomList {
            items,
            state: ListState::default(),
        }
    }

    pub fn update_items(&mut self, items: Vec<String>) {
        self.items = items;
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else { i + 1 }
            },
            None => 0
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() -1
                } else { i - 1 }
            },
            None => 0
        };
        self.state.select(Some(i));  
    }
}

pub fn get_cartridge_header(filename: &str) -> Vec<u8> {
    let mut f = File::open(filename).expect("Unable to open file!");
    let mut buffer = vec![0_u8; 0x014f];
    f.read_exact(&mut buffer);
    buffer
}

pub fn get_title(cartridge_header: &Vec<u8>) -> String {
    let mut title_data = cartridge_header[0x134..=0x143].to_vec();
    let mut title = match std::str::from_utf8(&title_data) {
        Ok(data) => data.to_string(),
        Err(data) => String::from("NO TITLE"),
    };
    title.trim_matches('\0').to_string()
}

pub fn get_licensee(cartridge_header: &Vec<u8>) -> String {
    let code = cartridge_header[0x14B];
    if code != 0x33 { 
        match_old_licensee_code(code)
    } else {
        let mut code_data = cartridge_header[0x144..=0x145].to_vec();
        let mut new_code = match std::str::from_utf8(&code_data) {
            Ok(data) => data.to_string(),
            Err(data) => String::from("NO LICENSEE"),
        };
        match_new_licensee_code(&new_code)
    }
}

pub fn get_destination(cartridge_header: &Vec<u8>) -> String {
    let destination = match cartridge_header[0x14A] {
        0 => "Japan",
        1 => "Overseas only",
        _ => "None",
    };
    destination.to_string()
}

pub fn get_rom_size(cartridge_header: &Vec<u8>) -> String {
    (32 * ((1 as u16) << cartridge_header[0x148])).to_string() + "KiB"
}

pub fn get_ram_size(cartridge_header: &Vec<u8>) -> String {
    let ram_size = match cartridge_header[0x149] {
        0x00 => "None",
        0x02 => "8 KiB",
        0x03 => "32 KiB",
        0x04 => "128 KiB",
        0x05 => "64 KiB",
        _ => "None",
    };
    ram_size.to_string()
}

pub fn get_cartridge_type(cartridge_header: &Vec<u8>) -> String {
    let cartridge_type = match cartridge_header[0x147] {
        0x00 => "ROM ONLY",
        0x01 => "MBC1",
        0x02 => "MBC1+RAM",
        0x03 => "MBC1+RAM+BATTERY",
        0x05 => "MBC2",
        0x06 => "MBC2+BATTERY",
        0x08 => "ROM+RAM 1",
        0x09 => "ROM+RAM+BATTERY 1",
        0x0B => "MMM01",
        0x0C => "MMM01+RAM",
        0x0D => "MMM01+RAM+BATTERY",
        0x0F => "MBC3+TIMER+BATTERY",
        0x10 => "MBC3+TIMER+RAM+BATTERY 2",
        0x11 => "MBC3",
        0x12 => "MBC3+RAM 2",
        0x13 => "MBC3+RAM+BATTERY 2",
        0x19 => "MBC5",
        0x1A => "MBC5+RAM",
        0x1B => "MBC5+RAM+BATTERY",
        0x1C => "MBC5+RUMBLE",
        0x1D => "MBC5+RUMBLE+RAM",
        0x1E => "MBC5+RUMBLE+RAM+BATTERY",
        0x20 => "MBC6",
        0x22 => "MBC7+SENSOR+RUMBLE+RAM+BATTERY",
        0xFC => "POCKET CAMERA",
        0xFD => "BANDAI TAMA5",
        0xFE => "HuC3",
        0xFF => "HuC1+RAM+BATTERY",
        _ => "None,"
    };
    cartridge_type.to_string()
}

pub fn match_old_licensee_code(code: u8) -> String {
    let licensee = match code {
        0x00 => "None",
        0x01 => "Nintendo",
        0x08 => "Capcom",
        0x09 => "Hot-B",
        0x0A => "Jaleco",
        0x0B => "Coconuts Japan",
        0x0C => "Elite Systems",
        0x13 => "EA (Electronic Arts)",
        0x18 => "Hudsonsoft",
        0x19 => "ITC Entertainment",
        0x1A => "Yanoman",
        0x1D => "Japan Clary",
        0x1F => "Virgin Interactive",
        0x24 => "PCM Complete",
        0x25 => "San-X",
        0x28 => "Kotobuki Systems",
        0x29 => "Seta",
        0x30 => "Infogrames",
        0x31 => "Nintendo",
        0x32 => "Bandai",
        0x33 => "Indicates that the New licensee code should be used instead.",
        0x34 => "Konami",
        0x35 => "HectorSoft",
        0x38 => "Capcom",
        0x39 => "Banpresto",
        0x3C => ".Entertainment i",
        0x3E => "Gremlin",
        0x41 => "Ubisoft",
        0x42 => "Atlus",
        0x44 => "Malibu",
        0x46 => "Angel",
        0x47 => "Spectrum Holoby",
        0x49 => "Irem",
        0x4A => "Virgin Interactive",
        0x4D => "Malibu",
        0x4F => "U.S. Gold",
        0x50 => "Absolute",
        0x51 => "Acclaim",
        0x52 => "Activision",
        0x53 => "American Sammy",
        0x54 => "GameTek",
        0x55 => "Park Place",
        0x56 => "LJN",
        0x57 => "Matchbox",
        0x59 => "Milton Bradley",
        0x5A => "Mindscape",
        0x5B => "Romstar",
        0x5C => "Naxat Soft",
        0x5D => "Tradewest",
        0x60 => "Titus",
        0x61 => "Virgin Interactive",
        0x67 => "Ocean Interactive",
        0x69 => "EA (Electronic Arts)",
        0x6E => "Elite Systems",
        0x6F => "Electro Brain",
        0x70 => "Infogrames",
        0x71 => "Interplay",
        0x72 => "Broderbund",
        0x73 => "Sculptered Soft",
        0x75 => "The Sales Curve",
        0x78 => "t.hq",
        0x79 => "Accolade",
        0x7A => "Triffix Entertainment",
        0x7C => "Microprose",
        0x7F => "Kemco",
        0x80 => "Misawa Entertainment",
        0x83 => "Lozc",
        0x86 => "Tokuma Shoten Intermedia",
        0x8B => "Bullet-Proof Software",
        0x8C => "Vic Tokai",
        0x8E => "Ape",
        0x8F => "I’Max",
        0x91 => "Chunsoft Co.",
        0x92 => "Video System",
        0x93 => "Tsubaraya Productions Co.",
        0x95 => "Varie Corporation",
        0x96 => "Yonezawa/S’Pal",
        0x97 => "Kaneko",
        0x99 => "Arc",
        0x9A => "Nihon Bussan",
        0x9B => "Tecmo",
        0x9C => "Imagineer",
        0x9D => "Banpresto",
        0x9F => "Nova",
        0xA1 => "Hori Electric",
        0xA2 => "Bandai",
        0xA4 => "Konami",
        0xA6 => "Kawada",
        0xA7 => "Takara",
        0xA9 => "Technos Japan",
        0xAA => "Broderbund",
        0xAC => "Toei Animation",
        0xAD => "Toho",
        0xAF => "Namco",
        0xB0 => "acclaim",
        0xB1 => "ASCII or Nexsoft",
        0xB2 => "Bandai",
        0xB4 => "Square Enix",
        0xB6 => "HAL Laboratory",
        0xB7 => "SNK",
        0xB9 => "Pony Canyon",
        0xBA => "Culture Brain",
        0xBB => "Sunsoft",
        0xBD => "Sony Imagesoft",
        0xBF => "Sammy",
        0xC0 => "Taito",
        0xC2 => "Kemco",
        0xC3 => "Squaresoft",
        0xC4 => "Tokuma Shoten Intermedia",
        0xC5 => "Data East",
        0xC6 => "Tonkinhouse",
        0xC8 => "Koei",
        0xC9 => "UFL",
        0xCA => "Ultra",
        0xCB => "Vap",
        0xCC => "Use Corporation",
        0xCD => "Meldac",
        0xCE => ".Pony Canyon or",
        0xCF => "Angel",
        0xD0 => "Taito",
        0xD1 => "Sofel",
        0xD2 => "Quest",
        0xD3 => "Sigma Enterprises",
        0xD4 => "ASK Kodansha Co.",
        0xD6 => "Naxat Soft",
        0xD7 => "Copya System",
        0xD9 => "Banpresto",
        0xDA => "Tomy",
        0xDB => "LJN",
        0xDD => "NCS",
        0xDE => "Human",
        0xDF => "Altron",
        0xE0 => "Jaleco",
        0xE1 => "Towa Chiki",
        0xE2 => "Yutaka",
        0xE3 => "Varie",
        0xE5 => "Epcoh",
        0xE7 => "Athena",
        0xE8 => "Asmik ACE Entertainment",
        0xE9 => "Natsume",
        0xEA => "King Records",
        0xEB => "Atlus",
        0xEC => "Epic/Sony Records",
        0xEE => "IGS",
        0xF0 => "A Wave",
        0xF3 => "Extreme Entertainment",
        0xFF => "LJN",
        _ => "None",
    };
    licensee.to_string()
}

pub fn match_new_licensee_code(code: &str) -> String {
    let licensee = match code {
        "00" => "None",
        "01" => "Nintendo R&D1",
        "08" => "Capcom",
        "13" => "Electronic Arts",
        "18" => "Hudson Soft",
        "19" => "b-ai",
        "20" => "kss",
        "22" => "pow",
        "24" => "PCM Complete",
        "25" => "san-x",
        "28" => "Kemco Japan",
        "29" => "seta",
        "30" => "Viacom",
        "31" => "Nintendo",
        "32" => "Bandai",
        "33" => "Ocean/Acclaim",
        "34" => "Konami",
        "35" => "Hector",
        "37" => "Taito",
        "38" => "Hudson",
        "39" => "Banpresto",
        "41" => "Ubi Soft",
        "42" => "Atlus",
        "44" => "Malibu",
        "46" => "angel",
        "47" => "Bullet-Proof",
        "49" => "irem",
        "50" => "Absolute",
        "51" => "Acclaim",
        "52" => "Activision",
        "53" => "American sammy",
        "54" => "Konami",
        "55" => "Hi tech entertainment",
        "56" => "LJN",
        "57" => "Matchbox",
        "58" => "Mattel",
        "59" => "Milton Bradley",
        "60" => "Titus",
        "61" => "Virgin",
        "64" => "LucasArts",
        "67" => "Ocean",
        "69" => "Electronic Arts",
        "70" => "Infogrames",
        "71" => "Interplay",
        "72" => "Broderbund",
        "73" => "sculptured",
        "75" => "sci",
        "78" => "THQ",
        "79" => "Accolade",
        "80" => "misawa",
        "83" => "lozc",
        "86" => "Tokuma Shoten Intermedia",
        "87" => "Tsukuda Original",
        "91" => "Chunsoft",
        "92" => "Video system",
        "93" => "Ocean/Acclaim",
        "95" => "Varie",
        "96" => "Yonezawa/s’pal",
        "97" => "Kaneko",
        "99" => "Pack in soft",
        "9H" => "Bottom Up",
        "A4" => "Konami (Yu-Gi-Oh!)",
        _ => "None",
    };
    licensee.to_string()
}

fn main() -> Result<(), io::Error> {

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

    /*let args:Vec<String> = env::args().collect();
    let mut filename = &args[1];
    if args.len() == 3 {
        let blargg_log_number = format!("blarggs_logs/{}", &args[2]);
    }*/
    /////////////////////////////////// TUI ///////////////////////////////////

    enable_raw_mode().expect("User input enabled");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture);
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let tabs_options:Vec<_> = ["Library", "Settings"]
    .iter().cloned().map(Spans::from).collect();

    let mut tab_index:usize = 0;
    let mut true_tab_index = 0;
    let mut rom_list_index = 0;
    let mut true_rom_list_index = 0;

    let mut darkest_green = Color::Rgb(15, 56, 15);
    let mut dark_green = Color::Rgb(48, 98, 48);
    let mut lightest_green = Color::Rgb(155, 188, 15);

    let mut roms: Vec<String> = Vec::new();
    let mut stateful_rom_list = RomList::new(roms);
    stateful_rom_list.state.select(Some(0));

    let mut filename = &String::from("TEMP");

    'running:loop {

        // FILES

        let entries = fs::read_dir("./").unwrap();
        let mut temp_roms: Vec<String> = Vec::new();
        for dir_entry in entries {
            let path = dir_entry.as_ref().unwrap().path();
            if let Some(extension) = path.extension() {
                if extension == "gb" {
                    let filename = path.file_name().and_then(|s| s.to_str()).unwrap().to_owned();
                    temp_roms.push(filename);
                }
            }
        }

        stateful_rom_list.update_items(temp_roms);
        filename = &stateful_rom_list.items[stateful_rom_list.state.selected().unwrap()];
        let cartridge_header = &get_cartridge_header(filename);

        let title = "Title: ".to_string() + &get_title(cartridge_header);
        let licensee = "Licensee: ".to_string() + &get_licensee(cartridge_header);
        let destination = "Destination: ".to_string() + &get_destination(cartridge_header);
        let cartridge_type = "Type: ".to_string() + &get_cartridge_type(cartridge_header);
        let rom_size = "Cart. ROM: ".to_string() + &get_rom_size(cartridge_header);
        let ram_size = "Cart. RAM: ".to_string() + &get_ram_size(cartridge_header);

        let rom_metadata = vec![title, licensee, destination, cartridge_type, rom_size, ram_size];

        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default()
                .title("Block")
                .borders(Borders::ALL)
                .style(Style::default().bg(lightest_green));
            f.render_widget(block, size);

            // MASTER LAYOUT
            let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(10),
                          Constraint::Percentage(90)].as_ref())
            .split(size);

            // LIBRARY LAYOUT
            let library_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(chunks[1]);
            let library_layout_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50),
                          Constraint::Percentage(50)].as_ref())
            .split(library_layout[0]);

            // SETTINGS LAYOUT
            let settings_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(90),
                          Constraint::Percentage(10)].as_ref())
            .split(chunks[1]);
            let settings_layout_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30),
                          Constraint::Percentage(70)].as_ref())
            .split(settings_layout[0]);

            // TABS SETUP
            true_tab_index = tab_index % tabs_options.len();

            let tabs = Tabs::new(tabs_options.clone())
            .select(true_tab_index)
            .style(Style::default().fg(dark_green))
            .highlight_style(Style::default().fg(darkest_green))
            .divider(Span::raw("|"))
            .block(
                Block::default().title("Tabs").borders(Borders::ALL)
            );

            // WIDGETS
            let items: Vec<ListItem> = stateful_rom_list.items.iter().map(|i| ListItem::new(i.as_ref())).collect();
            let library_list = List::new(items)
                .block(Block::default().title("In your library").borders(Borders::ALL))
                .style(Style::default().fg(dark_green))
                .highlight_style(Style::default().fg(darkest_green))
                .highlight_symbol(">>");

            let content = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(darkest_green));

            let items: Vec<ListItem> = rom_metadata.iter().map(|i| ListItem::new(i.as_ref())).collect();
            let rom_metadata_list = List::new(items)
            .block(Block::default().title("Rom Metadata").borders(Borders::ALL))
            .style(Style::default().fg(dark_green))
            .highlight_style(Style::default().fg(darkest_green))
            .highlight_symbol(">>");

            // RENDERING
            match true_tab_index {
                0 => {
                    f.render_widget(tabs, chunks[0]);
                    f.render_widget(content, chunks[1]);
                    f.render_stateful_widget(library_list, library_layout_horizontal[0], &mut stateful_rom_list.state);
                    f.render_widget(rom_metadata_list, library_layout_horizontal[1]);
                }
                1 => {
                    
                    f.render_widget(tabs, chunks[0]);
                    f.render_widget(content, chunks[1]);
                }
                _ => {}
            };

        })?;  

        if poll(std::time::Duration::from_millis(100))?{
            match read()?{
                CrosstermEvent::Key(KeyEvent {code:KeyCode::Esc, ..}, ..) => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break 'running;
                },
                CrosstermEvent::Key(KeyEvent {code:KeyCode::Left, ..}, ..) => 
                    if tab_index > 0 {tab_index -= 1}
                    else {tab_index = tabs_options.len() - 1},
                CrosstermEvent::Key(KeyEvent {code:KeyCode::Right, ..}, ..) =>
                    if tab_index < (tabs_options.len() -1) {tab_index += 1}
                    else {tab_index = 0},
                CrosstermEvent::Key(KeyEvent {code:KeyCode::Up, ..}, ..) =>
                    { stateful_rom_list.previous(); }
                CrosstermEvent::Key(KeyEvent {code:KeyCode::Down, ..}, ..) =>
                    { stateful_rom_list.next(); }
                CrosstermEvent::Key(KeyEvent {code:KeyCode::Enter, ..}, ..) => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break 'running;
                }
                _ => {},
            }
        } else{}
    }

    ///////////////////////////////// "MAIN" /////////////////////////////////

    let mut cpu = CPU::new();
    println!("CREATED CPU");
    println!("FILE => {}", filename);
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

        /*if cpu.pc > 100 {
            if (log_line.trim() != blargg_log_lines[line_index].trim()) {
                println!("{}", log_line);
                println!("{}", blargg_log_lines[line_index]);
                println!("Opcode {:x} is potentially erroneous", cpu.memory.read(cpu.pc.wrapping_sub(1)));
                panic!("Logs are not equal on line {}", line_index);
            }
            logfile.write(log_line.as_bytes());
            line_index += 1;
        }*/

        //vprintln!("{}", log_line);

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
                Event::KeyDown { keycode: Some(Keycode::U), .. } => { 
                    println!("MEM @ C0: ");
                    let mut pointer = 0xC000;
                    while pointer <= 0xC09F {
                        print!("{:x} / ", cpu.memory.read(pointer));
                        pointer += 1;
                    }
                    println!("OAM: ");
                    let mut oam_pointer = 0xFE00;
                    while oam_pointer <= 0xFE9F {
                        print!("{:x} / ", cpu.memory.read(oam_pointer));
                        oam_pointer += 1;
                    }
                },
                // Keybinds: (potentially temporary) WASD => DPad, Q => A, E => B, R => Start, F => Select
                // Ordered as they are in JOYP
                Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                    cpu.input_states.down = true;
                },
                Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                    cpu.input_states.up = true;
                },
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    cpu.input_states.left = true;
                },
                Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                    cpu.input_states.right = true;
                },
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    cpu.input_states.start = true;                    
                },
                Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                    cpu.input_states.select = true;
                },
                Event::KeyDown { keycode: Some(Keycode::E), .. } => {
                    cpu.input_states.b = true;
                },
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                    cpu.input_states.a = true;
                },
                // Input keys released
                Event::KeyUp { keycode: Some(Keycode::S), .. } => {
                    cpu.input_states.down = false;
                },
                Event::KeyUp { keycode: Some(Keycode::W), .. } => {
                    cpu.input_states.up = false;
                },
                Event::KeyUp { keycode: Some(Keycode::A), .. } => {
                    cpu.input_states.left = false;
                },
                Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                    cpu.input_states.right = false;
                },
                Event::KeyUp { keycode: Some(Keycode::R), .. } => {
                    cpu.input_states.start = false;                    
                },
                Event::KeyUp { keycode: Some(Keycode::F), .. } => {
                    cpu.input_states.select = false;
                },
                Event::KeyUp { keycode: Some(Keycode::E), .. } => {
                    cpu.input_states.b = false;
                },
                Event::KeyUp { keycode: Some(Keycode::Q), .. } => {
                    cpu.input_states.a = false;
                },
                _ => {},
            }
        }

        if !cpu.halted {
            let opcode = cpu.fetch();
            cpu.execute(opcode);
            // println!("{:x}", opcode);
        } else { cpu.m_cycle(); }
        cpu.interrupt_poll();
    }

    Ok(())
}
