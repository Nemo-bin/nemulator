//////////////////////////////// USE ////////////////////////////////

use crate::memory::Memory;
use crate::registers::*;
use crate::cpu::CPU;

use sdl2::{
    pixels::PixelFormatEnum,
    render::{
        Canvas,
        Texture
    },
    video::Window,
    event::Event,
    EventPump,
};

use std::borrow::BorrowMut;

//////////////////////////////// MACROS ////////////////////////////////

macro_rules! box_arr { // boxes arrays onto the heap. 
    ($t:expr; $size:expr) => {
        vec![$t; $size].into_boxed_slice().try_into().unwrap()
    };
}

// let arr: Box<[u8; 512]> = box_arr![0; 512];

/////////////////////////////// SDL2 ////////////////////////////////

pub struct SDLRenderer {
    width: u32,
    height: u32,
    canvas: Canvas<Window>,
    texture: Texture,
    pub displaybuffer: Vec<u8>,
    pub event_pump: EventPump,
}

impl SDLRenderer {
    const PIXELSIZE:usize = 4;

    pub fn new(width: u32, height: u32) -> Self {
        let sdl_context = sdl2::init().expect("failed to create sdl context");

        let event_pump = sdl_context.event_pump().expect("failed to create event pump");

        let video_subsystem = sdl_context.video().expect("failed to get video context");

        let window = video_subsystem.window("Nemulator", width * 3, height * 3)
        .build()
        .expect("failed to build window");
    
        let mut canvas: Canvas<Window> = window.into_canvas()
        .build()
        .expect("failed to build window's canvas");

        let texture_creator = canvas.texture_creator();
        let t = texture_creator.create_texture_streaming(
            PixelFormatEnum::RGB888,
            width,
            height,
        );
        let texture = if t.is_ok() { t.unwrap() } else { panic!("failed to create texture") };

        SDLRenderer{
            width,
            height,
            canvas,
            texture,
            displaybuffer: vec![0; width as usize * height as usize * Self::PIXELSIZE],
            event_pump,
        }
    }

    fn update(&mut self) {
        self.texture
            .update(None, &self.displaybuffer, self.width as usize * Self::PIXELSIZE);
        self.canvas
            .copy(&self.texture, None, None);
        self.canvas.present();
    }
}

/////////////////////////////// SPRITE ///////////////////////////////

pub struct Sprite {
    pub y: u8,
    pub x: u8,
    pub index: u8,
    pub attributes: u8, 
 }
 
 impl Sprite {
     pub fn new(y: u8, x: u8, index: u8, attributes: u8) -> Self {
         Sprite {
             y: y,
             x: x,
             index: index,
             attributes: attributes,
         }
     }
 }

/////////////////////////////// PIXELS ////////////////////////////////

pub struct SpritePixel {
    colour_id: u8,
    palette: u16,
    priority: u8,
}

impl SpritePixel {
    pub fn new(colour_id: u8, palette: u16, priority: u8) -> Self {
        SpritePixel {
            colour_id,
            palette,
            priority,
        }
    }
}

pub struct BackgroundPixel {
    colour_id: u8,
    palette: u16,
}

impl BackgroundPixel {
    pub fn new(colour_id: u8, palette: u16) -> Self {
        BackgroundPixel {
            colour_id,
            palette,
        }
    }
}

/////////////////////////////// FIFO ////////////////////////////////

pub struct QueueNode<T> {
    value: T,
    next: Option<Box<QueueNode<T>>>
}

impl<T> QueueNode<T> {
    pub fn new(value: T) -> Self {
        QueueNode {
            value,
            next: None,
        }
    }
}

pub struct Queue<T> {
    end: Option<QueueNode<T>>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            end: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.end {
            None => true,
            _ => false,
        }
    }

    pub fn add(&mut self, value: T) {
        let new_node = QueueNode::new(value);
        if let Some(end) = &mut self.end {
            let mut start = end;
            loop {
                if let Some(_) = &start.next {
                    start = (start.next.as_mut().unwrap())
                                .borrow_mut();
                } else { break; }
            }
            start.next = Some(Box::new(new_node));
        } else { self.end = Some(new_node); }
    }

    pub fn remove(&mut self) -> Option<T> {
        if !self.is_empty() {
            let end = std::mem::take(&mut self.end).unwrap();
            if let Some(next) = end.next {
                self.end = Some(*next);
            }
            Some (end.value)
        } else { None }
    }

    pub fn clear(&mut self) {
        while !self.is_empty() {
            let end = std::mem::take(&mut self.end).unwrap();
            if let Some(next) = end.next {
                self.end = Some(*next);
            }
        }
    }
}

/////////////////////////////// PIXELFETCHER ////////////////////////////////

