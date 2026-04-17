#[derive(Debug, Clone)]
pub struct Timer {
    div_counter: u32,
    tima_counter: u32,
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    interrupt: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div_counter: 0,
            tima_counter: 0,
            div: 0xab,
            tima: 0,
            tma: 0,
            tac: 0xf8,
            interrupt: false,
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        self.div_counter += cycles;
        while self.div_counter >= 256 {
            self.div_counter -= 256;
            self.div = self.div.wrapping_add(1);
        }

        if self.tac & 0x04 == 0 {
            return;
        }

        self.tima_counter += cycles;
        let period = match self.tac & 0x03 {
            0 => 1024,
            1 => 16,
            2 => 64,
            _ => 256,
        };
        while self.tima_counter >= period {
            self.tima_counter -= period;
            let (next, overflow) = self.tima.overflowing_add(1);
            self.tima = if overflow {
                self.interrupt = true;
                self.tma
            } else {
                next
            };
        }
    }

    pub fn take_interrupt(&mut self) -> bool {
        let value = self.interrupt;
        self.interrupt = false;
        value
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff04 => self.div,
            0xff05 => self.tima,
            0xff06 => self.tma,
            0xff07 => self.tac | 0xf8,
            _ => 0xff,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xff04 => {
                self.div = 0;
                self.div_counter = 0;
            }
            0xff05 => self.tima = value,
            0xff06 => self.tma = value,
            0xff07 => self.tac = value | 0xf8,
            _ => {}
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
