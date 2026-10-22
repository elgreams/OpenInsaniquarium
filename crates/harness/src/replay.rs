//! Rebuild sim state from a capture of the original and replay it.
//!
//! Captures from `tools/ce_capture.lua` carry each guppy's raw object bytes
//! (`"raw"`, hex) plus full RNG states in the header, so the sim can start
//! from the exact state of the original at the first captured tick.

use std::collections::BTreeMap;

use insaniquarium_sim::board::Input;
use insaniquarium_sim::shop::Slot;
use insaniquarium_sim::{Board, Coin, Fish, Food};
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
        facing: rd_f64(b, 0x188),
        patrol_dir: rd_i32(b, 0x178),
        floor_y: rd_i32(b, 0x190),
        min_x: rd_i32(b, 0x194),
        ceil_y: rd_i32(b, 0x198),
        max_x: rd_i32(b, 0x19c),
        hunger: rd_i32(b, 0x9c),
        stage: rd_i32(b, 0x1a0),
        growth: rd_i32(b, 0x1a4),
        growth_needed: rd_i32(b, 0x1a8),
        hungry_at: rd_i32(b, 0xa4),
        fade_to_hungry: b[0xa8] != 0,
        announces: b[0x228] != 0,
        state_ac: rd_i32(b, 0xac),
        link_11c: rd_i32(b, 0x11c),
        dead: false,
        mode: rd_i32(b, 0x1ac),
        mode_timer: rd_i32(b, 0x1b0),
        reroll_timer: rd_i32(b, 0x1b4),
        anim_frame: rd_i32(b, 0x1bc),
        swim_cycle: rd_i32(b, 0x1b8),
        cel: rd_i32(b, 0x1c0),
        turn_timer: rd_i32(b, 0x1c4),
        eat_timer: rd_i32(b, 0x1c8),
        pulse_timer: rd_i32(b, 0x1cc),
        timer_108: rd_i32(b, 0x108),
        timer_10c: rd_i32(b, 0x10c),
        timer_f8: rd_i32(b, 0xf8),
        fade: rd_i32(b, 0xa0),
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

