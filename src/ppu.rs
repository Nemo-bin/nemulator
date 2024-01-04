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
            displaybuffer: vec![0; 160 * 144 * Self::PIXELSIZE],
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
    len: u8,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            end: None,
            len: 0,
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
        self.len += 1;
    }

    pub fn remove(&mut self) -> Option<T> {
        if !self.is_empty() {
            let end = std::mem::take(&mut self.end).unwrap();
            if let Some(next) = end.next {
                self.end = Some(*next);
            }
            self.len -= 1;
            Some (end.value)
        } else { None }
    }

    pub fn clear(&mut self) {
        while !self.is_empty() {
            let end = std::mem::take(&mut self.end).unwrap();
            if let Some(next) = end.next {
                self.end = Some(*next);
            }
            self.len -= 1;
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
    sprite_tile_data_low: u8,
    sprite_tile_data_high: u8,

    rendering_window: bool,

    cycles: u8,
    bgwin_state: FetcherState,
    sprite_state: FetcherState,
    first_tile: bool,

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
            sprite_tile_data_low: 0,
            sprite_tile_data_high: 0,

            rendering_window: false,

            cycles: 0,
            bgwin_state: FetcherState::TileNumber,
            sprite_state: FetcherState::TileNumber,
            first_tile: false,

            sprite_fifo: Queue::new(),
            bgwin_fifo: Queue::new(),
        }
    }

    // The 4 fetcher steps in order - each takes 2 T-cycles (1/2 of an M-cycle)
    pub fn fetch_tile_number(&mut self, memory: &mut Memory, ly: u8) {
        let address = if self.rendering_window {
            let mut tilemap = if memory.read(0xFF40) & 0b01000000 == 0 { 0x9800 } else { 0x9C00 };
            let wx = memory.read(0xFF4A);
            let wy = memory.read(0xFF4B);
            let offset = ((((self.window_line_counter.wrapping_div(8)).wrapping_mul(32)).wrapping_add(self.fetcher_x & 0x1F)) as u16) & 0x3fff; //(32 * (self.window_line_counter / 8)) as u16;
            // println!("GETTING WINDOW TILES");
            tilemap + offset
        } else { 
            let mut tilemap = if memory.read(0xFF40) & 0b00001000 == 0 { 0x9800 } else { 0x9C00 }; 
            let scy = memory.read(0xFF42);
            let scx = memory.read(0xFF43);
            let offset = (((((ly.wrapping_add(scy) >> 3) & 31) as u16).wrapping_mul(32)).wrapping_add((((self.fetcher_x).wrapping_add(scx >> 3)) & 31) as u16)) as u16;
            // println!("OFFSET => {} LY => {} SCY => {} SCX => {} FETCHER_X => {}", offset, ly, scy, scx, self.fetcher_x);
            // println!("NOT GETTING WINDOW TILES");
            tilemap + offset
        };
        self.tile_number = memory.read(address);
    }

    pub fn fetch_tile_data_low(&mut self, memory: &mut Memory, ly: u8) {
        let tile_address = if memory.read(0xFF40) & 0b0001_0000 == 0 && self.tile_number < 128 { 
            0x9000 + ((self.tile_number as u16).wrapping_mul(16))
        } else { 0x8000 + ((self.tile_number as u16).wrapping_mul(16)) };

        let scy = memory.read(0xFF42);
        let offset = if self.rendering_window {
            (2 * (self.window_line_counter % 8)) as u16
        } else { (2 * ((ly.wrapping_add(scy)) % 8)) as u16};

        let byte_address = tile_address.wrapping_add(offset);

        self.tile_data_low = memory.read(byte_address);
    }

    pub fn fetch_tile_data_high(&mut self, memory: &mut Memory, ly: u8) {
        let tile_address = if memory.read(0xFF40) & 0b0001_0000 == 0 && self.tile_number < 128 { 
            0x9000 + ((self.tile_number as u16).wrapping_mul(16))
        } else { 0x8000 + ((self.tile_number as u16).wrapping_mul(16)) };

        let scy = memory.read(0xFF42);
        let offset = if self.rendering_window {
            (2 * (self.window_line_counter % 8)) as u16
        } else { (2 * (ly.wrapping_add(scy) % 8)) as u16};

        let byte_address = tile_address.wrapping_add(offset);

        self.tile_data_high = memory.read(byte_address.wrapping_add(1));

        if self.first_tile {
            self.first_tile = false;
            self.bgwin_state = FetcherState::TileNumber;
            // println!("RESET TO TILENUMBER STATE")
        }
    }

    pub fn push_to_fifo(&mut self) -> bool {
        // println!("PUSHED PIXELS TO BGWIN FIFO");
        if self.bgwin_fifo.is_empty() {
            // println!("PUSHING TO FIFO");
            for pixel_number in 0..=7 {
                let colour_high = ((self.tile_data_high & (0b10000000 >> pixel_number)) >> (7 - pixel_number)) << 1;
                let colour_low = ((self.tile_data_low & (0b10000000 >> pixel_number)) >> (7 - pixel_number));
                let colour = colour_high | colour_low;
                
                let pixel = BackgroundPixel::new(colour, 0xFF47);

                self.bgwin_fifo.add(pixel);
                //println!("BGWIN FIFO LEN => {}", self.bgwin_fifo.len);
            }
            // println!("FETCHERX => {}, FIRSTILE => {}", self.fetcher_x, self.first_tile);
            self.fetcher_x = self.fetcher_x.wrapping_add(1);
            true
        } else { false }
    }

    pub fn sprite_fetch_tile_data_low(&mut self, memory: &mut Memory, ly: u8, sprite: &Sprite) {
        let tile_address = 0x8000 + ((sprite.index as u16).wrapping_mul(16));

        let height = if (memory.read(0xff40) & 0x4) == 0 { 8 } else { 16 };
        let mut offset = ((ly as u16).wrapping_sub((sprite.y as u16).wrapping_sub(16)) % height).wrapping_mul(2) as u16;
        let y_flip = (sprite.attributes >> 6) & 1;
        offset = if y_flip == 1 { ((height - 1)*2) - offset } else { offset };

        let byte_address = tile_address.wrapping_add(offset);

        self.sprite_tile_data_low = memory.read(byte_address);
        // println!("TILE ADDRESS => {:x} BYTE ADDRESS => {:x} @ {}", tile_address, byte_address, ly);
    }

    pub fn sprite_fetch_tile_data_high(&mut self, memory: &mut Memory, ly: u8, sprite: &Sprite) {
        let tile_address = 0x8000 + ((sprite.index as u16).wrapping_mul(16));

        let height = if (memory.read(0xff40) & 0x4) == 0 { 8 } else { 16 };
        let mut offset = ((ly as u16).wrapping_sub((sprite.y as u16).wrapping_sub(16)) % height).wrapping_mul(2) as u16;
        let y_flip = (sprite.attributes >> 6) & 1;
        offset = if y_flip == 1 { ((height - 1)*2) - offset } else { offset };

        let byte_address = tile_address.wrapping_add(offset);

        self.sprite_tile_data_high = memory.read(byte_address.wrapping_add(1));
    }

    pub fn push_to_sprite_fifo(&mut self, sprite: &Sprite) {
        let x_flip = (sprite.attributes >> 5) & 1;
        // println!("SPRITE PIXELS PUSHED TO FIFO");
        for mut pixel_number in self.sprite_fifo.len..=7 {
            pixel_number = if x_flip == 1 { 7 - pixel_number } else { pixel_number };
            let colour_high = ((self.sprite_tile_data_high & (0b10000000 >> pixel_number)) >> (7 - pixel_number)) << 1;
            let colour_low = ((self.sprite_tile_data_low & (0b10000000 >> pixel_number)) >> (7 - pixel_number));
            let mut colour = colour_high | colour_low;
            let palette = match (sprite.attributes & 0b0001_0000) >> 4 {
                0 => 0xFF48,
                1 => 0xFF49,
                _ => unreachable!(),
            };

            let priority = (sprite.attributes & 0b1000_0000) >> 7;
            let pixel = SpritePixel::new(colour, palette, priority);

            self.sprite_fifo.add(pixel);
            // println!("SPRITE FIFO LEN => {}", self.sprite_fifo.len);
        }
    }
}

