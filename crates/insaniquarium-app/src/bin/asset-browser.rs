//! Browse every resource in the user's install: images (with cel animation),
//! ImageFonts (rendered with our own glyph layout), sounds, and the music
//! modules (if libopenmpt is installed).
//!
//! Usage: `cargo run --bin asset-browser -- --data <install>`
//! Keys: Left/Right = prev/next resource, PageUp/PageDown = jump 10,
//!       Space = play sound / restart anim, G = toggle cel grid.
//!
//! `--shot <RESOURCE_ID> <out.png>` opens that resource, saves a screenshot
//! after a few frames, and exits (for visual regression checks).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioSource;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::sprite::Anchor;
use sexy_core::anim::AnimInfo;
use sexy_core::data_dir::DataDir;
use sexy_core::font::FontData;
use sexy_core::image::{Rgba, get_image, load_image_res};
use sexy_core::music::Openmpt;
use sexy_core::resources::{Manifest, Resource};
use sexy_core::sound::{Pcm, decode_au};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
/// The original's logic tick (docs/re/framework/timing.md).
const TICK: Duration = Duration::from_millis(28);
const CAPTION_FONT: &str = "FONT_JUNGLEFEVER12OUTLINE";
const SAMPLE_TEXT: &[u8] = b"The quick brown fish jumps over 1,234 lazy aliens!";
/// (browser id, path, start order): the tank songs inside Insaniq2.mo3 per
/// the remap table in docs/re/assets/music.md.
const MUSIC: &[(&str, &str, i32)] = &[
    ("MUSIC_TANK1", "music/Insaniq2.mo3", 0),
    ("MUSIC_TANK2", "music/Insaniq2.mo3", 12),
    ("MUSIC_TANK3", "music/Insaniq2.mo3", 25),
    ("MUSIC_TANK4", "music/Insaniq2.mo3", 37),
    ("MUSIC_MENU", "music/Insaniq2.mo3", 45),
    ("MUSIC_ALIEN", "music/Alien.mo3", 0),
    ("MUSIC_LULLABY", "music/Lullaby.mo3", 0),
];
const MUSIC_PREVIEW_SECS: u32 = 60;

