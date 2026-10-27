//! `Sexy::Missle` (type 0x1e?), board list `+0xdc`. Only Stanley's ball
//! (kind 2) for now. Spec: `docs/re/objects/pet-stanley.md#the-ball-missle-kind-2`.

/// What a missile's update asks the board to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissileStep {
    Keep,
    /// Left the tank: removed (slot 76), then the end-of-wave check.
    Remove,
    /// Hit alien `target` for `damage` HP (the board applies it).
    Hit {
        target: usize,
        damage: i32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Missile {
    /// `+0x158`, `+0x160`
    pub x: f64,
    pub y: f64,
    /// `+0x168`, `+0x170`
    pub vx: f64,
    pub vy: f64,
    /// `+0x17c`: launch timer, −1 per tick while > 0.
    pub launch: i32,
    /// `+0x180`: spin counter 0..19.
    pub spin: i32,
    /// `+0x188`: speed divisor.
    pub divisor: f64,
    /// `+0x190`: bounced off its alien.
    pub bounced: bool,
    /// `+0x194`: cel.
    pub cel: i32,
    /// `+0x19c`: missile kind (2 = Stanley's ball).
    pub kind: i32,
    /// `+0x1a0`: row (ball colour), `grand() & 3`.
    pub row: i32,
    /// `+0x1a4`: index of the target alien in `board.aliens`, if any.
    pub target: Option<usize>,
    pub widget_x: i32,
    pub widget_y: i32,
}

impl Missile {
    /// `Missle::Missle` (`0x004eae40`), kind 2: at rest, launch timer 15,
    /// divisor 0.4, 50 × 50; one **global** draw for the row.
    pub fn new_ball(x: i32, y: i32, target: usize, global: &mut sexy_core::rand::MtRand) -> Self {
        Missile {
            x: f64::from(x),
            y: f64::from(y),
            vx: 0.0,
            vy: 0.0,
            launch: 15,
            spin: 0,
            divisor: 0.4,
            bounced: false,
            cel: 1,
            kind: 2,
            row: (global.next_u31() & 3) as i32,
            target: Some(target),
            widget_x: x,
            widget_y: y,
        }
    }

    /// `Missle::Update` (`0x004ffb30`), kind 2. `target` is the target
    /// alien's widget position (`None` once bounced or when it's gone);
    /// `aliens` = any alien or Bilaterus is in.
    pub fn update(&mut self, target: Option<(i32, i32)>, aliens: bool) -> MissileStep {
        let mut step = MissileStep::Keep;
        if !self.bounced
            && aliens
            && let Some((tx, ty)) = target
        {
            self.steer(tx, ty);
        }
        if !self.bounced
            && let Some((tx, ty)) = target
        {
            let (cx, cy) = (self.x + 25.0, self.y + 25.0);
            let (tx, ty) = (f64::from(tx), f64::from(ty));
            if tx + 20.0 < cx && cx < tx + 140.0 && ty + 20.0 < cy && cy < ty + 140.0 {
                self.bounced = true;
                let t = self.target.take().unwrap_or(0);
                self.divisor = 1.5;
                self.vx = -self.vx;
                self.vy = -self.vy;
                step = MissileStep::Hit {
                    target: t,
                    damage: 15,
                };
            }
        }
        if self.x > 580.0 || self.x < -20.0 || self.y > 380.0 || self.y < 45.0 {
            return MissileStep::Remove;
        }
        if self.bounced {
            self.vy += 0.2;
        }
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        if self.launch > 0 {
            self.launch -= 1;
        }
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        // Spin (`0x004e2170`): by 2, by 1 once bounced.
        let d = if self.bounced { 1 } else { 2 };
        if self.vx < 0.0 {
            self.spin = (self.spin + d) % 20;
        } else {
            self.spin -= d;
            if self.spin < 0 {
                self.spin += 20;
            }
        }
        self.cel = self.spin / 2;
        step
    }

    /// `0x004ff4e0`, kind 2 without a Bilaterus: aim the ball's centre
    /// (x + 25) at the alien's centre (tx + 80).
    fn steer(&mut self, tx: i32, ty: i32) {
        let p = self.x + 25.0;
        let t = f64::from(tx);
        #[allow(clippy::float_cmp)]
        if p > t + 200.0 {
            if self.vx > -1.8 {
                self.vx -= 0.2;
            }
        } else if p < t - 40.0 {
            if self.vx < 1.8 {
                self.vx += 0.2;
            }
        } else if p > t + 110.0 {
            if self.vx > -1.4 {
                self.vx -= 0.2;
            }
        } else if p < t + 50.0 {
            if self.vx < 1.4 {
                self.vx += 0.2;
            }
        } else if p < t + 80.0 {
            if self.vx < 0.6 {
                self.vx += 0.2;
            }
        } else if p != t + 80.0 && self.vx > -0.6 {
            self.vx -= 0.2;
        }
        let q = self.y + 25.0;
        let t = f64::from(ty);
        #[allow(clippy::float_cmp)]
        if q > t + 200.0 {
            if self.vy > -1.8 {
                self.vy -= 0.2;
            }
        } else if q < t - 40.0 {
            if self.vy < 1.8 {
                self.vy += 0.2;
            }
        } else if q > t + 110.0 {
            if self.vy > -1.4 {
                self.vy -= 0.2;
            }
        } else if q < t + 50.0 {
            if self.vy < 1.4 {
                self.vy += 0.2;
            }
        } else if q < t + 80.0 {
            if self.vy < 0.2 {
                self.vy += 0.1;
            }
        } else if q != t + 80.0 && self.vy > -0.2 {
            self.vy -= 0.1;
        }
    }
}
