//! MBC5: up to 512 ROM banks (9-bit), 16 cart RAM banks (4-bit).
//!
//! - 0x0000-0x1FFF: RAM enable (write 0x0A)
//! - 0x2000-0x2FFF: ROM bank low 8 bits
//! - 0x3000-0x3FFF: ROM bank bit 8
//! - 0x4000-0x5FFF: RAM bank (0-15)
//! - 0x6000-0x7FFF: unused

use super::{ram_enabled, Cartridge};

#[derive(Debug)]
pub struct Mbc5 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub ram_enabled: bool,
    pub rom_bank: u16, // 9 bits
    pub ram_bank: u8,  // 4 bits
}

impl Mbc5 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Mbc5 {
        Mbc5 {
            rom,
            ram: vec![0; ram_size],
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
        }
    }
}

impl Cartridge for Mbc5 {
    fn read(&self, addr: u16) -> u8 {
        if (0xA000..=0xBFFF).contains(&addr) {
            if !self.ram_enabled || self.ram.is_empty() {
                return 0xFF;
            }
            let off = (self.ram_bank as usize) * 0x2000 + (addr - 0xA000) as usize;
            if off < self.ram.len() {
                self.ram[off]
            } else {
                0x00
            }
        } else if addr >= 0x8000 {
            0xFF
        } else if addr < 0x4000 {
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
            0x2000..=0x2FFF => self.rom_bank = (self.rom_bank & 0x100) | val as u16,
            0x3000..=0x3FFF => self.rom_bank = (self.rom_bank & 0x0FF) | ((val as u16 & 1) << 8),
            0x4000..=0x5FFF => self.ram_bank = val & 0x0F,
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
