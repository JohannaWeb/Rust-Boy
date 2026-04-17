use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::cpu::CpuError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulatorMode {
    Dmg,
    Cgb,
}

pub struct Emulator {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl Emulator {
    pub fn new(cartridge: Cartridge, mode: EmulatorMode) -> Self {
        Self {
            cpu: Cpu::post_boot(mode),
            bus: Bus::new(cartridge, mode),
        }
    }

    pub fn step(&mut self) -> Result<u32, CpuError> {
        let cycles = self.cpu.step(&mut self.bus)?;
        self.bus.tick(cycles);
        Ok(cycles)
    }

    pub fn run_cycles(&mut self, target_cycles: u64) -> Result<u64, CpuError> {
        let mut elapsed = 0;
        while elapsed < target_cycles {
            elapsed += u64::from(self.step()?);
        }
        Ok(elapsed)
    }

    pub fn frame_buffer(&self) -> &[u32] {
        self.bus.ppu.frame_buffer()
    }
}
