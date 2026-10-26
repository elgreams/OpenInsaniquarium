//! Presentation against the original: music (per-track volume and fade,
//! the requested track/order) and dialogs, tick by tick, from captures
//! with a `pres` block (`tools/ce_capture.lua`). Skips when absent.

use std::path::PathBuf;

use harness::presentation::{
    capture_cues, capture_music, first_mismatch_in, music_from_capture, replay_cues, replay_music,
};
use harness::replay::{board_from_capture, by_board_clock, inferred_clicks, window};
use harness::trace::Trace;

fn capture(name: &str) -> Option<Trace> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original")
        .join(name);
    Some(Trace::open(p)?.unwrap())
}

/// Tank 1-2 from its first running tick to the last level-2 tick: two alien
/// waves (warning → fight → kill → tank song) and both hints.
#[test]
fn tank_1_2_music_and_dialogs_match() {
    let Some(full) = capture("tank1-2_pres_01.trace.jsonl") else {
        return;
    };
    let level2 = |s: &harness::trace::Snapshot| s.state["board"]["level"].as_i64() == Some(2);
    let i = full
        .snapshots
        .iter()
        .position(|s| level2(s) && s.state["board"]["paused"].as_bool() == Some(false))
        .unwrap();
    let from = full.snapshots[i].tick;
    let to = full.snapshots[i..]
        .iter()
        .take_while(|s| level2(s))
        .last()
        .unwrap()
        .tick;
    let raw = window(&full, from, to);
    let t = by_board_clock(&raw);
    let expected = capture_music(&t);
    // 1. The music model, driven by the cues the original raised.
    let cues = capture_cues(&t);
    assert_eq!(cues.len(), 6, "two waves: warning, fight, all clear");
    let actual = replay_cues(&t, &cues, 1);
    let music_only = expected
        .iter()
        .map(|(k, v)| (*k, serde_json::json!({ "music": v["music"] })))
        .collect();
    if let Some((tick, e, a)) = first_mismatch_in(&music_only, &actual) {
        panic!("music model, tick {tick}:\n  original {e}\n  ours     {a}");
    }
    // 2. The board raises the cues and hints itself, until the first
    //    alien fight (the replay drifts during fights without RNG
    //    re-syncs; the state replay tests cover those).
    let fight = *cues.keys().nth(1).unwrap();
    let inputs = inferred_clicks(&t).inputs;
    let board = board_from_capture(&t).unwrap();
    let music = music_from_capture(&t.snapshots[0].state["pres"]);
    let actual = replay_music(board, &inputs, fight, music);
    let until_fight = expected
        .range(..=fight)
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    eprintln!(
        "1-2 presentation: ticks {from}..={to}, {} records, board-driven to {fight}",
        expected.len()
    );
    if let Some((tick, e, a)) = first_mismatch_in(&until_fight, &actual) {
        panic!("board-driven, tick {tick}:\n  original {e}\n  ours     {a}");
    }
}
