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
    /// `+0x178`: patrol direction (1 or -1). Used by modes 5–9 (patrol).
    pub patrol_dir: i32,
    /// Splash timer (bought breeder enters from above; cosmetic in binary
    /// at `+0x128`, but gates the ceiling clamp in the sim).
    pub splash: i32,
    /// `+0x1b0`: swim mode (0–9).
    pub mode: i32,
    /// `+0x1b8`: mode-reroll timer (fires when > 20, 21-tick cycle).
    pub reroll_timer: i32,
    /// `+0x1c0`: animation frame (cosmetic, but affects modes 3/4 branching).
    pub anim: i32,
    /// `+0x1a0`: stage (dead fish kind = stage + 10).
    pub stage: i32,
    /// `+0x1a4`: growth counter (preset to 2 at level start).
    pub growth: i32,
    /// `+0x1a8`: growth threshold (pellets needed per stage, [4..8]).
    /// Re-drawn from app RNG on each stage advance.
    pub growth_needed: i32,
    /// `+0x1ac`: stage-advance pulse timer (cosmetic, 9→0).
    pub pulse_timer: i32,
    /// Spawn interval (offset uncertain, `static`).
    pub spawn_interval: i32,
    /// Spawn counter (offset uncertain, `static`).
    pub spawn_counter: i32,
    /// `+0x1d8`: vy-damp timer (Rhubarb sets to 50).
    pub damp_timer: i32,
    /// `+0x1b4`: mode timer (0→40 cycle, resets in swim_modes).
    pub mode_timer: i32,
    /// `+0x1bc`: animation sub-counter (cosmetic).
    pub anim_c: i32,
    /// `+0x1c4`: animation cel (cosmetic).
    pub cel: i32,
    pub widget_x: i32,
    pub widget_y: i32,
    /// The shadow's floor y (cosmetic).
    pub shadow_y: i32,
    /// Set during eat_check when a stage advance happens; consumed by
    /// draw_reroll to draw the next growth_needed from app RNG.
    pub needs_growth_draw: bool,
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
    /// Stage advance happened; value is the new stage (1 or 2).
    pub stage_up: Option<i32>,
}

impl Breeder {
    /// `Board::BuyBreeder` (`0x005470a0`): the x draw, then the ctor draws,
    /// y overwritten to 65 (drop from top). `static`.
    pub fn bought(rng: &mut MtRand) -> Self {
        let x = rng.next_range(520) as i32 + 20;
        // Ctor draws (5): growth_needed, divisor (unused), hunger, mode,
        // interval — by analogy with the Grubber. `static`.
        let growth_needed = rng.next_range(5) as i32 + 4; // R1: [4..8]
        let _divisor = rng.next_range(3); // R2
        let hunger = rng.next_range(200) as i32 + 300;
        let mode = rng.next_range(10) as i32;
        let spawn_interval = rng.next_range(200) as i32 + 1200;
        Breeder {
            x: f64::from(x),
            y: 65.0,
            vx: 0.0,
            vy: 0.0,
            divisor: 2.0,
            hunger,
            fade: 0,
            fade_to_hungry: false,
            patrol_dir: 1,
            splash: 45,
            mode,
            reroll_timer: 0,
            anim: 0,
            stage: 0,
            growth: 0,
            growth_needed,
            pulse_timer: 0,
            spawn_interval,
            spawn_counter: 0,
            damp_timer: 0,
            mode_timer: 0,
            anim_c: 0,
            cel: 0,
            widget_x: x,
            widget_y: 65,
            shadow_y: 410,
            needs_growth_draw: false,
        }
    }

    /// Level start (`0x005471e0`): one breeder at a random position (2 draws),
    /// then ctor draws. `static`.
    pub fn level_start(rng: &mut MtRand) -> Self {
        let x = rng.next_range(520) as i32 + 20;
        let y = rng.next_range(265) as i32 + 105;
        // Ctor draws: same as `bought`.
        let growth_needed = rng.next_range(5) as i32 + 4; // R1: [4..8]
        let _divisor = rng.next_range(3);
        let hunger = rng.next_range(200) as i32 + 300;
        let mode = rng.next_range(10) as i32;
        let spawn_interval = rng.next_range(200) as i32 + 1200;
        let mut b = Breeder {
            x: f64::from(x),
            y: f64::from(y),
            vx: 0.0,
            vy: 0.0,
            divisor: 2.0,
            hunger,
            fade: 0,
            fade_to_hungry: false,
            patrol_dir: 1,
            splash: 0,
            mode,
            reroll_timer: 0,
            anim: 0,
            stage: 0,
            growth: 2,
            growth_needed,
            pulse_timer: 0,
            spawn_interval,
            spawn_counter: 0,
            damp_timer: 0,
            mode_timer: 0,
            anim_c: 0,
            cel: 0,
            widget_x: x,
            widget_y: y,
            shadow_y: 410,
            needs_growth_draw: false,
        };
        // Shadow draw.
        b.shadow_y = rng.next_range(5) as i32 + 410;
        b
    }

