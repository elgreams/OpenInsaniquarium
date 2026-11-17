//! The Breeder (`Sexy::Breeder`, type `0x0a`, board list `+0xc0`). Spec:
//! `docs/re/objects/breeder.md`.
//!
//! A direct `GameObject` subclass (like `Starcatcher` and `Guppycruncher`),
//! **not** a `Fish`. Eats food pellets and spawns baby guppies periodically.
//! Replaces the guppy buy for the tank 4 economy.

use sexy_core::rand::MtRand;

use crate::food::Food;

#[derive(Debug, Clone, PartialEq)]
pub struct Breeder {
    /// `+0x158`, `+0x160`, `+0x168`, `+0x170`
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// `+0x180`: speed divisor (2.0).
    pub divisor: f64,
    /// `+0x9c`: hunger (decrements by 1 per tick when no aliens present).
    pub hunger: i32,
    /// `+0xa0`: colour fade counter.
    pub fade: i32,
    /// Whether fading toward hungry (true) or recovering (false).
    pub fade_to_hungry: bool,
    /// `+0x178`: splash timer (bubbles for the first 45 ticks).
    pub splash: i32,
    /// `+0x188`: the swim mode's target vx.
    pub target_vx: f64,
    /// `+0x190`: swim mode / chase state.
    pub mode: i32,
    /// `+0x194`: chase-steer timer; `+0x198`: reroll timer.
    pub chase_timer: i32,
    pub reroll_timer: i32,
    /// `+0x19c`: animation counter.
    pub anim: i32,
    /// `+0x1a0`: stage (dead fish kind = stage + 10).
    pub stage: i32,
    /// `+0x1a4`: growth counter (preset to 2 at level start).
    pub growth: i32,
    /// `+0x1a8`: guppies spawned.
    pub spawned: i32,
    /// `+0x1b4`: spawn counter (ticks since last spawn or start).
    pub spawn_counter: i32,
    /// `+0x1d4`: spawn interval.
    pub spawn_interval: i32,
    /// `+0x1d8`: vy-damp timer (Rhubarb sets to 50).
    pub damp_timer: i32,
    /// Animation sub-counters (`+0x1b8`, `+0x1bc`).
    pub anim_b: i32,
    pub anim_c: i32,
    /// Animation cel (`+0x1c4`).
    pub cel: i32,
    pub widget_x: i32,
    pub widget_y: i32,
    /// The shadow's floor y (cosmetic).
    pub shadow_y: i32,
}

/// What a Breeder's update asks the board to do.
#[derive(Debug, Default)]
pub struct BreederOut {
    /// Ate food `i` (removed).
    pub ate_food: Option<usize>,
    /// Spawn a baby guppy at these board coordinates.
    pub spawn_guppy: Option<(i32, i32)>,
    /// Starved: die (sound, corpse).
    pub died: bool,
    /// Position and vx when it died.
    pub died_at: (f64, f64, f64),
}

impl Breeder {
    /// `Board::BuyBreeder` (`0x005470a0`): the x draw, then the ctor draws,
    /// y overwritten to 65 (drop from top). `static`.
    pub fn bought(rng: &mut MtRand) -> Self {
        let x = rng.next_range(520) as i32 + 20;
        // Ctor draws: y, divisor (unused?), hunger, mode, interval —
        // by analogy with the Grubber (5 draws). `static`.
        let _ctor_y = rng.next_range(5);
        let _divisor = rng.next_range(3); // R2
        let hunger = rng.next_range(200) as i32 + 900;
        let mode = rng.next_range(10) as i32;
        let spawn_interval = rng.next_range(200) as i32 + 300;
        Breeder {
            x: f64::from(x),
            y: 65.0,
            vx: 0.0,
            vy: 0.0,
            divisor: 2.0,
            hunger,
            fade: 0,
            fade_to_hungry: false,
            splash: 45,
            target_vx: 0.0,
            mode,
            chase_timer: 40,
            reroll_timer: 0,
            anim: 0,
            stage: 0,
            growth: 0,
            spawned: 0,
            spawn_counter: 0,
            spawn_interval,
            damp_timer: 0,
            anim_b: 0,
            anim_c: 0,
            cel: 0,
            widget_x: x,
            widget_y: 65,
            shadow_y: 410,
        }
    }

