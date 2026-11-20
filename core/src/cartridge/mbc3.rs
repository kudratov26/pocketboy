//! MBC3: up to 128 ROM banks (7-bit), 8 cart RAM banks (0-7), optional RTC.
//! RTC registers are simplified out of scope (MISSION allows this); the RAM
//! banking and enable logic is implemented.

use super::{ram_enabled, Cartridge};

#[derive(Debug)]
pub struct Mbc3 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub ram_enabled: bool,
    pub rom_bank: u8, // 1-127
    pub ram_bank: u8, // 0-7
}

impl Mbc3 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Mbc3 {
        Mbc3 {
            rom,
            ram: vec![0; ram_size],
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
        }
    }

    fn ram_off(&self, addr: u16) -> Option<usize> {
        if self.ram.is_empty() {
            return None;
        }
        let mut off = (self.ram_bank as usize) * 0x2000 + (addr - 0xA000) as usize;
        // 8 x 2KiB banks
        if off >= 0x4000 && self.ram.len() >= 0x4000 {
            off -= 0x4000;
        } else if off >= self.ram.len() {
            off -= 0x4000;
        }
        if off < self.ram.len() {
            Some(off)
        } else {
            None
        }
    }
}

impl Cartridge for Mbc3 {
    fn read(&self, addr: u16) -> u8 {
        if (0xA000..=0xBFFF).contains(&addr) {
            if self.ram_enabled {
                if let Some(off) = self.ram_off(addr) {
                    return self.ram[off];
                }
            }
            return 0xFF;
        }
        if addr >= 0x8000 {
            return 0xFF;
        }
        if addr < 0x4000 {
            if (addr as usize) < self.rom.len() {
                self.rom[addr as usize]
            } else {
                0x00
            }
        } else {
            let off = (self.rom_bank as usize) * 0x4000 + (addr as usize - 0x4000);
            if off < self.rom.len() {
                self.rom[off]
            } else {
                0x00
            }
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = ram_enabled(val),
            0x2000..=0x3FFF => {
                let b = val as usize & 0x7F;
                self.rom_bank = if b == 0 { 1 } else { b as u8 };
            }
            0x4000..=0x5FFF => {
                let b = val as usize & 0x07;
                if b < 8 {
                    self.ram_bank = b as u8;
                }
            }
            0x6000..=0x7FFF => {} // RTC latch: simplified
            _ => {}
        }
    }

    fn save_ram(&self) -> Vec<u8> {
        self.ram.clone()
    }

    fn load_ram(&mut self, ram: Vec<u8>) {
        let n = self.ram.len();
        let m = ram.len().min(n);
        self.ram[..m].copy_from_slice(&ram[..m]);
    }

    fn ram_size(&self) -> usize {
        self.ram.len()
    }
}
