---
name: milestone-workflow
description: Use at the start and end of every milestone and every
  feature. Defines the build, test, commit loop so unattended work never
  ends in a broken state or a giant commit.
---

# Milestone workflow

For each milestone M<n>:

1. Re-read its scope. Break it into 4-10 small features and list them in
   PROGRESS.md, noting which ROMs should newly pass.
2. Run the scoreboard to record the baseline.
3. For EACH feature in the list:
   a. Implement it in small steps; run only the relevant unit test or ROM.
   b. Run `cargo build --workspace` and `cargo test --workspace`.
   c. Run the scoreboard if the feature could affect ROM results. No
   previously passing ROM may regress.
   d. Commit immediately with a Conventional Commit message
   (feat/fix/test/refactor/docs + scope + [M<n>]).
   e. Tick the feature in PROGRESS.md (batched into a docs commit at least
   once per milestone or per hour).
4. At milestone end: clippy with -D warnings, full scoreboard, update
   PROGRESS.md with scoreboard output, commit "docs: complete M<n>", then
   `git tag m<n>-done`.

Rules

- Never commit with failing tests, clippy errors, or a broken build.
- Never bundle unrelated changes in one commit. If you notice you've
  touched two unrelated things, split them with `git add -p`.
- Never amend, rebase, or squash. If a commit was wrong, fix it in a new
  commit.
- Never start M(n+1) work during M(n). Park ideas in IDEAS.md.
- If a milestone runs very long, cut to the minimum that passes its goal
  ROMs, record the cut in DECISIONS.md, and move on.