/////////////////////////////// PPU ////////////////////////////////

pub struct PPU {
    pub mode: u8,
    pub cycles: u16,
    pub ly: u8,
    pub x: u8,

    pub mode_3_penalty: u16,
    pub obj_penalty: u16,
    pub rendering_window: bool,
    pub entered_window: bool,
    pub entered_vblank: bool,
    pub stat_irq: bool,
    pub first_irq_on_scanline: bool,

    pub oam_pointer: usize,
    pub sprite_buffer: Vec<Sprite>,
    pub obj_checked_tiles: Vec<u8>,
    pub fetching_sprite: bool,
    pub sprite_to_render: Sprite,

    pub renderer: SDLRenderer,
    pub displaybuffer_index: usize,
    pub pixel_fetcher: PixelFetcher,
}

impl PPU  {
    pub fn new() -> Self {
        PPU {
            mode: 2,
            cycles: 0,
            ly: 0,
            x: 0,

            mode_3_penalty: 0,
            obj_penalty: 0,
            rendering_window: false,
            entered_window: false,
            entered_vblank: false,
            stat_irq: false,
            first_irq_on_scanline: false,

            oam_pointer: 0,
            sprite_buffer: Vec::new(),
            obj_checked_tiles: Vec::new(),
            fetching_sprite: false,
            sprite_to_render: Sprite::new(0, 0, 0, 0),

            renderer: SDLRenderer::new(160, 144),
            displaybuffer_index: 0,
            pixel_fetcher: PixelFetcher::new(),
        }
    }

