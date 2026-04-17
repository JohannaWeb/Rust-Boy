use crate::cartridge::Cartridge;
use crate::emulator::EmulatorMode;
use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::timer::Timer;

pub const INT_VBLANK: u8 = 0x01;
pub const INT_STAT: u8 = 0x02;
pub const INT_TIMER: u8 = 0x04;
pub const INT_JOYPAD: u8 = 0x10;

pub struct Bus {
    pub cartridge: Cartridge,
    pub ppu: Ppu,
    pub timer: Timer,
    pub joypad: Joypad,
    wram: [[u8; 0x1000]; 8],
    hram: [u8; 0x7f],
    wram_bank: usize,
    pub interrupt_enable: u8,
    pub interrupt_flag: u8,
    pub key1: u8,
    serial_data: u8,
    serial_control: u8,
    hdma: Hdma,
    boot_rom_disabled: bool,
    mode: EmulatorMode,
}

impl Bus {
    pub fn new(cartridge: Cartridge, mode: EmulatorMode) -> Self {
        Self {
            cartridge,
            ppu: Ppu::new(mode),
            timer: Timer::new(),
            joypad: Joypad::new(),
            wram: [[0; 0x1000]; 8],
            hram: [0; 0x7f],
            wram_bank: 1,
            interrupt_enable: 0,
            interrupt_flag: 0xe1,
            key1: if mode == EmulatorMode::Cgb {
                0x7e
            } else {
                0xff
            },
            serial_data: 0,
            serial_control: 0x7e,
            hdma: Hdma::default(),
            boot_rom_disabled: true,
            mode,
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        self.timer.tick(cycles);
        self.ppu.tick(cycles);
        if self.timer.take_interrupt() {
            self.request_interrupt(INT_TIMER);
        }
        if self.ppu.take_vblank_interrupt() {
            self.request_interrupt(INT_VBLANK);
        }
        if self.ppu.take_stat_interrupt() {
            self.request_interrupt(INT_STAT);
        }
        if self.joypad.take_interrupt() {
            self.request_interrupt(INT_JOYPAD);
        }
    }

    pub fn request_interrupt(&mut self, mask: u8) {
        self.interrupt_flag |= mask;
    }

    pub fn read8(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => self.cartridge.read_rom(addr),
            0x8000..=0x9fff => self.ppu.read_vram(addr),
            0xa000..=0xbfff => self.cartridge.read_ram(addr),
            0xc000..=0xcfff => self.wram[0][addr as usize - 0xc000],
            0xd000..=0xdfff => self.wram[self.wram_bank][addr as usize - 0xd000],
            0xe000..=0xfdff => self.read8(addr - 0x2000),
            0xfe00..=0xfe9f => self.ppu.read_oam(addr),
            0xfea0..=0xfeff => 0xff,
            0xff00 => self.joypad.read(),
            0xff01 => self.serial_data,
            0xff02 => self.serial_control,
            0xff04..=0xff07 => self.timer.read(addr),
            0xff0f => self.interrupt_flag | 0xe0,
            0xff40..=0xff4b | 0xff4f | 0xff68..=0xff6b => self.ppu.read_register(addr),
            0xff4d => self.key1,
            0xff51 => self.hdma.src_hi,
            0xff52 => self.hdma.src_lo,
            0xff53 => self.hdma.dst_hi,
            0xff54 => self.hdma.dst_lo,
            0xff55 => self.hdma.status,
            0xff50 => self.boot_rom_disabled as u8,
            0xff70 => {
                if self.mode == EmulatorMode::Cgb {
                    0xf8 | self.wram_bank as u8
                } else {
                    0xff
                }
            }
            0xff80..=0xfffe => self.hram[addr as usize - 0xff80],
            0xffff => self.interrupt_enable,
            _ => 0xff,
        }
    }

