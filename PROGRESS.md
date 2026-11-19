# MISSION

Build "Pocketboy", a Game Boy (DMG) emulator in Rust, fully autonomously
overnight. No human is available. When something is ambiguous, pick the
simplest reasonable option, record it in DECISIONS.md, and keep going.
Never stop to ask questions.

# SUCCESS IS MEASURED BY TEST ROMS (the oracle)

Test ROMs live in ./roms (read-only; never modify them). You must build a
headless test harness (`cargo run -p harness -- <rom> [options]`) and a
scoreboard (`cargo run -p harness -- --all`) that runs every ROM and prints
PASS/FAIL per ROM plus a total. The scoreboard is your source of truth for
progress. Record its output in PROGRESS.md at each milestone.

Pass signals:

- Blargg: ROM writes text to the serial port (write 0x81 to 0xFF02 after
  putting a byte in 0xFF01). Capture bytes; PASS if output contains "Passed",
  FAIL if it contains "Failed". Timeout after a fixed emulated-cycle budget.
- Mooneye: ROM executes `LD B,B` (opcode 0x40) when finished. PASS if
  B,C,D,E,H,L = 3,5,8,13,21,34. FAIL if all are 0x42. Detect via the
  executed-opcode hook in the harness.
- dmg-acid2: run N frames, dump the framebuffer to PNG, and compare it
  pixel-for-pixel with roms/acid/dmg-acid2-reference.png. Map the 4 DMG
  shades to the same 4 grey values the reference uses.

# DEFINITION OF DONE

Required (all must hold):