    pub fn tick(&mut self, memory: &mut Memory) {
        self.step(memory);
        self.step(memory);
        self.step(memory);
        self.step(memory);
    }

    pub fn step(&mut self, memory: &mut Memory) {
        let stat = memory.read(0xFF41);
        // println!("{:b}", stat);
        if (stat & 0b01000000 != 0) {
            // println!("STAT ALLOWS LY=LYC");
            if self.ly == memory.read(0xFF45) && !self.first_irq_on_scanline {
                self.stat_irq = true;
                memory.write(0xFF41, stat | 0b0000_0100);
                self.first_irq_on_scanline = true;
                // println!("REQUESTED STAT => LY=LYC LINE => {} FIRST IRQ => {} STAT => {:#010b}", self.ly, self.first_irq_on_scanline, stat);
            } else {
                memory.write(0xFF41, stat & !0b0000_0100);
            }
        }
        //println!("STAT => {} LY => {} LYC => {}", stat, self.ly, memory.read(0xFF45));

        // println!("CYCLES => {} MODE => {} X => {} LY => {} Fetcher Cycles => {}", self.cycles, self.mode, self.x, self.ly, self.pixel_fetcher.cycles);
        self.cycles = self.cycles.wrapping_add(1);
        self.pixel_fetcher.cycles = self.pixel_fetcher.cycles.wrapping_add(1);
        match self.mode {
            0 => self.h_blank(memory),
            1 => self.v_blank(memory),
            2 => self.oam_scan(memory), 
            3 => {
                // println!("FETCHING SPRITE => {}", self.fetching_sprite);
                if self.fetching_sprite {
                    self.mode_3_sprite_fetch(memory);
                } else { 
                    self.mode_3_bgwin_fetch(memory); 
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn inc_ly(&mut self, memory: &mut Memory) {
        self.ly = self.ly.wrapping_add(1);
        let ly = memory.read(0xFF44);
        memory.write(0xFF44, ly.wrapping_add(1));
        self.first_irq_on_scanline = false;
        // println!("NEW LINE, FIRST IRQ => {} LINE => {}", self.first_irq_on_scanline, self.ly);
    }

    pub fn rendering_window(&mut self, memory: &mut Memory) { // Checks if the ppu is rendering the window and returns bool
        let wy = memory.read(0xFF4A);
        let wx = memory.read(0xFF4B).wrapping_sub(8);
        let window_enabled = if memory.read(0xFF40) & 0b00100000 == 0 { false } else { true };
        // println!("LCDC => {:b}", memory.read(0xFF40));
        if self.ly >= wy && self.x >= wx && window_enabled {
            self.rendering_window = true;
            self.pixel_fetcher.rendering_window = true;
            // println!("RENDERING WINDOW");
            if !self.entered_window {
                self.pixel_fetcher.fetcher_x = 0;
                self.pixel_fetcher.bgwin_state = FetcherState::TileNumber;
                self.pixel_fetcher.bgwin_fifo.clear();
                self.entered_window = true;
            }
        } else { self.rendering_window = false; self.pixel_fetcher.rendering_window = false; }
    }

    pub fn h_blank(&mut self, memory: &mut Memory) {
        if self.rendering_window {
            self.rendering_window = false;
            self.pixel_fetcher.rendering_window = false;
            self.pixel_fetcher.window_line_counter = self.pixel_fetcher.window_line_counter.wrapping_add(1);
            self.sprite_buffer.drain(..);
        }
        if self.cycles == 456 {
            if self.ly == 143 { 
                self.set_to_v_blank(memory); 
            } else { 
                self.mode = 2;
                self.pixel_fetcher.bgwin_fifo.clear();
                // println!("NEW SCANLINE");
                let stat = memory.read(0xFF41);
                if stat & 0b0010_0000 != 0 && self.ly != memory.read(0xFF45) {
                    self.stat_irq = true;
                    // println!("REQUESTED STAT IN HBLANK");
                }
                memory.write(0xFF41, (stat & !0b0000_0011) | self.mode)
            }
            self.inc_ly(memory);
            self.entered_window = false;
            self.cycles = 0;
            self.x = 0;
            self.pixel_fetcher.fetcher_x = 0;
            self.mode_3_penalty = 0;
            self.obj_penalty = 0;
        }
    }

    pub fn v_blank(&mut self, memory: &mut Memory) {
        if self.ly == 153 && self.cycles == 456 {
            self.mode = 2;
            self.ly = 0;
            memory.write(0xFF44, 0);
            self.cycles = 0;
            self.x = 0;
            self.renderer.update();
            self.displaybuffer_index = 0;
            self.entered_vblank = false;
            self.pixel_fetcher.window_line_counter = 0;
        }
        if self.cycles == 456 {
            self.inc_ly(memory);
            self.cycles = 0;
        }
    }

    pub fn set_to_v_blank(&mut self, memory: &mut Memory) { 
        self.mode = 1;
        self.entered_vblank = true;
        self.pixel_fetcher.window_line_counter = 0;
        let stat = memory.read(0xFF41);
        if stat & 0b0001_0000 != 0 && self.ly != memory.read(0xFF45) {
            self.stat_irq = true;
            println!("REQUESTED STAT IN VBLANK");
        }
        memory.write(0xFF41, (stat & !0b0000_0011) | self.mode);
    }

    pub fn fetching_sprite(&mut self) -> bool {
        for sprite in &self.sprite_buffer {
            // println!("{} | {}", self.x, sprite.x);
            if sprite.x <= self.x + 8 {
                // println!("FETCHING SPRITE IS TRUE");
                self.pixel_fetcher.bgwin_state = FetcherState::TileNumber;
                self.sprite_to_render = self.sprite_buffer.remove(0);
                return true
            }
        }
        false
    }

    pub fn mode_3_bgwin_fetch(&mut self, memory: &mut Memory) { // this needs redoing to work off t cycles
        match self.pixel_fetcher.bgwin_state {
            FetcherState::TileNumber => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.fetch_tile_number(memory, self.ly);
                    self.pixel_fetcher.bgwin_state = FetcherState::TileDataLow;
                    self.pixel_fetcher.cycles = 0;
                }
            },
            FetcherState::TileDataLow => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.fetch_tile_data_low(memory, self.ly);
                    self.pixel_fetcher.bgwin_state = FetcherState::TileDataHigh;
                    self.pixel_fetcher.cycles = 0;
                }
            },
            FetcherState::TileDataHigh => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.fetch_tile_data_high(memory, self.ly);
                    self.pixel_fetcher.bgwin_state = FetcherState::PushToFifo;
                    self.pixel_fetcher.cycles = 0;
                }
            },
            FetcherState::PushToFifo => { // returns "success" status - true if works. 
                let success = self.pixel_fetcher.push_to_fifo();
                if success {
                    self.pixel_fetcher.bgwin_state = FetcherState::TileNumber;
                    self.pixel_fetcher.cycles = 0;
                    // println!("INCREMENTED FETCHER X => {} X => {}", self.pixel_fetcher.fetcher_x, self.x);
                }
            },
        }

        if !self.pixel_fetcher.bgwin_fifo.is_empty() {
            if self.x == 0 {
                let scx = memory.read(0xFF43);
                let pixels_to_be_dropped = scx % 8;
                for i in 0..pixels_to_be_dropped {
                    self.pixel_fetcher.bgwin_fifo.remove();
                }
            }
            self.push_to_lcd(memory);
            self.x = self.x.wrapping_add(1);
            self.fetching_sprite = self.fetching_sprite();
        }

        if self.x == 160 {
            // self.cycles = 0;
            self.mode = 0;
            self.x = 0;
            let stat = memory.read(0xFF41);
            if memory.read(0xFF41) & 0b0000_1000 != 0 && self.ly != memory.read(0xFF45) {
                self.stat_irq = true;
                // println!("REQUESTED STAT IN MODE3");
            }
            memory.write(0xFF41, (stat & !0b0000_0011) | self.mode);
        }
    }

    pub fn mode_3_sprite_fetch(&mut self, memory: &mut Memory) {
        match self.pixel_fetcher.sprite_state {
            FetcherState::TileNumber => {
                // Does nothing, tile number is read from sprite buffer
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.cycles = 0;
                    self.pixel_fetcher.sprite_state = FetcherState::TileDataLow;
                }
            },
            FetcherState::TileDataLow => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.sprite_fetch_tile_data_low(memory, self.ly, &self.sprite_to_render);
                    self.pixel_fetcher.cycles = 0;
                    self.pixel_fetcher.sprite_state = FetcherState::TileDataHigh;
                }
            },
            FetcherState::TileDataHigh => {
                if self.pixel_fetcher.cycles == 2 {
                    self.pixel_fetcher.sprite_fetch_tile_data_high(memory, self.ly, &self.sprite_to_render);
                    self.pixel_fetcher.cycles = 0;
                    self.pixel_fetcher.sprite_state = FetcherState::PushToFifo;
                }
            },
            FetcherState::PushToFifo => {
                self.pixel_fetcher.push_to_sprite_fifo(&self.sprite_to_render);
                self.pixel_fetcher.sprite_state = FetcherState::TileNumber;
                self.fetching_sprite = false;
                self.pixel_fetcher.cycles = 0;
            }
        }
    }

    pub fn oam_scan(&mut self, memory: &mut Memory) { // pushes a single sprite to oam buffer.
        // THIS ALL NEEDS TO BE CHECKED. CHECKED. CHECKKKEEEDDDDDD. HIGH RISK OF INCORRECT LOGIC.

        if self.cycles == 80 { 
            // println!("ENTERED MODE 3");
            /* println!("SPRITE INDEXES @ {}: ", self.ly);
            for sprite in &self.sprite_buffer {
                print!("{:x} | ", sprite.index);
            }
            println!("");*/
            self.mode = 3;
            self.pixel_fetcher.bgwin_state = FetcherState::TileNumber;
            let stat = memory.read(0xFF41);
            memory.write(0xFF41, (stat & !0b0000_0011) | self.mode);
            // self.cycles = 0;
            self.pixel_fetcher.cycles = 0;
            self.pixel_fetcher.first_tile = true;
            self.pixel_fetcher.bgwin_fifo.clear(); 
            self.pixel_fetcher.sprite_fifo.clear(); 
            self.oam_pointer = 0;

            self.sprite_buffer.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
            return
        }; 

        if self.sprite_buffer.len() == 10 {
            return
        };

        while self.oam_pointer < 40 {
            let height = if (memory.read(0xff40) & 0x4) == 0 { 8 } else { 16 };
            let y = memory.oam[self.oam_pointer * 4];   
            let x = memory.oam[(self.oam_pointer * 4).wrapping_add(1)];

            if ((y.wrapping_add(height) > self.ly.wrapping_add(16)) && (y <= self.ly.wrapping_add(16)) && (x > 0)) { 
                let mut index = memory.oam[(self.oam_pointer * 4).wrapping_add(2)];    
                let attributes = memory.oam[(self.oam_pointer * 4).wrapping_add(3)];

                index = if height == 16 { index & !1 } else { index };
                let sprite = Sprite::new(y, x, index, attributes);
                // println!("PUSHED SPRITE TO SPRITE BUFFER @ {} INDEX => {:x}", self.ly, sprite.index);
                self.sprite_buffer.push(sprite);   
                self.oam_pointer += 1;

                let obj_penalty_sum = 6;
                self.obj_penalty = obj_penalty_sum;
                return
            }
            self.oam_pointer += 1;
        }
    }    

