//! Pets. Specs: `docs/re/objects/pet-stinky.md` (pet 0, Stinky the snail)
//! and `pet-niko.md` (pet 1, Niko the oyster). Other pets are TODO(spec).

use sexy_core::rand::MtRand;

use crate::coin::Coin;

/// Pet ids (profile order). `OtherTypePet` covers 0, 1, 5, 7 and 14.
pub const STINKY: i32 = 0;
pub const NIKO: i32 = 1;

/// Niko's pearl timer milestones (`0x004e2a20`, pet-niko.md#update).
const NIKO_OPEN: i32 = 1224;
const NIKO_PEARL: i32 = 1233;
const NIKO_CLOSE: i32 = 1440;
const NIKO_RESET: i32 = 1450;

#[derive(Debug, Clone, PartialEq)]
pub struct Pet {
    /// `+0x1c4`
    pub id: i32,
    /// `+0x158`, `+0x160`
    pub x: f64,
    pub y: f64,
    /// `+0x168`, `+0x170`
    pub vx: f64,
    pub vy: f64,
    /// `+0x178`: last non-zero vx.
    pub facing: f64,
    /// `+0x180`
    pub target_vx: f64,
    /// `+0x188`
    pub divisor: f64,
    /// `+0x190`: 0 stop, 1 left, 2 right, 3..9 keep.
    pub mode: i32,
    /// `+0x194`: steering cadence counter.
    pub counter: i32,
    /// `+0x198`: mode-reroll timer.
    pub reroll_timer: i32,
    /// `+0x19c`, `+0x1a0`, `+0x1a4`
    pub turn: i32,
    pub anim: i32,
    pub cel: i32,
    /// `+0x1a8`, `+0x1ac`
    pub counter_1a8: i32,
    pub countdown_1ac: i32,
    /// `+0x1b0`
    pub slow: bool,
    /// `+0x1b4`: Stinky's shell counter (hiding while > 0); Niko's pearl
    /// timer.
    pub shell: i32,
    /// `+0x1c0`: Niko's pearl was taken (set when it's collected, cleared
    /// when a new one appears).
    pub pearl_taken: bool,
    /// `+0x1b8`, `+0x1bc`
    pub unk_1b8: i32,
    pub unk_1bc: i32,
    pub widget_x: i32,
    pub widget_y: i32,
    /// Its shadow's floor y (`410 + r % 5`, drawn in the Shadow ctor; a
    /// type-1 shadow never moves vertically). Presentation only.
    pub shadow_y: i32,
}

/// What a pet's update did that the board must apply.
#[derive(Debug, Default)]
pub struct PetOut {
    /// Index into the coin list of a coin picked up this tick.
    pub picked: Option<usize>,
    /// Niko made a pearl at this widget position (`AddCoin(wx + 1, wy − 2,
    /// kind 15)`).
    pub pearl: Option<(i32, i32)>,
    /// Niko's open / close sample (`0x129` / `0x12a`).
    pub sound: Option<crate::sound::Sound>,
}

impl Pet {
    /// `OtherTypePet::OtherTypePet` (`0x004eb5c0`) for Stinky: 2 app draws.
    pub fn stinky(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let _ = y; // Overwritten: Stinky lives on the floor.
        let mut p = Pet {
            id: STINKY,
            x: f64::from(x),
            y: 370.0,
            vx: 0.0,
            vy: 0.0,
            facing: 1.0,
            target_vx: 0.0,
            divisor: 1.2,
            mode: 0,
            counter: 40,
            reroll_timer: 0,
            turn: 0,
            anim: 0,
            cel: 0,
            counter_1a8: 0,
            countdown_1ac: 0,
            slow: false,
            shell: 0,
            pearl_taken: false,
            unk_1b8: 0,
            unk_1bc: 0,
            widget_x: x,
            widget_y: 370,
            shadow_y: 410,
        };
        p.mode = rng.next_range(10) as i32;
        p.unk_1b8 = rng.next_range(250) as i32 + 250;
        p
    }

    /// `OtherTypePet::OtherTypePet` for Niko (pet-niko.md#construction):
    /// like Stinky, but y isn't forced and the divisor is the default 2.0.
    pub fn niko(x: i32, y: i32, rng: &mut MtRand) -> Self {
        let mut p = Pet::stinky(x, y, rng);
        p.id = NIKO;
        p.y = f64::from(y);
        p.widget_y = y;
        p.divisor = 2.0;
        p
    }

