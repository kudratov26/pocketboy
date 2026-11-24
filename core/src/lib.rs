//! Pocketboy: a Game Boy (DMG) emulator — pure emulation core, no I/O.

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod gameboy;
pub mod joypad;
pub mod ppu;
pub mod serial;
pub mod timer;

#[cfg(test)]
mod trace_test;

pub use gameboy::GameBoy;
pub use joypad::{ButtonState, Joypad};

pub const FRAMEBUFFER_W: usize = 160;
pub const FRAMEBUFFER_H: usize = 144;
