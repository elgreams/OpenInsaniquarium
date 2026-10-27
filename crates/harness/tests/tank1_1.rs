//! Sim vs original on tank 1-1: idle swimming and feeding. Food clicks are
//! recovered from the capture itself.
//! Uses captures in `traces/original/` (gitignored); skips when absent.

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
                    && (n.ends_with(".trace.jsonl") || n.ends_with(".trace.jsonl.zst"))
            })
        })
        // `Trace::open` takes the uncompressed name and finds either.
        .map(|p| {
            let s = p.to_string_lossy();
            PathBuf::from(s.strip_suffix(".zst").unwrap_or(&s).to_string())
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
        let capture = until_pause(&Trace::open(&path).unwrap().unwrap());
        let inferred = inferred_clicks(&capture);
        let expected = inferred.clean(&decode_capture(&capture));
        // All captures here are tank 1-1, the tutorial level.
        let mut board = board_from_capture(&capture).unwrap();
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
    let Some(t) = Trace::open(&path) else { return };
    let capture = t.unwrap();
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
    let mut board = board_from_capture(&capture).unwrap();
    board.tutorial = true;
    assert!(diff(&expected, &replay_from(board, &expected, &fewer).unwrap()).is_some());
}

#[test]
fn coin_collection_capture_is_covered() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_coin_04.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let capture = until_pause(&t.unwrap());
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

/// M3 exit test: a full tank 1-1 (level start → third egg piece) played on
/// the original, replayed from the level-start snapshot with all inputs
/// inferred from the capture.
#[test]
fn full_level_replays_exactly() {
    use harness::replay::{rebase_rng, window};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_full_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    // The run that completes the level: from the last unpause to the end.
    let paused = |s: &harness::trace::Snapshot| {
        s.state["board"]["paused"].as_bool().unwrap_or(true) || s.state["board"]["money"].is_null()
    };
    let last_running = full.snapshots.iter().rposition(|s| !paused(s)).unwrap();
    let start = full.snapshots[..last_running]
        .iter()
        .rposition(paused)
        .unwrap_or(0);
    let (from, to) = (
        full.snapshots[start].tick,
        full.snapshots[last_running].tick,
    );
    let mut t = window(&full, from, to);
    for stream in ["app", "global"] {
        t.header
            .rng
            .insert(stream.into(), rebase_rng(&full, stream, from).unwrap());
    }
    // Start the window on its paused first snapshot (nothing moves while
    // paused), then replay to the last running tick.
    if let Some(serde_json::Value::Object(b)) = t.snapshots[0].state.get_mut("board") {
        b.insert("paused".into(), serde_json::Value::Bool(false));
    }
    let inferred = inferred_clicks(&t);
    let expected = inferred.clean(&decode_capture(&t));
    let mut board = board_from_capture(&t).unwrap();
    board.tutorial = true;
    if board.guppies.is_empty() {
        // The window opens on the "last fish died" dialog.
        board.paused = true;
        board.dialog = Some(insaniquarium_sim::board::Dialog::LastFishDied);
        // Opened before the capture window: long past its button delay.
        board.dialog_age = insaniquarium_sim::board::DIALOG_BUTTON_DELAY;
    }
    let mut end_board = board.clone();
    let actual = replay_from(board, &expected, &inferred.inputs).unwrap();
    eprintln!(
        "full level: ticks {from}..={to} ({} snapshots), {} inputs, {} masked",
        expected.snapshots.len(),
        inferred.inputs.values().map(Vec::len).sum::<usize>(),
        inferred.dirty.len()
    );
    if let Some(d) = diff(&expected, &actual) {
        panic!("{d}");
    }
    // The original tears the board down for the hatch screen in the same
    // frame as the third egg purchase, so the capture's last snapshot shows
    // 2 pieces bought. Check our board, replayed to that point, completes
    // the level on the next egg click as the original did.
    while end_board.tick < to {
        let next = end_board.tick + 1;
        end_board.step(inferred.inputs.get(&next).map_or(&[][..], Vec::as_slice));
    }
    assert_eq!(end_board.egg_count, 3);
    end_board.step(&[insaniquarium_sim::board::Input::Click { x: 465, y: 33 }]);
    assert!(
        end_board.level_complete,
        "third egg piece completes tank 1-1"
    );
}

/// Level start: from the RNG state just before tank 1-1 is populated, our
/// `Board::start_level` places the same two guppies with the same state.
#[test]
fn level_start_matches_original() {
    use harness::replay::rebase_rng;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_full_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    // First snapshot of the level (board present, unpaused) and the one
    // before it (no board yet: the level was populated in between).
    let i = full
        .snapshots
        .iter()
        .position(|s| s.state["board"]["paused"].as_bool() == Some(false))
        .unwrap();
    let before = full.snapshots[i - 1].tick;
    let rng = |stream: &str| match rebase_rng(&full, stream, before).unwrap() {
        harness::trace::RngState::Full { mt, mti } => {
            sexy_core::rand::MtRand::from_state(mt.try_into().unwrap(), mti as usize)
        }
        harness::trace::RngState::Seed { seed } => sexy_core::rand::MtRand::new(seed),
    };
    let mut b = insaniquarium_sim::Board::start_level(rng("app"), rng("global"), 1, 1, &[]);
    // The capture's first level snapshot is after one board update.
    b.tick = before;
    b.update();
    let expected = decode_capture(&harness::replay::window(&full, b.tick, b.tick));
    let actual = harness::replay::snapshot(&b);
    let exp_fish = &expected.snapshots[0].state["fish"];
    assert_eq!(exp_fish, &actual.state["fish"], "starting guppies");
    assert_eq!(
        expected.snapshots[0].state["rng_mti"]["app"],
        actual.state["rng_mti"]["app"]
    );
}

/// Tank 1-2 start with Stinky: our `start_level` reproduces the original's
/// pet and starting guppies from the pre-populate RNG state.
#[test]
fn level_1_2_start_with_stinky() {
    use harness::replay::rebase_rng;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-2_full_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let i = full
        .snapshots
        .iter()
        .position(|s| s.state["board"]["paused"].as_bool() == Some(false))
        .unwrap();
    let before = full.snapshots[i - 1].tick;
    let rng = |stream: &str| match rebase_rng(&full, stream, before).unwrap() {
        harness::trace::RngState::Full { mt, mti } => {
            sexy_core::rand::MtRand::from_state(mt.try_into().unwrap(), mti as usize)
        }
        harness::trace::RngState::Seed { seed } => sexy_core::rand::MtRand::new(seed),
    };
    let mut b = insaniquarium_sim::Board::start_level(rng("app"), rng("global"), 1, 2, &[0]);
    b.tick = before;
    b.update();
    let expected = decode_capture(&harness::replay::window(&full, b.tick, b.tick));
    let actual = harness::replay::snapshot(&b);
    let e = &expected.snapshots[0].state;
    assert_eq!(e["pets"], actual.state["pets"], "Stinky");
    assert_eq!(e["fish"], actual.state["fish"], "starting guppies");
    assert_eq!(e["rng_mti"]["app"], actual.state["rng_mti"]["app"]);
}

/// Tank 1-4 start with Stinky, Niko and Itchy: the pets in id order, then
/// the guppies (pet-itchy.md#construction).
#[test]
fn level_1_4_start_with_itchy() {
    use harness::replay::rebase_rng;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-4_full_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let i = full
        .snapshots
        .iter()
        .position(|s| s.state["board"].is_object())
        .unwrap();
    let before = full.snapshots[i - 1].tick;
    let rng = |stream: &str| match rebase_rng(&full, stream, before).unwrap() {
        harness::trace::RngState::Full { mt, mti } => {
            sexy_core::rand::MtRand::from_state(mt.try_into().unwrap(), mti as usize)
        }
        harness::trace::RngState::Seed { seed } => sexy_core::rand::MtRand::new(seed),
    };
    // Started from the menu's Adventure button: the menu screen's own last
    // update draws a few app values in the same frame, before the Board
    // ctor (not modelled; level-start.md#from-the-menu). Skip to the Board
    // ctor, found by its three light-band draws (the first tick's lights).
    let lights = &full.snapshots[i].state["board"]["lights"];
    let b = (0..16)
        .find_map(|skip| {
            let mut app = rng("app");
            for _ in 0..skip {
                app.next_u31();
            }
            let mut b = insaniquarium_sim::Board::start_level(app, rng("global"), 1, 4, &[0, 1, 2]);
            b.tick = before;
            b.update();
            let got: Vec<f64> = b.light_x.iter().map(|&x| f64::from(x)).collect();
            let want: Vec<f64> = lights
                .as_array()?
                .iter()
                .filter_map(|v| v.as_f64())
                .collect();
            got.iter()
                .zip(&want)
                .all(|(g, w)| (g - w).abs() < 1e-3)
                .then_some(b)
        })
        .expect("the Board ctor's light draws");
    let expected = decode_capture(&harness::replay::window(&full, b.tick, b.tick));
    let actual = harness::replay::snapshot(&b);
    let e = &expected.snapshots[0].state;
    assert_eq!(e["pets"], actual.state["pets"], "Stinky and Niko");
    assert_eq!(e["fish_pets"], actual.state["fish_pets"], "Itchy");
    assert_eq!(e["fish"], actual.state["fish"], "starting guppies");
    assert_eq!(e["rng_mti"]["app"], actual.state["rng_mti"]["app"]);
}

/// Tank 1-2 with Stinky, from level start until the alien warning (the
/// wave countdown is checked separately once Sylvester is modelled).
#[test]
fn tank_1_2_until_alien_matches() {
    use harness::replay::{rebase_rng, window};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-2_full_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let i = full
        .snapshots
        .iter()
        .position(|s| s.state["board"]["paused"].as_bool() == Some(false))
        .unwrap();
    let from = full.snapshots[i].tick;
    // Stop before the warning (countdown 275 draws spawn positions).
    let to = full.snapshots[i..]
        .iter()
        .find(|s| s.state["board"]["wave"]["countdown"].as_i64() == Some(276))
        .map(|s| s.tick)
        .unwrap();
    // ...or the first catch-up burst, where snapshots tear.
    let to = harness::replay::first_burst(&full, from).map_or(to, |b| b.min(to));
    let mut t = window(&full, from, to);
    for stream in ["app", "global"] {
        t.header
            .rng
            .insert(stream.into(), rebase_rng(&full, stream, from).unwrap());
    }
    let inferred = inferred_clicks(&t);
    let expected = inferred.clean(&decode_capture(&t));
    let mut board = board_from_capture(&t).unwrap();
    board.tank = 1;
    board.level = 2;
    board.shop = insaniquarium_sim::shop::Shop::adventure(1, 2);
    let actual = replay_from(board, &expected, &inferred.inputs).unwrap();
    eprintln!(
        "1-2: ticks {from}..={to}, {} inputs, {} masked",
        inferred.inputs.values().map(Vec::len).sum::<usize>(),
        inferred.dirty.len()
    );
    if let Some(d) = diff(&expected, &actual) {
        panic!("{d}");
    }
}

/// Sylvester on tank 1-2: from just before the warning through the first
/// wave's kill, with the player's shots inferred from click flashes.
#[test]
fn sylvester_wave_matches() {
    use harness::replay::{first_burst, rebase_rng, window};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-2_full_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let cd = |s: &harness::trace::Snapshot| s.state["board"]["wave"]["countdown"].as_i64();
    // On 1-2 the countdown stops at 276 for the "DANGER!" hint dialog
    // (board paused). Dismissing it resumes the board mid-update: that tick
    // runs Board::Update (countdown 275, warning draws) but not the objects.
    // Start on that snapshot, where the state is consistent.
    // The capture straddles a catch-up burst right after the dialog closes
    // (one tick shows pets moved but not guppies, the next guppies moved
    // twice), so start two snapshots later.
    let warn = full
        .snapshots
        .iter()
        .position(|s| cd(s) == Some(275))
        .unwrap();
    let mut start = warn + 2;
    while let Some(b) = first_burst(&full, full.snapshots[start].tick) {
        if b > full.snapshots[warn].tick + 400 {
            break;
        }
        start = full.snapshots.iter().position(|s| s.tick > b).unwrap() + 1;
    }
    let from = full.snapshots[start].tick;
    let end = first_burst(&full, from).unwrap_or(u32::MAX);
    // Through the dead alien's debris bursts (until it is gone). The sparks
    // are a documented deviation (alien-sylvester.md#deviations): the app RNG
    // is re-synced after each burst tick.
    let gone = full.snapshots[start..]
        .iter()
        .skip_while(|s| s.state["raw"].get("dead_alien").is_none())
        .find(|s| s.state["raw"].get("dead_alien").is_none())
        .map_or(u32::MAX, |s| s.tick);
    let killed = gone;
    let to = end.min(killed);
    let mut t = window(&full, from, to);
    for stream in ["app", "global"] {
        t.header
            .rng
            .insert(stream.into(), rebase_rng(&full, stream, from).unwrap());
    }
    if let Some(serde_json::Value::Object(b)) = t.snapshots[0].state.get_mut("board") {
        b.insert("paused".into(), serde_json::Value::Bool(false));
    }
    let inferred = inferred_clicks(&t);
    let expected = inferred.clean(&decode_capture(&t));
    let expected = harness::replay::without_debris_shots(&expected, &t);
    let mut board = board_from_capture(&t).unwrap();
    board.tank = 1;
    board.level = 2;
    board.shop = insaniquarium_sim::shop::Shop::adventure(1, 2);
    board.paused = false;
    let resync = harness::replay::deviation_resyncs(&full, &harness::replay::debris_ticks(&t));
    let mut actual = harness::replay::replay_resync_dirty(
        board,
        &expected,
        &inferred.inputs,
        &resync,
        &inferred.dirty,
    )
    .unwrap();
    harness::replay::mask_rng_sounds(&mut actual, &expected);
    eprintln!("debris resyncs: {}", resync.len());
    eprintln!(
        "sylvester: ticks {from}..={to}, {} inputs, {} masked",
        inferred.inputs.values().map(Vec::len).sum::<usize>(),
        inferred.dirty.len()
    );
    if let Some(d) = diff(&expected, &actual) {
        panic!("{d}");
    }
}

/// Full tank 1-2 (Stinky, food upgrade, Sylvester waves, egg pieces) from
/// the level start, keyed on the logic counter. Skips if the capture is
/// absent. Deviations: dead-alien debris (RNG re-synced after each burst).
#[test]
fn full_tank_1_2_replays() {
    use harness::replay::{
        debris_ticks, deviation_resyncs, rebase_rng, window, without_debris_shots,
    };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-2_full_02.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let level = |s: &harness::trace::Snapshot| s.state["board"]["level"].as_i64();
    let i = full
        .snapshots
        .iter()
        .position(|s| level(s) == Some(2) && s.state["board"]["paused"].as_bool() == Some(false))
        .unwrap();
    let from = full.snapshots[i].tick;
    let to = full.snapshots[i..]
        .iter()
        .take_while(|s| level(s) == Some(2))
        .last()
        .unwrap()
        .tick;
    // Re-key by the board clock (one per board update, paused or not), so
    // updates the capture missed become gaps rather than misalignment.
    let raw = window(&full, from, to);
    let mut t = harness::replay::by_board_clock(&raw);
    for stream in ["app", "global"] {
        t.header
            .rng
            .insert(stream.into(), rebase_rng(&full, stream, from).unwrap());
    }
    // The RNG at each snapshot must come from the capture's own ticks:
    // rebase_rng walks `full`, so translate when re-syncing.
    let mut inferred = inferred_clicks(&t);
    inferred.dirty.extend(harness::replay::after_gaps(&t));
    let expected = without_debris_shots(&inferred.clean(&decode_capture(&t)), &t);
    let board = board_from_capture(&t).unwrap();
    let mut board = board;
    board.tank = 1;
    board.level = 2;
    board.shop = {
        let mut sh = insaniquarium_sim::shop::Shop::adventure(1, 2);
        sh.revealed = board.shop.revealed;
        sh
    };
    // Debris resyncs come from the original tick numbering.
    let tick_of = |clock_tick: u32| harness::replay::original_tick(&raw, clock_tick);
    let resync: std::collections::BTreeMap<u32, _> = debris_ticks(&t)
        .into_iter()
        .filter_map(|ct| {
            Some((
                ct,
                deviation_resyncs(&full, &[tick_of(ct)?])
                    .into_values()
                    .next()?,
            ))
        })
        .collect();
    let mut actual = harness::replay::replay_resync_dirty(
        board,
        &expected,
        &inferred.inputs,
        &resync,
        &inferred.dirty,
    )
    .unwrap();
    harness::replay::mask_rng_sounds(&mut actual, &expected);
    eprintln!(
        "full 1-2: ticks {from}..={to}, {} inputs, {} masked, {} resyncs",
        inferred.inputs.values().map(Vec::len).sum::<usize>(),
        inferred.dirty.len(),
        resync.len()
    );
    if let Some(d) = diff(&expected, &actual) {
        panic!("{d}");
    }
}

/// Tank 1-1 from its first tick to the hatch screen with the message line
/// recorded (`tank1-1_pres_01`): our `start_level` builds the board from
/// the pre-level RNG state, and the whole level replays with the inferred
/// inputs. Every tutorial message (welcome, first fish, grown, coin ×3,
/// egg pieces) must show with the same text and timer on the same tick.
#[test]
fn tank_1_1_from_start_with_messages() {
    use harness::replay::{rebase_rng, window};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_pres_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let board_of = |s: &harness::trace::Snapshot| s.state["board"].is_object();
    let i = full.snapshots.iter().position(board_of).unwrap();
    let before = full.snapshots[i - 1].tick;
    // Up to the last board snapshot (the next click opens the hatch screen).
    let last = full.snapshots[i..]
        .iter()
        .take_while(|s| board_of(s))
        .last()
        .unwrap()
        .tick;
    let rng = |stream: &str| match rebase_rng(&full, stream, before).unwrap() {
        harness::trace::RngState::Full { mt, mti } => {
            sexy_core::rand::MtRand::from_state(mt.try_into().unwrap(), mti as usize)
        }
        harness::trace::RngState::Seed { seed } => sexy_core::rand::MtRand::new(seed),
    };
    let mut board = insaniquarium_sim::Board::start_level(rng("app"), rng("global"), 1, 1, &[]);
    board.tick = before;
    let t = window(&full, full.snapshots[i].tick, last);
    let inferred = inferred_clicks(&t);
    let expected = inferred.clean(&decode_capture(&t));
    let actual = replay_from(board, &expected, &inferred.inputs).unwrap();
    let msgs = expected
        .snapshots
        .windows(2)
        .filter(|w| w[0].state.get("message") != w[1].state.get("message"))
        .filter(|w| w[1].state["message"]["t"].as_i64() == Some(184))
        .count();
    eprintln!(
        "1-1 from start: ticks {}..={last}, {} inputs, {} masked, {msgs} messages shown",
        t.snapshots[0].tick,
        inferred.inputs.values().map(Vec::len).sum::<usize>(),
        inferred.dirty.len()
    );
    // (The welcome line is already up in the first snapshot.)
    assert!(msgs >= 6, "the capture shows the tutorial messages");
    if let Some(d) = diff(&expected, &actual) {
        panic!("{d}");
    }
}

/// The hatch screen after 1-1 (Stinky), from `tank1-1_pres_01`: from the
/// app RNG state when the screen opens, every tick's counter `t`, egg
/// jitter (two app draws per tick) and app `mti` match the original, and
/// the hatch song is the one requested. Covers `screens/hatch-screen.md`
/// §4 and §7.
#[test]
fn hatch_screen_after_1_1_matches() {
    use harness::replay::rebase_rng;
    use insaniquarium_sim::hatch::Hatch;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/tank1-1_pres_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let hatch: Vec<_> = full
        .snapshots
        .iter()
        .filter(|s| s.state["pres"]["hatch"].is_object())
        .collect();
    assert!(hatch.len() > 170, "the capture covers the hatch screen");
    // The screen is created during the last board tick's input; start from
    // the RNG state of the snapshot before its first update.
    let first = hatch[0];
    let before = full
        .snapshots
        .iter()
        .rev()
        .find(|s| s.tick < first.tick)
        .unwrap()
        .tick;
    let mut rng = match rebase_rng(&full, "app", before).unwrap() {
        harness::trace::RngState::Full { mt, mti } => {
            sexy_core::rand::MtRand::from_state(mt.try_into().unwrap(), mti as usize)
        }
        harness::trace::RngState::Seed { seed } => sexy_core::rand::MtRand::new(seed),
    };
    let pet = first.state["pres"]["hatch"]["pet"].as_i64().unwrap() as i32;
    let mut h = Hatch::new(pet);
    let mut sound_at = None;
    let (mut prev_t, mut torn) = (None, 0);
    for s in &hatch {
        // Follow the screen's own update counter: the app counter can step
        // without a widget update (a skipped frame), so missing snapshots
        // aren't one update each.
        let t = s.state["pres"]["hatch"]["t"].as_i64().unwrap() as i32;
        while h.t < t {
            if h.update(&mut rng, false).sound {
                sound_at = Some(h.t);
            }
        }
        let e = &s.state["pres"]["hatch"];
        // A snapshot read while the screen was mid-update (the capture
        // reads the RNG and the screen at different moments) shows `t` and
        // the jitter after the update but `mti` before it, and repeats `t`
        // on the next snapshot. Skip those torn reads.
        let repeated = prev_t == Some(t);
        prev_t = Some(t);
        let next_repeats = hatch
            .iter()
            .find(|n| n.tick > s.tick)
            .is_some_and(|n| n.state["pres"]["hatch"]["t"].as_i64() == Some(t as i64));
        if repeated || next_repeats {
            torn += 1;
            continue;
        }
        let got = (h.t, h.jitter.0, h.jitter.1, rng.state().1 as i64);
        let want = (
            e["t"].as_i64().unwrap() as i32,
            e["jx"].as_i64().unwrap() as i32,
            e["jy"].as_i64().unwrap() as i32,
            s.state["rng_mti"]["app"].as_i64().unwrap(),
        );
        assert_eq!(got, want, "tick {} (t, jx, jy, app mti)", s.tick);
        let m = &s.state["pres"]["music"];
        assert_eq!(
            (m["track"].as_i64(), m["order"].as_i64()),
            (Some(2), Some(54))
        );
    }
    assert_eq!(sound_at, Some(insaniquarium_sim::hatch::SOUND_AT + 1));
    eprintln!(
        "hatch screen: {} ticks match ({torn} torn reads skipped)",
        hatch.len()
    );
    assert!(torn < hatch.len() / 50, "few torn reads");
}

/// The whole of tank 1-3 (`tank1-3_full_01`), level start to the last egg
/// piece: Stinky and Niko (pearls), four kind-2 alien waves, a carnivore
/// bought, starving and dying, food upgrades and three egg pieces.
#[test]
fn tank_1_3_from_start() {
    assert!(replay_1_3("tank1-3_full_01") > 300 || !captured("tank1-3_full_01"));
}

/// Tank 1-3 with a carnivore fed small guppies (`tank1-3_eat_01`): six
/// meals (hunt, bite, eat, gore), diamonds, waves.
#[test]
fn tank_1_3_carnivore_eats() {
    assert!(replay_1_3("tank1-3_eat_01") > 1000 || !captured("tank1-3_eat_01"));
}

/// Tank 1-4 (`tank1-4_full_01`): Stinky, Niko and Itchy, three kind-3
/// alien waves (Itchy stabs them 52 times); the tank is lost to the third.
#[test]
fn tank_1_4_from_start() {
    replay_tank_1("tank1-4_full_01", 4);
}

/// Tank 1-4 won (`tank1-4_full_02`): a carnivore bought to reveal the egg
/// piece, weapon upgrades to laser 4, food quantity 2, six Balrog waves
/// (Itchy, shots and the carnivore lost to one), three egg pieces.
#[test]
fn tank_1_4_won() {
    replay_tank_1("tank1-4_full_02", 4);
}

/// Tank 1-5 won (`tank1-5_full_01`): Stinky, Niko and Prego chosen on the
/// pet screen; Prego's births, Balrog waves and a two-alien wave (kind 9).
#[test]
fn tank_1_5_won() {
    replay_tank_1("tank1-5_full_01", 5);
}

/// 1-2 replayed from the demo recorded with it (`-record`): every input
/// comes from the original's own input log, none inferred.
#[test]
fn tank_1_2_from_demo() {
    if captured("tank1-2_demo_01") {
        replay_level("tank1-2_demo_01", 2, true);
    }
}

/// Pet zoo A (`zoo_a_01`): 1-2 with Clyde, Vert and Rufus (owned and
/// chosen by editing the profile in memory), inputs from the demo. The
/// first time the pets' specs (`pet-clyde.md`, `pet-vert.md`,
/// `pet-rufus.md`) meet the original.
#[test]
fn zoo_a_clyde_vert_rufus() {
    if captured("zoo_a_01") {
        replay_level("zoo_a_01", 2, true);
    }
}

/// Pet zoo B (`zoo_b_01`): 1-2 with Zorf, Meryl and Wadsworth, three
/// waves, inputs from the demo (`pet-zorf.md`, `pet-meryl.md`,
/// `pet-wadsworth.md`).
#[test]
fn zoo_b_zorf_meryl_wadsworth() {
    if captured("zoo_b_01") {
        replay_level("zoo_b_01", 2, true);
    }
}

/// Pet zoo C (`zoo_c_01`): 1-2 with Seymour, Shrapnel and Gumbo, inputs
/// from the demo; Shrapnel's bombs end it (`pet-seymour.md`,
/// `pet-shrapnel.md`, `pet-gumbo.md`).
#[test]
fn zoo_c_seymour_shrapnel_gumbo() {
    if captured("zoo_c_01") {
        replay_level("zoo_c_01", 2, true);
    }
}

/// Pet zoo D (`zoo_d_02`): 1-2 with Blip, Rhubarb and Nimbus, two waves,
/// food dropped low so Rhubarb snaps, coins left for Nimbus; inputs from
/// the demo (`pet-blip.md`, `pet-rhubarb.md`, `pet-nimbus.md`).
#[test]
fn zoo_d_blip_rhubarb_nimbus() {
    if captured("zoo_d_02") {
        replay_level("zoo_d_02", 2, true);
    }
}

/// Pet zoo E (`zoo_e_01`): 1-2 with Shrapnel, Gash and Angie: bombs make
/// corpses for Angie, Gash bites aliens and (once his clock fills) swallows
/// a guppy; four waves, inputs from the demo (`pet-gash.md`,
/// `pet-angie.md`). Gash's clock was pushed forward with Cheat Engine to
/// get a snack inside the capture; the replay adopts those writes.
#[test]
fn zoo_e_shrapnel_gash_angie() {
    if captured("zoo_e_01") {
        replay_level("zoo_e_01", 2, true);
    }
}

/// Gash's guppy snack (`gash_snack_01`): 1-2 with Gash alone; his hunger
/// clock pushed to 1450 with Cheat Engine early on, so he swallows a guppy
/// ~170 ticks later inside one capture (`pet-gash.md#without-aliens-guppy-snack`).
#[test]
fn gash_snack() {
    if captured("gash_snack_01") {
        replay_level("gash_snack_01", 2, true);
    }
}

/// Amp's zap (`amp_zap_01`): 1-2 with Amp; his charge pushed to 2900
/// with Cheat Engine, then three clicks on him (autoplay `--zap-amp`): every
/// guppy dies into a $200 coin and a free guppy drops in (`pet-amp.md`).
#[test]
fn amp_zap() {
    if captured("amp_zap_01") {
        replay_level("amp_zap_01", 2, true);
    }
}

/// The pet screen after 1-5 (`petscreen_01`): hover in and out, Continue
/// with nothing chosen and No, three picks, a refused 4th, an un-pick and
/// a re-pick, then Continue into 2-1 (`screens/pet-screen.md`). Inputs are
/// inferred from the screen's state; every field and the RNG (untouched)
/// are compared each tick, and 2-1 must start with the chosen pets.
#[test]
fn pet_screen_matches_original() {
    use insaniquarium_sim::pet_screen::{PetInput, PetOutcome, PetScreen};
    use insaniquarium_sim::profile::Profile;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/original/petscreen_01.trace.jsonl");
    let Some(t) = Trace::open(&path) else { return };
    let full = t.unwrap();
    let hex = |s: &serde_json::Value| -> Vec<u8> {
        let s = s.as_str().unwrap();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    };
    let i32_at = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    // (tick, shown, fallback, count, fade, ticks, dialog open, chosen bytes)
    #[allow(clippy::type_complexity)]
    let states: Vec<(u32, i32, i32, i32, i32, i32, bool, Vec<u8>)> = full
        .snapshots
        .iter()
        .filter_map(|s| {
            let p = s.state.get("pres")?;
            let ps = p.get("petscreen")?;
            let raw = hex(&ps["raw"]);
            let dialog = p["dialogs"].as_array().is_some_and(|d| !d.is_empty());
            Some((
                s.tick,
                i32_at(&raw, 0x128),
                i32_at(&raw, 0x12c),
                i32_at(&raw, 0x124),
                i32_at(&raw, 0x134),
                i32_at(&raw, 0x120),
                dialog,
                hex(&ps["chosen"]),
            ))
        })
        .collect();
    assert!(states.len() > 1000);
    let first = full
        .snapshots
        .iter()
        .find(|s| {
            s.state
                .get("pres")
                .and_then(|p| p.get("petscreen"))
                .is_some()
        })
        .unwrap();
    let owned = hex(&first.state["pres"]["petscreen"]["owned"]);
    let mut profile = Profile {
        tank: 2,
        level: 1,
        ..Profile::default()
    };
    for (id, &o) in owned.iter().enumerate() {
        profile.pets[id] = o != 0;
    }
    // The first snapshot is the freshly built screen (t = 0).
    let mut screen = PetScreen::new(&profile);
    let mut outcome = PetOutcome::Stay;
    for w in states.windows(2) {
        let (prev, now) = (&w[0], &w[1]);
        // Infer this tick's inputs from what changed.
        let mut inputs = Vec::new();
        if now.6 && !prev.6 {
            inputs.push(PetInput::Continue);
        }
        if prev.6 && !now.6 {
            inputs.push(PetInput::ConfirmNo);
        }
        if now.1 != prev.1 && now.1 >= 0 {
            inputs.push(PetInput::Enter(now.1));
        }
        if now.4 == 6 && prev.4 == 0 {
            inputs.push(PetInput::Leave(prev.1));
        }
        if now.3 != prev.3 {
            // The toggled pet is the one under the mouse (shown).
            inputs.push(PetInput::Toggle(now.1));
        }
        outcome = screen.step(&inputs, &mut profile);
        let got = (
            screen.shown,
            screen.fallback,
            screen.count,
            screen.fade,
            screen.ticks,
            screen.confirm_open,
        );
        assert_eq!(
            got,
            (now.1, now.2, now.3, now.4, now.5, now.6),
            "tick {} after {:?}",
            now.0,
            inputs
        );
    }
    // The last snapshot left on Continue with 3 chosen.
    let last = states.last().unwrap();
    assert_eq!(outcome, PetOutcome::Stay);
    assert_eq!(
        screen.step(&[PetInput::Continue], &mut profile),
        PetOutcome::StartLevel
    );
    let chosen: Vec<i32> = last
        .7
        .iter()
        .enumerate()
        .filter(|&(_, &c)| c != 0)
        .map(|(i, _)| i as i32)
        .collect();
    assert_eq!(profile.level_pets(), vec![0, 3, 4], "Stinky, Prego, Zorf");
    // The chosen bytes are written on Continue; the capture's last pet-screen
    // snapshot is before it, and the dialog's Continue wrote none chosen.
    assert!(chosen.is_empty() || chosen == vec![0, 3, 4]);
    // The RNG doesn't move on this screen.
    let mti: Vec<u64> = full
        .snapshots
        .iter()
        .filter(|s| {
            s.state
                .get("pres")
                .and_then(|p| p.get("petscreen"))
                .is_some()
        })
        .filter_map(|s| s.state["rng_mti"]["app"].as_u64())
        .collect();
    assert!(mti.windows(2).all(|w| w[0] == w[1]), "no app draws");
    // And 2-1 started with those pets: two OtherTypePets (Stinky, and no
    // Niko) plus Prego and Zorf as fish pets.
    let board = full
        .snapshots
        .iter()
        .find(|s| s.state["board"].is_object())
        .unwrap();
    let raw = board.state["raw"]
        .get("pet_other")
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);
    let fish = board.state["raw"]
        .get("pet_fish")
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);
    assert_eq!((raw, fish), (1, 2));
}

