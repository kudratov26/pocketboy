//! Serial port (0xFF01, 0xFF02).
//!
//! Headless model: a write to 0xFF02 starts a transfer which completes
//! immediately; the byte currently in 0xFF01 is captured to `output`.

#[derive(Debug, Default, Clone)]
pub struct Serial {
    pub data: u8,    // 0xFF01
    pub control: u8, // 0xFF02
    pub output: Vec<u8>,
}

impl Serial {
    pub fn new() -> Serial {
        Serial {
            data: 0,
            control: 0,
            output: Vec::new(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF01 => self.data,
            0xFF02 => self.control,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF01 => self.data = val,
            0xFF02 => {
                // Transfer start: capture the byte in 0xFF01.
                self.output.push(self.data);
                self.control = val;
            }
            _ => {}
        }
    }
}
