//! Guppy (`Sexy::Fish`, type 0). Spec: `docs/re/objects/fish-guppy.md`.
//!
//! Field names follow the spec; raw offsets of the original are noted where
//! they aid cross-referencing captures.

use sexy_core::rand::MtRand;

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
    /// `+0x1a8`
    pub unk_1a8: i32,
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
            unk_1a8: 0,
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
        f.divisor = [2.0, 1.6, 1.8][rng.next_range(3) as usize];
        f.hunger = rng.next_range(200) as i32 + 400;
        f.unk_1a8 = rng.next_range(3) as i32 + 4;
        f.mode = rng.next_range(10) as i32;
        f.coin_interval = rng.next_range(200) as i32 + 150;
        f
    }

    /// `Fish::Update` (`0x004f6970`) for a guppy that isn't chasing food
    /// (hunger ≥ 500) on a board without aliens.
    ///
    /// TODO(spec): food chase (`UpdateHunger` → slot 84/87), the cursor-follow
    /// branch, coin drop, and alien-present behaviour are not yet modelled;
    /// see docs/re/objects/fish-feeding.md once written.
    pub fn update(&mut self, env: FishEnv, rng: &mut MtRand, global: &mut MtRand) {
        if env.paused {
            return;
        }
        self.pre_update();
        // UpdateHunger (0x004f15c0): hunger countdown when no aliens.
        if !env.has_aliens {
            self.hunger -= 1;
            if self.hunger < -1000 {
                self.hunger = -1000;
            }
        }
        // Hunger < 500 → food chase (not yet modelled); otherwise swim modes.
        self.swim_modes(rng, global);
        self.mode_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32 + 1;
            }
        }
        // Coin drop (slot 81) runs only when there are no aliens.
        // TODO(spec): coin_timer advance + drop needs coin.md; stage 0 guppies
        // only advance the timer when stage > 0 or state ≥ 0.
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
