//! Sim vs original on tank 1-1: idle swimming and feeding. Food clicks are
//! recovered from the capture itself.
//! Uses captures in `traces/original/` (gitignored); skips when absent.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use harness::replay::{decode_capture, inferred_food_clicks, replay};
use harness::trace::{Trace, diff};

fn captures() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../traces/original");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                (n.contains("_raw_") || n.contains("_feed_")) && n.ends_with(".trace.jsonl")
            })
        })
        .collect();
    v.sort();
    v
}

#[test]
fn tank1_1_matches_original() {
    let files = captures();
    if files.is_empty() {
        eprintln!("skipping: no raw captures in traces/original");
        return;
    }
    for path in files {
        let capture = Trace::read(BufReader::new(File::open(&path).unwrap())).unwrap();
        let expected = decode_capture(&capture);
        let actual = replay(&expected, &inferred_food_clicks(&capture)).unwrap();
        if let Some(d) = diff(&expected, &actual) {
            panic!("{}: {d}", path.display());
        }
    }
}

#[test]
fn feeding_replay_is_sensitive() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_feed_01.trace.jsonl");
    let Ok(f) = File::open(&path) else { return };
    let capture = Trace::read(BufReader::new(f)).unwrap();
    let expected = decode_capture(&capture);
    let clicks = inferred_food_clicks(&capture);
    assert_eq!(clicks.len(), 4, "{clicks:?}");
    let eaten: usize = expected
        .snapshots
        .iter()
        .map(|s| s.state["foods"].as_array().unwrap().len())
        .sum();
    assert!(eaten > 100, "food snapshots present");
    // Dropping one click must make the replay diverge.
    let mut fewer = clicks.clone();
    fewer.pop_first();
    assert!(diff(&expected, &replay(&expected, &fewer).unwrap()).is_some());
}
