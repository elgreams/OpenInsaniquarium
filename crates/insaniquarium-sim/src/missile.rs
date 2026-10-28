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
    /// Kind 0: reached its fish (the board kills it, then explodes this).
    HitFish,
    /// Kind 0: its target is gone: explode.
    Explode,
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
    /// `+0x1a4`: index of the target alien in `board.aliens`, if any
    /// (Stanley's ball). A kind-0 missile's fish points back at it by `id`.
    pub target: Option<usize>,
    /// Identity, for the fish's `+0x98` link.
    pub id: u32,
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
            id: 0,
            widget_x: x,
            widget_y: y,
        }
    }

    /// `Missle::Missle` (`0x004eae40`), kind 0: the Destructor's missile
    /// (alien-kind5.md#51): at rest, launch timer 15, divisor 0.8, 80 × 80;
    /// one global draw for the row.
    pub fn new_missile(x: i32, y: i32, id: u32, global: &mut sexy_core::rand::MtRand) -> Self {
        let mut m = Missile::new_ball(x, y, 0, global);
        m.kind = 0;
        m.divisor = 0.8;
        m.target = None;
        m.id = id;
        m
    }

    /// `Missle::Update` (`0x004ffb30`), kind 0, up to the smoke (the board
    /// draws it on `Keep`). `target` is its fish's widget position.
    pub fn update_missile(&mut self, target: Option<(i32, i32)>) -> MissileStep {
        let Some((tx, ty)) = target else {
            return MissileStep::Explode;
        };
        self.steer_missile(f64::from(tx), f64::from(ty));
        let (p, q) = (self.x + 40.0, self.y + 40.0);
        let (tx, ty) = (f64::from(tx), f64::from(ty));
        if tx + 10.0 < p && p < tx + 70.0 && ty + 10.0 < q && q < ty + 70.0 {
            return MissileStep::HitFish;
        }
        self.x = self.x.clamp(10.0, 550.0);
        self.y = self.y.clamp(95.0, 370.0);
        self.x += self.vx / self.divisor;
        self.y += self.vy / self.divisor;
        if self.launch > 0 {
            self.launch -= 1;
        }
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        self.cel = self.heading_cel();
        MissileStep::Keep
    }

    /// `0x004ff4e0`, kind 0, an 80 × 80 target: aim the centre (x + 40) at
    /// the fish's centre (alien-kind5.md#61).
    fn steer_missile(&mut self, tx: f64, ty: f64) {
        let p = self.x + 40.0;
        #[allow(clippy::float_cmp)]
        if p > tx + 80.0 {
            if self.vx > -1.8 {
                self.vx -= 0.2;
            }
        } else if p < tx {
            if self.vx < 1.8 {
                self.vx += 0.2;
            }
        } else if p > tx + 60.0 {
            if self.vx > -1.4 {
                self.vx -= 0.2;
            }
        } else if p < tx + 20.0 {
            if self.vx < 1.4 {
                self.vx += 0.2;
            }
        } else if p < tx + 40.0 {
            if self.vx < 0.6 {
                self.vx += 0.2;
            }
        } else if p != tx + 40.0 && self.vx > -0.6 {
            self.vx -= 0.2;
        }
        let q = self.y + 40.0;
        #[allow(clippy::float_cmp)]
        if q > ty + 80.0 {
            if self.vy > -1.8 {
                self.vy -= 0.2;
            }
        } else if q < ty {
            if self.vy < 1.8 {
                self.vy += 0.2;
            }
        } else if q > ty + 60.0 {
            if self.vy > -1.4 {
                self.vy -= 0.2;
            }
        } else if q < ty + 20.0 {
            if self.vy < 1.4 {
                self.vy += 0.2;
            }
        } else if q < ty + 40.0 {
            if self.vy < 0.2 {
                self.vy += 0.1;
            }
        } else if q != ty + 40.0 && self.vy > -0.2 {
            self.vy -= 0.1;
        }
    }

    /// `0x004e2170`, kind 0: 0 down, 4 left, 8 up, 12 right.
    fn heading_cel(&self) -> i32 {
        let (vx, vy) = (self.vx, self.vy);
        if -0.8 < vy && vy < 0.8 {
            return if vx > 0.0 { 12 } else { 4 };
        }
        if vx > 2.5 {
            return 12;
        }
        if vx < -2.5 {
            return 4;
        }
        let up = vy <= -0.8;
        let band = if vx > 2.0 {
            0
        } else if vx > 1.5 {
            1
        } else if vx > 1.0 {
            2
        } else if vx >= -1.0 {
            3
        } else if vx >= -1.5 {
            4
        } else if vx >= -2.0 {
            5
        } else {
            6
        };
        if up {
            [11, 10, 9, 8, 7, 6, 5][band]
        } else {
            [13, 14, 15, 0, 1, 2, 3][band]
        }
    }

    /// The board's click walk (`0x004d82a0`), kind 0: a 60 × 60 box round
    /// the centre, once 15 ticks old.
    pub fn clicked(&self, cx: i32, cy: i32) -> bool {
        let (p, q, cx, cy) = (self.x + 40.0, self.y + 40.0, f64::from(cx), f64::from(cy));
        self.kind == 0
            && self.launch < 1
            && p - 30.0 < cx
            && cx < p + 30.0
            && q - 30.0 < cy
            && cy < q + 30.0
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

/// Walter's boxing glove (`Sexy::BoxingGlove`, pet-walter.md#the-boxing-glove).
/// Owned by Walter, on no board list; he updates and draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glove {
    /// `+0x30`, `+0x34`: widget position (−200, −200 until it first moves).
    pub widget_x: i32,
    pub widget_y: i32,
    /// `+0x15c`: life, 30 → 0.
    pub life: i32,
    /// `+0x160`: true = right.
    pub right: bool,
    /// `+0x164`: punch-sound cooldown.
    pub sound_cooldown: i32,
}

impl Glove {
    pub fn new(right: bool) -> Self {
        Glove {
            widget_x: -200,
            widget_y: -200,
            life: 30,
            right,
            sound_cooldown: 0,
        }
    }

    /// `BoxingGlove::Update` (`0x004e9a60`) up to the hit pass: waits while
    /// the owner's punch timer is above 35, else follows him and counts
    /// down. Returns the fist point while it can hit (life > 12).
    pub fn update(&mut self, owner_wx: i32, owner_wy: i32, punch_timer: i32) -> Option<(i32, i32)> {
        if punch_timer > 35 {
            return None;
        }
        self.widget_x = owner_wx + if self.right { -20 } else { -60 };
        self.widget_y = owner_wy;
        self.life -= 1;
        if self.life <= 0 {
            return None;
        }
        if self.sound_cooldown > 0 {
            self.sound_cooldown -= 1;
        }
        if self.life <= 12 {
            return None;
        }
        let reach = match self.life {
            26.. => 0,
            21..=25 => 30,
            _ => 60,
        };
        let px = self.widget_x + 80 + if self.right { reach } else { -reach };
        Some((px, self.widget_y + 40))
    }
}
