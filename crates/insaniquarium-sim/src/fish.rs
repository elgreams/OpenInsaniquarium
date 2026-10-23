//! Guppy (`Sexy::Fish`, type 0). Spec: `docs/re/objects/fish-guppy.md`.
//!
//! Field names follow the spec; raw offsets of the original are noted where
//! they aid cross-referencing captures.

use sexy_core::rand::MtRand;

use crate::coin::Coin;
use crate::food::Food;
use crate::message::{self, HINTS, Message};
use crate::sound::{Played, Sound};

/// Guppy state. All positions/velocities are `f64` (x87 doubles in the
/// original); timers and bounds are `i32`.
#[derive(Debug, Clone, PartialEq)]
pub struct Fish {
    /// `+0x158`, `+0x160`
    pub x: f64,
    pub y: f64,
    /// `+0x168`, `+0x170`
    pub vx: f64,
    pub vy: f64,
    /// `+0x180`: position += velocity / divisor.
    pub divisor: f64,
    /// `+0x188`: last non-zero vx (±1.0 at spawn). Turns trigger when vx
    /// changes sign relative to it; drawing uses it when vx == 0.
    pub facing: f64,
    /// `+0x178`: patrol direction (+1 / −1).
    pub patrol_dir: i32,
    /// `+0x190`: floor (max y).
    pub floor_y: i32,
    /// `+0x194`, `+0x19c`: x bounds.
    pub min_x: i32,
    pub max_x: i32,
    /// `+0x198`: ceiling (min y).
    pub ceil_y: i32,
    /// `+0x9c`: counts down; hungry at ≤ 0, dies below −499.
    pub hunger: i32,
    /// `+0x1a0`: 0 small, 1 medium, 2 large, 3/4 king/star.
    pub stage: i32,
    /// `+0x1a4`: pellets eaten toward the next stage.
    pub growth: i32,
    /// `+0x1a8`: pellets needed per stage (4..=6).
    pub growth_needed: i32,
    /// `+0xa4`: hunger at which the hungry colours fade in (300).
    pub hungry_at: i32,
    /// `+0xa8`: the colour fade runs toward hungry (true) or fed.
    pub fade_to_hungry: bool,
    /// `+0x228`: shows hunger messages; starving fish without it die at 0.
    pub announces: bool,
    /// `+0xac`, `+0x11c`: −1 for ordinary guppies.
    pub state_ac: i32,
    pub link_11c: i32,
    /// Set when the fish dies this tick; the board removes it.
    pub dead: bool,
    /// `+0x1ac`: swim mode 0..=9, or 123 (startled).
    pub mode: i32,
    /// `+0x1b0`: ticks in current mode.
    pub mode_timer: i32,
    /// `+0x1b4`: mode-reroll timer (check when > 20).
    pub reroll_timer: i32,
    /// `+0x1bc`: speed bucket, trunc(|vx|); ≤ 1 animates at half rate.
    pub anim_frame: i32,
    /// `+0x1b8`: swim cycle counter 0..20 (cel = counter / 2).
    pub swim_cycle: i32,
    /// `+0x1c0`: sprite-sheet column (cel) to draw.
    pub cel: i32,
    /// `+0x1c4`: turn countdown: +20 (turning to face left) / −20 (to face
    /// right), stepping toward 0.
    pub turn_timer: i32,
    /// `+0x1c8`: eat animation countdown.
    pub eat_timer: i32,
    /// `+0x1cc`: pulse/stretch countdown (draw effect).
    pub pulse_timer: i32,
    /// `+0x108`, `+0x10c`, `+0xf8`: countdowns decremented by PreUpdate.
    /// `+0x108` > 100 holds the eat cel (4).
    pub timer_108: i32,
    pub timer_10c: i32,
    pub timer_f8: i32,
    /// `+0xa0`: hungry→fed colour fade (0 = none, drawn at `n·255/5`).
    pub fade: i32,
    /// `+0x208`: vy damping timer (also gates bubbles).
    pub vy_damp_timer: i32,
    /// `+0x20c`, `+0x218`: x push timer and push velocity.
    pub push_timer: i32,
    pub push: f64,
    /// `+0x210`
    pub misc_timer: i32,
    /// `+0x220`, `+0x224`: coin timer and interval.
    pub coin_timer: i32,
    pub coin_interval: i32,
    /// `+0x30`, `+0x34`: integer widget position (truncated x/y).
    pub widget_x: i32,
    pub widget_y: i32,
}

