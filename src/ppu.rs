use crate::memory::Memory;
use crate::registers::*;
use crate::cpu::CPU;

const KIB:usize = 1024;

//////////////////////////////// SDL2 ////////////////////////////////

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
            PixelFormatEnum::ARGB8888,
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

//////////////////////////////// PPU ////////////////////////////////

pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Pixel {
            r, g, b, a
        }
    }
}

pub struct PPU {
    pub mode: u8,
    pub lcdc: u8,
    pub ly: u8, // indicates current Y
    pub x: u8, // indicates current X
    pub scy: u8,
    pub scx: u8,

    pub viewport: [[u8; 160]; 144], // Index coord (x, y) with viewport[y][x]
    pub oam_buffer: Vec<Sprite>,
    pub tilemap_row: [u8; 32], // stores pointer (index) of background or window pixel to be drawn. 
    pub pixelmap: [[u8; 256]; 256],
    pub renderer: SDLRenderer,
}

impl PPU {
    pub fn new(cpu: &CPU) -> Self {
        let r = SDLRenderer::new(160, 144);

        PPU {
            mode: 2,
            lcdc: cpu.memory.read(0xff40),
            ly: cpu.memory.read(0xff44),
            x: 0,
            scy: cpu.memory.read(0xff42),
            scx: cpu.memory.read(0xff43),

            viewport: [[0; 160]; 144],
            oam_buffer: Vec::new(),
            tilemap_row: [0; 32],
            pixelmap: [[0; 256]; 256],
            renderer: r,
        }
    }

    // mode 2 related functions

    pub fn sprite_scan(&mut self, oam: [u8; 0xA0]) {
        let mut pointer:usize = 0;

        for byte in 0..0x28 { // length of oam / 4 to loop for the number of sprites.
            let height = if (self.lcdc & 0x4) == 0 { 8 } else { 16 }; // check if 8x8 or 8x16

            let y = oam[pointer];    
            let x = oam[pointer + 1];
            let index = oam[pointer + 2];    
            let attributes = oam[pointer + 3];
            let sprite = Sprite::new(y, x, index, attributes);

            if ((y + height > self.ly + 16) && (y <= self.ly + 16) && (x > 0)) { // check if has pixels on current row
                self.oam_buffer.push(sprite); // push sprite to oam buffer
                if self.oam_buffer.len() == 10 { return } // check not reached 10 sprites yet (hardware limit)
            }
            pointer += 4;
        }
    }

    pub fn tilemap_scan(&mut self, cpu: &CPU, wx: u8, wy: u8) {
        let window_map_pointer = if (self.lcdc & 128) == 0 { 0x9800 } else { 0x9C00 }; // checks which tilemap to use
        let bg_map_pointer = if (self.lcdc & 8) == 0 { 0x9800 } else { 0x9C00 };

        for byte in 0..32 {
            let mut x = byte as u16;
            if self.ly >= wy && self.lcdc & 32 != 0 { // check if its equal or below top right most pixel of window, check if window enabled
                if byte <= wx { // check if its equal or below to left most pixel of window
                    self.tilemap_row[x as usize] = cpu.memory.read((window_map_pointer + x + 32 as u16 * (self.ly % 8) as u16) as u16); // if it is, get index value for window
                } 
                else { self.tilemap_row[x as usize] = cpu.memory.read((bg_map_pointer + x + 32 as u16 * (self.ly % 8) as u16) as u16); } // if its not, get index for bg
            } 
            else { self.tilemap_row[x as usize] = cpu.memory.read((bg_map_pointer + x + 32 as u16 * (self.ly % 8) as u16) as u16); } // ^^^
            x += 1
        }
    }

    pub fn mode_2(&mut self, cpu: &CPU) {
        self.sprite_scan(cpu.memory.oam); // create buffer of sprites
        self.tilemap_scan(cpu, cpu.memory.read(0xFF4A), cpu.memory.read(0xFF4B).wrapping_sub(7)); // create buffer of win / bg indexes
    }

