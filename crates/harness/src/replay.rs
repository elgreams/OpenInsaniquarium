//! Rebuild sim state from a capture of the original and replay it.
//!
//! Captures from `tools/ce_capture.lua` carry each guppy's raw object bytes
//! (`"raw"`, hex) plus full RNG states in the header, so the sim can start
//! from the exact state of the original at the first captured tick.

use std::collections::BTreeMap;

use insaniquarium_sim::alien::{Alien, DeadAlien, Shot, Warp};
use insaniquarium_sim::board::Input;
use insaniquarium_sim::fish::{DeadFish, FishKind};
use insaniquarium_sim::message::Message;
use insaniquarium_sim::pet::Pet;
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
        kind: match rd_i32(b, 0x8c) {
            5 => FishKind::Carnivore,
            0x15 => FishKind::Itchy,
            _ => FishKind::Guppy,
        },
        eaten: false,
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

/// A `DeadFish` (`fish-guppy.md#dead-fish`) from its raw bytes.
pub fn dead_fish_from_raw(b: &[u8]) -> DeadFish {
    DeadFish {
        y: rd_f64(b, 0x160),
        x: rd_f64(b, 0x168),
        vx: rd_f64(b, 0x170),
        vy: rd_f64(b, 0x178),
        divisor: rd_f64(b, 0x180),
        kind: rd_i32(b, 0x1a4),
        facing_right: b[0x158] != 0,
        cel: rd_i32(b, 0x18c),
        turn: rd_i32(b, 0x190),
        alpha: rd_f64(b, 0x198),
        life: rd_i32(b, 0x1a0),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

/// A pet from its raw bytes (offsets: docs/re/objects/pet-stinky.md).
pub fn pet_from_raw(b: &[u8]) -> Pet {
    Pet {
        id: rd_i32(b, 0x1c4),
        x: rd_f64(b, 0x158),
        y: rd_f64(b, 0x160),
        vx: rd_f64(b, 0x168),
        vy: rd_f64(b, 0x170),
        facing: rd_f64(b, 0x178),
        target_vx: rd_f64(b, 0x180),
        divisor: rd_f64(b, 0x188),
        mode: rd_i32(b, 0x190),
        counter: rd_i32(b, 0x194),
        reroll_timer: rd_i32(b, 0x198),
        turn: rd_i32(b, 0x19c),
        anim: rd_i32(b, 0x1a0),
        cel: rd_i32(b, 0x1a4),
        counter_1a8: rd_i32(b, 0x1a8),
        countdown_1ac: rd_i32(b, 0x1ac),
        slow: b[0x1b0] != 0,
        shell: rd_i32(b, 0x1b4),
        pearl_taken: b[0x1c0] != 0,
        unk_1b8: rd_i32(b, 0x1b8),
        unk_1bc: rd_i32(b, 0x1bc),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
        shadow_y: 410,
    }
}

/// An alien from its raw bytes (offsets: docs/re/objects/alien-sylvester.md).
pub fn alien_from_raw(b: &[u8]) -> Alien {
    Alien {
        kind: rd_i32(b, 0x1ec),
        warp: rd_i32(b, 0x158),
        x: rd_f64(b, 0x160),
        y: rd_f64(b, 0x168),
        vx: rd_f64(b, 0x170),
        vy: rd_f64(b, 0x178),
        tvx: rd_f64(b, 0x180),
        tvy: rd_f64(b, 0x188),
        eat_delay: rd_i32(b, 0x1a0),
        anim: rd_i32(b, 0x1b0),
        turn: rd_i32(b, 0x1b4),
        cel: rd_i32(b, 0x1c0),
        divisor: rd_f64(b, 0x1b8),
        last_vx: rd_f64(b, 0x1c8),
        mode: rd_i32(b, 0x1d0),
        counter: rd_i32(b, 0x1d4),
        reroll: rd_i32(b, 0x1d8),
        health: rd_f64(b, 0x1f0),
        max_health: rd_f64(b, 0x1f8),
        cooldown: rd_i32(b, 0x200),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

/// A shot from its raw bytes.
pub fn shot_from_raw(b: &[u8]) -> Shot {
    Shot {
        kind: rd_i32(b, 0x160),
        delay: rd_i32(b, 0x164),
        age: rd_i32(b, 0x158),
        frame: rd_i32(b, 0x15c),
        r: rd_i32(b, 0x168),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

/// A dead alien from its raw bytes (lifetime at `+0x198`).
pub fn dead_alien_from_raw(b: &[u8]) -> DeadAlien {
    DeadAlien {
        life: rd_i32(b, 0x198),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
        kind: rd_i32(b, 0x1a0),
        facing_right: b[0x158] != 0,
        cel: rd_i32(b, 0x188),
        x: rd_f64(b, 0x168),
        y: rd_f64(b, 0x160),
        vx: rd_f64(b, 0x170),
        vy: rd_f64(b, 0x178),
        fade: rd_f64(b, 0x190),
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
    board.carnivores = raw_list(first, "oscar")
        .iter()
        .map(|b| fish_from_raw(b))
        .collect();
    board.pearls = raw_list(first, "coin_c")
        .iter()
        .map(|b| coin_from_raw(b))
        .collect();
    board.pets = raw_list(first, "pet_other")
        .iter()
        .map(|b| pet_from_raw(b))
        .collect();
    board.fish_pets = raw_list(first, "pet_fish")
        .iter()
        .map(|b| fish_from_raw(b))
        .collect();
    board.aliens = raw_list(first, "alien")
        .iter()
        .map(|b| alien_from_raw(b))
        .collect();
    board.has_aliens = !board.aliens.is_empty();
    board.shots = raw_list(first, "shot")
        .iter()
        .map(|b| shot_from_raw(b))
        .collect();
    board.dead_aliens = raw_list(first, "dead_alien")
        .iter()
        .map(|b| dead_alien_from_raw(b))
        .collect();
    board.dead_fish = raw_list(first, "dead_fish")
        .iter()
        .map(|b| dead_fish_from_raw(b))
        .collect();
    board.warps = raw_list(first, "warp")
        .iter()
        .map(|b| warp_from_raw(b))
        .collect();
    if let Some(w) = first.state.get("board").and_then(|b| b.get("wave")) {
        let g = |k: &str| w.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
        board.wave_kind = g("kind");
        board.wave_countdown = g("countdown");
        board.wave_2c4 = g("t2c4");
        board.food_lockout = g("lockout");
        board.wave_a = (g("ax"), g("ay"));
        board.wave_b = (g("bx"), g("by"));
        board.waves = g("waves");
    }
    if let Some(b) = first.state.get("board") {
        let g = |k: &str| b.get(k).and_then(Value::as_i64).map(|v| v as i32);
        if let (Some(x), Some(y)) = (g("shot_x"), g("shot_y")) {
            board.last_shot = (x, y);
        }
        if let Some(v) = g("laser") {
            board.laser = v;
        }
        if let Some(v) = g("food_level") {
            board.food_level = v;
        }
        if let Some(v) = g("max_food") {
            board.max_food = v;
        }
    }
    // Shop: captures record `board.revealed` (12 bytes) and `board.eggs`
    // when available; older ones are inferred from the largest guppy (the
    // tank 1-1 reveal triggers are stage-ups: medium → guppy, large → egg).
    let shop_raw = first
        .state
        .get("board")
        .and_then(|b| b.get("revealed"))
        .and_then(Value::as_str)
        .and_then(hex_bytes);
    // The shop layout and prices come from the level.
    let level_of = |k: &str| {
        first
            .state
            .get("board")
            .and_then(|b| b.get(k))
            .and_then(Value::as_i64)
    };
    if let (Some(t), Some(l)) = (level_of("tank"), level_of("level")) {
        board.shop = insaniquarium_sim::shop::Shop::adventure(t as i32, l as i32);
    }
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
            board.tank = tank as i32;
            board.level = level as i32;
        }
        // One-time hint bytes (`+0x4b0..`, board.md#message-line). Older
        // captures don't record them; those start mid-level, where the
        // one-time messages were most likely shown already, so all count
        // as shown (only the "REALLY hungry" one has a side effect, a sound).
        match b.get("hints").and_then(Value::as_str).and_then(hex_bytes) {
            Some(h) => {
                for (flag, &v) in board.hints.iter_mut().zip(&h) {
                    *flag = v != 0;
                }
            }
            None => {
                use insaniquarium_sim::message::{
                    HINT_BATTLE_TIP, HINT_DANGER, HINT_NEW_BREED, HINTS,
                };
                board.hints = [true; HINTS];
                // The hint dialogs pause the board, so the replay must
                // raise them: they start unshown, as before.
                for i in [HINT_DANGER, HINT_BATTLE_TIP, HINT_NEW_BREED] {
                    board.hints[i] = false;
                }
            }
        }
        if let Some(c) = b.get("clock").and_then(Value::as_i64) {
            board.clock = c as i32;
        }
        if let Some(v) = b.get("ticks").and_then(Value::as_i64) {
            board.level_ticks = v as i32;
        }
        if let Some(v) = b.get("flash").and_then(Value::as_i64) {
            board.money_flash = v as i32;
        }
        if let Some(m) = b.get("msg") {
            let g = |k: &str| m.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
            board.message.timer = g("t");
            board.message.flash = g("flash") != 0;
            board.message.id = g("hint");
            board.message.text = m.get("text").and_then(Value::as_str).unwrap_or("").into();
            // Captures don't record the colour; the warning texts are the
            // only red ones (board.md#the-alien-warning-line).
            board.message.warning = board.message.text.contains("APPROACHING")
                || board.message.text.contains("DETECTED");
        }
        if let Some(l) = b.get("lights").and_then(Value::as_array) {
            for (x, v) in board.light_x.iter_mut().zip(l) {
                *x = v.as_f64().unwrap_or(0.0) as f32;
            }
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

fn pet_json(p: &Pet) -> Value {
    json!({
        "id": p.id, "x": p.x, "y": p.y, "vx": p.vx, "vy": p.vy, "facing": p.facing,
        "tvx": p.target_vx, "mode": p.mode,
        "timers": [p.counter, p.reroll_timer, p.counter_1a8, p.countdown_1ac, p.shell],
        "pearl_taken": p.pearl_taken,
        "anim": [p.turn, p.anim, p.cel], "widget": [p.widget_x, p.widget_y],
    })
}

fn alien_json(a: &Alien) -> Value {
    json!({
        "kind": a.kind, "warp": a.warp, "x": a.x, "y": a.y, "vx": a.vx, "vy": a.vy,
        "tv": [a.tvx, a.tvy], "eat": a.eat_delay, "anim": [a.anim, a.turn, a.cel],
        "last_vx": a.last_vx, "mode": [a.mode, a.counter, a.reroll],
        "hp": a.health, "cool": a.cooldown, "widget": [a.widget_x, a.widget_y],
    })
}

fn dead_alien_json(d: &DeadAlien) -> Value {
    json!({
        "life": d.life, "x": d.x, "y": d.y, "v": [d.vx, d.vy], "fade": d.fade,
        "cel": d.cel, "mirror": d.facing_right, "widget": [d.widget_x, d.widget_y],
    })
}

fn warp_json(w: &Warp) -> Value {
    json!({ "counter": w.counter, "widget": [w.widget_x, w.widget_y] })
}

/// A warp hole from its raw bytes (counter at `+0x158`).
pub fn warp_from_raw(b: &[u8]) -> Warp {
    Warp {
        counter: rd_i32(b, 0x158),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

fn shot_json(s: &Shot) -> Value {
    json!({ "kind": s.kind, "widget": [s.widget_x, s.widget_y] })
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

/// The message line as compared: what's on screen (text while the timer
/// runs), the timer and the blink flag.
fn message_json(m: &Message) -> Value {
    json!({ "text": m.visible().then_some(m.text.as_str()), "t": m.timer, "flash": m.flash })
}

fn dead_fish_json(d: &DeadFish) -> Value {
    json!({ "x": d.x, "y": d.y, "vx": d.vx, "vy": d.vy, "kind": d.kind, "cel": d.cel,
            "alpha": d.alpha, "life": d.life, "widget": [d.widget_x, d.widget_y] })
}

/// The snapshot carries raw object lists (so an absent list means empty).
fn raw_present(s: &Snapshot) -> bool {
    s.state.get("raw").is_some_and(Value::is_object)
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
    state.insert(
        "carnivores".into(),
        Value::Array(board.carnivores.iter().map(fish_json).collect()),
    );
    state.insert(
        "pearls".into(),
        Value::Array(board.pearls.iter().map(coin_json).collect()),
    );
    state.insert(
        "pets".into(),
        Value::Array(board.pets.iter().map(pet_json).collect()),
    );
    state.insert(
        "fish_pets".into(),
        Value::Array(board.fish_pets.iter().map(fish_json).collect()),
    );
    state.insert(
        "aliens".into(),
        Value::Array(board.aliens.iter().map(alien_json).collect()),
    );
    state.insert(
        "shots".into(),
        Value::Array(board.shots.iter().map(shot_json).collect()),
    );
    state.insert(
        "dead_aliens".into(),
        Value::Array(board.dead_aliens.iter().map(dead_alien_json).collect()),
    );
    state.insert(
        "warps".into(),
        Value::Array(board.warps.iter().map(warp_json).collect()),
    );
    state.insert(
        "waves".into(),
        json!({ "countdown": board.wave_countdown, "lockout": board.food_lockout,
                "a": [board.wave_a.0, board.wave_a.1], "n": board.waves }),
    );
    state.insert(
        "dead_fish".into(),
        Value::Array(board.dead_fish.iter().map(dead_fish_json).collect()),
    );
    // The message line (board.md#message-line). Only compared when the
    // capture records it (`board.msg`).
    state.insert("message".into(), message_json(&board.message));
    // The global stream's `mti` isn't compared: the board's cosmetic bubble
    // systems draw from it and aren't modelled yet (board.md, open questions).
    state.insert("rng_mti".into(), json!({ "app": board.rng.state().1 }));
    // Throttled samples started this tick (compared separately, see
    // `presentation::capture_sounds`).
    state.insert(
        "sounds".into(),
        json!(crate::presentation::sim_sounds(board)),
    );

    Snapshot {
        tick: board.tick,
        state,
    }
}

/// A capture re-expressed with each guppy decoded from its raw bytes into
/// the same `"fish"` shape [`snapshot`] emits.
pub fn decode_capture(t: &Trace) -> Trace {
    let sounds = crate::presentation::capture_sounds(t);
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
            if raw_present(&s) {
                let carnivores = raw_list(&s, "oscar")
                    .iter()
                    .map(|b| fish_json(&fish_from_raw(b)))
                    .collect();
                s.state
                    .insert("carnivores".into(), Value::Array(carnivores));
                let pearls = raw_list(&s, "coin_c")
                    .iter()
                    .map(|b| coin_json(&coin_from_raw(b)))
                    .collect();
                s.state.insert("pearls".into(), Value::Array(pearls));
                let fish_pets = raw_list(&s, "pet_fish")
                    .iter()
                    .map(|b| fish_json(&fish_from_raw(b)))
                    .collect();
                s.state.insert("fish_pets".into(), Value::Array(fish_pets));
            }
            let pets = raw_list(&s, "pet_other")
                .iter()
                .map(|b| pet_json(&pet_from_raw(b)))
                .collect();
            s.state.insert("pets".into(), Value::Array(pets));
            let aliens = raw_list(&s, "alien")
                .iter()
                .map(|b| alien_json(&alien_from_raw(b)))
                .collect();
            s.state.insert("aliens".into(), Value::Array(aliens));
            let shots = raw_list(&s, "shot")
                .iter()
                .map(|b| shot_json(&shot_from_raw(b)))
                .collect();
            s.state.insert("shots".into(), Value::Array(shots));
            let dead = raw_list(&s, "dead_alien")
                .iter()
                .map(|b| dead_alien_json(&dead_alien_from_raw(b)))
                .collect();
            s.state.insert("dead_aliens".into(), Value::Array(dead));
            let warps = raw_list(&s, "warp")
                .iter()
                .map(|b| warp_json(&warp_from_raw(b)))
                .collect();
            s.state.insert("warps".into(), Value::Array(warps));
            if raw_present(&s) {
                let dead = raw_list(&s, "dead_fish")
                    .iter()
                    .map(|b| dead_fish_json(&dead_fish_from_raw(b)))
                    .collect();
                s.state.insert("dead_fish".into(), Value::Array(dead));
            }
            if let Some(m) = s.state.get("board").and_then(|b| b.get("msg")) {
                let g = |k: &str| m.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
                let msg = Message {
                    text: m.get("text").and_then(Value::as_str).unwrap_or("").into(),
                    timer: g("t"),
                    flash: g("flash") != 0,
                    id: g("hint"),
                    warning: false,
                };
                s.state.insert("message".into(), message_json(&msg));
            }
            if let Some(w) = s.state.get("board").and_then(|b| b.get("wave")).cloned() {
                let g = |k: &str| w.get(k).cloned().unwrap_or(Value::Null);
                s.state.insert(
                    "waves".into(),
                    json!({ "countdown": g("countdown"), "lockout": g("lockout"),
                            "a": [g("ax"), g("ay")], "n": g("waves") }),
                );
            }
            // Samples started this tick, from the sample table.
            if let Some(v) = sounds.get(&s.tick) {
                s.state.insert("sounds".into(), json!(v));
            }
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

/// The first snapshot tick (at or after `from`) where the capture missed or
/// tore logic updates: the board clock (`+0x28`, +1 per running update)
/// advanced by a different amount than the capture's tick. This happens in
/// the game's catch-up bursts, when several updates run back to back faster
/// than the capture polls (`docs/re/dynamic-analysis.md`).
pub fn first_burst(t: &Trace, from: u32) -> Option<u32> {
    let clock = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("clock"))
            .and_then(Value::as_i64)
    };
    let paused = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("paused"))
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    t.snapshots
        .windows(2)
        .filter(|w| w[0].tick >= from && !paused(&w[0]) && !paused(&w[1]))
        .find(|w| {
            clock(&w[1])
                .zip(clock(&w[0]))
                .is_some_and(|(b, a)| b - a != i64::from(w[1].tick - w[0].tick))
        })
        .map(|w| w[0].tick)
}

/// Snapshots read mid-update ("torn"): the board clock advanced by one but
/// every guppy is exactly where it was while a later-updating object (a pet)
/// moved. The capture reads guppies before pets, so a read that lands inside
/// Board::Update shows pre-update guppies with post-update pets. Such
/// snapshots match no whole-tick state; mask them.
pub fn torn_snapshots(t: &Trace) -> Vec<u32> {
    let fish_x = |s: &Snapshot| -> Vec<u64> {
        s.state
            .get("guppies")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|g| g.get("raw")?.as_str().and_then(hex_bytes))
                    .map(|b| rd_f64(&b, 0x158).to_bits())
                    .collect()
            })
            .unwrap_or_default()
    };
    let pet_x = |s: &Snapshot| {
        raw_list(s, "pet_other")
            .first()
            .map(|b| rd_f64(b, 0x158).to_bits())
    };
    let clock = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("clock"))
            .and_then(Value::as_i64)
    };
    t.snapshots
        .windows(2)
        .filter(|w| {
            let (a, b) = (&w[0], &w[1]);
            let fa = fish_x(a);
            clock(b).zip(clock(a)).is_some_and(|(cb, ca)| cb - ca == 1)
                && !fa.is_empty()
                && fa == fish_x(b)
                && pet_x(a).is_some()
                && pet_x(a) != pet_x(b)
        })
        .map(|w| w[1].tick)
        .collect()
}

/// Re-key a window by the board clock (`+0x28`): snapshot tick =
/// `start_tick + (clock − first clock)`. Drops snapshots that repeat a clock
/// value (no update happened between reads) and leaves gaps where the
/// capture missed updates; the diff skips missing ticks. Use for running
/// windows only (the clock stops while paused).
pub fn by_board_clock(t: &Trace) -> Trace {
    let clock = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("clock"))
            .and_then(Value::as_i64)
    };
    let Some(base) = t.snapshots.first().and_then(clock) else {
        return t.clone();
    };
    let tick0 = t.snapshots[0].tick;
    let mut out = Trace {
        header: t.header.clone(),
        snapshots: Vec::new(),
    };
    for s in &t.snapshots {
        let Some(c) = clock(s) else { continue };
        let tick = tick0 + (c - base) as u32;
        // A repeated clock value: a later read of the same board update. Keep
        // the latest (an earlier one may have been torn mid-update).
        match out.snapshots.last_mut() {
            Some(prev) if prev.tick == tick => {
                *prev = s.clone();
                prev.tick = tick;
                continue;
            }
            Some(prev) if prev.tick > tick => continue,
            _ => {}
        }
        let mut s = s.clone();
        s.tick = tick;
        out.snapshots.push(s);
    }
    out
}

/// Ticks right after a gap in a [`by_board_clock`] trace: the capture
/// missed updates there (a catch-up burst), so its read may be torn.
pub fn after_gaps(t: &Trace) -> Vec<u32> {
    t.snapshots
        .windows(2)
        .filter(|w| w[1].tick > w[0].tick + 1)
        .map(|w| w[1].tick)
        .collect()
}

/// The original capture tick whose board clock equals `clock_tick`'s clock
/// in a trace re-keyed by [`by_board_clock`] (`raw` is the un-rekeyed window).
pub fn original_tick(raw: &Trace, clock_tick: u32) -> Option<u32> {
    let clock = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("clock"))
            .and_then(Value::as_i64)
    };
    let c0 = clock(raw.snapshots.first()?)?;
    let want = c0 + i64::from(clock_tick - raw.snapshots[0].tick);
    raw.snapshots
        .iter()
        .find(|s| clock(s) == Some(want))
        .map(|s| s.tick)
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
        // A dirty tick's click is replayed on the next tick, so sounds the
        // original started then (a mid-frame click's sound) move with it.
        // Snapshots after a capture gap hold the gap's sounds too: the
        // tick before them is treated the same way.
        let mut carry: Vec<Value> = Vec::new();
        let mut snapshots = Vec::new();
        let mut prev_tick: Option<u32> = None;
        for s in &expected.snapshots {
            let gap = prev_tick.is_some_and(|p| s.tick > p + 1);
            prev_tick = Some(s.tick);
            if gap
                && let Some(Value::Array(v)) = s.state.get("sounds")
                && !v.is_empty()
            {
                // The sim's record for this tick only has its own sounds:
                // compare the union of the gap instead (`sounds_gap`).
                let mut s = s.clone();
                if let Some(Value::Object(_)) = s.state.get("board") {
                    let v = s.state.remove("sounds").unwrap_or(Value::Null);
                    s.state.insert("sounds_gap".into(), v);
                }
                carry.clear();
                snapshots.push(s);
                continue;
            }
            if self.dirty.contains(&s.tick) {
                if let Some(Value::Array(v)) = s.state.get("sounds") {
                    carry.extend(v.iter().cloned());
                }
                continue;
            }
            let mut s = s.clone();
            if !carry.is_empty()
                && let Some(Value::Array(v)) = s.state.get_mut("sounds")
            {
                v.append(&mut carry);
                v.sort_by_key(|x| x.as_i64());
                v.dedup();
            }
            carry.clear();
            snapshots.push(s);
        }
        Trace {
            header: expected.header.clone(),
            snapshots,
        }
    }
}

