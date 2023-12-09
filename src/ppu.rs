const KIB:usize = 1024;

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

/////////////////////////////// PPU ////////////////////////////////

pub struct PPU {
    pub mode: u8,
    pub cycles: u16,
    pub ly: u8,
    x: u8,
    pub oam_pointer: usize,

    // Buffers
    pub viewport: [[u8; 160]; 144], // Index coord (x, y) with viewport[y][x] - MAY BE DISREGARDED
    pub oam_buffer: Vec<Sprite>,
    pub bg_tile_row: [u8; 32], // stores pointer (index) of background tile to be drawn. - MAY BE DISREGARDED
    pub bg_row: [u8; 256],
    pub renderer: SDLRenderer,
}

impl PPU  {
    pub fn new() -> Self {
        let r = SDLRenderer::new(160, 144);

        PPU {
            mode: 2,
            cycles: 0,
            ly: 0,
            x: 0,
            oam_pointer: 0,

            viewport: [[0; 160]; 144],
            oam_buffer: Vec::new(),
            bg_tile_row: [0; 32],
            bg_row: [0; 256],
            renderer: r,
        }
    }

    pub fn tick(&mut self, memory: &Memory) {
        self.cycles = self.cycles.wrapping_add(4);

        if self.cycles % 456 == 0 { // inc ly
            self.ly = self.ly.wrapping_add(1);
            self.create_bg_row(memory);
            if self.ly == 143 {
                self.mode = 1;
            }
        }

        self.step(memory);
    }

    ///// IMPORTANT /////
    // Note that my mode 3 is NOT the same as the hardware mode 3. My pixels are "drawn to the LCD" via renderer update.
    // My mode 3 pushes a row of pixels to the pixel map. NOT THE LCD.
    // then, when the PPU is ticked, if the line progresses, the current line must be complete in pixel map.
    // the line is pushed to displaybuffer, then the renderer is updated. 
    // tldr; hardware mode 3 pushes to LCD, my mode 3 pushes to pixelmap, my ppu draws line when line complete.

    // Process:
    // Get sprites to be drawn. Store sprite struct in sprite buffer.
    // Generate array of bg pixels to be drawn... make this as wide as the viewport.
    // -> Create a temporary array of 32 in create_bg_row. Turn this into 20 (i think?) viewport width by chopping at scx
    // -> Make sure that this array is generated on the right row... scy + ly? i think?
    // -> This means can also add window onto row. Do this after and overwrite. 
    // Merge sprite array and bg/win array. Push result to lcd. 

    pub fn step(&mut self, memory: &Memory) {
        match self.mode {
            0 => { self.h_blank(); },
            1 => { self.v_blank(); },
            2 => { self.oam_scan(memory); self.oam_scan(memory); },
            3 => { self.pixel_push(memory); self.pixel_push(memory); self.pixel_push(memory); self.pixel_push(memory); },
            _ => { unreachable!() },
        }
    }

    pub fn h_blank(&mut self) { // wait till end of line before skipping to next mode
        if self.cycles % 456 == 0 { // check if its end of line
            self.mode = 2;
        }
    }

    pub fn v_blank(&mut self) {
        if self.ly == 153 { // reset cycles / ly at end of frame, step occurs after tick inc ly
            self.cycles = 0; // meaning that on the next step, if i reset ly, ly will then inc and be 1. 
            self.ly = 255; // at the start of mode 2. therefore i set it 255, as +1 = 0. 
            self.mode = 2;
        }
    }

    pub fn oam_scan(&mut self, memory: &Memory) { // pushes a single sprite to oam buffer.
        // THIS ALL NEEDS TO BE CHECKED. CHECKED. CHECKKKEEEDDDDDD. HIGH RISK OF INCORRECT LOGIC.
        if self.oam_buffer.len() == 10 {
            return
        }

        while self.oam_pointer < 40 {
            let height = if (memory.read(0xff40) & 0x4) == 0 { 8 } else { 16 };
            let y = memory.oam[self.oam_pointer * 4];   
            let x = memory.oam[(self.oam_pointer * 4).wrapping_add(1)];

            if ((y + height > self.ly.wrapping_add(16)) && (y <= self.ly.wrapping_add(16)) && (x > 0)) { 
                let index = memory.oam[(self.oam_pointer * 4).wrapping_add(2)];    
                let attributes = memory.oam[(self.oam_pointer * 4).wrapping_add(3)];
        
                let sprite = Sprite::new(y, x, index, attributes);
                self.oam_buffer.push(sprite);   
                self.oam_pointer += 1;
                return
            }
            self.oam_pointer += 1;
        }
    }    

    pub fn create_bg_row(&mut self, memory: &Memory) { // Note that ly refers to the row of the viewport, and doesnt exceed 153
        let lcdc = memory.read(0xff40); // Therefore, ly cannot be used to reference pixelmap. It may help to ditch the pixelmap.
        let bg_map_pointer = if (lcdc & 8) == 0 { 0x9800 } else { 0x9C00 }; // And instead use separate logic with scy / scx. 
        let bg_data_pointer = if (lcdc & 16) == 0 { 0x9000 } else { 0x8000 };
        let scy = memory.read(0xff42);

        let pixelmap_row = scy.wrapping_add(self.ly);
        let tilemap_row = (pixelmap_row / 8_u8) as u16; // Rust will force this to be u8, i.e ignore decimals.

        for x in 0..32_u16 { // for each tile in tilemap_row
            self.bg_tile_row[x as usize] =  memory.read(bg_map_pointer + tilemap_row*32 + x);
        }

        let mut pixel_x = 0;
        for tile in self.bg_tile_row {
            let row_of_tile = pixelmap_row % 8;
            let byte_index = (tile + row_of_tile*2) as u16; 
            let byte_a = memory.read(byte_index);
            let byte_b = memory.read(byte_index + 1);

            for pixel in 0..7 {
                let pixel = byte_a & (0b00000001 << (7 - pixel)) >> (6 - pixel) | (byte_b & 0b00000001 << (7 - pixel)) >> (7 - pixel);
                self.bg_row[pixel_x] = pixel;
                pixel_x += 1;
            }
        }
        // In theory... this should generate a 255 len array of pixels for the current row...
    }

    pub fn pixel_push(&mut self, memory: &Memory) { // THIS SHOULD... SHOOOOUUULD... DRAW THE BACKGROUND
        if self.x != 160 {
            let scx = memory.read(0xff32);
            self.renderer.displaybuffer[(self.ly*160).wrapping_add(self.x) as usize] = self.bg_row[scx.wrapping_add(self.x) as usize];
            self.x += 1;
        } else { self.mode = 0; self.x = 0; }
    }
}