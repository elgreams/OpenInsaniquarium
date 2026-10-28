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
            // FishTypePet: the pet id (`+0x234`) says which.
            0x15 if rd_i32(b, 0x234) == 3 => FishKind::Prego,
            0x15 if rd_i32(b, 0x234) == 6 => FishKind::Vert,
            0x15 if rd_i32(b, 0x234) == 4 => FishKind::Zorf,
            0x15 if rd_i32(b, 0x234) == 8 => FishKind::Meryl,
            0x15 if rd_i32(b, 0x234) == 9 => FishKind::Wadsworth,
            0x15 if rd_i32(b, 0x234) == 10 => FishKind::Seymour,
            0x15 if rd_i32(b, 0x234) == 11 => FishKind::Shrapnel,
            0x15 if rd_i32(b, 0x234) == 12 => FishKind::Gumbo,
            0x15 if rd_i32(b, 0x234) == 13 => FishKind::Blip,
            0x15 if rd_i32(b, 0x234) == 15 => FishKind::Nimbus,
            0x15 if rd_i32(b, 0x234) == 17 => FishKind::Gash,
            0x15 if rd_i32(b, 0x234) == 18 => FishKind::Angie,
            0x15 if rd_i32(b, 0x234) == 16 => FishKind::Amp,
            0x15 if rd_i32(b, 0x234) == 19 => FishKind::Presto,
            0x15 if rd_i32(b, 0x234) == 20 => FishKind::Brinkley,
            0x15 if rd_i32(b, 0x234) == 21 => FishKind::Nostradamus,
            0x15 if rd_i32(b, 0x234) == 22 => FishKind::Stanley,
            0x15 if rd_i32(b, 0x234) == 23 => FishKind::Walter,
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
        presto: rd_i32(b, 0x8c) == 0x15 && b.get(0x230).is_some_and(|&v| v != 0),
        recharge: if rd_i32(b, 0x8c) == 0x15 && b.len() > 0x23c {
            rd_i32(b, 0x238)
        } else {
            0
        },
        puff: if rd_i32(b, 0x8c) == 0x15 && b.len() > 0x24c {
            rd_i32(b, 0x248)
        } else {
            0
        },
        timer_23c: if rd_i32(b, 0x8c) == 0x15 && b.len() > 0x244 {
            rd_i32(b, 0x23c)
        } else {
            0
        },
        count_240: if rd_i32(b, 0x8c) == 0x15 && b.len() > 0x244 {
            rd_i32(b, 0x240)
        } else {
            0
        },
        clicks: if rd_i32(b, 0x8c) == 0x15
            && !matches!(rd_i32(b, 0x234), 22 | 23)
            && b.len() > 0x264
        {
            rd_i32(b, 0x260)
        } else {
            0
        },
        grow: if rd_i32(b, 0x8c) == 0x15 && rd_i32(b, 0x234) == 22 {
            rd_i32(b, 0x260)
        } else {
            0
        },
        // Walter's glove (`+0x24c`, not on any list: a pointer; its life and
        // position aren't captured) is kept as "has a glove" (pet-walter.md).
        glove: (rd_i32(b, 0x8c) == 0x15 && b.len() > 0x250 && rd_i32(b, 0x24c) != 0)
            .then(|| insaniquarium_sim::missile::Glove::new(false)),
        punches: if rd_i32(b, 0x8c) == 0x15
            && matches!(rd_i32(b, 0x234), 19 | 23)
            && b.len() > 0x264
        {
            rd_i32(b, 0x260)
        } else {
            0
        },
        tired: if rd_i32(b, 0x8c) == 0x15 && b.len() >= 0x268 {
            rd_i32(b, 0x264)
        } else {
            0
        },
        reverting: false,
        revert_state: None,
        // `+0x98`: a pointer in the capture; kept as "a missile is on it".
        missile: (b.len() > 0x9c && rd_i32(b, 0x98) != 0).then_some(0),
        crosshair: 0,
        // Meryl's blink flag `+0x244`; Wadsworth's sheltering flag `+0x250`.
        blink: rd_i32(b, 0x8c) == 0x15
            && match rd_i32(b, 0x234) {
                9 => b.get(0x250).is_some_and(|&v| v != 0),
                _ => b.get(0x244).is_some_and(|&v| v != 0),
            },
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
        tossed: b[0x190] != 0,
        risen: b[0x184] != 0,
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
        risen: b[0x168] != 0,
        text: None,
        colour: if b.len() > 0x1a4 { rd_i32(b, 0x1a0) } else { 0 },
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
        presto: b.get(0x1c8).is_some_and(|&v| v != 0),
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
        fire_timer: rd_i32(b, 0x1a4),
        fire_interval: rd_i32(b, 0x1a8),
        fire_anim: rd_i32(b, 0x1ac),
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
    board.starcatchers = raw_list(first, "penta")
        .iter()
        .map(|b| starcatcher_from_raw(b))
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
        // A chunk can open under the "Game Paused" dialog (18): focus went
        // to the recorder. The player's click resumes it.
        let paused = b.get("paused").and_then(Value::as_bool) == Some(true);
        let dialogs = first.state.get("pres").and_then(|p| p.get("dialogs"));
        if paused
            && dialogs
                .and_then(Value::as_array)
                .is_some_and(|d| d.iter().any(|v| v == 18))
        {
            board.paused = true;
            board.focus_paused = true;
        }
        if let Some(o) = b.get("owned").and_then(Value::as_str).and_then(hex_bytes) {
            for (flag, &v) in board.owned.iter_mut().zip(&o) {
                *flag = v != 0;
            }
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
        // Prego's birth sound is a profile setting the capture doesn't
        // record; take it from the first birth sound in the capture.
        let played = crate::presentation::capture_sounds(t);
        if played.values().flatten().any(|&v| v == 0x120) {
            board.prego_sound = insaniquarium_sim::board::PregoSound::Fart;
        } else if played.values().flatten().any(|&v| v == 0x121) {
            board.prego_sound = insaniquarium_sim::board::PregoSound::Fart2;
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
        "widget": [f.widget_x, f.widget_y], "blink": f.blink, "clicks": f.clicks,
        "presto": [f.presto, f.recharge, f.puff], "t23c": f.timer_23c, "c240": f.count_240,
        "targeted": f.missile.is_some(),
        "grow": f.grow, "walter": [f.glove.is_some(), f.punches, f.tired],
    })
}