    /// Level start (`0x005471e0`): one breeder at a random position (2 draws),
    /// then ctor draws. `static`.
    pub fn level_start(rng: &mut MtRand) -> Self {
        let x = rng.next_range(520) as i32 + 20;
        let y = rng.next_range(265) as i32 + 105;
        // Ctor draws: same as `bought`.
        let _ctor_y = rng.next_range(5);
        let _divisor = rng.next_range(3);
        let hunger = rng.next_range(200) as i32 + 900;
        let mode = rng.next_range(10) as i32;
        let spawn_interval = rng.next_range(200) as i32 + 300;
        let mut b = Breeder {
            x: f64::from(x),
            y: f64::from(y),
            vx: 0.0,
            vy: 0.0,
            divisor: 2.0,
            hunger,
            fade: 0,
            fade_to_hungry: false,
            splash: 0,
            target_vx: 0.0,
            mode,
            chase_timer: 40,
            reroll_timer: 0,
            anim: 0,
            stage: 0,
            growth: 2,
            spawned: 0,
            spawn_counter: 0,
            spawn_interval,
            damp_timer: 0,
            anim_b: 0,
            anim_c: 0,
            cel: 0,
            widget_x: x,
            widget_y: y,
            shadow_y: 410,
        };
        // Shadow draw.
        b.shadow_y = rng.next_range(5) as i32 + 410;
        b
    }

