//! Coins (`Sexy::Coin`, type 0x19). Spec: `docs/re/objects/coin.md`.

/// Coin floor (`y` of a resting coin).
const FLOOR_Y: f64 = 370.0;
/// Where collected coins fly to (the money counter).
const BANK: (f64, f64) = (550.0, 30.0);

/// Coin kinds (`+0x194`). Only the guppy drops are modelled.
pub mod kind {
    pub const SILVER: i32 = 1;
    pub const GOLD: i32 = 2;
    pub const STAR: i32 = 3;
    pub const DIAMOND: i32 = 4;
    /// Niko's pearl (pet-niko.md#the-pearl).
    pub const PEARL: i32 = 15;
    /// Shrapnel's bomb (pet-shrapnel.md#the-bomb-coin-kind-0x11).
    pub const BOMB: i32 = 0x11;
}

/// The rising speed of a tossed coin or pellet at height `y`
/// (`Coin::Update` / `Food::Update`, pet-nimbus.md).
pub fn rise_step(y: f64) -> f64 {
    if y >= 150.0 {
        8.0
    } else if y >= 135.0 {
        2.5
    } else if y >= 130.0 {
        1.5
    } else if y >= 125.0 {
        1.0
    } else {
        0.5
    }
}

/// A resting pearl disappears after this many ticks (`0xd9`; `static`).
const PEARL_LIFE: i32 = 217;

#[derive(Debug, Clone, PartialEq)]
pub struct Coin {
    /// `+0x158`, `+0x160`: top-left of the 72×72 widget.
    pub x: f64,
    pub y: f64,
    /// `+0x16c`: animation counter, wraps after 79.
    pub anim: i32,
    /// `+0x170`: age in ticks.
    pub age: i32,
    /// `+0x174`: cel to draw.
    pub cel: i32,
    /// `+0x178`: expiry countdown (removed on reaching 0).
    pub expire: i32,
    /// `+0x17c`: steps flown since collection.
    pub flight: i32,
    /// `+0x180`, `+0x184`: position when collected.
    pub from_x: i32,
    pub from_y: i32,
    /// `+0x188`: fall speed (only used in some modes).
    pub vy: f64,
    /// `+0x194`: coin kind.
    pub kind: i32,
    /// `+0x198`: collected; flying to the money counter.
    pub collected: bool,
    /// `+0x19c`: ticks spent resting on the floor.
    pub floor_time: i32,
    /// `+0x1c0`: value multiplier (Virtual Tank shells).
    pub multiplier: i32,
    /// `+0x168`: a rising coin (kinds 5, 8–14, 0x12) has reached the top
    /// and now falls like any coin (pet-nimbus.md).
    pub risen: bool,
    /// `+0x30`, `+0x34`: truncated position.
    pub widget_x: i32,
    pub widget_y: i32,
}

/// What a coin update asks the board to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinStep {
    Keep,
    /// Gone without paying (expired).
    Remove,
    /// Reached the counter: pay `value`.
    Bank(i32),
    /// A bomb's rest ran out: remove it with the explosion.
    Explode,
}

impl Coin {
    /// `Board::AddCoin` (`0x00544430`) → `Coin::Coin` (`0x004eead0`) with
    /// the default fall speed.
    pub fn new(x: i32, y: i32, kind: i32) -> Self {
        Coin {
            x: f64::from(x),
            y: f64::from(y),
            anim: 0,
            age: 0,
            cel: 0,
            expire: 0,
            flight: 0,
            from_x: 0,
            from_y: 0,
            vy: 1.5,
            kind,
            collected: false,
            floor_time: 0,
            multiplier: 1,
            risen: false,
            widget_x: x,
            widget_y: y,
        }
    }

    /// Niko's pearl (`AddCoin(x, y, 15, owner, vy 1.0)`).
    pub fn new_pearl(x: i32, y: i32) -> Self {
        let mut c = Coin::new(x, y, kind::PEARL);
        c.vy = 1.0;
        c
    }

