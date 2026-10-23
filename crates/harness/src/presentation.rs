//! Presentation comparison: what the player hears and which screen and
//! dialog are up, tick by tick (`docs/re/framework/presentation-capture.md`).
//!
//! Both sides are reduced to the same per-tick record:
//! - `sounds`: throttled samples started this tick (sample ids). From a
//!   capture, a board sample slot changing to the board clock; from the
//!   sim, its sound events.
//! - `music`: the last request (track, order) and, per track, whether it's
//!   playing and its volume (rounded, since the capture's fade ran on
//!   doubles in x87 and ours in SSE).
//! - `dialog` and `screen`.

use std::collections::BTreeMap;

use insaniquarium_sim::board::{Board, Dialog};
use insaniquarium_sim::music::MusicState;
use serde_json::{Value, json};

use crate::trace::{Snapshot, Trace};

/// Board dialog ids (`framework/dialog.md#board-dialogs`).
pub fn dialog_id(d: Dialog) -> i64 {
    match d {
        Dialog::AlienWarning | Dialog::BattleTip | Dialog::NewBreed => 0x0e,
        Dialog::LastFishDied => 0x10,
        Dialog::GameOver => 0x11,
        Dialog::OutOfMoney => 0x13,
    }
}

/// Volumes compared to 1/1000 (fades step by 0.002 or 0.008).
fn vol(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

/// Cooldown slots the sim's sounds stamped this step, the same thing the
/// original's sample table shows (`Sound::throttle_slot`).
pub fn sim_sounds(board: &Board) -> Vec<i64> {
    let mut v: Vec<i64> = board
        .sounds
        .iter()
        .filter_map(|p| p.sound.throttle_slot())
        .map(|s| s as i64)
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Our music state in the comparison shape: the tracks one can hear
/// (playing, volume above 0), with volume and fade. The capture can't see
/// whether a BASS channel plays, but stopping sets the volume to 0, so
/// "audible" is comparable on both sides.
pub fn sim_music(m: &MusicState) -> Value {
    let tracks: Vec<Value> = m
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.playing && t.volume > 0.0)
        .map(|(i, t)| json!({ "id": i, "vol": vol(t.volume), "fade": vol(t.fade) }))
        .collect();
    json!({ "req": [m.requested.0, m.requested.1], "tracks": tracks })
}

/// A sim presentation record.
pub fn sim_record(board: Option<&Board>, music: &MusicState, screen: &str) -> Value {
    json!({
        "sounds": board.map(sim_sounds).unwrap_or_default(),
        "music": sim_music(music),
        "dialog": board.and_then(|b| b.dialog).map(dialog_id),
        "screen": screen,
    })
}

/// Decode a capture's presentation records, keyed by tick. Needs the
/// board's `samples` table and `clock`, and the `pres` block.
pub fn capture_records(t: &Trace) -> BTreeMap<u32, Value> {
    let mut out = BTreeMap::new();
    let mut prev_samples: Option<Vec<i32>> = None;
    for s in &t.snapshots {
        let Some(pres) = s.state.get("pres") else {
            continue;
        };
        let board = s.state.get("board").filter(|b| !b.is_null());
        let samples = board
            .and_then(|b| b.get("samples"))
            .and_then(Value::as_str)
            .and_then(hex_i32s);
        let clock = board
            .and_then(|b| b.get("clock"))
            .and_then(Value::as_i64)
            .map(|c| c as i32);
        let mut sounds = Vec::new();
        if let (Some(now), Some(before), Some(c)) = (&samples, &prev_samples, clock) {
            for (i, (a, b)) in now.iter().zip(before).enumerate() {
                if a != b && (c - 1..=c).contains(a) {
                    sounds.push(0x106 + i as i64);
                }
            }
        }
        prev_samples = samples;
        let m = &pres["music"];
        let tracks: Vec<Value> = m["tracks"]
            .as_array()
            .map(|ts| {
                ts.iter()
                    .filter(|t| t["vol"].as_f64().unwrap_or(0.0) > 0.0)
                    .map(|t| {
                        json!({ "id": t["id"],
                                "vol": vol(t["vol"].as_f64().unwrap_or(0.0)),
                                "fade": vol(t["fade"].as_f64().unwrap_or(0.0)) })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let dialog = pres["dialogs"]
            .as_array()
            .and_then(|d| d.iter().filter_map(Value::as_i64).find(|&id| id < 0x20));
        let screens: Vec<&str> = pres["screens"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let screen = [
            "hatch",
            "interlude",
            "bonus",
            "pets",
            "selector",
            "help",
            "board",
        ]
        .into_iter()
        .find(|s| screens.contains(s))
        .unwrap_or("none");
        out.insert(
            s.tick,
            json!({
                "sounds": sounds,
                "music": { "req": [m["track"], m["order"]], "tracks": tracks },
                "dialog": dialog,
                "screen": screen,
            }),
        );
    }
    out
}

fn hex_i32s(h: &str) -> Option<Vec<i32>> {
    let b = (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    Some(
        b.as_chunks::<4>()
            .0
            .iter()
            .map(|c| i32::from_le_bytes(*c))
            .collect(),
    )
}

/// The first tick where `actual` differs from `expected`, with both
/// records. Ticks missing on either side are skipped.
pub fn first_mismatch(
    expected: &BTreeMap<u32, Value>,
    actual: &BTreeMap<u32, Value>,
) -> Option<(u32, Value, Value)> {
    expected.iter().find_map(|(t, e)| {
        let a = actual.get(t)?;
        (a != e).then(|| (*t, e.clone(), a.clone()))
    })
}

/// A sim trace's presentation records, from snapshots that carry `pres`.
pub fn sim_records(t: &Trace) -> BTreeMap<u32, Value> {
    t.snapshots
        .iter()
        .filter_map(|s: &Snapshot| Some((s.tick, s.state.get("pres")?.clone())))
        .collect()
}

/// Samples started per tick from a capture's sample table alone (captures
/// made before the `pres` block): sample `0x106 + i` started on tick t if
/// slot i changed since the previous snapshot. (A play during input handling
/// stamps the clock before that tick's update increments it, so the stamp
/// can be the clock or clock − 1.)
pub fn capture_sounds(t: &Trace) -> BTreeMap<u32, Vec<i64>> {
    let mut out = BTreeMap::new();
    let mut prev: Option<Vec<i32>> = None;
    for s in &t.snapshots {
        let board = s.state.get("board").filter(|b| !b.is_null());
        let now = board
            .and_then(|b| b.get("samples"))
            .and_then(Value::as_str)
            .and_then(hex_i32s);
        let clock = board.and_then(|b| b.get("clock")).and_then(Value::as_i64);
        if let (Some(n), Some(p), Some(c)) = (&now, &prev, clock) {
            let v: Vec<i64> = n
                .iter()
                .zip(p)
                .enumerate()
                .filter(|(_, (a, b))| a != b && (c - 1..=c).contains(&i64::from(**a)))
                .map(|(i, _)| 0x106 + i as i64)
                .collect();
            out.insert(s.tick, v);
        }
        prev = now;
    }
    out
}

/// Replay a board through `inputs` (as `replay::replay_resync_dirty` does,
/// without RNG re-syncs) and record music and dialogs per tick, with the
/// music state starting from the capture's first `pres` block. For checking
/// the music model and dialog flow against a capture's `pres` records.
pub fn replay_music(
    mut board: Board,
    inputs: &BTreeMap<u32, Vec<insaniquarium_sim::board::Input>>,
    last: u32,
    mut music: MusicState,
) -> BTreeMap<u32, Value> {
    let mut out = BTreeMap::new();
    while board.tick < last {
        let next = board.tick + 1;
        board.step(inputs.get(&next).map_or(&[][..], Vec::as_slice));
        music.cue(board.music, board.tank);
        music.update();
        out.insert(
            board.tick,
            json!({ "music": sim_music(&music),
                    "dialog": board.dialog.map(dialog_id) }),
        );
    }
    out
}

/// A capture's music state as a [`MusicState`] (to start a replay from).
pub fn music_from_capture(pres: &Value) -> MusicState {
    let mut m = MusicState::default();
    let m_req = &pres["music"];
    m.requested = (
        m_req["track"].as_i64().unwrap_or(-1) as i32,
        m_req["order"].as_i64().unwrap_or(0) as i32,
    );
    for t in m_req["tracks"].as_array().into_iter().flatten() {
        let Some(id) = t["id"].as_u64().filter(|&i| i < 5) else {
            continue;
        };
        let tr = &mut m.tracks[id as usize];
        tr.volume = t["vol"].as_f64().unwrap_or(0.0);
        tr.fade = t["fade"].as_f64().unwrap_or(0.0);
        tr.stop_flag = t["stop"].as_i64().unwrap_or(0) != 0;
        tr.playing = tr.volume > 0.0;
    }
    m
}

/// A capture's music and dialog records (the parts [`replay_music`] makes).
/// Captures from before the dialog map was found record no dialogs at all:
/// then `dialog` is left out and only music is compared.
pub fn capture_music(t: &Trace) -> BTreeMap<u32, Value> {
    let has_dialogs = t.snapshots.iter().any(|s| {
        s.state
            .get("pres")
            .and_then(|p| p["dialogs"].as_array())
            .is_some_and(|d| !d.is_empty())
    });
    capture_records(t)
        .into_iter()
        .map(|(k, v)| {
            let rec = if has_dialogs {
                json!({ "music": v["music"], "dialog": v["dialog"] })
            } else {
                json!({ "music": v["music"] })
            };
            (k, rec)
        })
        .collect()
}

/// [`first_mismatch`] comparing only the keys `expected` has.
pub fn first_mismatch_in(
    expected: &BTreeMap<u32, Value>,
    actual: &BTreeMap<u32, Value>,
) -> Option<(u32, Value, Value)> {
    expected.iter().find_map(|(t, e)| {
        let a = actual.get(t)?;
        let a = match (e, a) {
            (Value::Object(eo), Value::Object(ao)) => Value::Object(
                ao.iter()
                    .filter(|(k, _)| eo.contains_key(*k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
            _ => a.clone(),
        };
        (&a != e).then(|| (*t, e.clone(), a))
    })
}

/// Music cues as the original's board raised them, read from the capture's
/// own state (so a music check doesn't depend on replaying the whole board):
/// countdown 275 → alien song, countdown 1 → its fight section, the last
/// alien gone → tank song (`alien-sylvester.md#77-music`).
pub fn capture_cues(t: &Trace) -> BTreeMap<u32, insaniquarium_sim::sound::Music> {
    use insaniquarium_sim::sound::Music;
    let mut out = BTreeMap::new();
    let mut prev_aliens = 0usize;
    let mut prev_cd = None;
    for s in &t.snapshots {
        let Some(b) = s.state.get("board").filter(|b| !b.is_null()) else {
            continue;
        };
        let cd = b["wave"]["countdown"].as_i64();
        let aliens = s
            .state
            .get("raw")
            .and_then(|r| r.get("alien"))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if cd != prev_cd {
            match cd {
                Some(275) => {
                    out.insert(s.tick, Music::Alien);
                }
                Some(1) => {
                    out.insert(s.tick, Music::AlienFight);
                }
                _ => {}
            }
        }
        if prev_aliens > 0 && aliens == 0 {
            out.insert(s.tick, Music::Tank);
        }
        prev_aliens = aliens;
        prev_cd = cd;
    }
    out
}

/// Run the music model through `cues` from a capture's first music state,
/// one update per tick, and record it per tick.
pub fn replay_cues(
    t: &Trace,
    cues: &BTreeMap<u32, insaniquarium_sim::sound::Music>,
    tank: i32,
) -> BTreeMap<u32, Value> {
    let mut out = BTreeMap::new();
    let Some(first) = t.snapshots.first() else {
        return out;
    };
    let mut music = music_from_capture(&first.state["pres"]);
    for tick in first.tick + 1..=t.snapshots.last().map_or(0, |s| s.tick) {
        music.cue(cues.get(&tick).copied(), tank);
        music.update();
        out.insert(tick, json!({ "music": sim_music(&music) }));
    }
    out
}
