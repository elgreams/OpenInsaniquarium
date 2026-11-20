/// Temporary test: dump PopulateLevel tank-4 section in detail
/// Run: cargo test -p harness --test pe_inspect -- --nocapture

fn parse_pe(pe: &[u8]) -> Vec<(usize, usize, usize)> {
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
    secs
}

fn va_to_off(secs: &[(usize, usize, usize)], va: usize) -> Option<usize> {
    for &(start, size, ptr) in secs {
        if va >= start && va < start + size {
            return Some(ptr + va - start);
        }
    }
    None
}

fn read_bytes(pe: &[u8], secs: &[(usize, usize, usize)], va: usize, len: usize) -> Vec<u8> {
    let off = va_to_off(secs, va).unwrap();
    pe[off..off + len].to_vec()
}

fn dump_hex(raw: &[u8], base_va: usize) {
    for i in (0..raw.len()).step_by(16) {
        let end = (i + 16).min(raw.len());
        let hex: Vec<String> = raw[i..end].iter().map(|b| format!("{:02x}", b)).collect();
        eprintln!("  {:08x}: {}", base_va + i, hex.join(" "));
    }
}

fn find_calls(raw: &[u8], base_va: usize) -> Vec<(usize, usize)> {
    let mut calls = Vec::new();
    for i in 0..raw.len().saturating_sub(4) {
        if raw[i] == 0xE8 {
            let rel = i32::from_le_bytes(raw[i + 1..i + 5].try_into().unwrap());
            let target = (base_va as i64 + i as i64 + 5) + rel as i64;
            if target > 0x400000 && target < 0x600000 {
                calls.push((base_va + i, target as usize));
            }
        }
    }
    calls
}

fn count_rng_calls(raw: &[u8], base_va: usize) -> Vec<usize> {
    let mut sites = Vec::new();
    for i in 0..raw.len().saturating_sub(4) {
        if raw[i] == 0xE8 {
            let rel = i32::from_le_bytes(raw[i + 1..i + 5].try_into().unwrap());
            let target = (base_va as i64 + i as i64 + 5) + rel as i64;
            if target == 0x0040aeb0 {
                sites.push(base_va + i);
            }
        }
    }
    sites
}

#[test]
fn dump_populate_tank4() {
    let pe_path = std::path::Path::new("../../re/bin/Insaniquarium_game.exe");
    if !pe_path.exists() {
        eprintln!("SKIP: PE not found");
        return;
    }
    let pe = std::fs::read(pe_path).unwrap();
    let secs = parse_pe(&pe);

    // Dump the full tank-4 section of PopulateLevel
    // From the previous run:
    //   AddPet calls at 0054994e and 00549981 (before breeder)
    //   RNG calls at 005499ae, 005499cd, 00549a02, 00549a21
    //   LevelStartBreeder at 00549a36
    //   AddPet calls at 00549a8b, 00549aa4
    //   RNG calls at 00549abf, 00549ade
    // Let me dump from 00549900 to 00549b20
    eprintln!("=== PopulateLevel tank-4 section (005498c0 to 00549b00) ===");
    let start = 0x005498c0_usize;
    let len = 0x240;
    let raw = read_bytes(&pe, &secs, start, len);
    dump_hex(&raw, start);

    eprintln!("\n=== All calls in this region ===");
    let calls = find_calls(&raw, start);
    for (site, target) in &calls {
        let name = match *target {
            0x0040aeb0 => "MTRand::NextNoAssert (RNG)",
            0x00544a90 => "AddPet",
            0x005471e0 => "LevelStartBreeder",
            0x00564345 => "operator new",
            0x00542ee0 => "AddGameObject",
            0x00544800 => "Shadow",
            0x0053aa40 => "RebuildZOrder",
            _ => "unknown",
        };
        eprintln!("  {:08x}: CALL {:08x} ({})", site, target, name);
    }

    // Also check what the Shadow ctor (004ec4f0) does regarding RNG
    eprintln!("\n=== Shadow ctor 004ec4f0 (256 bytes) ===");
    let raw = read_bytes(&pe, &secs, 0x004ec4f0, 256);
    let rng_sites = count_rng_calls(&raw, 0x004ec4f0);
    eprintln!("  Direct RNG calls: {} at {:?}", rng_sites.len(),
        rng_sites.iter().map(|v| format!("{:08x}", v)).collect::<Vec<_>>());
    let calls = find_calls(&raw, 0x004ec4f0);
    eprintln!("  CALLS:");
    for (site, target) in &calls {
        let rng = if *target == 0x0040aeb0 { " <-- RNG" } else { "" };
        eprintln!("    {:08x}: CALL {:08x}{}", site, target, rng);
    }

    // Check 00489a20 (called from Shadow create 00544370)
    eprintln!("\n=== 00489a20 (called from shadow create) ===");
    let raw = read_bytes(&pe, &secs, 0x00489a20, 128);
    dump_hex(&raw, 0x00489a20);
    let rng_sites = count_rng_calls(&raw, 0x00489a20);
    eprintln!("  Direct RNG calls: {}", rng_sites.len());

    // Check the OtherTypePet ctor at 004eb5c0 (for Stinky)
    eprintln!("\n=== OtherTypePet ctor 004eb5c0 (256 bytes) ===");
    let raw = read_bytes(&pe, &secs, 0x004eb5c0, 256);
    dump_hex(&raw, 0x004eb5c0);
    let rng_sites = count_rng_calls(&raw, 0x004eb5c0);
    eprintln!("  Direct RNG calls: {} at {:?}", rng_sites.len(),
        rng_sites.iter().map(|v| format!("{:08x}", v)).collect::<Vec<_>>());
    let calls = find_calls(&raw, 0x004eb5c0);
    eprintln!("  CALLS:");
    for (site, target) in &calls {
        let rng = if *target == 0x0040aeb0 { " <-- RNG" } else { "" };
        eprintln!("    {:08x}: CALL {:08x}{}", site, target, rng);
    }

    // Check the FishTypePet ctor at 004ef420 (for Itchy/Zorf)
    eprintln!("\n=== FishTypePet ctor 004ef420 (512 bytes) ===");
    let raw = read_bytes(&pe, &secs, 0x004ef420, 512);
    let rng_sites = count_rng_calls(&raw, 0x004ef420);
    eprintln!("  Direct RNG calls: {} at {:?}", rng_sites.len(),
        rng_sites.iter().map(|v| format!("{:08x}", v)).collect::<Vec<_>>());
    let calls = find_calls(&raw, 0x004ef420);
    eprintln!("  CALLS:");
    for (site, target) in &calls {
        let rng = if *target == 0x0040aeb0 { " <-- RNG" } else { "" };
        eprintln!("    {:08x}: CALL {:08x}{}", site, target, rng);
    }

    // Also need to find what function the OtherTypePet ctor calls for Init
    // (the Stinky Init function). Let me check what 004eb5c0 calls.

    // Check what happens between AddPet#2 and breeder position draws
    // Need to understand the flow for each pet
    eprintln!("\n=== Stinky Init (should be called from OtherTypePet ctor) ===");
    // From OtherTypePet ctor calls, find the Init call
    // I'll check 004eb5c0's sub-calls for RNG

    // Also need the Fish::Init which is the common Init for FishTypePet
    // FishTypePet → Fish::Init
    // Let me find Fish::Init from the FishTypePet ctor calls
}