    /// Value in money when banked (`0x004d53a0`, normal game).
    pub fn value(&self) -> i32 {
        match self.kind {
            1 | 8 | 0x12 => 15,
            2 | 9 => 35,
            3 | 10 => 40,
            4 | 11 | 5 | 12 => 200,
            6 | 13 => 500,
            7 | 14 => 2000,
            0x11 => 150,
            0xf => 250,
            _ => 0,
        }
    }

    /// Does a click at board coordinates land on this coin's widget?
    pub fn hit(&self, x: i32, y: i32) -> bool {
        (self.widget_x..self.widget_x + 72).contains(&x)
            && (self.widget_y..self.widget_y + 72).contains(&y)
    }

    /// `Coin` mouse-down → collect (`0x004e51f0`), basic coins.
    /// Returns whether the click was taken (it always is, even if already
    /// collected, since the widget swallows it).
    pub fn collect(&mut self) {
        if self.collected {
            return;
        }
        self.flight = 1;
        self.from_x = self.x as i32;
        self.from_y = self.y as i32;
        self.collected = true;
    }

    /// `Coin::Update` (`0x004f5050`), normal game, basic coin kinds.
    /// `long_rest` is the tutorial level's longer floor time (150 vs 20).
    pub fn update(&mut self, long_rest: bool) -> CoinStep {
        self.update_with(long_rest, false)
    }

    /// [`Self::update`] with Seymour in the tank (`board+0x16c`,
    /// pet-seymour.md): slower fall (0.8), 200-tick rest, half-speed spin.
    /// A bomb (kind 0x11) explodes instead of fading: [`CoinStep::Explode`].
    pub fn update_with(&mut self, long_rest: bool, seymour: bool) -> CoinStep {
        self.anim += 1;
        self.age += 1;
        if self.anim > 79 {
            self.anim = 0;
        }
        self.cel = if self.kind == kind::BOMB {
            (self.anim / 2) % 5
        } else if seymour {
            (self.anim / 4) % 10
        } else {
            (self.anim / 2) % 10
        };
        if self.expire != 0 {
            if self.collected {
                self.expire = 0;
            } else {
                self.expire -= 1;
                return if self.expire == 0 {
                    CoinStep::Remove
                } else {
                    CoinStep::Keep
                };
            }
        }
        if self.kind == kind::PEARL && !self.collected {
            // A pearl only waits (`+0x48`, our floor time), then vanishes.
            self.floor_time += 1;
            return if self.floor_time >= PEARL_LIFE {
                CoinStep::Remove
            } else {
                CoinStep::Keep
            };
        }
        if self.collected {
            // Glide toward the money counter, 1/7 of the way per tick.
            if self.x > BANK.0 {
                self.x -= (self.x - BANK.0) / 7.0;
            } else if self.x < BANK.0 {
                self.x += (BANK.0 - self.x) / 7.0;
            }
            if self.y > BANK.1 {
                self.y -= (self.y - BANK.1) / 7.0;
            } else if self.y < BANK.1 {
                self.y += (BANK.1 - self.y) / 7.0;
            }
            if self.widget_y < 40 {
                return CoinStep::Bank(self.value() * self.multiplier);
            }
        } else if !self.risen && matches!(self.kind, 5 | 8..=14 | 0x12) {
            // Rising kinds go up until y 120 (pet-nimbus.md#effects-on-other-objects).
            if self.y < 120.0 {
                self.risen = true;
            } else {
                self.y -= rise_step(self.y);
            }
        } else {
            self.y += if seymour { 0.8 } else { 1.5 };
            if self.y >= FLOOR_Y {
                self.y = FLOOR_Y;
                self.floor_time += 1;
                let limit = if seymour {
                    200
                } else if long_rest {
                    150
                } else {
                    20
                };
                if self.floor_time >= limit {
                    if self.kind == kind::BOMB {
                        return CoinStep::Explode;
                    }
                    self.expire = 5;
                }
            }
        }
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        CoinStep::Keep
    }
}
