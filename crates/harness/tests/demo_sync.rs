//! Does the game's own demo recording (`-record`, `framework/demo-recording.md`)
//! line up with a CE capture of the same session? Prints the demo's
//! left-button presses with their tick next to the capture's snapshot at
//! that update count (`u`, `app+0x484`).
use std::path::PathBuf;

use harness::demo::{Demo, DemoEvent};
use harness::trace::Trace;

#[test]
#[ignore = "exploratory: run with --nocapture"]
fn demo_lines_up_with_capture() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../traces");
    let name = std::env::var("DEMO_SYNC").unwrap_or_else(|_| "sync_01".into());
    let Ok(data) = std::fs::read(dir.join(format!("demos/{name}.dmo"))) else {
        return;
    };
    let demo = Demo::parse(&data).unwrap();
    println!(
        "demo: {} ticks, {} events",
        demo.total_ticks,
        demo.events.len()
    );
    let Some(t) = Trace::open(dir.join(format!("original/{name}.trace.jsonl"))) else {
        return;
    };
    let t = t.unwrap();
    let u = |s: &harness::trace::Snapshot| s.state.get("u").and_then(|v| v.as_u64());
    let first = t.snapshots.first().unwrap();
    let last = t.snapshots.last().unwrap();
    println!(
        "capture: ticks {}..={}, u {:?}..={:?}",
        first.tick,
        last.tick,
        u(first),
        u(last)
    );
    for e in &demo.events {
        if let DemoEvent::MouseButton {
            down: true, x, y, ..
        } = e.event
        {
            let after = t
                .snapshots
                .iter()
                .find(|s| u(s).is_some_and(|u| u >= u64::from(e.tick)));
            let summary = after.map(|s| {
                let b = s.state.get("board");
                (
                    s.tick,
                    u(s),
                    b.and_then(|b| b.get("money")).cloned(),
                    s.state.get("counts").and_then(|c| c.get("food")).cloned(),
                )
            });
            println!(
                "tick {:>6} down ({x},{y}) -> first snapshot at/after: {:?}",
                e.tick, summary
            );
        }
    }
}

#[test]
#[ignore = "exploratory: run with --nocapture"]
fn demo_event_kinds() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../traces");
    let name = std::env::var("DEMO_SYNC").unwrap_or_else(|_| "sync_01".into());
    let Ok(data) = std::fs::read(dir.join(format!("demos/{name}.dmo"))) else {
        return;
    };
    let demo = Demo::parse(&data).unwrap();
    for e in &demo.events {
        match &e.event {
            DemoEvent::MouseMove { .. } => {}
            DemoEvent::FileRead(Some(b)) => println!("{} FileRead {} bytes", e.tick, b.len()),
            DemoEvent::RegistryRead(r) => println!("{} RegistryRead {}", e.tick, r.is_some()),
            ev => println!("{} {ev:?}", e.tick),
        }
    }
}
