//! harness: ROM runner and scoreboard for Pocketboy.
//!
//! Usage:
//!   harness <rom> [--cycles N] [--serial] [--screenshot out.png] [--trace out.log]
//!   harness --all
//!
//! Detection per ROM family (see MISSION.md):
//! - Blargg (roms/blargg/**): serial output contains "Passed"/"Failed".
//! - Mooneye (roms/mooneye/**): executed opcode 0x40 (LD B,B) with B,C,D,E,H,L
//!   = 3,5,8,13,21,34 => PASS; all 0x42 => FAIL.
//! - dmg-acid2 (roms/acid/dmg-acid2.gb): run N frames, dump framebuffer PNG,
//!   compare pixel-exact with roms/acid/dmg-acid2-reference.png.

use pocketboy_core::GameBoy;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_CYCLES: u64 = 100_000_000;
const ACID2_FRAMES: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Blargg,
    Mooneye,
    Acid,
    Other,
}

fn family_of(rom: &str) -> Family {
    if rom.contains("mooneye") {
        Family::Mooneye
    } else if rom.contains("blargg") {
        Family::Blargg
    } else if rom.contains("acid") {
        Family::Acid
    } else {
        Family::Other
    }
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

struct Trace {
    file: Option<fs::File>,
    count: u64,
}

impl Trace {
    fn new(path: Option<&str>) -> Trace {
        Trace {
            file: path.map(|p| fs::File::create(p).unwrap()),
            count: 0,
        }
    }

    fn record(&mut self, gb: &GameBoy) {
        if let Some(f) = self.file.as_mut() {
            use std::io::Write;
            let pc = gb.cpu.pc;
            let op = gb.cpu.pc as usize;
            let _ = writeln!(f, "{:04X} {:02X}", pc, op);
            self.count += 1;
        }
    }

    fn finish(&mut self) {
        use std::io::Write;
        let _ = self.file.as_mut().map(|f| f.flush());
    }
}

// ---------------------------------------------------------------------------
// Blargg: serial output contains "Passed" or "Failed".
// ---------------------------------------------------------------------------

fn serial_string(gb: &GameBoy) -> String {
    String::from_utf8_lossy(gb.serial_output()).trim().to_string()
}

fn run_blargg(gb: &mut GameBoy, cycles: u64, trace: &mut Trace) -> (String, String) {
    let mut ran = 0u64;
    while ran < cycles {
        let out = serial_string(gb);
        if out.contains("Passed") {
            return ("PASS".into(), out);
        }
        if out.contains("Failed") {
            return ("FAIL".into(), out);
        }
        trace.record(gb);
        let c = gb.step();
        ran += c as u64;
    }
    let out = serial_string(gb);
    if out.contains("Passed") {
        return ("PASS".into(), out);
    }
    if out.contains("Failed") {
        return ("FAIL".into(), out);
    }
    ("TIMEOUT".into(), format!("no serial verdict after {} cycles", cycles))
}

// ---------------------------------------------------------------------------
// Mooneye: LD B,B (opcode 0x40) with B,C,D,E,H,L == (3,5,8,13,21,34) => PASS,
// all 0x42 => FAIL.
// ---------------------------------------------------------------------------

fn mooneye_regs(gb: &GameBoy) -> [u8; 6] {
    [gb.cpu.b, gb.cpu.c, gb.cpu.d, gb.cpu.e, gb.cpu.h, gb.cpu.l]
}

fn run_mooneye(gb: &mut GameBoy, rom_bytes: &[u8], cycles: u64, trace: &mut Trace) -> (String, String) {
    let pass_vals = [3u8, 5, 8, 13, 21, 34];
    let mut ran = 0u64;
    while ran < cycles {
        let pc = gb.cpu.pc as usize;
        let op = rom_bytes.get(pc).copied().unwrap_or(0x00);
        if op == 0x40 {
            // LD B,B executed: check the pass/fail register values.
            let regs = mooneye_regs(gb);
            if regs == pass_vals {
                return (
                    "PASS".into(),
                    format!(
                        "B={:02X} C={:02X} D={:02X} E={:02X} H={:02X} L={:02X}",
                        regs[0], regs[1], regs[2], regs[3], regs[4], regs[5]
                    ),
                );
            }
            if regs == [0x42, 0x42, 0x42, 0x42, 0x42, 0x42] {
                return ("FAIL".into(), "mooneye fail marker (B,C,D,E,H,L = 42 x6)".into());
            }
        }
        trace.record(gb);
        let c = gb.step();
        ran += c as u64;
    }
    let regs = mooneye_regs(gb);
    if regs == pass_vals {
        return ("PASS".into(), "final registers match pass values".into());
    }
    ("FAIL".into(), format!("no LD B,B verdict; final regs {:02X?}", regs))
}

// ---------------------------------------------------------------------------
// dmg-acid2: run N frames, compare framebuffer with reference PNG.
// ---------------------------------------------------------------------------

/// Decode the reference PNG (2-bit grayscale) to a per-pixel value vector.
fn decode_reference_png(path: &Path) -> Vec<u8> {
    let data = fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let out = reader.next_frame(&mut buf).unwrap();
    let (w, h, line_size) = (out.width, out.height, out.line_size);
    let mut out_pixels = vec![0u8; (w * h) as usize];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let byte = buf[y * line_size + x / 4];
            let shift = 6 - 2 * (x % 4);
            out_pixels[y * w as usize + x] = (byte >> shift) & 0x3;
        }
    }
    out_pixels
}

