//! Playable Adventure prototype (tank 1): runs `insaniquarium-sim` at the
//! original's 28 ms tick and draws its state. Click to drop food, collect
//! coins and shoot aliens; the last egg piece leads to the hatch screen and
//! on to the next level (progress lasts for the session).
//!
//! Usage: `cargo run --bin tank -- --data <install> [--level N] [--seed N] [--no-music] [--finish]`
//! (`--finish` buys the last egg piece at once, to reach the hatch screen.)
//!
//! Draw rules come from the specs: guppies `fish-guppy.md#animation-and-drawing`,
//! food `food.md`, coins `coin.md`, Stinky `pet-stinky.md#drawing`, aliens,
//! warps, shots and corpses `alien-sylvester.md#7-drawing-and-sound-presentation`,
//! shadows `board.md#shadows`, dialogs `framework/dialog.md`, hatch screen
//! `screens/hatch-screen.md`.

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
use insaniquarium_app::assets::{ALIEN_SONG, GameImages, HATCH_SONG, TANK1_SONG, load_sound_bytes};
use insaniquarium_app::song::Song;
use insaniquarium_app::text::ImageFont;
use insaniquarium_sim::alien::{Alien, DeadAlien, Shot};
use insaniquarium_sim::board::{Dialog, Input};
use insaniquarium_sim::game::{Game, HatchInput, Screen};
use insaniquarium_sim::hatch::Hatch;
use insaniquarium_sim::pet::Pet;
use insaniquarium_sim::profile::Profile;
use insaniquarium_sim::shop::{BUTTON_X, BUTTON_Y, Slot};
use insaniquarium_sim::sound::{Music as MusicCue, Played, Sound};
use insaniquarium_sim::{Board, Fish};
use sexy_core::data_dir::DataDir;
use sexy_core::image::Rgba;
use sexy_core::music::Openmpt;
use sexy_core::rand::MtRand;
use sexy_core::resources::{Manifest, Resource};

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
    "IMAGE_MENUBTNU",
    "IMAGE_MENUBTND",
    "IMAGE_MENUBTNO",
    "IMAGE_MENUBUTTONREFLECTION",
    "IMAGE_EGGPIECES",
    "IMAGE_HELPARROW",
    "IMAGE_MONEYFLASH",
    "IMAGE_HATCHANIMATION",
    "IMAGE_SHADOW",
    "IMAGE_STINKY",
    "IMAGE_ZZZ",
    "IMAGE_SYLV",
    "IMAGE_WARPHOLE",
    "IMAGE_WARPGLOW",
    "IMAGE_LASERS",
    "IMAGE_CHOMP",
    "IMAGE_EXPLOSION",
    "IMAGE_EXPLOSIONSMALL",
    "IMAGE_EXPLOSIONTINY",
    "IMAGE_SMOKETINY",
    "IMAGE_SMOKESMALL",
    "IMAGE_SMOKEMISSILE",
    "IMAGE_HATCHSCREEN",
    "IMAGE_HATCHREFLECTION",
    "IMAGE_EGGCRACK1",
    "IMAGE_EGGCRACK2",
    "IMAGE_EGGSHARDS",
    "IMAGE_NIKO",
    "IMAGE_ITCHY",
    "IMAGE_PREGO",
    "IMAGE_ZORF",
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
    // Songs stream live through libopenmpt (`song.rs`).
    let music = if args.iter().any(|a| a == "--no-music") {
        None
    } else {
        Openmpt::load()
            .inspect_err(|e| eprintln!("music disabled: {e}"))
            .ok()
            .map(|lib| {
                let song = |(path, order): (&str, i32)| {
                    let bytes: Arc<[u8]> = std::fs::read(data.find(path)?).ok()?.into();
                    Song::new(&lib, bytes, order)
                };
                [song(TANK1_SONG), song(ALIEN_SONG), song(HATCH_SONG)]
            })
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
        // Dev shortcut: buy the last egg piece now (→ hatch screen).
        if let Screen::Tank(b) = &mut game.screen {
            b.money = 10_000;
            b.egg_count = 3;
            b.shop.reveal(Slot::EggPiece, false);
        }
        game.step_tank(&[Input::Buy(Slot::EggPiece)]);
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
        .insert_resource(Grid(
            SHEETS
                .iter()
                .filter_map(|&id| match manifest.get(id) {
                    Some(Resource::Image(i)) => Some((id, (i.cols.max(1), i.rows.max(1)))),
                    _ => None,
                })
                .collect(),
        ))
        .insert_resource(Sources(images))
        .init_resource::<Cursor>()
        .insert_resource(SoundBytes(sounds))
        .insert_resource(Music(music))
        .insert_resource(Hud {
            font: ImageFont::load(&data, "data/ContinuumBold12.txt").ok(),
            small: ImageFont::load(&data, "data/JungleFever10outline.txt").ok(),
            labels: HashMap::new(),
            dialog_fonts: [
                "data/JungleFever15outline.txt",
                "data/JungleFever10outline.txt",
                "data/JungleFever12outline.txt",
            ]
            .map(|p| ImageFont::load(&data, p).ok())
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok()),
            tiny: ImageFont::load(&data, "data/Pix118.txt").ok(),
            outline12: ImageFont::load(&data, "data/ContinuumBold12outline.txt").ok(),
            money: None,
            dialog: None,
            dialog_button: None,
            hatch_fonts: [
                "data/JungleFever17outline.txt",
                "data/JungleFever15outline.txt",
                "data/JungleFever12outline.txt",
                "data/JungleFever10outline.txt",
            ]
            .map(|p| ImageFont::load(&data, p).ok())
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok()),
            hatch: None,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse, hud_text, draw, title).chain())
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

