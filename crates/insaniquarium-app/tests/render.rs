//! CPU rendering of a tank frame from the game data (skipped without it).
//! Set `RENDER_OUT=1` to also write the frame to `target/render-test.png`.

use insaniquarium_app::assets::GameImages;
use insaniquarium_app::render::render;
use insaniquarium_app::scene::{self, Fonts};
use insaniquarium_sim::game::{Game, Screen};
use insaniquarium_sim::profile::Profile;
use sexy_core::data_dir::test_data_dir;
use sexy_core::rand::MtRand;
use sexy_core::resources::Manifest;

#[test]
fn renders_tank_1_2() {
    let Some(data) = test_data_dir() else { return };
    let src = std::fs::read_to_string(data.join("properties/resources.xml")).unwrap();
    let manifest = Manifest::parse(&src).unwrap();
    let images = GameImages::load(&data, &manifest).unwrap();
    let sheets = scene::sheets(&images, &manifest);
    let mut fonts = Fonts::load(&data);

    // As the tank binary does with `--seed 1234 --level 2`.
    let mut global = MtRand::new(1234);
    let app_rng = MtRand::new(global.next_u31());
    let mut profile = Profile::default();
    profile.finish_level(1, 1);
    let mut game = Game::new(profile, app_rng, global);
    for _ in 0..100 {
        game.step_tank(&[]);
    }
    let Screen::Tank(b) = &game.screen else {
        panic!("not in the tank")
    };
    assert_eq!((b.tank, b.level), (1, 2));

    let frame = scene::frame(&game, None, false, &mut fonts, &images);
    assert!(!frame.cels.is_empty());
    assert!(!frame.texts.is_empty(), "money and prices are drawn");
    let img = render(&frame, &sheets);
    assert_eq!((img.width, img.height), (640, 480));
    let p = img.pixels[(240 * 640 + 320) as usize];
    assert_ne!(&p[..3], &[0, 0, 0], "backdrop drawn");

    // Same state, same frame (the font cache is warm the second time).
    let again = render(
        &scene::frame(&game, None, false, &mut fonts, &images),
        &sheets,
    );
    assert_eq!(img.as_bytes(), again.as_bytes());

    if std::env::var_os("RENDER_OUT").is_some() {
        let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/render-test.png");
        image::save_buffer(
            out,
            img.as_bytes(),
            img.width,
            img.height,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    }
}
