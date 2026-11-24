#[test]
fn trace_cpu() {
    let rom = std::fs::read("../roms/blargg/cpu_instrs/cpu_instrs.gb").unwrap();
    let mut gb = crate::GameBoy::new(rom);
    let mut printed = 0;
    for i in 0..3_000_000 {
        let pc = gb.cpu.pc;
        if (0x076B..=0x0790).contains(&pc) && printed < 60 {
            eprintln!(
                "i={:>7} pc={:04X} A={:02X} F={:02X} SP={:04X} serial_len={}",
                i, gb.cpu.pc, gb.cpu.a, gb.cpu.f, gb.cpu.sp, gb.bus.serial.output.len()
            );
            printed += 1;
        }
        gb.step();
    }
}