impl Inferred {
    /// The sim side of [`Inferred::clean`]'s sound carry: sounds the sim
    /// played on a dirty tick (which isn't compared) count on the next one,
    /// as the capture's do. Without this, a sound the sim starts on a dirty
    /// tick for an unrelated reason (a guppy eating) goes missing.
    pub fn carry_dirty_sounds(&self, actual: &mut Trace) {
        let mut carry: Vec<Value> = Vec::new();
        for s in &mut actual.snapshots {
            if self.dirty.contains(&s.tick) {
                if let Some(Value::Array(v)) = s.state.get("sounds") {
                    carry.extend(v.iter().cloned());
                }
                continue;
            }
            if !carry.is_empty()
                && let Some(Value::Array(v)) = s.state.get_mut("sounds")
            {
                v.append(&mut carry);
                v.sort_by_key(|x| x.as_i64());
                v.dedup();
            }
            carry.clear();
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
    let mut prev_pearls: Vec<Coin> = Vec::new();
    let mut coins_collected_at: Vec<u32> = Vec::new();
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
            // The same coin, still free a tick ago: same kind and age, and
            // where it was collected (coins dropped together share kind and
            // age, so the position tells them apart).
            let was_free = prev_coins.iter().any(|p| {
                !p.collected
                    && p.kind == c.kind
                    && p.age + 1 == c.age
                    && (p.widget_x - c.from_x).abs() <= 2
                    && (p.widget_y - c.from_y).abs() <= 2
            });
            if was_free {
                // Collection records trunc(x), trunc(y); if the coin still
                // sits there it hasn't glided yet, i.e. the click came after
                // this tick's update (mid-frame).
                let mid = c.x as i32 == c.from_x && c.y as i32 == c.from_y;
                let click = Input::Click {
                    x: c.from_x + 36,
                    y: c.from_y + 36,
                };
                coins_collected_at.push(if mid { s.tick + 1 } else { s.tick });
                push(s.tick, mid, click, &mut out);
            }
        }
        prev_coins = coins;
        // Pearls (`coin_c`): same inference as coins.
        let pearls: Vec<Coin> = raw_list(s, "coin_c")
            .iter()
            .map(|b| coin_from_raw(b))
            .collect();
        for c in pearls.iter().filter(|c| c.collected) {
            // The same coin, still free a tick ago: same kind and age, and
            // where it was collected (coins dropped together share kind and
            // age, so the position tells them apart).
            let was_free = prev_pearls.iter().any(|p| {
                !p.collected
                    && p.kind == c.kind
                    && p.age + 1 == c.age
                    && (p.widget_x - c.from_x).abs() <= 2
                    && (p.widget_y - c.from_y).abs() <= 2
            });
            if was_free {
                let mid = c.x as i32 == c.from_x && c.y as i32 == c.from_y;
                let click = Input::Click {
                    x: c.from_x + 36,
                    y: c.from_y + 36,
                };
                push(s.tick, mid, click, &mut out);
            }
        }
        prev_pearls = pearls;
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
            // Still at its spawn y (30.0): bought after this tick's update
            // (mid-frame), so it applies before the next tick.
            let unmoved = s
                .state
                .get("guppies")
                .and_then(Value::as_array)
                .and_then(|g| g.last())
                .and_then(|g| g.get("raw")?.as_str().and_then(hex_bytes))
                .is_some_and(|raw| fish_from_raw(&raw).y == 30.0);
            push(s.tick, unmoved, Input::Buy(Slot::Guppy), &mut out);
        } else if guppies > prev_guppies && prev_guppies == 0 {
            // A free guppy into an empty tank: the "last fish died" dialog
            // was dismissed (mercy guppy).
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::DismissDialog);
        }
        // Shots while aliens are present: each click leaves a click-flash
        // Shot (kind 0, age 0 or 1) at `(x − 40, y − 40)`. Count only flashes
        // that weren't in the previous snapshot (an age-0 flash is seen again
        // at age 1). Age 1 means the click came before this tick's update,
        // age 0 after it (mid-frame).
        if !raw_list(s, "alien").is_empty() || !raw_list(s, "dead_alien").is_empty() {
            let flashes = |snap: Option<&Snapshot>| -> Vec<Shot> {
                snap.map(|x| {
                    raw_list(x, "shot")
                        .iter()
                        .map(|b| shot_from_raw(b))
                        .filter(|x| x.kind == 0)
                        .collect()
                })
                .unwrap_or_default()
            };
            let before = flashes(prev_snap);
            for f in flashes(Some(s)).iter().filter(|f| f.age <= 1) {
                let seen = before.iter().any(|p| {
                    p.widget_x == f.widget_x && p.widget_y == f.widget_y && p.age + 1 == f.age
                });
                if seen {
                    continue;
                }
                let click = Input::Click {
                    x: f.widget_x + 40,
                    y: f.widget_y + 40,
                };
                // A coin clicked inside an alien collects *and* shoots, so
                // one click shows as both a collected coin (inferred above)
                // and a flash. The coin's click point is its centre, the
                // flash's is the real one: replace the coin's.
                let at = if f.age == 0 { s.tick + 1 } else { s.tick };
                let coin_click = out.inputs.get(&at).and_then(|v| {
                    v.iter().position(|i| {
                        matches!(*i, Input::Click { x, y }
                            if (x - (f.widget_x + 40)).abs() <= 36 && (y - (f.widget_y + 40)).abs() <= 36)
                    })
                });
                if let Some(k) = coin_click
                    && coins_collected_at.contains(&at)
                {
                    out.inputs.get_mut(&at).unwrap()[k] = click;
                    continue;
                }
                push(s.tick, f.age == 0, click, &mut out);
            }
        }
        // The 1-2 "DANGER!" dialog: the board unpauses with the countdown at
        // 276 → the player dismissed it before this tick.
        let cd = |s: Option<&Snapshot>| {
            s.and_then(|s| s.state.get("board"))
                .and_then(|b| b.get("wave"))
                .and_then(|w| w.get("countdown"))
                .and_then(Value::as_i64)
        };
        let paused = |s: Option<&Snapshot>| {
            s.and_then(|s| s.state.get("board"))
                .and_then(|b| b.get("paused"))
                .and_then(Value::as_bool)
        };
        if paused(prev_snap) == Some(true) && paused(Some(s)) == Some(false) {
            if cd(prev_snap) == Some(276) {
                out.inputs
                    .entry(s.tick)
                    .or_default()
                    .push(Input::DismissDialog);
            } else {
                out.inputs
                    .entry(s.tick)
                    .or_default()
                    .push(Input::SetPaused(false));
            }
        } else if paused(prev_snap) == Some(false)
            && paused(Some(s)) == Some(true)
            && cd(Some(s)) != Some(276)
            && !s
                .state
                .get("guppies")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        {
            // A pause that isn't a board dialog (focus loss / menu). It
            // happened before this tick's update.
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::SetPaused(true));
        }
        // A carnivore appearing (raw list `oscar`) with money down: bought.
        let n_oscar = |snap: Option<&Snapshot>| snap.map(|x| raw_list(x, "oscar").len());
        if let (Some(a), Some(b)) = (n_oscar(prev_snap), n_oscar(Some(s)))
            && b > a
            && money_of(Some(s)) < money_of(prev_snap)
        {
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::Buy(Slot::Carnivore));
        }
        // Food upgrades: the board's food level / quantity step up.
        let field = |s: Option<&Snapshot>, k: &str| {
            s.and_then(|s| s.state.get("board"))
                .and_then(|b| b.get(k))
                .and_then(Value::as_i64)
        };
        if let (Some(a), Some(b)) = (field(prev_snap, "food_level"), field(Some(s), "food_level"))
            && b > a
        {
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::Buy(Slot::FoodQuality));
        }
        if let (Some(a), Some(b)) = (field(prev_snap, "max_food"), field(Some(s), "max_food"))
            && b > a
        {
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::Buy(Slot::FoodQuantity));
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
    out.dirty.extend(torn_snapshots(t));
    out
}

