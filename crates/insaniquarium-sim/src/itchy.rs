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

/// Pet id 4 (Zorf, same class; `pet-zorf.md`).
pub const ZORF: i32 = 4;
/// Pet id 8 (Meryl, same class; `pet-meryl.md`).
pub const MERYL: i32 = 8;
/// Pet id 9 (Wadsworth, same class; `pet-wadsworth.md`).
pub const WADSWORTH: i32 = 9;
/// Pet ids 10–12 (`pet-seymour.md`, `pet-shrapnel.md`, `pet-gumbo.md`).
pub const SEYMOUR: i32 = 10;
pub const SHRAPNEL: i32 = 11;
pub const GUMBO: i32 = 12;

/// Vert's coin interval (`FishTypePet` ctor, id 6, normal game).
const VERT_INTERVAL: i32 = 216;
/// Zorf's feed interval.
const ZORF_INTERVAL: i32 = 65;
/// Meryl's song interval (normal game; the song is its last 100 ticks).
const MERYL_INTERVAL: i32 = 1400;

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
        // `+0x244 = 1` for every FishTypePet (only Meryl reads it).
        f.blink = true;
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

    /// `AddPet(4)`'s fish part: the id-4 ctor branch (no RNG): feed
    /// interval 65, divisor 3.0, floor 270.
    pub fn new_zorf(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Zorf;
        f.coin_interval = ZORF_INTERVAL;
        f.divisor = 3.0;
        f.floor_y = 270;
        f
    }

    /// `AddPet(10)`'s fish part: no id-10 ctor branch.
    pub fn new_seymour(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Seymour;
        f
    }

    /// `AddPet(11)`'s fish part: the bomb interval `r % 20 + 633`, drawn
    /// after `Fish::Init`'s 6 (the board draws the shadow after).
    pub fn new_shrapnel(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Shrapnel;
        f.coin_interval = rng.next_range(20) as i32 + 633;
        f
    }

    /// `AddPet(12)`'s fish part: no id-12 ctor branch.
    pub fn new_gumbo(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Gumbo;
        f
    }

    /// `AddPet(9)`'s fish part: mouth-open length 120, divisor 4.0 (no RNG).
    pub fn new_wadsworth(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Wadsworth;
        f.coin_interval = 120;
        f.divisor = 4.0;
        // `blink` holds his sheltering flag `+0x250` (0 at start).
        f.blink = false;
        f
    }

    /// `AddPet(8)`'s fish part: the song interval (no RNG); the blink flag
    /// starts set.
    pub fn new_meryl(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish::new_itchy(x, y, rng);
        f.kind = FishKind::Meryl;
        f.coin_interval = MERYL_INTERVAL;
        f
    }

    /// A `FishTypePet`'s update: Itchy's or Prego's. Returns what the
    /// board must apply.
    pub fn update_fish_pet(&mut self, ctx: PetCtx<'_>, rng: &mut MtRand) -> PetFishOut {
        let aliens = ctx.aliens;
        match self.kind {
            FishKind::Prego => self.update_prego(aliens, rng),
            FishKind::Vert => self.update_vert(aliens, rng),
            FishKind::Zorf => self.update_zorf(ctx, rng),
            FishKind::Meryl => self.update_meryl(aliens, rng),
            FishKind::Wadsworth => self.update_wadsworth(ctx, rng),
            FishKind::Seymour => self.update_seymour(rng),
            FishKind::Shrapnel => self.update_shrapnel(ctx.aliens, rng),
            FishKind::Gumbo => self.update_gumbo(ctx.aliens, rng),
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

    /// The swim part every calm-swimming FishTypePet shares: PreUpdate,
    /// the swim modes and the 21-tick re-roll.
    fn swim_and_reroll(&mut self, rng: &mut MtRand) {
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
    }

    /// The default swim animation without the guppy's eat-cel extras
    /// (`0x004dfc50` default branch): turn, else the cycle at speed 1 or 2.
    fn animate_pet(&mut self, every_tick: bool) {
        if self.facing > 0.0 && self.vx < 0.0 {
            self.turn_timer = 20;
        } else if self.facing < 0.0 && self.vx > 0.0 {
            self.turn_timer = -20;
        }
        if self.turn_timer != 0 {
            self.turn_timer -= self.turn_timer.signum();
            if self.turn_timer == 0 {
                self.eat_timer = 0;
            }
        }
        if self.turn_timer > 0 {
            self.cel = 9 - self.turn_timer / 2;
        } else if self.turn_timer < 0 {
            self.cel = self.turn_timer / 2 + 9;
        } else {
            self.swim_cycle += if every_tick || self.anim_frame <= 1 {
                1
            } else {
                2
            };
            if self.swim_cycle > 19 {
                self.swim_cycle = 0;
            }
            self.cel = self.swim_cycle / 2;
        }
    }

    fn update_facing(&mut self) {
        #[allow(clippy::float_cmp)]
        if self.vx != self.facing && self.vx != 0.0 && self.facing != 0.0 {
            self.facing = self.vx;
        }
    }

    /// `FishTypePet::Update` for id 4 (pet-zorf.md): calm swimming, and
    /// while no alien is in, a level-1 pellet at a hungry tank (slot 81).
    fn update_zorf(&mut self, ctx: PetCtx<'_>, rng: &mut MtRand) -> PetFishOut {
        let mut out = PetFishOut::default();
        self.swim_and_reroll(rng);
        if ctx.aliens.is_empty() {
            self.coin_timer += 1;
            if self.coin_timer >= self.coin_interval && ctx.hungry {
                self.coin_timer = -10;
                let motion = if self.vx < 0.0 { 1 } else { 2 };
                out.food = Some((self.widget_x + 15, self.widget_y + 10, motion));
            }
        }
        self.damping_timers();
        self.drift_by_vx_bucket();
        self.clamp_pet();
        self.edge_steer();
        // Id 4: the cycle steps by 1 every tick; it jumps to 10 on the 9th
        // tick after a spit.
        self.animate_pet(true);
        if self.turn_timer == 0 && self.coin_timer == -1 {
            self.swim_cycle = 10;
        }
        self.update_facing();
        self.integrate();
        out
    }

    /// `FishTypePet::Update` for id 8 (pet-meryl.md): calm swimming, the
    /// song (slot 81) while no alien is in, and the blink draw.
    fn update_meryl(&mut self, aliens: &[Alien], rng: &mut MtRand) -> PetFishOut {
        let mut out = PetFishOut::default();
        self.swim_and_reroll(rng);
        if aliens.is_empty() {
            self.coin_timer += 1;
            if self.coin_timer == self.coin_interval - 100 {
                out.song = Some(true);
            } else if self.coin_timer >= self.coin_interval {
                out.song = Some(false);
                self.coin_timer = 0;
            }
        }
        self.damping_timers();
        self.drift_by_vx_bucket();
        self.clamp_pet();
        self.edge_steer();
        self.animate_pet(false);
        // The blink check (`0x004e000c`), on non-turning ticks at cel 0.
        if self.turn_timer == 0 && self.cel == 0 {
            if self.blink {
                self.blink = false;
            } else {
                self.blink = rng.next_range(10) == 0;
            }
        }
        self.update_facing();
        self.integrate();
        out
    }

    /// `FishTypePet::Update` for id 9 (pet-wadsworth.md): calm swimming;
    /// the gate (slot 82) runs the mouth timer and the shelter, and always
    /// lets the swim modes run. `coin_timer` is the mouth timer, `blink` the
    /// sheltering flag.
    fn update_wadsworth(&mut self, ctx: PetCtx<'_>, rng: &mut MtRand) -> PetFishOut {
        let mut out = PetFishOut::default();
        self.pre_update();
        // The gate: mouth timer, then the threat test.
        if self.coin_timer > 0 {
            self.coin_timer -= 1;
        } else if self.blink && ctx.guppy_outside(self) {
            self.coin_timer = 100;
        }
        if ctx.aliens.is_empty() {
            if self.blink {
                self.blink = false;
                self.coin_timer = 30;
                out.shelter = Some(false);
            }
        } else {
            if !self.blink {
                self.blink = true;
                self.coin_timer = self.coin_interval;
            }
            out.shelter = Some(true);
        }
        self.swim_modes_calm();
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
        out
    }

    /// A calm-swimming pet with its own swim-cycle shape (no ability):
    /// cel = cycle / `div`, the cycle wraps above `wrap`; it steps by 2 once
    /// the speed bucket exceeds `fast_at`.
    fn swim_tail(&mut self, div: i32, wrap: i32, fast_at: i32) {
        self.damping_timers();
        self.drift_by_vx_bucket();
        self.clamp_pet();
        self.edge_steer();
        if self.facing > 0.0 && self.vx < 0.0 {
            self.turn_timer = 20;
        } else if self.facing < 0.0 && self.vx > 0.0 {
            self.turn_timer = -20;
        }
        if self.turn_timer != 0 {
            self.turn_timer -= self.turn_timer.signum();
            if self.turn_timer == 0 {
                self.eat_timer = 0;
            }
        }
        if self.turn_timer > 0 {
            self.cel = 9 - self.turn_timer / 2;
        } else if self.turn_timer < 0 {
            self.cel = self.turn_timer / 2 + 9;
        } else {
            self.swim_cycle += if self.anim_frame <= fast_at { 1 } else { 2 };
            if self.swim_cycle > wrap {
                self.swim_cycle = 0;
            }
            self.cel = self.swim_cycle / div;
        }
        self.update_facing();
        self.integrate();
    }

    /// Seymour (pet-seymour.md): swims, no ability; cel = cycle / 8 (the
    /// cycle wraps above 79). His effect is in the coins' update.
    fn update_seymour(&mut self, rng: &mut MtRand) -> PetFishOut {
        self.swim_and_reroll(rng);
        self.swim_tail(8, 79, 1);
        PetFishOut::default()
    }

    /// Shrapnel (pet-shrapnel.md): swims; every `+0x224` calm ticks a bomb
    /// at (wx + 15, wy + 10); cel = cycle / 6 (cycle wraps at 60).
    fn update_shrapnel(&mut self, aliens: &[Alien], rng: &mut MtRand) -> PetFishOut {
        let mut out = PetFishOut::default();
        self.swim_and_reroll(rng);
        if aliens.is_empty() {
            self.coin_timer += 1;
            if self.coin_timer >= self.coin_interval {
                self.coin_timer = 0;
                out.bomb = Some((self.widget_x + 15, self.widget_y + 10));
            }
        }
        self.swim_tail(6, 59, 2);
        out
    }

    /// Gumbo (pet-gumbo.md): with an alien in he flees to the far corner
    /// (steering every 5th tick, no swim modes); calm, he swims.
    fn update_gumbo(&mut self, aliens: &[Alien], rng: &mut MtRand) -> PetFishOut {
        self.pre_update();
        if aliens.is_empty() {
            self.swim_modes_calm();
        } else {
            self.flee(aliens);
        }
        self.mode_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32 + 1;
            }
        }
        self.swim_tail(2, 19, 1);
        PetFishOut::default()
    }

    /// Gumbo's chase (`0x004e02d0`, id 12): steer by the nearest alien's
    /// centre toward the opposite corner, every 5th tick.
    fn flee(&mut self, aliens: &[Alien]) {
        let mut best = 10_000;
        let mut target = None;
        for (i, a) in aliens.iter().enumerate() {
            let dx = ((self.x + 40.0) - f64::from(a.widget_x + 80)) as i32;
            let dy = ((self.y + 40.0) - f64::from(a.widget_y + 80)) as i32;
            let d = (dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)) as f32).sqrt() as i32;
            if d < best {
                best = d;
                target = Some(i);
            }
        }
        let Some(t) = target else { return };
        if self.mode_timer < 5 {
            return;
        }
        self.mode_timer = 0;
        let (acx, acy) = (aliens[t].widget_x + 80, aliens[t].widget_y + 80);
        if acy >= 261 && self.vy > -8.0 {
            self.vy -= 2.0;
        } else if acy < 300 && self.vy < 8.0 {
            self.vy += 2.0;
        }
        if acx >= 291 && self.vx > -8.0 {
            self.vx -= 2.0;
        } else if acx < 330 && self.vx < 8.0 {
            self.vx += 2.0;
        }
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
    /// Zorf spits a pellet: `AddFood(x, y, motion)`.
    pub food: Option<(i32, i32, i32)>,
    /// Meryl starts (true) or ends (false) her song.
    pub song: Option<bool>,
    /// Wadsworth shelters (true: publish his position) or releases (false).
    pub shelter: Option<bool>,
    /// Shrapnel drops a bomb at this widget position.
    pub bomb: Option<(i32, i32)>,
}

/// What a fish-shaped pet's update reads from the board.
#[derive(Clone, Copy)]
pub struct PetCtx<'a> {
    pub aliens: &'a [Alien],
    /// Some guppy (or breeder) has hunger below 300 (Zorf's check).
    pub hungry: bool,
    pub guppies: &'a [Fish],
}

impl PetCtx<'_> {
    /// `0x004e7010`: some small or medium guppy is more than 10 px off
    /// Wadsworth on **both** axes.
    fn guppy_outside(&self, w: &Fish) -> bool {
        self.guppies.iter().any(|g| {
            g.stage < 2
                && ((w.x + 40.0) - f64::from(g.widget_x + 40)).abs() > 10.0
                && ((w.y + 40.0) - f64::from(g.widget_y + 40)).abs() > 10.0
        })
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
