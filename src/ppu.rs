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
    colour: u8,
    palette: u8,
    priority: u8,
}

impl SpritePixel {
    pub fn new(colour: u8, palette: u8, priority: u8) -> Self {
        SpritePixel {
            colour,
            palette,
            priority,
        }
    }
}

pub struct BackgroundPixel {
    colour: u8,
    palette: u8,
}

impl BackgroundPixel {
    pub fn new(colour: u8, palette: u8) -> Self {
        BackgroundPixel {
            colour,
            palette,
        }
    }
}

/////////////////////////////// FIFO ////////////////////////////////

pub struct PixelFetcher {
    fetcher_x: u8,
    window_line_counter: u8,
    tile_number: u8,
    tile_data_low: u8,
    tile_data_high: u8,
}

impl PixelFetcher { // pixel fetcher fetches 1 row of a tile at a time
    pub fn new() -> Self {
        PixelFetcher {
            fetcher_x: 0, // keeps track of which tile it is on. not the pixel. 
            window_line_counter: 0, // incremented each time the last scanline had window data on. 
            tile_number: 0,
            tile_data_low: 0,
            tile_data_high: 0,
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
            let offset = (((scx / 8) & 0x1F) + 32 * (((ly + scy) & 0xFF) / 8)) as u16;
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
        } else { (2 * ((ly + scy) % 8)) as u16};

        let byte_address = tile_address + offset;

        self.tile_data_low = memory.read(byte_address + 1);
    }

    pub fn push_to_fifo() {
        // To be implemented. 
        self.fetcher_x += 1;
    }
}

/////////////////////////////// PPU ////////////////////////////////

pub struct PPU {
    pub mode: u8,
    pub cycles: u16,
    pub ly: u8,
    pub x: u8,

    pub sprite_buffer: Vec<Sprite>,
    pub sprite_fifo: Vec<SpritePixel>,
    pub bgwin_fifo: Vec<BackgroundPixel>,

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
            sprite_fifo: Vec::new(),
            bgwin_fifo: Vec::new(),

            renderer: SDLRenderer::new(160, 144),
            pixel_fetcher: PixelFetcher::new(),
        }
    }

    pub fn tick(&mut self, memory: &Memory) {
        self.cycles = self.cycles.wrapping_add(4);
    }

    pub fn rendering_window(&mut self, memory: &Memory) -> bool { // Checks if the ppu is rendering the window and returns bool
        let wy = memory.read(0xFF4A);
        let wx = memory.read(0xFF4B).wrapping_sub(7);
        let window_enabled = if memory.read(0xFF40) & 0b00100000 == 0 { false } else { true };
        if self.ly >= wy && self.x >= wx && window_enabled {
            true 
        } else { false }
    }
}