/// Board facts a guppy update reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct FishEnv {
    /// `board+0x94`
    pub paused: bool,
    /// Any alien on the board (`board+0xb8` or `+0xf4` non-empty).
    pub has_aliens: bool,
}

/// Board state a guppy update reads and writes.
pub struct World<'a> {
    pub foods: &'a mut Vec<Food>,
    pub coins: &'a mut Vec<Coin>,
    pub rng: &'a mut MtRand,
    pub global: &'a mut MtRand,
    /// Board update counter (`board+0x28`), the sound-throttle clock.
    pub clock: i32,
    /// Per-sample last-played clock (`board+0x1a4`, indexed by id − 0x106).
    pub sample_last: &'a mut [i32; 64],
    /// Sounds started this tick (presentation only).
    pub sounds: &'a mut Vec<Played>,
    /// New stages reached this tick (`Fish::OnStageUp` triggers), in order.
    pub stage_ups: &'a mut Vec<i32>,
    /// The message line and one-time hint bytes (board.md#message-line).
    pub message: &'a mut Message,
    pub hints: &'a mut [bool; HINTS],
    /// Tutorial 1-1 (`0x00537b80`).
    pub tutorial: bool,
}

impl World<'_> {
    /// `0x005381f0`: may sample `id` play now (≥ `gap` ticks since last)?
    /// Records the play when it may.
    pub fn sample_ready(&mut self, id: usize, gap: i32) -> bool {
        let slot = &mut self.sample_last[id - 0x106];
        if self.clock.wrapping_sub(*slot) >= gap {
            *slot = self.clock;
            true
        } else {
            false
        }
    }

    /// `0x005384c0`: slurp (sample 0x138), variant picked by one draw.
    fn play_slurp(&mut self) {
        if self.sample_ready(0x138, 3) {
            let r = self.rng.next_range(5);
            self.sounds.push(Sound::slurp(r).into());
        }
    }
}

impl Fish {
    /// `Fish::Init` (`0x004de370`): 6 app-RNG draws, in this order.
    pub fn new(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut f = Fish {
            x: f64::from(x),
            y: f64::from(y),
            vx: 0.0,
            vy: -0.5,
            divisor: 1.0,
            facing: 1.0,
            patrol_dir: 1,
            floor_y: 370,
            min_x: 10,
            max_x: 540,
            ceil_y: 95,
            hunger: 0,
            stage: 0,
            growth: 0,
            growth_needed: 0,
            hungry_at: 300,
            fade_to_hungry: false,
            announces: true,
            state_ac: -1,
            link_11c: -1,
            dead: false,
            mode: 0,
            mode_timer: 40,
            reroll_timer: 0,
            anim_frame: 0,
            swim_cycle: 0,
            cel: 0,
            turn_timer: 0,
            eat_timer: 0,
            pulse_timer: 0,
            timer_108: 0,
            timer_10c: 0,
            timer_f8: 0,
            fade: 0,
            vy_damp_timer: 0,
            push_timer: 0,
            push: 0.0,
            misc_timer: 0,
            coin_timer: 0,
            coin_interval: 0,
            widget_x: x,
            widget_y: y,
        };
        if rng.next_u31() & 1 == 0 {
            f.vx = -0.1;
            f.facing = -1.0;
        }
        f.divisor = [2.0, 1.8, 1.6][rng.next_range(3) as usize];
        f.hunger = rng.next_range(200) as i32 + 400;
        f.growth_needed = rng.next_range(3) as i32 + 4;
        f.mode = rng.next_range(10) as i32;
        f.coin_interval = rng.next_range(200) as i32 + 150;
        f
    }

