---
name: ppu-accuracy
description: Use during the PPU milestone and whenever dmg-acid2 or any
  graphics output is wrong.
---

# PPU accuracy

Structure

- 456 dots per scanline, 154 lines (144 visible + 10 VBlank).
- Per visible line: Mode 2 (OAM scan, 80 dots), Mode 3 (drawing, 172+
  dots, extended by sprites and window), Mode 0 (HBlank, remainder).
- Start with fixed mode lengths, render each scanline at the Mode 3 to
  HBlank transition, and get dmg-acid2 working. Refine only if later
  timing ROMs require it.
- STAT interrupt uses a combined "STAT line" with rising-edge detection
  (mode 0/1/2 sources and LYC=LY). Do not fire on every mode change.
- LCD off (LCDC bit 7 = 0): LY=0, mode 0, PPU stops, and the framebuffer
  keeps blank output. Turning it back on restarts at line 0.

dmg-acid2 debugging

- Use the diff PNG to locate wrong regions. Typical causes per region:
  - Face eyes/nose missing: sprite priority (lower X wins, then lower OAM
    index), 10-sprites-per-line limit, 8x16 tile index masking.
  - Mouth/hair wrong: window internal line counter (increments only on
    lines where the window was actually drawn), WX=7 offset, WY condition.
  - Tilemap/tile data area mix-up: LCDC bits 3, 4, 6 select map/data areas;
    signed tile index addressing at 0x8800 mode.
  - Palette: BGP/OBP shade mapping; sprite color 0 is transparent;
    BG-over-OBJ priority flag on sprites.
  - Scroll: SCX/SCY wraparound at 256.

Output

- Framebuffer is 160x144 bytes holding shade indices 0-3. The harness maps
  them to grey values only when saving PNGs.
