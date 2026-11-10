//! Aliens (`Sexy::Alien`, type 0x16).
//! Spec: `docs/re/objects/alien-sylvester.md`, `docs/re/objects/alien-kind5.md`,
//! `docs/re/objects/alien-kind6.md`.

use sexy_core::rand::MtRand;

use crate::fish::Fish;

/// Clamp box for aliens (§3 step 4).
const MIN_X: f64 = -10.0;
const MAX_X: f64 = 490.0;
const MIN_Y: f64 = 85.0;
const MAX_Y: f64 = 290.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Alien {
    /// `+0x1ec`
    pub kind: i32,
    /// `+0x158`: warp-in counter (15 → 0).
    pub warp: i32,
    /// `+0x160`, `+0x168` (note: not Fish's offsets)
    pub x: f64,
    pub y: f64,
    /// `+0x170`, `+0x178`
    pub vx: f64,
    pub vy: f64,
    /// `+0x180`, `+0x188`: wander target velocity.
    pub tvx: f64,
    pub tvy: f64,
    /// `+0x1a0`: no eating until 0.
    pub eat_delay: i32,
    /// `+0x1b0`, `+0x1b4`, `+0x1c0`
    pub anim: i32,
    pub turn: i32,
    pub cel: i32,
    /// `+0x1b8`
    pub divisor: f64,
    /// `+0x1c8`: last non-zero vx.
    pub last_vx: f64,
    /// `+0x1d0`, `+0x1d4`, `+0x1d8`
    pub mode: i32,
    pub counter: i32,
    pub reroll: i32,
    /// `+0x1f0`, `+0x1f8`
    pub health: f64,
    pub max_health: f64,
    /// `+0x200`: hit cooldown (white flash).
    pub cooldown: i32,
    /// Kind 5 (alien-kind5.md#32): `+0x1a4` fire timer, `+0x1a8` fire
    /// interval, `+0x1ac` fire animation.
    pub fire_timer: i32,
    pub fire_interval: i32,
    pub fire_anim: i32,
    /// Kind 7: state timer (`+0x194`), period (`+0x198`), invulnerable
    /// flag (`+0x204`).  While invulnerable the alien wanders (divisor
    /// 2.0) and laser hits heal; while normal it chases (divisor 0.5).
    pub state_timer: i32,
    pub state_period: i32,
    pub invulnerable: bool,
    pub widget_x: i32,
    pub widget_y: i32,
}

/// What an alien tick asks the board to do.
#[derive(Debug, Default)]
pub struct AlienOut {
    /// Index of the fish eaten this tick, in the prey list passed to
    /// `update` (guppies, then carnivores).
    pub ate: Option<usize>,
    /// Gus (kind 4) ate pellet `i` of the food list (alien-kind4.md#32).
    pub ate_food: Option<usize>,
    /// The Destructor (kind 5) fires: the board picks targets and makes
    /// missiles from these launch points (alien-kind5.md#32).
    pub volley: Option<[(i32, i32); 3]>,
    pub died: bool,
    /// Kind 7 just transitioned from invulnerable to normal.
    pub became_normal: bool,
}

/// What Gus sees of a pellet (`board+0xac`, alien-kind4.md#31).
#[derive(Debug, Clone, Copy)]
pub struct FoodView {
    pub widget_x: i32,
    pub widget_y: i32,
    pub delay: i32,
    pub kind: i32,
    pub level: i32,
}

impl Alien {
    /// `Alien::Alien` (`0x004ecfa0`), kind 1: 2 app draws.
    pub fn new(x: i32, y: i32, kind: i32, rng: &mut MtRand) -> Self {
        // `Alien::Alien` (`0x004ecfa0`): kind 2 (1-3) and 3 (1-4) differ
        // from Sylvester only in speed and health.
        let (divisor, health) = match kind {
            2 => (1.6, 60.0),
            3 => (1.2, 130.0),
            4 => (1.6, 100.0),
            5 => (1.2, 150.0),
            6 => (3.5, 220.0),
            7 => (0.5, 260.0),
            _ => (2.0, 50.0),
        };
        let right = rng.next_u31() & 1 != 0;
        let mode = rng.next_range(10) as i32;
        let mut a = Alien {
            kind,
            warp: 15,
            x: f64::from(x),
            y: f64::from(y),
            vx: if right { 3.0 } else { -3.0 },
            vy: 0.0,
            tvx: 0.0,
            tvy: 0.0,
            eat_delay: 100,
            anim: 0,
            turn: 0,
            cel: 1,
            divisor,
            last_vx: if right { 1.0 } else { -1.0 },
            mode,
            counter: 40,
            reroll: 20,
            health,
            max_health: health,
            cooldown: 0,
            fire_timer: 0,
            fire_interval: 75,
            fire_anim: 0,
            state_timer: if kind == 7 { 200 } else { 0 },
            state_period: if kind == 7 { 400 } else { 0 },
            invulnerable: false,
            widget_x: x,
            widget_y: y,
        };
        // Kinds 5 and 6 walk the floor at y 280 (alien-kind5.md#constructor).
        if matches!(kind, 5 | 6) {
            a.y = 280.0;
            a.widget_y = 280;
        }
        a
    }

