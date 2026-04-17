#[derive(Debug, Clone, Copy)]
pub enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

#[derive(Debug, Clone)]
pub struct Joypad {
    select: u8,
    buttons: u8,
    dpad: u8,
    interrupt: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            select: 0x30,
            buttons: 0x0f,
            dpad: 0x0f,
            interrupt: false,
        }
    }

    pub fn set_button(&mut self, button: Button, pressed: bool) {
        let (group, bit) = match button {
            Button::Right => (&mut self.dpad, 0),
            Button::Left => (&mut self.dpad, 1),
            Button::Up => (&mut self.dpad, 2),
            Button::Down => (&mut self.dpad, 3),
            Button::A => (&mut self.buttons, 0),
            Button::B => (&mut self.buttons, 1),
            Button::Select => (&mut self.buttons, 2),
            Button::Start => (&mut self.buttons, 3),
        };
        let before = *group;
        if pressed {
            *group &= !(1 << bit);
        } else {
            *group |= 1 << bit;
        }
        if pressed && before != *group {
            self.interrupt = true;
        }
    }

    pub fn take_interrupt(&mut self) -> bool {
        let value = self.interrupt;
        self.interrupt = false;
        value
    }

    pub fn read(&self) -> u8 {
        let mut low = 0x0f;
        if self.select & 0x10 == 0 {
            low &= self.buttons;
        }
        if self.select & 0x20 == 0 {
            low &= self.dpad;
        }
        0xc0 | self.select | low
    }

    pub fn write(&mut self, value: u8) {
        self.select = value & 0x30;
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Self::new()
    }
}
