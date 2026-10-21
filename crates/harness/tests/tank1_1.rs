//! Sim vs original on tank 1-1: idle swimming and feeding. Food clicks are
//! recovered from the capture itself.
//! Uses captures in `traces/original/` (gitignored); skips when absent.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use harness::replay::{
    board_from_capture, decode_capture, inferred_clicks, replay_from, until_pause,
};
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
                ["_raw_", "_feed_", "_coin_", "_buy_"]
                    .iter()
                    .any(|k| n.contains(k))
                    && n.ends_with(".trace.jsonl")
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
        let capture =
            until_pause(&Trace::read(BufReader::new(File::open(&path).unwrap())).unwrap());
        let inferred = inferred_clicks(&capture);
        let expected = inferred.clean(&decode_capture(&capture));
        // All captures here are tank 1-1, the tutorial level.
        let mut board = board_from_capture(&expected).unwrap();
        board.tutorial = true;
        let actual = replay_from(board, &expected, &inferred.inputs).unwrap();
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
    let clicks = inferred_clicks(&capture).inputs;
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
    let mut board = board_from_capture(&expected).unwrap();
    board.tutorial = true;
    assert!(diff(&expected, &replay_from(board, &expected, &fewer).unwrap()).is_some());
}

#[test]
fn coin_collection_capture_is_covered() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_coin_04.trace.jsonl");
    let Ok(f) = File::open(&path) else { return };
    let capture = until_pause(&Trace::read(BufReader::new(f)).unwrap());
    let inferred = inferred_clicks(&capture);
    let expected = inferred.clean(&decode_capture(&capture));
    let ticks = expected.snapshots.len();
    let money: Vec<i64> = expected
        .snapshots
        .iter()
        .map(|s| s.state["board"]["money"].as_i64().unwrap())
        .collect();
    let banked = money.windows(2).filter(|w| w[1] > w[0]).count();
    eprintln!(
        "{ticks} ticks compared, {} masked, {} clicks, {banked} coins banked",
        inferred.dirty.len(),
        inferred.inputs.values().map(Vec::len).sum::<usize>()
    );
    assert!(ticks > 5000);
    assert!(banked >= 10);
    assert!(inferred.dirty.len() < 20);
}