    /// `Breeder::Update` (`0x004f4180`, ~3780 bytes to `0x4f5043`).
    ///
    /// Binary order: hunger → food chase / swim modes → timer increments →
    /// reroll (app RNG) → guppy spawn → damp → drift → clamps → integrate.
    ///
    /// Food chase and swim modes are mutually exclusive (same as guppy):
    /// when hungry (hunger < 500) with food present (and no Gus), the
    /// breeder chases food (steer_to with mode_timer >= 3 threshold + eat
    /// check every tick). Otherwise, swim modes run (guppy-identical:
    /// modes 0–4 and patrol 5–9, 40-tick mode_timer threshold).
    ///
    /// `traced` (tank4-1_full_01).
    pub fn update(
        &mut self,
        has_aliens: bool,
        gus_first: bool,
        foods: &[Food],
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
        if self.hunger <= 0 {
            out.died = true;
            out.died_at = (self.x, self.y, self.vx);
        }

        // Splash timer (cosmetic in original at +0x128; we use it to gate
        // the ceiling clamp for bought breeders entering from above).
        if self.splash >= 1 {
            self.splash -= 1;
        }
        // Pulse timer (cosmetic, +0x1ac): 9→0 after a stage advance.
        if self.pulse_timer > 0 {
            self.pulse_timer -= 1;
        }

        // Food chase or swim modes (mutually exclusive, like guppy).
        // With Gus first in the alien list, breeders leave food to him
        // (`0x004f13e5`, same gate as guppies). `traced` (tank4-1_full_01).
        let chasing = if self.hunger < 500 && !gus_first {
            self.chase_food(foods, &mut out)
        } else {
            false
        };
        if !chasing {
            self.swim_modes();
        }

        // Timer increments (binary at `0x4f4c5f`).
        self.mode_timer += 1;
        // Note: reroll_timer increment and check are in Breeder::Draw
        // (`0x004dce10`), not here. See `draw_reroll()`.

        // Guppy spawn timer (`static`). During stage 0 the counter stays
        // at 0. At stage 0→1 the board sets it to `board_clock − 35` (the
        // number of non-alien ticks minus a 35-tick offset; see
        // `update_breeders`). After that it increments each non-alien tick.
        if !has_aliens && self.stage >= 1 {
            self.spawn_counter += 1;
            if self.spawn_counter >= self.spawn_interval {
                self.spawn_counter = 0;
                out.spawn_guppy = Some((self.widget_x + 5, self.widget_y + 10));
            }
        }

        // Damp timer (Rhubarb fling: vy decays, `0x4f4d0e`).
        if self.damp_timer > 0 {
            self.vy *= 0.9;
            self.damp_timer -= 1;
        }

        // Drift by vx bucket (same as guppy, `0x4f4d30`).
        self.drift_by_vx_bucket();

        // Y clamp: floor (`0x4f4dd8`, field `+0x190`).
        // 370.0 — same as the fish floor (bounds[3]).
        // `traced` (tank4-1_full_01, tick 2478: y settles at 371.75 = 370 + 3.5/2).
        if self.y > 370.0 {
            self.y = 370.0;
        }
        // Y clamp: ceiling (gated by splash and damp like the guppy).
        if self.damp_timer > 0 && self.vy > 0.0 {
            // Being flung upward — skip ceiling clamp.
        } else if self.splash == 0 && self.y < 95.0 {
            self.y = 95.0;
        }

        // X clamp (`0x4f4df8`, fields `+0x194`/`+0x19c`).
        if self.x > 540.0 {
            self.x = 540.0;
        }
        if self.x < 10.0 {
            self.x = 10.0;
        }

        // Integrate (`0x4f4fda`).
        let x_pre = self.x;
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;

        // Wall deceleration (Draw-side, `Breeder::Draw` @ `0x004dce10`):
        // after Update integrates, Draw checks the integer projected
        // position `widget_x + (vx as i32)` against the x boundaries and
        // eases vx by 0.1, re-integrating from x_pre. The `vx > 0.1`
        // guard matches the pattern in Fish::edge_steer, Starcatcher, and
        // Guppycruncher — it prevents perpetual decel when vx is tiny
        // (the x-clamp at 540 would otherwise keep widget_x at 540 and
        // the check would fire every tick).
        // `traced` (tank4-1_full_01, ticks 1960–1985 left, 2760+ right).
        let ivx = self.vx as i32;
        if self.widget_x + ivx < 10 && self.vx < -0.1 {
            self.vx += 0.1;
            self.x = x_pre + self.vx / self.divisor;
            self.widget_x = self.x as i32;
        } else if self.widget_x + ivx >= 540 && self.vx > 0.1 {
            self.vx -= 0.1;
            self.x = x_pre + self.vx / self.divisor;
            self.widget_x = self.x as i32;
        }
        out
    }

