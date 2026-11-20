//! ROM-only cartridge (no memory controller) with optional fixed RAM area.

use super::{Cartridge, Header};

/// Simple cartridge: fixed ROM 0x0000-0x7FFF (0x0000-0x3FFF bank 0, 0x4000-0x7FFF
/// mirrors the last 16KiB), RAM area 0xA000-0xBFFF.
#[derive(Debug)]
pub struct RomOnly {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    ram_base: u32, // base offset of the mapped RAM inside the ROM image, or -1
}

impl RomOnly {
    pub fn new(rom: Vec<u8>, header: &Header) -> RomOnly {
        let ram_size = header.ram_size();
        // ROM-only with RAM (0x07-0x09): some carts map RAM at 0xA000 of a 32KiB ROM.
        // We keep it simple: RAM is separate storage; if the ROM image has RAM bytes,
        // they are accessible through `ram_base`.
        let ram_base = if header.cart_type >= 0x07 && header.cart_type <= 0x09 && rom.len() >= 0xC000 + 0x2000 {
            (0xC000 - 0x8000) as u32
        } else {
            u32::MAX
        };
        RomOnly {
            rom,
            ram: vec![0; ram_size],
            ram_base,
        }
    }
}

impl Cartridge for RomOnly {
    fn read(&self, addr: u16) -> u8 {
        if (0x8000..=0x9FFF).contains(&addr) {
            return 0xFF;
        }
        if (0xA000..=0xBFFF).contains(&addr) {
            if self.ram_base != u32::MAX {
                let off = self.ram_base + (addr - 0xA000) as u32;
                if (off as usize) < self.rom.len() {
                    return self.rom[off as usize];
                }
            }
            return 0xFF;
        }
        let off = addr as usize;
        if off < self.rom.len() {
            self.rom[off]
        } else {
            0x00
        }
    }

    fn write(&mut self, addr: u16, _val: u8) {
        let _ = addr; // ROM-only: writes are ignored
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
