//! The tank (`Sexy::Board`). Spec: `docs/re/sim/board.md`.
//!
//! Objects live in typed, insertion-ordered vectors that mirror the
//! original's Board vectors; update order follows `Board::RebuildZOrder`.

use sexy_core::rand::MtRand;

use crate::coin::{Coin, CoinStep};
use crate::fish::{Fish, FishEnv, World};
use crate::food::{Food, FoodStep};
use crate::shop::{EGG_DONE, Shop, Slot};
use crate::sound::Sound;

/// Player input, applied at the start of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Mouse-down at board coordinates.
    Click { x: i32, y: i32 },
    /// Dismiss the current modal dialog (its default button).
    DismissDialog,
    /// Press a shop button directly (same as clicking it).
    Buy(Slot),
}

/// Modal dialogs the board can raise. While one is up the board is paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// Dialog 0x10: "Your last fish has died!" (tutorial level only).
    /// Continuing adds a random guppy.
    LastFishDied,
    /// Dialog 0x11: game over.
    GameOver,
    /// Dialog 0x13 "OUT OF MONEY!" (tutorial, money 0).
    /// TODO(spec): the loan it offers.
    OutOfMoney,
}

/// What a click did (for UI feedback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    Ignored,
    Bought(Slot),
    CantAfford,
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
    /// `board+0x3c8`, `+0x3cc`
    pub tank: i32,
    pub level: i32,
    pub shop: Shop,
    /// `board+0x43c`: egg pieces + 1; the level completes at 4.
    pub egg_count: i32,
    /// Set when the last egg piece is bought (hatch screen next).
    pub level_complete: bool,
    /// `board+0x450`: money-flash countdown after a failed purchase.
    pub money_flash: i32,
    /// `board+0x444`: running-tick counter (animates button icons).
    pub level_ticks: i32,
    /// Tutorial help arrows: `+0x4b2` (buy fish), `+0x4d8` (buy egg).
    pub arrow_buy_fish: bool,
    pub arrow_buy_egg: bool,
    /// The modal dialog raised by the board, if any.
    pub dialog: Option<Dialog>,
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
    /// Sounds started during the last `step` (presentation only; cleared
    /// at the start of each step).
    pub sounds: Vec<Sound>,
}

impl Board {
    pub fn new(rng: MtRand, global_rng: MtRand) -> Self {
        Self {
            money: 0,
            food_price: 5,
            max_food: 1,
            food_level: 0,
            paused: false,
            tank: 1,
            level: 1,
            shop: Shop::adventure(1, 1),
            egg_count: 1,
            level_complete: false,
            money_flash: 0,
            level_ticks: 0,
            arrow_buy_fish: false,
            arrow_buy_egg: false,
            dialog: None,
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
            sounds: Vec::new(),
        }
    }

    /// `Board::AddRandomGuppy` (`0x00546d70`): x, y, then the 6 ctor draws,
    /// then two more.
    pub fn add_random_guppy(&mut self) {
        let x = self.rng.next_range(520) as i32 + 20;
        let y = self.rng.next_range(265) as i32 + 105;
        let mut f = Fish::new(x, y, &mut self.rng);
        // The new guppy splashes in from the top: it starts at y = 30 moving
        // down fast, and the vy damping (×0.9 per tick) slows it over
        // 45..54 ticks, emitting bubbles while vy > 0 (fish-guppy.md#spawn).
        // `y` from the first draw only matters to the constructor.
        f.vy = f64::from(self.rng.next_range(5) + 18);
        f.y = 30.0;
        f.widget_y = 30;
        f.vy_damp_timer = self.rng.next_range(10) as i32 + 45;
        f.stage = 0;
        self.guppies.push(f);
        // `0x00544800` gives it a floor shadow; `Shadow::Shadow` (`0x004ec4f0`)
        // draws its y as `410 + r % 5`. Shadows are cosmetic otherwise.
        // TODO(spec): Shadow::Update, if it ever draws.
        self.rng.next_range(5);
    }

