//! Joypad (0xFF00).
//!
//! Read layout (GB VI, DMG):
//!   bits 0-3: direction keys, bit0=Right, bit1=Left, bit2=Up, bit3=Down
//!   bits 4-7: buttons, bit4=A, bit5=B, bit6=Select, bit7=Start
//!   unpressed keys read as 1.
//! Write: bit 4 selects the direction column, bit 5 the button column.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Joypad {
    pub write: u8,
    pub buttons: ButtonState,
    interrupt_pending: bool,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            write: 0x30,
            buttons: ButtonState::default(),
            interrupt_pending: false,
        }
    }

    pub fn set(&mut self, buttons: ButtonState) {
        let old = self.buttons;
        self.buttons = buttons;
        // Joypad interrupt fires when a button goes from unpressed to pressed.
        let newly_pressed =
            (!old.a && buttons.a) || (!old.b && buttons.b) || (!old.select && buttons.select)
                || (!old.start && buttons.start) || (!old.up && buttons.up)
                || (!old.down && buttons.down) || (!old.left && buttons.left)
                || (!old.right && buttons.right);
        if newly_pressed {
            self.interrupt_pending = true;
        }
    }

    pub fn take_interrupt(&mut self) -> bool {
        if self.interrupt_pending {
            self.interrupt_pending = false;
            true
        } else {
            false
        }
    }

    pub fn read(&self) -> u8 {
        let mut v: u8 = 0x00;
        if self.write & 0x10 != 0 {
            if self.buttons.right {
                v |= 0x01;
            }
            if self.buttons.left {
                v |= 0x02;
            }
            if self.buttons.up {
                v |= 0x04;
            }
            if self.buttons.down {
                v |= 0x08;
            }
        } else {
            v |= 0x0F;
        }
        if self.write & 0x20 != 0 {
            if self.buttons.a {
                v |= 0x10;
            }
            if self.buttons.b {
                v |= 0x20;
            }
            if self.buttons.select {
                v |= 0x40;
            }
            if self.buttons.start {
                v |= 0x80;
            }
        } else {
            v |= 0xF0;
        }
        v
    }

    pub fn write(&mut self, v: u8) {
        self.write = v;
    }
}
