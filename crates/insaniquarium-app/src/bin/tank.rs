//! Playable Adventure prototype (tank 1): runs `insaniquarium-sim` at the
//! original's 28 ms tick and draws its state. Click to drop food, collect
//! coins and shoot aliens; the last egg piece leads to the hatch screen and
//! on to the next level (progress lasts for the session).
//!
//! Usage: `cargo run --bin tank -- --data <install> [--level N] [--seed N] [--no-music] [--finish]`
//! (`--finish` puts the last egg piece up for sale, to reach the hatch screen.)
//!
//! What a frame shows comes from [`insaniquarium_app::scene`] (Bevy-free,
//! with the draw rules and their specs); this binary turns it into sprites.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioSource;
use bevy::audio::{AddAudioSource, Volume};
use bevy::image::Image;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::sprite_render::Material2dPlugin;
use bevy::window::PrimaryWindow;
use insaniquarium_app::additive::{Additive, AdditiveMaterial};
use insaniquarium_app::assets::{ALIEN_SONG, GameImages, TANK1_SONG, load_sound_bytes};
use insaniquarium_app::scene::{self, Cel, Fonts, HATCH_CONTINUE};
use insaniquarium_app::song::Song;
use insaniquarium_sim::Board;
use insaniquarium_sim::board::Input;
use insaniquarium_sim::game::{Game, HatchInput, Screen};
use insaniquarium_sim::profile::Profile;
use insaniquarium_sim::shop::Slot;
use insaniquarium_sim::sound::{Played, Sound};
use sexy_core::data_dir::DataDir;
use sexy_core::image::Rgba;
use sexy_core::music::Openmpt;
use sexy_core::rand::MtRand;
use sexy_core::resources::Manifest;

const W: f32 = 640.0;
const H: f32 = 480.0;
/// The original's logic tick (docs/re/framework/timing.md).
const TICK: Duration = Duration::from_millis(28);

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
    // Songs stream live through libopenmpt (`song.rs`); the sim decides
    // which track plays from which order at what volume.
    let lib = if args.iter().any(|a| a == "--no-music") {
        None
    } else {
        Openmpt::load()
            .inspect_err(|e| eprintln!("music disabled: {e}"))
            .ok()
    };
    let read =
        |path: &str| -> Option<Arc<[u8]>> { Some(std::fs::read(data.find(path)?).ok()?.into()) };
    let songs = Songs {
        bytes: [None, read(ALIEN_SONG.0), None, None, read(TANK1_SONG.0)],
        lib,
        players: [None; 5],
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
    // Session-only progress (TODO(M4): profiles and saves). `--level N`
    // starts at 1-N as if the earlier tank-1 levels were won.
    let level: i32 = arg("--level").and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut profile = Profile::default();
    for l in 1..level.clamp(1, 5) {
        profile.finish_level(1, l);
    }
    let mut game = Game::new(profile, app_rng, global);
    if args.iter().any(|a| a == "--finish") {
        // Dev shortcut: the last egg piece is up and affordable (buy it to
        // reach the hatch screen).
        if let Screen::Tank(b) = &mut game.screen {
            b.money = 10_000;
            b.egg_count = 3;
            b.shop.reveal(Slot::EggPiece, false);
        }
    }

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
        .add_plugins(Material2dPlugin::<AdditiveMaterial>::default())
        .add_audio_source::<Song>()
        .insert_resource(Time::<Fixed>::from_duration(TICK))
        .insert_resource(Sim {
            game,
            pending: Vec::new(),
            hatch_inputs: Vec::new(),
            hatch_sound: false,
        })
        .insert_resource(Sheets {
            table: scene::sheets(&images, &manifest),
            handles: HashMap::new(),
        })
        .insert_resource(Sources(images))
        .init_resource::<Cursor>()
        .insert_resource(SoundBytes(sounds))
        .insert_resource(songs)
        .insert_resource(Hud(Fonts::load(&data)))
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse, draw, title).chain())
        .add_systems(FixedUpdate, (step, play_sounds, play_music).chain())
        .run();
}