/// Replay a capture: start from its first snapshot and step once per
/// captured tick, feeding `inputs[tick]` before that tick's update.
pub fn replay(t: &Trace, inputs: &BTreeMap<u32, Vec<Input>>) -> Result<Trace, ReplayError> {
    replay_from(board_from_capture(t)?, t, inputs)
}

/// [`replay`] from a prepared board (e.g. with level settings applied).
pub fn replay_from(
    board: Board,
    t: &Trace,
    inputs: &BTreeMap<u32, Vec<Input>>,
) -> Result<Trace, ReplayError> {
    replay_resync(board, t, inputs, &BTreeMap::new())
}

/// Ticks where the sim deliberately deviates from the original's RNG use
/// (implementation artifacts, CLAUDE.md "Bugs"): after stepping such a tick,
/// the app RNG is set to the capture's state there. `full` is the whole
/// capture whose header RNG the states are derived from.
pub fn deviation_resyncs(full: &Trace, ticks: &[u32]) -> BTreeMap<u32, MtRand> {
    ticks
        .iter()
        .filter_map(|&t| Some((t, rng_from(&rebase_rng(full, "app", t)?))))
        .collect()
}

/// Dead-alien debris ticks (effect sparks are allocated): the snapshot shows
/// new effect shots (kind ≥ 3) with age ≤ 1. These are the ticks where the
/// original's stale-field read can draw extra RNG.
pub fn debris_ticks(t: &Trace) -> Vec<u32> {
    // Every tick that creates an effect shot (kind != 0) can hit the
    // original's stale-field draw: dead-alien debris bursts (life a multiple
    // of 7 above 50), hit sparks and gore. Detect bursts by the dead alien's
    // life, and the rest by a new effect shot appearing (age 0/1, not seen
    // in the previous snapshot).
    let mut out = Vec::new();
    let mut prev: Vec<Shot> = Vec::new();
    for s in &t.snapshots {
        let shots: Vec<Shot> = raw_list(s, "shot")
            .iter()
            .map(|b| shot_from_raw(b))
            .collect();
        let burst = raw_list(s, "dead_alien").first().is_some_and(|b| {
            let life = dead_alien_from_raw(b).life;
            life % 7 == 0 && life > 50
        });
        let fresh = shots.iter().any(|x| {
            x.kind != 0
                && x.age <= 1
                && !prev.iter().any(|p| {
                    p.kind == x.kind
                        && p.widget_x == x.widget_x
                        && p.widget_y == x.widget_y
                        && p.age + 1 == x.age
                })
        });
        if burst || fresh {
            out.push(s.tick);
        }
        prev = shots;
    }
    out
}

