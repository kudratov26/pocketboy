# Progress

Status: M1 (foundations) — CPU implemention next; see milestone workflow.
Baseline scoreboard: see PROGRESS.md M1 section.

## Milestones

### M1 Foundations — DONE
Workspace skeleton: `pocketboy-core` (bus/memory map, cartridge ROM-only, timer, serial, joypad, PPU skeleton) + `harness` (ROM runner, scoreboard skeleton, required.txt).
Harness: `cargo run -p harness -- <rom> [--cycles N] [--serial] [--screenshot out.png] [--trace out.log]`, `harness --all`.
Scoreboard skeleton ready.

### M2 CPU — next
All 245 base opcodes + 256 CB opcodes, flags, DAA, interrupts (IME, EI delay, HALT), serial port capture. Goal: each cpu_instrs individual ROM passes.

### M3 Timers + interrupts
DIV, TIMA, TMA, TAC; interrupt flags/enables. Goal: cpu_instrs.gb + instr_timing pass.

### M4 PPU
Modes 0-3, LY/LYC/STAT, background, window, sprites, palettes, OAM DMA. Goal: dmg-acid2 pixel-exact.

### M5 Cartridges
MBC1, MBC3, MBC5, RAM enable, battery saves.

### M6 Input + frontend
Joypad register/interrupt; desktop window frontend (minifb).

### M7 Accuracy pass
mem_timing, halt_bug, Mooneye acceptance subset.

### M8 Stretch/polish
APU, README scoreboard table, screenshots, SUMMARY.md.

## Scoreboard

Record the latest scoreboard output in PROGRESS.md at each milestone.

Last green commit: (pending M1 commit)
Commits so far: (pending M1 commit)