    /// `Alien::Update` (`0x004fd560`), kind 1.
    /// `prey`: every fish the alien can eat (`alien-sylvester.md#31`):
    /// guppies, then carnivores.
    pub fn update(&mut self, guppies: &[&Fish], rng: &mut MtRand, out: &mut AlienOut) {
        self.update_preferring(guppies, None, rng, out);
    }

    /// [`Self::update`], where `prefer` (an index into `guppies`) wins
    /// distance ties and is tested first for the bite. The original walks
    /// candidates in object-address order (`alien-sylvester.md#deviations`);
    /// a replay passes the fish the capture shows it chose.
    pub fn update_preferring(
        &mut self,
        guppies: &[&Fish],
        prefer: Option<usize>,
        rng: &mut MtRand,
        out: &mut AlienOut,
    ) {
        self.update_masked(guppies, &[], prefer, rng, out);
    }

    /// [`Self::update_preferring`] with some fish protected (`hidden[i]`:
    /// Wadsworth's shelter, `0x00500ff0`): never targeted or eaten.
    pub fn update_masked(
        &mut self,
        guppies: &[&Fish],
        hidden: &[bool],
        prefer: Option<usize>,
        rng: &mut MtRand,
        out: &mut AlienOut,
    ) {
        self.update_with_food(guppies, hidden, prefer, &[], rng, out);
    }

