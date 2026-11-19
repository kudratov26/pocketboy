---
name: cpu-debugging
description: Use when a Blargg cpu_instrs or instr_timing ROM fails, or
  when CPU behavior seems wrong.
---

# CPU debugging

Workflow

1. Run the failing individual ROM, never the combined one, first. The
   serial output names the failing opcode(s), e.g. "27 DAA".
2. Re-derive that instruction's behavior from first principles: which flags
   change (Z N H C), operand width, cycle count. Compare against your code
   line by line.
3. Add a focused unit test for that instruction with hand-computed expected
   registers and flags before fixing it.
4. Common culprits, check these first:
   - DAA: depends on N, H, C flags and the A value; carry handling.
   - ADD/ADC/SBC/SUB/CP: half-carry and carry with carry-in.
   - ADD HL,rr and ADD SP,e8 / LD HL,SP+e8: half-carry/carry come from the
     low byte (or bit 11/15 for HL), flags Z reset.
   - POP AF: low nibble of F must always be zero.
   - Rotates: RLCA/RLA etc. always clear Z, while the CB versions set Z.
   - BIT sets Z=!bit, N=0, H=1; carry untouched.
   - (HL) CB ops take extra cycles; BIT n,(HL) differs from RES/SET.
   - EI takes effect after the NEXT instruction; HALT with IME=0 and a
     pending interrupt triggers the halt bug.
   - Interrupt dispatch takes 20 cycles and pushes PC high byte first.
5. Optional: add a trace mode printing PC, opcode, registers per
   instruction to a file (never to stdout for long runs) and inspect the
   region around the failure.

Timing

- instr_timing failures: check the cycle table. Conditional jumps/calls/
  returns take different cycles when taken vs not taken.
