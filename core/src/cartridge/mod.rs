//! Cartridge: the memory-mapped area 0x0000-0x7FFF plus cart RAM (0xA000-0xBFFF).
//!
//! Cart types (GB VI header 0x147):
//! - 0x00-0x03: ROM-only / MBC1
//! - 0x04-0x06: MBC2
//! - 0x07-0x09: ROM-only with RAM
//! - 0x0A-0x0F: MBC3 (RTC simplified: RAM bank switching only)
//! - 0x10-0x1F: MBC5

pub mod mbc1;
pub mod mbc3;
pub mod mbc5;
pub mod rom_only;

pub use mbc1::Mbc1;
pub use mbc3::Mbc3;
pub use mbc5::Mbc5;
pub use rom_only::RomOnly;

#[derive(Debug, Clone)]
pub struct Header {
    pub title: String,
    pub manufacturer: [u8; 4],
    pub license: [u8; 2],
    pub sgb_flag: u8,
    pub cart_type: u8,
    pub rom_size_raw: u8,
    pub ram_size_raw: u8,
    pub dest_code: u8,
    pub version: u8,
    pub checksum: u8,
    pub global_checksum: [u8; 2],
}

impl Header {
    pub fn parse(rom: &[u8]) -> Header {
        let title_end = rom[0x134..=0x143]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(rom[0x134..=0x143].len());
        Header {
            title: String::from_utf8_lossy(&rom[0x134..0x134 + title_end]).into_owned(),
            manufacturer: [rom[0x13F], rom[0x13E], rom[0x13D], rom[0x13C]],
            license: [rom[0x144], rom[0x145]],
            sgb_flag: rom[0x146],
            cart_type: rom[0x147],
            rom_size_raw: rom[0x148],
            ram_size_raw: rom[0x149],
            dest_code: rom[0x14A],
            version: rom[0x14B],
            checksum: rom[0x14E],
            global_checksum: [rom[0x14F], rom[0x150]],
        }
    }

    pub fn rom_size(&self) -> usize {
        32768 << self.rom_size_raw.min(8)
    }

    pub fn ram_size(&self) -> usize {
        match self.ram_size_raw {
            0x00 => 0,
            0x01 => 0x0800,
            0x02 => 0x2000,
            0x03 => 0x8000,
            0x04 => 0x8000,
            0x05 => 0x2000,
            _ => 0x2000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeKind {
    RomOnly,
    RomOnlyWithRam,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
}

pub fn cartridge_kind(cart_type: u8) -> CartridgeKind {
    match cart_type {
        0x00 => CartridgeKind::RomOnly,
        0x01 | 0x02 | 0x03 => CartridgeKind::Mbc1,
        0x04 | 0x05 | 0x06 => CartridgeKind::Mbc2,
        0x07 | 0x08 | 0x09 => CartridgeKind::RomOnlyWithRam,
        0x0A..=0x0F => CartridgeKind::Mbc3,
        0x10..=0x1F => CartridgeKind::Mbc5,
        _ => CartridgeKind::RomOnly,
    }
}

pub trait Cartridge: std::fmt::Debug {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn save_ram(&self) -> Vec<u8>;
    fn load_ram(&mut self, ram: Vec<u8>);
    fn ram_size(&self) -> usize;
}

pub fn ram_enabled(v: u8) -> bool {
    v & 0x0F == 0x0A
}