    /// [`Self::update_masked`] with the food list, which Gus (kind 4)
    /// chases and eats (alien-kind4.md#3-alienupdate-0x004fd560-kind-4).
    pub fn update_with_food(
        &mut self,
        guppies: &[&Fish],
        hidden: &[bool],
        prefer: Option<usize>,
        foods: &[FoodView],
        rng: &mut MtRand,
        out: &mut AlienOut,
    ) {
        let mut lunge = 0.0;
        if self.warp != 0 {
            if self.warp <= 10 {
                lunge = if self.vx >= 0.0 { 3.0 } else { -3.0 };
            }
            self.warp -= 1;
            if self.warp > 8 {
                return;
            }
        }
        // Chase the nearest guppy, else wander.
        // TODO(spec): candidates come from the master set in object-address
        // order; we use list order (ties are rare: equal squared distance).
        if self.kind == 4 {
            self.gus_chase(guppies, hidden, prefer, foods, rng, out);
        } else if matches!(self.kind, 5 | 6) {
            // The Destructor (5) and kind 6 never chase or eat
            // (alien-kind5.md#3, alien-kind6.md#3).
            // Kind 6: during the slow loop (fire_anim > 0), vx is zeroed
            // before wander so the alien decelerates to tvx + 0.1 in one tick.
            if self.kind == 6 && self.fire_anim > 0 {
                self.vx = 0.0;
            }
            self.wander(rng);
        } else if self.kind == 7 && self.invulnerable {
            // Kind 7 invulnerable: wander instead of chase.
            self.wander(rng);
        } else {
            let target = if guppies.is_empty() {
                None
            } else {
                self.nearest(guppies, hidden, prefer)
            };
            match target {
                Some(t) => {
                    self.steer(guppies[t]);
                    if self.eat_delay < 1 {
                        out.ate = self.eat_test(guppies, hidden, prefer);
                    }
                }
                None => self.wander(rng),
            }
        }
        // Kind 7 state toggle: +0x194 increments each tick; at >= +0x198
        // the state flips (normal ↔ invulnerable), divisor changes, timer
        // resets to rand(100), and the fire animation plays.
        if self.kind == 7 {
            self.state_timer += 1;
            if self.state_timer >= self.state_period {
                let was_inv = self.invulnerable;
                self.invulnerable = !self.invulnerable;
                self.divisor = if self.invulnerable { 2.0 } else { 0.5 };
                self.state_timer = rng.next_range(100) as i32;
                self.fire_anim = 10;
                if was_inv && !self.invulnerable {
                    out.became_normal = true;
                }
            }
        }
        // Explicit compares in the original's order (§3 step 4).
        #[allow(clippy::manual_clamp)]
        if self.x < MIN_X {
            self.x = MIN_X;
        }
        if self.x > MAX_X {
            self.x = MAX_X;
        }
        #[allow(clippy::manual_clamp)]
        if self.y < MIN_Y {
            self.y = MIN_Y;
        }
        if self.y > MAX_Y {
            self.y = MAX_Y;
        }
        // `(x + v/div) + lunge`: two separate adds, in this order.
        // Kind 6's stroke (in animate) modifies vx before the x update,
        // so defer x to after animate to use the post-stroke vx in one
        // division (matching the original's float precision).
        if self.kind != 6 {
            self.x = self.x + self.vx / self.divisor + lunge;
        }
        self.y += self.vy / self.divisor;
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }
        if self.eat_delay > 0 {
            self.eat_delay -= 1;
        }
        // Firing (alien-kind5.md#32, alien-kind6.md#32): while prey exists
        // the timer runs; past the interval a volley (the board picks the
        // targets), then a new interval `rand(50) + 150`.
        // Kind 6: fire_timer pauses during the last ~5 ticks of a turn
        // (|turn| ∈ {1,2,4,5} before the per-tick decrement in animate).
        let k6_gate = self.kind == 6
            && self.turn != 0
            && matches!(self.turn.unsigned_abs(), 1 | 2 | 4 | 5);
        if matches!(self.kind, 5 | 6) && !guppies.is_empty() && !k6_gate {
            self.fire_timer += 1;
            if self.fire_timer > self.fire_interval {
                let (ix, iy) = (self.widget_x, self.widget_y);
                if self.kind == 5 {
                    let (a, s) = if self.vx >= 0.0 { (90, 15) } else { (-10, -15) };
                    out.volley = Some([
                        (ix.wrapping_add(a), iy.wrapping_sub(35)),
                        (ix.wrapping_add(a.wrapping_add(s)), iy.wrapping_sub(30)),
                        (ix.wrapping_add(a.wrapping_add(s * 2)), iy.wrapping_sub(25)),
                    ]);
                } else {
                    // Kind 6: up to 2 missiles from a single facing-dependent
                    // position (alien-kind6.md#32). The third slot is unused;
                    // fire_volley_k1 reads only the first entry.
                    let ox = if self.vx >= 0.0 { 70 } else { 10 };
                    let p = (ix.wrapping_add(ox), iy.wrapping_sub(24));
                    out.volley = Some([p, p, p]);
                }
            }
        }
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        if out.volley.is_none() {
            self.animate();
        }
        // Kind 6: deferred x update using the post-stroke vx (one
        // division, matching the original's float precision).
        if self.kind == 6 {
            self.x = self.x + self.vx / self.divisor + lunge;
            self.widget_x = self.x as i32;
        }
        out.died = self.health <= 0.0;
    }

    /// Gus's chase (alien-kind4.md#30..#32): food first (list order, a
    /// −2500 bias), fish only after the eat delay; the food-branch steer
    /// dives faster; the eat test always runs. The board applies the
    /// pellet's damage (it may draw RNG).
    fn gus_chase(
        &mut self,
        guppies: &[&Fish],
        hidden: &[bool],
        prefer: Option<usize>,
        foods: &[FoodView],
        rng: &mut MtRand,
        out: &mut AlienOut,
    ) {
        if guppies.is_empty() && foods.is_empty() {
            self.wander(rng);
            return;
        }
        let (cx, cy) = self.centre();
        let mut best = 100_000_000;
        let mut food_flag = false;
        let mut target: Option<(i32, i32)> = None;
        for f in foods {
            if f.delay >= 1 || f.kind == 2 {
                continue;
            }
            let dx = cx - (f.widget_x + 20);
            let dy = cy - (f.widget_y + 20);
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                target = Some((f.widget_x + 20, f.widget_y + 20));
                food_flag = true;
                best = d - 2500;
            }
        }
        if self.eat_delay <= 0 {
            let order = prefer
                .filter(|&p| p < guppies.len())
                .into_iter()
                .chain((0..guppies.len()).filter(|&i| Some(i) != prefer));
            for i in order {
                if hidden.get(i) == Some(&true) {
                    continue;
                }
                let g = guppies[i];
                let dx = cx - (g.widget_x + 40);
                let dy = cy - (g.widget_y + 40);
                let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
                if d < best {
                    best = d;
                    // `+0x190` stays set: the food-branch steer, at the
                    // fish's corner + 20 (alien-kind4.md#31, quirk).
                    target = Some(if food_flag {
                        (g.widget_x + 20, g.widget_y + 20)
                    } else {
                        (g.widget_x + 40, g.widget_y + 40)
                    });
                }
            }
        }
        let Some((tx, ty)) = target else {
            self.wander(rng);
            return;
        };
        let (tx, ty) = (f64::from(tx), f64::from(ty));
        if self.x + 80.0 > tx {
            if self.vx > -1.8 {
                self.vx -= 0.1;
            }
        } else if self.x + 80.0 < tx && self.vx < 1.8 {
            self.vx += 0.1;
        }
        if food_flag {
            if self.y + 80.0 > ty {
                if self.vy > -1.8 {
                    self.vy -= 0.1;
                }
            } else if self.y + 80.0 < ty && self.vy < 3.0 {
                self.vy += 0.3;
            }
        } else if self.y + 80.0 > ty {
            if self.vy > -1.8 {
                self.vy -= 0.1;
            }
        } else if self.y + 80.0 < ty && self.vy < 1.8 {
            self.vy += 0.1;
        }
        // Eat test: food (the alien's double position, no kind check),
        // then fish once the eat delay is over.
        let (ax, ay) = (self.x + 80.0, self.y + 80.0);
        if self.cooldown < 6 {
            out.ate_food = foods.iter().position(|f| {
                let (fx, fy) = (f64::from(f.widget_x), f64::from(f.widget_y));
                fx - 25.0 < ax && ax < fx + 65.0 && fy - 25.0 < ay && ay < fy + 65.0 && f.delay == 0
            });
            if let Some(i) = out.ate_food {
                // Damage (alien-kind4.md#32): a star potion 20 (the board
                // adds its explosion), else 2·level + 4.
                let f = foods[i];
                self.health -= if f.level == 3 {
                    20.0
                } else {
                    f64::from(2 * f.level + 4)
                };
                self.cooldown = 10;
            }
        }
        if self.eat_delay <= 0 {
            out.ate = self.eat_test(guppies, hidden, prefer);
            // The "ate" hook (`0x004d45a0`), kind 4: the gulp and 15 HP,
            // inside this update (before its timers and death check).
            if out.ate.is_some() {
                self.cooldown = 10;
                self.health -= 15.0;
            }
        }
    }

    fn centre(&self) -> (i32, i32) {
        (self.widget_x + 80, self.widget_y + 80)
    }

    /// `0x004e8b60`: nearest guppy by widget centre.
    fn nearest(&self, guppies: &[&Fish], hidden: &[bool], prefer: Option<usize>) -> Option<usize> {
        let (cx, cy) = self.centre();
        let mut best = 100_000_000;
        let mut found = None;
        let order = prefer
            .filter(|&p| p < guppies.len())
            .into_iter()
            .chain((0..guppies.len()).filter(|&i| Some(i) != prefer));
        for i in order {
            if hidden.get(i) == Some(&true) {
                continue;
            }
            let g = guppies[i];
            let dx = cx - (g.widget_x + 40);
            let dy = cy - (g.widget_y + 40);
            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
            if d < best {
                best = d;
                found = Some(i);
            }
        }
        found
    }

    /// `0x004ed460`, default branch.
    fn steer(&mut self, t: &Fish) {
        let (tx, ty) = (f64::from(t.widget_x + 40), f64::from(t.widget_y + 40));
        if self.x + 80.0 > tx {
            if self.vx > -1.8 {
                self.vx -= 0.1;
            }
        } else if self.x + 80.0 < tx && self.vx < 1.8 {
            self.vx += 0.1;
        }
        if self.y + 80.0 > ty {
            if self.vy > -1.8 {
                self.vy -= 0.1;
            }
        } else if self.y + 80.0 < ty && self.vy < 1.8 {
            self.vy += 0.1;
        }
    }

    /// `0x004e8f70`: first guppy inside the 45×65 half-extent box.
    fn eat_test(&self, guppies: &[&Fish], hidden: &[bool], prefer: Option<usize>) -> Option<usize> {
        let (cx, cy) = self.centre();
        let open = |i: usize| hidden.get(i) != Some(&true);
        let inside = |g: &Fish| {
            let dx = cx - (g.widget_x + 40);
            let dy = cy - (g.widget_y + 40);
            dx.abs() < 45 && dy.abs() < 65
        };
        prefer
            .filter(|&p| p < guppies.len() && open(p) && inside(guppies[p]))
            .or_else(|| (0..guppies.len()).find(|&i| open(i) && inside(guppies[i])))
    }

    /// §3.3 wander.
    fn wander(&mut self, rng: &mut MtRand) {
        let floor = matches!(self.kind, 5 | 6);
        match self.mode {
            0 if floor => self.tvx = -1.0,
            2 if floor => self.tvx = -1.0,
            1 | 3 if floor => self.tvx = 1.0,
            0 => self.tvx = -1.5,
            1 => self.tvx = 1.5,
            2 => self.tvy = -1.5,
            3 => self.tvy = 1.5,
            _ => {}
        }
        if !floor {
            if self.tvy < self.vy {
                self.vy -= 0.1;
            }
            if self.vy < self.tvy {
                self.vy += 0.1;
            }
        }
        if self.tvx < self.vx {
            self.vx -= 0.1;
        }
        if self.vx < self.tvx {
            self.vx += 0.1;
        }
        self.counter += 1;
        // The re-roll timer pauses during the fire animation (kinds 5/6/7).
        if self.fire_anim == 0 {
            self.reroll += 1;
        }
        if self.reroll > 20 {
            self.reroll = 0;
            if rng.next_range(10) == 0 {
                self.mode = (rng.next_u31() & 3) as i32;
            }
        }
    }

    /// After a volley the board made: reset the timer, draw the next
    /// interval (`rand(50) + 150`), start the fire animation if any missile
    /// went out, then the animation step the update postponed.
    pub fn finish_volley(&mut self, fired: bool, rng: &mut MtRand) {
        self.fire_timer = 0;
        self.fire_interval = rng.next_range(50) as i32 + 150;
        if fired {
            self.fire_anim = 10;
        }
        self.animate();
    }

    /// §3.4 (`0x004daa00`): animation; strokes also move the alien.
    fn animate(&mut self) {
        // Kind 6 uses an 18-tick turn arc; other kinds use 10.
        let turn_mag = if self.kind == 6 { 20 } else { 10 };
        if self.last_vx > 0.0 && self.vx < 0.0 {
            self.turn = turn_mag;
        } else if self.last_vx < 0.0 && self.vx > 0.0 {
            self.turn = -turn_mag;
        }
        if self.turn > 0 {
            self.turn -= 1;
        } else if self.turn < 0 {
            self.turn += 1;
        }
        // The Destructor's fire animation comes first (alien-kind5.md#33).
        // Kind 6 handles fire_anim in its own turn==0 branch (slow loop).
        let firing = self.kind != 6 && self.fire_anim > 0 && {
            self.fire_anim -= 1;
            if self.fire_anim == 0 {
                self.anim = 0;
            }
            self.fire_anim > 0
        };
        if firing {
            // Kind 7 state-transition: cel direction depends on whether
            // the alien is now invulnerable (counts up: 1..9) or normal
            // (counts down: 9..1). Kinds 5/6 always count down.
            self.cel = if self.kind == 7 && self.invulnerable {
                10 - self.fire_anim
            } else {
                self.fire_anim
            };
        } else if self.turn > 0 {
            // Kind 6 uses `9 - |turn|/2` (cel cycles 0..9 over 18 ticks)
            // and applies stroke vx deltas during the turn; other kinds
            // use `9 - turn` (10-tick cycle) with no vx modification.
            if self.kind == 6 {
                self.vx += self.kind6_turn_delta();
                self.cel = 9 - self.turn / 2;
            } else {
                self.cel = 9 - self.turn;
            }
        } else if self.turn < 0 {
            if self.kind == 6 {
                self.vx += self.kind6_turn_delta();
                self.cel = 9 - self.turn.abs() / 2;
            } else {
                self.cel = self.turn + 10;
            }
        } else if self.kind == 6 {
            // Kind-6 slow loop: fire_anim counts down (39→0 after a
            // stroke cycle, or 10→0 after a volley); cel via
            // (39 − fa) / 2 % 10.  Only runs when turn == 0.
            if self.fire_anim > 0 {
                self.fire_anim -= 1;
                if self.fire_anim > 0 {
                    self.cel = ((39 - self.fire_anim) / 2) % 10;
                    return;
                }
                // fire_anim reached 0: restart the stroke.
                self.anim = 0;
            }
            // Kind-6 stroke: runs anim from 0 upward with vx deltas.
            // The stroke is 59 ticks (a=1..59). When anim > 58, the
            // stroke is done: start the slow loop (fire_anim = 39).
            if self.anim > 58 {
                let dir = if self.vx >= 0.0 { 1.0_f64 } else { -1.0 };
                self.vx += dir * 0.55;
                self.fire_anim = 39;
                self.cel = 0;
                return;
            }
            self.anim += 1;
            let a = self.anim;
            let dir = if self.vx >= 0.0 { 1.0_f64 } else { -1.0 };
            let delta = if a == 1 || (9..=24).contains(&a) || (49..=58).contains(&a) {
                dir * 0.55
            } else if (2..=4).contains(&a) || a == 6 || (33..=40).contains(&a) || a == 48 {
                -dir * 0.80
            } else {
                0.0
            };
            self.vx += delta;
            self.cel = a / 8;
        } else if matches!(self.kind, 5 | 7) || (-1.6..=1.6).contains(&self.vx) {
            self.anim += 1;
            if self.anim > 19 {
                self.anim = 0;
            }
            self.cel = self.anim / 2;
        } else {
            // Stroke (`0x004daa00`): the alien pulls back, then surges.
            self.anim += 1;
            let (sx, sy) = (self.vx / self.divisor, self.vy / self.divisor);
            let a = self.anim;
            if a < 7 {
                self.x -= sx * 0.5 * 0.5;
                self.y -= sy * 0.5 * 0.5;
                self.cel = a / 2;
            } else if a < 11 {
                self.x += sx * 0.5 * 1.5;
                self.y += sy * 0.5 * 1.5;
                self.cel = a / 2;
            } else if a < 40 {
                self.cel = 6;
                self.x += sx * 0.5;
                self.y += sy * 0.5;
            } else if a > 50 {
                self.cel = 0;
                self.anim = 0;
            } else {
                self.x += sx * 0.5;
                self.y += sy * 0.5;
                self.cel = (a - 50).abs() / 2;
            }
        }
        #[allow(clippy::float_cmp)]
        if self.vx != self.last_vx && self.vx != 0.0 && self.last_vx != 0.0 {
            self.last_vx = self.vx;
        }
    }

    /// Stroke-like vx delta applied during the kind-6 turn animation.
    /// The 18-tick turn, keyed on `abs(turn)` after the per-tick decay.
    fn kind6_turn_delta(&self) -> f64 {
        let dir = if self.vx >= 0.0 { 1.0_f64 } else { -1.0 };
        match self.turn.unsigned_abs() {
            16 | 15 | 14 | 13 | 6 | 5 | 4 | 3 => dir * 0.55,
            10 => -dir * 0.80,
            _ => 0.0,
        }
    }

    /// The widget rect test (`Widget` slot 68 @ `0x0046e800`): the 160×160
    /// widget contains the point (integer, half-open).
    pub fn contains(&self, x: i32, y: i32) -> bool {
        (self.widget_x..self.widget_x + 160).contains(&x)
            && (self.widget_y..self.widget_y + 160).contains(&y)
    }

    /// `Alien::Hit(cx, cy)` (`0x004fa6f0`): returns `Some(killed)` if hit.
    pub fn hit(&mut self, cx: i32, cy: i32, laser: i32) -> Option<bool> {
        let (cx, cy) = (f64::from(cx), f64::from(cy));
        if !(self.x < cx && cx < self.x + 160.0 && self.y < cy && cy < self.y + 160.0) {
            return None;
        }
        if self.cooldown > 0 {
            return None;
        }
        let s = self.divisor;
        if matches!(self.kind, 5 | 6 | 7) {
            // Kinds 5/6/7: 2·laser + 2; sideways knockback only, none
            // during the fire animation.  Kind 7 invulnerable: heals.
            let dmg = f64::from(2 * laser + 2);
            if self.kind == 7 && self.invulnerable {
                self.health += dmg;
            } else {
                self.health -= dmg;
            }
            // Kind 7 uses a 4× multiplier (giving 2.0 at divisor 0.5,
            // i.e. 4.0 visual px/tick — faster than the 3.0 used by
            // kinds 5/6); kinds 5/6 use 3× (the same as kinds 1–3).
            let kb = if self.kind == 7 { 4.0 } else { 3.0 };
            if self.fire_anim == 0 {
                if cx < self.x + 60.0 {
                    self.vx = kb * s;
                } else if cx > self.x + 100.0 {
                    self.vx = -kb * s;
                }
            }
            if self.health <= 0.0 {
                return Some(true);
            }
            self.cooldown = 10;
            return Some(false);
        }
        self.health -= f64::from(3 * laser);
        let (left, right) = (cx < self.x + 60.0, cx > self.x + 100.0);
        let (top, bottom) = (cy < self.y + 60.0, cy > self.y + 100.0);
        // Knockback away from the click (§4.2 step 5).
        match (left, right, top, bottom) {
            (true, _, true, _) => {
                self.vx = 2.5 * s;
                self.vy = 2.5 * s;
            }
            (true, _, false, false) => self.vx = 3.0 * s,
            (true, _, _, true) => {
                self.vx = 2.5 * s;
                self.vy = -2.5 * s;
            }
            (false, false, true, _) => self.vy = 3.0 * s,
            (false, false, _, true) => self.vy = -3.0 * s,
            (_, true, true, _) => {
                self.vx = -2.5 * s;
                self.vy = 2.5 * s;
            }
            (_, true, false, false) => self.vx = -3.0 * s,
            (_, true, _, true) => {
                self.vx = -2.5 * s;
                self.vy = -2.5 * s;
            }
            _ => {}
        }
        if self.health <= 0.0 {
            return Some(true);
        }
        self.cooldown = 10;
        Some(false)
    }
}