    /// `Breeder::Update`, normal game. `static` (the exact update logic
    /// is not known from Ghidra; this is a best-guess model based on the
    /// Starcatcher/Guppycruncher pattern and capture observation).
    pub fn update(
        &mut self,
        has_aliens: bool,
        foods: &[Food],
        rng: &mut MtRand,
    ) -> BreederOut {
        let mut out = BreederOut::default();

        // Hunger fade.
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
        // Hunger countdown (no aliens gate).
        if !has_aliens {
            self.hunger -= 1;
            if self.hunger == 304 {
                self.fade_to_hungry = true;
                self.fade = 1;
            } else if self.hunger < -1000 {
                self.hunger = -1000;
            }
        }
        // Starving check.
        let busy = if self.hunger <= 0 {
            out.died = true;
            out.died_at = (self.x, self.y, self.vx);
            false
        } else if self.hunger < 900 {
            self.chase(foods, &mut out)
        } else {
            false
        };

        // Swim modes (when not chasing).
        if !busy {
            match self.mode {
                0 | 5 | 6 => self.target_vx = 0.0,
                1 => self.target_vx = -0.5,
                2 => self.target_vx = 0.5,
                3 => self.target_vx = -1.0,
                4 => self.target_vx = 1.0,
                7 => self.target_vx = -2.5,
                8 => self.target_vx = 2.5,
                _ => {}
            }
            if self.target_vx < self.vx {
                self.vx -= 0.1;
            }
            if self.vx < self.target_vx {
                self.vx += 0.1;
            }
        }

        // Timers and reroll.
        self.chase_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 || self.x <= 10.0 || self.x >= 540.0 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32;
            }
        }

        // Clamps.
        #[allow(clippy::manual_clamp)]
        {
            if self.x > 550.0 {
                self.x = 550.0;
            }
            if self.x < 10.0 {
                self.x = 10.0;
            }
        }
        if self.y > 359.0 {
            self.y = 359.0;
            self.vy = 0.0;
        }

        // Splash (bubbles while dropping in).
        if self.splash >= 1 {
            let n = if self.splash > 35 { 8 } else { 5 };
            if rng.next_range(n) == 0 {
                rng.next_u31();
                rng.next_u31();
            }
            self.splash -= 1;
        } else if self.y < 95.0 {
            self.y = 95.0;
        }

        // Edge steering.
        if self.x > 535.0 && self.vx > 0.1 {
            self.vx -= 0.1;
        }
        if self.x < 15.0 && self.vx < -0.1 {
            self.vx += 0.1;
        }

        // Gravity.
        if self.y < 359.0 {
            self.vy += 0.4;
        }

        // Guppy spawn timer (Prego-like, `static`).
        if !has_aliens {
            self.spawn_counter += 1;
            if self.spawn_counter >= self.spawn_interval {
                self.spawn_counter = 0;
                out.spawn_guppy = Some((self.widget_x, self.widget_y));
                self.spawned += 1;
            }
        }

        // Animation (shared with Starcatcher, `0x004d8f10`).
        if self.vx >= 1.0 {
            self.anim = (self.anim + 1) % 20;
            self.cel = self.anim / 2;
        } else if self.vx <= -1.0 {
            self.anim = (self.anim + 19) % 20;
            self.cel = self.anim / 2;
        } else {
            self.anim = if self.vx > 0.0 {
                (self.anim + 1) % 40
            } else {
                (self.anim + 39) % 40
            };
            self.cel = self.anim / 4;
        }

        // Integrate.
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        out
    }

    /// Food chase: steer toward the nearest food pellet, eat the first
    /// in reach. `static` (modelled on the Guppycruncher's chase).
    fn chase(&mut self, foods: &[Food], out: &mut BreederOut) -> bool {
        let (px, py) = (self.x + 40.0, self.y + 40.0);
        let mut best = 100_000_000;
        let mut prey = None;
        for (i, f) in foods.iter().enumerate() {
            if f.delay > 0 || f.kind == 2 {
                continue; // skip delayed food and potions
            }
            let dx = (f64::from(f.widget_x) - px) as i32;
            let dy = (f64::from(f.widget_y) - py) as i32;
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                best = d;
                prey = Some(i);
            }
        }

        if self.chase_timer >= 5 {
            let Some(pi) = prey else { return false };
            self.chase_timer = 0;
            let f = &foods[pi];
            let tx = f64::from(f.widget_x);
            let ty = f64::from(f.widget_y);
            let hungry = self.hunger <= 300;
            let x_cap = if hungry { 5.0 } else { 4.0 };
            if px > tx + 8.0 {
                if self.vx > -x_cap {
                    self.vx -= 1.5;
                }
            } else if px < tx - 8.0 {
                if self.vx < x_cap {
                    self.vx += 1.5;
                }
            } else if px > tx + 4.0 {
                if self.vx > -x_cap {
                    self.vx -= 0.2;
                }
            } else if px < tx - 4.0 {
                if self.vx < x_cap {
                    self.vx += 0.2;
                }
            } else if px > tx {
                if self.vx > -x_cap {
                    self.vx -= if hungry { 0.05 } else { 0.1 };
                }
            } else if px < tx && self.vx < x_cap {
                self.vx += if hungry { 0.05 } else { 0.1 };
            }
            // Y steering.
            let y_cap_down = -3.0;
            let y_cap_up = 4.0;
            if py > ty + 6.0 {
                if self.vy > y_cap_down {
                    self.vy -= if hungry { 1.3 } else { 0.8 };
                }
            } else if py < ty - 6.0 {
                if self.vy < y_cap_up {
                    self.vy += if hungry { 1.5 } else { 1.3 };
                }
            } else if py > ty {
                if self.vy > y_cap_down {
                    self.vy -= if hungry { 0.5 } else { 0.3 };
                }
            } else if py < ty && self.vy < y_cap_up {
                self.vy += if hungry { 0.7 } else { 0.5 };
            }
        }

        if prey.is_none() {
            return false;
        }

        // Bite: the first food pellet in the bite box.
        let bite_idx = foods.iter().position(|f| {
            if f.delay > 0 || f.kind == 2 {
                return false;
            }
            let (fx, fy) = (f64::from(f.widget_x), f64::from(f.widget_y));
            fx + 10.0 < px && px < fx + 70.0 && fy + 10.0 < py && py < fy + 60.0
        });
        if let Some(i) = bite_idx {
            let was_hungry = self.hunger < 301 && self.fade == 0;
            self.hunger = (self.hunger + 700).min(800);
            if was_hungry && !(self.hunger < 301 && self.fade == 0) {
                self.fade_to_hungry = false;
                self.fade = 5;
            }
            out.ate_food = Some(i);
        }
        true
    }
}