    /// Board mouse-down, normal food-drop path. Spec: food.md#dropping-food.
    pub fn click(&mut self, x: i32, y: i32) -> ClickResult {
        // Shop buttons are top-level widgets: they take the click first and
        // purchase on mouse-down (shop-bar.md#input-path).
        if let Some(slot) = self.shop.hit(x, y) {
            return self.purchase(slot);
        }
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
        if matches!(kind, 1 | 2 | 3 | 8 | 9 | 10) && self.sample_ready(0x12c, 3) {
            let r = self.rng.next_u31() & 3;
            self.sounds.push(Sound::points(r));
        }
    }

    /// `0x005381f0`: may sample `id` play now (≥ `gap` ticks since last)?
    /// Records the play when it may.
    fn sample_ready(&mut self, id: usize, gap: i32) -> bool {
        let slot = &mut self.sample_last[id - 0x106];
        if self.clock.wrapping_sub(*slot) >= gap {
            *slot = self.clock;
            true
        } else {
            false
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
        if self.sample_ready(0x118, 3) {
            self.sounds.push(Sound::DropFood);
        }
        ClickResult::FoodDropped
    }

    /// One logic tick (28 ms) with this tick's inputs.
    pub fn step(&mut self, inputs: &[Input]) {
        self.sounds.clear();
        for i in inputs {
            match *i {
                Input::Click { x, y } if self.dialog.is_none() => {
                    self.click(x, y);
                }
                Input::Click { .. } => {}
                Input::DismissDialog => self.dismiss_dialog(),
                Input::Buy(slot) if self.dialog.is_none() => {
                    self.purchase(slot);
                }
                Input::Buy(_) => {}
            }
        }
        self.update();
    }

    /// Value of collected coins still flying to the counter
    /// (`PendingCollectValue` @ `0x0053a0b0`).
    pub fn pending_coin_value(&self) -> i32 {
        self.coins
            .iter()
            .filter(|c| c.collected)
            .map(|c| c.value() * c.multiplier)
            .sum()
    }

    /// `Board::TrySpend(price, buzz = 1)` (`0x00540b30`). Money may go
    /// negative when coins in flight cover the gap.
    fn try_spend(&mut self, price: i32) -> bool {
        if self.money < price && self.money + self.pending_coin_value() < price {
            if self.sample_ready(0x112, 3) {
                self.sounds.push(Sound::Buzzer);
            }
            self.money_flash = 70;
            if self.money == 0 && self.tutorial && self.dialog.is_none() {
                self.paused = true;
                self.dialog = Some(Dialog::OutOfMoney);
            }
            return false;
        }
        self.money -= price;
        true
    }

    /// `Board::Purchase(slot)` (`0x00549380`), the slots tank 1 uses.
    pub fn purchase(&mut self, slot: Slot) -> ClickResult {
        let i = slot as usize;
        if !self.shop.revealed[i] {
            return ClickResult::Ignored;
        }
        let price = self.shop.price[i];
        let ok = match slot {
            Slot::Guppy => self.buy_guppy(price),
            Slot::EggPiece => self.buy_egg_piece(price),
            // TODO(spec): other slots (not on tank 1-1).
            _ => return ClickResult::Ignored,
        };
        if ok {
            ClickResult::Bought(slot)
        } else {
            ClickResult::CantAfford
        }
    }

    /// Slot 0: spend, then `AddRandomGuppy` (12 draws with the splash).
    fn buy_guppy(&mut self, price: i32) -> bool {
        if !self.try_spend(price) {
            return false;
        }
        self.add_random_guppy();
        self.arrow_buy_fish = false;
        // Purchase sounds: a throttled "buy" sample, then a splash variant
        // picked with one unthrottled draw (`0x00538360`: `% 3`).
        if self.sample_ready(0x111, 3) {
            self.sounds.push(Sound::Buy);
        }
        let r = self.rng.next_range(3);
        self.sounds.push(Sound::splash(r));
        true
    }

    /// Slot 11: `Board::BuyEggPiece` (`0x00545960`). No RNG.
    fn buy_egg_piece(&mut self, price: i32) -> bool {
        if !self.try_spend(price) {
            return false;
        }
        self.arrow_buy_egg = false;
        self.egg_count += 1;
        if self.sample_ready(0x111, 3) {
            self.sounds.push(Sound::Buy);
        }
        if self.egg_count >= EGG_DONE {
            // SettlePendingCoins (`0x0053c2d0`), then the hatch screen.
            self.money = (self.money + self.pending_coin_value()).clamp(0, 9_999_999);
            self.level_complete = true;
            self.paused = true;
        }
        true
    }

    /// `Fish::OnStageUp` (`0x004f1840`) shop reveals for tank 1-1.
    fn on_stage_up(&mut self, stage: i32) {
        let first_level = self.tank == 1 && self.level == 1;
        match stage {
            1 if !self.shop.revealed[Slot::Guppy as usize] => {
                self.shop.reveal(Slot::Guppy, true);
                if self.tutorial {
                    self.arrow_buy_fish = true;
                }
            }
            2 if first_level && !self.shop.revealed[Slot::EggPiece as usize] => {
                self.shop.reveal(Slot::EggPiece, true);
                self.arrow_buy_fish = false;
                self.arrow_buy_egg = true;
            }
            // TODO(spec): other levels' reveal lists.
            _ => {}
        }
    }

    /// `WinFishApp` dialog-button handler (vtable slot 3 @ `0x00552a8a`).
    fn dismiss_dialog(&mut self) {
        match self.dialog.take() {
            Some(Dialog::LastFishDied) => {
                self.paused = false;
                self.add_random_guppy();
            }
            Some(Dialog::OutOfMoney) => self.paused = false,
            Some(Dialog::GameOver) | None => {}
        }
    }

    /// One logic tick (28 ms).
    pub fn update(&mut self) {
        // `tick` mirrors the app's update counter, which runs while the
        // board is paused; the board clock (`+0x28`) does not.
        if self.paused {
            self.tick += 1;
            return;
        }
        self.clock += 1;
        self.level_ticks += 1;
        if self.money_flash > 0 {
            self.money_flash -= 1;
        }
        self.shop.tick();
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
        let mut stage_ups = Vec::new();
        let mut world = World {
            foods: &mut self.foods,
            coins: &mut self.coins,
            rng: &mut self.rng,
            global: &mut self.global_rng,
            clock: self.clock,
            sample_last: &mut self.sample_last,
            sounds: &mut self.sounds,
            stage_ups: &mut stage_ups,
        };
        for f in &mut self.guppies {
            f.update(env, &mut world);
        }
        for s in stage_ups {
            self.on_stage_up(s);
        }
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
        // Board::Update (`0x00548e29`): no fish left → pause and raise the
        // tutorial's second-chance dialog, or game over. (Coins still on the
        // board are banked first in the original via `0x0053c2d0`; that only
        // matters once coin values on-screen are tracked separately.)
        // TODO(spec): other fish kinds and pets count as fish (`0x005392a0`).
        if self.guppies.is_empty() && self.dialog.is_none() {
            self.paused = true;
            self.dialog = Some(if self.tutorial {
                Dialog::LastFishDied
            } else {
                Dialog::GameOver
            });
        }
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tank() -> Board {
        let mut b = Board::new(MtRand::new(1), MtRand::new(2));
        b.money = 200;
        b.tutorial = true;
        b.add_random_guppy();
        b
    }

    #[test]
    fn starving_guppy_dies_and_tutorial_offers_another() {
        let mut b = tank();
        let mut ticks = 0;
        while b.dialog.is_none() {
            b.step(&[]);
            ticks += 1;
            assert!(ticks < 2000, "guppy should starve within ~1100 ticks");
        }
        // Hunger starts at 400..599 and death is below −499.
        assert!((900..1100).contains(&ticks), "{ticks}");
        assert_eq!(b.dialog, Some(Dialog::LastFishDied));
        assert!(b.paused && b.guppies.is_empty());
        // Paused: clicks do nothing, ticks don't advance the board.
        let before = b.clock;
        b.step(&[Input::Click { x: 300, y: 200 }]);
        assert_eq!((b.clock, b.money, b.foods.len()), (before, 200, 0));
        b.step(&[Input::DismissDialog]);
        assert!(b.dialog.is_none() && !b.paused);
        assert_eq!(b.guppies.len(), 1);
    }

    /// Feed every guppy-ish 150 ticks and collect every coin; returns
    /// once `until` holds or panics after `limit` ticks.
    fn play(b: &mut Board, limit: u32, until: impl Fn(&Board) -> bool) {
        for t in 0..limit {
            if until(b) {
                return;
            }
            let mut inputs = Vec::new();
            if let Some(c) = b.coins.iter().find(|c| !c.collected) {
                inputs.push(Input::Click {
                    x: c.widget_x + 36,
                    y: c.widget_y + 36,
                });
            } else if t % 150 == 0 {
                let f = &b.guppies[0];
                inputs.push(Input::Click {
                    x: (f.widget_x + 40).clamp(31, 586),
                    y: (f.widget_y - 10).clamp(70, 399),
                });
            }
            b.step(&inputs);
            assert!(b.dialog.is_none(), "dialog at tick {t}: {:?}", b.dialog);
        }
        panic!("condition not reached in {limit} ticks");
    }

    #[test]
    fn tank_1_1_can_be_completed() {
        let mut b = tank();
        assert!(!b.shop.revealed.iter().any(|&r| r), "no buttons at start");
        play(&mut b, 20_000, |b| b.shop.revealed[Slot::Guppy as usize]);
        assert!(b.arrow_buy_fish);
        // The guppy button is at bar position 0: (18..76, 3..63).
        play(&mut b, 20_000, |b| b.money >= 100);
        let n = b.guppies.len();
        b.step(&[Input::Click { x: 40, y: 30 }]);
        assert_eq!(b.guppies.len(), n + 1, "bought a guppy");
        assert!(!b.arrow_buy_fish);
        play(&mut b, 40_000, |b| b.shop.revealed[Slot::EggPiece as usize]);
        for piece in 0..3 {
            play(&mut b, 40_000, |b| b.money >= 150);
            b.step(&[Input::Click { x: 460, y: 30 }]);
            assert_eq!(b.egg_count, 2 + piece);
        }
        assert!(b.level_complete && b.paused);
    }

    #[test]
    fn failed_purchase_flashes_money() {
        let mut b = tank();
        b.shop.reveal(Slot::Guppy, false);
        b.money = 50;
        assert_eq!(b.purchase(Slot::Guppy), ClickResult::CantAfford);
        assert_eq!((b.money_flash, b.money, b.guppies.len()), (70, 50, 1));
        assert_eq!(b.sounds, vec![Sound::Buzzer]);
        // Unrevealed slots can't be bought at all.
        assert_eq!(b.purchase(Slot::EggPiece), ClickResult::Ignored);
    }

    #[test]
    fn feeding_keeps_a_guppy_alive_and_grows_it() {
        let mut b = tank();
        for t in 0..6000 {
            let mut inputs = Vec::new();
            if t % 150 == 0 {
                let f = &b.guppies[0];
                inputs.push(Input::Click {
                    x: (f.widget_x + 40).clamp(31, 586),
                    y: (f.widget_y - 10).clamp(61, 399),
                });
            }
            b.step(&inputs);
            assert!(b.dialog.is_none(), "starved at tick {t}");
        }
        assert!(b.guppies[0].stage >= 1);
    }
}