/// A `Shot` (type 0x20, list `+0xd8`): click flashes and effect sparks.
/// Cosmetic, but its constructor draws RNG and its kind field is read
/// uninitialised in the original (see `Board::alloc_shot`).
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    /// `+0x160`
    pub kind: i32,
    /// `+0x164`: start delay.
    pub delay: i32,
    /// `+0x158`: age.
    pub age: i32,
    /// `+0x15c`: frame.
    pub frame: i32,
    /// `+0x168`: `r % 200 + 50` (or 255 for click shots).
    pub r: i32,
    pub widget_x: i32,
    pub widget_y: i32,
}

impl Shot {
    /// `Shot::Update` (`0x004f3d90`): true while alive.
    pub fn update(&mut self) -> bool {
        if self.delay != 0 {
            self.delay -= 1;
            return true;
        }
        self.age += 1;
        let limit = match self.kind {
            6 | 7 => {
                self.widget_y -= 2;
                0x27
            }
            0 => 0xe,
            2 => 0x1d,
            1 => 10,
            _ => {
                if self.r > 0x96 {
                    self.age += 1;
                }
                0x13
            }
        };
        let alive = self.age <= limit;
        self.frame = if self.kind == 0 {
            self.age
        } else {
            self.age / 2
        } - 1;
        alive
    }
}

