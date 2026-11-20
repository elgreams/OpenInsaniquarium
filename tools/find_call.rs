// Quick one-off: find CALL targets in the wrapper at 0x00547270
// Usage: cargo run --example find_call (after placing in examples/)
// Or: rustc tools/find_call.rs -o /tmp/find_call && /tmp/find_call

use std::fs;

fn main() {
    let pe = fs::read("re/bin/Insaniquarium_game.exe").expect("read PE");

    // Parse PE sections
    let e = u32::from_le_bytes(pe[0x3C..0x40].try_into().unwrap()) as usize;
    let n = u16::from_le_bytes(pe[e + 6..e + 8].try_into().unwrap()) as usize;
    let opt = u16::from_le_bytes(pe[e + 20..e + 22].try_into().unwrap()) as usize;
    let base = u32::from_le_bytes(pe[e + 24 + 28..e + 24 + 32].try_into().unwrap()) as usize;

    let mut secs = Vec::new();
    for i in 0..n {
        let o = e + 24 + opt + 40 * i;
        let va = u32::from_le_bytes(pe[o + 12..o + 16].try_into().unwrap()) as usize;
        let raw_size = u32::from_le_bytes(pe[o + 16..o + 20].try_into().unwrap()) as usize;
        let raw_ptr = u32::from_le_bytes(pe[o + 20..o + 24].try_into().unwrap()) as usize;
        secs.push((base + va, raw_size, raw_ptr));
    }

    let va_to_off = |va: usize| -> Option<usize> {
        for &(start, size, ptr) in &secs {
            if va >= start && va < start + size {
                return Some(ptr + va - start);
            }
        }
        None
    };

    // Read 128 bytes from the wrapper at 0x00547270
    let func_va: usize = 0x00547270;
    let off = va_to_off(func_va).unwrap();
    let raw = &pe[off..off + 128];

    println!("Hex dump of 0x{:08x}:", func_va);
    for i in (0..raw.len()).step_by(16) {
        let end = (i + 16).min(raw.len());
        let hex: Vec<String> = raw[i..end].iter().map(|b| format!("{:02x}", b)).collect();
        println!("  {:08x}: {}", func_va + i, hex.join(" "));
    }

    // Find CALL (E8) instructions
    println!("\nCALL instructions:");
    for i in 0..raw.len() - 4 {
        if raw[i] == 0xE8 {
            let rel = i32::from_le_bytes(raw[i + 1..i + 5].try_into().unwrap());
            let target = (func_va + i + 5) as i64 + rel as i64;
            println!(
                "  {:08x}: CALL {:08x} (rel {:+})",
                func_va + i,
                target,
                rel
            );
        }
    }

    // Also dump the BuyBreeder at 0x005470a0 and level-start at 0x005471e0
    for &(name, addr) in &[
        ("BuyBreeder", 0x005470a0_usize),
        ("LevelStart", 0x005471e0_usize),
    ] {
        let off = va_to_off(addr).unwrap();
        let raw = &pe[off..off + 200];
        println!("\n{} at 0x{:08x}:", name, addr);
        for i in (0..raw.len()).step_by(16) {
            let end = (i + 16).min(raw.len());
            let hex: Vec<String> = raw[i..end].iter().map(|b| format!("{:02x}", b)).collect();
            println!("  {:08x}: {}", addr + i, hex.join(" "));
        }
        println!("CALL instructions:");
        for i in 0..raw.len() - 4 {
            if raw[i] == 0xE8 {
                let rel = i32::from_le_bytes(raw[i + 1..i + 5].try_into().unwrap());
                let target = (addr + i + 5) as i64 + rel as i64;
                println!(
                    "  {:08x}: CALL {:08x} (rel {:+})",
                    addr + i,
                    target,
                    rel
                );
            }
        }
    }
}
