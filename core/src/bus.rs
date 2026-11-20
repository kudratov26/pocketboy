//! Bus: the system bus with the full DMG memory map and IO registers.

use crate::cartridge::{Cartridge, Header};
use crate::cartridge::{RomOnly, Mbc1, Mbc3, Mbc5, cartridge_kind};
use crate::joypad::{ButtonState, Joypad};
use crate::ppu::{Ppu, FRAMEBUFFER_H, FRAMEBUFFER_W, LINE, palette_color};
use crate::serial::Serial;
use crate::timer::Timer;

pub struct Bus {
    pub cartridge: Box<dyn Cartridge>,
    pub header: Header,
    pub vram: [u8; 8192],
    pub oam: [u8; 160],
    pub io: [u8; 0x80], // 0xFF00-0xFF7F
    pub hram: [u8; 127], // 0xFF80-0xFFFE
    pub wram: [u8; 8192],
    pub ie: u8,
    pub joypad: Joypad,
    pub serial: Serial,
    pub timer: Timer,
    pub ppu: Ppu,
}

impl Bus {
    pub fn new(rom: Vec<u8>) -> Bus {
        let header = Header::parse(&rom);
        let cart = make_cartridge(&rom, &header);
        let mut io = [0u8; 0x80];
        // DMG default IO values (GB VI "Memory map" / "Palettes"):
        // Some unused/reset IO regs read back as their initial values.
        // FF46 (OAM DMA address) starts as 0? GB VI: default 0? We store 0.
        io[0x4C] = 0x01; // KEY1 (prepare speed switch): on DMG reads 1? keep 0? We set 0.
        io[0x0F] = 0x00; // IF
        Bus {
            cartridge: cart,
            header,
            vram: [0; 8192],
            oam: [0x00; 160],
            io,
            hram: [0; 127],
            wram: [0; 8192],
            ie: 0,
            joypad: Joypad::new(),
            serial: Serial::new(),
            timer: Timer::new(),
            ppu: Ppu::new(),
        }
    }

    fn vram_read(&self, addr: u16) -> u8 {
        self.vram[(addr & 0x1FFF) as usize]
    }

    fn vram_write(&mut self, addr: u16, val: u8) {
        self.vram[(addr & 0x1FFF) as usize] = val;
    }

    /// Advance all hardware by `cycles` (one instruction's worth).
    pub fn tick(&mut self, cycles: u32) {
        {
            let io_if = &mut self.io[0x0F];
            self.timer.tick(cycles, io_if);
        }
        if self.joypad.take_interrupt() {
            self.io[0x0F] |= 0x10;
        }
        for _ in 0..cycles {
            self.ppu_tick();
        }
    }

    // ------------------------------------------------------------------
    // PPU tick
    // ------------------------------------------------------------------

    fn mode3_duration(&self) -> u16 {
        let sprites = self.sprites_on_line();
        80 + 2 * (sprites as u16).min(10)
    }

    fn sprites_on_line(&self) -> u8 {
        let line = self.ppu.line;
        let h = if self.ppu.lcdc & 0x04 != 0 { 16 } else { 8 };
        let l = line as i16;
        let mut count = 0u8;
        for i in 0..40 {
            let base = i * 4;
            let y0 = self.oam[base] as i16;
            let top = y0 - 16;
            if top <= l && l < top + h {
                count += 1;
            }
        }
        count
    }

    fn mode0_duration(&self) -> u16 {
        LINE.saturating_sub(80).saturating_sub(self.mode3_duration())
    }

    fn stat_fire(&mut self, bit: u8) {
        if self.io[0x41] & bit != 0 {
            self.io[0x0F] |= 0x02; // STAT interrupt
        }
    }

    fn next_line(&mut self) {
        self.ppu.line = self.ppu.line.wrapping_add(1);
        if self.ppu.mode == 1 {
            // VBlank line completed.
            if self.ppu.line >= 154 {
                self.ppu.line = 0;
                self.ppu.mode = 2;
            }
            self.ppu.mode_timer = 0;
            return;
        }
        if self.ppu.line == 154 {
            // Defensive: wrap.
            self.ppu.line = 0;
            self.ppu.mode = 2;
            self.ppu.mode_timer = 0;
            return;
        }
        // LY=LYC check.
        if self.ppu.line == self.ppu.lym && self.ppu.lym != 0 {
            if self.io[0x41] & 0x40 != 0 {
                self.io[0x0F] |= 0x02;
            }
        }
        if self.ppu.line == 144 {
            self.ppu.mode = 1;
            self.ppu.mode_timer = 0;
            if self.io[0x41] & 0x10 != 0 {
                self.io[0x0F] |= 0x01; // VBlank interrupt
            }
        } else {
            self.ppu.mode = 2;
            self.ppu.mode_timer = 0;
        }
    }