    /// `Fish::Update` (`0x004f6970`).
    ///
    /// TODO(spec): the cursor-follow branch, coin drop and alien-present
    /// behaviour are not modelled yet.
    pub fn update(&mut self, env: FishEnv, w: &mut World<'_>) {
        if env.paused {
            return;
        }
        self.pre_update();
        if self.dead {
            return;
        }
        if !self.update_hunger(env, w) {
            self.swim_modes(w.rng, w.global);
        }
        // Mode reroll, then the coin drop (slot 81) when there are no aliens.
        self.mode_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if w.rng.next_range(10) == 0 {
                self.mode = w.rng.next_range(9) as i32 + 1;
            }
        }
        if !env.has_aliens {
            self.update_coin(w);
        }
        let rng = &mut *w.rng;
        self.damping_timers();
        self.drift_by_vx_bucket();
        if self.floor_y <= 320 {
            self.y -= 0.25;
        }
        self.clamp_and_bubbles(rng);
        self.edge_steer();
        self.animate();
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
    }

    /// `Fish::UpdateHunger` (`0x004f15c0`). Returns true while chasing food,
    /// which suppresses the swim modes. Spec: food.md#guppy-hunger-and-feeding.
    fn update_hunger(&mut self, env: FishEnv, w: &mut World<'_>) -> bool {
        // Colour fade step.
        if self.fade > 0 {
            if self.fade_to_hungry {
                self.fade += 1;
                if self.fade > 5 {
                    self.fade = 0;
                }
            } else {
                self.fade -= 1;
            }
        }
        if !env.has_aliens {
            if self.link_11c == -1 {
                self.hunger -= 1;
            }
            if self.hunger == self.hungry_at + 4 {
                self.start_fade(true);
            } else if self.hunger < -1000 {
                self.hunger = -1000;
            }
        }
        let starving = self.state_ac < 0 && self.hunger <= 0;
        if starving && !self.announces {
            self.die(w);
            return false;
        }
        if self.announces {
            // Message ticks (−1, −200, −400) skip the chase for that tick.
            match self.hunger {
                -1 => {
                    let text = "Fish are hungry! Click to drop food!";
                    message::show_chain(w.message, w.hints, text, true, [8, 0x17, 0x18]);
                    return false;
                }
                -200 => {
                    if !w.hints[0x15] {
                        let text = "Fish are REALLY hungry! Click to feed!";
                        message::show(w.message, w.hints, text, true, 0x15);
                        if w.sample_ready(0x106, 3) {
                            w.sounds.push(Sound::Awooga.into());
                        }
                    }
                    return false;
                }
                -400 => {
                    let text = "Fish are about to die of hunger! Click on aquarium!";
                    message::show(w.message, w.hints, text, true, 0x16);
                    return false;
                }
                _ => {}
            }
            if self.hunger < -499 {
                // TODO(spec): DeadFish spawn and the mercy-fish rule.
                self.die(w);
                return false;
            }
        }
        if self.hunger < 500 {
            // TODO(spec): with aliens present only alien type ≠ 4 allows it.
            return self.chase_food(w);
        }
        false
    }

    /// Death (slot 88 @ `0x004d5f90`): the throttled die sound. No RNG.
    fn die(&mut self, w: &mut World<'_>) {
        self.dead = true;
        if w.sample_ready(0x117, 3) {
            w.sounds.push(Sound::Die.into());
        }
    }

    /// `Fish::UpdateCoin` (slot 81 @ `0x004f1400`), normal game.
    fn update_coin(&mut self, w: &mut World<'_>) {
        if self.stage <= 0 && self.state_ac < 0 {
            return;
        }
        self.coin_timer += 1;
        if self.coin_timer < self.coin_interval {
            return;
        }
        self.coin_timer = 0;
        // TODO(spec): `+0x22a` (gold-capable) and pet-linked `+0x124` tables.
        if self.stage > 0 {
            w.coins
                .push(Coin::new(self.widget_x + 5, self.widget_y + 10, self.stage));
        }
        // After every drop attempt on 1-1 (board.md#message-line).
        if w.tutorial {
            let text = "Click on coins for extra money!";
            message::show_chain(w.message, w.hints, text, true, [5, 0x19, 0x1a]);
        }
    }

    /// `0x004d6920`: start the hungry (true) or fed (false) colour fade.
    fn start_fade(&mut self, to_hungry: bool) {
        self.fade_to_hungry = to_hungry;
        self.fade = if to_hungry { 1 } else { 5 };
    }

    /// `0x004d6f30`: drawn with the hungry sheet.
    pub fn looks_hungry(&self) -> bool {
        self.hunger < 301 && self.fade == 0
    }

    /// Food chase (slot 84 @ `0x004d5b20`).
    fn chase_food(&mut self, w: &mut World<'_>) -> bool {
        let target = self.find_food(w.foods);
        let fx = (self.x + 40.0) as i32;
        let fy = (self.y + 40.0) as i32;
        let Some(ti) = target else {
            return false;
        };
        if self.mode_timer >= 3 {
            self.mode_timer = 0;
            let f = &w.foods[ti];
            let (tx, ty) = (f.widget_x + 20, f.widget_y + 20);
            self.steer_to(fx, fy, tx, ty);
            if self.anim_frame < 5 {
                self.anim_frame += 1;
            }
        }
        self.eat_check(w);
        true
    }

    fn steer_to(&mut self, fx: i32, fy: i32, tx: i32, ty: i32) {
        // (big, mid, cap) for x; (far_down, near_down, down_cap,
        // far_up, near_up, up_cap) for y.
        let hungry = self.hunger <= 300;
        let (big, mid, cap) = if hungry {
            (1.3, 0.2, 4.0)
        } else {
            (1.0, 0.1, 3.0)
        };
        let (fd, nd, dcap, fu, nu, ucap) = if hungry {
            (1.0, 0.5, -3.0, 1.3, 0.7, 4.0)
        } else {
            (0.6, 0.3, -2.0, 1.0, 0.5, 3.0)
        };
        let (dec, inc) = if fx > tx + 8 {
            (Some(big), None)
        } else if fx < tx - 8 {
            (None, Some(big))
        } else if fx > tx + 4 {
            (Some(mid), None)
        } else if fx < tx - 4 {
            (None, Some(mid))
        } else if fx > tx {
            (Some(0.05), None)
        } else if fx < tx {
            (None, Some(0.05))
        } else {
            (None, None)
        };
        if let Some(d) = dec
            && -cap < self.vx
        {
            self.vx -= d;
        }
        if let Some(i) = inc
            && cap > self.vx
        {
            self.vx += i;
        }
        if fy > ty + 6 {
            if dcap < self.vy {
                self.vy -= fd;
            }
        } else if fy < ty - 6 {
            if ucap > self.vy {
                self.vy += fu;
            }
        } else if fy > ty {
            if dcap < self.vy {
                self.vy -= nd;
            }
        } else if fy < ty && ucap > self.vy {
            self.vy += nu;
        }
    }

    /// Find food (slot 85 @ `0x004ef050`): the first nearest edible pellet.
    fn find_food(&mut self, foods: &[Food]) -> Option<usize> {
        let mut best = 100_000_000;
        let mut found = None;
        for (i, f) in foods.iter().enumerate() {
            let dx = (f64::from(f.widget_x + 20) - (self.x + 40.0)) as i32;
            let dy = (f64::from(f.widget_y + 20) - (self.y + 40.0)) as i32;
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if f.edible() && d < best {
                best = d;
                found = Some(i);
            }
        }
        if best < 10_000 {
            self.timer_f8 = 100;
        }
        found
    }

    /// Eat check (slot 86 @ `0x004ef1b0`).
    fn eat_check(&mut self, w: &mut World<'_>) {
        let cx = self.x + 40.0;
        let cy = self.y + 40.0;
        let mut i = 0;
        while i < w.foods.len() {
            let f = &w.foods[i];
            let (wx, wy) = (f64::from(f.widget_x), f64::from(f.widget_y));
            let bite = wx + 35.0 > cx && wx + 5.0 < cx && wy + 35.0 > cy && wy < cy;
            if bite && f.edible() {
                let food = w.foods.remove(i);
                self.eat(&food, w);
                if self.eat_timer == 0 {
                    self.eat_timer = 8;
                }
                return;
            }
            if self.eat_timer == 0
                && wx + 50.0 > cx
                && wx - 10.0 < cx
                && wy + 40.0 > cy
                && wy - 5.0 < cy
                && f.edible()
            {
                self.eat_timer = 20;
            }
            i += 1;
        }
    }

    /// Eat (slot 77 @ `0x004f1ba0`), pellet outside Virtual Tank.
    fn eat(&mut self, food: &Food, w: &mut World<'_>) {
        let was_hungry = self.looks_hungry();
        w.play_slurp();
        self.timer_f8 = 0;
        // Pellet levels (`0x004f1ba0`): 0 +500 cap 800 growth 1; 1 +700 cap
        // 1000 growth 2; 2 +1100 cap 1400 growth 3. TODO(spec): level 3
        // (star potion: stage-ups and explosions, tank 2).
        let (gain, cap, growth) = match food.level {
            0 => (500, 800, 1),
            1 => (700, 1000, 2),
            _ => (1100, 1400, 3),
        };
        self.hunger += gain;
        if food.level == 0 && self.announces {
            self.hunger += 200;
        }
        if self.hunger > cap {
            self.hunger = cap;
        }
        self.growth += growth;
        if self.growth >= self.growth_needed {
            if self.stage < 2 {
                self.stage += 1;
                self.growth = 0;
                self.pulse_timer = 10;
                w.stage_ups.push(self.stage);
                if w.sample_ready(0x122, 3) {
                    w.sounds.push(Sound::Grow.into());
                }
            }
            if self.growth >= 15 * self.growth_needed && self.stage >= 2 && self.stage != 4 {
                self.stage = 4;
            }
        }
        if was_hungry && !self.looks_hungry() {
            self.start_fade(false);
        }
    }

    /// `Fish::PreUpdate`: countdowns. (Its emote-bubble branch needs
    /// `+0x114` > 0, which tank 1-1 guppies never set.)
    fn pre_update(&mut self) {
        for t in [&mut self.timer_108, &mut self.timer_10c, &mut self.timer_f8] {
            if *t != 0 {
                *t -= 1;
            }
        }
    }

    /// Animation/turn update (vtable slot 89). Spec: fish-guppy.md#drawing.
    fn animate(&mut self) {
        if self.facing > 0.0 && self.vx < 0.0 {
            self.turn_timer = 20;
        } else if self.facing < 0.0 && self.vx > 0.0 {
            self.turn_timer = -20;
        }
        if self.turn_timer != 0 {
            self.turn_timer -= self.turn_timer.signum();
            if self.eat_timer != 0 {
                self.eat_timer -= 1;
            }
        }
        if self.turn_timer > 0 {
            self.cel = 9 - self.turn_timer / 2;
        } else if self.turn_timer < 0 {
            self.cel = self.turn_timer / 2 + 9;
        } else if self.eat_timer > 0 {
            self.eat_timer -= 1;
            self.cel = 9 - self.eat_timer / 2;
        } else {
            self.swim_cycle += if self.anim_frame <= 1 { 1 } else { 2 };
            if self.swim_cycle >= 20 {
                self.swim_cycle = 0;
            }
            self.cel = self.swim_cycle / 2;
        }
        if self.pulse_timer > 0 {
            self.pulse_timer -= 1;
        }
        #[allow(clippy::float_cmp)]
        if self.vx != self.facing && self.vx != 0.0 && self.facing != 0.0 {
            self.facing = self.vx;
        }
        if self.timer_108 > 100 && self.turn_timer == 0 {
            self.cel = 4;
        }
    }

    fn swim_modes(&mut self, rng: &mut MtRand, global: &mut MtRand) {
        match self.mode {
            123 => {
                self.anim_frame = self.vx.abs() as i32;
                self.reroll_timer = 0;
                if self.mode_timer > 30 {
                    if self.vx.abs() > 2.0 {
                        self.vx *= 0.95;
                    }
                    if self.vy.abs() > 2.0 {
                        self.vy *= 0.95;
                    }
                }
                if self.mode_timer > 50 {
                    self.mode = rng.next_range(9) as i32 + 1;
                    self.mode_timer = 0;
                    let near_edge =
                        f64::from(self.max_x - 20) < self.x || self.x < f64::from(self.min_x + 20);
                    if near_edge || global.next_u31().is_multiple_of(3) {
                        self.vx = -self.vx;
                    }
                }
            }
            m if m > 4 => self.patrol(),
            0 => {
                if self.vy_damp_timer == 0 {
                    self.vy = 0.5;
                }
                if self.mode_timer >= 40 {
                    self.mode_timer = 0;
                    // Ease vx toward 0 in 0.5 steps; |vx| == 0.5 is left alone.
                    if self.vx > 0.5 {
                        self.vx -= 0.5;
                    } else if self.vx < -0.5 {
                        self.vx += 0.5;
                    }
                    self.anim_frame = self.vx.abs() as i32;
                }
                self.y -= 0.25 / self.divisor;
            }
            1 | 2 => {
                if self.vy_damp_timer == 0 {
                    self.vy = -0.5;
                }
                if self.mode_timer >= 40 {
                    self.mode_timer = 0;
                    let target = if self.mode == 1 { 1.0 } else { -1.0 };
                    if self.vx > target {
                        self.vx -= 1.0;
                    } else if self.vx < target {
                        self.vx += 1.0;
                    }
                    self.anim_frame = self.vx.abs() as i32;
                }
                self.y -= 0.5 / self.divisor;
            }
            3 | 4 => {
                if self.mode_timer >= 40 {
                    self.mode_timer = 0;
                    let target = if self.mode == 3 { -1.0 } else { 1.0 };
                    if self.vx > target {
                        self.vx -= 1.0;
                    } else if self.vx < target {
                        self.vx += 1.0;
                    }
                    if self.vy > 3.0 {
                        self.vy -= 1.0;
                    } else if self.vy < 3.0 {
                        self.vy += 1.0;
                    }
                    if self.anim_frame > 4 {
                        self.anim_frame -= 1;
                    } else if self.vy < 4.0 {
                        self.anim_frame += 1;
                    }
                }
                if self.y > 240.0 {
                    self.mode = 0;
                }
            }
            _ => {}
        }
    }

    /// Modes 5–9.
    fn patrol(&mut self) {
        if self.vy_damp_timer == 0 {
            self.vy = if self.y < 115.0 { -0.1 } else { -0.5 };
        }
        if self.mode_timer < 40 {
            return;
        }
        self.mode_timer = 0;
        // Accelerate along the patrol direction, harder while still moving
        // against it.
        match self.patrol_dir {
            1 => {
                self.vx += if self.vx < 0.0 { 2.0 } else { 1.0 };
                self.anim_frame = self.vx.abs() as i32;
                if self.x > 250.0 {
                    self.patrol_dir = -1;
                    self.vx -= 2.0;
                }
            }
            -1 => {
                self.vx -= if self.vx > 0.0 { 2.0 } else { 1.0 };
                self.anim_frame = self.vx.abs() as i32;
                if self.x < 175.0 {
                    self.patrol_dir = 1;
                    self.vx += 2.0;
                }
            }
            _ => {}
        }
    }

    fn damping_timers(&mut self) {
        if self.vy_damp_timer != 0 {
            self.vy *= 0.9;
            self.vy_damp_timer -= 1;
        }
        if self.push_timer != 0 {
            self.push *= 0.9;
            self.push_timer -= 1;
            self.x += self.push;
        }
        if self.misc_timer > 0 {
            self.misc_timer -= 1;
        }
    }

    /// Exact-equality buckets on vx (reproduced deliberately).
    fn drift_by_vx_bucket(&mut self) {
        #[allow(clippy::float_cmp)]
        {
            if self.vx == 0.0 {
                self.y += 1.0 / self.divisor;
            }
            if self.vx == 1.0 {
                self.y += 0.75 / self.divisor;
            }
            if self.vx == 2.0 {
                self.y += 0.5 / self.divisor;
            }
            if self.vx == 3.0 {
                self.y += 0.25 / self.divisor;
            }
        }
    }

    fn clamp_and_bubbles(&mut self, rng: &mut MtRand) {
        if f64::from(self.max_x) < self.x {
            self.x = f64::from(self.max_x);
        }
        if self.x < f64::from(self.min_x) {
            self.x = f64::from(self.min_x);
        }
        if f64::from(self.floor_y) < self.y {
            self.y = f64::from(self.floor_y);
        }
        if self.vy_damp_timer > 0 && self.vy > 0.0 {
            // Bubble emission while rising out of a startle; positions only
            // feed the (cosmetic) bubble system but the RNG draws matter.
            if self.vy_damp_timer > 30 {
                let n = if self.vy_damp_timer > 40 { 1 } else { 2 };
                if rng.next_u31().is_multiple_of(n) {
                    self.emit_bubbles(rng);
                }
            }
        } else if self.y < f64::from(self.ceil_y) {
            self.y = f64::from(self.ceil_y);
        }
    }

    fn emit_bubbles(&mut self, rng: &mut MtRand) {
        if self.stage == 0 {
            if rng.next_u31() & 1 != 0 {
                return;
            }
            rng.next_range(30);
            rng.next_range(30);
        } else {
            // (Object type 6 uses a 3-pair %120/%80/%80 pattern; guppies are
            // type 0 so it never applies here.)
            rng.next_range(60);
            rng.next_range(60);
            rng.next_range(40);
            rng.next_range(40);
        }
    }

    fn edge_steer(&mut self) {
        if f64::from(self.max_x - 5) < self.x && self.vx > 0.1 {
            self.vx -= 0.1;
        }
        if self.x < 15.0 && self.vx < -0.1 {
            self.vx += 0.1;
        }
    }
}