/// Manifest (cols, rows) per sheet.
#[derive(Resource)]
struct Grid(HashMap<&'static str, (u32, u32)>);

const ALL_SOUNDS: &[Sound] = &[
    Sound::Hatch,
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

/// The tank, alien and hatch songs, if music is on (taken by `setup`).
#[derive(Resource)]
struct Music(Option<[Option<Song>; 3]>);

/// Song players (`music.md`). A song (re)starts by spawning a new player;
/// the tank and alien songs cross-fade linearly, 0.008 out and 0.002 in
/// per update.
#[derive(Resource)]
struct Songs {
    /// Tank, alien, hatch.
    handles: [Option<Handle<Song>>; 3],
    players: [Option<Entity>; 3],
    /// The screen the songs were last set up for (true = hatch screen).
    on_hatch: Option<bool>,
    /// Target: true = alien song.
    alien_on: bool,
    tank_vol: f32,
    alien_vol: f32,
}

impl Songs {
    /// Stop song `i` and start it again from its first order at `vol`.
    fn restart(&mut self, commands: &mut Commands, i: usize, vol: f32) {
        if let Some(e) = self.players[i].take() {
            commands.entity(e).despawn();
        }
        self.players[i] = self.handles[i].clone().map(|h| {
            commands
                .spawn((
                    AudioPlayer(h),
                    PlaybackSettings::ONCE.with_volume(Volume::Linear(vol)),
                ))
                .id()
        });
    }

    fn stop(&mut self, commands: &mut Commands, i: usize) {
        if let Some(e) = self.players[i].take() {
            commands.entity(e).despawn();
        }
    }
}

/// Uploaded audio, by sound id.
#[derive(Resource, Default)]
struct SoundHandles(HashMap<&'static str, Handle<AudioSource>>);

/// Uploaded sheets plus their cel size and grid.
#[derive(Resource)]
struct Sheets(HashMap<&'static str, (Handle<Image>, UVec2, UVec2)>);

/// HUD text, re-rasterised only when the value changes.
#[derive(Resource)]
struct Hud {
    font: Option<ImageFont>,
    /// Dialog header / body / button fonts (`framework/dialog.md`).
    dialog_fonts: Option<[ImageFont; 3]>,
    money: Option<(i32, Handle<Image>, UVec2)>,
    dialog: Option<(Dialog, Handle<Image>, UVec2)>,
    /// The dialog's button rect in screen pixels (x, y, w, h).
    dialog_button: Option<(i32, i32, i32, i32)>,
    /// Hatch-screen fonts: 17, 15, 12 and 10 outline.
    hatch_fonts: Option<[ImageFont; 4]>,
    /// Hatch screen for a pet: title bar with "You have found:", the name,
    /// the description, and the Continue button ("Please Wait..." /
    /// "Click Here to Continue"), each a full 640×480 overlay.
    hatch: Option<(i32, [Handle<Image>; 4])>,
    /// `FONT_TINY` for button prices.
    tiny: Option<ImageFont>,
    /// `FONT_CONTINUUMBOLD12OUTLINE` for button secondary text (the food
    /// quantity number).
    outline12: Option<ImageFont>,
    /// `FONT_JUNGLEFEVER10OUTLINE`: help-arrow captions and the tank label.
    small: Option<ImageFont>,
    labels: HashMap<String, (Handle<Image>, UVec2)>,
}

/// Mouse position in board pixels and whether the left button is held
/// (button hover/press art).
#[derive(Resource, Default)]
struct Cursor {
    pos: Option<(i32, i32)>,
    down: bool,
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
    src: Res<Sources>,
    grid: Res<Grid>,
    mut images: ResMut<Assets<Image>>,
    mut music: ResMut<Music>,
    mut song_assets: ResMut<Assets<Song>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera2d);
    commands.insert_resource(Quad(meshes.add(Rectangle::new(1.0, 1.0))));
    let handles = music.0.take().map_or([None, None, None], |s| {
        s.map(|s| s.map(|s| song_assets.add(s)))
    });
    commands.insert_resource(Songs {
        handles,
        players: [None; 3],
        on_hatch: None,
        alien_on: false,
        tank_vol: 1.0,
        alien_vol: 0.0,
    });
    let mut sheets = HashMap::new();
    for &id in SHEETS {
        let Some(img) = src.0.get(id) else {
            eprintln!("warning: missing {id}");
            continue;
        };
        // Fish/food/money sheets declare no grid in the manifest; the
        // game draws them as 80×80 (fish) / 40×40 / 72×72 cels, 10 across.
        let (cols, rows) = grid.0.get(id).copied().unwrap_or((1, 1));
        let ten_across = matches!(
            id,
            "IMAGE_SMALLSWIM"
                | "IMAGE_SMALLTURN"
                | "IMAGE_SMALLEAT"
                | "IMAGE_HUNGRYSWIM"
                | "IMAGE_HUNGRYTURN"
                | "IMAGE_HUNGRYEAT"
                | "IMAGE_FOOD"
                | "IMAGE_MONEY"
        );
        // Sheets whose grid the manifest omits: 17 warp frames, 20 smoke.
        let (cols, rows) = match id {
            "IMAGE_WARPHOLE" | "IMAGE_WARPGLOW" => (17, 1),
            "IMAGE_SMOKETINY" | "IMAGE_SMOKESMALL" | "IMAGE_SMOKEMISSILE" => (20, 1),
            "IMAGE_EXPLOSION" | "IMAGE_EXPLOSIONSMALL" | "IMAGE_EXPLOSIONTINY" => (10, 1),
            "IMAGE_CHOMP" => (5, 1),
            _ => (cols, rows),
        };
        let (cel, grid) = if ten_across {
            let w = img.width / 10;
            let h = w.min(img.height);
            (UVec2::new(w, h), UVec2::new(10, img.height / h))
        } else {
            (
                UVec2::new(img.width / cols, img.height / rows),
                UVec2::new(cols, rows),
            )
        };
        sheets.insert(id, (images.add(to_bevy(img)), cel, grid));
    }
    commands.insert_resource(Sheets(sheets));
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

/// Music by screen (`assets/music.md`): entering a screen stops all music
/// and starts its song (tank: `PlayTankMusic`; hatch: `0x0054b1a0(2, 54)`,
/// no fade). In the tank, `PlayAlienMusic` fades the tank song out (0.008
/// per update) and starts the alien song fading in (0.002);
/// `PlayTankMusic(1)` does the reverse.
fn play_music(
    mut commands: Commands,
    sim: Res<Sim>,
    mut songs: ResMut<Songs>,
    mut sinks: Query<&mut AudioSink>,
) {
    const TANK: usize = 0;
    const ALIEN: usize = 1;
    const HATCH: usize = 2;
    let on_hatch = sim.board().is_none();
    if songs.on_hatch != Some(on_hatch) {
        songs.on_hatch = Some(on_hatch);
        for i in [TANK, ALIEN, HATCH] {
            songs.stop(&mut commands, i);
        }
        songs.alien_on = false;
        songs.alien_vol = 0.0;
        if on_hatch {
            songs.restart(&mut commands, HATCH, 1.0);
        } else {
            songs.tank_vol = 1.0;
            songs.restart(&mut commands, TANK, 1.0);
        }
        return;
    }
    let Some(board) = sim.board() else { return };
    match board.music {
        Some(MusicCue::Alien) => {
            songs.alien_on = true;
            songs.alien_vol = 0.0;
            songs.restart(&mut commands, ALIEN, 0.0);
        }
        Some(MusicCue::Tank) => {
            songs.alien_on = false;
            songs.tank_vol = 0.0;
            songs.restart(&mut commands, TANK, 0.0);
        }
        None => {}
    }
    let on = songs.alien_on;
    let step = |v: f32, up: bool| {
        if up {
            (v + 0.002).min(1.0)
        } else {
            (v - 0.008).max(0.0)
        }
    };
    songs.tank_vol = step(songs.tank_vol, !on);
    songs.alien_vol = step(songs.alien_vol, on);
    // A song faded out to silence is stopped (`BassMusicInterface` slot 19).
    for (i, vol) in [(TANK, songs.tank_vol), (ALIEN, songs.alien_vol)] {
        if vol <= 0.0 && (i == ALIEN) != on {
            songs.stop(&mut commands, i);
        } else if let Some(mut sink) = songs.players[i].and_then(|e| sinks.get_mut(e).ok()) {
            sink.set_volume(Volume::Linear(vol));
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
        let (bx, by, bw, bh) = HATCH_CONTINUE;
        let on_button = (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y);
        if on_button && buttons.just_released(MouseButton::Left) {
            sim.hatch_inputs.push(HatchInput::Continue);
        } else if !on_button && buttons.pressed(MouseButton::Left) {
            sim.hatch_inputs.push(HatchInput::Held);
        }
        return;
    };
    if board.dialog.is_some() {
        // Only the dialog's button closes it, on release (`dialog.md`).
        // Before its image exists, any click does.
        let on_button = hud.dialog_button.is_none_or(|(bx, by, bw, bh)| {
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

/// One cel to draw: column/row in `sheet`, top-left at game pixel (`x`, `y`).
/// Art faces left; `mirror` flips it per cel.
#[derive(Clone)]
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
    /// Colourise tint (RGB); white = none.
    tint: [u8; 3],
    /// Draw mode 1 (additive).
    additive: bool,
    /// Destination size, if stretched (`DrawImageMirror` with a dest rect).
    size: Option<(i32, i32)>,
}

impl<'a> Cel<'a> {
    fn new(sheet: &'a str, col: i32, row: i32, x: i32, y: i32, z: f32) -> Self {
        Cel {
            sheet,
            col,
            row,
            x,
            y,
            z,
            mirror: false,
            alpha: 255,
            tint: [255; 3],
            additive: false,
            size: None,
        }
    }

    fn mirror(mut self, m: bool) -> Self {
        self.mirror = m;
        self
    }

    fn alpha(mut self, a: i32) -> Self {
        self.alpha = a.clamp(0, 255) as u8;
        self
    }

    fn tint(mut self, rgb: [u8; 3]) -> Self {
        self.tint = rgb;
        self
    }

    fn additive(mut self) -> Self {
        self.additive = true;
        self
    }

    /// The cel as a `ColorMaterial` on a unit quad: the UV transform picks
    /// (and mirrors) the cel; the transform places and sizes the quad.
    fn additive_parts(&self, sheets: &Sheets) -> Option<(ColorMaterial, Transform)> {
        let (handle, size, grid) = sheets.0.get(self.sheet)?;
        if self.col < 0 || self.row < 0 || self.col as u32 >= grid.x || self.row as u32 >= grid.y {
            return None;
        }
        let (gw, gh) = (grid.x as f32, grid.y as f32);
        let (u0, v0) = (self.col as f32 / gw, self.row as f32 / gh);
        let (su, sv) = (1.0 / gw, 1.0 / gh);
        let uv = if self.mirror {
            Affine2::from_cols(
                Vec2::new(-su, 0.0),
                Vec2::new(0.0, sv),
                Vec2::new(u0 + su, v0),
            )
        } else {
            Affine2::from_cols(Vec2::new(su, 0.0), Vec2::new(0.0, sv), Vec2::new(u0, v0))
        };
        let (w, h) = self.size.map_or((size.x as f32, size.y as f32), |(w, h)| {
            (w as f32, h as f32)
        });
        let [r, g, b] = self.tint;
        let mat = ColorMaterial {
            color: Color::srgba_u8(r, g, b, self.alpha),
            alpha_mode: bevy::sprite_render::AlphaMode2d::Blend,
            uv_transform: uv,
            texture: Some(handle.clone()),
        };
        let t = Transform::from_xyz(
            self.x as f32 + w / 2.0 - W / 2.0,
            H / 2.0 - self.y as f32 - h / 2.0,
            self.z,
        )
        .with_scale(Vec3::new(w, h, 1.0));
        Some((mat, t))
    }

    /// `DrawImageCel` semantics: out-of-range cels draw nothing.
    fn bundle(&self, sheets: &Sheets) -> Option<(Sprite, Anchor, Transform)> {
        let (handle, size, grid) = sheets.0.get(self.sheet)?;
        if self.col < 0 || self.row < 0 || self.col as u32 >= grid.x || self.row as u32 >= grid.y {
            return None;
        }
        let cx = self.col as u32 * size.x;
        let cy = self.row as u32 * size.y;
        let [r, g, b] = self.tint;
        Some((
            Sprite {
                image: handle.clone(),
                rect: Some(Rect::new(
                    cx as f32,
                    cy as f32,
                    (cx + size.x) as f32,
                    (cy + size.y) as f32,
                )),
                flip_x: self.mirror,
                color: Color::srgba_u8(r, g, b, self.alpha),
                custom_size: self.size.map(|(w, h)| Vec2::new(w as f32, h as f32)),
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

/// Shop buttons, help arrows and money flash (shop-bar.md#button-art).
fn shop_cels(b: &Board, cursor: &Cursor, cels: &mut Vec<Cel<'static>>) {
    // Button art by state (ButtonWidget::Draw): pressed (held on it) →
    // MENUBTND, hovered → MENUBTNO, else MENUBTNU. MAXed slots are
    // disabled and always draw the up image.
    let hovered = cursor
        .pos
        .and_then(|(x, y)| b.shop.hit(x, y))
        .filter(|&s| !maxed(b, s) && b.dialog.is_none());
    for slot in Slot::ALL {
        let i = slot as usize;
        let (Some(pos), true) = (b.shop.position[i], b.shop.revealed[i]) else {
            continue;
        };
        let (bx, by) = (BUTTON_X[pos], BUTTON_Y);
        let pop = b.shop.pop[i];
        if pop < 7 {
            let art = match (hovered == Some(slot), cursor.down) {
                (true, true) => "IMAGE_MENUBTND",
                (true, false) => "IMAGE_MENUBTNO",
                _ => "IMAGE_MENUBTNU",
            };
            cels.push(Cel::new(art, 0, 0, bx, by, 11.0));
            match slot {
                Slot::Guppy => cels.push(Cel::new(
                    "IMAGE_SMALLSWIM",
                    (b.level_ticks / 2) % 10,
                    0,
                    bx - 12,
                    by - 18,
                    11.1,
                )),
                // Food quality: the next pellet's row (`0x00540724`); at the
                // cap the button reads MAX and has no icon.
                Slot::FoodQuality if b.food_level < 2 => cels.push(Cel::new(
                    "IMAGE_FOOD",
                    0,
                    b.food_level + 1,
                    bx + 9,
                    by + 3,
                    11.1,
                )),
                Slot::EggPiece if (1..=3).contains(&b.egg_count) => cels.push(Cel::new(
                    "IMAGE_EGGPIECES",
                    b.egg_count - 1,
                    0,
                    bx + 7,
                    by + 3,
                    11.1,
                )),
                _ => {}
            }
        }
        if (1..=6).contains(&pop) {
            cels.push(Cel::new(
                "IMAGE_HATCHANIMATION",
                (6 - pop) / 2,
                0,
                bx,
                by,
                11.2,
            ));
        }
        cels.push(Cel::new("IMAGE_MENUBUTTONREFLECTION", 0, 0, bx, by, 11.3));
    }
    if b.money_flash > 0 && b.money_flash % 20 < 10 {
        cels.push(Cel::new("IMAGE_MONEYFLASH", 0, 0, 545, 39, 11.4));
    }
    for (x, _) in arrows(b) {
        cels.push(Cel::new("IMAGE_HELPARROW", 0, 0, x, 70, 11.5));
    }
}

/// Help arrows and their two caption lines, left x first (`0x0053f6a0`):
/// tutorial "buy fish"/"buy egg piece", and 1-2's food-quality hint.
/// A help arrow: its x and two caption lines (text, x, baseline y).
type Arrow = (i32, [(&'static str, i32, i32); 2]);

fn arrows(b: &Board) -> Vec<Arrow> {
    let mut v = Vec::new();
    if b.arrow_buy_fish {
        v.push((40, [("Click here to", 65, 95), ("buy fish!", 83, 107)]));
    } else if b.arrow_buy_egg {
        v.push((
            460,
            [("Click here to", 485, 95), ("buy egg piece!", 480, 107)],
        ));
    } else if b.arrow_food_quality && b.tank == 1 && b.level == 2 {
        v.push((110, [("Click here to", 135, 95), ("upgrade!", 152, 107)]));
    }
    v
}

/// Floor shadows (board.md#shadows): tank 1 colour (66, 48, 41).
fn shadow_cels(b: &Board, cels: &mut Vec<Cel<'static>>) {
    const TINT: [u8; 3] = [66, 48, 41];
    // Types 0 (fish) and 2 (alien) follow the owner's height and stretch.
    let mut moving = |ix: i32, iy: i32, kind2: bool| {
        let dark = (iy - 50) / 2;
        let x = ix + if kind2 { 40 } else { 0 };
        let y = 415 - ((370 - iy).max(0) * 30) / 370;
        let alpha = if kind2 {
            (2 * dark).clamp(0, 255)
        } else {
            dark.clamp(0, 150)
        };
        let mut f = ((y - iy) as f32 / 100.0).clamp(0.9, 1.3);
        if kind2 {
            f += 0.4;
        }
        let g = f - 1.0;
        let (dx, dy) = ((80.0 * g) as i32, (40.0 * g) as i32);
        let mut c = Cel::new("IMAGE_SHADOW", 0, 0, x - dx, y - dy, 0.5)
            .alpha(alpha)
            .tint(TINT);
        c.size = Some((80 + 2 * dx, 40 + 2 * dy));
        cels.push(c);
    };
    for f in &b.guppies {
        moving(f.widget_x, f.widget_y, false);
    }
    for a in &b.aliens {
        if a.warp < 10 {
            moving(a.widget_x, a.widget_y, true);
        }
    }
    // Type 1 (Stinky): fixed floor y, unstretched.
    for p in &b.pets {
        let dark = (p.widget_y - 50) / 2;
        cels.push(
            Cel::new("IMAGE_SHADOW", 0, 0, p.widget_x, p.shadow_y, 0.5)
                .alpha(dark.clamp(0, 150))
                .tint(TINT),
        );
    }
}

/// Stinky (pet-stinky.md#drawing).
fn pet_cels(p: &Pet, z: f32, cels: &mut Vec<Cel<'static>>) {
    let mirror = if p.shell > 0 {
        p.facing >= 0.0
    } else if p.turn != 0 {
        p.turn > 0
    } else {
        p.vx > 0.0 || (p.vx == 0.0 && p.facing > 0.0)
    };
    if p.slow {
        let zx = if mirror { 55 } else { 25 };
        cels.push(Cel::new(
            "IMAGE_ZZZ",
            0,
            0,
            p.widget_x + zx,
            p.widget_y - 14,
            z,
        ));
    }
    let row = if p.shell != 0 {
        2
    } else if p.turn != 0 {
        1
    } else {
        0
    };
    cels.push(
        Cel::new(
            "IMAGE_STINKY",
            p.cel,
            row,
            p.widget_x,
            p.widget_y,
            z + 0.0001,
        )
        .mirror(mirror),
    );
}

/// Sylvester (alien-sylvester.md#71-alien).
fn alien_cels(a: &Alien, z: f32, cels: &mut Vec<Cel<'static>>) {
    let (row, mirror) = if a.turn != 0 {
        (1, a.turn > 0)
    } else {
        (0, a.vx >= 0.0)
    };
    let base = Cel::new("IMAGE_SYLV", a.cel, row, a.widget_x, a.widget_y, z).mirror(mirror);
    if a.warp > 0 {
        if a.warp > 9 {
            return;
        }
        let s = (160.0 * a.warp as f32 / 10.0) as i32;
        let mut c = base;
        c.x += s / 2;
        c.y += s / 2;
        c.size = Some((160 - s, 160 - s));
        cels.push(c);
        return;
    }
    cels.push(base.clone());
    if a.cooldown > 0 {
        cels.push(
            Cel {
                z: z + 0.0001,
                ..base
            }
            .additive()
            .alpha(25 * a.cooldown),
        );
    }
}

/// Laser colours by level (`0x004d9160`, `0x004d9280`).
fn laser_colours(laser: i32) -> ([u8; 3], [u8; 3]) {
    let base = match laser {
        3 | 10 => [255, 0, 0],
        4 => [255, 55, 0],
        5 => [255, 200, 0],
        6 => [125, 255, 90],
        7 => [10, 200, 35],
        8 => [5, 150, 250],
        9 => [160, 0, 255],
        12 => [235, 0, 215],
        _ => [50, 180, 200],
    };
    let high = match laser {
        4 | 7 => [255, 255, 100],
        6 => [255, 255, 0],
        10 => [100, 255, 255],
        11 => [180, 225, 70],
        12 => [0, 255, 0],
        _ => [255, 255, 255],
    };
    (base, high)
}

/// Shots (alien-sylvester.md#73-shot). Nothing while the start delay runs.
fn shot_cels(s: &Shot, laser: i32, z: f32, cels: &mut Vec<Cel<'static>>) {
    if s.delay != 0 {
        return;
    }
    let (x, y) = (s.widget_x, s.widget_y);
    let one = |sheet| Cel::new(sheet, s.frame, 0, x, y, z).alpha(s.r);
    match s.kind {
        0 => {
            let pair = match laser {
                ..7 => 0,
                7..10 => 1,
                _ => 2,
            } * 2;
            let (base, high) = laser_colours(laser);
            let c = Cel::new("IMAGE_LASERS", s.frame, pair, x, y, z);
            cels.push(c.clone().tint(base).alpha(100));
            cels.push(
                Cel {
                    z: z + 0.0001,
                    ..c.clone()
                }
                .tint(base)
                .alpha(s.r)
                .additive(),
            );
            cels.push(
                Cel {
                    row: pair + 1,
                    z: z + 0.0002,
                    ..c
                }
                .tint(high)
                .alpha(s.r)
                .additive(),
            );
        }
        1 => cels.push(one("IMAGE_CHOMP")),
        3 => cels.push(one("IMAGE_EXPLOSION").additive()),
        4 => cels.push(one("IMAGE_EXPLOSIONSMALL").additive()),
        5 => cels.push(one("IMAGE_EXPLOSIONTINY")),
        6 => cels.push(one("IMAGE_SMOKETINY")),
        7 => cels.push(one("IMAGE_SMOKESMALL")),
        8 => cels.push(one("IMAGE_SMOKEMISSILE")),
        // Kind 2 (hit spark) draws nothing.
        _ => {}
    }
}

/// Sinking corpse (alien-sylvester.md#74-dead-alien).
fn dead_alien_cels(d: &DeadAlien, z: f32, cels: &mut Vec<Cel<'static>>) {
    let c = Cel::new("IMAGE_SYLV", d.cel, 0, d.widget_x, d.widget_y, z)
        .mirror(d.facing_right)
        .alpha((d.fade * 255.0) as i32);
    cels.push(c.clone());
    if d.life > 115 {
        // Five additive passes: a white flash fading over 10 ticks.
        let a = ((d.life - 115) * 20 * 5).min(255);
        cels.push(Cel { z: z + 0.0001, ..c }.additive().alpha(a));
    }
}

/// The food-quantity button's number: pellets after the next upgrade,
/// `min(qty + 1, 9)`; none at MAX (`shop-bar.md#button-art`). Keyed with
/// a `#` prefix in the label cache (it's a different font).
fn quantity_text(b: &Board) -> Option<String> {
    let i = Slot::FoodQuantity as usize;
    (b.shop.revealed[i] && !maxed(b, Slot::FoodQuantity))
        .then(|| format!("#{}", (b.max_food + 1).min(9)))
}

/// A slot at its cap: the button reads MAX and is disabled (`0x00534450`).
fn maxed(b: &Board, slot: Slot) -> bool {
    match slot {
        Slot::FoodQuality => b.food_level >= 2,
        Slot::FoodQuantity => b.max_food >= 9,
        _ => false,
    }
}

/// Button price text: `$n`, or MAX at a cap (`0x00534450`).
fn price_text(b: &Board, slot: Slot) -> String {
    let price = b.shop.price[slot as usize];
    if maxed(b, slot) {
        "MAX".into()
    } else if price > 0 {
        format!("${price}")
    } else {
        String::new()
    }
}

fn hud_text(
    sim: Res<Sim>,
    src: Res<Sources>,
    mut hud: ResMut<Hud>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Screen::Hatch(h) = &sim.game.screen {
        let hatch = &h.hatch;
        if hud.hatch.as_ref().is_none_or(|h| h.0 != hatch.pet)
            && let Some(fonts) = &hud.hatch_fonts
            && let Some(imgs) = render_hatch_text(&src.0, fonts, hatch.pet)
        {
            let [a, b, c, d] = imgs.map(|i| images.add(to_bevy(&i)));
            hud.hatch = Some((hatch.pet, [a, b, c, d]));
        }
        return;
    }
    let Some(b) = sim.board() else { return };
    if let Some(t) = quantity_text(b)
        && !hud.labels.contains_key(&t)
        && let Some(f) = &hud.outline12
    {
        let img = f.render(&t.as_bytes()[1..]);
        let size = UVec2::new(img.width, img.height);
        let h = images.add(to_bevy(&img));
        hud.labels.insert(t, (h, size));
    }
    let mut want: Vec<(String, bool)> = Slot::ALL
        .into_iter()
        .filter(|&s| b.shop.revealed[s as usize])
        .map(|s| (price_text(b, s), true))
        .collect();
    for (_, lines) in arrows(b) {
        for (t, _, _) in lines {
            want.push((t.to_string(), false));
        }
    }
    for (text, tiny) in want {
        if text.is_empty() || hud.labels.contains_key(&text) {
            continue;
        }
        let font = if tiny { &hud.tiny } else { &hud.small };
        if let Some(f) = font {
            let img = f.render(text.as_bytes());
            let size = UVec2::new(img.width, img.height);
            let h = images.add(to_bevy(&img));
            hud.labels.insert(text, (h, size));
        }
    }
    let money = b.money;
    if hud.money.as_ref().is_none_or(|m| m.0 != money)
        && let Some(f) = &hud.font
    {
        let img = f.render(money.to_string().as_bytes());
        let size = UVec2::new(img.width, img.height);
        hud.money = Some((money, images.add(to_bevy(&img)), size));
    }
    match b.dialog {
        Some(d) if hud.dialog.as_ref().is_none_or(|x| x.0 != d) => {
            if let Some((img, button)) = hud
                .dialog_fonts
                .as_ref()
                .and_then(|f| render_dialog(&src.0, f, d))
            {
                let size = UVec2::new(img.width, img.height);
                hud.dialog = Some((d, images.add(to_bevy(&img)), size));
                hud.dialog_button = Some(button);
            }
        }
        None => {
            hud.dialog = None;
            hud.dialog_button = None;
        }
        _ => {}
    }
}

/// Header and body of a board dialog (`framework/dialog.md#board-dialogs`).
fn dialog_text(d: Dialog) -> (&'static str, &'static str) {
    match d {
        Dialog::AlienWarning => (
            "DANGER!",
            "A vicious alien is about to enter your tank! Defeat it with your laser weapon by clicking on it!",
        ),
        Dialog::LastFishDied => (
            "YOUR LAST FISH HAS DIED!",
            "Normally this would end your game, but this time we'll give you another fish to keep playing! Make sure you keep it well fed!",
        ),
        Dialog::BattleTip => (
            "BATTLE TIP",
            "Shoot the alien's head to push it downwards! Shoot its tail to deflect it upwards!",
        ),
        Dialog::NewBreed => (
            "WARNING!",
            "A new breed of alien is fast approaching!  Lasers can't hurt this baddie, so find another way to defeat it!",
        ),
        Dialog::GameOver => ("GAME OVER", "Oops!  All of your fish have died!"),
        Dialog::OutOfMoney => (
            "OUT OF MONEY!",
            "You've run out of money!  Because you're new, we'll float you a loan, but be careful next time!",
        ),
    }
}

/// A `MoneyDialog` rendered to one image (`framework/dialog.md`): the panel
/// as a 3×3 box, the header, the wrapped body and the "Click to Continue"
/// button. Drawn at `DIALOG_POS`.
fn render_dialog(
    img: &GameImages,
    [head, body, btn]: &[ImageFont; 3],
    d: Dialog,
) -> Option<(Rgba, (i32, i32, i32, i32))> {
    let panel = img.get("IMAGE_DIALOG")?;
    let button = img.get("IMAGE_DIALOGBUTTON")?;
    let (header, text) = dialog_text(d);
    let (left, top, right, bottom) = (36, 15, 36, 36);
    let w = (head.width(header.as_bytes()) + 150).max(348);
    let inner = w - left - right;
    let lines = body.wrap(text, inner - 4);
    let bh = button.height as i32;
    // Dialog::GetPreferredHeight (`0x00476570`): insets, the header line,
    // 10 px, the body lines (each line spacing apart), 8 px, the button.
    let body_h = lines.len() as i32 * body.line_spacing();
    let h = top + head.line_spacing() + 10 + body_h + 8 + bh + bottom;
    let mut out = Rgba::new_transparent(w as u32, h as u32);
    out.draw_image_box(panel, (0, 0, w, h));
    let centred = |f: &ImageFont, t: &str| left + (inner - f.width(t.as_bytes())) / 2;
    let mut y = top + head.ascent();
    head.draw(
        &mut out,
        header.as_bytes(),
        centred(head, header),
        y,
        [255, 200, 0],
    );
    // Body baselines: the first sits one ascent below the header's line.
    y = top + head.line_spacing() + 10 + body.ascent();
    for l in &lines {
        body.draw(&mut out, l.as_bytes(), centred(body, l), y, [255, 255, 255]);
        y += body.line_spacing();
    }
    // The button: its normal-state third of DIALOGBUTTON as a 3×3 box.
    let normal = button.crop(0, 0, button.width / 3, button.height);
    let (bx, by, bw) = (left, h - bottom - bh, inner);
    out.draw_image_box(&normal, (bx, by, bw, bh));
    let label = "Click to Continue";
    btn.draw(
        &mut out,
        label.as_bytes(),
        bx + (bw - btn.width(label.as_bytes())) / 2,
        by + (bh + btn.ascent()) / 2 - 2,
        [255, 255, 100],
    );
    let (px, py) = (DIALOG_POS.0 as i32, DIALOG_POS.1 as i32);
    Some((out, (px + bx, py + by, bw, bh)))
}

/// `NewDialog` places board dialogs at (143, 142).
const DIALOG_POS: (f32, f32) = (143.0, 142.0);

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

#[allow(clippy::too_many_arguments)] // Bevy system parameters.
fn draw(
    mut commands: Commands,
    sim: Res<Sim>,
    sheets: Res<Sheets>,
    hud: Res<Hud>,
    quad: Res<Quad>,
    cursor: Res<Cursor>,
    mut materials: ResMut<Assets<AdditiveMaterial>>,
    old: Query<Entity, With<Drawn>>,
    mut pool: Query<(&Pooled, &mut Transform, &mut Visibility)>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let cels = match &sim.game.screen {
        Screen::Tank(b) => tank_cels(b, &cursor),
        Screen::Hatch(h) => hatch_cels(&h.hatch),
    };
    let mut slots = pool.iter_mut();
    for c in &cels {
        if !c.additive {
            if let Some(b) = c.bundle(&sheets) {
                commands.spawn((Drawn, b.0, b.1, b.2));
            }
            continue;
        }
        let Some((mat, transform)) = c.additive_parts(&sheets) else {
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
    match &sim.game.screen {
        Screen::Tank(b) => tank_text(&mut commands, b, &hud),
        Screen::Hatch(h) => hatch_text(&mut commands, &h.hatch, &hud),
    }
}

/// The tank: back to front as in `Board::RebuildZOrder` (board.md).
fn tank_cels(b: &Board, cursor: &Cursor) -> Vec<Cel<'static>> {
    let mut cels = vec![Cel::new("IMAGE_AQUARIUM1", 0, 0, 0, 0, 0.0)];
    // Back to front as in Board::RebuildZOrder (board.md): shadows, dead
    // aliens, warps, food, guppies, aliens, pets, shots, coins.
    shadow_cels(b, &mut cels);
    for (i, d) in b.dead_aliens.iter().enumerate() {
        dead_alien_cels(d, 0.6 + i as f32 * 0.001, &mut cels);
    }
    for (i, w) in b.warps.iter().enumerate() {
        let (f, z) = (w.frame(), 0.8 + i as f32 * 0.001);
        cels.push(Cel::new(
            "IMAGE_WARPHOLE",
            f,
            0,
            w.widget_x + 20,
            w.widget_y,
            z,
        ));
        cels.push(Cel::new("IMAGE_WARPGLOW", f, 0, w.widget_x, w.widget_y, z + 0.0001).additive());
    }
    for f in &b.foods {
        cels.push(Cel::new(
            "IMAGE_FOOD",
            f.anim / f.anim_speed.max(1),
            f.level,
            f.widget_x - 5,
            f.widget_y - 4,
            1.0,
        ));
    }
    for (i, f) in b.guppies.iter().enumerate() {
        let (sheet, col, row, mirror) = guppy_cel(f);
        let z = 2.0 + i as f32 * 0.001;
        cels.push(Cel::new(sheet, col, row, f.widget_x, f.widget_y, z).mirror(mirror));
        // Hungry↔fed colour fade: the hungry sheet over the fed one at
        // fade·255/5 (fish-guppy.md#animation-and-drawing).
        if f.fade > 0 {
            let (sheet, col, row, mirror) = guppy_cel_as(f, true);
            cels.push(
                Cel::new(sheet, col, row, f.widget_x, f.widget_y, z + 0.0005)
                    .mirror(mirror)
                    .alpha(f.fade.clamp(0, 5) * 255 / 5),
            );
        }
    }
    for (i, a) in b.aliens.iter().enumerate() {
        alien_cels(a, 2.5 + i as f32 * 0.001, &mut cels);
    }
    for (i, p) in b.pets.iter().enumerate() {
        pet_cels(p, 2.7 + i as f32 * 0.001, &mut cels);
    }
    for (i, s) in b.shots.iter().enumerate() {
        shot_cels(s, b.laser, 2.8 + i as f32 * 0.001, &mut cels);
    }
    for (i, c) in b.coins.iter().enumerate() {
        // `IMAGE_MONEY` rows (`0x004dd7a0`, normal game): silver 0, gold 1,
        // star 2, diamond 3. Other kinds not drawn yet.
        let row = match c.kind {
            1 | 8 => 0,
            2 | 9 => 1,
            3 | 10 => 2,
            4 | 5 | 11 | 12 => 3,
            _ => continue,
        };
        cels.push(Cel::new(
            "IMAGE_MONEY",
            c.cel,
            row,
            c.widget_x,
            c.widget_y,
            3.0 + i as f32 * 0.001,
        ));
    }
    cels.push(Cel::new("IMAGE_MENUBAR", 0, 0, 0, 0, 10.0));
    shop_cels(b, cursor, &mut cels);
    cels
}

fn tank_text(commands: &mut Commands, b: &Board, hud: &Hud) {
    // Money label: rect (535, 40, 80, h), right-aligned → right edge 615
    // (shop-bar.md#bar-money-counter-flash-help-arrows). The rendered
    // bitmap has a 4 px pad on each side.
    if let Some((_, h, size)) = &hud.money {
        let mut t = text_sprite(h, 615.0 + 4.0 - size.x as f32, 40.0 - 4.0, 11.6);
        t.1.color = Color::srgb_u8(180, 255, 90);
        commands.spawn(t);
    }
    // Button prices: FONT_TINY centred on the 58-wide button, baseline
    // ascent + 45 (shop-bar.md#button-art); shown once the pop-in ends.
    for slot in Slot::ALL {
        let i = slot as usize;
        let (Some(pos), true) = (b.shop.position[i], b.shop.revealed[i]) else {
            continue;
        };
        let text = price_text(b, slot);
        if let Some((h, size)) = hud.labels.get(&text) {
            let x = BUTTON_X[pos] as f32 + (58.0 - size.x as f32) / 2.0;
            let mut t = text_sprite(h, x, BUTTON_Y as f32 + 45.0 - 4.0, 11.25);
            t.1.color = Color::srgb_u8(110, 250, 110);
            commands.spawn(t);
        }
    }
    // Food quantity number: centred, baseline ascent + 9, (110, 250, 110),
    // drawn while the button isn't popping in.
    let qi = Slot::FoodQuantity as usize;
    if let (Some(t), Some(pos), true) = (quantity_text(b), b.shop.position[qi], b.shop.pop[qi] < 7)
        && let Some((h, size)) = hud.labels.get(&t)
    {
        let x = BUTTON_X[pos] as f32 + (58.0 - size.x as f32) / 2.0;
        // Baseline at ascent + 9 puts the line's top 9 px below the button
        // top; the bitmap has a 4 px pad.
        let y = (BUTTON_Y + 9) as f32 - 4.0;
        let mut t = text_sprite(h, x, y, 11.15);
        t.1.color = Color::srgb_u8(110, 250, 110);
        commands.spawn(t);
    }
    // Help-arrow captions (JungleFever10outline, (110, 250, 110)) at their
    // baselines; the bitmap's top is ascent above the baseline.
    for (_, lines) in arrows(b) {
        for (text, x, y) in lines {
            if let Some((h, _)) = hud.labels.get(text) {
                let mut t = text_sprite(h, x as f32 - 4.0, (y - 11) as f32 - 4.0, 11.55);
                t.1.color = Color::srgb_u8(110, 250, 110);
                commands.spawn(t);
            }
        }
    }
    if let Some((_, h, _)) = &hud.dialog {
        commands.spawn(text_sprite(h, DIALOG_POS.0, DIALOG_POS.1, 12.0));
    }
}

/// The hatch screen's Continue button rect (`hatch-screen.md#3`).
const HATCH_CONTINUE: (i32, i32, i32, i32) = (186, 445, 264, 29);

/// Name and description of a pet on the hatch screen (tank 1 for now).
/// TODO(spec): the other pets' text.
fn pet_text(pet: i32) -> (&'static str, &'static [&'static str]) {
    match pet {
        0 => (
            "STINKY the Snail",
            &[
                "STINKY roams around the",
                "bottom of your tank, catching",
                "any coins you may have missed.",
            ],
        ),
        1 => (
            "NIKO the Oyster",
            &[
                "NIKO produces pearls that",
                "you can click on for a",
                "hefty sum of money.",
            ],
        ),
        2 => (
            "ITCHY the Swordfish",
            &["ITCHY helps you by attacking", "aliens when they appear."],
        ),
        3 => (
            "PREGO the Momma Fish",
            &[
                "PREGO helps populate your",
                "tank by giving birth to a new",
                "baby guppy every so often.",
            ],
        ),
        4 => (
            "ZORF the Sea Horse",
            &["ZORF gives you a hand in", "keeping your fish fed."],
        ),
        _ => ("", &[]),
    }
}

/// The hatch screen's text layers, each 640×480 (`hatch-screen.md#5`):
/// [title bar + "You have found:", name, description + "saved", the
/// Continue button "Please Wait..." / "Click Here to Continue"].
/// Text x is centred as `(650 − width) / 2 − 3`; y is the baseline.
fn render_hatch_text(
    img: &GameImages,
    [f17, f15, f12, f10]: &[ImageFont; 4],
    pet: i32,
) -> Option<[Rgba; 4]> {
    let centre = |f: &ImageFont, t: &str| ((650 - f.width(t.as_bytes())) >> 1) - 3;
    let layer = || Rgba::new_transparent(W as u32, H as u32);
    let (name, lines) = pet_text(pet);
    let mut title = layer();
    if let Some(t) = img.get("IMAGE_SCREENTITLE") {
        title.draw_image_box(t, (20, 0, 600, t.height as i32));
    }
    f17.draw(&mut title, b"You have found:", 215, 25, [255, 200, 0]);
    let mut n = layer();
    let nf = if matches!(pet, 9 | 11 | 14 | 21) {
        f12
    } else {
        f15
    };
    nf.draw(
        &mut n,
        name.as_bytes(),
        centre(nf, name),
        260,
        [255, 200, 0],
    );
    let mut desc = layer();
    for (i, l) in lines.iter().enumerate() {
        f10.draw(
            &mut desc,
            l.as_bytes(),
            centre(f10, l),
            300 + 20 * i as i32,
            [255; 3],
        );
    }
    f10.draw(
        &mut desc,
        b"Your game has been saved.",
        221,
        390,
        [255, 255, 100],
    );
    // Both button labels in one layer: "Please Wait..." on top, the
    // clickable label 30 px lower (the draw picks one by clipping).
    let mut button = layer();
    if let Some(b) = img.get("IMAGE_MAINBUTTON") {
        let (x, y, w, h) = HATCH_CONTINUE;
        let up = b.crop(0, 0, b.width / 3, b.height);
        for (k, label) in ["Please Wait...", "Click Here to Continue"]
            .iter()
            .enumerate()
        {
            let yy = y - 30 * (1 - k as i32);
            button.draw_image_box(&up, (x, yy, w, h));
            let tx = x + (w - f12.width(label.as_bytes())) / 2;
            f12.draw(
                &mut button,
                label.as_bytes(),
                tx,
                yy + (h + f12.ascent()) / 2 - 3,
                [255, 240, 0],
            );
        }
    }
    Some([title, n, desc, button])
}

/// Egg bob by `T % 32` (`hatch-screen.md#5` step 5).
fn egg_bob(t: i32) -> i32 {
    [0, -1, -2, -1, 0, 1, 2, 1][(t % 32 / 4) as usize]
}

/// Shard velocities (px per tick) for the 8 `IMAGE_EGGSHARDS` cels.
const SHARDS: [(i32, i32); 8] = [
    (1, -10),
    (-11, -15),
    (-14, 3),
    (12, -13),
    (-15, 10),
    (11, 5),
    (13, 14),
    (-1, 12),
];

/// The hatch screen's images (`hatch-screen.md#5`), `T` = updates so far.
fn hatch_cels(h: &Hatch) -> Vec<Cel<'static>> {
    let t = h.t;
    let (jx, jy) = h.jitter;
    let b = egg_bob(t);
    let mut cels = vec![Cel::new("IMAGE_HATCHSCREEN", 0, 0, 0, 0, 0.0)];
    if t <= 80 {
        let cel = if t < 56 { 0 } else { (t - 56) / 2 };
        cels.push(Cel::new("IMAGE_EGGCRACK1", cel, 0, 258, 60 + b, 1.0));
    } else if t < 140 {
        // (At T = 140 the original asks for cel 10 of 10: nothing drawn.)
        let cel = if t < 120 { 0 } else { (t - 120) / 2 };
        cels.push(Cel::new(
            "IMAGE_EGGCRACK2",
            cel,
            0,
            258 + jx,
            60 + jy + b,
            1.0,
        ));
    } else if t > 140 {
        // The pet swims in the incubator window: a clip of its sheet.
        if let Some(sheet) = pet_sheet(h.pet) {
            let y = match h.pet {
                0 | 5 => 100,
                1 => 70,
                _ => 90,
            };
            let f = match h.pet {
                5 | 10 | 11 | 14 | 20 => (t % 40) / 4,
                1 | 6 => {
                    let f = t % 20;
                    if f > 9 { 19 - f } else { f }
                }
                _ => (t % 20) / 2,
            };
            cels.push(Cel::new(sheet, f, 0, 278, y, 1.0));
        }
    }
    // The overlay, on top of everything: reflection, then flying shards.
    cels.push(Cel::new("IMAGE_HATCHREFLECTION", 0, 0, 240, 60, 20.0));
    if (140..200).contains(&t) {
        let s = t - 140;
        for (i, (vx, vy)) in SHARDS.iter().enumerate() {
            cels.push(Cel::new(
                "IMAGE_EGGSHARDS",
                i as i32,
                0,
                268 + vx * s,
                70 + vy * s,
                20.1,
            ));
        }
    }
    cels
}

/// A pet's full-size swim sheet (tank 1 pets for now).
fn pet_sheet(pet: i32) -> Option<&'static str> {
    Some(match pet {
        0 => "IMAGE_STINKY",
        1 => "IMAGE_NIKO",
        2 => "IMAGE_ITCHY",
        3 => "IMAGE_PREGO",
        4 => "IMAGE_ZORF",
        _ => return None,
    })
}

fn hatch_text(commands: &mut Commands, h: &Hatch, hud: &Hud) {
    let Some((_, [title, name, desc, button])) = &hud.hatch else {
        return;
    };
    let full = |img: &Handle<Image>, z: f32, rect: Option<Rect>| {
        let mut s = Sprite::from_image(img.clone());
        s.rect = rect;
        let (x, y) = rect.map_or((0.0, 0.0), |r| (r.min.x, r.min.y));
        (
            Drawn,
            s,
            Anchor::TOP_LEFT,
            Transform::from_xyz(x - W / 2.0, H / 2.0 - y, z),
        )
    };
    commands.spawn(full(title, 5.0, None));
    if h.t > 140 {
        commands.spawn(full(name, 5.1, None));
    }
    if h.t > 170 {
        commands.spawn(full(desc, 5.2, None));
    }
    // The button: "Please Wait..." until clickable, then the real label
    // (drawn 30 px lower in the layer, so shift its source rect).
    let (x, y, w, bh) = HATCH_CONTINUE;
    let (x, w, bh) = (x as f32, w as f32, bh as f32);
    let src_y = if h.can_continue() {
        y as f32
    } else {
        y as f32 - 30.0
    };
    let mut s = Sprite::from_image(button.clone());
    s.rect = Some(Rect::new(x, src_y, x + w, src_y + bh));
    commands.spawn((
        Drawn,
        s,
        Anchor::TOP_LEFT,
        Transform::from_xyz(x - W / 2.0, H / 2.0 - y as f32, 19.0),
    ));
}
