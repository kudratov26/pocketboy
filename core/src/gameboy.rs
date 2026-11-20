//! Top-level GameBoy: owns the CPU and Bus, exposes the architecture API.

use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::joypad::ButtonState;

pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(rom: Vec<u8>) -> GameBoy {
        GameBoy {
            cpu: Cpu::new(),
            bus: Bus::new(rom),
        }
    }

    /// One CPU instruction; returns the number of T-cycles consumed.
    pub fn step(&mut self) -> u32 {
        let cycles = self.cpu.step(&mut self.bus);
        self.bus.tick(cycles);
        cycles as u32
    }

    /// Run until the scanline counter wraps back to 0 (end of a frame).
    pub fn run_frame(&mut self) -> u32 {
        let mut total = 0u32;
        let mut seen = false;
        for _ in 0..5_000_000 {
            if self.bus.ppu.line == 0 && seen {
                break;
            }
            if self.bus.ppu.line != 0 {
                seen = true;
            }
            total += self.step();
        }
        total
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.bus.ppu.framebuffer
    }

    pub fn set_buttons(&mut self, buttons: ButtonState) {
        self.bus.joypad.set(buttons);
    }

    pub fn serial_output(&self) -> &[u8] {
        &self.bus.serial.output
    }

    pub fn save_ram(&self) -> Vec<u8> {
        self.bus.cartridge.save_ram()
    }

    pub fn load_ram(&mut self, ram: Vec<u8>) {
        self.bus.cartridge.load_ram(ram);
    }
}