/// Drop the shot list from snapshots while debris sparks are alive: their
/// kinds, positions and delays come after an artifact draw the sim doesn't
/// make (alien-sylvester.md#deviations), so the list legitimately differs.
/// Everything else (fish, pets, alien, coins, RNG after re-sync) is compared.
///
/// `capture` is the raw capture `decoded` was built from (decoding drops the
/// raw object bytes this needs).
pub fn without_debris_shots(decoded: &Trace, capture: &Trace) -> Trace {
    let decoded = &without_deviation_sounds(decoded, capture);
    // A spark that got the artifact start delay outlives ours: mask the list
    // for its whole life, tracking it by (kind, widget, r).
    let mut delayed = std::collections::BTreeSet::new();
    let mut debris = std::collections::BTreeSet::new();
    for s in &capture.snapshots {
        let shots: Vec<Shot> = raw_list(s, "shot")
            .iter()
            .map(|b| shot_from_raw(b))
            .collect();
        for x in &shots {
            // Delayed: still waiting (delay > 0), or the delay already ran
            // out and left its frame behind its age (frame −1 on the tick
            // the wait ends; ours never lags).
            let lags = x.frame != if x.kind == 0 { x.age } else { x.age / 2 } - 1;
            if x.kind != 0 && (x.delay > 0 || lags) {
                delayed.insert((x.kind, x.widget_x, x.widget_y, x.r));
            }
        }
        if !raw_list(s, "dead_alien").is_empty()
            || shots
                .iter()
                .any(|x| x.kind >= 3 || delayed.contains(&(x.kind, x.widget_x, x.widget_y, x.r)))
        {
            debris.insert(s.tick);
        }
    }
    let mut out = decoded.clone();
    for s in &mut out.snapshots {
        if debris.contains(&s.tick) {
            s.state.remove("shots");
        }
    }
    out
}