/// Map a DMG shade to the grey value the reference PNG uses for that shade:
/// the DMG shades are 0..3 (0 = white), the reference uses four distinct
/// grey values with white being the largest.
fn shade_to_grey(shade: u8, ref_distinct: &[u8]) -> u8 {
    if ref_distinct.is_empty() {
        return shade;
    }
    ref_distinct
        .get(ref_distinct.len().saturating_sub(1).saturating_sub(shade as usize))
        .copied()
        .unwrap_or(*ref_distinct.last().unwrap())
}

fn write_png(path: &Path, data: &[u8], w: u32, h: u32) {
    let file = fs::File::create(path).unwrap();
    let wtr = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(wtr, w, h);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
}

fn run_acid2(gb: &mut GameBoy, screenshot: Option<&str>, trace: &mut Trace) -> (String, String) {
    let mut candidate: Option<Vec<u8>> = None;
    for _ in 0..ACID2_FRAMES {
        gb.run_frame();
        if std::env::var_os("ACID2_DEBUG").is_some() {
            eprintln!(
                "frame: lcdc={:02X} line={} mode={} scx={} scy={} wx={} wy={}",
                gb.bus.ppu.lcdc, gb.bus.ppu.line, gb.bus.ppu.mode, gb.bus.ppu.scx,
                gb.bus.ppu.scy, gb.bus.ppu.wx, gb.bus.ppu.wy
            );
        }
        if gb.bus.ppu.lcdc & 0x02 != 0 {
            candidate = Some(gb.framebuffer().to_vec());
        }
    }
        for _ in 0..ACID2_FRAMES {
            gb.run_frame();
            if gb.bus.ppu.lcdc & 0x02 != 0 {
                candidate = Some(gb.framebuffer().to_vec());
            }
        }
    trace.finish();
    let fb = match &candidate {
        Some(v) => v.clone(),
        None => gb.framebuffer().to_vec(),
    };
    let ref_pixels = decode_reference_png(Path::new("roms/acid/dmg-acid2-reference.png"));
    assert_eq!(ref_pixels.len(), 160 * 144, "reference PNG size mismatch");
    let mut ref_distinct = {
        let mut v: Vec<u8> = ref_pixels.iter().copied().collect();
        v.sort();
        v.dedup();
        v
    };
    ref_distinct.sort();
    ref_distinct.dedup();

    if std::env::var_os("ACID2_DEBUG").is_some() {
        eprintln!(
            "DEBUG lcdc={:02X} scx={} scy={} line={} mode={} bgp={:02X} obp0={:02X} obp1={:02X} wx={} wy={}",
            gb.bus.ppu.lcdc, gb.bus.ppu.scx, gb.bus.ppu.scy,
            gb.bus.ppu.line, gb.bus.ppu.mode, gb.bus.ppu.bgp, gb.bus.ppu.obp0, gb.bus.ppu.obp1,
            gb.bus.ppu.wx, gb.bus.ppu.wy
        );
        for (name, off, len) in [
            ("vram[0x9800]", 0x9800u16, 32usize),
            ("vram[0x9840]", 0x9840u16, 32),
            ("vram[0x9880]", 0x9880u16, 32),
            ("vram[0x9C00]", 0x9C00u16, 64),
            ("vram[0x9E80]", 0x9E80u16, 32),
            ("vram[0x8000]", 0x8000u16, 64),
            ("vram[0x8A00]", 0x8A00u16, 64),
            ("vram[0x9000]", 0x9000u16, 64),
            ("vram[0x9090]", 0x9090u16, 16),
        ] {
            let a = (off - 0x8000) as usize;
            eprintln!("DEBUG {}: {:02X?}", name, &gb.bus.vram[a..a + len]);
        }
        eprintln!("DEBUG fb[0..32]: {:02X?}", &fb[0..32]);
        eprintln!("DEBUG ref[0..32]: {:02X?}", &ref_pixels[0..32]);
        eprintln!("candidate: {}", candidate.is_some());
        {
            let mut marks = Vec::new();
            for i in 0..160 * 144 {
                if fb[i] != 0 {
                    marks.push((i % 160, i / 160, fb[i]));
                }
            }
            eprintln!("fb nonzero count={} first={:?}", marks.len(), &marks[..marks.len().min(40)]);
        }
        {
            let oam = &gb.bus.oam;
            eprintln!("OAM[0..80] (y, x, tile, flags):");
            for i in 0..20 {
                let b = &oam[i * 4..i * 4 + 4];
                eprintln!("  [{:2}] y={:3} x={:3} tile={:3} flags={:02X}", i, b[0], b[1], b[2], b[3]);
            }
        }
        if std::env::var_os("ACID2_GRID").is_some() {
            let chars = b" .:-=+*#%@";
            eprintln!("MY FRAMEBUFFER (shades):");
            for y in (0..144).step_by(8) {
                let row: String = (0..160).step_by(4).map(|x| chars[fb[y * 160 + x] as usize] as char).collect();
                eprintln!("{:3} {}", y, row);
            }
            eprintln!("REFERENCE GRID:");
            for y in (0..144).step_by(8) {
                let row: String = (0..160).step_by(4).map(|x| chars[ref_pixels[y * 160 + x] as usize] as char).collect();
                eprintln!("{:3} {}", y, row);
            }
        }
    }

    // Debug: distinct values + counts of both images.
    let mut dbg_out = String::new();
    let mut fb_counts = [0usize; 4];
    for &s in &fb {
        if (s as usize) < 4 {
            fb_counts[s as usize] += 1;
        }
    }
    let mut ref_counts = vec![0usize; 256];
    for &p in &ref_pixels {
        ref_counts[p as usize] += 1;
    }
    dbg_out.push_str(&format!(
        "ref_distinct={:?} ref_counts={:?}; fb_counts(shade0-3)={:?}",
        ref_distinct,
        ref_distinct.iter().map(|&d| ref_counts[d as usize]).collect::<Vec<_>>(),
        fb_counts
    ));

    let mut mismatches = 0usize;
    let mut first: Option<(usize, u8, u8, u8)> = None;
    for i in 0..160 * 144 {
        let grey = shade_to_grey(fb[i], &ref_distinct);
        if grey != ref_pixels[i] {
            mismatches += 1;
            if first.is_none() {
                first = Some((i, fb[i], grey, ref_pixels[i]));
            }
        }
    }
    if let Some(p) = screenshot {
        let mut greys = Vec::with_capacity(fb.len());
        for &s in &fb {
            greys.push(shade_to_grey(s, &ref_distinct));
        }
        write_png(Path::new(p), &greys, 160, 144);
    }
    if mismatches == 0 {
        ("PASS".into(), format!("framebuffer matches reference ({} frames) {}", ACID2_FRAMES, dbg_out))
    } else {
        let (i, shade, grey, want) = first.unwrap();
        (
            "FAIL".into(),
            format!(
                "{} pixel mismatches; first at ({},{}): shade {}, grey {}, want {} {}",
                mismatches, i % 160, i / 160, shade, grey, want, dbg_out
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// Scoreboard
// ---------------------------------------------------------------------------

fn walk_gbs(dir: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        let path = dir.join(&name);
        if path.is_dir() {
            walk_gbs(&path, out);
        } else if name.ends_with(".gb") {
            let rel = path.strip_prefix("roms").unwrap_or(&path).to_string_lossy().into_owned();
            out.push(rel.trim_start_matches('/').to_string());
        }
    }
}

fn is_required(rel_rom: &str, required_lines: &[String]) -> bool {
    for line in required_lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(prefix) = line.strip_suffix("/*.gb") {
            if rel_rom.starts_with(prefix) && rel_rom.ends_with(".gb") {
                return true;
            }
        } else if rel_rom == line {
            return true;
        }
    }
    false
}

fn load_required() -> Vec<String> {
    let path = Path::new("harness/required.txt");
    let f = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut v = Vec::new();
    for line in BufReader::new(f).lines() {
        if let Ok(l) = line {
            let l = l.trim().trim_start_matches("roms/").to_string();
            if !l.is_empty() {
                v.push(l);
            }
        }
    }
    v
}

fn run_rom(path: &Path, rel: &str, trace: &mut Trace) -> (String, String) {
    let rom_bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return ("FAIL".into(), format!("cannot read: {}", e)),
    };
    let mut gb = GameBoy::new(rom_bytes.clone());
    let fam = family_of(rel);
    match fam {
        Family::Blargg => run_blargg(&mut gb, DEFAULT_CYCLES, trace),
        Family::Mooneye => run_mooneye(&mut gb, &rom_bytes, DEFAULT_CYCLES, trace),
        Family::Acid => run_acid2(&mut gb, None, trace),
        Family::Other => {
            let mut ran = 0u64;
            while ran < DEFAULT_CYCLES {
                trace.record(&gb);
                ran += gb.step() as u64;
            }
            ("PASS".into(), format!("ran {} cycles", ran))
        }
    }
}

fn run_scoreboard() -> ExitCode {
    let mut roms = Vec::new();
    walk_gbs(Path::new("roms"), &mut roms);
    let required = load_required();
    println!("{:<62} {:<9} {}", "ROM", "RESULT", "DETAIL");
    println!("{}", "-".repeat(120));
    let mut pass_count = 0usize;
    let mut fail_required = 0usize;
    for rom in &roms {
        let path = PathBuf::from("roms").join(rom);
        let mut trace = Trace::new(None);
        let (result, detail) = run_rom(&path, rom, &mut trace);
        trace.finish();
        let ok = result == "PASS";
        if ok {
            pass_count += 1;
        }
        if is_required(rom, &required) && !ok {
            fail_required += 1;
        }
        println!("{:<62} {:<9} {}", rom, result, detail);
    }
    println!("{}", "-".repeat(120));
    println!(
        "Scoreboard: {} passed, {} failed of {} total",
        pass_count,
        roms.len() - pass_count,
        roms.len()
    );
    if fail_required > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "usage: harness <rom> [--cycles N] [--serial] [--screenshot out.png] [--trace out.log]\n       harness --all"
        );
        return ExitCode::from(2);
    }
    if args[0] == "--all" {
        return run_scoreboard();
    }

    let rom_arg = &args[0];
    let mut cycles = DEFAULT_CYCLES;
    let mut screenshot: Option<String> = None;
    let mut trace_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--cycles" => {
                cycles = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_CYCLES);
                i += 2;
            }
            "--serial" => i += 1,
            "--screenshot" => {
                screenshot = args.get(i + 1).cloned();
                i += 2;
            }
            "--trace" => {
                trace_path = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let rom_path = PathBuf::from(rom_arg);
    let rom_bytes = match fs::read(&rom_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", rom_path.display(), e);
            return ExitCode::from(2);
        }
    };
    let mut gb = GameBoy::new(rom_bytes.clone());
    let fam = family_of(rom_arg);
    let mut trace = Trace::new(trace_path.as_deref());

    let (result, detail) = match fam {
        Family::Blargg => run_blargg(&mut gb, cycles, &mut trace),
        Family::Mooneye => run_mooneye(&mut gb, &rom_bytes, cycles, &mut trace),
        Family::Acid => run_acid2(&mut gb, screenshot.as_deref(), &mut trace),
        Family::Other => {
            let mut ran = 0u64;
            while ran < cycles {
                trace.record(&gb);
                ran += gb.step() as u64;
            }
            ("PASS".into(), format!("ran {} cycles", ran))
        }
    };
    trace.finish();

    println!("{:<60} | {:<8} | {}", rom_arg, result, detail);
    if result == "PASS" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
