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
/// Pet id 3 (Prego, same class; `pet-prego.md`).
pub const PREGO: i32 = 3;
/// Pet id 6 (Vert, same class; `pet-vert.md`).
pub const VERT: i32 = 6;

/// Vert's coin interval (`FishTypePet` ctor, id 6, normal game).
const VERT_INTERVAL: i32 = 216;

/// Prego's first birth interval (`FishTypePet` ctor, id 3).
const PREGO_INTERVAL: i32 = 930;

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

    /// `AddPet(3)`'s fish part: as Itchy, then the id-3 ctor branch (no
    /// RNG): the birth interval replaces the coin interval, floor 360. The
    /// birth counter is the coin timer (`+0x220`, 0).
    pub fn new_prego(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Prego;
        f.coin_interval = PREGO_INTERVAL;
        f.floor_y = 360;
        f
    }

    pub fn is_prego(&self) -> bool {
        self.kind == FishKind::Prego
    }

    /// `AddPet(6)`'s fish part: as Itchy, then the id-6 ctor branch (no RNG
    /// outside the Virtual Tank): the coin interval becomes 216.
    pub fn new_vert(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Vert;
        f.coin_interval = VERT_INTERVAL;
        f
    }

    pub fn is_vert(&self) -> bool {
        self.kind == FishKind::Vert
    }

    /// A `FishTypePet`'s update: Itchy's or Prego's. Returns what the
    /// board must apply.
    pub fn update_fish_pet(&mut self, aliens: &[Alien], rng: &mut MtRand) -> PetFishOut {
        match self.kind {
            FishKind::Prego => self.update_prego(aliens, rng),
            FishKind::Vert => self.update_vert(aliens, rng),
            _ => PetFishOut {
                stabbed: self.update_itchy(aliens, rng),
                ..Default::default()
            },
        }
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
        self.clamp_pet();
        self.edge_steer();
        self.animate();
        self.integrate();
        stabbed
    }

    /// `FishTypePet::Update` for id 3 (pet-prego.md#update): Itchy's
    /// movement without the chase, plus the birth timer (slot 81) while no
    /// alien is in. The board adds the newborn (`AddGuppyAt`, 7 draws) at
    /// the returned spot, inside this update's draw order.
    fn update_prego(&mut self, aliens: &[Alien], rng: &mut MtRand) -> PetFishOut {
        let mut out = PetFishOut::default();
        self.pre_update();
        self.swim_modes_calm();
        self.mode_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32 + 1;
            }
        }
        if aliens.is_empty() {
            self.coin_timer += 1;
            if self.coin_timer >= self.coin_interval {
                self.coin_timer = 0;
                out.birth = Some((self.widget_x + 7, self.widget_y + 25));
            }
        }
        out
    }

    /// `FishTypePet::Update` for id 6 (pet-vert.md#update-0x004f7ad0-id-6):
    /// Prego's movement, the coin drop (slot 81) while no alien is in, and
    /// the ping-pong swim animation.
    fn update_vert(&mut self, aliens: &[Alien], rng: &mut MtRand) -> PetFishOut {
        let mut out = PetFishOut::default();
        self.pre_update();
        self.swim_modes_calm();
        self.mode_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32 + 1;
            }
        }
        if aliens.is_empty() {
            self.coin_timer += 1;
            if self.coin_timer >= self.coin_interval {
                self.coin_timer = 0;
                // `AddCoin(wx + 15, wy + 10, kind 2)`: a gold coin.
                out.coin = Some((self.widget_x + 15, self.widget_y + 10));
            }
        }
        self.damping_timers();
        self.drift_by_vx_bucket();
        self.clamp_pet();
        self.edge_steer();
        self.animate();
        // Id 6's swim branch (`0x004dfc50`): the cycle ping-pongs, cel =
        // cycle below 10, else 19 − cycle (the cycle itself advanced in
        // `animate`).
        if self.turn_timer == 0 && self.eat_timer == 0 {
            self.cel = if self.swim_cycle < 10 {
                self.swim_cycle
            } else {
                19 - self.swim_cycle
            };
        }
        self.integrate();
        out
    }

    /// The rest of Prego's update, after the board placed any newborn
    /// (`guppies` = the guppy count including it, for the next interval).
    pub fn finish_prego(&mut self, born: bool, guppies: usize) {
        if born {
            self.coin_interval = if guppies > 10 {
                2000
            } else if guppies > 5 {
                PREGO_INTERVAL + 300
            } else {
                PREGO_INTERVAL
            };
        }
        self.damping_timers();
        self.drift_by_vx_bucket();
        self.clamp_pet();
        self.edge_steer();
        // Slot 89, id 3: in the last 219 ticks before a birth she doesn't
        // turn; a reversal stops her instead.
        if self.coin_timer > self.coin_interval - 220
            && ((self.facing > 0.0 && self.vx < 0.0) || (self.facing < 0.0 && self.vx > 0.0))
        {
            self.vx = 0.0;
        }
        self.animate();
        self.integrate();
    }

    /// Explicit compares, in the original's order; no bubbles.
    #[allow(clippy::manual_clamp)]
    fn clamp_pet(&mut self) {
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

    fn integrate(&mut self) {
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
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

/// What a fish-shaped pet's update asks the board to do.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PetFishOut {
    /// Itchy stabbed this alien (index).
    pub stabbed: Option<usize>,
    /// Prego gives birth: `AddGuppyAt` here, then `finish_prego`.
    pub birth: Option<(i32, i32)>,
    /// Vert drops a gold coin at this widget position.
    pub coin: Option<(i32, i32)>,
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