    pub fn mode_3(&mut self, cpu: &CPU) {
        self.lcdc = cpu.memory.read(0xff40);
        self.ly = cpu.memory.read(0xff44);

        for tile in self.tilemap_row {
            let bytes_index = if self.lcdc & 16 == 0 && tile < 128 { 0x9000 + tile as u16 * 16 }
            else { 0x8000 + tile as u16 * 16 };
            let mut byte_a = cpu.memory.read(bytes_index); // lower byte of bgwin row
            let mut byte_b = cpu.memory.read(bytes_index + 1); // upper byte of bgwin row
            for pixel in 0..7 {
                byte_a = ((byte_a >> (7 - pixel)) & 0b1) << 1;
                byte_b = (byte_b >> (7 - pixel)) & 0b1;
                self.pixelmap[self.ly as usize][self.x as usize] = byte_a | byte_b;  
                self.x += 1;
            }
        }
        self.x = 0;

        for sprite in &self.oam_buffer { // need to add sprite prioritisation
            if sprite.x == self.x {
                let bytes_index = sprite.index + (2*(self.ly - sprite.y) % 8); // get the pointer for sprite data on that row
                let mut byte_a = cpu.memory.vram[bytes_index as usize]; // lower byte of sprite row
                let mut byte_b = cpu.memory.vram[bytes_index as usize + 1]; // upper byte of sprite row
                for pixel in 0..7 {
                    byte_a = ((byte_a >> (7 - pixel)) & 0b1) << 1;
                    byte_b = (byte_b >> (7 - pixel)) & 0b1;
                    self.pixelmap[self.ly as usize][self.x as usize] = byte_a | byte_b | 128 | sprite.attributes & 0b00010000 << 2; // signs the pixelmap byte as a sprite pixel  
                    self.x += 1;
                }
            }
        }
        self.x = 0;
    }

    pub fn draw_frame(&mut self, cpu: &CPU) { // All palette related logic is completely wrong. Palettes can change. 
        for row in 0..self.pixelmap.len() {
            self.mode_2(&cpu);
            self.mode_3(&cpu);
            self.ly += 1;
        }

        let scy = cpu.memory.read(0xff42);
        let scx = cpu.memory.read(0xff43);

        let mut y = 0;
        let mut x = 0;
        for i in 0..(self.renderer.width as usize * self.renderer.height as usize * SDLRenderer::PIXELSIZE) { 
            // the below logic is wrong. Only works for bg pixels.
            let mut colour_id = self.pixelmap[((scy + y) as u16 % 256) as usize][((scx + x) as u16 % 256) as usize];

            let mut pixel = Pixel::new(0, 0, 0, 0);

            if colour_id & 0b10000000 == 0 { // is bg/win
                let palette_value = (cpu.memory.read(0xff47) & (0b00000011 << (colour_id*2))) >> (colour_id*2);
                let colour = match palette_value {
                    0 => 0,
                    1 => 84,
                    2 => 169,
                    3 => 255,
                    _ => 255,
                };
                pixel.r = colour;
                pixel.g = colour;
                pixel.b = colour;
                pixel.a = 0;
            } else { // is sprite 
                colour_id &= 0b00000011; 
                let palette_value = (cpu.memory.read(0xff48 + ((colour_id & 0b01000000) >> 6) as u16) & (0b00000011 << (colour_id*2))) >> (colour_id*2);
                let colour = match palette_value {
                    0 => 0,
                    1 => 84,
                    2 => 169,
                    3 => 255,
                    _ => 255,
                };
                pixel.r = colour;
                pixel.g = colour;
                pixel.b = colour;
                pixel.a = if colour == 0 { 255 } else { 0 };
            }

            self.renderer.displaybuffer[i*4] = pixel.r;
            self.renderer.displaybuffer[i*4 + 1] = pixel.g;
            self.renderer.displaybuffer[i*4 + 2] = pixel.b;
            self.renderer.displaybuffer[i*4 + 3] = pixel.a;

            x += 1;
            if x == 143 { y += 1; x = 0;}
            if y == 159 { break; }
        }
        self.renderer.update();
    }
}