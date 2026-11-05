//! Integration tests for kind-1 missile path and redirect
//! (`alien-kind6.md`). These exercise the `Missile` struct directly and
//! a minimal `Board` scenario with a kind-6 alien firing at guppies.

use insaniquarium_sim::missile::{Missile, MissileStep};

fn rng() -> sexy_core::rand::MtRand {
    sexy_core::rand::MtRand::new(12345)
}

#[test]
fn missile_k1_steer_hit_without_shelter() {
    let mut g = rng();
    let mut m = Missile::new_missile_k1(100, 100, 1, &mut g);
    m.launch = 0;
    // Target overlaps: centre at (140, 140), target at (100, 100).
    assert_eq!(
        m.update_missile_k1(Some((100, 100)), false),
        MissileStep::HitFish
    );
}

#[test]
fn missile_k1_shelter_prevents_hit() {
    let mut g = rng();
    let mut m = Missile::new_missile_k1(100, 100, 1, &mut g);
    m.launch = 0;
    assert_eq!(
        m.update_missile_k1(Some((100, 100)), true),
        MissileStep::Keep
    );
}

#[test]
fn missile_k1_redirect_and_exit() {
    let mut g = rng();
    let mut m = Missile::new_missile_k1(300, 200, 1, &mut g);
    m.launch = 0;
    m.vx = 1.6;
    m.vy = 0.4;

    // Redirect: velocity reverses.
    assert!(m.click_redirect(340, 240));
    assert!(m.bounced);
    assert!(m.vx < 0.0);

    // Fly until removed.
    let mut ticks = 0;
    loop {
        ticks += 1;
        assert!(ticks < 1000, "should exit the tank");
        if let MissileStep::Remove = m.update_missile_k1(None, false) {
            break;
        }
    }
}

#[test]
fn missile_k1_explodes_when_target_gone() {
    let mut g = rng();
    let mut m = Missile::new_missile_k1(300, 200, 1, &mut g);
    m.launch = 0;
    // No target and not redirected → explode.
    assert_eq!(m.update_missile_k1(None, false), MissileStep::Explode);
}

#[test]
fn missile_k1_stanley_shootdown() {
    let mut g = rng();
    let mut m = Missile::new_missile_k1(300, 200, 1, &mut g);
    m.launch = 0;
    m.shot_progress = 1;
    m.shot_duration = 5;
    // Should count up to the duration and then explode.
    assert_eq!(
        m.update_missile_k1(Some((400, 300)), false),
        MissileStep::Keep
    );
    assert_eq!(m.shot_progress, 2);
    assert_eq!(
        m.update_missile_k1(Some((400, 300)), false),
        MissileStep::Keep
    );
    assert_eq!(
        m.update_missile_k1(Some((400, 300)), false),
        MissileStep::Keep
    );
    assert_eq!(
        m.update_missile_k1(Some((400, 300)), false),
        MissileStep::Explode
    );
}