fn pet_json(p: &Pet) -> Value {
    json!({
        "id": p.id, "x": p.x, "y": p.y, "vx": p.vx, "vy": p.vy, "facing": p.facing,
        "tvx": p.target_vx, "mode": p.mode,
        "timers": [p.counter, p.reroll_timer, p.counter_1a8, p.countdown_1ac, p.shell],
        "pearl_taken": p.pearl_taken, "presto": p.presto,
        "anim": [p.turn, p.anim, p.cel], "widget": [p.widget_x, p.widget_y],
    })
}

fn alien_json(a: &Alien) -> Value {
    json!({
        "kind": a.kind, "warp": a.warp, "x": a.x, "y": a.y, "vx": a.vx, "vy": a.vy,
        "tv": [a.tvx, a.tvy], "eat": a.eat_delay, "anim": [a.anim, a.turn, a.cel],
        "last_vx": a.last_vx, "mode": [a.mode, a.counter, a.reroll],
        "hp": a.health, "cool": a.cooldown, "fire": [a.fire_timer, a.fire_interval, a.fire_anim], "widget": [a.widget_x, a.widget_y],
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

/// A missile from its raw bytes (pet-stanley.md#the-ball-missle-kind-2).
/// The target is a pointer in the capture: it is kept as "has a target".
pub fn missile_from_raw(b: &[u8]) -> insaniquarium_sim::missile::Missile {
    insaniquarium_sim::missile::Missile {
        x: rd_f64(b, 0x158),
        y: rd_f64(b, 0x160),
        vx: rd_f64(b, 0x168),
        vy: rd_f64(b, 0x170),
        launch: rd_i32(b, 0x17c),
        spin: rd_i32(b, 0x180),
        divisor: rd_f64(b, 0x188),
        bounced: b[0x190] != 0,
        cel: rd_i32(b, 0x194),
        kind: rd_i32(b, 0x19c),
        row: rd_i32(b, 0x1a0),
        target: (rd_i32(b, 0x1a4) != 0).then_some(0),
        id: 0,
        shot_progress: rd_i32(b, 0x1a8),
        shot_duration: rd_i32(b, 0x1ac),
        shot_from: (rd_i32(b, 0x1b0), rd_i32(b, 0x1b4)),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
    }
}

/// A Starcatcher from its raw bytes (starcatcher.md#fields).
pub fn starcatcher_from_raw(b: &[u8]) -> insaniquarium_sim::starcatcher::Starcatcher {
    insaniquarium_sim::starcatcher::Starcatcher {
        x: rd_f64(b, 0x158),
        y: rd_f64(b, 0x160),
        vx: rd_f64(b, 0x168),
        vy: rd_f64(b, 0x170),
        divisor: rd_f64(b, 0x178),
        hunger: rd_i32(b, 0x9c),
        fade: rd_i32(b, 0xa0),
        fade_to_hungry: b[0xa8] != 0,
        splash: rd_i32(b, 0x180),
        target_vx: rd_f64(b, 0x188),
        mode: rd_i32(b, 0x190),
        chase_timer: rd_i32(b, 0x194),
        reroll_timer: rd_i32(b, 0x198),
        anim: rd_i32(b, 0x19c),
        cel: rd_i32(b, 0x1a0),
        eats: rd_i32(b, 0x1a4),
        widget_x: rd_i32(b, 0x30),
        widget_y: rd_i32(b, 0x34),
        shadow_y: 410,
    }
}

fn starcatcher_json(s: &insaniquarium_sim::starcatcher::Starcatcher) -> Value {
    json!({
        "x": s.x, "y": s.y, "vx": s.vx, "vy": s.vy, "div": s.divisor, "hunger": s.hunger,
        "fade": [s.fade, s.fade_to_hungry], "splash": s.splash, "tvx": s.target_vx,
        "mode": s.mode, "timers": [s.chase_timer, s.reroll_timer], "anim": [s.anim, s.cel],
        "eats": s.eats, "widget": [s.widget_x, s.widget_y],
    })
}

fn missile_json(m: &insaniquarium_sim::missile::Missile) -> Value {
    json!({
        "x": m.x, "y": m.y, "vx": m.vx, "vy": m.vy, "launch": m.launch, "spin": m.spin,
        "div": m.divisor, "bounced": m.bounced, "cel": m.cel, "kind": m.kind, "row": m.row,
        "targeted": m.target.is_some() || m.kind == 0, "widget": [m.widget_x, m.widget_y],
        "shot": if m.shot_progress > 0 {
            json!([m.shot_progress, m.shot_duration, m.shot_from.0, m.shot_from.1])
        } else {
            // Unset (heap leftovers) until Stanley shoots it.
            json!(null)
        },
    })
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
        "x": f.x, "y": f.y, "vx": f.vx, "vy": f.vy, "anim": [f.anim, f.anim_speed], "fade": f.fade,
        "level": f.level, "motion": f.motion, "delay": f.delay, "widget": [f.widget_x, f.widget_y],
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
        "missiles".into(),
        Value::Array(board.missiles.iter().map(missile_json).collect()),
    );
    state.insert(
        "starcatchers".into(),
        Value::Array(board.starcatchers.iter().map(starcatcher_json).collect()),
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
            if raw_present(&s) {
                let stars = raw_list(&s, "penta")
                    .iter()
                    .map(|b| starcatcher_json(&starcatcher_from_raw(b)))
                    .collect();
                s.state.insert("starcatchers".into(), Value::Array(stars));
                let missiles = raw_list(&s, "missile")
                    .iter()
                    .map(|b| missile_json(&missile_from_raw(b)))
                    .collect();
                s.state.insert("missiles".into(), Value::Array(missiles));
            }
            let shots = raw_list(&s, "shot")
                .iter()
                .map(|b| shot_json(&shot_from_raw(b)))
                .collect();
            s.state.insert("shots".into(), Value::Array(shots));
            // The full shot state, for replays that adopt the capture's list
            // (`replay_resync_dirty`). The sim has no such key: not compared.
            let shots_full: Vec<Value> = raw_list(&s, "shot")
                .iter()
                .map(|b| {
                    let x = shot_from_raw(b);
                    json!([x.kind, x.delay, x.age, x.frame, x.r, x.widget_x, x.widget_y])
                })
                .collect();
            s.state
                .insert("shots_full".into(), Value::Array(shots_full));
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
            if let Some(Value::Object(b)) = s.state.get_mut("board")
                && let Some(Value::String(h)) = b.remove("samples")
                && let Some(raw) = hex_bytes(&h)
            {
                // Keep the RNG-picked shot sounds' last-play clocks, for
                // the deviation re-syncs (`replay_resync_global`).
                let slot = |id: usize| {
                    raw.get((id - 0x106) * 4..(id - 0x106) * 4 + 4)
                        .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
                };
                b.insert("rng_samples".into(), json!([slot(0x145), slot(0x132)]));
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

/// The replay tick (board-clock keyed, as [`by_board_clock`]) for each
/// update count `u` in a raw capture window: the last snapshot read at or
/// before `u` gives the clock; later updates add one each.
pub fn demo_tick_map(raw: &Trace) -> impl Fn(i64) -> Option<u32> + use<> {
    let clock = |s: &Snapshot| {
        s.state
            .get("board")
            .and_then(|b| b.get("clock"))
            .and_then(Value::as_i64)
    };
    let pairs: Vec<(i64, i64)> = raw
        .snapshots
        .iter()
        .filter_map(|s| Some((s.state.get("u")?.as_i64()?, clock(s)?)))
        .collect();
    let tick0 = raw.snapshots.first().map_or(0, |s| s.tick);
    let base = pairs.first().map_or(0, |p| p.1);
    move |u: i64| {
        let i = pairs.partition_point(|p| p.0 <= u).checked_sub(1)?;
        let (pu, pc) = pairs[i];
        u32::try_from(i64::from(tick0) + (pc + (u - pu)) - base).ok()
    }
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
        let mut skip_sounds = 0;
        let mut prev_was_gap = false;
        for s in &expected.snapshots {
            let gap = prev_tick.is_some_and(|p| s.tick > p + 1);
            prev_tick = Some(s.tick);
            // A long gap (a stalled capture) can hold sounds the snapshot
            // after it can't place (only the sample table's last stamp is
            // read): don't compare sounds on the two snapshots after it.
            // A torn burst (several short gaps in a row) can't place its
            // sounds either; treat back-to-back gaps like a long one.
            if gap && (prev_gap_len(&expected.snapshots, s.tick) > 3 || prev_was_gap) {
                skip_sounds = 2;
            }
            prev_was_gap = gap;
            if skip_sounds > 0 {
                skip_sounds -= 1;
                if !self.dirty.contains(&s.tick) {
                    let mut s = s.clone();
                    s.state.remove("sounds");
                    carry.clear();
                    snapshots.push(s);
                }
                continue;
            }
            if gap
                && !self.dirty.contains(&s.tick)
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

/// Ticks missing before the snapshot at `tick` (0 if it follows its
/// predecessor directly).
fn prev_gap_len(snapshots: &[Snapshot], tick: u32) -> u32 {
    let i = snapshots.partition_point(|s| s.tick < tick);
    match i.checked_sub(1).map(|j| snapshots[j].tick) {
        Some(p) => tick - p - 1,
        None => 0,
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
        // New pellets are appended. Normally anything past the survivors is
        // new, but a pellet eaten on the tick another is dropped keeps the
        // count: also take any pellet just made (anim 0/1) that wasn't in the
        // previous snapshot.
        let prev_foods: Vec<Food> = prev_snap
            .map(|p| {
                raw_list(p, "food")
                    .iter()
                    .map(|b| food_from_raw(b))
                    .collect()
            })
            .unwrap_or_default();
        let fresh = |f: &Food| {
            !prev_foods
                .iter()
                .any(|p| p.x == f.x && p.anim_speed == f.anim_speed && p.y <= f.y)
        };
        for (k, f) in foods.iter().enumerate() {
            if k < prev_food.min(foods.len()) && !fresh(f) {
                continue;
            }
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
        let newest = s
            .state
            .get("guppies")
            .and_then(Value::as_array)
            .and_then(|g| g.last())
            .and_then(|g| g.get("raw")?.as_str().and_then(hex_bytes))
            .map(|raw| fish_from_raw(&raw));
        // Bought: it starts at y 30 falling 18..22 px/tick (÷ div), so it is
        // near the surface on its first snapshot. Money usually drops, but a
        // coin banked on the same tick can hide that.
        let splashed = newest
            .as_ref()
            .is_some_and(|f| f.y < 45.0 && f.vy >= 15.0 && f.vy_damp_timer >= 44);
        let paid = money_of(Some(s)) < money_of(prev_snap);
        if guppies > prev_guppies && (paid || (splashed && prev_guppies > 0)) {
            // Still at its spawn y (30.0): bought after this tick's update
            // (mid-frame), so it applies before the next tick.
            let unmoved = newest.is_some_and(|f| f.y == 30.0);
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
            // Shots don't age while the board is paused: the same flash
            // then shows with the same age.
            let still = s
                .state
                .get("board")
                .and_then(|b| b.get("paused"))
                .and_then(Value::as_bool)
                == Some(true);
            for f in flashes(Some(s)).iter().filter(|f| f.age <= 1) {
                let seen = before.iter().any(|p| {
                    p.widget_x == f.widget_x
                        && p.widget_y == f.widget_y
                        && (p.age + 1 == f.age || (still && p.age == f.age))
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
        if let (Some(a), Some(b)) = (field(prev_snap, "laser"), field(Some(s), "laser"))
            && b > a
        {
            out.inputs
                .entry(s.tick)
                .or_default()
                .push(Input::Buy(Slot::Weapon));
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
    // One pass over the capture, stepping the app stream by each snapshot's
    // draw count (as `rebase_rng` does) and keeping the wanted ticks.
    let wanted: std::collections::BTreeSet<u32> = ticks.iter().copied().collect();
    let mut out = BTreeMap::new();
    let Some(mut rng) = full.header.rng.get("app").map(rng_from) else {
        return out;
    };
    let mut prev = rng.state().1;
    for s in &full.snapshots {
        if wanted.range(s.tick..).next().is_none() {
            break;
        }
        let Some(m) = s
            .state
            .get("rng_mti")
            .and_then(|r| r.get("app"))
            .and_then(Value::as_u64)
        else {
            continue;
        };
        let m = m as usize;
        for _ in 0..(m + 624 - prev) % 624 {
            rng.next_u31();
        }
        prev = m;
        if wanted.contains(&s.tick) {
            out.insert(s.tick, rng.clone());
        }
    }
    out
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
        // Age up to 2: a spark made inside the object pass can already have
        // been aged twice by the snapshot (missile smoke, alien-kind5.md#52).
        let fresh = shots.iter().any(|x| {
            x.kind != 0
                && x.age <= 2
                && !prev.iter().any(|p| {
                    p.kind == x.kind
                        && p.widget_x == x.widget_x
                        && p.widget_y == x.widget_y
                        && p.age + 1 == x.age
                })
        });
        // A paused board updates nothing: its frozen sparks aren't new.
        let paused = s
            .state
            .get("board")
            .and_then(|b| b.get("paused"))
            .and_then(Value::as_bool)
            == Some(true);
        if (burst || fresh) && !paused {
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
        // Gore (kind 1, a fish eaten): its positions are drawn after the
        // first spark's possible artifact draw, so a later piece can sit
        // elsewhere; mask its short life (10 ticks) too.
        if !raw_list(s, "dead_alien").is_empty()
            || shots.iter().any(|x| {
                x.kind >= 3
                    || x.kind == 1
                    || delayed.contains(&(x.kind, x.widget_x, x.widget_y, x.r))
            })
        {
            debris.insert(s.tick);
        }
    }
    // Sparks with a different start delay (or none) die a few ticks apart
    // from ours: keep masking briefly after the last one seen.
    let tail: std::collections::BTreeSet<u32> = debris.iter().flat_map(|&t| t..=t + 3).collect();
    let mut out = decoded.clone();
    for s in &mut out.snapshots {
        if tail.contains(&s.tick) {
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

/// The prey index (guppies, then carnivores) whose loss reproduces the
/// capture's fish and carnivore lists, when the sim's own pick doesn't.
fn eat_choice(board: &Board, inputs: &[Input], t: &Trace, tick: u32) -> Option<usize> {
    let target = t.snapshots.iter().find(|s| s.tick == tick)?;
    let fits = |b: &Board| {
        let snap = snapshot(b);
        // Which fish survive (by stage and position), and where the aliens
        // steer.
        let key = |v: &Value| (v.get("stage").cloned(), v.get("widget").cloned());
        let lists = ["fish", "carnivores"].iter().all(|k| {
            match (target.state.get(*k), snap.state.get(*k)) {
                (Some(Value::Array(x)), Some(Value::Array(y))) => {
                    x.len() == y.len() && x.iter().zip(y).all(|(a, b)| key(a) == key(b))
                }
                _ => true,
            }
        });
        let vel = |v: &Value| (v.get("vx").cloned(), v.get("vy").cloned());
        let aliens = match (target.state.get("aliens"), snap.state.get("aliens")) {
            (Some(Value::Array(x)), Some(Value::Array(y))) => {
                x.len() == y.len() && x.iter().zip(y).all(|(a, b)| vel(a) == vel(b))
            }
            _ => true,
        };
        lists && aliens
    };
    let mut b = board.clone();
    b.step(inputs);
    if fits(&b) {
        return None;
    }
    let n = board.guppies.len() + board.carnivores.len();
    (0..n).find(|&c| {
        let mut b = board.clone();
        b.artifacts.eat_choice = Some(c);
        b.step(inputs);
        fits(&b)
    })
}

/// Should a volley's distance ties go to the later fish this tick?
/// (Replay-only; see `ArtifactPlan::volley_ties_last`.) The capture's
/// missile/fish links are pointers, so both choices are run ahead with the
/// replay's inputs until the capture's "which fish still has a missile"
/// tells them apart.
fn volley_ties_last(
    board: &Board,
    all_inputs: &BTreeMap<u32, Vec<Input>>,
    t: &Trace,
    tick: u32,
) -> bool {
    let inputs = |k: u32| all_inputs.get(&k).map_or(&[][..], Vec::as_slice);
    let mut plain = board.clone();
    plain.step(inputs(tick));
    if plain.missiles.len() == board.missiles.len() {
        return false;
    }
    let mut tied = board.clone();
    tied.artifacts.volley_ties_last = true;
    tied.step(inputs(tick));
    let links = |b: &Board| -> Vec<Option<u32>> { b.guppies.iter().map(|f| f.missile).collect() };
    if links(&plain) == links(&tied) {
        return false;
    }
    let marks = |s: &Snapshot| -> Vec<Value> {
        s.state
            .get("fish")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|f| f.get("targeted").cloned().unwrap_or(Value::Null))
                    .collect()
            })
            .unwrap_or_default()
    };
    for k in tick + 1..tick + 400 {
        plain.step(inputs(k));
        tied.step(inputs(k));
        let Some(s) = t.snapshots.iter().find(|s| s.tick == k) else {
            continue;
        };
        let want = marks(s);
        let (p, q) = (
            marks(&snapshot(&plain)) == want,
            marks(&snapshot(&tied)) == want,
        );
        if p != q {
            return q;
        }
    }
    false
}

/// Should this tick's glove hits run in reverse order to match the
/// capture's pushes? (Replay-only; see `ArtifactPlan::glove_reversed`.)
fn glove_reversed(board: &Board, inputs: &[Input], t: &Trace, tick: u32) -> bool {
    let Some(target) = t.snapshots.iter().find(|s| s.tick == tick) else {
        return false;
    };
    let pushes = |s: &Snapshot| -> Vec<Value> {
        ["fish", "fish_pets"]
            .iter()
            .filter_map(|k| s.state.get(*k)?.as_array().cloned())
            .flatten()
            .map(|f| f.get("push").cloned().unwrap_or(Value::Null))
            .collect()
    };
    let want = pushes(target);
    let mut b = board.clone();
    b.step(inputs);
    if pushes(&snapshot(&b)) == want {
        return false;
    }
    let mut b = board.clone();
    b.artifacts.glove_reversed = true;
    b.step(inputs);
    pushes(&snapshot(&b)) == want
}

/// The artifact plan for one replayed tick (see [`replay_resync_dirty`]).
/// Tries every set of delayed sparks, fewest first; returns the first that
/// matches the capture's RNG position and its fish, food and shots, else
/// the first that matches the RNG alone, else none.
fn artifact_plan(
    board: &Board,
    inputs: &[Input],
    t: &Trace,
    tick: u32,
    then: Option<&[Input]>,
) -> insaniquarium_sim::alien::ArtifactPlan {
    use insaniquarium_sim::alien::ArtifactPlan;
    // A dirty (mid-frame) snapshot isn't in the cleaned trace: step the next
    // tick too (with its inputs, which include the late click) and aim for
    // its position and state.
    let target_tick = if then.is_some() { tick + 1 } else { tick };
    let Some(want) = mti_at(t, target_tick) else {
        return ArtifactPlan::default();
    };
    let target = t.snapshots.iter().find(|s| s.tick == target_tick);
    let fits = |b: &Board| {
        let snap = snapshot(b);
        target.is_none_or(|e| {
            ["fish", "foods", "shots"]
                .iter()
                .all(|k| match (e.state.get(*k), snap.state.get(*k)) {
                    (Some(x), Some(y)) => x == y,
                    _ => true,
                })
        })
    };
    let run = |plan: &ArtifactPlan| {
        let mut b = board.clone();
        b.artifacts = plan.clone();
        b.step(inputs);
        if let Some(next) = then {
            b.artifacts = ArtifactPlan::default();
            b.step(next);
        }
        b
    };
    // How many effect shots this tick makes (without artifacts).
    let n = {
        let mut b = board.clone();
        b.artifacts = ArtifactPlan::default();
        b.step(inputs);
        b.artifacts.made.min(12)
    };
    let mut masks: Vec<u32> = (0..1u32 << n).collect();
    masks.sort_by_key(|m| m.count_ones());
    let mut first = None;
    for m in masks {
        let plan = ArtifactPlan {
            delay_draws: (0..n).map(|i| m & (1 << i) != 0).collect(),
            ..ArtifactPlan::default()
        };
        let b = run(&plan);
        if b.rng.state().1 == want {
            if fits(&b) {
                return plan;
            }
            first.get_or_insert(plan);
        }
    }
    first.unwrap_or_default()
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

/// The global MTRand state after every snapshot tick of `full`, stepped by
/// each snapshot's `rng_mti.global` draw count from the header state (as
/// [`rebase_rng`] does for one tick). The board's cosmetic bubbles draw from
/// this stream and aren't modelled, so a replay that needs a global-stream
/// decision (Nostradamus's sneeze) re-syncs it before each tick.
pub fn global_states(full: &Trace) -> BTreeMap<u32, MtRand> {
    let mut out = BTreeMap::new();
    let Some(mut rng) = full.header.rng.get("global").map(rng_from) else {
        return out;
    };
    let mut prev = rng.state().1;
    for s in &full.snapshots {
        let Some(m) = s
            .state
            .get("rng_mti")
            .and_then(|r| r.get("global"))
            .and_then(Value::as_u64)
        else {
            continue;
        };
        let m = m as usize;
        for _ in 0..(m + 624 - prev) % 624 {
            rng.next_u31();
        }
        prev = m;
        out.insert(s.tick, rng.clone());
    }
    out
}

/// [`replay_resync`], skipping re-syncs on `dirty` (mid-frame) ticks.
pub fn replay_resync_dirty(
    board: Board,
    t: &Trace,
    inputs: &BTreeMap<u32, Vec<Input>>,
    resync: &BTreeMap<u32, MtRand>,
    dirty: &[u32],
) -> Result<Trace, ReplayError> {
    replay_resync_global(board, t, inputs, resync, dirty, &BTreeMap::new())
}

/// [`replay_resync_dirty`], also setting the global stream to `global[tick]`
/// after each tick that has an entry (keyed like `t`).
pub fn replay_resync_global(
    mut board: Board,
    t: &Trace,
    inputs: &BTreeMap<u32, Vec<Input>>,
    resync: &BTreeMap<u32, MtRand>,
    dirty: &[u32],
    global: &BTreeMap<u32, MtRand>,
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
        // Before a deviation tick, take the capture's shot list: delayed
        // sparks outlive ours, and the 20-shot cap on new effect shots then
        // decides differently how many sparks (and draws) a burst makes.
        // Shots are cosmetic and masked while debris lives. (A mid-frame
        // snapshot isn't usable: take the latest whole one, aged to now.)
        // Not right after a dirty tick: the aged older list would lose the
        // shots that tick made.
        if resync.contains_key(&next)
            && !dirty.contains(&board.tick)
            && let Some(prev) = t
                .snapshots
                .iter()
                .rev()
                .find(|s| s.tick <= board.tick && !dirty.contains(&s.tick))
            && board.tick - prev.tick <= 3
            && let Some(Value::Array(list)) = prev.state.get("shots_full")
        {
            let g = |v: &Value, i: usize| v.get(i).and_then(Value::as_i64).unwrap_or(0) as i32;
            let mut shots: Vec<Shot> = list
                .iter()
                .map(|v| Shot {
                    kind: g(v, 0),
                    delay: g(v, 1),
                    age: g(v, 2),
                    frame: g(v, 3),
                    r: g(v, 4),
                    widget_x: g(v, 5),
                    widget_y: g(v, 6),
                })
                .collect();
            let at = prev.tick;
            for _ in at..board.tick {
                shots.retain_mut(Shot::update);
            }
            board.shots = shots;
        }
        // On a deviation tick the original's effect-shot ctor may draw a
        // stale start delay (`alien-sylvester.md#deviations`) for some of
        // the tick's sparks. Re-syncing after the tick fixes the RNG but not
        // what the shifted draws decided inside it, so find which sparks
        // drew: the plan whose result lands on the capture's RNG position
        // and reproduces its fish, food and shots.
        if resync.contains_key(&next) {
            // On a dirty (mid-frame) tick the capture's RNG includes a click
            // the replay applies next tick: aim for the position without it.
            let then = dirty
                .contains(&next)
                .then(|| inputs.get(&(next + 1)).map_or(&[][..], Vec::as_slice));
            board.artifacts = artifact_plan(&board, step_inputs, t, next, then);
        }
        // An alien with several fish in its eat box takes the first in
        // object-address order in the original (a deviation we don't
        // reproduce): if the capture lost a different fish, pick that one.
        if !dirty.contains(&next) && !board.aliens.is_empty() {
            board.artifacts.eat_choice = eat_choice(&board, step_inputs, t, next);
        }
        // The Destructor's volley: a distance tie goes by heap address
        // (`alien-kind5.md#61`). If our list order gives the wrong targets,
        // let ties go to the later fish.
        if board.aliens.iter().any(|a| a.kind == 5) {
            board.artifacts.volley_ties_last = volley_ties_last(&board, inputs, t, next);
        }
        // Walter's glove: several hits in one tick take the global draws in
        // heap order (`pet-walter.md#deviations`). If our order gives the
        // wrong pushes, try the reverse.
        if !dirty.contains(&next) && board.fish_pets.iter().any(|f| f.glove.is_some()) {
            board.artifacts.glove_reversed = glove_reversed(&board, step_inputs, t, next);
        }
        // The board's bubble systems (not modelled, `sim/board.md#the-global-
        // rng-stream-in-normal-play`) draw the global stream before the
        // object pass, so the sim's own global draws (pets, missiles) are
        // the last of the tick: start them that many draws before the
        // capture's end-of-tick position.
        if let (Some(start), Some(end)) = (global.get(&board.tick), global.get(&next)) {
            let total = (end.state().1 + 624 - start.state().1) % 624;
            let mut trial = board.clone();
            trial.global_rng = start.clone();
            trial.step(step_inputs);
            let ours = (trial.global_rng.state().1 + 624 - start.state().1) % 624;
            if ours > 0 && ours <= total {
                let mut g = start.clone();
                for _ in 0..total - ours {
                    g.next_u31();
                }
                board.global_rng = g;
            }
        }
        board.step(step_inputs);
        board.artifacts = Default::default();
        if let Some(g) = global.get(&board.tick) {
            board.global_rng = g.clone();
        }
        // A fish pet's timer written by the recorder (Cheat Engine), to get
        // to an ability inside one capture: the capture's `+0x220` jumps by
        // more than 100 in one tick, which no pet does on its own. Adopt it.
        // (The decoded trace keeps the pets as `fish_pets` JSON, `coin_t`.)
        if let Some(Value::Array(pets)) = t
            .snapshots
            .iter()
            .find(|s| s.tick == board.tick)
            .and_then(|s| s.state.get("fish_pets"))
        {
            for (f, p) in board.fish_pets.iter_mut().zip(pets) {
                if let Some(c) = p.get("coin_t").and_then(Value::as_i64) {
                    let c = c as i32;
                    if c - f.coin_timer > 100 {
                        f.coin_timer = c;
                    }
                }
                // Brinkley's pellet count `+0x240` pushed to just before a
                // rare coin (pet-brinkley.md#recording): it only ever rises
                // by 1 on its own.
                if let Some(c) = p.get("c240").and_then(Value::as_i64) {
                    let c = c as i32;
                    if f.kind == insaniquarium_sim::fish::FishKind::Brinkley && c - f.count_240 > 1
                    {
                        f.count_240 = c;
                    }
                }
            }
        }
        // Money written by the recorder (Cheat Engine), not by the game: the
        // capture's money jumps by 500 or more to a round hundreds value,
        // more than any coin pays. Adopt it so the replay can carry on (RE tooling only).
        if let Some(m) = t
            .snapshots
            .iter()
            .find(|s| s.tick == board.tick)
            .and_then(|s| s.state.get("board"))
            .and_then(|b| b.get("money"))
            .and_then(Value::as_i64)
            && m % 100 == 0
            && m - i64::from(board.money) >= 500
        {
            board.money = m as i32;
        }
        // The wave countdown pushed by the recorder to bring a wave forward
        // (`docs/re/dynamic-analysis.md`): it only ever counts down by one,
        // or jumps up to a new wave's start. A drop of more than 100 in one
        // tick is the recorder's write. Adopt it.
        if let Some(c) = t
            .snapshots
            .iter()
            .find(|s| s.tick == board.tick)
            .and_then(|s| s.state.get("waves"))
            .and_then(|w| w.get("countdown"))
            .and_then(Value::as_i64)
            && i64::from(board.wave_countdown) - c > 100
        {
            board.wave_countdown = c as i32;
        }
        // Re-sync only where the snapshot is a whole tick: a mid-frame click
        // (input applied at `next + 1` and marked dirty) means the capture's
        // RNG already includes that click's draws.
        if let Some(r) = resync.get(&board.tick)
            && !dirty.contains(&board.tick)
        {
            board.rng = r.clone();
            // The shifted draws also picked which shot-sound variants
            // (ZZAM, RICOCHET) played, and their 3-tick throttle carries
            // into the next ticks: take the capture's last-play clocks.
            if let Some(Value::Array(v)) = t
                .snapshots
                .iter()
                .find(|s| s.tick == board.tick)
                .and_then(|s| s.state.get("board")?.get("rng_samples"))
            {
                for (id, c) in [0x145, 0x132].into_iter().zip(v) {
                    if let Some(c) = c.as_i64() {
                        board.sample_last[id - 0x106] = c as i32;
                    }
                }
            }
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

/// Board inputs from a demo recorded alongside the capture
/// (`framework/demo-recording.md#using--record-alongside-a-ce-capture`).
///
/// A demo event at tick `T` (the update count `app+0x484`, which captures
/// record as `"u"`) is handled after update `T`, so it reaches the board
/// before update `T + 1`. `offset` is `u − tick` for the trace being replayed
/// (constant within a level when it's keyed by the board clock). The
/// snapshot at `u == T` may have been read before or after the event, so it
/// is marked dirty.
///
/// - Left-button down (single or double click) → [`Input::Click`]. The board
///   ignores it while a dialog is up.
/// - Left-button up → [`Input::Release`]: dialog buttons fire on
///   mouse-up, and a release with no dialog open does nothing. TODO: check
///   the board dialog's button rect (`dialog.md`); a release off the button
///   doesn't close the original's dialog.
/// - Right-button down / up → [`Input::RightClick`] / [`Input::RightRelease`]
///   (Presto, `pet-presto.md`).
/// - Focus loss → [`Input::SetPaused`]`(true)`; focus regained and keys →
///   nothing (the board has no keyboard handling).
/// - Moves, mouse enter/leave and recorded I/O don't touch the board.
///
/// Anything else in `from..=to` (middle clicks, minimising) is
/// returned in `unsupported`, so a replay can refuse it rather than drift.
pub fn demo_board_inputs(
    demo: &crate::demo::Demo,
    offset: i64,
    from: u32,
    to: u32,
) -> (Inferred, Vec<crate::demo::TimedEvent>) {
    demo_board_inputs_mapped(demo, |u| u32::try_from(u - offset).ok(), from, to)
}

/// [`demo_board_inputs`] with an explicit map from the demo's update count
/// to the replay's tick. The board clock skips an update now and then (a
/// widget update the board missed), so a fixed offset drifts by one; use
/// [`demo_tick_map`] for the capture's own `u` → board-clock pairs.
pub fn demo_board_inputs_mapped(
    demo: &crate::demo::Demo,
    map: impl Fn(i64) -> Option<u32>,
    from: u32,
    to: u32,
) -> (Inferred, Vec<crate::demo::TimedEvent>) {
    use crate::demo::{DemoEvent, MouseButton};
    let mut out = Inferred::default();
    let mut unsupported = Vec::new();
    for e in &demo.events {
        let Some(at) = map(i64::from(e.tick)) else {
            continue;
        };
        if !(from..to).contains(&at) {
            continue;
        }
        let input = match e.event {
            DemoEvent::MouseButton {
                button: MouseButton::Left,
                down,
                x,
                y,
                ..
            } => {
                if down {
                    Input::Click { x, y }
                } else {
                    Input::Release { x, y }
                }
            }
            DemoEvent::MouseButton {
                button: MouseButton::Right,
                down,
                x,
                y,
                ..
            } => {
                if down {
                    Input::RightClick { x, y }
                } else {
                    Input::RightRelease { x, y }
                }
            }
            // Focus loss pauses the board; regaining focus doesn't resume
            // it (the player clicks Resume) (`dynamic-analysis.md`).
            DemoEvent::Activate(false) => Input::SetPaused(true),
            DemoEvent::Activate(true) => continue,
            // The board has no keyboard handling (typing goes to dialogs and
            // screens, `pet-screen.md#input`).
            DemoEvent::KeyDown(_) | DemoEvent::KeyUp(_) | DemoEvent::Char(_) => continue,
            DemoEvent::MouseMove { .. }
            | DemoEvent::MouseEnter
            | DemoEvent::MouseLeave
            | DemoEvent::RegistryRead(_)
            | DemoEvent::RegistryWrite(_)
            | DemoEvent::RegistryQuery(_)
            | DemoEvent::FileRead(_)
            | DemoEvent::FileWrite(_) => continue,
            _ => {
                unsupported.push(e.clone());
                continue;
            }
        };
        out.inputs.entry(at + 1).or_default().push(input);
        if out.dirty.last() != Some(&at) {
            out.dirty.push(at);
        }
    }
    (out, unsupported)
}
