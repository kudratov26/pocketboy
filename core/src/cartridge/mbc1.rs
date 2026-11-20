//! MBC1: 16 ROM banks (up to 8Mbit), 2 cart RAM banks (up to 8KB).
//!
//! - 0x0000-0x1FFF: RAM enable (write 0x0A to enable, 0x00 to disable)
//! - 0x2000-0x3FFF: ROM bank low 5 bits
//! - 0x4000-0x5FFF: ROM bank high 2 bits
//! - 0x6000-0x7FFF: cart mode (8KiB ROM / 32KiB RAM)

use super::{ram_enabled, Cartridge};

#[derive(Debug)]
pub struct Mbc1 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub ram_enabled: bool,
    pub rom_bank: u8,   // 5 bits
    pub high2: u8,      // 2 bits
    pub mode: u8,       // 0 or 1 (from 0x6000-0x7FFF)
}

impl Mbc1 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Mbc1 {
        Mbc1 {
            rom,
            ram: vec![0; ram_size],
            ram_enabled: false,
            rom_bank: 1,
            high2: 0,
            mode: 0,
        }
    }

    fn bank(&self) -> usize {
        let mut b = self.rom_bank as usize & 0x1F;
        if b == 0 {
            b = 1; // MBC1 never selects bank 0 in the upper half
        }
        if self.mode == 1 {
            b |= (self.high2 as usize) << 5;
        }
        b
    }
}

impl Cartridge for Mbc1 {
    fn read(&self, addr: u16) -> u8 {
        if (0xA000..=0xBFFF).contains(&addr) {
            if !self.ram_enabled || self.ram.is_empty() {
                return 0xFF;
            }
            return self.ram[(addr - 0xA000) as usize & (self.ram.len() - 1)];
        }
        if addr >= 0x8000 {
            return 0xFF;
        }
        if addr < 0x4000 {
            // Always the first 16KiB of the ROM.
            if (addr as usize) < self.rom.len() {
                self.rom[addr as usize]
            } else {
                0x00
            }
        } else {
            let bank = self.bank() as usize * 0x4000;
            let off = bank + (addr as usize - 0x4000);
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
            0x2000..=0x3FFF => self.rom_bank = (self.rom_bank & 0xE0) | (val & 0x1F),
            0x4000..=0x5FFF => self.high2 = val & 0x03,
            0x6000..=0x7FFF => self.mode = val & 0x01,
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