    /// `OtherTypePet::Update` (`0x004f2e80`), pet 0, normal game.
    /// A collected coin is reported in `out`; the board banks it and plays
    /// its sound (which may draw RNG).
    pub fn update(&mut self, has_aliens: bool, coins: &[Coin], rng: &mut MtRand, out: &mut PetOut) {
        if self.id == NIKO {
            self.update_niko(rng, out);
            return;
        }
        // Coin chase (`0x004efe80`): for pet 0, runs whenever the coin list is
        // non-empty, there are no aliens and he isn't slowed, and then
        // reports "chasing" regardless of whether any coin was eligible
        // (collected coins still flying to the counter count). So he keeps
        // his speed instead of wandering.
        let chasing = !coins.is_empty() && !has_aliens && !self.slow;
        if chasing {
            self.chase(coins, out);
        }
        if !chasing {
            // In his shell (`+0x1b4 > 0`) the target is 0, whatever the mode.
            match self.mode {
                _ if self.shell != 0 => self.target_vx = 0.0,
                0 => self.target_vx = 0.0,
                1 => self.target_vx = -0.5,
                2 => self.target_vx = 0.5,
                _ => {}
            }
            if self.vx > self.target_vx {
                self.vx -= 0.1;
                if self.vx < self.target_vx {
                    self.vx = self.target_vx;
                }
            } else if self.vx < self.target_vx {
                self.vx += 0.1;
                if self.target_vx < self.vx {
                    self.vx = self.target_vx;
                }
            }
        }
        // In his shell he stops dead.
        if self.shell != 0 {
            self.vx = 0.0;
        }
        self.counter += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20
            || (self.x <= 10.0 && self.target_vx <= 0.0)
            || (self.x >= 540.0 && self.target_vx > 0.0)
        {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(3) as i32;
            }
        }
        if self.countdown_1ac != 0 {
            self.countdown_1ac -= 1;
        }
        if self.counter_1a8 < 20 {
            self.counter_1a8 += 1;
        }
        // Shell (`0x004e2a20`).
        if has_aliens {
            if self.shell < 9 {
                self.shell += 1;
            }
            self.unk_1bc = 0;
        } else if self.shell > 0 {
            self.shell -= 1;
        }
        // Explicit compares, in the original's order (NaN-safe like it).
        #[allow(clippy::manual_clamp)]
        if self.x > 550.0 {
            self.x = 550.0;
        }
        if self.x < 10.0 {
            self.x = 10.0;
        }
        if self.y > 370.0 {
            self.y = 370.0;
            self.vy = 0.0;
        }
        if self.y < 95.0 {
            self.y = 95.0;
        }
        if self.x > 535.0 && self.vx > 0.1 {
            self.mode = 1;
        }
        if self.x < 15.0 && self.vx < -0.1 {
            self.mode = 2;
        }
        self.animate();
        let div = if self.slow {
            self.divisor * 5.0
        } else {
            self.divisor
        };
        self.x += self.vx / div;
        self.y += self.vy / div;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
    }

    /// `OtherTypePet::Update` for pet 1 (pet-niko.md#update): no steering,
    /// the 21-tick mode re-roll (RNG), the pearl timer, clamps and the
    /// simple animation counter.
    fn update_niko(&mut self, rng: &mut MtRand, out: &mut PetOut) {
        self.counter += 1;
        self.reroll_timer += 1;
        if self.reroll_timer > 20 || (self.x <= 10.0 && self.target_vx <= 0.0) {
            self.reroll_timer = 0;
            if rng.next_range(10) == 0 {
                self.mode = rng.next_range(3) as i32;
            }
        }
        if self.countdown_1ac != 0 {
            self.countdown_1ac -= 1;
        }
        if self.counter_1a8 < 20 {
            self.counter_1a8 += 1;
        }
        // Pearl timer (`0x004e2a20`).
        self.shell += 1;
        match self.shell {
            NIKO_OPEN => out.sound = Some(crate::sound::Sound::NikoOpen),
            NIKO_PEARL => {
                out.pearl = Some((self.widget_x + 1, self.widget_y - 2));
                self.pearl_taken = false;
                // TODO(spec): two global-stream bubbles (not modelled).
            }
            NIKO_CLOSE => out.sound = Some(crate::sound::Sound::NikoClose),
            NIKO_RESET => self.shell = rng.next_range(50) as i32,
            _ => {}
        }
        #[allow(clippy::manual_clamp)]
        if self.x > 540.0 {
            self.x = 540.0;
        }
        if self.x < 10.0 {
            self.x = 10.0;
        }
        if self.y > 350.0 {
            self.y = 350.0;
            self.vy = 0.0;
        }
        if self.y < 95.0 {
            self.y = 95.0;
        }
        // `0x004eb7c0`, ids other than 0/5/7/14: a bare counter.
        self.anim = (self.anim + 1) % 19;
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
    }

    fn eligible(c: &Coin, have_starcatcher: bool) -> bool {
        let star = matches!(c.kind, 3 | 10);
        !c.collected && !matches!(c.kind, 0x11 | 0x12) && (!star || !have_starcatcher)
    }

    /// `0x004ebc80` (target `0x004e7270`, collect `0x004e7c30`).
    fn chase(&mut self, coins: &[Coin], out: &mut PetOut) -> bool {
        // TODO(spec): starcatchers (`+0xd0`) aren't modelled; none on tank 1.
        let have_starcatcher = false;
        let mut best = 100_000_000;
        let mut target = None;
        for (i, c) in coins.iter().enumerate() {
            if !Self::eligible(c, have_starcatcher) {
                continue;
            }
            let dx = (f64::from(c.widget_x + 36) - (self.x + 40.0)) as i32;
            let dy = (f64::from(c.widget_y + 36) - (self.y + 40.0)) as i32;
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                best = d;
                target = Some(i);
            }
        }
        let Some(t) = target else { return false };
        if self.counter > 4 {
            self.counter = 0;
            let cx = self.x + 40.0;
            let w = coins[t].widget_x;
            let at = |o: i32| f64::from(w + o);
            if cx > at(48) {
                if self.vx > -2.3 {
                    self.vx -= 1.0;
                }
            } else if cx < at(24) {
                if self.vx < 2.3 {
                    self.vx += 1.0;
                }
            } else if cx > at(40) {
                if self.vx > -1.3 {
                    self.vx -= 0.5;
                }
            } else if cx < at(32) {
                if self.vx < 1.3 {
                    self.vx += 0.5;
                }
            } else if cx > at(36) {
                if self.vx > -0.3 {
                    self.vx = 0.0;
                }
            } else if cx < at(36) && self.vx < 0.3 {
                self.vx = 0.0;
            }
        }
        // Collect: first eligible coin within the pickup box.
        let (cx, cy) = (self.x + 40.0, self.y + 40.0);
        for (i, c) in coins.iter().enumerate() {
            if !Self::eligible(c, have_starcatcher) {
                continue;
            }
            let (wx, wy) = (f64::from(c.widget_x), f64::from(c.widget_y));
            if cx < wx + 56.0 && wx + 16.0 < cx && cy < wy + 56.0 && wy + 16.0 < cy && c.kind < 15 {
                out.picked = Some(i);
                self.unk_1bc = 0;
                break;
            }
        }
        true
    }

    /// `0x004eb7c0`, pet 0 path. No RNG.
    fn animate(&mut self) {
        if self.facing > 0.0 && self.vx < 0.0 {
            self.turn = 20;
        } else if self.facing < 0.0 && self.vx > 0.0 {
            self.turn = -20;
        }
        if self.turn > 0 {
            self.turn -= 1;
        } else if self.turn < 0 {
            self.turn += 1;
        }
        if self.turn == 0 {
            if self.vx.abs() >= 0.3 {
                self.anim = (self.anim + 1) % 20;
                self.cel = self.anim / 2;
            } else {
                self.anim = (self.anim + 1) % 40;
                self.cel = self.anim / 4;
            }
        } else if self.turn > 0 {
            self.cel = 9 - self.turn / 2;
        } else {
            self.cel = self.turn / 2 + 9;
        }
        if self.shell > 0 {
            self.cel = self.shell;
        }
        #[allow(clippy::float_cmp)]
        if self.vx != self.facing && self.vx != 0.0 && self.facing != 0.0 && self.shell == 0 {
            self.facing = self.vx;
        }
    }
}