pub enum FetcherState {
    TileNumber,
    TileDataLow,
    TileDataHigh,
    PushToFifo,
}

pub struct PixelFetcher {
    fetcher_x: u8,
    window_line_counter: u8,
    tile_number: u8,
    tile_data_low: u8,
    tile_data_high: u8,

    cycles: u8,
    state: FetcherState,

    pub sprite_fifo: Queue<SpritePixel>,
    pub bgwin_fifo: Queue<BackgroundPixel>,
}

impl PixelFetcher { // pixel fetcher fetches 1 row of a tile at a time
    pub fn new() -> Self {
        PixelFetcher {
            fetcher_x: 0, // keeps track of which tile it is on. not the pixel. 
            window_line_counter: 0, // incremented each time the last scanline had window data on. 
            tile_number: 0,
            tile_data_low: 0,
            tile_data_high: 0,

            cycles: 0,
            state: FetcherState::TileNumber,

            sprite_fifo: Queue::new(),
            bgwin_fifo: Queue::new(),
        }
    }

    // The 4 fetcher steps in order - each takes 2 T-cycles (1/2 of an M-cycle)
    pub fn fetch_tile_number(&mut self, memory: &Memory, rendering_window: bool, ly: u8) {
        let address = if rendering_window {
            let mut tilemap = if memory.read(0xFF40) & 0b01000000 == 0 { 0x9800 } else { 0x9C00 };
            let offset = (32 * (self.window_line_counter / 8)) as u16;
            tilemap + offset
        } else { 
            let mut tilemap = if memory.read(0xFF40) & 0b00001000 == 0 { 0x9800 } else { 0x9C00 }; 
            let scy = memory.read(0xFF42);
            let scx = memory.read(0xFF43);
            let offset = (((scx / 8) & 0x1F).wrapping_add(32).wrapping_mul((ly.wrapping_add(scy) & 0xFF) / 8)) as u16;
            tilemap + offset
        };
        self.tile_number = memory.read(address);
    }

    pub fn fetch_tile_data_low(&mut self, memory: &Memory, rendering_window: bool, ly: u8) {
        let tile_address = if memory.read(0xFF40) & 0b00010000 == 0 && self.tile_number < 128 { 
            0x9000 + (self.tile_number * 16) as u16
        } else { 0x8000 + (self.tile_number * 16) as u16 };

        let scy = memory.read(0xFF42);
        let offset = if rendering_window {
            (2 * (self.window_line_counter % 8)) as u16
        } else { (2 * ((ly + scy) % 8)) as u16};

        let byte_address = tile_address + offset;

        self.tile_data_low = memory.read(byte_address);
    }

    pub fn fetch_tile_data_high(&mut self, memory: &Memory, rendering_window: bool, ly: u8) {
        let tile_address = if memory.read(0xFF40) & 0b00010000 == 0 && self.tile_number < 128 { 
            0x9000 + (self.tile_number * 16) as u16
        } else { 0x8000 + (self.tile_number * 16) as u16 };

        let scy = memory.read(0xFF42);
        let offset = if rendering_window {
            (2 * (self.window_line_counter % 8)) as u16
        } else { (2 * (ly.wrapping_add(scy) % 8)) as u16};

        let byte_address = tile_address.wrapping_add(offset);

        self.tile_data_high = memory.read(byte_address.wrapping_add(1));
    }

    pub fn push_to_fifo(&mut self) -> bool {
        if self.bgwin_fifo.is_empty() {
            println!("PUSHING TO FIFO");
            for pixel_number in 0..7 {
                let colour_high = ((self.tile_data_high & (0b10000000 >> pixel_number)) >> (7 - pixel_number)) << 1;
                let colour_low = ((self.tile_data_high & (0b10000000 >> pixel_number)) >> (7 - pixel_number));
                let colour = colour_high | colour_low;
                let pixel = BackgroundPixel::new(colour, 0xFF47);
    
                self.bgwin_fifo.add(pixel);
            }
            self.fetcher_x = self.fetcher_x.wrapping_add(1);
            true
        } else { false }
    }
}

/////////////////////////////// PPU ////////////////////////////////

pub struct PPU {
    pub mode: u8,
    pub cycles: u16,
    pub ly: u8,
    pub x: u8,

    pub sprite_buffer: Vec<Sprite>,

    pub renderer: SDLRenderer,
    pub pixel_fetcher: PixelFetcher,
}

impl PPU  {
    pub fn new() -> Self {
        PPU {
            mode: 2,
            cycles: 0,
            ly: 0,
            x: 0,

            sprite_buffer: Vec::new(),

            renderer: SDLRenderer::new(160, 144),
            pixel_fetcher: PixelFetcher::new(),
        }
    }

