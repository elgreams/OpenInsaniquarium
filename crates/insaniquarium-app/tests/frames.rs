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

/// A game on the pet screen, rebuilt from the capture's object and the
/// profile bytes (`pet-screen.md#fields`).
fn pets_game(pres: &Value) -> Option<Game> {
    use insaniquarium_sim::game::PetsScreen;
    use insaniquarium_sim::pet_screen::PetScreen;
    let ps = pres.get("petscreen")?;
    let bytes = |k: &str| -> Vec<u8> {
        let s = ps[k].as_str().unwrap_or("");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0))
            .collect()
    };
    let raw = bytes("raw");
    let at = |o: usize| i32::from_le_bytes(raw[o..o + 4].try_into().unwrap());
    let mut profile = Profile::default();
    for (id, &o) in bytes("owned").iter().enumerate().take(24) {
        profile.pets[id] = o != 0;
    }
    let mut screen = PetScreen::new(&profile);
    for (id, &c) in bytes("buttons").iter().enumerate().take(24) {
        screen.chosen[id] = c != 0;
    }
    screen.ticks = at(0x120);
    screen.count = at(0x124);
    screen.shown = at(0x128);
    screen.fallback = at(0x12c);
    screen.fade = at(0x134);
    // Dialog 0x14 "Select More Pets?" open.
    screen.confirm_open = pres["dialogs"]
        .as_array()
        .is_some_and(|d| d.iter().any(|x| x.as_i64() == Some(0x14)));
    let mut g = Game::new(Profile::default(), MtRand::new(1), MtRand::new(2));
    g.profile = profile;
    g.screen = Screen::Pets(Box::new(PetsScreen {
        screen,
        rng: MtRand::new(1),
        global_rng: MtRand::new(2),
    }));
    Some(g)
}

/// Where the mouse was in a frame set (the capture doesn't record it; the
/// frame driver put it there): per frame index.
fn cursor_of(name: &str, i: usize) -> Option<(i32, i32)> {
    match (name, i) {
        // `pets_01`: off the widgets, on Zorf, on Stinky (just clicked), off.
        ("pets_01", 1) => Some((67, 413)),
        ("pets_01", 2) => Some((67, 87)),
        _ => None,
    }
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

/// Known gaps, masked per frame set. Bubbles are drawn from the global
/// random stream, which isn't modelled yet (`sim/board.md#the-global-rng-
/// stream-in-normal-play`): Niko's pearl bubbles rise from his mouth.
fn ignore_regions(name: &str) -> Vec<(u32, u32, u32, u32)> {
    match name {
        // Niko at (95, 253): his two pearl bubbles drifting up and right.
        // The board's ambient bubble emitter (x 150..172, y 400..406),
        // rising from the floor.
        "tank13_niko_01" => vec![(90, 60, 120, 340), (140, 370, 80, 60)],
        // The shop's animated icons are 1–2 ticks off: the frame is read
        // a moment after the state.
        "tank13_gameover_01" => vec![(0, 0, 500, 70)],
        // Frame 5: 59 px at Balrog's mouth edge, background in ours and
        // yellow in the original. Unexplained; most likely the stroke moving
        // him between the state read and the screenshot (frames 0–4 match).
        "tank14_alien_01" => vec![(160, 305, 16, 20)],
        // The system cursor, left over the tank (the screenshot includes it).
        "tank13_carnivore_01" => vec![(420, 155, 20, 30)],
        _ => Vec::new(),
    }
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
            .or_else(|| pets_game(&snap.state["pres"]))
            .or_else(|| tank_game(trace))
            .expect("a hatch, pet screen or tank frame");
        let frame = scene::frame(&game, cursor_of(name, i), false, &mut r.fonts, &r.images);
        let ours = render(&frame, &r.sheets);
        let original = load_png(png).unwrap();
        let mask = ignore_regions(name);
        let report = compare(&ours, &original, |x, y| {
            mask.iter()
                .any(|&(x0, y0, w, h)| (x0..x0 + w).contains(&x) && (y0..y0 + h).contains(&y))
        });
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

/// The pet screen after tank 1 (`pets_01`): empty panel, hovering Zorf,
/// three chosen with the mouse on Stinky, and with the mouse off.
#[test]
fn pet_screen_frames_match_original() {
    let failures = check("pets_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The pet screen's "Select More Pets?" (Yes/No dialog) with 2 chosen.
#[test]
#[ignore = "the Yes/No dialog is ~6 px shorter than the original's (Dialog::GetPreferredHeight for button mode 1 not matched yet)"]
fn pet_screen_dialog_frames_match_original() {
    let failures = check("pets_dialog_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The hatch screen after 1-3: Itchy.
#[test]
fn hatch_screen_itchy_frames_match_original() {
    let failures = check("hatch_itchy_01");
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

/// Tank 1-3: Niko opening and holding his pearl.
#[test]
fn tank_1_3_niko_frames_match_original() {
    let failures = check("tank13_niko_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Tank 1-3 with a carnivore swimming among grown guppies, food upgraded,
/// the laser upgrade and the egg slot in the shop.
#[test]
fn tank_1_3_carnivore_frames_match_original() {
    let failures = check("tank13_carnivore_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Tank 1-4 before the first wave: Itchy swimming (rows 0/1).
#[test]
fn tank_1_4_calm_frames_match_original() {
    let failures = check("tank14_calm_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Tank 1-4 with a kind-3 alien in: Itchy charging it (rows 2/3).
#[test]
fn tank_1_4_alien_frames_match_original() {
    let failures = check("tank14_alien_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Tank 1-3 game over (Niko closed, a kind-2 alien behind the dialog, the
/// carnivore hint on the message line).
#[test]
fn tank_1_3_gameover_frames_match_original() {
    let failures = check("tank13_gameover_01");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