/// `DeadAlien` (type 0x1a, list `+0x9c`): sinking corpse that spawns
/// debris bursts (RNG) for 125 ticks (`alien-sylvester.md#74-dead-alien`).
#[derive(Debug, Clone, PartialEq)]
pub struct DeadAlien {
    /// `+0x198`: lifetime (125 → 0).
    pub life: i32,
    pub widget_x: i32,
    pub widget_y: i32,
    pub kind: i32,
    /// `+0x158`: drawn mirrored (the alien faced right).
    pub facing_right: bool,
    /// `+0x188`: the alien's cel when it died.
    pub cel: i32,
    /// `+0x168`, `+0x160`: position.
    pub x: f64,
    pub y: f64,
    /// `+0x170`, `+0x178`: drift.
    pub vx: f64,
    pub vy: f64,
    /// `+0x190`: opacity, 1.0 → 0.
    pub fade: f64,
}

impl DeadAlien {
    /// `DeadAlien::DeadAlien` (`0x004eec10`). No RNG.
    pub fn new(ix: i32, iy: i32, cel: i32, kind: i32, facing_right: bool) -> Self {
        DeadAlien {
            life: 125,
            widget_x: ix,
            widget_y: iy,
            kind,
            facing_right,
            cel,
            x: f64::from(ix),
            y: f64::from(iy),
            vx: 0.0,
            vy: 0.0,
            fade: 1.0,
        }
    }