    fn ppu_tick(&mut self) {
        if false && self.ppu.lcdc & 0x80 == 0 {
            // Display off (DMG): stay in mode 2, LY at 0.
            self.ppu.mode = 2;
            self.ppu.line = 0;
            self.ppu.mode_timer = 0;
            return;
        }
        self.ppu.mode_timer += 1;
        match self.ppu.mode {
            2 => {
                // OAM scan.
                if self.ppu.mode_timer == 1 {
                    self.stat_fire(0x20); // OAM interrupt
                }
                if self.ppu.mode_timer >= 80 {
                    self.ppu.mode = 3;
                    self.ppu.mode_timer = 0;
                    self.render_line();
                }
            }
            3 => {
                if self.ppu.mode_timer >= self.mode3_duration() {
                    self.ppu.mode = 0;
                    self.ppu.mode_timer = 0;
                }
            }
            0 => {
                if self.ppu.mode_timer >= self.mode0_duration() {
                    self.ppu.mode_timer = 0;
                    self.stat_fire(0x08); // HBlank interrupt
                    self.next_line();
                }
            }
            1 => {
                // VBlank
                if self.ppu.mode_timer >= LINE {
                    self.ppu.mode_timer = 0;
                    self.next_line();
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    fn render_line(&mut self) {
        let line = self.ppu.line as u16;
        if false && self.ppu.lcdc & 0x80 == 0 {
            return;
        }
        let mut shade = [0u8; 160];
        let bgp = self.ppu.bgp;
        let mut sprite_done = [false; 160];
        // ---- Background ----
        let bg_on = self.ppu.lcdc & 0x01 != 0;
        let sprites_on = self.ppu.lcdc & 0x02 != 0;
        let tile_data_base = if self.ppu.lcdc & 0x10 != 0 { 0x8000u16 } else { 0x8800u16 };
        let bg_map_base = if self.ppu.lcdc & 0x08 != 0 { 0x9C00u16 } else { 0x9800u16 };
        let scy = self.ppu.scy as u16;
        let scx = self.ppu.scx as u16;
        if bg_on {
            let bg_y = line + scy;
            let ty = ((bg_y >> 3) & 0x1F) as u16;
            let py = (bg_y & 7) as u16;
            for x in 0..160u16 {
                let txy = x + scx;
                let tx = (txy >> 3) & 0x1F;
                let bit = 7 - (txy & 7);
                let addr = bg_map_base + (ty << 5) + tx;
                let tile = self.vram_read(addr) as u16;
                let p0 = self.vram_read(tile_data_base + tile * 16 + py * 2) as u16;
                let p1 = self.vram_read(tile_data_base + tile * 16 + py * 2 + 1) as u16;
                let v = ((p1 >> bit) & 1) << 1 | ((p0 >> bit) & 1);
                shade[x as usize] = palette_color(bgp, v as u8);
            }
        }

        // ---- Window ----
        if self.ppu.lcdc & 0x40 != 0 {
            let wy = self.ppu.wy as i16;
            let wx = self.ppu.wx as i16;
            let l = line as i16;
            if l >= wy {
                let win_map_base = if self.ppu.lcdc & 0x20 != 0 { 0x9C00u16 } else { 0x9800u16 };
                let wy_rel = (l - wy) as u16;
                let ty = (wy_rel >> 3) & 0x1F;
                let py = wy_rel & 7;
                for x in 0..160i16 {
                    let cx = x - wx;
                    if cx < 0 {
                        continue;
                    }
                    let cxx = cx as u16;
                    let tx = (cxx >> 3) & 0x1F;
                    let bit = 7 - (cxx & 7);
                    let addr = win_map_base + (ty << 5) + tx;
                    let tile = self.vram_read(addr) as u16;
                    let p0 = self.vram_read(0x8000 + tile * 16 + py * 2) as u16;
                    let p1 = self.vram_read(0x8000 + tile * 16 + py * 2 + 1) as u16;
                    let v = ((p1 >> bit) & 1) << 1 | ((p0 >> bit) & 1);
                    shade[x as usize] = palette_color(bgp, v as u8);
                }
            }
        }

        // ---- Sprites ----
        // Collect visible sprites for this line: (oam index, x, tile, flags, spr_y).
        let h = if self.ppu.lcdc & 0x04 != 0 { 16 } else { 8 };
        let l = line as i16;
        let mut sprites: [(u8, i16, u8, u8, i16); 10] = [(0, 0, 0, 0, 0); 10];
        let mut nspr = 0usize;
        for i in 0..40u16 {
            let base = i * 4;
            let y0 = self.oam[base as usize] as i16;
            let top = y0;
            if top <= l && l < top + h {
                let spr_y = line as i16 - top;
                let x0 = self.oam[base as usize + 1] as i16;
                let tile = self.oam[base as usize + 2];
                let flags = self.oam[base as usize + 3];
                if nspr == 10 {
                    // The 10-per-line limit: sprites are evaluated in OAM order
                    // and the first 10 covering this line win.
                    break;
                }
                sprites[nspr] = (i as u8, x0, tile, flags, spr_y);
                nspr += 1;
            }
        }
        // Sort by (x asc, OAM index asc) — simple insertion sort over <= 10.
        for k in 1..nspr {
            let mut j = k;
            while j > 0
                && (sprites[j - 1].1 > sprites[j].1
                    || (sprites[j - 1].1 == sprites[j].1 && sprites[j - 1].0 > sprites[j].0))
            {
                sprites.swap(j - 1, j);
                j -= 1;
            }
        }

        if std::env::var_os("ACID2_DEBUG").is_some() && line == 112 {
            eprintln!(
                "render 112: lcdc={:02X} sprites_on={} nspr={}",
                self.ppu.lcdc, sprites_on, nspr
            );
            for s in &sprites[..nspr] {
                eprintln!("  sprite idx={} x={} tile={} flags={:02X} spr_y={}", s.0, s.1, s.2, s.3, s.4);
            }
        }

        for &(_idx, x0, tile, flags, spr_y) in &sprites[..nspr] {
            if !sprites_on {
                break;
            }
            let behind = flags & 0x20 != 0;
            let palette_sel = flags & 0x07;
            let obp = if palette_sel == 1 { self.ppu.obp1 } else { self.ppu.obp0 };
            let top_tile = tile & !1;
            for dx in 0..8i16 {
                let px = x0 + dx;
                if px < 0 || px >= 160 {
                    continue;
                }
                let sx = px as usize;
                if sprite_done[sx] {
                    continue;
                }
                if shade[sx] != 0 && behind {
                    // Sprite is behind the background and the background wins.
                    continue;
                }
                let col = if flags & 0x08 != 0 { 7 - dx } else { dx };
                // 8x16 sprites: the bottom half uses tile+1 and rows 8-15.
                let (t, row) = if h == 16 && spr_y >= 8 {
                    (top_tile + 1, spr_y - 8)
                } else {
                    (top_tile, spr_y)
                };
                let (row, col) = if flags & 0x10 != 0 { (h - 1 - row, col) } else { (row, col) };
                let row_base = self.vram_read(0x8000 + (t as u16) * 16 + (row as u16) * 2);
                let row_base1 = self.vram_read(0x8000 + (t as u16) * 16 + (row as u16) * 2 + 1);
                let bit = 7 - (col as u16);
                let v = (((row_base1 >> bit) & 1) << 1) | ((row_base >> bit) & 1);
                if v == 0 {
                    continue;
                }
                shade[sx] = palette_color(obp, v);
                sprite_done[sx] = true;
            }
        }

        // Apply to framebuffer.
        if std::env::var_os("ACID2_DEBUG").is_some() && line == 40 {
            eprintln!("render 40: nspr={} sprites_on={} shade[60..80]={:?}", nspr, sprites_on, &shade[60..80]);
        }
        for x in 0..160usize {
            self.ppu.framebuffer[line as usize * 160 + x] = shade[x];
        }
    }

    // ------------------------------------------------------------------
    // Memory map
    // ------------------------------------------------------------------

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cartridge.read(addr),
            0x8000..=0x9FFF => self.vram_read(addr),
            0xA000..=0xBFFF => self.cartridge.read(addr),
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            0xFEA0..=0xFEFF => 0,
            0xFF00..=0xFF7F => self.io_read(addr),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x7FFF => self.cartridge.write(addr, val),
            0x8000..=0x9FFF => self.vram_write(addr, val),
            0xA000..=0xBFFF => self.cartridge.write(addr, val),
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = val,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = val,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = val,
            0xFEA0..=0xFEFF => {}
            0xFF00..=0xFF7F => self.io_write(addr, val),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = val,
            0xFFFF => self.ie = val,
        }
    }

    fn io_read(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF00 => self.joypad.read(),
            0xFF01 => self.serial.read(addr),
            0xFF02 => self.serial.read(addr),
            0xFF04 => self.timer.div as u8,
            0xFF05 => self.timer.tima,
            0xFF06 => self.timer.tma,
            0xFF07 => self.timer.tac,
            0xFF40 => self.ppu.lcdc,
            0xFF41 => {
                let mut v = self.io[0x41] & 0xF8;
                if self.ppu.line == self.ppu.lym {
                    v |= 0x04;
                }
                v |= self.ppu.mode as u8;
                v
            }
            0xFF44 => self.ppu.line,
            0xFF45 => self.ppu.lym,
            0xFF46 => self.ppu.oam_dma_addr,
            0xFF47 => self.ppu.bgp,
            0xFF48 => self.ppu.obp0,
            0xFF49 => self.ppu.obp1,
            0xFF4A => self.ppu.wy,
            0xFF4B => self.ppu.wx,
            _ => self.io[(addr - 0xFF00) as usize],
        }
    }

    fn io_write(&mut self, addr: u16, val: u8) {
        if std::env::var_os("ACID2_DEBUG").is_some()
            && matches!(addr, 0xFF40..=0xFF49)
        {
            eprintln!("io_write {:04X} = {:02X}", addr, val);
        }
        match addr {
            0xFF00 => self.joypad.write(val),
            0xFF01 | 0xFF02 => self.serial.write(addr, val),
            0xFF04 => self.timer.div = 0,
            0xFF05 => self.timer.tima = val,
            0xFF06 => self.timer.tma = val,
            0xFF07 => self.timer.set_tac(val),
            0xFF0F => self.io[0x0F] = val & 0x1F,
            0xFF40 => {
                self.ppu.lcdc = val;
            }
            0xFF41 => self.io[0x41] = (self.io[0x41] & 0x07) | (val & 0x78),
            0xFF42 => self.ppu.scy = val,
            0xFF43 => self.ppu.scx = val,
            0xFF44 => {} // LY is read-only on the DMG
            0xFF45 => self.ppu.lym = val,
            0xFF46 => {
                self.ppu.oam_dma_addr = val;
                // OAM DMA: copy 0x00-0x9F from (val << 8) to OAM. Immediate model.
                let src = val as u16;
                for i in 0..160u16 {
                    let b = self.read(src << 8 | i);
                    self.oam[i as usize] = b;
                }
            }
            0xFF47 => self.ppu.bgp = val,
            0xFF48 => self.ppu.obp0 = val,
            0xFF49 => self.ppu.obp1 = val,
            0xFF4A => self.ppu.wy = val,
            0xFF4B => self.ppu.wx = val,
            0xFF4C => self.io[0x4C] = val,
            _ => self.io[(addr - 0xFF00) as usize] = val,
        }
    }
}

fn make_cartridge(rom: &[u8], header: &Header) -> Box<dyn Cartridge> {
    match cartridge_kind(header.cart_type) {
        crate::cartridge::CartridgeKind::RomOnly => {
            Box::new(RomOnly::new(rom.to_vec(), header))
        }
        crate::cartridge::CartridgeKind::RomOnlyWithRam => {
            Box::new(RomOnly::new(rom.to_vec(), header))
        }
        crate::cartridge::CartridgeKind::Mbc1 => {
            Box::new(Mbc1::new(rom.to_vec(), header.ram_size()))
        }
        crate::cartridge::CartridgeKind::Mbc2 => {
            Box::new(RomOnly::new(rom.to_vec(), header))
        }
        crate::cartridge::CartridgeKind::Mbc3 => Box::new(Mbc3::new(rom.to_vec(), header.ram_size())),
        crate::cartridge::CartridgeKind::Mbc5 => Box::new(Mbc5::new(rom.to_vec(), header.ram_size())),
    }
}

// Re-exported for API users.
pub fn framebuffer_len() -> usize {
    FRAMEBUFFER_W * FRAMEBUFFER_H
}
pub fn button_state_default() -> ButtonState {
    ButtonState::default()
}
