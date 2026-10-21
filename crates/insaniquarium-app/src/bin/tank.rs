//! Playable tank 1-1 prototype: runs `insaniquarium-sim` at the original's
//! 28 ms tick and draws its state. Click to drop food or collect coins.
//!
//! Usage: `cargo run --bin tank -- --data <install> [--seed N] [--no-music]`
//!
//! Draw rules come from the specs: guppies `fish-guppy.md#animation-and-drawing`,
//! food `food.md`, coins `coin.md`. Money is shown in the window title until
//! ImageFont text rendering moves out of the asset browser.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioSource;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use insaniquarium_app::assets::{GameImages, TANK1_SONG, load_sound_bytes, render_song};
use insaniquarium_app::text::ImageFont;
use insaniquarium_sim::board::{Dialog, Input};
use insaniquarium_sim::sound::Sound;
use insaniquarium_sim::{Board, Fish};
use sexy_core::data_dir::DataDir;
use sexy_core::image::Rgba;
use sexy_core::rand::MtRand;
use sexy_core::resources::Manifest;

const W: f32 = 640.0;
const H: f32 = 480.0;
/// The original's logic tick (docs/re/framework/timing.md).
const TICK: Duration = Duration::from_millis(28);

const SHEETS: &[&str] = &[
    "IMAGE_AQUARIUM1",
    "IMAGE_SMALLSWIM",
    "IMAGE_SMALLTURN",
    "IMAGE_SMALLEAT",
    "IMAGE_HUNGRYSWIM",
    "IMAGE_HUNGRYTURN",
    "IMAGE_HUNGRYEAT",
    "IMAGE_FOOD",
    "IMAGE_MONEY",
    "IMAGE_MENUBAR",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
    };
    let data = match DataDir::resolve(arg("--data").map(PathBuf::from).as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let src =
        std::fs::read_to_string(data.join("properties/resources.xml")).expect("read resources.xml");
    let manifest = Manifest::parse(&src).expect("parse resources.xml");
    let images = GameImages::load(&data, &manifest).expect("load images");
    let sounds: HashMap<&'static str, Vec<u8>> = ALL_SOUNDS
        .iter()
        .filter_map(|s| Some((s.id(), load_sound_bytes(&data, &manifest, s.id())?)))
        .collect();
    // TODO: stream instead of pre-rendering; 5 minutes loops acceptably.
    let music = if args.iter().any(|a| a == "--no-music") {
        None
    } else {
        render_song(&data, TANK1_SONG.0, TANK1_SONG.1, 300)
            .inspect_err(|e| eprintln!("music disabled: {e}"))
            .ok()
    };

    // The sim never reads the clock; the app picks the seed, as the
    // original seeds its global RNG from GetTickCount.
    let seed = arg("--seed")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(4357, |d| d.subsec_nanos())
        });
    let mut global = MtRand::new(seed);
    let app_rng = MtRand::new(global.next_u31());
    let mut board = Board::new(app_rng, global);
    board.money = 200;
    board.tutorial = true;
    // TODO(spec): level-start sequence (board-update.md); tank 1-1 starts
    // with two guppies.
    board.add_random_guppy();
    board.add_random_guppy();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Insaniquarium".into(),
                        resolution: (W as u32, H as u32).into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(Time::<Fixed>::from_duration(TICK))
        .insert_resource(Sim {
            board,
            pending: Vec::new(),
        })
        .insert_resource(Sources(images))
        .insert_resource(SoundBytes(sounds))
        .insert_resource(Music(music))
        .insert_resource(Hud {
            font: ImageFont::load(&data, "data/ContinuumBold14.txt").ok(),
            dialog_font: ImageFont::load(&data, "data/JungleFever15outline.txt").ok(),
            money: None,
            dialog: None,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse, hud_text, draw, title).chain())
        .add_systems(FixedUpdate, (step, play_sounds).chain())
        .run();
}

