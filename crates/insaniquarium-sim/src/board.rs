//! The tank (`Sexy::Board`). Spec: `docs/re/sim/board.md`.
//!
//! Objects live in typed, insertion-ordered vectors that mirror the
//! original's Board vectors; update order follows `Board::RebuildZOrder`.

use sexy_core::rand::MtRand;

use crate::coin::{Coin, CoinStep};
use crate::fish::{Fish, FishEnv, World};
use crate::food::{Food, FoodStep};

/// Player input, applied at the start of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Mouse-down at board coordinates.
    Click { x: i32, y: i32 },
}

/// What a click did (for UI feedback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    Ignored,
    CoinCollected,
    FoodDropped,
    TooPoor,
    FoodLimit,
}

#[derive(Debug, Clone)]
pub struct Board {
    /// `board+0x3f0`
    pub money: i32,
    /// `board+0x4ac`: price of one food drop.
    pub food_price: i32,
    /// Global `0x005df58c`: pellets allowed at once (food-quantity upgrade).
    pub max_food: i32,
    /// Global `0x005e89dc`: food level for new pellets.
    pub food_level: i32,
    /// `board+0x94`
    pub paused: bool,
    /// `board+0xa0`: guppies in insertion order.
    pub guppies: Vec<Fish>,
    /// `board+0xac`: food in insertion order.
    pub foods: Vec<Food>,
    /// `board+0xa8`: coins in insertion order.
    pub coins: Vec<Coin>,
    /// Tutorial level (`board+0x3c8 == 1 && +0x3cc == 1`): coins rest
    /// 150 ticks instead of 20.
    pub tutorial: bool,
    /// Aliens present (`board+0xb8`/`+0xf4` non-empty). Aliens aren't
    /// modelled yet; this flag lets traces with aliens be represented.
    pub has_aliens: bool,
    /// App RNG (`[app+0x7b0]`) — almost all gameplay randomness.
    pub rng: MtRand,
    /// Global RNG (`0x005eb928`).
    pub global_rng: MtRand,
    /// Logic ticks executed (`app+0x47c`).
    pub tick: u32,
    /// `board+0x28`: board update counter, the sound-throttle clock.
    pub clock: i32,
    /// `board+0x1a4`: per-sample last-played clock (sound throttling,
    /// which gates some RNG draws).
    pub sample_last: [i32; 64],
}

impl Board {
    pub fn new(rng: MtRand, global_rng: MtRand) -> Self {
        Self {
            money: 0,
            food_price: 5,
            max_food: 1,
            food_level: 0,
            paused: false,
            guppies: Vec::new(),
            foods: Vec::new(),
            coins: Vec::new(),
            tutorial: false,
            has_aliens: false,
            rng,
            global_rng,
            tick: 0,
            clock: 0,
            sample_last: [i32::MIN / 2; 64],
        }
    }

    /// `Board::AddRandomGuppy` (`0x00546d70`): x, y, then the 6 ctor draws,
    /// then two more.
    pub fn add_random_guppy(&mut self) {
        let x = self.rng.next_range(520) as i32 + 20;
        let y = self.rng.next_range(265) as i32 + 105;
        let mut f = Fish::new(x, y, &mut self.rng);
        // TODO(spec): AddRandomGuppy then sets two more fields
        // (`%5+18` and `%10+45`); the decompiler offsets are ambiguous, so
        // we only consume the draws for now (fish-guppy.md open question).
        self.rng.next_range(5);
        self.rng.next_range(10);
        f.vy_damp_timer = 0;
        self.guppies.push(f);
    }