fn captured(name: &str) -> bool {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../traces/original/{name}.trace.jsonl"));
    p.exists() || p.with_extension("jsonl.zst").exists()
}

fn replay_1_3(name: &str) -> usize {
    replay_tank_1(name, 3)
}

static BIG_REPLAY: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Replay a whole tank-1 level `level` from its first board tick; returns
/// the number of ticks with a carnivore up.
fn replay_tank_1(name: &str, level: i32) -> usize {
    replay_level(name, level, false)
}

/// [`replay_tank_1`], with the inputs taken from the demo recorded in the
/// same session (`traces/demos/<name>.dmo`) instead of inferred from the
/// capture.
fn replay_level(name: &str, level: i32, from_demo: bool) -> usize {
    replay_level_chunk(name, level, from_demo, 0)
}

/// [`replay_level`] for the `chunk`-th run of a capture made of several
/// chunks joined together (chunks are separated by holes in the board
/// clock); chunk 0 starts at the level's first tick, later chunks start
/// mid-level from their first snapshot.
fn replay_level_chunk(name: &str, level: i32, from_demo: bool, chunk: usize) -> usize {
    use harness::replay::{
        debris_ticks, deviation_resyncs, rebase_rng, window, without_debris_shots,
    };
    // Whole-level replays hold several copies of a capture (several GB for
    // the long ones): run them one at a time.
    let _one_at_a_time = BIG_REPLAY.lock().unwrap_or_else(|e| e.into_inner());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../traces/original/{name}.trace.jsonl"));
    let Some(t) = Trace::open(&path) else {
        return 0;
    };
    let full = t.unwrap();
    let board_of = |s: &harness::trace::Snapshot| s.state["board"].is_object();
    // From the level's first board tick (a capture may open on the help
    // screen) to the last one. Keyed on the board clock, like the full 1-2
    // replay. A capture holding several levels replays the last one (the
    // board clock restarts at 1).
    let mut i = full
        .snapshots
        .iter()
        .rposition(|s| s.state["board"]["clock"].as_i64() == Some(1))
        .or_else(|| full.snapshots.iter().position(board_of))
        .unwrap();
    for _ in 0..chunk {
        let c = |k: usize| full.snapshots[k].state["board"]["clock"].as_i64();
        i = (i + 1..full.snapshots.len())
            .find(|&k| c(k).zip(c(k - 1)).is_some_and(|(b, a)| b - a > 50))
            .expect("no such chunk");
    }
    let from = full.snapshots[i].tick;
    // Up to the last board tick, or the first hole between joined capture
    // chunks (the board clock jumping by more than 50: nothing the replay
    // could follow is in it).
    let clock_of = |s: &harness::trace::Snapshot| s.state["board"]["clock"].as_i64();
    let to = full.snapshots[i..]
        .windows(2)
        .take_while(|w| {
            board_of(&w[1])
                && clock_of(&w[0])
                    .zip(clock_of(&w[1]))
                    .is_none_or(|(a, b)| b - a <= 50)
        })
        .last()
        .map_or(full.snapshots[i].tick, |w| w[1].tick);
    let raw = window(&full, from, to);
    // The demo's tick basis (`u`, app+0x484) against the board clock.
    let demo_offset = raw.snapshots[0]
        .state
        .get("u")
        .and_then(|u| u.as_i64())
        .map(|u| u - i64::from(from));
    let demo_map = from_demo.then(|| harness::replay::demo_tick_map(&raw));
    // The RNG at every debris tick, then the full capture can go: long
    // captures are several GB once parsed.
    let debris = debris_ticks(&harness::replay::by_board_clock(&raw));
    let tick_of = |clock_tick: u32| harness::replay::original_tick(&raw, clock_tick);
    let originals: Vec<u32> = debris.iter().filter_map(|&ct| tick_of(ct)).collect();
    let resync_orig = deviation_resyncs(&full, &originals);
    let rng0: Vec<_> = ["app", "global"]
        .iter()
        .map(|s| (s.to_string(), rebase_rng(&full, s, from).unwrap()))
        .collect();
    drop(full);
    let resync: std::collections::BTreeMap<u32, _> = debris
        .into_iter()
        .filter_map(|ct| Some((ct, resync_orig.get(&tick_of(ct)?)?.clone())))
        .collect();
    drop(resync_orig);
    let mut t = harness::replay::by_board_clock(&raw);
    drop(raw);
    t.header.rng.extend(rng0);
    let mut inferred = if from_demo {
        let demo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../traces/demos/{name}.dmo"));
        let demo = harness::demo::Demo::parse(&std::fs::read(demo_path).unwrap()).unwrap();
        let last = t.snapshots.last().unwrap().tick;
        assert!(demo_offset.is_some(), "capture has no update count (u)");
        let (mut inputs, unsupported) = harness::replay::demo_board_inputs_mapped(
            &demo,
            demo_map.as_ref().unwrap(),
            from,
            last,
        );
        assert!(
            unsupported.is_empty(),
            "demo events the replay can't feed: {unsupported:?}"
        );
        inputs.dirty.extend(harness::replay::torn_snapshots(&t));
        inputs
    } else {
        inferred_clicks(&t)
    };
    inferred.dirty.extend(harness::replay::after_gaps(&t));
    let expected = without_debris_shots(&inferred.clean(&decode_capture(&t)), &t);
    let mut board = board_from_capture(&t).unwrap();
    drop(t);
    board.shop = {
        let mut sh = insaniquarium_sim::shop::Shop::adventure(1, level);
        sh.revealed = board.shop.revealed;
        sh
    };
    let mut actual = harness::replay::replay_resync_dirty(
        board,
        &expected,
        &inferred.inputs,
        &resync,
        &inferred.dirty,
    )
    .unwrap();
    harness::replay::mask_rng_sounds(&mut actual, &expected);
    inferred.carry_dirty_sounds(&mut actual);
    eprintln!(
        "{name}: ticks {from}..={to}, {} inputs, {} masked, {} resyncs",
        inferred.inputs.values().map(Vec::len).sum::<usize>(),
        inferred.dirty.len(),
        resync.len()
    );
    // The comparison must cover the new objects, not pass vacuously.
    let carn_ticks = expected
        .snapshots
        .iter()
        .filter(|s| {
            s.state["carnivores"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
        })
        .count();
    let pearl_ticks = expected
        .snapshots
        .iter()
        .filter(|s| s.state["pearls"].as_array().is_some_and(|a| !a.is_empty()))
        .count();
    eprintln!(
        "{name}: {} snapshots compared; carnivore on {carn_ticks}, pearl on {pearl_ticks}",
        expected.snapshots.len()
    );
    let _ = pearl_ticks;
    // Report the first RNG divergence too: the alphabetical first field to
    // differ is often a symptom of it.
    let rng_only = |t: &Trace| Trace {
        header: t.header.clone(),
        snapshots: t
            .snapshots
            .iter()
            .map(|s| harness::trace::Snapshot {
                tick: s.tick,
                state: s
                    .state
                    .iter()
                    .filter(|(k, _)| *k == "rng_mti")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            })
            .collect(),
    };
    if let Some(d) = diff(&rng_only(&expected), &rng_only(&actual)) {
        eprintln!("{name}: first RNG divergence: {d}");
        if std::env::var_os("REPLAY_DUMP").is_some() {
            for t in d.tick - 2..=d.tick {
                let e = expected.snapshots.iter().find(|s| s.tick == t);
                let a = actual.snapshots.iter().find(|s| s.tick == t);
                eprintln!(
                    "--- {t}\nexpected {}\nactual   {}",
                    serde_json::to_string(&e.map(|s| &s.state)).unwrap(),
                    serde_json::to_string(&a.map(|s| &s.state)).unwrap()
                );
            }
        }
    }
    if let Some(d) = diff(&expected, &actual) {
        if std::env::var_os("REPLAY_DUMP_FIRST").is_some() {
            for t in d.tick - 2..=d.tick + 1 {
                eprintln!(
                    "inputs {t}: {:?} dirty {} resync {}",
                    inferred.inputs.get(&t),
                    inferred.dirty.contains(&t),
                    resync.contains_key(&t)
                );
            }
            for t in d.tick - 2..=d.tick {
                let e = expected.snapshots.iter().find(|s| s.tick == t);
                let a = actual.snapshots.iter().find(|s| s.tick == t);
                eprintln!(
                    "--- {t}\nexpected {}\nactual   {}",
                    serde_json::to_string(&e.map(|s| &s.state)).unwrap(),
                    serde_json::to_string(&a.map(|s| &s.state)).unwrap()
                );
            }
        }
        panic!("{name}: {d}");
    }
    carn_ticks
}