#[derive(Resource)]
struct Sim {
    board: Board,
    /// Clicks since the last tick, applied before it (like the original's
    /// message pump running between updates).
    pending: Vec<Input>,
}

#[derive(Resource)]
struct Sources(GameImages);

const ALL_SOUNDS: &[Sound] = &[
    Sound::DropFood,
    Sound::Slurp,
    Sound::Slurp2,
    Sound::Slurp3,
    Sound::Points,
    Sound::Points2,
    Sound::Points3,
    Sound::Points4,
    Sound::Splash,
    Sound::Splash2,
    Sound::Splash3,
    Sound::Buy,
    Sound::Grow,
    Sound::Die,
    Sound::Buzzer,
];

#[derive(Resource)]
struct SoundBytes(HashMap<&'static str, Vec<u8>>);

#[derive(Resource)]
struct Music(Option<Vec<u8>>);

/// Uploaded audio, by sound id.
#[derive(Resource, Default)]
struct SoundHandles(HashMap<&'static str, Handle<AudioSource>>);

/// Uploaded sheets plus their cel size.
#[derive(Resource)]
struct Sheets(HashMap<&'static str, (Handle<Image>, UVec2)>);

/// HUD text, re-rasterised only when the value changes.
#[derive(Resource)]
struct Hud {
    font: Option<ImageFont>,
    dialog_font: Option<ImageFont>,
    money: Option<(i32, Handle<Image>, UVec2)>,
    dialog: Option<(Dialog, Handle<Image>, UVec2)>,
}

/// Per-frame sprites; rebuilt each frame from sim state.
#[derive(Component)]
struct Drawn;

fn to_bevy(img: &Rgba) -> Image {
    Image::new(
        Extent3d {
            width: img.width,
            height: img.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        img.as_bytes().to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn setup(
    mut commands: Commands,
    src: Res<Sources>,
    mut images: ResMut<Assets<Image>>,
    mut music: ResMut<Music>,
    mut audio: ResMut<Assets<AudioSource>>,
) {
    commands.spawn(Camera2d);
    if let Some(wav) = music.0.take() {
        let h = audio.add(AudioSource {
            bytes: Arc::from(wav),
        });
        commands.spawn((AudioPlayer::new(h), PlaybackSettings::LOOP));
    }
    let mut sheets = HashMap::new();
    for &id in SHEETS {
        let Some(img) = src.0.get(id) else {
            eprintln!("warning: missing {id}");
            continue;
        };
        // Sprite sheets here are 10 square-ish cels across.
        let cel = if matches!(id, "IMAGE_AQUARIUM1" | "IMAGE_MENUBAR") {
            UVec2::new(img.width, img.height)
        } else {
            let w = img.width / 10;
            UVec2::new(w, w.min(img.height))
        };
        sheets.insert(id, (images.add(to_bevy(img)), cel));
    }
    commands.insert_resource(Sheets(sheets));
}

fn step(mut sim: ResMut<Sim>) {
    let inputs = std::mem::take(&mut sim.pending);
    sim.board.step(&inputs);
}

fn play_sounds(
    mut commands: Commands,
    sim: Res<Sim>,
    bytes: Res<SoundBytes>,
    mut handles: Local<SoundHandles>,
    mut audio: ResMut<Assets<AudioSource>>,
) {
    for s in &sim.board.sounds {
        let id = s.id();
        let h = match handles.0.get(id) {
            Some(h) => h.clone(),
            None => {
                let Some(b) = bytes.0.get(id) else { continue };
                let h = audio.add(AudioSource {
                    bytes: Arc::from(b.clone()),
                });
                handles.0.insert(id, h.clone());
                h
            }
        };
        commands.spawn((AudioPlayer::new(h), PlaybackSettings::DESPAWN));
    }
}

fn mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut sim: ResMut<Sim>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(w) = window.single() else { return };
    if sim.board.dialog.is_some() {
        // TODO: draw the dialog; any click presses its button for now.
        sim.pending.push(Input::DismissDialog);
        return;
    }
    if let Some(p) = w.cursor_position() {
        sim.pending.push(Input::Click {
            x: p.x as i32,
            y: p.y as i32,
        });
    }
}

fn title(sim: Res<Sim>, mut window: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut w) = window.single_mut() {
        let note = match sim.board.dialog {
            Some(Dialog::LastFishDied) => " — Your last fish has died! Click to continue",
            Some(Dialog::GameOver) => " — Game over",
            None => "",
        };
        let t = format!("Insaniquarium — tank 1-1 — ${}{note}", sim.board.money);
        if w.title != t {
            w.title = t;
        }
    }
}

/// One cel to draw: column/row in `sheet`, top-left at game pixel (`x`, `y`).
/// Art faces left; `mirror` flips it per cel.
struct Cel<'a> {
    sheet: &'a str,
    col: i32,
    row: i32,
    x: i32,
    y: i32,
    z: f32,
    mirror: bool,
    /// 0..=255 opacity (colourised draw in the original).
    alpha: u8,
}

impl Cel<'_> {
    fn bundle(&self, sheets: &Sheets) -> Option<(Drawn, Sprite, Anchor, Transform)> {
        let (handle, size) = sheets.0.get(self.sheet)?;
        let cx = self.col.max(0) as u32 * size.x;
        let cy = self.row.max(0) as u32 * size.y;
        Some((
            Drawn,
            Sprite {
                image: handle.clone(),
                rect: Some(Rect::new(
                    cx as f32,
                    cy as f32,
                    (cx + size.x) as f32,
                    (cy + size.y) as f32,
                )),
                flip_x: self.mirror,
                color: Color::srgba_u8(255, 255, 255, self.alpha),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(self.x as f32 - W / 2.0, H / 2.0 - self.y as f32, self.z),
        ))
    }
}

/// Guppy sheet, cel, row and mirroring (fish-guppy.md#animation-and-drawing).
fn guppy_cel(f: &Fish) -> (&'static str, i32, i32, bool) {
    guppy_cel_as(f, f.looks_hungry())
}

/// As [`guppy_cel`], but with the hungry/fed sheet forced.
fn guppy_cel_as(f: &Fish, hungry: bool) -> (&'static str, i32, i32, bool) {
    let sheet = match (
        f.turn_timer != 0,
        f.eat_timer >= 1 || f.timer_108 >= 101,
        hungry,
    ) {
        (true, _, false) => "IMAGE_SMALLTURN",
        (true, _, true) => "IMAGE_HUNGRYTURN",
        (false, true, false) => "IMAGE_SMALLEAT",
        (false, true, true) => "IMAGE_HUNGRYEAT",
        (false, false, false) => "IMAGE_SMALLSWIM",
        (false, false, true) => "IMAGE_HUNGRYSWIM",
    };
    let row = match f.stage {
        3 => 2,
        4 => 3,
        s => s.clamp(0, 2),
    };
    let mirror = match f.turn_timer {
        t if t > 0 => true,
        t if t < 0 => false,
        _ => f.vx > 0.0 || (f.vx == 0.0 && f.facing > 0.0),
    };
    (sheet, f.cel, row, mirror)
}

fn hud_text(sim: Res<Sim>, mut hud: ResMut<Hud>, mut images: ResMut<Assets<Image>>) {
    let money = sim.board.money;
    if hud.money.as_ref().is_none_or(|m| m.0 != money)
        && let Some(f) = &hud.font
    {
        let img = f.render(money.to_string().as_bytes());
        let size = UVec2::new(img.width, img.height);
        hud.money = Some((money, images.add(to_bevy(&img)), size));
    }
    match sim.board.dialog {
        Some(d) if hud.dialog.as_ref().is_none_or(|x| x.0 != d) => {
            if let Some(f) = &hud.dialog_font {
                let text: &[u8] = match d {
                    Dialog::LastFishDied => b"YOUR LAST FISH HAS DIED! CLICK TO CONTINUE",
                    Dialog::GameOver => b"GAME OVER",
                };
                let img = f.render(text);
                let size = UVec2::new(img.width, img.height);
                hud.dialog = Some((d, images.add(to_bevy(&img)), size));
            }
        }
        None => hud.dialog = None,
        _ => {}
    }
}

fn text_sprite(
    handle: &Handle<Image>,
    x: f32,
    y: f32,
    z: f32,
) -> (Drawn, Sprite, Anchor, Transform) {
    (
        Drawn,
        Sprite::from_image(handle.clone()),
        Anchor::TOP_LEFT,
        Transform::from_xyz(x - W / 2.0, H / 2.0 - y, z),
    )
}

fn draw(
    mut commands: Commands,
    sim: Res<Sim>,
    sheets: Res<Sheets>,
    hud: Res<Hud>,
    old: Query<Entity, With<Drawn>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let b = &sim.board;
    let mut cels = vec![Cel {
        sheet: "IMAGE_AQUARIUM1",
        col: 0,
        row: 0,
        x: 0,
        y: 0,
        z: 0.0,
        mirror: false,
        alpha: 255,
    }];
    // Back to front as in Board::RebuildZOrder: food, guppies, coins.
    for f in &b.foods {
        cels.push(Cel {
            sheet: "IMAGE_FOOD",
            col: f.anim / f.anim_speed.max(1),
            row: f.level,
            x: f.widget_x - 5,
            y: f.widget_y - 4,
            z: 1.0,
            mirror: false,
            alpha: 255,
        });
    }
    for (i, f) in b.guppies.iter().enumerate() {
        let (sheet, col, row, mirror) = guppy_cel(f);
        cels.push(Cel {
            sheet,
            col,
            row,
            x: f.widget_x,
            y: f.widget_y,
            z: 2.0 + i as f32 * 0.001,
            mirror,
            alpha: 255,
        });
        // Hungry↔fed colour fade: the hungry sheet over the fed one at
        // fade·255/5 (fish-guppy.md#animation-and-drawing).
        if f.fade > 0 {
            let (sheet, col, row, mirror) = guppy_cel_as(f, true);
            cels.push(Cel {
                sheet,
                col,
                row,
                x: f.widget_x,
                y: f.widget_y,
                z: 2.0005 + i as f32 * 0.001,
                mirror,
                alpha: (f.fade.clamp(0, 5) * 255 / 5) as u8,
            });
        }
    }
    for (i, c) in b.coins.iter().enumerate() {
        // Silver row 0, gold row 1 (coin.md); other kinds not drawn yet.
        let row = match c.kind {
            1 | 8 => 0,
            2 | 9 => 1,
            _ => continue,
        };
        cels.push(Cel {
            sheet: "IMAGE_MONEY",
            col: c.cel,
            row,
            x: c.widget_x,
            y: c.widget_y,
            z: 3.0 + i as f32 * 0.001,
            mirror: false,
            alpha: 255,
        });
    }
    cels.push(Cel {
        sheet: "IMAGE_MENUBAR",
        col: 0,
        row: 0,
        x: 0,
        y: 0,
        z: 10.0,
        mirror: false,
        alpha: 255,
    });
    commands.spawn_batch(
        cels.iter()
            .filter_map(|c| c.bundle(&sheets))
            .collect::<Vec<_>>(),
    );
    // Money, right-aligned in the counter. TODO(spec): exact position/font
    // (shop-bar.md).
    if let Some((_, h, size)) = &hud.money {
        commands.spawn(text_sprite(h, 632.0 - size.x as f32, 38.0, 11.0));
    }
    if let Some((_, h, size)) = &hud.dialog {
        let x = (W - size.x as f32) / 2.0;
        commands.spawn(text_sprite(h, x, 230.0, 12.0));
    }
}
