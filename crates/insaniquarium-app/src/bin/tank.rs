//! Playable tank 1-1 prototype: runs `insaniquarium-sim` at the original's
//! 28 ms tick and draws its state. Click to drop food or collect coins.
//!
//! Usage: `cargo run --bin tank -- --data <install> [--seed N]`
//!
//! Draw rules come from the specs: guppies `fish-guppy.md#animation-and-drawing`,
//! food `food.md`, coins `coin.md`. Money is shown in the window title until
//! ImageFont text rendering moves out of the asset browser.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use insaniquarium_app::assets::GameImages;
use insaniquarium_sim::board::Input;
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
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse, draw, title).chain())
        .add_systems(FixedUpdate, step)
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

/// Uploaded sheets plus their cel size.
#[derive(Resource)]
struct Sheets(HashMap<&'static str, (Handle<Image>, UVec2)>);

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

fn setup(mut commands: Commands, src: Res<Sources>, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);
    let mut sheets = HashMap::new();
    for &id in SHEETS {
        let Some(img) = src.0.get(id) else {
            eprintln!("warning: missing {id}");
            continue;
        };
        // Sprite sheets here are 10 square-ish cels across.
        let cel = if id == "IMAGE_AQUARIUM1" {
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

fn mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut sim: ResMut<Sim>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(w) = window.single() else { return };
    if let Some(p) = w.cursor_position() {
        sim.pending.push(Input::Click {
            x: p.x as i32,
            y: p.y as i32,
        });
    }
}

fn title(sim: Res<Sim>, mut window: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut w) = window.single_mut() {
        let t = format!("Insaniquarium — tank 1-1 — ${}", sim.board.money);
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
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(self.x as f32 - W / 2.0, H / 2.0 - self.y as f32, self.z),
        ))
    }
}

/// Guppy sheet, cel, row and mirroring (fish-guppy.md#animation-and-drawing).
fn guppy_cel(f: &Fish) -> (&'static str, i32, i32, bool) {
    let hungry = f.looks_hungry();
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

fn draw(
    mut commands: Commands,
    sim: Res<Sim>,
    sheets: Res<Sheets>,
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
        });
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
        });
    }
    commands.spawn_batch(
        cels.iter()
            .filter_map(|c| c.bundle(&sheets))
            .collect::<Vec<_>>(),
    );
}
