//! Food pellets (`Sexy::Food`, type 0x1c). Spec: `docs/re/objects/food.md`.

use sexy_core::rand::MtRand;

/// Floor where pellets settle and start fading.
const FLOOR_Y: f64 = 410.0;
/// Ticks a settled pellet takes to fade out.
const FADE_TICKS: i32 = 15;

#[derive(Debug, Clone, PartialEq)]
pub struct Food {
    /// `+0x158`, `+0x160`: top-left of the 40×40 widget.
    pub x: f64,
    pub y: f64,
    /// `+0x168`, `+0x170`: only drifting food (motion ≠ 0) uses these.
    pub vx: f64,
    pub vy: f64,
    /// `+0x178`, `+0x17c`: animation counter and speed (3 or 4).
    pub anim: i32,
    pub anim_speed: i32,
    /// `+0x180`: fade-out countdown; removed on reaching 0.
    pub fade: i32,
    /// `+0x185`: carried (can't be eaten).
    pub carried: bool,
    /// `+0x188`: food level (0 = basic pellet).
    pub level: i32,
    /// `+0x18c`: motion type (0 = straight down).
    pub motion: i32,
    /// `+0x194`: 0 pellet, 2 potion.
    pub kind: i32,
    /// `+0x198`: ticks until edible.
    pub delay: i32,
    /// `+0x30`, `+0x34`: truncated position.
    pub widget_x: i32,
    pub widget_y: i32,
}

/// What a food update asks the board to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodStep {
    Keep,
    Remove,
}

impl Food {
    /// `Food::Food` (`0x004ef840`) for a straight-falling pellet, followed
    /// by `Board::AddFood`'s overrides (edible delay 0). One app-RNG draw.
    pub fn new_pellet(x: i32, y: i32, level: i32, rng: &mut MtRand) -> Self {
        let anim_speed = (rng.next_u31() & 1) as i32 + 3;
        Food {
            x: f64::from(x),
            y: f64::from(y),
            vx: -3.0,
            vy: -2.0,
            anim: 0,
            anim_speed,
            fade: 0,
            carried: false,
            level,
            motion: 0,
            kind: 0,
            delay: 0,
            widget_x: x,
            widget_y: y,
        }
    }

    /// Can a guppy eat this (`0x004d5f20`, pellet case)?
    pub fn edible(&self) -> bool {
        !self.carried && self.delay == 0 && self.kind == 0
    }

    /// `Food::Update` (`0x004f8aa0`), basic pellet.
    pub fn update(&mut self) -> FoodStep {
        if self.delay != 0 {
            self.delay -= 1;
        }
        self.anim += 1;
        if self.anim > self.anim_speed * 10 - 1 {
            self.anim = 0;
        }
        if self.fade != 0 {
            self.fade -= 1;
            return if self.fade == 0 {
                FoodStep::Remove
            } else {
                FoodStep::Keep
            };
        }
        // TODO(spec): carried, dropped-from-above and drifting variants.
        self.y += 1.5;
        if self.y > FLOOR_Y || (self.level == 3 && self.y > 400.0) {
            // TODO(spec): level-3 food explodes (RNG); potions vanish.
            if self.kind == 2 {
                return FoodStep::Remove;
            }
            if self.fade <= 0 {
                self.fade = FADE_TICKS;
            }
            self.y = FLOOR_Y;
        }
        self.widget_x = self.x as i32;
        self.widget_y = self.y as i32;
        FoodStep::Keep
    }
}