#[derive(Resource)]
struct Sim {
    game: Game,
    /// Clicks since the last tick, applied before it (like the original's
    /// message pump running between updates).
    pending: Vec<Input>,
    hatch_inputs: Vec<HatchInput>,
    /// The hatch screen asked for its sound this tick.
    hatch_sound: bool,
}

impl Sim {
    /// The tank, when it's the current screen.
    fn board(&self) -> Option<&Board> {
        self.game.board()
    }
}

#[derive(Resource)]
struct Sources(GameImages);

const ALL_SOUNDS: &[Sound] = &[
    Sound::Hatch,
    Sound::Bubbles,
    Sound::Awooga,
    Sound::Roar,
    Sound::Zap,
    Sound::Zzam,
    Sound::Ricochet,
    Sound::Hit,
    Sound::Chomp,
    Sound::Chomp2,
    Sound::Explode,
    Sound::Explosion1,
    Sound::Diamond,
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

/// Song players mirroring the sim's music state (`insaniquarium_sim::music`):
/// a track restarts by spawning a new player from its order.
#[derive(Resource)]
struct Songs {
    lib: Option<Arc<Openmpt>>,
    bytes: [Option<Arc<[u8]>>; 5],
    players: [Option<(Entity, u32)>; 5],
}

/// Uploaded audio, by sound id.
#[derive(Resource, Default)]
struct SoundHandles(HashMap<&'static str, Handle<AudioSource>>);

/// The sheets' cel size and grid, and their uploaded images.
#[derive(Resource)]
struct Sheets {
    table: scene::Sheets,
    handles: HashMap<&'static str, Handle<Image>>,
}

/// Fonts and the text they've rendered (`scene::Fonts`).
#[derive(Resource)]
struct Hud(Fonts);

/// Uploaded text images, keyed by the address of the cached `Arc<Rgba>`
/// they came from (the `Arc` is kept so the address isn't reused).
#[derive(Default)]
struct TextHandles {
    uploads: HashMap<usize, (Arc<Rgba>, Handle<Image>)>,
}

/// Mouse position in board pixels and whether the left button is held
/// (button hover/press art).
#[derive(Resource, Default)]
struct Cursor {
    pos: Option<(i32, i32)>,
    down: bool,
    /// The current press began on the hatch screen itself (not a button).
    hatch_press: bool,
}

/// The unit quad additive cels are drawn on.
#[derive(Resource)]
struct Quad(Handle<Mesh>);

/// An additive cel: a quad with its own material, kept and updated in
/// place from frame to frame (materials are GPU resources, too costly to
/// recreate every frame).
#[derive(Component)]
struct Pooled(Handle<AdditiveMaterial>);

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
    mut sheets: ResMut<Sheets>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera2d);
    commands.insert_resource(Quad(meshes.add(Rectangle::new(1.0, 1.0))));
    let Sheets { table, handles } = &mut *sheets;
    for (&id, sheet) in table.iter() {
        handles.insert(id, images.add(to_bevy(&sheet.image)));
    }
}

fn step(mut sim: ResMut<Sim>) {
    let inputs = std::mem::take(&mut sim.pending);
    let hatch_inputs = std::mem::take(&mut sim.hatch_inputs);
    sim.hatch_sound = false;
    match sim.game.screen {
        Screen::Tank(_) => sim.game.step_tank(&inputs),
        Screen::Hatch(_) => sim.hatch_sound = sim.game.step_hatch(&hatch_inputs).sound,
    }
}

fn play_sounds(
    mut commands: Commands,
    sim: Res<Sim>,
    bytes: Res<SoundBytes>,
    mut handles: Local<SoundHandles>,
    mut audio: ResMut<Assets<AudioSource>>,
) {
    let hatch = sim.hatch_sound.then_some(Played {
        sound: Sound::Hatch,
        volume: 1.0,
    });
    let sounds = sim.board().map_or(&[][..], |b| &b.sounds[..]);
    for p in sounds.iter().chain(&hatch) {
        let id = p.sound.id();
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
        commands.spawn((
            AudioPlayer::new(h),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(p.volume as f32)),
        ));
    }
}

