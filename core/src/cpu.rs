//! CPU: registers, flags, interrupt handling, and instruction execution.

use crate::bus::Bus;

const FZ: u8 = 0x80; // Z
const FN: u8 = 0x40; // N
const FH: u8 = 0x20; // H
const FC: u8 = 0x10; // C

#[derive(Debug, Clone, Copy)]
pub struct Cpu {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
    pub ime: bool,
    pub halt: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu::new()
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        // Post-boot DMG values (boot ROM is skipped).
        Cpu {
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            pc: 0x0100,
            sp: 0xFFFE,
            ime: false,
            halt: false,
        }
    }

    // ---- flag helpers ----
    fn set_z(&mut self, v: bool) {
        if v {
            self.f |= FZ;
        } else {
            self.f &= !FZ;
        }
    }
    fn set_n(&mut self, v: bool) {
        if v {
            self.f |= FN;
        } else {
            self.f &= !FN;
        }
    }
    fn set_h(&mut self, v: bool) {
        if v {
            self.f |= FH;
        } else {
            self.f &= !FH;
        }
    }
    fn set_c(&mut self, v: bool) {
        if v {
            self.f |= FC;
        } else {
            self.f &= !FC;
        }
    }

    // ---- register helpers ----
    fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = v as u8;
    }
    fn de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = v as u8;
    }
    fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    fn r8(&mut self, bus: &mut Bus, r: u8) -> u8 {
        match r {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => bus.read(self.hl()),
            _ => self.a,
        }
    }

    fn set_r8(&mut self, bus: &mut Bus, r: u8, v: u8) {
        match r {
            0 => self.b = v,
            1 => self.c = v,
            2 => self.d = v,
            3 => self.e = v,
            4 => self.h = v,
            5 => self.l = v,
            6 => bus.write(self.hl(), v),
            _ => self.a = v,
        }
    }

    fn push16(&mut self, bus: &mut Bus, v: u16) {
        self.sp = self.sp.wrapping_sub(2);
        bus.write(self.sp, v as u8);
        bus.write(self.sp + 1, (v >> 8) as u8);
    }

    fn pop16(&mut self, bus: &mut Bus) -> u16 {
        let lo = bus.read(self.sp);
        let hi = bus.read(self.sp + 1);
        self.sp = self.sp.wrapping_add(2);
        ((hi as u16) << 8) | lo as u16
    }

    // ---- interrupt handling ----
    fn interrupt_pending(&self, bus: &mut Bus) -> bool {
        self.ime && (bus.read(0xFF0F) & bus.ie) != 0
    }

    fn service_interrupt(&mut self, bus: &mut Bus) -> u32 {
        let v = bus.read(0xFF0F) & bus.ie;
        // Priority: VBlank (bit 0) > LCDC (1) > Timer (2) > Serial (3) > Joypad (4).
        let n = v.trailing_zeros() as u16;
        let ifr = bus.read(0xFF0F);
        bus.write(0xFF0F, ifr & !(1 << n));
        self.halt = false;
        self.push16(bus, self.pc);
        self.pc = 0x40 + 8 * n;
        5
    }

    pub fn step(&mut self, bus: &mut Bus) -> u32 {
        if self.interrupt_pending(bus) {
            return self.service_interrupt(bus);
        }
        if self.halt {
            return 2;
        }
        let op = bus.read(self.pc);
        self.pc += 1;
        if op == 0xCB {
            let op2 = bus.read(self.pc);
            self.pc += 1;
            self.execute_cb(op2, bus)
        } else {
            self.execute(op, bus)
        }
    }

    fn fetch16(&mut self, bus: &mut Bus) -> u16 {
        let lo = bus.read(self.pc);
        self.pc += 1;
        let hi = bus.read(self.pc);
        self.pc += 1;
        ((hi as u16) << 8) | lo as u16
    }

    // ---- ALU helpers ----
    fn add_r(&mut self, bus: &mut Bus, r: u8, carry_in: bool) {
        let a = self.a;
        let b = self.r8(bus, r);
        let c = carry_in as u8;
        let res = a.wrapping_add(b).wrapping_add(c);
        self.a = res;
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(((a & 0x0F) as u16 + (b & 0x0F) as u16 + c as u16) & 0x10 != 0);
        self.set_c((a as u16 + b as u16 + c as u16) > 0xFF);
    }

    fn sub_r(&mut self, bus: &mut Bus, r: u8, carry_in: bool) {
        let a = self.a;
        let b = self.r8(bus, r);
        let c = carry_in as u8;
        let res = a.wrapping_sub(b).wrapping_sub(c);
        self.a = res;
        self.set_z(res == 0);
        self.set_n(true);
        self.set_h((a & 0x0F) < (b & 0x0F).wrapping_add(c));
        self.set_c((a as i16 - b as i16 - c as i16) < 0);
    }

    fn and_r(&mut self, bus: &mut Bus, r: u8) {
        let a = self.a & self.r8(bus, r);
        self.a = a;
        self.set_z(a == 0);
        self.set_n(false);
        self.set_h(true);
        self.set_c(false);
    }

    fn xor_r(&mut self, bus: &mut Bus, r: u8) {
        let a = self.a ^ self.r8(bus, r);
        self.a = a;
        self.set_z(a == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);
    }

    fn or_r(&mut self, bus: &mut Bus, r: u8) {
        let a = self.a | self.r8(bus, r);
        self.a = a;
        self.set_z(a == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);
    }

    fn cp_r(&mut self, bus: &mut Bus, r: u8) {
        let a = self.a;
        let b = self.r8(bus, r);
        self.set_z(a == b);
        self.set_n(true);
        self.set_h((a & 0x0F) < (b & 0x0F).wrapping_add(0)); // borrow from bit 3
        self.set_c((a as i16 - b as i16) < 0);
    }

    fn inc_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let res = v.wrapping_add(1);
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h((v & 0x0F) + 1 > 0x0F);
    }

    fn dec_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let res = v.wrapping_sub(1);
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(true);
        self.set_h(v & 0x0F == 0);
    }

    fn daa(&mut self) {
        // DMG DAA (see DECISIONS.md).
        let mut c = self.f & FC != 0;
        if self.f & FN == 0 {
            if self.f & FH != 0 || self.a & 0x0F > 0x09 {
                self.a = self.a.wrapping_add(0x06);
            }
            if c || self.a > 0x99 {
                self.a = self.a.wrapping_add(0x60);
                c = true;
            }
        } else {
            if self.f & FH != 0 || self.a & 0x0F > 0x09 {
                self.a = self.a.wrapping_sub(0x06);
            }
            if c || self.a > 0x99 {
                self.a = self.a.wrapping_sub(0x60);
                c = false;
            }
        }
        self.set_z(self.a == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c);
    }

    // ---- rotates/shifts/swaps ----
    fn rlc_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = (v >> 7) & 1;
        let res = ((v << 1) & 0xFF) | c;
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn rrc_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = v & 1;
        let res = (v >> 1) | (c << 7);
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn rl_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = (v >> 7) & 1;
        let res = ((v << 1) & 0xFF) | (self.f & FC != 0) as u8;
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn rr_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = v & 1;
        let res = (v >> 1) | ((self.f & FC != 0) as u8) << 7;
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn sla_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = (v >> 7) & 1;
        let res = (v << 1) & 0xFF;
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn sra_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = v & 1;
        let res = (v >> 1) | (v & 0x80);
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn swap_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let res = ((v & 0x0F) << 4) | (v >> 4);
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);
    }

    fn srl_r(&mut self, bus: &mut Bus, r: u8) {
        let v = self.r8(bus, r);
        let c = v & 1;
        let res = v >> 1;
        self.set_r8(bus, r, res);
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(c == 1);
    }

    fn bit_r(&mut self, bus: &mut Bus, op: u8) {
        let b = ((op >> 3) & 7) as u8;
        let r = (op & 7) as u8;
        let v = self.r8(bus, r);
        self.set_z((v >> b) & 1 == 0);
        self.set_n(false);
        self.set_h(true);
    }

    fn set_r(&mut self, bus: &mut Bus, op: u8) {
        let b = 1u8 << ((op >> 3) & 7);
        let r = (op & 7) as u8;
        let v = self.r8(bus, r) | b;
        self.set_r8(bus, r, v);
    }

    fn res_r(&mut self, bus: &mut Bus, op: u8) {
        let b = 1u8 << ((op >> 3) & 7);
        let r = (op & 7) as u8;
        let v = self.r8(bus, r) & !b;
        self.set_r8(bus, r, v);
    }

    fn add_rp(&mut self, _bus: &mut Bus, rp: u16) {
        let hl = self.hl();
        let res = hl.wrapping_add(rp);
        self.set_hl(res);
        self.set_n(false);
        self.set_h((((hl & 0x0FFF) as u32 + (rp & 0x0FFF) as u32) & 0x1000) != 0);
        self.set_c((hl as u32 + rp as u32) > 0xFFFF);
    }

    fn add_imm(&mut self, _bus: &mut Bus, v: u8, carry_in: bool) {
        let a = self.a;
        let c = carry_in as u8;
        let res = a.wrapping_add(v).wrapping_add(c);
        self.a = res;
        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(((a & 0x0F) as u16 + (v & 0x0F) as u16 + c as u16) & 0x10 != 0);
        self.set_c((a as u16 + v as u16 + c as u16) > 0xFF);
    }

    fn sub_imm(&mut self, _bus: &mut Bus, v: u8, carry_in: bool) {
        let a = self.a;
        let c = carry_in as u8;
        let res = a.wrapping_sub(v).wrapping_sub(c);
        self.a = res;
        self.set_z(res == 0);
        self.set_n(true);
        self.set_h((a & 0x0F) < (v & 0x0F).wrapping_add(c));
        self.set_c((a as i16 - v as i16 - c as i16) < 0);
    }

    fn and_imm(&mut self, _bus: &mut Bus, v: u8) {
        self.a &= v;
        self.set_z(self.a == 0);
        self.set_n(false);
        self.set_h(true);
        self.set_c(false);
    }

    fn or_imm(&mut self, _bus: &mut Bus, v: u8) {
        self.a |= v;
        self.set_z(self.a == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);
    }

    fn xor_imm(&mut self, _bus: &mut Bus, v: u8) {
        self.a ^= v;
        self.set_z(self.a == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);
    }

    fn cp_imm(&mut self, _bus: &mut Bus, v: u8) {
        self.set_z(self.a == v);
        self.set_n(true);
        self.set_h((self.a & 0x0F) < (v & 0x0F));
        self.set_c((self.a as i16 - v as i16) < 0);
    }

    // ---- execute one base opcode ----
    fn execute(&mut self, op: u8, bus: &mut Bus) -> u32 {
        match op {
            0x00 => 1, // NOP
            0x01 => {
                let v = self.fetch16(bus);
                self.set_bc(v);
                3
            }
            0x02 => {
                bus.write(self.bc(), self.a);
                2
            }
            0x03 => {
                let v = self.bc().wrapping_add(1);
                self.set_bc(v);
                3
            }
            0x04 => {
                self.inc_r(bus, 0);
                1
            }
            0x05 => {
                self.dec_r(bus, 0);
                1
            }
            0x06 => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.b = v;
                2
            }
            0x07 => {
                self.rlc_r(bus, 7); // rotate A
                1
            }
            0x08 => {
                let nn = self.fetch16(bus);
                bus.write(nn, self.sp as u8);
                bus.write(nn + 1, (self.sp >> 8) as u8);
                3
            }
            0x09 => {
                self.add_rp(bus, self.bc());
                1
            }
            0x0A => {
                self.a = bus.read(self.bc());
                2
            }
            0x0B => {
                let v = self.bc().wrapping_sub(1);
                self.set_bc(v);
                3
            }
            0x0C => {
                self.inc_r(bus, 1);
                1
            }
            0x0D => {
                self.dec_r(bus, 1);
                1
            }
            0x0E => {
                self.c = bus.read(self.pc);
                self.pc += 1;
                2
            }
            0x0F => {
                self.rrc_r(bus, 7);
                1
            }
            0x10 => {
                self.pc += 1; // STOP 0
                2
            }
            0x11 => {
                let v = self.fetch16(bus);
                self.set_de(v);
                3
            }
            0x12 => {
                bus.write(self.de(), self.a);
                2
            }
            0x13 => {
                let v = self.de().wrapping_add(1);
                self.set_de(v);
                3
            }
            0x14 => {
                self.inc_r(bus, 2);
                1
            }
            0x15 => {
                self.dec_r(bus, 2);
                1
            }
            0x16 => {
                self.d = bus.read(self.pc);
                self.pc += 1;
                2
            }
            0x17 => {
                self.rl_r(bus, 7);
                1
            }
            0x18 => {
                let n = bus.read(self.pc) as i8 as i16;
                self.pc = self.pc.wrapping_add(1);
                self.pc = self.pc.wrapping_add(n as u16);
                2
            }
            0x19 => {
                self.add_rp(bus, self.de());
                1
            }
            0x1A => {
                self.a = bus.read(self.de());
                2
            }
            0x1B => {
                let v = self.de().wrapping_sub(1);
                self.set_de(v);
                3
            }
            0x1C => {
                self.inc_r(bus, 3);
                1
            }
            0x1D => {
                self.dec_r(bus, 3);
                1
            }
            0x1E => {
                self.e = bus.read(self.pc);
                self.pc += 1;
                2
            }
            0x1F => {
                self.rr_r(bus, 7);
                1
            }
            0x20 => {
                // JR NZ
                let n = bus.read(self.pc) as i8 as i16;
                self.pc = self.pc.wrapping_add(1);
                if self.f & FZ == 0 {
                    self.pc = self.pc.wrapping_add(n as u16);
                }
                2
            }
            0x21 => {
                let v = self.fetch16(bus);
                self.set_hl(v);
                3
            }
            0x22 => {
                // LD (HL+),A
                bus.write(self.hl(), self.a);
                let v = self.hl().wrapping_add(1);
                self.set_hl(v);
                2
            }
            0x23 => {
                let v = self.hl().wrapping_add(1);
                self.set_hl(v);
                3
            }
            0x24 => {
                self.inc_r(bus, 4);
                1
            }
            0x25 => {
                self.dec_r(bus, 4);
                1
            }
            0x26 => {
                self.h = bus.read(self.pc);
                self.pc += 1;
                2
            }
            0x27 => {
                self.daa();
                4
            }
            0x28 => {
                // JR Z
                let n = bus.read(self.pc) as i8 as i16;
                self.pc = self.pc.wrapping_add(1);
                if self.f & FZ != 0 {
                    self.pc = self.pc.wrapping_add(n as u16);
                }
                2
            }
            0x29 => {
                self.add_rp(bus, self.hl());
                1
            }
            0x2A => {
                // LD A,(HL+)
                self.a = bus.read(self.hl());
                let v = self.hl().wrapping_add(1);
                self.set_hl(v);
                2
            }
            0x2B => {
                let v = self.hl().wrapping_sub(1);
                self.set_hl(v);
                3
            }
            0x2C => {
                self.inc_r(bus, 5);
                1
            }
            0x2D => {
                self.dec_r(bus, 5);
                1
            }
            0x2E => {
                self.l = bus.read(self.pc);
                self.pc += 1;
                2
            }
            0x2F => {
                self.a ^= 0xFF;
                self.set_n(true);
                self.set_h(true);
                1
            }
            0x30 => {
                // JP NC
                let w = self.fetch16(bus);
                if self.f & FC == 0 {
                    self.pc = w;
                }
                3
            }
            0x31 => {
                self.sp = self.fetch16(bus);
                3
            }
            0x32 => {
                // LD (HL-),A
                bus.write(self.hl(), self.a);
                let v = self.hl().wrapping_sub(1);
                self.set_hl(v);
                2
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                3
            }
            0x34 => {
                // INC (HL)
                let old = bus.read(self.hl());
                let v = old.wrapping_add(1);
                bus.write(self.hl(), v);
                self.set_z(v == 0);
                self.set_n(false);
                self.set_h((old & 0x0F) + 1 > 0x0F);
                2
            }
            0x35 => {
                // DEC (HL)
                let old = bus.read(self.hl());
                let v = old.wrapping_sub(1);
                bus.write(self.hl(), v);
                self.set_z(v == 0);
                self.set_n(true);
                self.set_h(old & 0x0F == 0);
                2
            }
            0x36 => {
                let v = bus.read(self.pc);
                self.pc += 1;
                bus.write(self.hl(), v);
                2
            }
            0x37 => {
                self.set_c(true);
                1
            }
            0x38 => {
                // JP C
                let w = self.fetch16(bus);
                if self.f & FC != 0 {
                    self.pc = w;
                }
                3
            }
            0x39 => {
                self.add_rp(bus, self.sp);
                1
            }
            0x3A => {
                // LD A,(HL-)
                self.a = bus.read(self.hl());
                let v = self.hl().wrapping_sub(1);
                self.set_hl(v);
                2
            }
            0x3B => {
                self.sp = self.sp.wrapping_sub(1);
                3
            }
            0x3C => {
                self.inc_r(bus, 7);
                1
            }
            0x3D => {
                self.dec_r(bus, 7);
                1
            }
            0x3E => {
                self.a = bus.read(self.pc);
                self.pc += 1;
                2
            }
            0x3F => {
                self.set_c(self.f & FC == 0);
                1
            }
            0x40..=0x75 | 0x78..=0x7F => {
                // LD r,r (0x76 is HALT)
                let d = ((op as i8 - 0x40) / 8) as u8;
                let s = ((op as i8 - 0x40) % 8) as u8;
                let v = self.r8(bus, s);
                self.set_r8(bus, d, v);
                1
            }
            0x76 => {
                self.halt = true;
                1
            }
            0x80..=0x87 => {
                self.add_r(bus, (op & 7) as u8, false);
                1
            }
            0x88..=0x8F => {
                self.add_r(bus, (op & 7) as u8, true);
                1
            }
            0x90..=0x97 => {
                self.sub_r(bus, (op & 7) as u8, false);
                1
            }
            0x98..=0x9F => {
                self.sub_r(bus, (op & 7) as u8, true);
                1
            }
            0xA0..=0xA7 => {
                self.and_r(bus, (op & 7) as u8);
                1
            }
            0xA8..=0xAF => {
                self.xor_r(bus, (op & 7) as u8);
                1
            }
            0xB0..=0xB7 => {
                self.or_r(bus, (op & 7) as u8);
                1
            }
            0xB8..=0xBF => {
                self.cp_r(bus, (op & 7) as u8);
                1
            }
            0xC0 => {
                if self.f & FZ == 0 {
                    self.pc = self.pop16(bus);
                }
                1
            }
            0xC1 => {
                let v = self.pop16(bus);
                self.set_bc(v);
                3
            }
            0xC2 => {
                let w = self.fetch16(bus);
                if self.f & FZ == 0 {
                    self.pc = w;
                }
                3
            }
            0xC3 => {
                self.pc = self.fetch16(bus);
                3
            }
            0xC4 => {
                let w = self.fetch16(bus);
                if self.f & FZ == 0 {
                    self.push16(bus, self.pc);
                    self.pc = w;
                }
                3
            }
            0xC5 => {
                self.push16(bus, self.bc());
                3
            }
            0xC6 => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.add_imm(bus, v, false);
                2
            }
            0xC7 => {
                self.push16(bus, self.pc);
                self.pc = 0x00;
                3
            }
            0xC8 => {
                if self.f & FZ != 0 {
                    self.pc = self.pop16(bus);
                }
                1
            }
            0xC9 => {
                self.pc = self.pop16(bus);
                1
            }
            0xCA => {
                let w = self.fetch16(bus);
                if self.f & FZ != 0 {
                    self.pc = w;
                }
                3
            }
            0xCB => 0, // handled by step()
            0xCC => {
                let w = self.fetch16(bus);
                if self.f & FZ != 0 {
                    self.push16(bus, self.pc);
                    self.pc = w;
                }
                3
            }
            0xCD => {
                let w = self.fetch16(bus);
                self.push16(bus, self.pc);
                self.pc = w;
                3
            }
            0xCE => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.add_imm(bus, v, true);
                2
            }
            0xCF => {
                self.push16(bus, self.pc);
                self.pc = 0x08;
                3
            }
            0xD0 => {
                if self.f & FC == 0 {
                    self.pc = self.pop16(bus);
                }
                1
            }
            0xD1 => {
                let v = self.pop16(bus);
                self.set_de(v);
                3
            }
            0xD2 => {
                let w = self.fetch16(bus);
                if self.f & FC == 0 {
                    self.pc = w;
                }
                3
            }
            0xD3 => 1, // undocumented on the DMG: NOP
            0xD4 => {
                let w = self.fetch16(bus);
                if self.f & FC == 0 {
                    self.push16(bus, self.pc);
                    self.pc = w;
                }
                3
            }
            0xD5 => {
                self.push16(bus, self.de());
                3
            }
            0xD6 => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.sub_imm(bus, v, false);
                2
            }
            0xD7 => {
                self.push16(bus, self.pc);
                self.pc = 0x10;
                3
            }
            0xD8 => {
                if self.f & FC != 0 {
                    self.pc = self.pop16(bus);
                }
                1
            }
            0xD9 => {
                self.pc = self.pop16(bus);
                self.ime = true;
                3
            }
            0xDA => {
                let w = self.fetch16(bus);
                if self.f & FC != 0 {
                    self.pc = w;
                }
                3
            }
            0xDB => 1, // NOP
            0xDC => {
                let w = self.fetch16(bus);
                if self.f & FC != 0 {
                    self.push16(bus, self.pc);
                    self.pc = w;
                }
                3
            }
            0xDD => 1, // NOP
            0xDE => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.sub_imm(bus, v, true);
                2
            }
            0xDF => {
                self.push16(bus, self.pc);
                self.pc = 0x18;
                3
            }
            0xE0 => {
                let n = bus.read(self.pc);
                self.pc += 1;
                bus.write(0xFF00 + n as u16, self.a);
                2
            }
            0xE1 => {
                let v = self.pop16(bus);
                self.set_hl(v);
                3
            }
            0xE2 => {
                bus.write(0xFF00 + self.c as u16, self.a);
                2
            }
            0xE3 => 1, // NOP
            0xE4 => 1, // NOP
            0xE5 => {
                self.push16(bus, self.hl());
                3
            }
            0xE6 => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.and_imm(bus, v);
                2
            }
            0xE7 => {
                self.push16(bus, self.pc);
                self.pc = 0x20;
                3
            }
            0xE8 => {
                // ADD SP,e8
                let n = bus.read(self.pc) as i8 as i16;
                self.pc += 1;
                let res = self.sp as i16 + n;
                self.set_z(false);
                self.set_n(false);
                self.set_h((((self.sp & 0x0F) as i16) + (n & 0x0F)) & 0x10 != 0);
                self.set_c(res > 0xFF);
                self.sp = res as u16;
                4
            }
            0xE9 => {
                self.pc = self.hl();
                1
            }
            0xEA => {
                let nn = self.fetch16(bus);
                bus.write(nn, self.a);
                4
            }
            0xEB => 1, // NOP
            0xEC => 1, // NOP
            0xED => 1, // NOP
            0xEE => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.xor_imm(bus, v);
                2
            }
            0xEF => {
                self.push16(bus, self.pc);
                self.pc = 0x28;
                3
            }
            0xF0 => {
                let n = bus.read(self.pc);
                self.pc += 1;
                self.a = bus.read(0xFF00 + n as u16);
                2
            }
            0xF1 => {
                let v = self.pop16(bus);
                self.a = (v >> 8) as u8;
                self.f = (v as u8) & 0xF0;
                3
            }
            0xF2 => {
                self.a = bus.read(self.c as u16);
                2
            }
            0xF3 => {
                self.ime = false;
                1
            }
            0xF4 => 1, // NOP
            0xF5 => {
                self.push16(bus, ((self.a as u16) << 8) | self.f as u16);
                3
            }
            0xF6 => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.or_imm(bus, v);
                2
            }
            0xF7 => {
                self.push16(bus, self.pc);
                self.pc = 0x30;
                3
            }
            0xF8 => {
                // LD HL,SP+e8
                let n = bus.read(self.pc) as i8 as i16;
                self.pc += 1;
                let res = self.sp as i16 + n;
                self.set_z(false);
                self.set_n(false);
                self.set_h((((self.sp & 0x0F) as i16) + (n & 0x0F)) & 0x10 != 0);
                self.set_c(res > 0xFF);
                self.set_hl(res as u16);
                4
            }
            0xF9 => {
                self.sp = self.hl();
                3
            }
            0xFA => {
                let nn = self.fetch16(bus);
                self.a = bus.read(nn);
                4
            }
            0xFB => {
                self.ime = true;
                1
            }
            0xFC => 1, // NOP
            0xFD => 1, // NOP
            0xFE => {
                let v = bus.read(self.pc);
                self.pc += 1;
                self.cp_imm(bus, v);
                2
            }
            0xFF => {
                self.push16(bus, self.pc);
                self.pc = 0x38;
                3
            }
            _ => 0,
        }
    }

    fn execute_cb(&mut self, op: u8, bus: &mut Bus) -> u32 {
        match op {
            0x00..=0x07 => {
                self.rlc_r(bus, op & 7);
                2
            }
            0x08..=0x0F => {
                self.rrc_r(bus, op & 7);
                2
            }
            0x10..=0x17 => {
                self.rl_r(bus, op & 7);
                2
            }
            0x18..=0x1F => {
                self.rr_r(bus, op & 7);
                2
            }
            0x20..=0x27 => {
                self.sla_r(bus, op & 7);
                2
            }
            0x28..=0x2F => {
                self.sra_r(bus, op & 7);
                2
            }
            0x30..=0x37 => {
                self.swap_r(bus, op & 7);
                2
            }
            0x38..=0x3F => {
                self.srl_r(bus, op & 7);
                2
            }
            0x40..=0x7F => {
                self.bit_r(bus, op);
                2
            }
            0x80..=0xBF => {
                self.set_r(bus, op);
                2
            }
            0xC0..=0xFF => {
                self.res_r(bus, op);
                2
            }
        }
    }
}
