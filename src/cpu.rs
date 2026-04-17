use crate::bus::Bus;
use crate::emulator::EmulatorMode;
use std::fmt;

const Z: u8 = 0x80;
const N: u8 = 0x40;
const H: u8 = 0x20;
const C: u8 = 0x10;

#[derive(Debug, Clone)]
pub struct Cpu {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
    halted: bool,
    ime_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuError {
    UnimplementedOpcode { opcode: u8, pc: u16 },
    UnimplementedCbOpcode { opcode: u8, pc: u16 },
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnimplementedOpcode { opcode, pc } => {
                write!(f, "unimplemented opcode 0x{opcode:02x} at 0x{pc:04x}")
            }
            Self::UnimplementedCbOpcode { opcode, pc } => {
                write!(f, "unimplemented CB opcode 0x{opcode:02x} at 0x{pc:04x}")
            }
        }
    }
}

impl std::error::Error for CpuError {}

impl Cpu {
    pub fn post_boot(mode: EmulatorMode) -> Self {
        match mode {
            EmulatorMode::Dmg => Self {
                a: 0x01,
                f: 0xb0,
                b: 0x00,
                c: 0x13,
                d: 0x00,
                e: 0xd8,
                h: 0x01,
                l: 0x4d,
                sp: 0xfffe,
                pc: 0x0100,
                ime: false,
                halted: false,
                ime_pending: false,
            },
            EmulatorMode::Cgb => Self {
                a: 0x11,
                f: 0x80,
                b: 0x00,
                c: 0x00,
                d: 0xff,
                e: 0x56,
                h: 0x00,
                l: 0x0d,
                sp: 0xfffe,
                pc: 0x0100,
                ime: false,
                halted: false,
                ime_pending: false,
            },
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> Result<u32, CpuError> {
        if let Some(cycles) = self.service_interrupt(bus) {
            return Ok(cycles);
        }
        if self.halted {
            return Ok(4);
        }

        let pc = self.pc;
        let opcode = self.fetch8(bus);
        let cycles = self.execute(opcode, pc, bus)?;
        if self.ime_pending {
            self.ime = true;
            self.ime_pending = false;
        }
        Ok(cycles)
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    fn execute(&mut self, opcode: u8, pc: u16, bus: &mut Bus) -> Result<u32, CpuError> {
        let cycles = match opcode {
            0x00 => 4,
            0x01 => {
                let value = self.fetch16(bus);
                self.set_bc(value);
                12
            }
            0x02 => {
                bus.write8(self.bc(), self.a);
                8
            }
            0x03 => {
                self.set_bc(self.bc().wrapping_add(1));
                8
            }
            0x04 => {
                self.b = self.inc8(self.b);
                4
            }
            0x05 => {
                self.b = self.dec8(self.b);
                4
            }
            0x06 => {
                self.b = self.fetch8(bus);
                8
            }
            0x07 => {
                self.a = self.rlc(self.a);
                self.f &= !Z;
                4
            }
            0x08 => {
                let addr = self.fetch16(bus);
                bus.write16(addr, self.sp);
                20
            }
            0x09 => {
                self.add_hl(self.bc());
                8
            }
            0x0a => {
                self.a = bus.read8(self.bc());
                8
            }
            0x0b => {
                self.set_bc(self.bc().wrapping_sub(1));
                8
            }
            0x0c => {
                self.c = self.inc8(self.c);
                4
            }
            0x0d => {
                self.c = self.dec8(self.c);
                4
            }
            0x0e => {
                self.c = self.fetch8(bus);
                8
            }
            0x0f => {
                self.a = self.rrc(self.a);
                self.f &= !Z;
                4
            }
            0x10 => {
                self.fetch8(bus);
                bus.stop_speed_switch();
                4
            }
            0x11 => {
                let value = self.fetch16(bus);
                self.set_de(value);
                12
            }
            0x12 => {
                bus.write8(self.de(), self.a);
                8
            }
            0x13 => {
                self.set_de(self.de().wrapping_add(1));
                8
            }
            0x14 => {
                self.d = self.inc8(self.d);
                4
            }
            0x15 => {
                self.d = self.dec8(self.d);
                4
            }
            0x16 => {
                self.d = self.fetch8(bus);
                8
            }
            0x17 => {
                self.a = self.rl(self.a);
                self.f &= !Z;
                4
            }
            0x18 => {
                self.jr(bus, true);
                12
            }
            0x19 => {
                self.add_hl(self.de());
                8
            }
            0x1a => {
                self.a = bus.read8(self.de());
                8
            }
            0x1b => {
                self.set_de(self.de().wrapping_sub(1));
                8
            }
            0x1c => {
                self.e = self.inc8(self.e);
                4
            }
            0x1d => {
                self.e = self.dec8(self.e);
                4
            }
            0x1e => {
                self.e = self.fetch8(bus);
                8
            }
            0x1f => {
                self.a = self.rr(self.a);
                self.f &= !Z;
                4
            }
            0x20 => {
                if self.jr(bus, !self.flag(Z)) {
                    12
                } else {
                    8
                }
            }
            0x21 => {
                let value = self.fetch16(bus);
                self.set_hl(value);
                12
            }
            0x22 => {
                let addr = self.hl();
                bus.write8(addr, self.a);
                self.set_hl(addr.wrapping_add(1));
                8
            }
            0x23 => {
                self.set_hl(self.hl().wrapping_add(1));
                8
            }
            0x24 => {
                self.h = self.inc8(self.h);
                4
            }
            0x25 => {
                self.h = self.dec8(self.h);
                4
            }
            0x26 => {
                self.h = self.fetch8(bus);
                8
            }
            0x27 => {
                self.daa();
                4
            }
            0x28 => {
                if self.jr(bus, self.flag(Z)) {
                    12
                } else {
                    8
                }
            }
            0x29 => {
                self.add_hl(self.hl());
                8
            }
            0x2a => {
                let addr = self.hl();
                self.a = bus.read8(addr);
                self.set_hl(addr.wrapping_add(1));
                8
            }
            0x2b => {
                self.set_hl(self.hl().wrapping_sub(1));
                8
            }
            0x2c => {
                self.l = self.inc8(self.l);
                4
            }
            0x2d => {
                self.l = self.dec8(self.l);
                4
            }
            0x2e => {
                self.l = self.fetch8(bus);
                8
            }
            0x2f => {
                self.a ^= 0xff;
                self.f = (self.f & (Z | C)) | N | H;
                4
            }
            0x30 => {
                if self.jr(bus, !self.flag(C)) {
                    12
                } else {
                    8
                }
            }
            0x31 => {
                self.sp = self.fetch16(bus);
                12
            }
            0x32 => {
                let addr = self.hl();
                bus.write8(addr, self.a);
                self.set_hl(addr.wrapping_sub(1));
                8
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                8
            }
            0x34 => {
                let addr = self.hl();
                let value = self.inc8(bus.read8(addr));
                bus.write8(addr, value);
                12
            }
            0x35 => {
                let addr = self.hl();
                let value = self.dec8(bus.read8(addr));
                bus.write8(addr, value);
                12
            }
            0x36 => {
                let value = self.fetch8(bus);
                bus.write8(self.hl(), value);
                12
            }
            0x37 => {
                self.f = (self.f & Z) | C;
                4
            }
            0x38 => {
                if self.jr(bus, self.flag(C)) {
                    12
                } else {
                    8
                }
            }
            0x39 => {
                self.add_hl(self.sp);
                8
            }
            0x3a => {
                let addr = self.hl();
                self.a = bus.read8(addr);
                self.set_hl(addr.wrapping_sub(1));
                8
            }
            0x3b => {
                self.sp = self.sp.wrapping_sub(1);
                8
            }
            0x3c => {
                self.a = self.inc8(self.a);
                4
            }
            0x3d => {
                self.a = self.dec8(self.a);
                4
            }
            0x3e => {
                self.a = self.fetch8(bus);
                8
            }
            0x3f => {
                self.f = (self.f & Z) | if self.flag(C) { 0 } else { C };
                4
            }
            0x40..=0x7f => {
                if opcode == 0x76 {
                    self.halted = true;
                    4
                } else {
                    let value = self.read_reg((opcode & 0x07) as usize, bus);
                    self.write_reg(((opcode >> 3) & 0x07) as usize, value, bus);
                    if opcode & 0x07 == 6 || opcode & 0x38 == 0x30 {
                        8
                    } else {
                        4
                    }
                }
            }
            0x80..=0x87 => {
                self.add_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0x88..=0x8f => {
                self.adc_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0x90..=0x97 => {
                self.sub_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0x98..=0x9f => {
                self.sbc_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0xa0..=0xa7 => {
                self.and_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0xa8..=0xaf => {
                self.xor_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0xb0..=0xb7 => {
                self.or_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0xb8..=0xbf => {
                self.cp_a(self.read_reg((opcode & 7) as usize, bus));
                if opcode & 7 == 6 { 8 } else { 4 }
            }
            0xc0 => {
                if self.ret(!self.flag(Z), bus) {
                    20
                } else {
                    8
                }
            }
            0xc1 => {
                let value = self.pop(bus);
                self.set_bc(value);
                12
            }
            0xc2 => {
                if self.jp(!self.flag(Z), bus) {
                    16
                } else {
                    12
                }
            }
            0xc3 => {
                self.pc = self.fetch16(bus);
                16
            }
            0xc4 => {
                if self.call(!self.flag(Z), bus) {
                    24
                } else {
                    12
                }
            }
            0xc5 => {
                self.push(self.bc(), bus);
                16
            }
            0xc6 => {
                let value = self.fetch8(bus);
                self.add_a(value);
                8
            }
            0xc7 => {
                self.rst(0x00, bus);
                16
            }
            0xc8 => {
                if self.ret(self.flag(Z), bus) {
                    20
                } else {
                    8
                }
            }
            0xc9 => {
                self.pc = self.pop(bus);
                16
            }
            0xca => {
                if self.jp(self.flag(Z), bus) {
                    16
                } else {
                    12
                }
            }
            0xcb => {
                let cb_opcode = self.fetch8(bus);
                return self.execute_cb(cb_opcode, pc, bus);
            }
            0xcc => {
                if self.call(self.flag(Z), bus) {
                    24
                } else {
                    12
                }
            }
            0xcd => {
                self.call(true, bus);
                24
            }
            0xce => {
                let value = self.fetch8(bus);
                self.adc_a(value);
                8
            }
            0xcf => {
                self.rst(0x08, bus);
                16
            }
            0xd0 => {
                if self.ret(!self.flag(C), bus) {
                    20
                } else {
                    8
                }
            }
            0xd1 => {
                let value = self.pop(bus);
                self.set_de(value);
                12
            }
            0xd2 => {
                if self.jp(!self.flag(C), bus) {
                    16
                } else {
                    12
                }
            }
            0xd4 => {
                if self.call(!self.flag(C), bus) {
                    24
                } else {
                    12
                }
            }
            0xd5 => {
                self.push(self.de(), bus);
                16
            }
            0xd6 => {
                let value = self.fetch8(bus);
                self.sub_a(value);
                8
            }
            0xd7 => {
                self.rst(0x10, bus);
                16
            }
            0xd8 => {
                if self.ret(self.flag(C), bus) {
                    20
                } else {
                    8
                }
            }
            0xd9 => {
                self.pc = self.pop(bus);
                self.ime = true;
                16
            }
            0xda => {
                if self.jp(self.flag(C), bus) {
                    16
                } else {
                    12
                }
            }
            0xdc => {
                if self.call(self.flag(C), bus) {
                    24
                } else {
                    12
                }
            }
            0xde => {
                let value = self.fetch8(bus);
                self.sbc_a(value);
                8
            }
            0xdf => {
                self.rst(0x18, bus);
                16
            }
            0xe0 => {
                let offset = self.fetch8(bus);
                bus.write8(0xff00 | offset as u16, self.a);
                12
            }
            0xe1 => {
                let value = self.pop(bus);
                self.set_hl(value);
                12
            }
            0xe2 => {
                bus.write8(0xff00 | self.c as u16, self.a);
                8
            }
            0xe5 => {
                self.push(self.hl(), bus);
                16
            }
            0xe6 => {
                let value = self.fetch8(bus);
                self.and_a(value);
                8
            }
            0xe7 => {
                self.rst(0x20, bus);
                16
            }
            0xe8 => {
                let value = self.fetch8(bus) as i8 as i16 as u16;
                let old = self.sp;
                self.sp = self.sp.wrapping_add(value);
                self.f = flags(
                    false,
                    false,
                    (old & 0x0f) + (value & 0x0f) > 0x0f,
                    (old & 0xff) + (value & 0xff) > 0xff,
                );
                16
            }
            0xe9 => {
                self.pc = self.hl();
                4
            }
            0xea => {
                let addr = self.fetch16(bus);
                bus.write8(addr, self.a);
                16
            }
            0xee => {
                let value = self.fetch8(bus);
                self.xor_a(value);
                8
            }
            0xef => {
                self.rst(0x28, bus);
                16
            }
            0xf0 => {
                let offset = self.fetch8(bus);
                self.a = bus.read8(0xff00 | offset as u16);
                12
            }
            0xf1 => {
                let value = self.pop(bus);
                self.set_af(value);
                12
            }
            0xf2 => {
                self.a = bus.read8(0xff00 | self.c as u16);
                8
            }
            0xf3 => {
                self.ime = false;
                self.ime_pending = false;
                4
            }
            0xf5 => {
                self.push(self.af(), bus);
                16
            }
            0xf6 => {
                let value = self.fetch8(bus);
                self.or_a(value);
                8
            }
            0xf7 => {
                self.rst(0x30, bus);
                16
            }
            0xf8 => {
                let value = self.fetch8(bus) as i8 as i16 as u16;
                let old = self.sp;
                self.set_hl(self.sp.wrapping_add(value));
                self.f = flags(
                    false,
                    false,
                    (old & 0x0f) + (value & 0x0f) > 0x0f,
                    (old & 0xff) + (value & 0xff) > 0xff,
                );
                12
            }
            0xf9 => {
                self.sp = self.hl();
                8
            }
            0xfa => {
                let addr = self.fetch16(bus);
                self.a = bus.read8(addr);
                16
            }
            0xfb => {
                self.ime_pending = true;
                4
            }
            0xfe => {
                let value = self.fetch8(bus);
                self.cp_a(value);
                8
            }
            0xff => {
                self.rst(0x38, bus);
                16
            }
            _ => return Err(CpuError::UnimplementedOpcode { opcode, pc }),
        };
        Ok(cycles)
    }

    fn execute_cb(&mut self, opcode: u8, _pc: u16, bus: &mut Bus) -> Result<u32, CpuError> {
        let reg = (opcode & 0x07) as usize;
        let cycles = if reg == 6 { 16 } else { 8 };
        match opcode {
            0x00..=0x07 => {
                let value = self.rlc(self.read_reg(reg, bus));
                self.write_reg(reg, value, bus);
            }
            0x08..=0x0f => {
                let value = self.rrc(self.read_reg(reg, bus));
                self.write_reg(reg, value, bus);
            }
            0x10..=0x17 => {
                let value = self.rl(self.read_reg(reg, bus));
                self.write_reg(reg, value, bus);
            }
            0x18..=0x1f => {
                let value = self.rr(self.read_reg(reg, bus));
                self.write_reg(reg, value, bus);
            }
            0x20..=0x27 => {
                let old = self.read_reg(reg, bus);
                let value = old << 1;
                self.f = flags(value == 0, false, false, old & 0x80 != 0);
                self.write_reg(reg, value, bus);
            }
            0x28..=0x2f => {
                let old = self.read_reg(reg, bus);
                let value = (old >> 1) | (old & 0x80);
                self.f = flags(value == 0, false, false, old & 1 != 0);
                self.write_reg(reg, value, bus);
            }
            0x30..=0x37 => {
                let old = self.read_reg(reg, bus);
                let value = old.rotate_left(4);
                self.f = flags(value == 0, false, false, false);
                self.write_reg(reg, value, bus);
            }
            0x38..=0x3f => {
                let old = self.read_reg(reg, bus);
                let value = old >> 1;
                self.f = flags(value == 0, false, false, old & 1 != 0);
                self.write_reg(reg, value, bus);
            }
            0x40..=0x7f => {
                let bit = (opcode - 0x40) / 8;
                let value = self.read_reg(reg, bus);
                self.f = (self.f & C) | flags(value & (1 << bit) == 0, false, true, self.flag(C));
            }
            0x80..=0xbf => {
                let bit = (opcode - 0x80) / 8;
                let value = self.read_reg(reg, bus) & !(1 << bit);
                self.write_reg(reg, value, bus);
            }
            0xc0..=0xff => {
                let bit = (opcode - 0xc0) / 8;
                let value = self.read_reg(reg, bus) | (1 << bit);
                self.write_reg(reg, value, bus);
            }
        }
        if matches!(opcode, 0x40..=0x7f) && reg == 6 {
            Ok(12)
        } else {
            Ok(cycles)
        }
    }

    fn service_interrupt(&mut self, bus: &mut Bus) -> Option<u32> {
        let pending = bus.interrupt_enable & bus.interrupt_flag & 0x1f;
        if pending == 0 {
            return None;
        }
        self.halted = false;
        if !self.ime {
            return None;
        }
        self.ime = false;
        let (mask, vector) = if pending & 0x01 != 0 {
            (0x01, 0x40)
        } else if pending & 0x02 != 0 {
            (0x02, 0x48)
        } else if pending & 0x04 != 0 {
            (0x04, 0x50)
        } else if pending & 0x08 != 0 {
            (0x08, 0x58)
        } else {
            (0x10, 0x60)
        };
        bus.interrupt_flag &= !mask;
        self.push(self.pc, bus);
        self.pc = vector;
        Some(20)
    }

    fn fetch8(&mut self, bus: &Bus) -> u8 {
        let value = bus.read8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn fetch16(&mut self, bus: &Bus) -> u16 {
        u16::from_le_bytes([self.fetch8(bus), self.fetch8(bus)])
    }

    fn read_reg(&self, index: usize, bus: &Bus) -> u8 {
        match index {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => bus.read8(self.hl()),
            7 => self.a,
            _ => unreachable!(),
        }
    }

    fn write_reg(&mut self, index: usize, value: u8, bus: &mut Bus) {
        match index {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            6 => bus.write8(self.hl(), value),
            7 => self.a = value,
            _ => unreachable!(),
        }
    }

    fn inc8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.f = (self.f & C) | flags(result == 0, false, (value & 0x0f) == 0x0f, self.flag(C));
        result
    }

    fn dec8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.f = (self.f & C) | flags(result == 0, true, (value & 0x0f) == 0, self.flag(C));
        result
    }

    fn add_a(&mut self, value: u8) {
        let old = self.a;
        let (result, carry) = old.overflowing_add(value);
        self.a = result;
        self.f = flags(
            result == 0,
            false,
            (old & 0x0f) + (value & 0x0f) > 0x0f,
            carry,
        );
    }

    fn adc_a(&mut self, value: u8) {
        let carry_in = u8::from(self.flag(C));
        let old = self.a;
        let (partial, carry1) = old.overflowing_add(value);
        let (result, carry2) = partial.overflowing_add(carry_in);
        self.a = result;
        self.f = flags(
            result == 0,
            false,
            (old & 0x0f) + (value & 0x0f) + carry_in > 0x0f,
            carry1 || carry2,
        );
    }

    fn sub_a(&mut self, value: u8) {
        let old = self.a;
        let (result, carry) = old.overflowing_sub(value);
        self.a = result;
        self.f = flags(result == 0, true, (old & 0x0f) < (value & 0x0f), carry);
    }

    fn sbc_a(&mut self, value: u8) {
        let carry_in = u8::from(self.flag(C));
        let old = self.a;
        let (partial, carry1) = old.overflowing_sub(value);
        let (result, carry2) = partial.overflowing_sub(carry_in);
        self.a = result;
        self.f = flags(
            result == 0,
            true,
            (old & 0x0f) < ((value & 0x0f) + carry_in),
            carry1 || carry2,
        );
    }

    fn and_a(&mut self, value: u8) {
        self.a &= value;
        self.f = flags(self.a == 0, false, true, false);
    }

    fn xor_a(&mut self, value: u8) {
        self.a ^= value;
        self.f = flags(self.a == 0, false, false, false);
    }

    fn or_a(&mut self, value: u8) {
        self.a |= value;
        self.f = flags(self.a == 0, false, false, false);
    }

    fn cp_a(&mut self, value: u8) {
        let result = self.a.wrapping_sub(value);
        self.f = flags(
            result == 0,
            true,
            (self.a & 0x0f) < (value & 0x0f),
            self.a < value,
        );
    }

    fn add_hl(&mut self, value: u16) {
        let old = self.hl();
        let result = old.wrapping_add(value);
        self.set_hl(result);
        self.f = (self.f & Z)
            | flags(
                self.flag(Z),
                false,
                (old & 0x0fff) + (value & 0x0fff) > 0x0fff,
                old > 0xffff - value,
            );
    }

    fn daa(&mut self) {
        let mut adjust = 0;
        let mut carry = self.flag(C);
        if !self.flag(N) {
            if self.flag(H) || (self.a & 0x0f) > 9 {
                adjust |= 0x06;
            }
            if carry || self.a > 0x99 {
                adjust |= 0x60;
                carry = true;
            }
            self.a = self.a.wrapping_add(adjust);
        } else {
            if self.flag(H) {
                adjust |= 0x06;
            }
            if carry {
                adjust |= 0x60;
            }
            self.a = self.a.wrapping_sub(adjust);
        }
        self.f = flags(self.a == 0, self.flag(N), false, carry);
    }

    fn rlc(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(1);
        self.f = flags(result == 0, false, false, value & 0x80 != 0);
        result
    }

    fn rrc(&mut self, value: u8) -> u8 {
        let result = value.rotate_right(1);
        self.f = flags(result == 0, false, false, value & 1 != 0);
        result
    }

    fn rl(&mut self, value: u8) -> u8 {
        let result = (value << 1) | u8::from(self.flag(C));
        self.f = flags(result == 0, false, false, value & 0x80 != 0);
        result
    }

    fn rr(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | if self.flag(C) { 0x80 } else { 0 };
        self.f = flags(result == 0, false, false, value & 1 != 0);
        result
    }

    fn jr(&mut self, bus: &Bus, condition: bool) -> bool {
        let offset = self.fetch8(bus) as i8;
        if condition {
            self.pc = self.pc.wrapping_add(offset as i16 as u16);
        }
        condition
    }

    fn jp(&mut self, condition: bool, bus: &Bus) -> bool {
        let addr = self.fetch16(bus);
        if condition {
            self.pc = addr;
        }
        condition
    }

    fn call(&mut self, condition: bool, bus: &mut Bus) -> bool {
        let addr = self.fetch16(bus);
        if condition {
            self.push(self.pc, bus);
            self.pc = addr;
        }
        condition
    }

    fn ret(&mut self, condition: bool, bus: &Bus) -> bool {
        if condition {
            self.pc = self.pop(bus);
        }
        condition
    }

    fn rst(&mut self, vector: u16, bus: &mut Bus) {
        self.push(self.pc, bus);
        self.pc = vector;
    }

    fn push(&mut self, value: u16, bus: &mut Bus) {
        let [lo, hi] = value.to_le_bytes();
        self.sp = self.sp.wrapping_sub(1);
        bus.write8(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        bus.write8(self.sp, lo);
    }

    fn pop(&mut self, bus: &Bus) -> u16 {
        let lo = bus.read8(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read8(self.sp);
        self.sp = self.sp.wrapping_add(1);
        u16::from_le_bytes([lo, hi])
    }

    fn flag(&self, flag: u8) -> bool {
        self.f & flag != 0
    }

    fn af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f & 0xf0])
    }

    fn set_af(&mut self, value: u16) {
        let [a, f] = value.to_be_bytes();
        self.a = a;
        self.f = f & 0xf0;
    }

    fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    fn set_bc(&mut self, value: u16) {
        let [b, c] = value.to_be_bytes();
        self.b = b;
        self.c = c;
    }

    fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    fn set_de(&mut self, value: u16) {
        let [d, e] = value.to_be_bytes();
        self.d = d;
        self.e = e;
    }

    fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    fn set_hl(&mut self, value: u16) {
        let [h, l] = value.to_be_bytes();
        self.h = h;
        self.l = l;
    }
}

fn flags(z: bool, n: bool, h: bool, c: bool) -> u8 {
    (if z { Z } else { 0 })
        | (if n { N } else { 0 })
        | (if h { H } else { 0 })
        | (if c { C } else { 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    fn bus_with_program(program: &[u8]) -> Bus {
        let mut rom = vec![0; 0x8000];
        rom[0x147] = 0;
        rom[0x148] = 0;
        rom[0x149] = 0;
        rom[0x100..0x100 + program.len()].copy_from_slice(program);
        Bus::new(Cartridge::from_bytes(rom).unwrap(), EmulatorMode::Cgb)
    }

    #[test]
    fn executes_basic_arithmetic() {
        let mut cpu = Cpu::post_boot(EmulatorMode::Cgb);
        let mut bus = bus_with_program(&[0x3e, 0x02, 0xc6, 0x03, 0x76]);
        assert_eq!(cpu.step(&mut bus).unwrap(), 8);
        assert_eq!(cpu.step(&mut bus).unwrap(), 8);
        assert_eq!(cpu.a, 5);
        assert_eq!(cpu.f & Z, 0);
    }

    #[test]
    fn calls_and_returns() {
        let mut cpu = Cpu::post_boot(EmulatorMode::Cgb);
        let mut bus = bus_with_program(&[0xcd, 0x06, 0x01, 0x3e, 0x09, 0x76, 0x3e, 0x42, 0xc9]);
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.a, 0x42);
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.a, 0x09);
    }
}