fn render_music(data: &DataDir, path: &str, order: i32) -> Result<Vec<u8>, String> {
    let lib = Openmpt::load().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(data.find(path).ok_or("file missing")?).map_err(|e| e.to_string())?;
    let mut m = lib.open(&bytes).map_err(|e| e.to_string())?;
    m.set_repeat_count(-1);
    m.seek_order(order);
    let rate = 44100;
    let mut samples = vec![0i16; (2 * rate * MUSIC_PREVIEW_SECS) as usize];
    let frames = m.read_stereo(rate, &mut samples);
    samples.truncate(frames * 2);
    Ok(Pcm {
        sample_rate: rate,
        channels: 2,
        samples,
    }
    .to_wav())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cli = args
        .iter()
        .position(|a| a == "--data")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let data = match DataDir::resolve(cli.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let src =
        std::fs::read_to_string(data.join("properties/resources.xml")).expect("read resources.xml");
    let manifest = Manifest::parse(&src).expect("parse resources.xml");
    let mut ids: Vec<String> = manifest.iter().map(|r| r.id().to_string()).collect();
    // Music isn't in resources.xml; it's loaded by path (docs/re/assets/music.md).
    ids.extend(MUSIC.iter().map(|(id, _, _)| id.to_string()));
    let shot = args
        .iter()
        .position(|a| a == "--shot")
        .map(|i| (args[i + 1].clone(), PathBuf::from(&args[i + 2])));
    let index = match &shot {
        Some((id, _)) => ids.iter().position(|x| x == id).unwrap_or_else(|| {
            eprintln!("error: no resource {id}");
            std::process::exit(2);
        }),
        None => 0,
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Insaniquarium asset browser".into(),
                        resolution: (WIDTH as u32, HEIGHT as u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
        .insert_resource(Time::<Fixed>::from_duration(TICK))
        .insert_resource(Browser {
            data,
            manifest,
            ids,
            index,
            dirty: true,
            show_grid: false,
            tick: 0,
        })
        .insert_resource(Shot(shot.map(|(_, p)| p), 0))
        .add_systems(Startup, setup)
        .add_systems(Update, (input, rebuild, take_shot).chain())
        .add_systems(FixedUpdate, animate)
        .run();
}

#[derive(Resource)]
struct Browser {
    data: DataDir,
    manifest: Manifest,
    ids: Vec<String>,
    index: usize,
    dirty: bool,
    show_grid: bool,
    tick: i32,
}

/// Everything spawned for the current resource; despawned on change.
#[derive(Component)]
struct Shown;

#[derive(Component)]
struct Animated {
    info: AnimInfo,
    cel_w: f32,
    cel_h: f32,
    /// Animate along x (cols) or y (rows); see AnimInfo spec on num_cels.
    horizontal: bool,
}

#[derive(Resource)]
struct Shot(Option<PathBuf>, u32);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn take_shot(mut commands: Commands, mut shot: ResMut<Shot>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = shot.0.clone() else { return };
    shot.1 += 1;
    if shot.1 == 30 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if shot.1 == 60 {
        exit.write(AppExit::Success);
    }
}

fn input(keys: Res<ButtonInput<KeyCode>>, mut b: ResMut<Browser>) {
    let n = b.ids.len();
    let step = |b: &mut Browser, d: isize| {
        b.index = (b.index as isize + d).rem_euclid(n as isize) as usize;
        b.dirty = true;
    };
    if keys.just_pressed(KeyCode::ArrowRight) {
        step(&mut b, 1);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        step(&mut b, -1);
    }
    if keys.just_pressed(KeyCode::PageDown) {
        step(&mut b, 10);
    }
    if keys.just_pressed(KeyCode::PageUp) {
        step(&mut b, -10);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        b.show_grid = !b.show_grid;
        b.dirty = true;
    }
    if keys.just_pressed(KeyCode::Space) {
        b.dirty = true;
    }
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

/// Draw `text` with an ImageFont using our layout (pen advance + kerning +
/// per-glyph offsets). Top-left of the text at `origin` (Bevy world coords).
fn spawn_text(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    data: &DataDir,
    font_path: &str,
    text: &[u8],
    origin: Vec2,
) -> Result<f32, String> {
    let bytes = data
        .find(font_path)
        .and_then(|p| std::fs::read(p).ok())
        .ok_or("font file missing")?;
    let font = FontData::parse(&bytes).map_err(|e| e.to_string())?;
    let layer = font.layers.first().ok_or("font has no layers")?;
    let dir = font_path.rsplit_once('/').map_or("", |(d, _)| d);
    let sheet = get_image(
        data,
        &format!("{dir}/{}", layer.image.as_deref().unwrap_or("")),
    )
    .map_err(|e| e.to_string())?;
    let handle = images.add(to_bevy(&sheet));
    let mut pen = 0;
    let mut prev = None;
    for &c in text {
        if let Some(p) = prev {
            pen += layer.kerning.get(&(p, c)).copied().unwrap_or(0);
        }
        if let Some(g) = layer.glyphs.get(&c) {
            if g.rect.w > 0 && g.rect.h > 0 {
                let r = g.rect;
                commands.spawn((
                    Shown,
                    Sprite {
                        image: handle.clone(),
                        rect: Some(Rect::new(
                            r.x as f32,
                            r.y as f32,
                            (r.x + r.w) as f32,
                            (r.y + r.h) as f32,
                        )),
                        ..default()
                    },
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(
                        origin.x + (pen + g.offset.0) as f32,
                        origin.y - g.offset.1 as f32,
                        1.0,
                    ),
                ));
            }
            pen += g.width;
        }
        prev = Some(c);
    }
    Ok((layer.ascent + layer.line_spacing_offset).max(12) as f32 + 6.0)
}

fn rebuild(
    mut commands: Commands,
    mut b: ResMut<Browser>,
    shown: Query<Entity, With<Shown>>,
    mut images: ResMut<Assets<Image>>,
    mut audio: ResMut<Assets<AudioSource>>,
) {
    if !b.dirty {
        return;
    }
    b.dirty = false;
    b.tick = 0;
    for e in &shown {
        commands.entity(e).despawn();
    }
    let id = b.ids[b.index].clone();
    let top_left = Vec2::new(-WIDTH / 2.0 + 16.0, HEIGHT / 2.0 - 16.0);

    let mut info = format!("[{}/{}] {id}", b.index + 1, b.ids.len());
    let caption_font = match b.manifest.get(CAPTION_FONT) {
        Some(Resource::Font(f)) => Some(f.path.clone()),
        _ => None,
    };

    if let Some((_, path, order)) = MUSIC.iter().find(|(m, _, _)| *m == id) {
        info +=
            &format!("  {path} @ order {order}  ({MUSIC_PREVIEW_SECS}s preview, Space to replay)");
        match render_music(&b.data, path, *order) {
            Ok(wav) => {
                let h = audio.add(AudioSource {
                    bytes: Arc::from(wav),
                });
                commands.spawn((Shown, AudioPlayer::new(h), PlaybackSettings::DESPAWN));
            }
            Err(e) => info += &format!("  ERROR: {e} (install libopenmpt)"),
        }
        info!("{info}");
        if let Some(fp) = caption_font {
            let ascii: Vec<u8> = info.bytes().filter(|c| c.is_ascii()).collect();
            let _ = spawn_text(&mut commands, &mut images, &b.data, &fp, &ascii, top_left);
        }
        return;
    }
    let res = b.manifest.get(&id).cloned().unwrap();

    let body_top = top_left - Vec2::new(0.0, 40.0);
    match &res {
        Resource::Image(r) => {
            info += &format!("  {}  {}x{} cels", r.path, r.cols, r.rows);
            match load_image_res(&b.data, r) {
                Ok(img) => {
                    info += &format!("  {}x{}px", img.width, img.height);
                    let (cw, ch) = ((img.width / r.cols) as f32, (img.height / r.rows) as f32);
                    let handle = images.add(to_bevy(&img));
                    // Full sheet.
                    commands.spawn((
                        Shown,
                        Sprite::from_image(handle.clone()),
                        Anchor::TOP_LEFT,
                        Transform::from_translation(body_top.extend(0.0)),
                    ));
                    if b.show_grid {
                        for i in 1..r.cols {
                            commands.spawn((
                                Shown,
                                Sprite::from_color(
                                    Color::srgba(1.0, 0.0, 1.0, 0.6),
                                    Vec2::new(1.0, img.height as f32),
                                ),
                                Anchor::TOP_LEFT,
                                Transform::from_xyz(body_top.x + i as f32 * cw, body_top.y, 2.0),
                            ));
                        }
                        for j in 1..r.rows {
                            commands.spawn((
                                Shown,
                                Sprite::from_color(
                                    Color::srgba(1.0, 0.0, 1.0, 0.6),
                                    Vec2::new(img.width as f32, 1.0),
                                ),
                                Anchor::TOP_LEFT,
                                Transform::from_xyz(body_top.x, body_top.y - j as f32 * ch, 2.0),
                            ));
                        }
                    }
                    // Animated preview at the right edge.
                    if let Some(anim) = AnimInfo::for_image(r) {
                        info += &format!(
                            "  anim {:?} delay {} total {}t",
                            anim.kind, anim.frame_delay, anim.total_time
                        );
                        commands.spawn((
                            Shown,
                            Sprite {
                                image: handle,
                                rect: Some(Rect::new(0.0, 0.0, cw, ch)),
                                ..default()
                            },
                            Anchor::TOP_LEFT,
                            Transform::from_xyz(WIDTH / 2.0 - cw - 16.0, body_top.y, 3.0),
                            Animated {
                                info: anim,
                                cel_w: cw,
                                cel_h: ch,
                                horizontal: r.cols >= r.rows,
                            },
                        ));
                    }
                }
                Err(e) => info += &format!("  ERROR: {e}"),
            }
        }
        Resource::Font(f) => {
            info += &format!("  {}", f.path);
            let mut y = body_top.y;
            for line in [
                SAMPLE_TEXT,
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                b"abcdefghijklmnopqrstuvwxyz 0123456789 $%!?",
            ] {
                match spawn_text(
                    &mut commands,
                    &mut images,
                    &b.data,
                    &f.path,
                    line,
                    Vec2::new(body_top.x, y),
                ) {
                    Ok(h) => y -= h,
                    Err(e) => {
                        info += &format!("  ERROR: {e}");
                        break;
                    }
                }
            }
        }
        Resource::Sound(s) => {
            info += &format!("  {}  (Space to replay)", s.path);
            let bytes = b.data.find_with_ext(&s.path, &["ogg", "au"]).and_then(|p| {
                let raw = std::fs::read(&p).ok()?;
                if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("au")) {
                    decode_au(&raw).ok().map(|pcm| pcm.to_wav())
                } else {
                    Some(raw)
                }
            });
            match bytes {
                Some(bytes) => {
                    let h = audio.add(AudioSource {
                        bytes: Arc::from(bytes),
                    });
                    commands.spawn((Shown, AudioPlayer::new(h), PlaybackSettings::DESPAWN));
                }
                None => info += "  ERROR: could not load",
            }
        }
    }

    info!("{info}");
    let ascii: Vec<u8> = info.bytes().filter(|c| c.is_ascii()).collect();
    if let Some(fp) = caption_font {
        let _ = spawn_text(&mut commands, &mut images, &b.data, &fp, &ascii, top_left);
    }
}

fn animate(mut b: ResMut<Browser>, mut q: Query<(&mut Sprite, &Animated)>) {
    b.tick += 1;
    for (mut sprite, a) in &mut q {
        let cel = a.info.cel(b.tick);
        if cel < 0 {
            sprite.color = Color::NONE;
            continue;
        }
        sprite.color = Color::WHITE;
        let (x, y) = if a.horizontal {
            (cel as f32 * a.cel_w, 0.0)
        } else {
            (0.0, cel as f32 * a.cel_h)
        };
        sprite.rect = Some(Rect::new(x, y, x + a.cel_w, y + a.cel_h));
    }
}