/// On ticks that create an effect shot, the original may draw an extra
/// start-delay number before the shot sound's variant draws (the stale
/// field, `alien-sylvester.md#deviations`); the RNG is re-synced only after
/// the tick. So on those ticks the RNG-picked shot sound variants (ZZAM,
/// RICOCHET) aren't compared; [`mask_rng_sounds`] drops them from the
/// sim's side on the same ticks.
pub fn without_deviation_sounds(decoded: &Trace, capture: &Trace) -> Trace {
    let ticks: std::collections::BTreeSet<u32> = debris_ticks(capture).into_iter().collect();
    let mut out = decoded.clone();
    for s in &mut out.snapshots {
        if ticks.contains(&s.tick) {
            split_rng_sounds(&mut s.state);
        }
    }
    out
}

/// Drop sounds whose choice depends on a shot-sound RNG draw.
fn split_rng_sounds(state: &mut BTreeMap<String, Value>) {
    if let Some(Value::Array(v)) = state.get_mut("sounds") {
        v.retain(|x| !matches!(x.as_i64(), Some(0x145 | 0x132)));
        state.insert("sounds_rng_masked".into(), Value::Bool(true));
    }
}

/// Apply [`without_deviation_sounds`]' masking to a sim trace: on ticks the
/// expected trace masked, the sim's variants are set aside too.
pub fn mask_rng_sounds(actual: &mut Trace, expected: &Trace) {
    let ticks: std::collections::BTreeSet<u32> = expected
        .snapshots
        .iter()
        .filter(|s| s.state.contains_key("sounds_rng_masked"))
        .map(|s| s.tick)
        .collect();
    for s in &mut actual.snapshots {
        if ticks.contains(&s.tick) {
            split_rng_sounds(&mut s.state);
        }
    }
}

