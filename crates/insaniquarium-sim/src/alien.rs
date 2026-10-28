//! Aliens (`Sexy::Alien`, type 0x16), Sylvester (kind 1) only.
//! Spec: `docs/re/objects/alien-sylvester.md`.

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
    pub widget_x: i32,
    pub widget_y: i32,
}

/// What an alien tick asks the board to do.
#[derive(Debug, Default)]
pub struct AlienOut {
    /// Index of the fish eaten this tick, in the prey list passed to
    /// `update` (guppies, then carnivores).
    pub ate: Option<usize>,
    pub died: bool,
}

impl Alien {
    /// `Alien::Alien` (`0x004ecfa0`), kind 1: 2 app draws.
    pub fn new(x: i32, y: i32, kind: i32, rng: &mut MtRand) -> Self {
        // `Alien::Alien` (`0x004ecfa0`): kind 2 (1-3) and 3 (1-4) differ
        // from Sylvester only in speed and health.
        let (divisor, health) = match kind {
            2 => (1.6, 60.0),
            3 => (1.2, 130.0),
            _ => (2.0, 50.0),
        };
        let right = rng.next_u31() & 1 != 0;
        let mode = rng.next_range(10) as i32;
        Alien {
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
            widget_x: x,
            widget_y: y,
        }
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
        self.x = self.x + self.vx / self.divisor + lunge;
        self.y += self.vy / self.divisor;
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }
        if self.eat_delay > 0 {
            self.eat_delay -= 1;
        }
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        self.animate();
        out.died = self.health <= 0.0;
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
        match self.mode {
            0 => self.tvx = -1.5,
            1 => self.tvx = 1.5,
            2 => self.tvy = -1.5,
            3 => self.tvy = 1.5,
            _ => {}
        }
        if self.tvy < self.vy {
            self.vy -= 0.1;
        }
        if self.vy < self.tvy {
            self.vy += 0.1;
        }
        if self.tvx < self.vx {
            self.vx -= 0.1;
        }
        if self.vx < self.tvx {
            self.vx += 0.1;
        }
        self.counter += 1;
        self.reroll += 1;
        if self.reroll > 20 {
            self.reroll = 0;
            if rng.next_range(10) == 0 {
                self.mode = (rng.next_u31() & 3) as i32;
            }
        }
    }

    /// §3.4 (`0x004daa00`): animation; strokes also move the alien.
    fn animate(&mut self) {
        if self.last_vx > 0.0 && self.vx < 0.0 {
            self.turn = 10;
        } else if self.last_vx < 0.0 && self.vx > 0.0 {
            self.turn = -10;
        }
        if self.turn > 0 {
            self.turn -= 1;
        } else if self.turn < 0 {
            self.turn += 1;
        }
        if self.turn > 0 {
            self.cel = 9 - self.turn;
        } else if self.turn < 0 {
            self.cel = self.turn + 10;
        } else if (-1.6..=1.6).contains(&self.vx) {
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
        self.health -= f64::from(3 * laser);
        let s = self.divisor;
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
