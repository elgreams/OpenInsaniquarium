//! Frame comparison against the original: rebuild the state of each
//! captured frame (`tools/frame_capture.lua`, `traces/frames/<name>/`),
//! render it on the CPU and compare with the original's screenshot of the
//! same tick (`framediff`). Writes ours | original | diff sheets to
//! `target/frames/<name>/` for looking at. Skips without data or frames.

use std::path::{Path, PathBuf};

use insaniquarium_app::assets::GameImages;
use insaniquarium_app::framediff::{compare, load_png, save_png, sheet};
use insaniquarium_app::render::render;
use insaniquarium_app::scene::{self, Fonts};
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

/// (state, screenshot) pairs in a frame directory, in order.
fn frames(dir: &Path) -> Vec<(Value, PathBuf)> {
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
            let state = png.with_extension("state.jsonl");
            let text = std::fs::read_to_string(state).ok()?;
            let snap: Value = serde_json::from_str(text.lines().nth(1)?).ok()?;
            Some((snap, png))
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

#[test]
fn hatch_screen_frames_match_original() {
    let (Some(data), dir) = (test_data_dir(), frames_dir("hatch_niko_01")) else {
        return;
    };
    let pairs = frames(&dir);
    if pairs.is_empty() {
        return;
    }
    let src = std::fs::read_to_string(data.join("properties/resources.xml")).unwrap();
    let manifest = Manifest::parse(&src).unwrap();
    let images = GameImages::load(&data, &manifest).unwrap();
    let sheets = scene::sheets(&images, &manifest);
    let mut fonts = Fonts::load(&data);
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/frames/hatch_niko_01");
    std::fs::create_dir_all(&out).unwrap();
    let mut failures = Vec::new();
    for (i, (snap, png)) in pairs.iter().enumerate() {
        let game = hatch_game(&snap["pres"]).expect("a hatch-screen frame");
        let ours = render(
            &scene::frame(&game, None, false, &mut fonts, &images),
            &sheets,
        );
        let original = load_png(png).unwrap();
        let report = compare(&ours, &original, |_, _| false);
        save_png(
            &sheet(&ours, &original, &report),
            &out.join(format!("{i:05}.png")),
        )
        .unwrap();
        eprintln!("frame {i} (tick {}): {}", snap["tick"], report.summary());
        if !report.matches() {
            failures.push(format!("frame {i}: {}", report.summary()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
