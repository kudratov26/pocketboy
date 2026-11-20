//! PPU: LCDC, SCY/SCX, WY/WX, palettes, LY/LYC/STAT, scanline modes,
//! background, window and sprites rendering.
//!
//! Timing model (DMG, approximation; see DECISIONS.md):
//! - Line = 160 cycles.
//! - Mode 2 (OAM scan): 80 cycles.
//! - Mode 3 (render): 80 + 2 * min(10, sprites_on_line).
//! - Mode 0 (HBlank): 160 - 80 - mode3.
//! - Mode 1 (VBlank): entered when LY == 144, each VBlank line lasts 160
//!   cycles; when LY wraps to 0 the mode returns to 2.
//! - LY increments at the end of mode 0.

pub const FRAMEBUFFER_W: usize = 160;
pub const FRAMEBUFFER_H: usize = 144;
pub const LINE: u16 = 160;

#[derive(Debug, Clone)]
pub struct Ppu {
    // Registers
    pub lcdc: u8,          // 0xFF40
    pub scy: u8,           // 0xFF42
    pub scx: u8,           // 0xFF43
    pub line: u8,          // 0xFF44 (LY)
    pub lym: u8,           // 0xFF45 (LYC)
    pub bgp: u8,           // 0xFF47
    pub obp0: u8,          // 0xFF48
    pub obp1: u8,          // 0xFF49
    pub wy: u8,            // 0xFF4A
    pub wx: u8,            // 0xFF4B
    // State
    pub mode: u8,          // 0-3
    pub mode_timer: u16,
    pub oam_dma_addr: u8,  // 0xFF46
    pub framebuffer: Vec<u8>,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            lcdc: 0,
            scy: 0,
            scx: 0,
            line: 0,
            lym: 0,
            bgp: 0xE4,
            obp0: 0xE4,
            obp1: 0xE4,
            wy: 0,
            wx: 0,
            mode: 2,
            mode_timer: 0,
            oam_dma_addr: 0,
            framebuffer: vec![0; FRAMEBUFFER_W * FRAMEBUFFER_H],
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Ppu::new()
    }
}

/// Map a tile data value (0-3) through a palette register to a shade (0-3).
pub fn palette_color(pal: u8, value: u8) -> u8 {
    ((pal >> (value * 2)) & 3) as u8
}