    pub fn tick(&mut self, memory: &Memory) {
        self.step(memory);
        self.step(memory);
        self.step(memory);
        self.step(memory);
    }

    pub fn step(&mut self, memory: &Memory) {
        self.cycles = self.cycles.wrapping_add(1);
        println!("[Cycles: {}] [Mode: {}] [LY: {}] [X: {}]", self.cycles, self.mode, self.ly, self.x);
        self.pixel_fetcher.cycles = self.pixel_fetcher.cycles.wrapping_add(1);
        match self.mode {
            0 => self.h_blank(),
            1 => self.v_blank(),
            2 => { if self.cycles == 80 { 
                        self.mode = 3; 
                        self.cycles = 0;
                        self.pixel_fetcher.bgwin_fifo.clear(); 
                        self.pixel_fetcher.sprite_fifo.clear(); 
                        }; 
                }, // oam scan - need to implement, will do at another time. 
            3 => self.mode_3(memory),
            _ => unreachable!(),
        }
    }

    pub fn rendering_window(&mut self, memory: &Memory) -> bool { // Checks if the ppu is rendering the window and returns bool
        let wy = memory.read(0xFF4A);
        let wx = memory.read(0xFF4B).wrapping_sub(7);
        let window_enabled = if memory.read(0xFF40) & 0b00100000 == 0 { false } else { true };
        if self.ly >= wy && self.x >= wx && window_enabled {
            true 
        } else { false }
    }

    pub fn h_blank(&mut self) {
        if self.cycles == 456 {
            self.mode = 2;
            self.ly = self.ly.wrapping_add(1);
            self.cycles = 0;
            self.x = 0;
            if self.ly == 144 { self.mode = 1; }
        }
    }

    pub fn v_blank(&mut self) {
        if self.cycles == 4560 {
            self.mode = 2;
            self.ly = 0;
            self.cycles = 0;
            self.x = 0;
            self.renderer.update();
        }
    }

    pub fn mode_3(&mut self, memory: &Memory) { // this needs redoing to work off t cycles
        let rendering_window = self.rendering_window(memory);
        match self.pixel_fetcher.state {
            FetcherState::TileNumber => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.fetch_tile_number(memory, rendering_window, self.ly);
                    self.pixel_fetcher.state = FetcherState::TileDataLow;
                    self.pixel_fetcher.cycles = 0;
                }
            },
            FetcherState::TileDataLow => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.fetch_tile_data_low(memory, rendering_window, self.ly);
                    self.pixel_fetcher.state = FetcherState::TileDataHigh;
                    self.pixel_fetcher.cycles = 0;
                }
            },
            FetcherState::TileDataHigh => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.fetch_tile_data_high(memory, rendering_window, self.ly);
                    self.pixel_fetcher.state = FetcherState::PushToFifo;
                    self.pixel_fetcher.cycles = 0;
                }
            },
            FetcherState::PushToFifo => { // returns "success" status - true if works. 
                let success = self.pixel_fetcher.push_to_fifo();
                if success {
                    self.pixel_fetcher.state = FetcherState::TileNumber;
                    self.pixel_fetcher.cycles = 0;
                }
            },
        }

        if !self.pixel_fetcher.bgwin_fifo.is_empty() {
            self.push_to_lcd(memory);
            self.x = self.x.wrapping_add(1);
        }

        if self.cycles == 289 {
            self.cycles = 0;
            self.mode = 0;
            self.x = 0;
        }
    }

////////////////////////////////////////////////////////////////////

    pub fn push_to_lcd(&mut self, memory: &Memory) { // each pixel pushed takes 1 dot
        let lcdc = memory.read(0xFF40); // this may very well be wrong, i need to think on how to do this.
                                        // probably draw a diagram...
        let mut rgb = 0;

        // if lcdc & 0b00000001 == 1 {
            let pixel = self.pixel_fetcher.bgwin_fifo.remove().unwrap(); // pixel.colour tells us the id 
            let palette = memory.read(pixel.palette); // aka which 2 bits of the palette to use
            let colour = (palette & 0b00000011 << pixel.colour_id * 2) >> pixel.colour_id * 2;
            let rgb = match colour {
                0 => 255,
                1 => 169,
                2 => 84,
                3 => 0,
                _ => unreachable!(),
            };
        // }

        println!("PIXEL_PUSHED: {}", rgb);
        
        let displaybuffer_index: usize = ((self.ly.wrapping_mul(160).wrapping_add(self.x)).wrapping_mul(4)) as usize;
        self.renderer.displaybuffer[displaybuffer_index] = rgb;
        self.renderer.displaybuffer[displaybuffer_index.wrapping_add(1)] = rgb;
        self.renderer.displaybuffer[displaybuffer_index.wrapping_add(2)] = rgb;
    }
}