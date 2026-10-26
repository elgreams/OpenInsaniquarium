//! Does the game's own demo recording (`-record`, `framework/demo-recording.md`)
//! line up with a CE capture of the same session? Prints the demo's
//! left-button presses with their tick (the app update counter, `+0x484`)
//! next to what the capture shows changing at that tick.
use std::path::PathBuf;

use harness::demo::{Demo, DemoEvent};
use harness::trace::Trace;

#[test]
#[ignore = "exploratory: run with --nocapture"]
fn demo_lines_up_with_capture() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../traces");
    let Ok(data) = std::fs::read(dir.join("demos/tank2-1_pets_01.dmo")) else {
        return;
    };
    let demo = Demo::parse(&data).unwrap();
    println!(
        "demo: {} ticks, {} events",
        demo.total_ticks,
        demo.events.len()
    );
    let Some(t) = Trace::open(dir.join("original/demo_test_01.trace.jsonl")) else {
        return;
    };
    let t = t.unwrap();
    let first = t.snapshots.first().unwrap().tick;
    let last = t.snapshots.last().unwrap().tick;
    println!("capture: ticks {first}..={last}");
    for e in &demo.events {
        if let DemoEvent::MouseButton { down, x, y, .. } = e.event {
            let snap = t.snapshots.iter().find(|s| s.tick >= e.tick);
            let money = snap
                .and_then(|s| s.state.get("board"))
                .and_then(|b| b.get("money"))
                .and_then(|m| m.as_i64());
            println!(
                "tick {:>7} {} ({x},{y}) money then {:?}",
                e.tick,
                if down { "down" } else { "up  " },
                money
            );
        }
    }
}