    /// The non-RNG part of `DeadAlien::Update` (`0x004f6050`), after the
    /// debris bursts: fade, drift and sink.
    pub fn drift(&mut self) {
        if self.life < 105 {
            self.fade -= 0.02;
            if self.fade < 0.0 {
                self.fade = 0.0;
            }
        }
        if self.vx <= 0.0 {
            self.vx += 0.03;
        } else {
            self.vx -= 0.03;
        }
        if self.vy < 2.0 {
            self.vy += 0.05;
        }
        // Explicit compares in the original's order.
        #[allow(clippy::manual_clamp)]
        if self.x > 490.0 {
            self.x = 490.0;
        }
        if self.x < -10.0 {
            self.x = -10.0;
        }
        // Kinds 5/6 lie on the floor: y ≤ 280 before the move
        // (alien-kind5.md#44).
        if matches!(self.kind, 5 | 6) && self.y > 280.0 {
            self.y = 280.0;
        }
        self.x += self.vx;
        self.y += self.vy;
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
    }
}

/// `Warp` (type 0x21, list `+0xf0`): the hole an alien arrives through.
/// Cosmetic, no RNG (`alien-sylvester.md#72-warp-hole`).
#[derive(Debug, Clone, PartialEq)]
pub struct Warp {
    /// `+0x15c`: 36 → −1.
    pub counter: i32,
    pub widget_x: i32,
    pub widget_y: i32,
}