    /// `Breeder::Draw` (`0x004dce10`): the breeder's swim-mode reroll is
    /// in the draw function, not in Update. It fires every 21 ticks (same
    /// cycle as the other entities' rerolls in their Update functions).
    /// Draws 1 app RNG value (r%10 reroll check), and if the check passes
    /// or mode is -1, 1 more (r%9+1 for the new mode 1–9).
    /// `traced` (tank4-1_full_01).
    pub fn draw_reroll(&mut self, rng: &mut MtRand) {
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 || self.mode == -1 {
                self.mode = rng.next_range(9) as i32 + 1;
            }
        }
        // Growth-needed re-draw: in the original, this happens inside
        // Breeder::Draw after the reroll, following the slurp sound draw
        // in Update. The draws between eat (Update) and here shift the
        // RNG to the correct position.
        // `traced` (tank4-1_full_01, tick 2298: growth_needed=8).
        if self.needs_growth_draw {
            self.growth_needed = rng.next_range(5) as i32 + 4;
            self.needs_growth_draw = false;
        }
    }

    /// Food chase (same structure as `Fish::chase_food`): steer toward the
    /// nearest edible food every 3 ticks, eat check every tick. Returns
    /// true when a food target exists. `traced` (tank4-1_full_01).
    fn chase_food(
        &mut self,
        foods: &[Food],
        out: &mut BreederOut,
    ) -> bool {
        let target = self.find_food(foods);
        if target.is_none() {
            return false;
        }
        let fx = (self.x + 40.0) as i32;
        let fy = (self.y + 40.0) as i32;
        if self.mode_timer >= 3 {
            self.mode_timer = 0;
            let f = &foods[target.unwrap()];
            // prev_widget: steer_to needs the Draw-synced widget from
            // the previous tick, not the inline-synced current one.
            // `traced` (tank4-1_full_01, tick 4574).
            let (tx, ty) = (f.prev_widget_x + 20, f.prev_widget_y + 20);
            self.steer_to(fx, fy, tx, ty);
        }
        self.eat_check(foods, out);
        true
    }

    /// Find the nearest edible food pellet. Uses prev_widget for
    /// distance (same timing as steer_to). `traced` (tank4-1_full_01).
    fn find_food(&self, foods: &[Food]) -> Option<usize> {
        let mut best = 100_000_000;
        let mut found = None;
        for (i, f) in foods.iter().enumerate() {
            if f.delay > 0 || f.kind == 2 {
                continue;
            }
            let dx = (f64::from(f.prev_widget_x + 20) - (self.x + 40.0)) as i32;
            let dy = (f64::from(f.prev_widget_y + 20) - (self.y + 40.0)) as i32;
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                best = d;
                found = Some(i);
            }
        }
        found
    }

    /// Steer toward (tx, ty). Same hunger-dependent constants as
    /// `Fish::steer_to` but the Y far/near threshold is **1** (not 6).
    /// `traced` (tank4-1_full_01).
    fn steer_to(&mut self, fx: i32, fy: i32, tx: i32, ty: i32) {
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
        if let Some(d) = dec {
            if -cap < self.vx {
                self.vx -= d;
            }
        }
        if let Some(i) = inc {
            if cap > self.vx {
                self.vx += i;
            }
        }
        // Y thresholds: 1 for going down, 6 for going up. The guppy
        // uses 6 for both; the breeder has an asymmetric threshold.
        // `traced` (tank4-1_full_01: tick 1526 gap=+2 far-down;
        // tick 4574 gap=+1 near-down; tick 4577 gap=-6 near-up).
        if fy > ty + 1 {
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

    /// Eat check: eat the first edible food whose bite box contains the
    /// breeder's integer centre (`widget_x + 40`, `widget_y + 40`).
    /// The box is `(wx, wy+5)` to `(wx+40, wy+40)` — the food widget
    /// rect with a 5-pixel top margin.  Using integer coordinates is
    /// load-bearing: at tick 2292 of the tank-4-1 trace, the float
    /// centre y (318.2) passes the food's adjusted top edge (318+5=323)
    /// but the integer centre (318) does not.
    /// `traced` (tank4-1_full_01).
    fn eat_check(&mut self, foods: &[Food], out: &mut BreederOut) {
        // Float centre (same as the guppy's eat_check): sub-pixel
        // precision matters when food has just fallen past the
        // boundary (tick 2298: wy+5 == cy as ints, but cy_float =
        // 333.5 so the strict `<` still passes).
        let cx = self.x + 40.0;
        let cy = self.y + 40.0;
        let bite_idx = foods.iter().position(|f| {
            if f.delay > 0 || f.kind == 2 {
                return false;
            }
            let (wx, wy) = (f64::from(f.widget_x), f64::from(f.widget_y));
            wx < cx && cx < wx + 40.0 && wy + 5.0 < cy && cy < wy + 42.0
        });
        if let Some(i) = bite_idx {
            let was_hungry = self.hunger < 301 && self.fade == 0;
            // Food-level table (same as guppies): level 0 +500 cap 800
            // growth 1; level 1 +700 cap 1000 growth 2; level 2 +1100
            // cap 1400 growth 3. `traced` (tank4-1_full_01).
            let (gain, cap, growth) = match foods[i].level {
                0 => (500, 800, 1),
                1 => (700, 1000, 2),
                _ => (1100, 1400, 3),
            };
            self.hunger = (self.hunger + gain).min(cap);
            self.growth += growth;
            // Stage advance: when growth exceeds the threshold (strict >)
            // and stage is below max (2), the breeder grows to the next
            // stage. The growth_needed re-draw is done by the board
            // AFTER the slurp sound draw (the slurp consumes 1 app RNG
            // before the growth_needed draw).
            // `traced` (tank4-1_full_01, tick 936→2298: growth=6 > 5).
            if self.growth > self.growth_needed && self.stage < 2 {
                self.stage += 1;
                self.growth = 0;
                self.pulse_timer = 9;
                self.needs_growth_draw = true;
                // At stage 2, spawn interval changes deterministically
                // (no RNG draw). `static` — only one data point: 1320→563.
                if self.stage == 2 {
                    self.spawn_interval = 563;
                }
                out.stage_up = Some(self.stage);
            }
            if was_hungry && !(self.hunger < 301 && self.fade == 0) {
                self.fade_to_hungry = false;
                self.fade = 5;
            }
            out.ate_food = Some(i);
        }
    }

    /// Swim modes 0–9: identical to guppy (`Fish::swim_modes`).
    /// `0x4f47bb`–`0x4f4c59` in `Breeder::Update`.
    fn swim_modes(&mut self) {
        match self.mode {
            m if m > 4 => self.patrol(),
            0 => {
                if self.damp_timer == 0 {
                    self.vy = 0.5;
                }
                if self.mode_timer >= 40 {
                    self.mode_timer = 0;
                    if self.vx > 0.5 {
                        self.vx -= 0.5;
                    } else if self.vx < -0.5 {
                        self.vx += 0.5;
                    }
                    self.anim = self.vx.abs() as i32;
                }
                self.y -= 0.25 / self.divisor;
            }
            1 | 2 => {
                if self.damp_timer == 0 {
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
                    self.anim = self.vx.abs() as i32;
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
                    if self.anim > 4 {
                        self.anim -= 1;
                    } else if self.vy < 4.0 {
                        self.anim += 1;
                    }
                }
                if self.y > 240.0 {
                    self.mode = 0;
                }
            }
            _ => {}
        }
    }

    /// Patrol (modes 5–9), identical to guppy.
    fn patrol(&mut self) {
        if self.damp_timer == 0 {
            self.vy = if self.y < 115.0 { -0.1 } else { -0.5 };
        }
        if self.mode_timer < 40 {
            return;
        }
        self.mode_timer = 0;
        match self.patrol_dir {
            1 => {
                self.vx += if self.vx < 0.0 { 2.0 } else { 1.0 };
                self.anim = self.vx.abs() as i32;
                if self.x > 250.0 {
                    self.patrol_dir = -1;
                    self.vx -= 2.0;
                }
            }
            -1 => {
                self.vx -= if self.vx > 0.0 { 2.0 } else { 1.0 };
                self.anim = self.vx.abs() as i32;
                if self.x < 175.0 {
                    self.patrol_dir = 1;
                    self.vx += 2.0;
                }
            }
            _ => {}
        }
    }

    /// Exact-equality buckets on `vx` (reproduced deliberately, same as guppy).
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

}