/// [`replay_from`] with RNG re-syncs at deviation ticks.
pub fn replay_resync(
    board: Board,
    t: &Trace,
    inputs: &BTreeMap<u32, Vec<Input>>,
    resync: &BTreeMap<u32, MtRand>,
) -> Result<Trace, ReplayError> {
    replay_resync_dirty(board, t, inputs, resync, &[])
}

/// The capture's app-RNG position at `tick`.
fn mti_at(t: &Trace, tick: u32) -> Option<usize> {
    t.snapshots
        .iter()
        .find(|s| s.tick == tick)?
        .state
        .get("rng_mti")?
        .get("app")?
        .as_u64()
        .map(|m| m as usize)
}

/// [`replay_resync`], skipping re-syncs on `dirty` (mid-frame) ticks.
pub fn replay_resync_dirty(
    mut board: Board,
    t: &Trace,
    inputs: &BTreeMap<u32, Vec<Input>>,
    resync: &BTreeMap<u32, MtRand>,
    dirty: &[u32],
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
        let step_inputs = inputs.get(&next).map_or(&[][..], Vec::as_slice);
        // On a deviation tick with a click, the original may have drawn the
        // stale start delay (`alien-sylvester.md#deviations`) while handling
        // the click, before the rest of the tick. Re-syncing after the tick
        // fixes the RNG but not what the shifted draws decided inside it, so
        // discard that many draws first: the count whose result lands on the
        // capture's RNG position.
        let skip =
            if resync.contains_key(&next) && !dirty.contains(&next) && !step_inputs.is_empty() {
                let want = mti_at(t, next);
                (0..3).find(|&k| {
                    let mut b = board.clone();
                    for _ in 0..k {
                        b.rng.next_u31();
                    }
                    b.step(step_inputs);
                    want == Some(b.rng.state().1)
                })
            } else {
                None
            };
        for _ in 0..skip.unwrap_or(0) {
            board.rng.next_u31();
        }
        board.step(step_inputs);
        // Re-sync only where the snapshot is a whole tick: a mid-frame click
        // (input applied at `next + 1` and marked dirty) means the capture's
        // RNG already includes that click's draws.
        if let Some(r) = resync.get(&board.tick)
            && !dirty.contains(&board.tick)
        {
            board.rng = r.clone();
        }
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