/// Mirror the sim's music state onto song players: a playing track gets a
/// player started from its order (restarted when the sim restarts it) at
/// the track's volume; a stopped one loses its player.
fn play_music(
    mut commands: Commands,
    sim: Res<Sim>,
    mut songs: ResMut<Songs>,
    mut assets: ResMut<Assets<Song>>,
    mut sinks: Query<&mut AudioSink>,
) {
    let Some(lib) = songs.lib.clone() else { return };
    for (i, t) in sim.game.music.tracks.iter().enumerate() {
        let current = songs.players[i];
        if !t.playing {
            if let Some((e, _)) = songs.players[i].take() {
                commands.entity(e).despawn();
            }
            continue;
        }
        let vol = Volume::Linear(t.volume as f32);
        match current {
            Some((e, start)) if start == t.start => {
                if let Ok(mut sink) = sinks.get_mut(e) {
                    sink.set_volume(vol);
                }
            }
            _ => {
                if let Some((e, _)) = current {
                    commands.entity(e).despawn();
                }
                let Some(song) = songs.bytes[i]
                    .clone()
                    .and_then(|b| Song::new(&lib, b, t.order))
                else {
                    continue;
                };
                let e = commands
                    .spawn((
                        AudioPlayer(assets.add(song)),
                        PlaybackSettings::ONCE.with_volume(vol),
                    ))
                    .id();
                songs.players[i] = Some((e, t.start));
            }
        }
    }
}

fn mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    hud: Res<Hud>,
    mut cursor: ResMut<Cursor>,
    mut sim: ResMut<Sim>,
) {
    let Ok(w) = window.single() else { return };
    cursor.pos = w.cursor_position().map(|p| (p.x as i32, p.y as i32));
    cursor.down = buttons.pressed(MouseButton::Left);
    let Some((x, y)) = cursor.pos else { return };
    let Some(board) = sim.board() else {
        // Hatch screen: holding the button down on the screen skips the
        // egg; the Continue button acts on release (`hatch-screen.md#6`).
        // A widget is "down" only for a press that began on it: the press
        // that bought the last egg piece (still held as the screen opens)
        // doesn't skip anything.
        let (bx, by, bw, bh) = HATCH_CONTINUE;
        let on_button = (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y);
        if buttons.just_pressed(MouseButton::Left) {
            cursor.hatch_press = !on_button;
        } else if !buttons.pressed(MouseButton::Left) {
            cursor.hatch_press = false;
        }
        if on_button && buttons.just_released(MouseButton::Left) {
            sim.hatch_inputs.push(HatchInput::Continue);
        } else if cursor.hatch_press {
            sim.hatch_inputs.push(HatchInput::Held);
        }
        return;
    };
    if board.dialog.is_some() {
        // Only the dialog's button closes it, on release (`dialog.md`).
        // Before its image exists, any click does.
        let on_button = hud.0.dialog_button().is_none_or(|(bx, by, bw, bh)| {
            (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y)
        });
        if buttons.just_released(MouseButton::Left) && on_button {
            sim.pending.push(Input::DismissDialog);
        }
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        sim.pending.push(Input::Click { x, y });
    }
}

fn title(sim: Res<Sim>, mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut w) = window.single_mut() else {
        return;
    };
    let t = match sim.board() {
        Some(b) => format!("Insaniquarium — tank {}-{} — ${}", b.tank, b.level, b.money),
        None => "Insaniquarium".into(),
    };
    if w.title != t {
        w.title = t;
    }
}