////////////////////////////////////////////////////////////////////

    pub fn push_to_lcd(&mut self, memory: &mut Memory) {
        let lcdc = memory.read(0xFF40);
        let rgb = if !self.pixel_fetcher.sprite_fifo.is_empty() && !self.pixel_fetcher.bgwin_fifo.is_empty() { // mix
            // println!("SPRITE FIFO HAS DATA @ ({}, {})", self.x, self.ly);
            let mut bg_pixel = self.pixel_fetcher.bgwin_fifo.remove().unwrap();
            let mut sprite_pixel = self.pixel_fetcher.sprite_fifo.remove().unwrap();
            bg_pixel.colour_id = if lcdc & 0b0000_0001 == 0 { 0 } else { bg_pixel.colour_id };
            sprite_pixel.colour_id = if lcdc & 0b0000_0010 == 0 { 0 } else { sprite_pixel.colour_id };
            // println!("LCDC => {:#010b} @ ({}, {})", lcdc, self.x, self.ly);
            // println!("COLOUR => {} | PALETTE => {} | PRIORITY => {}", sprite_pixel.colour_id, sprite_pixel.palette, sprite_pixel.priority);

            if sprite_pixel.colour_id == 0 || (sprite_pixel.priority == 1 && bg_pixel.colour_id != 0) {
                let palette = memory.read(bg_pixel.palette); // aka which 2 bits of the palette to use
                let colour = (palette & (0b00000011 << (bg_pixel.colour_id * 2))) >> (bg_pixel.colour_id * 2);
                match colour {
                    0 => 255,
                    1 => 169,
                    2 => 84,
                    3 => 0,
                    _ => unreachable!(),
                }
            } else {
                // println!("RENDERING SPRITE PIXEL @ ({},{})", self.x, self.ly);
                let palette = memory.read(sprite_pixel.palette); // aka which 2 bits of the palette to use
                let colour = (palette & (0b00000011 << (sprite_pixel.colour_id * 2))) >> (sprite_pixel.colour_id * 2);
                match colour {
                    0 => 255,
                    1 => 169,
                    2 => 84,
                    3 => 0,
                    _ => unreachable!(),
                }
            }
        } else { // only bother with bg
            let mut bg_pixel = self.pixel_fetcher.bgwin_fifo.remove().unwrap(); // pixel.colour tells us the id 
            bg_pixel.colour_id = if lcdc & 0b0000_0001 == 0 { 0 } else { bg_pixel.colour_id };
            let palette = memory.read(bg_pixel.palette); // aka which 2 bits of the palette to use
            let colour = (palette & (0b00000011 << (bg_pixel.colour_id * 2))) >> (bg_pixel.colour_id * 2);
            match colour {
                0 => 255,
                1 => 169,
                2 => 84,
                3 => 0,
                _ => unreachable!(),
            }
        };

        self.renderer.displaybuffer[self.displaybuffer_index] = rgb;
        self.displaybuffer_index = self.displaybuffer_index.wrapping_add(1);
        self.renderer.displaybuffer[self.displaybuffer_index] = rgb;
        self.displaybuffer_index = self.displaybuffer_index.wrapping_add(1);
        self.renderer.displaybuffer[self.displaybuffer_index] = rgb;
        self.displaybuffer_index = self.displaybuffer_index.wrapping_add(2);

        self.rendering_window(memory);
    }
}