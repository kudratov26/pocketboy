//! Timer: 0xFF04 DIV, 0xFF05 TIMA, 0xFF06 TMA, 0xFF07 TAC.
//!
//! Assumptions (record in DECISIONS.md):
//! - DIV increments every 2^14 cycles.
//! - TAC divisors: 00: 2^14, 01: 2^12, 10: 2^10, 11: 2^8.
//! - Writing DIV sets it to 0.

#[derive(Debug, Clone)]
pub struct Timer {
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    div_count: u16,
    tim_count: u16,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_count: 1 << 14,
            tim_count: 1 << 14,
        }
    }

    fn divisor(&self) -> u16 {
        match self.tac & 0x03 {
            0 => 1 << 14,
            1 => 1 << 12,
            2 => 1 << 10,
            _ => 1 << 8,
        }
    }

    pub fn set_tac(&mut self, v: u8) {
        self.tac = v & 0x07;
        self.tim_count = self.divisor();
    }
    /// Advance by `cycles`; on TIMA overflow set the timer interrupt bit in
    /// the IF register (bit 2).
    pub fn tick(&mut self, cycles: u32, io_if: &mut u8) {
        let divisor = self.divisor();
        let enabled = self.tac & 0x04 != 0;
        let mut c = cycles;
        while c > 0 {
            // TIMA countdown
            let step = c.min(self.tim_count as u32);
            c -= step;
            self.tim_count -= step as u16;
            if self.tim_count == 0 {
                if enabled {
                    self.tima = self.tima.wrapping_add(1);
                    if self.tima == 0 {
                        self.tima = self.tma;
                        *io_if |= 0x04;
                    }
                }
                self.tim_count = divisor;
            }
            // DIV countdown
            let step = c.min(self.div_count as u32);
            c -= step;
            self.div_count -= step as u16;
            if self.div_count == 0 {
                self.div = self.div.wrapping_add(1);
                self.div_count = 1 << 14;
            }
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer::new()
    }
}
