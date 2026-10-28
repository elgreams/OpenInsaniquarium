//! The Starcatcher (`Sexy::Penta`, type 8, board list `+0xd0`). Spec:
//! `docs/re/objects/starcatcher.md`.

use sexy_core::rand::MtRand;

use crate::coin::Coin;

#[derive(Debug, Clone, PartialEq)]
pub struct Starcatcher {
    /// `+0x158`, `+0x160`, `+0x168`, `+0x170`
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// `+0x178`: speed divisor (2.5..2.7).
    pub divisor: f64,
    /// `+0x9c`
    pub hunger: i32,
    /// `+0xa0`: colour fade; `true` while turning hungry.
    pub fade: i32,
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
    /// `+0x1a4`: eat counter.
    pub eats: i32,
    pub widget_x: i32,
    pub widget_y: i32,
    /// The shadow's floor y (cosmetic).
    pub shadow_y: i32,
}

/// What a Starcatcher's update asks the board to do.
#[derive(Debug, Default)]
pub struct StarOut {
    /// Ate coin `i` (removed, not banked): drop a $200 kind-5 coin here.
    pub ate: Option<(usize, i32, i32)>,
    /// Starved: die (sound, corpse, messages).
    pub died: bool,
}

/// Star filter (`0x004d6bf0`, normal game): an uncollected star.
fn is_star(c: &Coin) -> bool {
    !c.collected && matches!(c.kind, 3 | 10 | 0x12)
}

impl Starcatcher {
    /// `Board::BuyStarcatcher` (`0x00545260`) up to the shadow: the x draw,
    /// then the ctor's 4 (y, divisor, hunger, mode); y is overwritten.
    pub fn bought(rng: &mut MtRand) -> Self {
        let x = rng.next_range(520) as i32 + 20;
        let _ctor_y = rng.next_range(5);
        let divisor = [2.7, 2.5, 2.6][rng.next_range(3) as usize];
        let hunger = rng.next_range(200) as i32 + 900;
        let mode = rng.next_range(10) as i32;
        Starcatcher {
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
            eats: 0,
            widget_x: x,
            widget_y: 65,
            shadow_y: 410,
        }
    }

    /// `Penta::Update` (`0x004f94e0`), normal game.
    pub fn update(&mut self, has_aliens: bool, coins: &[Coin], rng: &mut MtRand) -> StarOut {
        let mut out = StarOut::default();
        // Behaviour (`0x004f9460`): the fade, hunger, starving, the chase.
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
            false
        } else if self.hunger < 900 {
            self.chase(coins, &mut out)
        } else {
            false
        };
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
        self.chase_timer += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 || self.x <= 10.0 || self.x >= 540.0 {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(9) as i32;
            }
        }
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
        if self.splash >= 1 {
            let n = if self.splash > 35 { 8 } else { 5 };
            if rng.next_range(n) == 0 {
                // A bubble (its own draws are on the global stream).
                rng.next_u31();
                rng.next_u31();
            }
            self.splash -= 1;
        } else if self.y < 95.0 {
            self.y = 95.0;
        }
        if self.x > 535.0 && self.vx > 0.1 {
            self.vx -= 0.1;
        }
        if self.x < 15.0 && self.vx < -0.1 {
            self.vx += 0.1;
        }
        if self.y < 359.0 {
            self.vy += 0.4;
        }
        // Animation (`0x004d8f10`).
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
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        out
    }

    /// Star chase (`0x004eff50`): steer x every 5th tick at the nearest
    /// star, then bite the first in reach.
    fn chase(&mut self, coins: &[Coin], out: &mut StarOut) -> bool {
        let (px, py) = (self.x + 40.0, self.y + 40.0);
        let mut best = 100_000_000;
        let mut star = None;
        for (i, c) in coins.iter().enumerate() {
            if !is_star(c) {
                continue;
            }
            let dx = (f64::from(c.widget_x + 36) - px) as i32;
            let dy = (f64::from(c.widget_y + 36) - py) as i32;
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                best = d;
                star = Some(i);
            }
        }
        if self.chase_timer >= 5 {
            let Some(si) = star else { return false };
            self.chase_timer = 0;
            let tx = f64::from(coins[si].widget_x);
            let hungry = self.hunger <= 300;
            let (far, near) = if hungry { (6.0, 2.5) } else { (4.5, 1.5) };
            let (big, small) = if hungry { (2.5, 1.2) } else { (1.8, 1.0) };
            if px > tx + 46.0 {
                if self.vx > -far {
                    self.vx -= big;
                }
            } else if px < tx + 26.0 {
                if self.vx < far {
                    self.vx += big;
                }
            } else if px > tx + 36.0 {
                if self.vx > -near {
                    self.vx -= small;
                }
            } else if px < tx + 36.0 && self.vx < near {
                self.vx += small;
            }
        }
        if star.is_none() {
            return false;
        }
        // Bite (`0x004ec2f0`): the first star within reach.
        if let Some(i) = coins.iter().position(|c| {
            let (wx, wy) = (f64::from(c.widget_x), f64::from(c.widget_y));
            is_star(c) && wx + 16.0 < px && px < wx + 56.0 && wy + 16.0 < py && py < wy + 66.0
        }) {
            // Eat (slot 77): hunger +900 capped at 1300, then the drop;
            // fade back to the fed colours if it looked hungry
            // (`0x004d6f30` / `0x004e13d0`).
            let was_hungry = self.hunger < 301 && self.fade == 0;
            self.hunger = (self.hunger + 900).min(1300);
            self.eats = 0;
            if was_hungry && !(self.hunger < 301 && self.fade == 0) {
                self.fade_to_hungry = false;
                self.fade = 5;
            }
            out.ate = Some((i, self.widget_x + 5, self.widget_y - 12));
        }
        true
    }
}