    /// Board mouse-down, normal food-drop path. Spec: food.md#dropping-food.
    pub fn click(&mut self, x: i32, y: i32) -> ClickResult {
        // Widgets on top get the click first: coins are in front of fish
        // and food (z-order), last-added on top.
        // TODO(spec): aliens, pets, fish clicks and buttons.
        if let Some(i) = self.coins.iter().rposition(|c| c.hit(x, y)) {
            let c = &mut self.coins[i];
            if !c.collected {
                c.collect();
                let kind = c.kind;
                self.play_collect_sound(kind);
            }
            return ClickResult::CoinCollected;
        }
        if !(31..=586).contains(&x) || !(61..=399).contains(&y) {
            return ClickResult::Ignored;
        }
        if self.money < self.food_price {
            // TODO(spec): buzzer, money flash and "out of money" handling.
            return ClickResult::TooPoor;
        }
        self.money -= self.food_price;
        self.add_food(x - 10, y - 10)
    }

    /// Coin-collect sounds (`0x004e51f0`); silver/gold pick a variant with
    /// one draw when the sample isn't throttled (`0x005383d0`).
    fn play_collect_sound(&mut self, kind: i32) {
        if matches!(kind, 1 | 2 | 3 | 8 | 9 | 10) {
            let slot = &mut self.sample_last[0x12c - 0x106];
            if self.clock.wrapping_sub(*slot) >= 3 {
                *slot = self.clock;
                self.rng.next_u31();
            }
        }
    }

    /// `Board::AddFood` (`0x00543280`) for a player pellet.
    fn add_food(&mut self, x: i32, y: i32) -> ClickResult {
        let extra = self
            .foods
            .iter()
            .filter(|f| f.motion != 0 || f.kind != 0)
            .count() as i32;
        if self.foods.len() as i32 >= self.max_food + extra {
            if !self.has_aliens {
                self.money += self.food_price;
            }
            return ClickResult::FoodLimit;
        }
        let food = Food::new_pellet(x, y, self.food_level, &mut self.rng);
        self.foods.push(food);
        ClickResult::FoodDropped
    }

    /// One logic tick (28 ms) with this tick's inputs.
    pub fn step(&mut self, inputs: &[Input]) {
        for i in inputs {
            match *i {
                Input::Click { x, y } => {
                    self.click(x, y);
                }
            }
        }
        self.update();
    }

    /// One logic tick (28 ms).
    pub fn update(&mut self) {
        if self.paused {
            return;
        }
        self.clock += 1;
        // Board::Update ambient bubbles (`0x0054815e`): one draw per tick;
        // on `% 1000 == 0`, a burst of `(r & 3) + 3` bubbles, each placed with
        // two draws (`0x00538a50`: x `150 + r % 22`, y `400 + r % 6`).
        // TODO(spec): the emitter skips a bubble when it already holds ≥ 50
        // (the bubble system isn't modelled; tank 1-1 never gets close).
        if self.rng.next_range(1000) == 0 {
            let n = (self.rng.next_u31() & 3) + 3;
            for _ in 0..n {
                self.rng.next_range(22);
                self.rng.next_range(6);
            }
        }
        // Objects update in RebuildZOrder order: food, guppies, then coins.
        self.foods.retain_mut(|f| f.update() == FoodStep::Keep);
        let env = FishEnv {
            paused: self.paused,
            has_aliens: self.has_aliens,
        };
        let mut world = World {
            foods: &mut self.foods,
            coins: &mut self.coins,
            rng: &mut self.rng,
            global: &mut self.global_rng,
            clock: self.clock,
            sample_last: &mut self.sample_last,
        };
        for f in &mut self.guppies {
            f.update(env, &mut world);
        }
        // TODO(spec): DeadFish, "last fish died" mercy guppy.
        self.guppies.retain(|f| !f.dead);
        let mut banked = 0;
        let long_rest = self.tutorial;
        self.coins.retain_mut(|c| match c.update(long_rest) {
            CoinStep::Keep => true,
            CoinStep::Remove => false,
            CoinStep::Bank(v) => {
                banked += v;
                false
            }
        });
        // `0x0053c1e0`: add money, capped at 9,999,999.
        self.money = (self.money + banked).min(9_999_999);
        self.tick += 1;
    }
}