/// `DeadFish` (type 0x1b, list `board+0x98`): a dead guppy rolling over,
/// sinking and fading (`fish-guppy.md#dead-fish`). Cosmetic: no RNG.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadFish {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub divisor: f64,
    /// `+0x1a4`: the guppy's stage (the sprite row).
    pub kind: i32,
    /// `+0x158`: faces right (the art is mirrored).
    pub facing_right: bool,
    /// `+0x18c`
    pub cel: i32,
    /// `+0x190`
    pub turn: i32,
    /// `+0x198`, 0..=1
    pub alpha: f64,
    /// `+0x1a0`: counts down from 125; removed below 1.
    pub life: i32,
    pub widget_x: i32,
    pub widget_y: i32,
}

impl DeadFish {
    /// `DeadFish::DeadFish` (`0x004eed20`) from a dying guppy.
    pub fn from_fish(f: &Fish) -> Self {
        // `Fish::Die` passes the position as ints.
        let (x, y) = (f64::from(f.x as i32), f64::from(f.y as i32));
        let vy = if y >= 115.0 && f.vy >= -3.0 {
            f.vy - 2.0
        } else {
            f.vy - 1.0
        };
        DeadFish {
            x,
            y,
            vx: f.vx,
            vy,
            divisor: f.divisor,
            kind: f.stage,
            facing_right: f.vx > 0.0,
            cel: 0,
            turn: 100,
            alpha: 1.0,
            life: 125,
            widget_x: x as i32,
            widget_y: y as i32,
        }
    }

