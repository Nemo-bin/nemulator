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

            // RENDERING
            match true_tab_index {
                0 => {
                    f.render_widget(tabs, chunks[0]);
                    f.render_widget(content, chunks[1]);
                    f.render_stateful_widget(library_list, library_layout_horizontal[0], &mut stateful_rom_list.state);
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
                    filename = &stateful_rom_list.items[stateful_rom_list.state.selected().unwrap()];
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
        } else { cpu.m_cycle(); }
        cpu.interrupt_poll();
    }

    Ok(())
}
