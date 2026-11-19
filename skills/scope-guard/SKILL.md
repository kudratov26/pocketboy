---
name: scope-guard
description: Use whenever tempted to add a feature, dependency, or
  refactor not in the mission prompt, or when a task is taking too long.
---

# Scope guard

Before adding anything, ask:

1. Is it in the current milestone or the definition of done? If not, write
   it to IDEAS.md and move on.
2. Is it on the NON-GOALS list (CGB, save states, debugger GUI, audio
   playback...)? Then do not build it.
3. Does a simpler version satisfy the target ROM? Ship the simpler version.

Dependencies: only `png` (and `minifb` for the stretch frontend). Anything
else needs a DECISIONS.md entry, and the default answer is no.

Refactors: only to unblock the current milestone.

If a task blows its time box, cut scope instead of extending. A passing
smaller set of ROMs beats a half-finished accuracy rewrite.
