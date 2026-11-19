---
name: progress-and-recovery
description: Use throughout the run to keep PROGRESS.md, DECISIONS.md and
  BLOCKERS.md accurate so the human can review fast and a crashed run can resume.
---

# Progress and recovery

Files at repo root

- PROGRESS.md: milestone checklist, latest scoreboard, last green commit
  hash, and "next action" as one sentence.
- DECISIONS.md: non-obvious choices: what, why, alternatives.
- BLOCKERS.md: ROM failures not yet solved: ROM name, serial output or
  diff summary, hypotheses tried.
- IDEAS.md: out-of-scope ideas, never implemented.

Cadence

- Update PROGRESS.md at every milestone start, every commit, and about
  every 45 minutes.

On restart or resume

1. Read PROGRESS.md, DECISIONS.md, BLOCKERS.md, and `git log --oneline -20`.
2. `git status`. If dirty, run tests and the scoreboard. If no regression
   against PROGRESS.md, commit. Otherwise `git stash` and re-verify the last commit.
3. Continue from "next action". Do not redo completed milestones.

Budget

- If under ~10% of budget remains: stop new work, ensure the last commit is
  green, write SUMMARY.md.