/// The cel as a `ColorMaterial` on a unit quad: the UV transform picks
/// (and mirrors) the cel; the transform places and sizes the quad.
fn additive_parts(c: &Cel, sheets: &Sheets) -> Option<(ColorMaterial, Transform)> {
    let sheet = sheets.table.get(c.sheet)?;
    let handle = sheets.handles.get(c.sheet)?;
    sheet.cel_rect(c.col, c.row)?;
    let (gw, gh) = (sheet.grid.0 as f32, sheet.grid.1 as f32);
    let (u0, v0) = (c.col as f32 / gw, c.row as f32 / gh);
    let (su, sv) = (1.0 / gw, 1.0 / gh);
    let uv = if c.mirror {
        Affine2::from_cols(
            Vec2::new(-su, 0.0),
            Vec2::new(0.0, sv),
            Vec2::new(u0 + su, v0),
        )
    } else {
        Affine2::from_cols(Vec2::new(su, 0.0), Vec2::new(0.0, sv), Vec2::new(u0, v0))
    };
    let (w, h) = c
        .size
        .map_or((sheet.cel.0 as f32, sheet.cel.1 as f32), |(w, h)| {
            (w as f32, h as f32)
        });
    let [r, g, b] = c.tint;
    let mat = ColorMaterial {
        color: Color::srgba_u8(r, g, b, c.alpha),
        alpha_mode: bevy::sprite_render::AlphaMode2d::Blend,
        uv_transform: uv,
        texture: Some(handle.clone()),
    };
    let t = Transform::from_xyz(
        c.x as f32 + w / 2.0 - W / 2.0,
        H / 2.0 - c.y as f32 - h / 2.0,
        c.z,
    )
    .with_scale(Vec3::new(w, h, 1.0));
    Some((mat, t))
}

/// `DrawImageCel` semantics: out-of-range cels draw nothing.
fn cel_sprite(c: &Cel, sheets: &Sheets) -> Option<(Sprite, Anchor, Transform)> {
    let (cx, cy, w, h) = sheets.table.get(c.sheet)?.cel_rect(c.col, c.row)?;
    let handle = sheets.handles.get(c.sheet)?;
    let [r, g, b] = c.tint;
    Some((
        Sprite {
            image: handle.clone(),
            rect: Some(Rect::new(
                cx as f32,
                cy as f32,
                (cx + w) as f32,
                (cy + h) as f32,
            )),
            flip_x: c.mirror,
            color: Color::srgba_u8(r, g, b, c.alpha),
            custom_size: c.size.map(|(w, h)| Vec2::new(w as f32, h as f32)),
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(c.x as f32 - W / 2.0, H / 2.0 - c.y as f32, c.z),
    ))
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters.
fn draw(
    mut commands: Commands,
    sim: Res<Sim>,
    sheets: Res<Sheets>,
    src: Res<Sources>,
    mut hud: ResMut<Hud>,
    quad: Res<Quad>,
    cursor: Res<Cursor>,
    mut images: ResMut<Assets<Image>>,
    mut text_handles: Local<TextHandles>,
    mut materials: ResMut<Assets<AdditiveMaterial>>,
    old: Query<Entity, With<Drawn>>,
    mut pool: Query<(&Pooled, &mut Transform, &mut Visibility)>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let frame = scene::frame(&sim.game, cursor.pos, cursor.down, &mut hud.0, &src.0);
    let mut slots = pool.iter_mut();
    for c in &frame.cels {
        if !c.additive {
            if let Some(b) = cel_sprite(c, &sheets) {
                commands.spawn((Drawn, b.0, b.1, b.2));
            }
            continue;
        }
        let Some((mat, transform)) = additive_parts(c, &sheets) else {
            continue;
        };
        if let Some((p, mut tf, mut vis)) = slots.next() {
            if let Some(mut m) = materials.get_mut(&p.0) {
                m.base = mat;
            }
            *tf = transform;
            *vis = Visibility::Inherited;
        } else {
            let h = materials.add(AdditiveMaterial {
                base: mat,
                extension: Additive {},
            });
            commands.spawn((
                Pooled(h.clone()),
                Mesh2d(quad.0.clone()),
                MeshMaterial2d(h),
                transform,
            ));
        }
    }
    for (_, _, mut vis) in slots {
        vis.set_if_neq(Visibility::Hidden);
    }
    // Text images the font cache has dropped (old money values, a closed
    // dialog) are only held here now: free their uploads.
    text_handles
        .uploads
        .retain(|_, (img, _)| Arc::strong_count(img) > 1);
    for t in &frame.texts {
        let handle = text_handles
            .uploads
            .entry(Arc::as_ptr(&t.image) as usize)
            .or_insert_with(|| (t.image.clone(), images.add(to_bevy(&t.image))))
            .1
            .clone();
        let [r, g, b] = t.colour;
        commands.spawn((
            Drawn,
            Sprite {
                image: handle,
                color: Color::srgb_u8(r, g, b),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(t.x - W / 2.0, H / 2.0 - t.y, t.z),
        ));
    }
}
