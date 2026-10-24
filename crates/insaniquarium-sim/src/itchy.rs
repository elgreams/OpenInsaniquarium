//! Itchy the swordfish (`Sexy::FishTypePet`, pet id 2). Spec:
//! `docs/re/objects/pet-itchy.md`.
//!
//! Itchy is a `Fish` with the pet's update: the guppy's swim modes while the
//! tank is calm, a charge at the nearest alien while one is in.

use sexy_core::rand::MtRand;

use crate::alien::Alien;
use crate::fish::{Fish, FishKind};

/// Pet id 2.
pub const ITCHY: i32 = 2;

/// Damage per stab (`0x004fb5b0`, kinds other than 4 and 5).
const STAB: f64 = 1.0;

impl Fish {
    /// `AddPet(2)`'s fish part: `Fish::Init`'s 6 draws (the caller makes the
    /// 2 position draws before and the shadow draw after).
    pub fn new_itchy(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new(x, y, rng);
        f.kind = FishKind::Itchy;
        f.announces = false;
        f
    }

    pub fn is_itchy(&self) -> bool {
        self.kind == FishKind::Itchy
    }

    /// `FishTypePet::Update` (`0x004f7ad0`), id 2, normal game. Returns the
    /// index of the alien stabbed this tick, if any.
    pub fn update_itchy(&mut self, aliens: &[Alien], rng: &mut MtRand) -> Option<usize> {
        self.pre_update();
        let mut stabbed = None;
        if aliens.is_empty() {
            self.swim_modes_calm();
        } else {
            stabbed = self.charge(aliens);
        }
        self.mode_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32 + 1;
            }
        }
        self.damping_timers();
        self.drift_by_vx_bucket();
        // Explicit compares, in the original's order; no bubbles.
        #[allow(clippy::manual_clamp)]
        {
            if f64::from(self.max_x) < self.x {
                self.x = f64::from(self.max_x);
            }
            if self.x < f64::from(self.min_x) {
                self.x = f64::from(self.min_x);
            }
            if f64::from(self.floor_y) < self.y {
                self.y = f64::from(self.floor_y);
            }
            if self.y < f64::from(self.ceil_y) {
                self.y = f64::from(self.ceil_y);
            }
        }
        self.edge_steer();
        self.animate();
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        stabbed
    }

    /// The guppy's swim modes without the startle mode or the `+0x208`
    /// gate on vy (pet-itchy.md, Update step 4).
    fn swim_modes_calm(&mut self) {
        let saved = self.vy_damp_timer;
        self.vy_damp_timer = 0;
        // No mode draws RNG outside the startle mode, which pets never use.
        let mut unused = MtRand::new(0);
        let mut unused_global = MtRand::new(0);
        if self.mode != 123 {
            self.swim_modes(&mut unused, &mut unused_global);
        }
        self.vy_damp_timer = saved;
    }

    /// Chase (`0x004e02d0`) and attack (`0x004fe740`).
    fn charge(&mut self, aliens: &[Alien]) -> Option<usize> {
        let target = self.nearest_alien(aliens)?;
        if self.mode_timer >= 5 {
            self.mode_timer = 0;
            let a = &aliens[target];
            let (tx, ty) = (f64::from(a.widget_x + 80), f64::from(a.widget_y + 80));
            let (cx, cy) = (self.x + 40.0, self.y + 40.0);
            if cx > tx {
                if self.vx > -10.0 {
                    self.vx -= 2.5;
                }
            } else if cx < tx && self.vx < 10.0 {
                self.vx += 2.5;
            }
            if cy > ty {
                if self.vy > -4.0 {
                    self.vy -= 1.5;
                }
            } else if cy < ty && self.vy < 4.0 {
                self.vy += 1.5;
            }
            if self.anim_frame < 5 {
                self.anim_frame += 1;
            }
        }
        let (cx, cy) = (self.x + 40.0, self.y + 40.0);
        aliens.iter().position(|a| {
            (cx - f64::from(a.widget_x + 80)).abs() < 20.0
                && (cy - f64::from(a.widget_y + 80)).abs() < 20.0
        })
    }

    /// `0x004e5fc0`: the nearest alien, distance `trunc(f32 sqrt(dx² + dy²))`
    /// from Itchy's centre to the alien's, in list order (`d < best`).
    fn nearest_alien(&self, aliens: &[Alien]) -> Option<usize> {
        let mut best = 10_000;
        let mut found = None;
        for (i, a) in aliens.iter().enumerate() {
            let dx = (f64::from(a.widget_x + 80) - (self.x + 40.0)) as i32;
            let dy = (f64::from(a.widget_y + 80) - (self.y + 40.0)) as i32;
            let sq = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)) as f32;
            let d = sq.sqrt() as i32;
            if d < best {
                best = d;
                found = Some(i);
            }
        }
        found
    }
}

/// `0x004fb5b0` for Itchy: the alien loses [`STAB`] health.
pub fn stab(alien: &mut Alien) {
    alien.health -= STAB;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    /// A 1-4 board: Itchy is added after Stinky and Niko, swims while calm
    /// and stabs the first alien once it's in.
    #[test]
    fn itchy_stabs_the_alien() {
        let mut b = Board::start_level(MtRand::new(7), MtRand::new(8), 1, 4, &[0, 1, 2]);
        assert_eq!(b.fish_pets.len(), 1);
        assert!(b.fish_pets[0].is_itchy());
        let mut damaged = false;
        for _ in 0..4000 {
            // Keep the guppies alive: plenty of money, food on them.
            b.money = 1000;
            if let Some(g) = b.guppies.first()
                && b.foods.is_empty()
                && b.aliens.is_empty()
            {
                let (x, y) = (g.widget_x + 40, g.widget_y + 20);
                b.step(&[crate::board::Input::Click { x, y }]);
            } else {
                b.step(&[]);
            }
            if let Some(a) = b.aliens.first()
                && a.health < a.max_health
            {
                damaged = true;
                break;
            }
        }
        assert!(damaged, "Itchy never reached the alien");
    }
}
