//! Sim vs original: idle guppies on tank 1-1 (no input, no food).
//! Uses captures in `traces/original/` (gitignored); skips when absent.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use harness::replay::replay_idle;
use harness::trace::{Trace, diff};

fn captures() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../traces/original");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("_raw_") && n.ends_with(".trace.jsonl"))
        })
        .collect();
    v.sort();
    v
}

#[test]
fn idle_guppies_match_original() {
    let files = captures();
    if files.is_empty() {
        eprintln!("skipping: no raw captures in traces/original");
        return;
    }
    for path in files {
        let expected = Trace::read(BufReader::new(File::open(&path).unwrap())).unwrap();
        let actual = replay_idle(&expected).unwrap();
        if let Some(d) = diff(&expected, &actual) {
            panic!("{}: {d}", path.display());
        }
    }
}
