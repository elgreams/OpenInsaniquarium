//! Rebuild sim state from a capture of the original and replay it.
//!
//! Captures from `tools/ce_capture.lua` carry each guppy's raw object bytes
//! (`"raw"`, hex) plus full RNG states in the header, so the sim can start
//! from the exact state of the original at the first captured tick.

use std::collections::BTreeMap;

use insaniquarium_sim::{Board, Fish};
use serde_json::{Value, json};
use sexy_core::rand::MtRand;

use crate::trace::{Header, RngState, Snapshot, Trace};

fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

fn rd_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

fn rd_f64(b: &[u8], off: usize) -> f64 {
    f64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

/// A guppy from its raw object bytes (offsets: docs/re/objects/fish-guppy.md).
pub fn fish_from_raw(b: &[u8]) -> Fish {
    Fish {
        x: rd_f64(b, 0x158),
        y: rd_f64(b, 0x160),
        vx: rd_f64(b, 0x168),
        vy: rd_f64(b, 0x170),
        divisor: rd_f64(b, 0x180),
        facing_scale: rd_f64(b, 0x188),
        patrol_dir: rd_i32(b, 0x178),
        floor_y: rd_i32(b, 0x190),
        min_x: rd_i32(b, 0x194),
        ceil_y: rd_i32(b, 0x198),
        max_x: rd_i32(b, 0x19c),
        hunger: rd_i32(b, 0x9c),
        stage: rd_i32(b, 0x1a0),
        unk_1a8: rd_i32(b, 0x1a8),
        mode: rd_i32(b, 0x1ac),
        mode_timer: rd_i32(b, 0x1b0),
        reroll_timer: rd_i32(b, 0x1b4),
        anim_frame: rd_i32(b, 0x1bc),
        vy_damp_timer: rd_i32(b, 0x208),
        push_timer: rd_i32(b, 0x20c),
        push: rd_f64(b, 0x218),
        misc_timer: rd_i32(b, 0x210),
        coin_timer: rd_i32(b, 0x220),
        coin_interval: rd_i32(b, 0x224),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

fn rng_from(state: &RngState) -> MtRand {
    match state {
        RngState::Seed { seed } => MtRand::new(*seed),
        RngState::Full { mt, mti } => {
            let mt: [u32; 624] = mt.as_slice().try_into().expect("624 MT words");
            MtRand::from_state(mt, *mti as usize)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("trace has no snapshots")]
    Empty,
    #[error("missing {0}")]
    Missing(&'static str),
}

/// Board state equal to the original's at the trace's first snapshot.
pub fn board_from_capture(t: &Trace) -> Result<Board, ReplayError> {
    let first = t.snapshots.first().ok_or(ReplayError::Empty)?;
    let app = t
        .header
        .rng
        .get("app")
        .ok_or(ReplayError::Missing("app rng"))?;
    let global = t
        .header
        .rng
        .get("global")
        .ok_or(ReplayError::Missing("global rng"))?;
    let mut board = Board::new(rng_from(app), rng_from(global));
    board.tick = first.tick;
    if let Some(m) = first.state.get("board").and_then(|b| b.get("money")) {
        board.money = m.as_i64().ok_or(ReplayError::Missing("money"))? as i32;
    }
    let guppies = first
        .state
        .get("guppies")
        .and_then(Value::as_array)
        .ok_or(ReplayError::Missing("guppies"))?;
    for g in guppies {
        let raw = g
            .get("raw")
            .and_then(Value::as_str)
            .and_then(hex_bytes)
            .ok_or(ReplayError::Missing("guppy raw bytes"))?;
        board.guppies.push(fish_from_raw(&raw));
    }
    Ok(board)
}

/// The same fields `ce_capture.lua` records, from sim state.
pub fn snapshot(board: &Board) -> Snapshot {
    let guppies: Vec<Value> = board
        .guppies
        .iter()
        .map(|f| {
            json!({
                "x": f.x, "y": f.y, "vx": f.vx, "vy": f.vy, "div": f.divisor,
                "hunger": f.hunger, "mode": f.mode, "mode_t": f.mode_timer,
                "reroll_t": f.reroll_timer, "stage": f.stage,
                "coin_t": f.coin_timer, "coin_iv": f.coin_interval,
            })
        })
        .collect();
    let mut state = BTreeMap::new();
    state.insert("board".into(), json!({ "money": board.money }));
    state.insert("guppies".into(), Value::Array(guppies));
    state.insert(
        "rng_mti".into(),
        json!({ "app": board.rng.state().1, "global": board.global_rng.state().1 }),
    );
    Snapshot {
        tick: board.tick,
        state,
    }
}

/// Replay a capture with no input: start from its first snapshot and step
/// once per captured tick.
pub fn replay_idle(t: &Trace) -> Result<Trace, ReplayError> {
    let mut board = board_from_capture(t)?;
    let last = t.snapshots.last().ok_or(ReplayError::Empty)?.tick;
    let mut snapshots = vec![snapshot(&board)];
    while board.tick < last {
        board.update();
        snapshots.push(snapshot(&board));
    }
    let mut header = Header::new("sim", &t.header.build, t.header.start_tick);
    header.rng = t.header.rng.clone();
    Ok(Trace { header, snapshots })
}
