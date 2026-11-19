---
name: git-discipline
description: Use before every commit and whenever the working tree has
  grown large. Keeps history small-step, readable, and always green.
---

# Git discipline

Pre-commit checklist (run every time)

1. `git status`: confirm only intended files are changed; no /target,
   ROMs, .sav, or artifacts.
2. `cargo build --workspace && cargo test --workspace` must pass.
3. `git diff --stat`: if the diff spans multiple unrelated areas or more
   than ~400 changed lines, split it into several commits using
   `git add -p` or by path.
4. Write the message: `<type>(<scope>): <imperative summary> [M<n>]`
   Types: feat, fix, test, refactor, perf, docs, chore.
   Scopes: cpu, bus, timer, ppu, cart, joypad, serial, harness, desktop.

Good history looks like
chore: initialize cargo workspace and .gitignore [M1]
feat(cart): parse ROM header and validate checksum [M1]
feat(bus): implement full memory map [M1]
feat(cpu): add register file and flag helpers [M2]
feat(cpu): implement 8-bit loads [M2]
feat(cpu): implement ALU opcodes [M2]
fix(cpu): correct DAA flags (passes 01-special) [M2]
...
Bad history: "M2: CPU", "wip", "updates", one commit touching 30 files.

Safety

- Commit before starting any risky refactor so there is a clean point to
  return to. If a refactor fails, `git restore .` or `git stash`, never
  rewrite published history.
- If the tree is dirty and tests are red after a crash/resume, stash and
  re-verify the last commit before continuing.
- Every ~hour, run `git log --oneline | wc -l` and `git log --oneline -15`
  to confirm commits are small and frequent.
