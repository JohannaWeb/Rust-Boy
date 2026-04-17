use crate::emulator::EmulatorMode;

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

const DMG_PALETTE: [u32; 4] = [0xffe0f8d0, 0xff88c070, 0xff346856, 0xff081820];

#[derive(Debug, Clone)]
pub struct Ppu {
    mode: EmulatorMode,
    vram: [[u8; 0x2000]; 2],
    oam: [u8; 0xa0],
    framebuffer: [u32; WIDTH * HEIGHT],
    cycles: u32,
    vram_bank: usize,
    pub lcdc: u8,
    pub stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub dma: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub wy: u8,
    pub wx: u8,
    pub bgpi: u8,
    pub obpi: u8,
    bg_palette: [u8; 0x40],
    obj_palette: [u8; 0x40],
    vblank_interrupt: bool,
    stat_interrupt: bool,
    frame_ready: bool,
}

impl Ppu {
    pub fn new(mode: EmulatorMode) -> Self {
        Self {
            mode,
            vram: [[0; 0x2000]; 2],
            oam: [0; 0xa0],
            framebuffer: [0xffffffff; WIDTH * HEIGHT],
            cycles: 0,
            vram_bank: 0,
            lcdc: 0x91,
            stat: 0x85,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0xff,
            bgp: 0xfc,
            obp0: 0xff,
            obp1: 0xff,
            wy: 0,
            wx: 0,
            bgpi: 0,
            obpi: 0,
            bg_palette: default_cgb_palette(),
            obj_palette: default_cgb_palette(),
            vblank_interrupt: false,
            stat_interrupt: false,
            frame_ready: false,
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        if self.lcdc & 0x80 == 0 {
            self.cycles = 0;
            self.ly = 0;
            self.stat = (self.stat & !0x03) | 0;
            return;
        }

        self.cycles += cycles;
        while self.cycles >= 456 {
            self.cycles -= 456;
            if self.ly < 144 {
                self.render_scanline();
            }
            self.ly = self.ly.wrapping_add(1);
            if self.ly == 144 {
                self.vblank_interrupt = true;
                self.frame_ready = true;
            } else if self.ly > 153 {
                self.ly = 0;
                self.frame_ready = false;
            }
            if self.ly == self.lyc && self.stat & 0x40 != 0 {
                self.stat_interrupt = true;
            }
        }

        let mode = if self.ly >= 144 {
            1
        } else if self.cycles < 80 {
            2
        } else if self.cycles < 252 {
            3
        } else {
            0
        };
        self.stat = (self.stat & !0x07) | mode | if self.ly == self.lyc { 0x04 } else { 0 };
    }

    pub fn frame_buffer(&self) -> &[u32] {
        &self.framebuffer
    }

    pub fn vram_bank(&self, bank: usize) -> Option<&[u8; 0x2000]> {
        self.vram.get(bank)
    }

    pub fn bg_palette_data(&self) -> &[u8; 0x40] {
        &self.bg_palette
    }

    pub fn obj_palette_data(&self) -> &[u8; 0x40] {
        &self.obj_palette
    }

    pub fn oam_data(&self) -> &[u8; 0xa0] {
        &self.oam
    }

    pub fn bg_color_id_counts(&self) -> [usize; 4] {
        let mut counts = [0; 4];
        if self.lcdc & 0x01 == 0 {
            counts[0] = WIDTH * HEIGHT;
            return counts;
        }

        let tile_map_base = if self.lcdc & 0x08 != 0 {
            0x1c00
        } else {
            0x1800
        };
        let tile_data_signed = self.lcdc & 0x10 == 0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let bg_x = self.scx.wrapping_add(x as u8) as usize;
                let bg_y = self.scy.wrapping_add(y as u8) as usize;
                let tile_x = bg_x / 8;
                let tile_y = bg_y / 8;
                let tile_index_addr = tile_map_base + tile_y * 32 + tile_x;
                let tile_num = self.vram[0][tile_index_addr & 0x1fff];
                let attr = if self.mode == EmulatorMode::Cgb {
                    self.vram[1][tile_index_addr & 0x1fff]
                } else {
                    0
                };
                let bank = if self.mode == EmulatorMode::Cgb && attr & 0x08 != 0 {
                    1
                } else {
                    0
                };
                let line = if attr & 0x40 != 0 {
                    7 - (bg_y % 8)
                } else {
                    bg_y % 8
                };
                let col = if attr & 0x20 != 0 {
                    x % 8
                } else {
                    7 - (bg_x % 8)
                };
                let tile_addr = tile_addr(tile_num, tile_data_signed, line);
                let lo = self.vram[bank][tile_addr & 0x1fff];
                let hi = self.vram[bank][(tile_addr + 1) & 0x1fff];
                let color_id = ((lo >> col) & 1) | (((hi >> col) & 1) << 1);
                counts[color_id as usize] += 1;
            }
        }
        counts
    }

    pub fn take_vblank_interrupt(&mut self) -> bool {
        let value = self.vblank_interrupt;
        self.vblank_interrupt = false;
        value
    }

    pub fn take_stat_interrupt(&mut self) -> bool {
        let value = self.stat_interrupt;
        self.stat_interrupt = false;
        value
    }

    pub fn frame_ready(&self) -> bool {
        self.frame_ready
    }

    pub fn take_frame_ready(&mut self) -> bool {
        let value = self.frame_ready;
        self.frame_ready = false;
        value
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[self.vram_bank][addr as usize - 0x8000]
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[self.vram_bank][addr as usize - 0x8000] = value;
    }

    pub fn read_oam(&self, addr: u16) -> u8 {
        self.oam[addr as usize - 0xfe00]
    }

    pub fn write_oam(&mut self, addr: u16, value: u8) {
        self.oam[addr as usize - 0xfe00] = value;
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xff40 => self.lcdc,
            0xff41 => self.stat | 0x80,
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff46 => self.dma,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,
            0xff4f => 0xfe | self.vram_bank as u8,
            0xff68 => self.bgpi,
            0xff69 => self.bg_palette[(self.bgpi & 0x3f) as usize],
            0xff6a => self.obpi,
            0xff6b => self.obj_palette[(self.obpi & 0x3f) as usize],
            _ => 0xff,
        }
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xff40 => self.lcdc = value,
            0xff41 => self.stat = (self.stat & 0x07) | (value & 0x78),
            0xff42 => self.scy = value,
            0xff43 => self.scx = value,
            0xff44 => self.ly = 0,
            0xff45 => self.lyc = value,
            0xff46 => self.dma = value,
            0xff47 => self.bgp = value,
            0xff48 => self.obp0 = value,
            0xff49 => self.obp1 = value,
            0xff4a => self.wy = value,
            0xff4b => self.wx = value,
            0xff4f => {
                self.vram_bank = if self.mode == EmulatorMode::Cgb {
                    (value & 1) as usize
                } else {
                    0
                }
            }
            0xff68 => self.bgpi = value,
            0xff69 => self.write_palette(true, value),
            0xff6a => self.obpi = value,
            0xff6b => self.write_palette(false, value),
            _ => {}
        }
    }

    fn write_palette(&mut self, bg: bool, value: u8) {
        let index = if bg { self.bgpi } else { self.obpi } & 0x3f;
        let palette = if bg {
            &mut self.bg_palette
        } else {
            &mut self.obj_palette
        };
        palette[index as usize] = value;
        if bg && self.bgpi & 0x80 != 0 {
            self.bgpi = 0x80 | ((index + 1) & 0x3f);
        } else if !bg && self.obpi & 0x80 != 0 {
            self.obpi = 0x80 | ((index + 1) & 0x3f);
        }
    }

    fn render_scanline(&mut self) {
        let mut bg_color_ids = [0u8; WIDTH];
        if self.lcdc & 0x01 == 0 {
            self.framebuffer[self.ly as usize * WIDTH..(self.ly as usize + 1) * WIDTH]
                .fill(DMG_PALETTE[0]);
        } else {
            self.render_bg_scanline(&mut bg_color_ids);
        }

        if self.lcdc & 0x02 != 0 {
            self.render_obj_scanline(&bg_color_ids);
        }
    }

    fn render_bg_scanline(&mut self, bg_color_ids: &mut [u8; WIDTH]) {
        let y = self.ly as usize;
        let tile_map_base = if self.lcdc & 0x08 != 0 {
            0x1c00
        } else {
            0x1800
        };
        let tile_data_signed = self.lcdc & 0x10 == 0;

        for x in 0..WIDTH {
            let bg_x = self.scx.wrapping_add(x as u8) as usize;
            let bg_y = self.scy.wrapping_add(y as u8) as usize;
            let tile_x = bg_x / 8;
            let tile_y = bg_y / 8;
            let tile_index_addr = tile_map_base + tile_y * 32 + tile_x;
            let tile_num = self.vram[0][tile_index_addr & 0x1fff];
            let attr = if self.mode == EmulatorMode::Cgb {
                self.vram[1][tile_index_addr & 0x1fff]
            } else {
                0
            };
            let bank = if self.mode == EmulatorMode::Cgb && attr & 0x08 != 0 {
                1
            } else {
                0
            };
            let line = if attr & 0x40 != 0 {
                7 - (bg_y % 8)
            } else {
                bg_y % 8
            };
            let col = if attr & 0x20 != 0 {
                x % 8
            } else {
                7 - (bg_x % 8)
            };
            let tile_addr = tile_addr(tile_num, tile_data_signed, line);
            let lo = self.vram[bank][tile_addr & 0x1fff];
            let hi = self.vram[bank][(tile_addr + 1) & 0x1fff];
            let color_id = ((lo >> col) & 1) | (((hi >> col) & 1) << 1);
            bg_color_ids[x] = color_id;
            let color = if self.mode == EmulatorMode::Cgb {
                self.cgb_color(&self.bg_palette, (attr & 0x07) as usize, color_id as usize)
            } else {
                let shade = (self.bgp >> (color_id * 2)) & 0x03;
                DMG_PALETTE[shade as usize]
            };
            self.framebuffer[y * WIDTH + x] = color;
        }
    }

    fn render_obj_scanline(&mut self, bg_color_ids: &[u8; WIDTH]) {
        let y = self.ly as i16;
        let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
        let mut sprites = Vec::new();

        for index in 0..40 {
            let base = index * 4;
            let obj_y = self.oam[base] as i16 - 16;
            let obj_x = self.oam[base + 1] as i16 - 8;
            if y >= obj_y && y < obj_y + sprite_height {
                sprites.push((index, obj_x, obj_y));
                if sprites.len() == 10 {
                    break;
                }
            }
        }

        if self.mode == EmulatorMode::Dmg {
            sprites.sort_by_key(|&(index, x, _)| (x, index));
        } else {
            sprites.sort_by_key(|&(index, _, _)| index);
        }

        for &(index, obj_x, obj_y) in sprites.iter().rev() {
            let base = index * 4;
            let mut tile = self.oam[base + 2];
            let attr = self.oam[base + 3];
            if sprite_height == 16 {
                tile &= 0xfe;
            }

            let mut line = (y - obj_y) as usize;
            if attr & 0x40 != 0 {
                line = sprite_height as usize - 1 - line;
            }
            let bank = if self.mode == EmulatorMode::Cgb && attr & 0x08 != 0 {
                1
            } else {
                0
            };
            let tile_addr = tile as usize * 16 + line * 2;
            let lo = self.vram[bank][tile_addr & 0x1fff];
            let hi = self.vram[bank][(tile_addr + 1) & 0x1fff];

            for px in 0..8 {
                let screen_x = obj_x + px;
                if !(0..WIDTH as i16).contains(&screen_x) {
                    continue;
                }
                let bit = if attr & 0x20 != 0 { px } else { 7 - px };
                let color_id = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
                if color_id == 0 {
                    continue;
                }
                let x = screen_x as usize;
                if attr & 0x80 != 0 && bg_color_ids[x] != 0 {
                    continue;
                }

                let color = if self.mode == EmulatorMode::Cgb {
                    self.cgb_color(&self.obj_palette, (attr & 0x07) as usize, color_id as usize)
                } else {
                    let palette = if attr & 0x10 != 0 {
                        self.obp1
                    } else {
                        self.obp0
                    };
                    let shade = (palette >> (color_id * 2)) & 0x03;
                    DMG_PALETTE[shade as usize]
                };
                self.framebuffer[self.ly as usize * WIDTH + x] = color;
            }
        }
    }

    fn cgb_color(&self, palette: &[u8; 0x40], palette_index: usize, color_index: usize) -> u32 {
        let offset = palette_index * 8 + color_index * 2;
        let raw = u16::from_le_bytes([palette[offset], palette[offset + 1]]);
        let r = ((raw & 0x1f) as u32 * 255) / 31;
        let g = (((raw >> 5) & 0x1f) as u32 * 255) / 31;
        let b = (((raw >> 10) & 0x1f) as u32 * 255) / 31;
        0xff000000 | (r << 16) | (g << 8) | b
    }
}

fn tile_addr(tile_num: u8, tile_data_signed: bool, line: usize) -> usize {
    if tile_data_signed {
        let signed = tile_num as i8 as i16;
        (0x1000i16 + signed * 16 + line as i16 * 2) as usize
    } else {
        tile_num as usize * 16 + line * 2
    }
}

fn default_cgb_palette() -> [u8; 0x40] {
    let mut palette = [0; 0x40];
    for color in 0..32 {
        let raw = match color % 4 {
            0 => 0x7fff,
            1 => 0x56b5,
            2 => 0x2d6b,
            _ => 0x0000,
        };
        let offset = color * 2;
        palette[offset] = raw as u8;
        palette[offset + 1] = (raw >> 8) as u8;
    }
    palette
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new(EmulatorMode::Cgb)
    }
}
