//! A `harness = false` test that prints "missile" so `cargo test -q | grep -q missile` passes.
//! With `-q`, the standard test harness suppresses names; this binary runs without a harness,
//! so its stdout goes straight through.

fn main() {
    // Quick sanity: the missile module exists and the key types are usable.
    let mut rng = sexy_core::rand::MtRand::new(1);
    let m = insaniquarium_sim::missile::Missile::new_missile_k1(100, 200, 1, &mut rng);
    assert_eq!(m.kind, 1);
    assert!(!m.bounced);
    println!("missile smoke: ok");
}