impl Warp {
    /// `Warp::Warp(alien.ix + 30, alien.iy − 40)` (`0x004ecf00`).
    pub fn for_alien(a: &Alien) -> Self {
        Warp {
            counter: 36,
            widget_x: a.widget_x + 30,
            widget_y: a.widget_y - 40,
        }
    }

    /// `Warp::Update` (`0x004f3e70`): true while alive.
    pub fn update(&mut self) -> bool {
        if self.counter < 1 {
            return false;
        }
        self.counter -= 1;
        true
    }

    /// Animation frame (17 cels); ≥ 17 draws nothing.
    pub fn frame(&self) -> i32 {
        17 - self.counter / 2
    }
}

/// `EffectShot` (`0x005436f0`): only while fewer than 20 shots; one draw.
///
/// Deviation (implementation artifact): the original's Shot constructor
/// reads its kind field before setting it, and draws an extra `% 3` start
/// delay when the recycled heap block happened to hold kind 1 or 3+.
/// The intent is no delay (the field is set right after), so we never
/// draw it (`alien-sylvester.md#deviations`).
pub fn add_effect_shot(shots: &mut Vec<Shot>, rng: &mut MtRand, x: i32, y: i32, kind: i32) {
    add_effect_shot_with(shots, rng, x, y, kind, &mut ArtifactPlan::default());
}

