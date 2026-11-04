//! The Guppycruncher (`Sexy::Grubber`, type 9, board list `+0xc4`). Spec:
//! `docs/re/objects/guppycruncher.md`.
//!
//! A direct `GameObject` subclass (like `Starcatcher`), **not** a `Fish`.
//! Eats small guppies and drops Larvae (Beetles) on a timer.

use sexy_core::rand::MtRand;

use crate::fish::Fish;

#[derive(Debug, Clone, PartialEq)]
pub struct Guppycruncher {
    /// `+0x158`, `+0x160`, `+0x168`, `+0x170`
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// `+0x178`: speed divisor (2.7 / 2.5 / 2.6).
    pub divisor: f64,
    /// `+0x9c`: hunger (900..1099 at spawn).
    pub hunger: i32,
    /// `+0xa0`: colour fade counter.
    pub fade: i32,
    /// Whether fading toward hungry (true) or recovering (false).
    pub fade_to_hungry: bool,
    /// `+0x180`: splash timer (bubbles for the first 45 ticks).
    pub splash: i32,
    /// `+0x188`: the swim mode's target vx.
    pub target_vx: f64,
    /// `+0x190`: swim mode 0..9.
    pub mode: i32,
    /// `+0x194`: chase-steer timer; `+0x198`: reroll timer.
    pub chase_timer: i32,
    pub reroll_timer: i32,
    /// `+0x19c`, `+0x1a0`: animation counter and cel.
    pub anim: i32,
    pub cel: i32,
    /// `+0x1a8`: Larva drop counter (ticks since last drop).
    pub drop_counter: i32,
    /// `+0x1ac`: Larva drop interval (300..499 in Adventure).
    pub drop_interval: i32,
    pub widget_x: i32,
    pub widget_y: i32,
    /// The shadow's floor y (cosmetic).
    pub shadow_y: i32,
}

/// What a Guppycruncher's update asks the board to do.
#[derive(Debug, Default)]
pub struct GrubOut {
    /// Ate guppy `i` (removed): gore draws.
    pub ate: Option<usize>,
    /// Drop a Larva at these board coordinates.
    pub drop_larva: Option<(i32, i32)>,
    /// Starved: die (sound, corpse).
    pub died: bool,
    /// Position and vx when it starved: the corpse uses these.
    pub died_at: (f64, f64, f64),
}

impl Guppycruncher {
    /// `Board::BuyGuppycruncher` (`0x00545430`): the x draw, then the
    /// ctor's 5 draws (y, divisor, hunger, mode, interval); y is
    /// overwritten to 65. `static`.
    pub fn bought(rng: &mut MtRand) -> Self {
        let x = rng.next_range(520) as i32 + 20;
        // Init (0x004d7970): 5 draws.
        let _ctor_y = rng.next_range(5); // R1: y = r%5+360, discarded.
        // R2: divisor.
        let divisor = [2.7, 2.5, 2.6][rng.next_range(3) as usize];
        // R3: hunger = r%200+900.
        let hunger = rng.next_range(200) as i32 + 900;
        // R4: mode = r%10.
        let mode = rng.next_range(10) as i32;
        // R5: Larva drop interval = r%200+300 (normal play).
        let drop_interval = rng.next_range(200) as i32 + 300;
        Guppycruncher {
            x: f64::from(x),
            y: 65.0,
            vx: 0.0,
            vy: 0.0,
            divisor,
            hunger,
            fade: 0,
            fade_to_hungry: false,
            splash: 45,
            target_vx: 0.0,
            mode,
            chase_timer: 40,
            reroll_timer: 0,
            anim: 0,
            cel: 0,
            drop_counter: 0,
            drop_interval,
            widget_x: x,
            widget_y: 65,
            shadow_y: 410,
        }
    }

