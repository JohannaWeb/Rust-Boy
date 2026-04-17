use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum CartridgeError {
    Io(std::io::Error),
    TooSmall,
    UnsupportedRomSize(u8),
    UnsupportedRamSize(u8),
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::TooSmall => write!(f, "ROM is smaller than a Game Boy cartridge header"),
            Self::UnsupportedRomSize(code) => write!(f, "unsupported ROM size code 0x{code:02x}"),
            Self::UnsupportedRamSize(code) => write!(f, "unsupported RAM size code 0x{code:02x}"),
        }
    }
}

impl std::error::Error for CartridgeError {}

impl From<std::io::Error> for CartridgeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeKind {
    RomOnly,
    Mbc1,
    Mbc3,
    Mbc5,
    Unknown(u8),
}

#[derive(Debug, Clone)]
pub struct CartridgeHeader {
    pub title: String,
    pub cgb_capable: bool,
    pub cgb_only: bool,
    pub cartridge_type: u8,
    pub rom_banks: usize,
    pub ram_banks: usize,
}

pub struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    header: CartridgeHeader,
    kind: CartridgeKind,
    ram_enabled: bool,
    rom_bank: usize,
    ram_bank: usize,
    banking_mode: u8,
}

impl Cartridge {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, CartridgeError> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn from_bytes(mut rom: Vec<u8>) -> Result<Self, CartridgeError> {
        if rom.len() < 0x150 {
            return Err(CartridgeError::TooSmall);
        }

        let cartridge_type = rom[0x147];
        let rom_banks = rom_banks(rom[0x148])?;
        let ram_banks = ram_banks(rom[0x149])?;
        let expected_rom_len = rom_banks * 0x4000;
        if rom.len() < expected_rom_len {
            rom.resize(expected_rom_len, 0xff);
        }

        let title_end = (0x134..=0x143).find(|&idx| rom[idx] == 0).unwrap_or(0x144);
        let title = String::from_utf8_lossy(&rom[0x134..title_end])
            .trim()
            .to_string();
        let cgb_flag = rom[0x143];
        let header = CartridgeHeader {
            title,
            cgb_capable: cgb_flag == 0x80 || cgb_flag == 0xc0,
            cgb_only: cgb_flag == 0xc0,
            cartridge_type,
            rom_banks,
            ram_banks,
        };
        let kind = match cartridge_type {
            0x00 => CartridgeKind::RomOnly,
            0x01..=0x03 => CartridgeKind::Mbc1,
            0x0f..=0x13 => CartridgeKind::Mbc3,
            0x19..=0x1e => CartridgeKind::Mbc5,
            other => CartridgeKind::Unknown(other),
        };

        Ok(Self {
            rom,
            ram: vec![0; ram_banks * 0x2000],
            header,
            kind,
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            banking_mode: 0,
        })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    pub fn kind(&self) -> CartridgeKind {
        self.kind
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            0x0000..=0x3fff => {
                let bank = if self.kind == CartridgeKind::Mbc1 && self.banking_mode == 1 {
                    (self.ram_bank << 5) % self.header.rom_banks
                } else {
                    0
                };
                self.rom[(bank * 0x4000 + addr) % self.rom.len()]
            }
            0x4000..=0x7fff => {
                let bank = self.rom_bank % self.header.rom_banks.max(1);
                self.rom[(bank * 0x4000 + (addr - 0x4000)) % self.rom.len()]
            }
            _ => 0xff,
        }
    }

    pub fn write_rom(&mut self, addr: u16, value: u8) {
        match self.kind {
            CartridgeKind::RomOnly | CartridgeKind::Unknown(_) => {}
            CartridgeKind::Mbc1 => self.write_mbc1(addr, value),
            CartridgeKind::Mbc3 => self.write_mbc3(addr, value),
            CartridgeKind::Mbc5 => self.write_mbc5(addr, value),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        if !self.ram_enabled || self.ram.is_empty() {
            return 0xff;
        }
        let offset = self.selected_ram_offset(addr);
        self.ram[offset % self.ram.len()]
    }

    pub fn write_ram(&mut self, addr: u16, value: u8) {
        if !self.ram_enabled || self.ram.is_empty() {
            return;
        }
        let offset = self.selected_ram_offset(addr);
        let len = self.ram.len();
        self.ram[offset % len] = value;
    }

    fn selected_ram_offset(&self, addr: u16) -> usize {
        let bank = match self.kind {
            CartridgeKind::Mbc1 if self.banking_mode == 0 => 0,
            CartridgeKind::Mbc1 | CartridgeKind::Mbc3 | CartridgeKind::Mbc5 => self.ram_bank,
            _ => 0,
        };
        bank * 0x2000 + (addr as usize - 0xa000)
    }

    fn write_mbc1(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.ram_enabled = value & 0x0f == 0x0a,
            0x2000..=0x3fff => {
                let low = (value as usize & 0x1f).max(1);
                self.rom_bank = (self.rom_bank & 0x60) | low;
            }
            0x4000..=0x5fff => {
                let upper = value as usize & 0x03;
                self.ram_bank = upper;
                self.rom_bank = (self.rom_bank & 0x1f) | (upper << 5);
                if self.rom_bank & 0x1f == 0 {
                    self.rom_bank += 1;
                }
            }
            0x6000..=0x7fff => self.banking_mode = value & 1,
            _ => {}
        }
    }

    fn write_mbc3(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.ram_enabled = value & 0x0f == 0x0a,
            0x2000..=0x3fff => self.rom_bank = (value as usize & 0x7f).max(1),
            0x4000..=0x5fff => self.ram_bank = value as usize & 0x03,
            _ => {}
        }
    }

    fn write_mbc5(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.ram_enabled = value & 0x0f == 0x0a,
            0x2000..=0x2fff => self.rom_bank = (self.rom_bank & 0x100) | value as usize,
            0x3000..=0x3fff => {
                self.rom_bank = (self.rom_bank & 0xff) | (((value & 1) as usize) << 8)
            }
            0x4000..=0x5fff => self.ram_bank = value as usize & 0x0f,
            _ => {}
        }
    }
}

fn rom_banks(code: u8) -> Result<usize, CartridgeError> {
    match code {
        0x00..=0x08 => Ok(2usize << code),
        0x52 => Ok(72),
        0x53 => Ok(80),
        0x54 => Ok(96),
        other => Err(CartridgeError::UnsupportedRomSize(other)),
    }
}

fn ram_banks(code: u8) -> Result<usize, CartridgeError> {
    match code {
        0x00 => Ok(0),
        0x01 | 0x02 => Ok(1),
        0x03 => Ok(4),
        0x04 => Ok(16),
        0x05 => Ok(8),
        other => Err(CartridgeError::UnsupportedRamSize(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rom(kind: u8, rom_size: u8, ram_size: u8) -> Vec<u8> {
        let mut rom = vec![0; 0x8000];
        rom[0x134..0x138].copy_from_slice(b"TEST");
        rom[0x147] = kind;
        rom[0x148] = rom_size;
        rom[0x149] = ram_size;
        rom
    }

    #[test]
    fn parses_header() {
        let cart = Cartridge::from_bytes(test_rom(0, 0, 0)).unwrap();
        assert_eq!(cart.header().title, "TEST");
        assert_eq!(cart.kind(), CartridgeKind::RomOnly);
        assert_eq!(cart.header().rom_banks, 2);
    }
}