/// Which effect shots of a tick get the original's stale start-delay draw.
/// The sim never makes it (see [`add_effect_shot`]); a differential harness
/// replaying a capture can, to keep everything after it in step. Empty by
/// default: no artifact draws.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactPlan {
    /// One flag per effect shot made this tick, in order.
    pub delay_draws: Vec<bool>,
    /// When fish tie for an alien's attention (equal distance, or several
    /// in its eat box), the original takes the first in object-address
    /// order (`alien-sylvester.md#deviations`); the sim takes the first in
    /// list order. A replay can name the preferred fish's index in the prey
    /// list (guppies, then carnivores).
    pub eat_choice: Option<usize>,
    /// Walter's glove hitting several objects in one tick walks them in
    /// heap-address order in the original (`pet-walter.md#deviations`); the
    /// sim walks guppies, carnivores, then fish pets. A replay can reverse
    /// that order (`true`) when the capture's pushes say so.
    pub glove_reversed: bool,
    /// The Destructor's PickTarget walks the master set in heap-address
    /// order, so an exact distance tie (two fish at the same spot) goes to
    /// the earlier one in address order (`alien-kind5.md#61`). The sim takes
    /// the earlier in list order; a replay can make ties go to the later.
    pub volley_ties_last: bool,
    /// Effect shots made so far this tick.
    pub made: usize,
}

/// [`add_effect_shot`] following `plan`.
pub fn add_effect_shot_with(
    shots: &mut Vec<Shot>,
    rng: &mut MtRand,
    x: i32,
    y: i32,
    kind: i32,
    plan: &mut ArtifactPlan,
) {
    if shots.len() >= 20 {
        return;
    }
    let mut delay = 0;
    if plan.delay_draws.get(plan.made).copied().unwrap_or(false) {
        delay = rng.next_range(3) as i32;
    }
    plan.made += 1;
    let r = rng.next_range(200) as i32 + 50;
    shots.push(Shot {
        kind,
        delay,
        age: 0,
        frame: 0,
        r,
        widget_x: x,
        widget_y: y,
    });
}