    /// `Grubber::Update` (`0x004f8fe0`), normal game.
    /// `guppycruncher.md#per-tick-update`. `static`.
    pub fn update(&mut self, has_aliens: bool, guppies: &[Fish], rng: &mut MtRand) -> GrubOut {
        let mut out = GrubOut::default();

        // 2. Behaviour: fade, hunger, starving, chase.
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
        if !has_aliens {
            self.hunger -= 1;
            if self.hunger == 304 {
                self.fade_to_hungry = true;
                self.fade = 1;
            } else if self.hunger < -1000 {
                self.hunger = -1000;
            }
        }
        let busy = if self.hunger <= 0 {
            out.died = true;
            out.died_at = (self.x, self.y, self.vx);
            false
        } else if self.hunger < 900 {
            self.chase(guppies, &mut out)
        } else {
            false
        };

        // 3. Swim modes (only when not chasing).
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

        // 4. Timers and reroll.
        self.chase_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 || self.x <= 10.0 || self.x >= 540.0 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32;
            }
        }

        // 5. Clamps.
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

        // 6. Splash timer (bubbles while splashing in).
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

        // 7. Edge steering.
        if self.x > 535.0 && self.vx > 0.1 {
            self.vx -= 0.1;
        }
        if self.x < 15.0 && self.vx < -0.1 {
            self.vx += 0.1;
        }

        // 8. Gravity (same as Starcatcher).
        if self.y < 359.0 {
            self.vy += 0.4;
        }

        // 9. Larva drop timer (0x004d7ae0).
        self.drop_counter += 1;
        if self.drop_counter >= self.drop_interval {
            self.drop_counter = 0;
            // Gate: in challenge mode, only drop if hunger > 0.
            // In Adventure, always pass. (We don't model challenge yet.)
            out.drop_larva = Some((self.widget_x + 4, self.widget_y - 10));
        }

        // 10. Animation (shared with Starcatcher, `0x004d8f10`).
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

        // 11. Integrate.
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        out
    }

    /// Guppy chase: steer toward the nearest small guppy, bite the first
    /// in reach. `guppycruncher.md#guppy-chase`. `static` (steering
    /// constants are guesses modelled on the Carnivore).
    fn chase(&mut self, guppies: &[Fish], out: &mut GrubOut) -> bool {
        let (px, py) = (self.x + 40.0, self.y + 40.0);
        let mut best = 100_000_000;
        let mut prey = None;
        for (i, g) in guppies.iter().enumerate() {
            // Small guppies only (stage 0), alive (`+0xac < 0`).
            if g.stage != 0 || g.state_ac >= 0 || g.dead {
                continue;
            }
            let dx = (f64::from(g.widget_x + 40) - px) as i32;
            let dy = (f64::from(g.widget_y + 40) - py) as i32;
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                best = d;
                prey = Some(i);
            }
        }

        if self.chase_timer >= 5 {
            let Some(pi) = prey else { return false };
            self.chase_timer = 0;
            let g = &guppies[pi];
            let fx = px;
            let fy = py;
            let tx = f64::from(g.widget_x + 40);
            let ty = f64::from(g.widget_y + 40);
            // Steering: guess, using Carnivore-like constants.
            let hungry = self.hunger <= 300;
            let x_cap = if hungry { 5.0 } else { 4.0 };
            if fx > tx + 8.0 {
                if self.vx > -x_cap {
                    self.vx -= 1.5;
                }
            } else if fx < tx - 8.0 {
                if self.vx < x_cap {
                    self.vx += 1.5;
                }
            } else if fx > tx + 4.0 {
                if self.vx > -x_cap {
                    self.vx -= 0.2;
                }
            } else if fx < tx - 4.0 {
                if self.vx < x_cap {
                    self.vx += 0.2;
                }
            } else if fx > tx {
                if self.vx > -x_cap {
                    self.vx -= if hungry { 0.05 } else { 0.1 };
                }
            } else if fx < tx && self.vx < x_cap {
                self.vx += if hungry { 0.05 } else { 0.1 };
            }
            // Y steering.
            let y_cap_down = -3.0;
            let y_cap_up = 4.0;
            if fy > ty + 6.0 {
                if self.vy > y_cap_down {
                    self.vy -= if hungry { 1.3 } else { 0.8 };
                }
            } else if fy < ty - 6.0 {
                if self.vy < y_cap_up {
                    self.vy += if hungry { 1.5 } else { 1.3 };
                }
            } else if fy > ty {
                if self.vy > y_cap_down {
                    self.vy -= if hungry { 0.5 } else { 0.3 };
                }
            } else if fy < ty && self.vy < y_cap_up {
                self.vy += if hungry { 0.7 } else { 0.5 };
            }
        }

        if prey.is_none() {
            return false;
        }

        // Bite: the first small guppy in the bite box.
        let bite_idx = guppies.iter().position(|g| {
            if g.stage != 0 || g.state_ac >= 0 || g.dead {
                return false;
            }
            let (gx, gy) = (f64::from(g.widget_x), f64::from(g.widget_y));
            gx + 10.0 < px && px < gx + 70.0 && gy + 22.0 < py && py < gy + 58.0
        });
        if let Some(i) = bite_idx {
            let was_hungry = self.hunger < 301 && self.fade == 0;
            // Eat: hunger += 1000, cap 1400 (confirmed from bytecode).
            self.hunger = (self.hunger + 1000).min(1400);
            if was_hungry && !(self.hunger < 301 && self.fade == 0) {
                self.fade_to_hungry = false;
                self.fade = 5;
            }
            out.ate = Some(i);
        }
        true
    }
}