    /// `DeadFish::Update` (`0x004f64e0`), unpaused ticks. Returns false
    /// when it's gone.
    pub fn update(&mut self) -> bool {
        if (0..=10).contains(&self.turn) {
            self.turn -= 1;
            self.cel = self.turn;
            if self.turn == 10 {
                self.turn = 10;
            }
        } else {
            self.cel = match self.life {
                l if l > 105 => 9 - (l - 106) / 2,
                103 | 104 => 8,
                101 | 102 => 7,
                l if l > 100 => 9,
                _ => 6,
            };
        }
        if self.life < 105 {
            self.alpha = (self.alpha - 0.02).max(0.0);
        }
        if self.life < 1 {
            return false;
        }
        if self.y > 370.0 || self.life > 105 {
            self.life -= 1;
        }
        if self.vx > 0.0 {
            self.vx = (self.vx - 0.03).max(0.0);
        } else if self.vx < 0.0 {
            self.vx = (self.vx + 0.03).min(0.0);
        }
        if self.vy < 2.0 {
            self.vy += 0.05;
        }
        self.y += self.vy / self.divisor;
        self.x += self.vx / self.divisor;
        self.y = self.y.clamp(85.0, 380.0);
        self.x = self.x.clamp(10.0, 540.0);
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        true
    }
}
