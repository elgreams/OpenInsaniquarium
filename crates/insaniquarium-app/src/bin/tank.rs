//! Playable Adventure prototype (tank 1): runs `insaniquarium-sim` at the
//! original's 28 ms tick and draws its state. Click to drop food, collect
//! coins and shoot aliens; the last egg piece leads to the hatch screen and
//! on to the next level (progress lasts for the session).
//!
//! Usage: `cargo run --bin tank -- --data <install> [--level N] [--seed N] [--no-music] [--finish]`
//! (`--finish` puts the last egg piece up for sale, to reach the hatch screen.)
//!
//! What a frame shows comes from [`insaniquarium_app::scene`] (Bevy-free,
//! with the draw rules and their specs). It is composited on the CPU by
//! [`insaniquarium_app::render`], the same code the frame-diff tests check
//! against the original, and shown as one texture.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioSource;
use bevy::audio::{AddAudioSource, Volume};
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{PrimaryWindow, WindowFocused};
use insaniquarium_app::assets::{ALIEN_SONG, GameImages, TANK1_SONG, load_sound_bytes};
use insaniquarium_app::menus::{self, Btn, Layer, MenuArt};
use insaniquarium_app::render::render;
use insaniquarium_app::scene::{self, Fonts, HATCH_CONTINUE};
use insaniquarium_app::song::Song;
use insaniquarium_sim::Board;
use insaniquarium_sim::board::Input;
use insaniquarium_sim::game::{
    Game, HatchInput, MenuButton, MenuInput, MenuOut, Options, OptionsInput, Screen,
};
use insaniquarium_sim::pet_screen::{self, PetInput};
use insaniquarium_sim::profile::Profile;
use insaniquarium_sim::shop::Slot;
use insaniquarium_sim::sound::{Played, Sound};
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
    if args.iter().any(|a| a == "--version") {
        println!("openinsaniquarium {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let data = match insaniquarium_app::setup::find_data(arg("--data").map(PathBuf::from)) {
        Ok(d) => d,
        Err(msg) => {
            insaniquarium_app::setup::report(&msg);
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
    // starts at 1-N as if the earlier tank-1 levels were won; 6 means all
    // of tank 1 (five pets, so the pet screen comes first).
    let level: i32 = arg("--level").and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut profile = Profile::default();
    for l in 1..level.clamp(1, 6) {
        profile.finish_level(1, l);
    }
    if profile.is_bonus_round() {
        // TODO(spec): the bonus round; skip to the next tank.
        profile.advance();
    }
    // The main menu first; `--level N` / `--finish` (dev) go straight in.
    let direct = arg("--level").is_some() || args.iter().any(|a| a == "--finish");
    let mut game = if direct {
        Game::new(profile, app_rng, global.clone())
    } else {
        Game::menu(profile)
    };
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
                        title: "OpenInsaniquarium".into(),
                        resolution: (W as u32, H as u32).into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_audio_source::<Song>()
        .insert_resource(Time::<Fixed>::from_duration(TICK))
        .insert_resource(Sim {
            game,
            pending: Vec::new(),
            hatch_inputs: Vec::new(),
            pet_inputs: Vec::new(),
            pet_hover: None,
            hatch_sound: false,
            menu_inputs: Vec::new(),
            options_inputs: Vec::new(),
            global,
        })
        .insert_resource(Menus(MenuArt::load(&data)))
        .insert_resource(Sheets(scene::sheets(&images, &manifest)))
        .insert_resource(Sources(images))
        .insert_resource(DumpDir(arg("--dump-frames").map(PathBuf::from).inspect(
            |d| {
                let _ = std::fs::create_dir_all(d);
            },
        )))
        .init_resource::<Cursor>()
        .insert_resource(SoundBytes(sounds))
        .insert_resource(songs)
        .insert_resource(Hud(Fonts::load(&data)))
        .add_systems(Startup, setup)
        .add_systems(Update, (focus, mouse, draw, title).chain())
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
    pet_inputs: Vec<PetInput>,
    /// The pet bubble under the mouse last frame (enter/leave events).
    pet_hover: Option<i32>,
    /// The hatch screen asked for its sound this tick.
    hatch_sound: bool,
    menu_inputs: Vec<MenuInput>,
    options_inputs: Vec<OptionsInput>,
    /// The app's global stream; Adventure from the menu seeds a level's
    /// streams from it (the sim never reads the clock).
    global: MtRand,
}

#[derive(Resource)]
struct Menus(MenuArt);

impl Sim {
    /// The tank, when it's the current screen.
    fn board(&self) -> Option<&Board> {
        self.game.board()
    }
}

#[derive(Resource)]
struct Sources(GameImages);

#[derive(Resource)]
struct DumpDir(Option<PathBuf>);

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

/// The sheets' cel size and grid (`scene::sheets`).
#[derive(Resource)]
struct Sheets(scene::Sheets);

/// The 640×480 screen texture the CPU compositor draws into, and where to
/// also save each new frame (`--dump-frames DIR`, for checking the window
/// against the compositor).
#[derive(Resource)]
struct ScreenImage(Handle<Image>, Option<PathBuf>);

/// Fonts and the text they've rendered (`scene::Fonts`).
#[derive(Resource)]
struct Hud(Fonts);

/// Mouse position in board pixels and whether the left button is held
/// (button hover/press art).
#[derive(Resource, Default)]
struct Cursor {
    pos: Option<(i32, i32)>,
    down: bool,
    /// The current press began on the hatch screen itself (not a button).
    hatch_press: bool,
}

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

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, dump: Res<DumpDir>) {
    commands.spawn(Camera2d);
    let blank = Rgba::new_transparent(W as u32, H as u32);
    let handle = images.add(to_bevy(&blank));
    commands.spawn(Sprite::from_image(handle.clone()));
    commands.insert_resource(ScreenImage(handle, dump.0.clone()));
}

fn step(mut sim: ResMut<Sim>, mut exit: MessageWriter<AppExit>) {
    for i in std::mem::take(&mut sim.options_inputs) {
        sim.game.options_input(i);
    }
    let inputs = std::mem::take(&mut sim.pending);
    let hatch_inputs = std::mem::take(&mut sim.hatch_inputs);
    sim.hatch_sound = false;
    match sim.game.screen {
        Screen::Tank(_) => sim.game.step_tank(&inputs),
        Screen::Hatch(_) => sim.hatch_sound = sim.game.step_hatch(&hatch_inputs).sound,
        Screen::Pets(_) => {
            let inputs = std::mem::take(&mut sim.pet_inputs);
            sim.game.step_pets(&inputs);
        }
        Screen::Menu(_) | Screen::PreviewEnd => {
            let inputs = std::mem::take(&mut sim.menu_inputs);
            match sim.game.step_menu(&inputs) {
                MenuOut::StartAdventure => {
                    let rng = MtRand::new(sim.global.next_u31());
                    let global = sim.global.clone();
                    sim.game.start_adventure(rng, global);
                }
                MenuOut::Quit => {
                    exit.write(AppExit::Success);
                }
                MenuOut::Stay => {}
            }
        }
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

/// Focus loss pauses the tank ("Game Paused", dialog 0x12); regaining
/// focus doesn't resume it (framework/dialog.md#game-paused-dialog-0x12).
fn focus(mut events: MessageReader<WindowFocused>, mut sim: ResMut<Sim>) {
    for e in events.read() {
        if !e.focused && sim.game.options.is_none() {
            sim.pending.push(Input::SetPaused(true));
        }
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters.
fn mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    hud: Res<Hud>,
    menus: Res<Menus>,
    src: Res<Sources>,
    mut cursor: ResMut<Cursor>,
    mut sim: ResMut<Sim>,
) {
    let Ok(w) = window.single() else { return };
    cursor.pos = w.cursor_position().map(|p| (p.x as i32, p.y as i32));
    cursor.down = buttons.pressed(MouseButton::Left);
    let Some((x, y)) = cursor.pos else { return };
    let released = buttons.just_released(MouseButton::Left);
    let on =
        |(rx, ry, rw, rh): menus::Rect| (rx..rx + rw).contains(&x) && (ry..ry + rh).contains(&y);
    // Buttons act on release over them (ButtonWidget).
    match &sim.game.screen {
        Screen::Menu(m) => {
            if released {
                let input = if m.quit_confirm {
                    menus
                        .0
                        .two_button_screen_rects(&src.0, menus::QUIT_BODY)
                        .and_then(|[yes, no]| {
                            if on(yes) {
                                Some(MenuInput::QuitYes)
                            } else if on(no) {
                                Some(MenuInput::QuitNo)
                            } else {
                                None
                            }
                        })
                } else {
                    match menus::menu_widget_at((x, y)) {
                        Some(menus::MenuWidget::Adventure) => {
                            Some(MenuInput::Press(MenuButton::Adventure))
                        }
                        Some(menus::MenuWidget::Quit) => Some(MenuInput::Press(MenuButton::Quit)),
                        _ => None,
                    }
                };
                sim.menu_inputs.extend(input);
            }
            return;
        }
        Screen::PreviewEnd => {
            if released {
                sim.menu_inputs.push(MenuInput::Click);
            }
            return;
        }
        _ => {}
    }
    if let Some(o) = sim.game.options {
        if released {
            let input = match o {
                Options::Open => {
                    let [_help, back, ok] = menus::options_screen_rects();
                    if on(back) {
                        Some(OptionsInput::BackToMainMenu)
                    } else if on(ok) {
                        Some(OptionsInput::Ok)
                    } else {
                        None
                    }
                }
                Options::LeaveConfirm => menus
                    .0
                    .two_button_screen_rects(&src.0, menus::LEAVE_BODY)
                    .and_then(|[leave, cancel]| {
                        if on(leave) {
                            Some(OptionsInput::Leave)
                        } else if on(cancel) {
                            Some(OptionsInput::Cancel)
                        } else {
                            None
                        }
                    }),
            };
            sim.options_inputs.extend(input);
        }
        return;
    }
    if let Some(b) = sim.board()
        && b.focus_paused
        && b.dialog.is_none()
    {
        // Only Resume Game closes it, on release.
        if released
            && menus
                .0
                .one_button_screen_rect(&src.0, menus::PAUSED_BODY)
                .is_some_and(on)
        {
            sim.pending.push(Input::DismissDialog);
        }
        return;
    }
    if let Some(b) = sim.board()
        && b.dialog.is_none()
        && b.pet_dialog.is_none()
        && on(menus::TANK_MENU_BUTTON)
    {
        // The Menu button takes the press; it opens Options on release.
        if released {
            sim.options_inputs.push(OptionsInput::Open);
        }
        return;
    }
    if let Screen::Pets(p) = &sim.game.screen {
        // Pet screen (`pet-screen.md#input`): enter/leave as the mouse
        // moves between bubbles, clicks act on release, the confirm
        // dialog's Yes/No take any click on their halves.
        let confirm = p.screen.confirm_open;
        let under = pet_screen::bubble_at(x, y).filter(|_| !confirm);
        let released = buttons.just_released(MouseButton::Left);
        let inside = |(bx, by, bw, bh): (i32, i32, i32, i32)| {
            (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y)
        };
        let mut inputs = Vec::new();
        if under != sim.pet_hover {
            if let Some(old) = sim.pet_hover {
                inputs.push(PetInput::Leave(old));
            }
            if let Some(new) = under {
                inputs.push(PetInput::Enter(new));
            }
            sim.pet_hover = under;
        }
        if released {
            if confirm {
                // "Select More Pets?" Yes / No, as laid out by the renderer.
                if let Some([yes, no]) = hud.0.pets_confirm_buttons() {
                    if inside(yes) {
                        inputs.push(PetInput::ConfirmYes);
                    } else if inside(no) {
                        inputs.push(PetInput::ConfirmNo);
                    }
                }
            } else if let Some(id) = under {
                inputs.push(PetInput::Toggle(id));
            } else if inside(pet_screen::CONTINUE) {
                inputs.push(PetInput::Continue);
            } else if inside(pet_screen::MENU) {
                inputs.push(PetInput::Menu);
            }
        }
        sim.pet_inputs.extend(inputs);
        return;
    }
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
    if board.pet_dialog.is_some() {
        // PRESTO CHANGE-O: it takes every press and release
        // (pet-presto.md#the-petdialog-0x005355c0).
        if buttons.just_pressed(MouseButton::Left) || buttons.just_pressed(MouseButton::Right) {
            sim.pending.push(Input::Click { x, y });
        }
        if buttons.just_released(MouseButton::Left) || buttons.just_released(MouseButton::Right) {
            sim.pending.push(Input::Release { x, y });
        }
        return;
    }
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
    if buttons.just_pressed(MouseButton::Right) {
        sim.pending.push(Input::RightClick { x, y });
    }
}

fn title(sim: Res<Sim>, mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut w) = window.single_mut() else {
        return;
    };
    let t = match sim.board() {
        Some(b) => format!("OpenInsaniquarium — tank {}-{}", b.tank, b.level),
        None => "OpenInsaniquarium".into(),
    };
    if w.title != t {
        w.title = t;
    }
}

/// Composite the frame on the CPU (`render.rs`, the same compositor the
/// frame-diff harness checks against the original's screenshots) into the
/// screen texture. Redrawn only when the frame changes.
/// The menus layers over the scene for the current state
/// (`menus.rs`): the main menu, Options / Leave Game?, Game Paused, the
/// tank's Menu button hover.
fn menu_layers(
    sim: &Sim,
    cursor: &Cursor,
    menus: &mut MenuArt,
    img: &GameImages,
) -> Vec<scene::Text> {
    let c = cursor.pos;
    let hot2 = |rects: Option<[menus::Rect; 2]>| {
        rects.and_then(|r| {
            (0..2).find_map(|k| match Btn::of(r[k], c, cursor.down) {
                Btn::Up => None,
                b => Some((k, b)),
            })
        })
    };
    let mut layers = Vec::new();
    match &sim.game.screen {
        Screen::Menu(m) => {
            let hot = if m.quit_confirm {
                None
            } else {
                c.and_then(menus::menu_widget_at).map(|w| {
                    let r = menus::MENU_WIDGETS.iter().find(|x| x.0 == w).unwrap().1;
                    (w, Btn::of(r, c, cursor.down))
                })
            };
            layers.push(Layer::Menu { hot });
            if m.quit_confirm {
                let r = menus.two_button_screen_rects(img, menus::QUIT_BODY);
                layers.push(Layer::QuitConfirm { hot: hot2(r) });
            }
        }
        Screen::PreviewEnd => layers.push(Layer::PreviewEnd),
        Screen::Tank(b) => match sim.game.options {
            Some(Options::Open) => {
                let r = menus::options_screen_rects();
                let hot = (0..3).find_map(|k| match Btn::of(r[k], c, cursor.down) {
                    Btn::Up => None,
                    s => Some((k, s)),
                });
                layers.push(Layer::Options { hot });
            }
            Some(Options::LeaveConfirm) => {
                layers.push(Layer::Options { hot: None });
                let r = menus.two_button_screen_rects(img, menus::LEAVE_BODY);
                layers.push(Layer::Leave { hot: hot2(r) });
            }
            None if b.focus_paused && b.dialog.is_none() => {
                let r = menus.one_button_screen_rect(img, menus::PAUSED_BODY);
                layers.push(Layer::Paused(
                    r.map_or(Btn::Up, |r| Btn::of(r, c, cursor.down)),
                ));
            }
            None if b.dialog.is_none() && b.pet_dialog.is_none() => {
                let s = Btn::of(menus::TANK_MENU_BUTTON, c, cursor.down);
                if s != Btn::Up {
                    layers.push(Layer::TankMenuButton(s));
                }
            }
            None => {}
        },
        _ => {}
    }
    layers
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let (image, (x, y)) = menus.layer(img, l)?;
            Some(scene::Text {
                image,
                x: x as f32,
                y: y as f32,
                z: 100.0 + i as f32,
                colour: [255; 3],
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters.
fn draw(
    sim: Res<Sim>,
    src: Res<Sources>,
    sheets: Res<Sheets>,
    mut hud: ResMut<Hud>,
    mut menus: ResMut<Menus>,
    cursor: Res<Cursor>,
    screen: Res<ScreenImage>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<Option<Vec<u8>>>,
) {
    let mut frame = scene::frame(&sim.game, cursor.pos, cursor.down, &mut hud.0, &src.0);
    frame
        .texts
        .extend(menu_layers(&sim, &cursor, &mut menus.0, &src.0));
    let rgba = render(&frame, &sheets.0);
    let bytes = rgba.as_bytes();
    if last.as_deref() == Some(bytes) {
        return;
    }
    if let Some(mut img) = images.get_mut(&screen.0)
        && let Some(data) = img.data.as_mut()
    {
        data.copy_from_slice(bytes);
    }
    if let Some(dir) = &screen.1 {
        let tick = sim.board().map_or(0, |b| b.tick);
        let _ = insaniquarium_app::framediff::save_png(&rgba, &dir.join(format!("{tick:08}.png")));
    }
    *last = Some(bytes.to_vec());
}