1. `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
   both pass.
2. Scoreboard passes: all 11 Blargg cpu_instrs individual ROMs, the combined
   cpu_instrs.gb, instr_timing, and dmg-acid2 (pixel-exact).
3. Cartridge support: ROM-only, MBC1, MBC3 (RTC may be simplified), MBC5,
   with battery-backed save files (.sav) loaded and saved.
4. Screenshots exist in /screenshots (see skill: screenshots).
5. README.md: how to build, run, test, and the current scoreboard table.
6. PROGRESS.md is current; git history shows many small, focused commits
   (at least one per feature, at least 40 total by the end), with a tag
   m<n>-done on each completed milestone.
   Stretch (only after required items pass): mem_timing, halt_bug, interrupt
   tests, a Mooneye acceptance subset, APU audio, desktop window frontend.

# STACK (fixed, do not deliberate)

- Rust stable, cargo workspace:
  /core library crate `pocketboy-core`: no I/O, no std::fs, no
  windowing; pure emulation. Deterministic.
  /harness binary crate: ROM runner, scoreboard, PNG dump, trace dump.
  /desktop binary crate (stretch only): window + input via `minifb`.
- Allowed dependencies: `png` (screenshots) and `minifb` (stretch only).
  Everything else is std. No external emulator code or crates.
- Do NOT copy code from existing emulators from memory verbatim or crates.
  Implement from your understanding of the hardware (Pan Docs knowledge).
- No network access is assumed. Do not try to download anything.

# ARCHITECTURE RULES

- Core exposes: `GameBoy::new(rom: Vec<u8>)`, `step()` (one CPU
  instruction, returns T-cycles), `run_frame()`, `framebuffer() -> &[u8]`
  (160x144), `set_buttons(...)`, `serial_output() -> &[u8]`,
  `save_ram()/load_ram()`.
- Cycle-accurate-ish: advance timers, PPU, and DMA by the cycles each
  instruction consumed. Memory accesses should be structured so that
  sub-instruction timing can be refined later without a rewrite.
- All hardware (CPU, bus, timer, PPU, cartridge, joypad, serial) in separate
  modules with unit tests where logic is non-trivial.
- Boot ROM: skip it. Initialize registers and I/O to the documented
  post-boot DMG values (AF=0x01B0, BC=0x0013, DE=0x00D8, HL=0x014D,
  SP=0xFFFE, PC=0x0100, plus I/O registers).

# MILESTONES (one commit each; build in order)

M1 Foundations: workspace, ROM loader, header parsing, ROM-only cartridge,
bus with full memory map, harness that loads a ROM and runs N cycles,
scoreboard skeleton. `cargo test` passes.
M2 CPU: all 245 base opcodes + 256 CB opcodes, flags, DAA, interrupt
handling (IME, EI delay, HALT), serial port capture. Goal: each
cpu_instrs individual ROM passes. Fix them one by one using the
scoreboard.
M3 Timers + interrupts: DIV, TIMA, TMA, TAC with correct falling-edge
behavior, interrupt flags/enables, correct per-instruction cycle counts.
Goal: cpu_instrs.gb combined and instr_timing pass.
M4 PPU: modes 0-3 with correct dot timing, LY/LYC, STAT interrupts,
background, window, sprites (10 per line limit, priority, flipping,
8x16), palettes BGP/OBP0/OBP1, OAM DMA. Goal: dmg-acid2 pixel-exact.
M5 Cartridges: MBC1, MBC3, MBC5, RAM enable, banking edge cases,
battery saves. Unit tests for banking logic.
M6 Input + frontend: joypad register and interrupt; harness can replay a
scripted input file; desktop window frontend with keyboard mapping
(only if minifb builds; otherwise skip and note in BLOCKERS.md).
M7 Accuracy pass: mem_timing, halt_bug, interrupt timing, OAM/STAT edge
cases; Mooneye acceptance subset. Work from the scoreboard failures.
M8 Stretch/polish: APU (4 channels) if time remains, README scoreboard
table, screenshots, SUMMARY.md.

# NON-GOALS (do not build)

Game Boy Color features, Super Game Boy, link cable networking, save
states, rewind, debugger GUI, shaders/filters, netplay, boot ROM execution,
audio output to hardware (APU sample generation is allowed, playback is not).

# WORKING RULES

- Follow the skills in .claude/skills/: milestone-workflow, test-rom-harness,
  cpu-debugging, ppu-accuracy, progress-and-recovery, scope-guard,
  screenshots. Read them before starting.
- Never weaken, skip, or special-case a test ROM to get a pass. No code
  paths that detect a particular ROM by name or checksum. Fix the emulator.
- After each feature: build + test + commit (see GIT COMMIT POLICY).
  After each milestone: also run clippy and the full scoreboard, update
  PROGRESS.md, commit, and tag.
- If stuck on one failure for ~30 minutes of effort or 3 distinct failed
  approaches, write it to BLOCKERS.md, move to the next failing ROM, and
  come back later.
- Prefer clarity over cleverness. Performance goal is modest: run at least
  10x real-time headless in release mode.
- Do not touch anything outside the project directory.

# GIT COMMIT POLICY (strict)

- Commit early and often. Never let more than ~30-45 minutes of work or
  more than one logical feature accumulate uncommitted. The final history
  must NOT look like one big commit.
- Commit after EACH logical unit, not just each milestone. Examples:
  one CPU opcode group (e.g. "8-bit loads"), the DAA fix, timer DIV/TIMA,
  one PPU stage (background rendering), sprites, window, each MBC type,
  harness serial capture, each bug fix that turns a ROM from FAIL to PASS.
- Every commit must build (`cargo build --workspace`) and pass
  `cargo test --workspace`. Run clippy before committing too.
  Never commit a red state. No "WIP" or "fix stuff" commits.
- Commit messages use Conventional Commits with the milestone tag:
  feat(cpu): implement 8-bit load and arithmetic opcodes [M2]
  fix(cpu): correct DAA half-carry handling [M2]
  test(harness): add Mooneye LD B,B result detection [M1]
  feat(ppu): render background scanlines [M4]
  docs: update PROGRESS.md scoreboard after M3
  Subject line max 72 chars, imperative mood. Add a short body when the
  "why" isn't obvious (for example which ROM this fixes).
- Each time a test ROM flips from FAIL to PASS, make a dedicated commit
  for it and mention the ROM name in the message.
- Milestone completion gets its own commit that updates PROGRESS.md
  (scoreboard + checklist), tagged: `git tag m<n>-done`.
- Commit PROGRESS.md/DECISIONS.md/BLOCKERS.md updates separately from
  code changes (docs: ... commits).
- Never use `git commit --amend`, rebase, force-push, or squash. History
  is append-only. Never commit build output (/target), ROMs, or save files;
  keep .gitignore correct from the first commit.
- First commit of the run: .gitignore + empty workspace skeleton.

# FINAL STEP

When all required items pass (or time/budget is nearly out), ensure the last
commit is green, then write SUMMARY.md: what works, scoreboard, what doesn't,
known inaccuracies, how to run, suggested next steps.
