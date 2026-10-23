//! Frame comparison against the original: rebuild the state of each
//! captured frame (`tools/frame_capture.lua`, `traces/frames/<name>/`),
//! render it on the CPU and compare with the original's screenshot of the
//! same tick (`framediff`). Writes ours | original | diff sheets to
//! `target/frames/<name>/` for looking at. Skips without data or frames.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use harness::replay::board_from_capture;
use harness::trace::Trace;
use insaniquarium_app::assets::GameImages;
use insaniquarium_app::framediff::{compare, load_png, save_png, sheet};
use insaniquarium_app::render::render;
use insaniquarium_app::scene::{self, Fonts, Sheets};
use insaniquarium_sim::board::Dialog;
use insaniquarium_sim::game::{Game, HatchScreen, Screen};
use insaniquarium_sim::hatch::Hatch;
use insaniquarium_sim::profile::Profile;
use serde_json::Value;
use sexy_core::data_dir::test_data_dir;
use sexy_core::rand::MtRand;
use sexy_core::resources::Manifest;

fn frames_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../traces/frames")
        .join(name)
}

/// (capture, screenshot) pairs in a frame directory, in order.
fn frames(dir: &Path) -> Vec<(Trace, PathBuf)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut pngs: Vec<PathBuf> = rd
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    pngs.sort();
    pngs.into_iter()
        .filter_map(|png| {
            let f = File::open(png.with_extension("state.jsonl")).ok()?;
            Some((Trace::read(BufReader::new(f)).ok()?, png))
        })
        .collect()
}

/// A game on the hatch screen as the capture shows it.
fn hatch_game(pres: &Value) -> Option<Game> {
    let h = pres.get("hatch")?;
    let pet = h["pet"].as_i64()? as i32;
    let mut hatch = Hatch::new(pet);
    hatch.t = h["t"].as_i64()? as i32;
    hatch.jitter = (h["jx"].as_i64()? as i32, h["jy"].as_i64()? as i32);
    let mut g = Game::new(Profile::default(), MtRand::new(1), MtRand::new(2));
    g.screen = Screen::Hatch(Box::new(HatchScreen {
        hatch,
        rng: MtRand::new(1),
        global_rng: MtRand::new(2),
    }));
    Some(g)
}

/// A game in the tank, rebuilt from the capture's board.
fn tank_game(t: &Trace) -> Option<Game> {
    let mut board = board_from_capture(t).ok()?;
    let s = &t.snapshots[0].state;
    board.paused = s["board"]["paused"].as_bool().unwrap_or(false);
    // Board dialogs up in this frame (`dialog.md`).
    board.dialog = s["pres"]["dialogs"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|d| match d.as_i64()? {
            // 0x0e is one of the hint dialogs; which one comes from the
            // hint bytes (the first unset one was just shown).
            0x0e => Some(
                if t.snapshots[0].state["board"]["hints"]
                    .as_str()
                    .is_some_and(|h| h.get(8..10) == Some("01"))
                {
                    Dialog::BattleTip
                } else {
                    Dialog::AlienWarning
                },
            ),
            0x10 => Some(Dialog::LastFishDied),
            0x11 => Some(Dialog::GameOver),
            0x13 => Some(Dialog::OutOfMoney),
            _ => None,
        });
    let mut g = Game::new(Profile::default(), MtRand::new(1), MtRand::new(2));
    g.screen = Screen::Tank(Box::new(board));
    Some(g)
}

struct Renderer {
    images: GameImages,
    sheets: Sheets,
    fonts: Fonts,
}

impl Renderer {
    fn load() -> Option<Self> {
        let data = test_data_dir()?;
        let src = std::fs::read_to_string(data.join("properties/resources.xml")).ok()?;
        let manifest = Manifest::parse(&src).ok()?;
        let images = GameImages::load(&data, &manifest).ok()?;
        let sheets = scene::sheets(&images, &manifest);
        let fonts = Fonts::load(&data);
        Some(Self {
            images,
            sheets,
            fonts,
        })
    }
}

/// Render every frame of `name`, compare, write the sheets, and return the
/// mismatching frames' summaries.
fn check(name: &str) -> Vec<String> {
    let pairs = frames(&frames_dir(name));
    let Some(mut r) = Renderer::load().filter(|_| !pairs.is_empty()) else {
        return Vec::new();
    };
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/frames")
        .join(name);
    std::fs::create_dir_all(&out).unwrap();
    let mut failures = Vec::new();
    let mut rows = Vec::new();
    for (i, (trace, png)) in pairs.iter().enumerate() {
        let snap = &trace.snapshots[0];
        let game = hatch_game(&snap.state["pres"])
            .or_else(|| tank_game(trace))
            .expect("a hatch or tank frame");
        let frame = scene::frame(&game, None, false, &mut r.fonts, &r.images);
        let ours = render(&frame, &r.sheets);
        let original = load_png(png).unwrap();
        let report = compare(&ours, &original, |_, _| false);
        save_png(
            &sheet(&ours, &original, &report),
            &out.join(format!("{i:05}.png")),
        )
        .unwrap();
        eprintln!(
            "{name} frame {i} (tick {}): {}",
            snap.tick,
            report.summary()
        );
        rows.push(format!(
            "{i:05}.png\t{}\t{}\t{}",
            snap.tick,
            report.matches(),
            report.summary()
        ));
        if !report.matches() {
            failures.push(format!("frame {i}: {}", report.summary()));
        }
    }
    // For the viewer (`tools/frame_report.sh` → target/frames/index.html).
    std::fs::write(out.join("summary.tsv"), rows.join("\n") + "\n").unwrap();
    failures
}

#[test]
fn hatch_screen_frames_match_original() {
    let failures = check("hatch_niko_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The first seconds of tank 1-2: two guppies, Stinky, the backdrop.
#[test]
fn tank_1_2_start_frames_match_original() {
    let failures = check("tank12_start_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Tank 1-2 left alone: the guppies starve, then GAME OVER is up.
#[test]
fn tank_1_2_idle_frames_match_original() {
    let failures = check("tank12_idle_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
