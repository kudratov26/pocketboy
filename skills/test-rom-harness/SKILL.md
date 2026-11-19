---
name: test-rom-harness
description: Use when building or changing the headless harness and
  scoreboard, and when interpreting test ROM results.
---

# Test ROM harness

Harness requirements

- `harness <rom> [--cycles N] [--serial] [--screenshot out.png]
[--trace out.log]`
- `harness --all` discovers every .gb under ./roms, runs each with a
  timeout, and prints a table: ROM path | PASS/FAIL/TIMEOUT | detail.
  Exit code is nonzero if any REQUIRED ROM fails. Keep a REQUIRED list in
  a text file (harness/required.txt) matching the definition of done.

Detection per ROM family

- Blargg: accumulate serial bytes. Stop early when the output contains
  "Passed" or "Failed". Print the full serial text on failure: Blargg
  prints the failing sub-test number, which is the most useful clue.
  Default timeout: about 100 million T-cycles (cpu_instrs combined needs
  more than individual ROMs; scale per ROM).
- Mooneye: hook opcode 0x40 (LD B,B). On hit, check B,C,D,E,H,L against
  3,5,8,13,21,34 (pass) or 0x42 x6 (fail). Anything else: report as FAIL
  with the register values.
- dmg-acid2: run about 60 frames, save PNG, compare to the reference.
  On mismatch, also write a diff PNG highlighting wrong pixels and print
  the count of mismatched pixels and the first mismatch coordinates.

Important

- The harness may read ROMs and write only to ./target and ./artifacts.
- The scoreboard output is a deterministic text table. Paste it into
  PROGRESS.md at milestones.
- Never special-case a ROM by filename or hash.
- Record the last green commit hash and the count of commits so far in
  PROGRESS.md. If more than 45 minutes have passed since the last commit,
  stop and commit (or split) what you have before continuing.