    pub fn write8(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7fff => self.cartridge.write_rom(addr, value),
            0x8000..=0x9fff => self.ppu.write_vram(addr, value),
            0xa000..=0xbfff => self.cartridge.write_ram(addr, value),
            0xc000..=0xcfff => self.wram[0][addr as usize - 0xc000] = value,
            0xd000..=0xdfff => self.wram[self.wram_bank][addr as usize - 0xd000] = value,
            0xe000..=0xfdff => self.write8(addr - 0x2000, value),
            0xfe00..=0xfe9f => self.ppu.write_oam(addr, value),
            0xfea0..=0xfeff => {}
            0xff00 => self.joypad.write(value),
            0xff01 => self.serial_data = value,
            0xff02 => self.serial_control = value,
            0xff04..=0xff07 => self.timer.write(addr, value),
            0xff0f => self.interrupt_flag = value | 0xe0,
            0xff40..=0xff4b | 0xff4f | 0xff68..=0xff6b => {
                self.ppu.write_register(addr, value);
                if addr == 0xff46 {
                    self.dma_transfer(value);
                }
            }
            0xff4d => self.key1 = (self.key1 & 0x80) | (value & 0x01) | 0x7e,
            0xff51 => self.hdma.src_hi = value,
            0xff52 => self.hdma.src_lo = value & 0xf0,
            0xff53 => self.hdma.dst_hi = value & 0x1f,
            0xff54 => self.hdma.dst_lo = value & 0xf0,
            0xff55 => self.cgb_dma_transfer(value),
            0xff50 => self.boot_rom_disabled = value != 0,
            0xff70 => {
                if self.mode == EmulatorMode::Cgb {
                    self.wram_bank = (value as usize & 0x07).max(1);
                }
            }
            0xff80..=0xfffe => self.hram[addr as usize - 0xff80] = value,
            0xffff => self.interrupt_enable = value,
            _ => {}
        }
    }

    pub fn read16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.read8(addr), self.read8(addr.wrapping_add(1))])
    }

    pub fn write16(&mut self, addr: u16, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.write8(addr, lo);
        self.write8(addr.wrapping_add(1), hi);
    }

    pub fn stop_speed_switch(&mut self) {
        if self.mode == EmulatorMode::Cgb && self.key1 & 0x01 != 0 {
            self.key1 = (self.key1 ^ 0x80) & !0x01 | 0x7e;
        }
    }

    fn dma_transfer(&mut self, value: u8) {
        let base = (value as u16) << 8;
        for offset in 0..0xa0u16 {
            let byte = self.read8(base + offset);
            self.ppu.write_oam(0xfe00 + offset, byte);
        }
    }

    fn cgb_dma_transfer(&mut self, value: u8) {
        if self.mode != EmulatorMode::Cgb {
            return;
        }

        let blocks = (value & 0x7f) as u16 + 1;
        let mut source = u16::from_be_bytes([self.hdma.src_hi, self.hdma.src_lo]) & 0xfff0;
        let mut dest = 0x8000 | (((self.hdma.dst_hi as u16) & 0x1f) << 8) | self.hdma.dst_lo as u16;

        for _ in 0..blocks {
            for offset in 0..0x10u16 {
                let byte = self.read8(source.wrapping_add(offset));
                self.write8(dest.wrapping_add(offset), byte);
            }
            source = source.wrapping_add(0x10);
            dest = 0x8000 | ((dest.wrapping_add(0x10) - 0x8000) & 0x1fff);
        }

        let remaining = 0x7f;
        self.hdma.src_hi = (source >> 8) as u8;
        self.hdma.src_lo = source as u8 & 0xf0;
        self.hdma.dst_hi = ((dest - 0x8000) >> 8) as u8 & 0x1f;
        self.hdma.dst_lo = dest as u8 & 0xf0;
        self.hdma.status = remaining | 0x80;
    }
}

#[derive(Debug, Clone, Default)]
struct Hdma {
    src_hi: u8,
    src_lo: u8,
    dst_hi: u8,
    dst_lo: u8,
    status: u8,
}