/// A food object from its raw bytes (offsets: docs/re/objects/food.md).
pub fn food_from_raw(b: &[u8]) -> Food {
    Food {
        x: rd_f64(b, 0x158),
        y: rd_f64(b, 0x160),
        vx: rd_f64(b, 0x168),
        vy: rd_f64(b, 0x170),
        anim: rd_i32(b, 0x178),
        anim_speed: rd_i32(b, 0x17c),
        fade: rd_i32(b, 0x180),
        carried: b[0x185] != 0,
        level: rd_i32(b, 0x188),
        motion: rd_i32(b, 0x18c),
        kind: rd_i32(b, 0x194),
        delay: rd_i32(b, 0x198),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

/// A coin from its raw bytes (offsets: docs/re/objects/coin.md).
pub fn coin_from_raw(b: &[u8]) -> Coin {
    Coin {
        x: rd_f64(b, 0x158),
        y: rd_f64(b, 0x160),
        anim: rd_i32(b, 0x16c),
        age: rd_i32(b, 0x170),
        cel: rd_i32(b, 0x174),
        expire: rd_i32(b, 0x178),
        flight: rd_i32(b, 0x17c),
        from_x: rd_i32(b, 0x180),
        from_y: rd_i32(b, 0x184),
        vy: rd_f64(b, 0x188),
        kind: rd_i32(b, 0x194),
        collected: b[0x198] != 0,
        floor_time: rd_i32(b, 0x19c),
        multiplier: rd_i32(b, 0x1c0),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

fn raw_list(s: &Snapshot, list: &str) -> Vec<Vec<u8>> {
    s.state
        .get("raw")
        .and_then(|r| r.get(list))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().and_then(hex_bytes))
                .collect()
        })
        .unwrap_or_default()
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
    board.foods = raw_list(first, "food")
        .iter()
        .map(|b| food_from_raw(b))
        .collect();
    board.coins = raw_list(first, "coin")
        .iter()
        .map(|b| coin_from_raw(b))
        .collect();
    // Shop: captures record `board.revealed` (12 bytes) and `board.eggs`
    // when available; older ones are inferred from the largest guppy (the
    // tank 1-1 reveal triggers are stage-ups: medium → guppy, large → egg).
    let shop_raw = first
        .state
        .get("board")
        .and_then(|b| b.get("revealed"))
        .and_then(Value::as_str)
        .and_then(hex_bytes);
    if let Some(r) = shop_raw {
        for (i, &v) in r.iter().take(12).enumerate() {
            if v != 0 {
                board.shop.reveal(Slot::ALL[i], false);
            }
        }
    } else {
        let max_stage = board.guppies.iter().map(|g| g.stage).max().unwrap_or(0);
        if max_stage >= 1 {
            board.shop.reveal(Slot::Guppy, false);
        }
        if max_stage >= 2 {
            board.shop.reveal(Slot::EggPiece, false);
        }
    }
    if let Some(e) = first
        .state
        .get("board")
        .and_then(|b| b.get("eggs"))
        .and_then(Value::as_i64)
    {
        board.egg_count = e as i32;
    }
    if let Some(b) = first.state.get("board") {
        if let (Some(tank), Some(level)) = (
            b.get("tank").and_then(Value::as_i64),
            b.get("level").and_then(Value::as_i64),
        ) {
            board.tutorial = tank == 1 && level == 1;
        }
        if let Some(c) = b.get("clock").and_then(Value::as_i64) {
            board.clock = c as i32;
        }
        if let Some(raw) = b.get("samples").and_then(Value::as_str).and_then(hex_bytes) {
            for (slot, w) in board.sample_last.iter_mut().zip(raw.as_chunks::<4>().0) {
                *slot = i32::from_le_bytes(*w);
            }
        }
    }
    Ok(board)
}

/// Every modelled guppy field, so raw captures can be compared in full.
fn fish_json(f: &Fish) -> Value {
    json!({
        "x": f.x, "y": f.y, "vx": f.vx, "vy": f.vy, "div": f.divisor,
        "facing": f.facing, "patrol_dir": f.patrol_dir,
        "bounds": [f.min_x, f.ceil_y, f.max_x, f.floor_y],
        "hunger": f.hunger, "stage": f.stage,
        "growth": [f.growth, f.growth_needed], "fade_to_hungry": f.fade_to_hungry,
        "mode": f.mode, "mode_t": f.mode_timer, "reroll_t": f.reroll_timer,
        "anim": [f.anim_frame, f.swim_cycle, f.cel, f.turn_timer, f.eat_timer, f.pulse_timer],
        "timers": [f.timer_108, f.timer_10c, f.timer_f8, f.fade, f.vy_damp_timer,
                   f.push_timer, f.misc_timer],
        "push": f.push, "coin_t": f.coin_timer, "coin_iv": f.coin_interval,
        "widget": [f.widget_x, f.widget_y],
    })
}

fn coin_json(c: &Coin) -> Value {
    json!({
        "x": c.x, "y": c.y, "anim": [c.anim, c.age, c.cel], "expire": c.expire,
        "kind": c.kind, "collected": c.collected, "floor_time": c.floor_time,
        "widget": [c.widget_x, c.widget_y],
    })
}

fn food_json(f: &Food) -> Value {
    json!({
        "x": f.x, "y": f.y, "anim": [f.anim, f.anim_speed], "fade": f.fade,
        "level": f.level, "delay": f.delay, "widget": [f.widget_x, f.widget_y],
    })
}

/// Sim state as a snapshot.
pub fn snapshot(board: &Board) -> Snapshot {
    let mut state = BTreeMap::new();
    state.insert("board".into(), json!({ "money": board.money }));
    state.insert(
        "fish".into(),
        Value::Array(board.guppies.iter().map(fish_json).collect()),
    );
    state.insert(
        "foods".into(),
        Value::Array(board.foods.iter().map(food_json).collect()),
    );
    state.insert(
        "coins".into(),
        Value::Array(board.coins.iter().map(coin_json).collect()),
    );
    // The global stream's `mti` isn't compared: the board's cosmetic bubble
    // systems draw from it and aren't modelled yet (board.md, open questions).
    state.insert("rng_mti".into(), json!({ "app": board.rng.state().1 }));
    Snapshot {
        tick: board.tick,
        state,
    }
}

/// A capture re-expressed with each guppy decoded from its raw bytes into
/// the same `"fish"` shape [`snapshot`] emits.
pub fn decode_capture(t: &Trace) -> Trace {
    let snapshots = t
        .snapshots
        .iter()
        .map(|s| {
            let mut s = s.clone();
            let fish: Option<Vec<Value>> =
                s.state.get("guppies").and_then(Value::as_array).map(|gs| {
                    gs.iter()
                        .filter_map(|g| g.get("raw")?.as_str().and_then(hex_bytes))
                        .map(|raw| fish_json(&fish_from_raw(&raw)))
                        .collect()
                });
            if let Some(f) = fish {
                s.state.insert("fish".into(), Value::Array(f));
            }
            let foods = raw_list(&s, "food")
                .iter()
                .map(|b| food_json(&food_from_raw(b)))
                .collect();
            s.state.insert("foods".into(), Value::Array(foods));
            let coins = raw_list(&s, "coin")
                .iter()
                .map(|b| coin_json(&coin_from_raw(b)))
                .collect();
            s.state.insert("coins".into(), Value::Array(coins));
            // Raw hex is decoded above; drop it (and the capture's summary
            // fields) so long traces stay small in memory.
            s.state.remove("raw");
            s.state.remove("guppies");
            if let Some(Value::Object(b)) = s.state.get_mut("board") {
                b.remove("samples");
            }
            s
        })
        .collect();
    Trace {
        header: t.header.clone(),
        snapshots,
    }
}

/// `t` cut before the first snapshot where the board was paused. Captures
/// record `board.paused`; older ones don't, so a tick where the first
/// guppy's hunger doesn't move is taken as the pause.
pub fn until_pause(t: &Trace) -> Trace {
    let mut out = t.clone();
    // Skip a paused lead-in (captures may start while a dialog is up).
    let is_paused = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("paused"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    // Keep the last paused snapshot as the start: nothing (RNG included)
    // changes while paused, so it still matches the header's RNG state.
    let lead = t.snapshots.iter().take_while(|s| is_paused(s)).count();
    if lead > 1 {
        out.snapshots.drain(..lead - 1);
    }
    if let Some(first) = out.snapshots.first_mut()
        && let Some(Value::Object(b)) = first.state.get_mut("board")
    {
        b.insert("paused".into(), Value::Bool(false));
    }
    until_pause_inner(out)
}

fn until_pause_inner(t: Trace) -> Trace {
    let mut out = t.clone();
    let mut prev: Option<Option<Value>> = None;
    let cut = t.snapshots.iter().position(|s| {
        let paused = s
            .state
            .get("board")
            .and_then(|b| b.get("paused"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        // Hunger falls every running tick (no aliens on tank 1-1 captures).
        let h = s
            .state
            .get("guppies")
            .and_then(|g| g.get(0))
            .and_then(|g| g.get("hunger"))
            .cloned();
        let frozen = h.is_some() && prev.as_ref() == Some(&h);
        prev = Some(h);
        paused || frozen
    });
    if let Some(i) = cut {
        out.snapshots.truncate(i);
    }
    out
}

/// The snapshots with `from <= tick <= to`, keeping the header (whose RNG
/// state is only valid for the capture's first snapshot; see
/// [`rebase_rng`]).
pub fn window(t: &Trace, from: u32, to: u32) -> Trace {
    Trace {
        header: t.header.clone(),
        snapshots: t
            .snapshots
            .iter()
            .filter(|s| (from..=to).contains(&s.tick))
            .cloned()
            .collect(),
    }
}

/// Rewrite the header RNG so it's valid at the trace's first snapshot, by
/// replaying the original capture's RNG streams forward. Only the app
/// stream's draw count per tick is known (`rng_mti`), so this advances the
/// generator from the original header state until its `mti` and the number
/// of 624-word regenerations match. Works because the capture starts at the
/// header state and records `mti` every tick.
pub fn rebase_rng(full: &Trace, stream: &str, start_tick: u32) -> Option<RngState> {
    let mut rng = rng_from(full.header.rng.get(stream)?);
    let mut prev = rng.state().1;
    for s in &full.snapshots {
        if s.tick > start_tick {
            break;
        }
        let m = s.state.get("rng_mti")?.get(stream)?.as_u64()? as usize;
        let draws = (m + 624 - prev) % 624;
        for _ in 0..draws {
            rng.next_u31();
        }
        prev = m;
    }
    let (mt, mti) = rng.state();
    Some(RngState::Full {
        mt: mt.to_vec(),
        mti: mti as u32,
    })
}

/// Inputs recovered from a capture, plus snapshot ticks that caught a click
/// mid-frame (processed after that tick's update but before the capture
/// read it) and so match no whole-tick sim state.
#[derive(Debug, Default, Clone)]
pub struct Inferred {
    pub inputs: BTreeMap<u32, Vec<Input>>,
    pub dirty: Vec<u32>,
}

impl Inferred {
    /// `expected` without the dirty snapshots.
    pub fn clean(&self, expected: &Trace) -> Trace {
        Trace {
            header: expected.header.clone(),
            snapshots: expected
                .snapshots
                .iter()
                .filter(|s| !self.dirty.contains(&s.tick))
                .cloned()
                .collect(),
        }
    }
}

/// Player clicks recovered from a capture.
///
/// - Food: a new pellet at `(x, y)` came from a click at `(x + 10, y + 10)`
///   before it was first updated. If the snapshot shows it already updated
///   once (`anim == 1`, fallen 1.5) the click preceded that tick; if not
///   (`anim == 0`) the click came after that tick's update.
/// - Coins: a coin newly marked collected was clicked at its centre as of
///   collection (`from_x/from_y + 36`); if it hasn't glided yet, the click
///   came after that tick's update.
///
/// Clicks the original refused (food limit) leave no trace and cost no RNG.
pub fn inferred_clicks(t: &Trace) -> Inferred {
    let mut out = Inferred::default();
    let push = |tick: u32, mid_frame: bool, input: Input, out: &mut Inferred| {
        let at = if mid_frame {
            out.dirty.push(tick);
            tick + 1
        } else {
            tick
        };
        out.inputs.entry(at).or_default().push(input);
    };
    let money_of = |s: Option<&Snapshot>| {
        s.and_then(|s| s.state.get("board"))
            .and_then(|b| b.get("money"))
            .and_then(Value::as_i64)
    };
    let mut prev_snap: Option<&Snapshot> = None;
    let mut prev_prev_snap: Option<&Snapshot> = None;
    let mut prev_food = 0;
    let mut prev_coins: Vec<Coin> = Vec::new();
    for s in &t.snapshots {
        let foods: Vec<Food> = raw_list(s, "food")
            .iter()
            .map(|b| food_from_raw(b))
            .collect();
        // New pellets are appended; anything past the survivors is new.
        for f in foods.iter().skip(prev_food.min(foods.len())) {
            if f.fade != 0 || f.anim > 1 {
                continue;
            }
            let mid = f.anim == 0;
            let y = if mid { f.y } else { f.y - 1.5 };
            // The capture reads money before the lists, so a click landing
            // mid-read shows the price paid one snapshot before the pellet.
            if !mid && money_of(prev_snap) != money_of(prev_prev_snap) {
                out.dirty.push(s.tick - 1);
            }
            let click = Input::Click {
                x: f.x as i32 + 10,
                y: y as i32 + 10,
            };
            push(s.tick, mid, click, &mut out);
        }
        prev_food = foods.len();
        let coins: Vec<Coin> = raw_list(s, "coin")
            .iter()
            .map(|b| coin_from_raw(b))
            .collect();
        for c in coins.iter().filter(|c| c.collected) {
            let was_free = prev_coins
                .iter()
                .any(|p| !p.collected && p.kind == c.kind && p.age + 1 == c.age);
            if was_free {
                // Collection records trunc(x), trunc(y); if the coin still
                // sits there it hasn't glided yet, i.e. the click came after
                // this tick's update (mid-frame).
                let mid = c.x as i32 == c.from_x && c.y as i32 == c.from_y;
                let click = Input::Click {
                    x: c.from_x + 36,
                    y: c.from_y + 36,
                };
                push(s.tick, mid, click, &mut out);
            }
        }
        prev_coins = coins;
        // A guppy appearing at the surface (y = 30 before its first move)
        // was bought; money drops by the price on the same snapshot.
        let guppies = s
            .state
            .get("guppies")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let prev_guppies = prev_snap
            .and_then(|p| p.state.get("guppies"))
            .and_then(Value::as_array)
            .map_or(guppies, Vec::len);
        if guppies > prev_guppies && money_of(Some(s)) < money_of(prev_snap) {
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::Buy(Slot::Guppy));
        } else if guppies > prev_guppies && prev_guppies == 0 {
            // A free guppy into an empty tank: the "last fish died" dialog
            // was dismissed (mercy guppy).
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::DismissDialog);
        }
        // Egg pieces: the board's egg counter steps up.
        let eggs = |s: Option<&Snapshot>| {
            s.and_then(|s| s.state.get("board"))
                .and_then(|b| b.get("eggs"))
                .and_then(Value::as_i64)
        };
        if let (Some(e), Some(p)) = (eggs(Some(s)), eggs(prev_snap))
            && e > p
        {
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::Buy(Slot::EggPiece));
        }
        prev_prev_snap = prev_snap;
        prev_snap = Some(s);
    }
    out
}

/// Replay a capture: start from its first snapshot and step once per
/// captured tick, feeding `inputs[tick]` before that tick's update.
pub fn replay(t: &Trace, inputs: &BTreeMap<u32, Vec<Input>>) -> Result<Trace, ReplayError> {
    replay_from(board_from_capture(t)?, t, inputs)
}

/// [`replay`] from a prepared board (e.g. with level settings applied).
pub fn replay_from(
    mut board: Board,
    t: &Trace,
    inputs: &BTreeMap<u32, Vec<Input>>,
) -> Result<Trace, ReplayError> {
    let last = t.snapshots.last().ok_or(ReplayError::Empty)?.tick;
    let mut snapshots = vec![snapshot(&board)];
    let first = board.tick;
    while board.tick < last {
        assert!(
            board.tick - first <= last - first,
            "replay ran past the capture (tick not advancing?)"
        );
        let next = board.tick + 1;
        board.step(inputs.get(&next).map_or(&[][..], Vec::as_slice));
        snapshots.push(snapshot(&board));
    }
    let mut header = Header::new("sim", &t.header.build, t.header.start_tick);
    header.rng = t.header.rng.clone();
    Ok(Trace { header, snapshots })
}

/// Replay a capture with no input.
pub fn replay_idle(t: &Trace) -> Result<Trace, ReplayError> {
    replay(t, &BTreeMap::new())
}
