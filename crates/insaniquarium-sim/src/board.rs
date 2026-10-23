//! The tank (`Sexy::Board`). Spec: `docs/re/sim/board.md`.
//!
//! Objects live in typed, insertion-ordered vectors that mirror the
//! original's Board vectors; update order follows `Board::RebuildZOrder`.

use sexy_core::rand::MtRand;

use crate::alien::{Alien, AlienOut, DeadAlien, Shot, Warp};
use crate::coin::{Coin, CoinStep};
use crate::fish::{Fish, FishEnv, World};
use crate::food::{Food, FoodStep};
use crate::pet::{Pet, PetOut, STINKY};
use crate::shop::{EGG_DONE, Shop, Slot};
use crate::sound::{Music, Played, Sound};

/// Player input, applied at the start of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Mouse-down at board coordinates.
    Click { x: i32, y: i32 },
    /// Dismiss the current modal dialog (its default button).
    DismissDialog,
    /// Press a shop button directly (same as clicking it).
    Buy(Slot),
    /// Pause / unpause the board (focus loss or the menu; no dialog).
    SetPaused(bool),
}

/// Modal dialogs the board can raise. While one is up the board is paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// Dialog 0x10: "Your last fish has died!" (tutorial level only).
    /// Continuing adds a random guppy.
    LastFishDied,
    /// Dialog 0x11: game over.
    GameOver,
    /// Dialog 0x0e "DANGER! A vicious alien is about to enter your tank!"
    /// (tank 1-2, first wave, countdown 276).
    AlienWarning,
    /// Dialog 0x0e "BATTLE TIP" (tank 1-2, second wave).
    BattleTip,
    /// Dialog 0x0e "WARNING! A new breed of alien…" (tank 1-3).
    NewBreed,
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

/// Behaviour switches. Defaults are the engine's: original gameplay bugs
/// fixed. `classic` restores every original behaviour that has a fix (and is
/// what trace verification runs with). See CLAUDE.md, "Bugs".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rules {
    pub classic: bool,
}

impl Rules {
    /// Play exactly like the original (bugs included).
    pub const CLASSIC: Rules = Rules { classic: true };
}

#[derive(Debug, Clone)]
pub struct Board {
    pub rules: Rules,
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
    /// `+0x4bf`: "upgrade food quality" hint on 1-2.
    pub arrow_food_quality: bool,
    /// The modal dialog raised by the board, if any.
    pub dialog: Option<Dialog>,
    /// Board hint bytes `+0x4b3` (DANGER), `+0x4b4` (battle tip), `+0x4c1`
    /// (new breed): each is shown once per level.
    pub hints_shown: [bool; 3],
    /// `board+0xa0`: guppies in insertion order.
    pub guppies: Vec<Fish>,
    /// `board+0xac`: food in insertion order.
    pub foods: Vec<Food>,
    /// `board+0xa8`: coins in insertion order.
    pub coins: Vec<Coin>,
    /// `board+0xb0`: `OtherTypePet` pets (Stinky) in insertion order.
    pub pets: Vec<Pet>,
    /// Tutorial level (`board+0x3c8 == 1 && +0x3cc == 1`): coins rest
    /// 150 ticks instead of 20.
    pub tutorial: bool,
    /// Aliens present (`board+0xb8`/`+0xf4` non-empty); kept in sync with
    /// `aliens`.
    pub has_aliens: bool,
    /// `board+0xb8`: aliens.
    pub aliens: Vec<Alien>,
    /// `board+0xd8`: shots (click flashes, effect sparks).
    pub shots: Vec<Shot>,
    /// `board+0x9c`: dead aliens.
    pub dead_aliens: Vec<DeadAlien>,
    /// Alien waves (`alien-sylvester.md#1`): `+0x2bc` kind, `+0x2c0`
    /// countdown, `+0x2c4`, `+0x2c8` food lockout, spawn positions, count.
    pub wave_kind: i32,
    pub wave_countdown: i32,
    pub wave_2c4: i32,
    pub food_lockout: i32,
    pub wave_a: (i32, i32),
    pub wave_b: (i32, i32),
    pub waves: i32,
    /// `+0x3e4`: laser level (damage 3 × laser per hit).
    pub laser: i32,
    /// `+0x3d0`, `+0x3d4`: last shot click position.
    pub last_shot: (i32, i32),
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
    pub sounds: Vec<Played>,
    /// Music change requested during the last `step` (presentation only).
    pub music: Option<Music>,
    /// `board+0xf0`: warp-in holes (cosmetic, no RNG).
    pub warps: Vec<Warp>,
}

impl Board {
    pub fn new(rng: MtRand, global_rng: MtRand) -> Self {
        Self {
            rules: Rules::default(),
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
            arrow_food_quality: false,
            dialog: None,
            hints_shown: [false; 3],
            guppies: Vec::new(),
            foods: Vec::new(),
            coins: Vec::new(),
            pets: Vec::new(),
            tutorial: false,
            has_aliens: false,
            aliens: Vec::new(),
            shots: Vec::new(),
            dead_aliens: Vec::new(),
            wave_kind: 0,
            wave_countdown: 3000,
            wave_2c4: 0,
            food_lockout: 0,
            wave_a: (0, 0),
            wave_b: (0, 0),
            waves: 0,
            laser: 2,
            last_shot: (0, 0),
            rng,
            global_rng,
            tick: 0,
            clock: 0,
            sample_last: [i32::MIN / 2; 64],
            sounds: Vec::new(),
            music: None,
            warps: Vec::new(),
        }
    }

    /// `Board::AddRandomGuppy` (`0x00546d70`): x, y, then the 6 ctor draws,
    /// then two more.
    pub fn add_random_guppy(&mut self) {
        let x = self.rng.next_range(520) as i32 + 20;
        let y = self.rng.next_range(265) as i32 + 105;
        let mut f = Fish::new(x, y, &mut self.rng);
        // Fish ctor: `+0x228` = tutorial level (`0x00537b80`).
        f.announces = self.tutorial;
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

    /// A board at the start of an Adventure level: `Board::InitLevel` then
    /// `Board::PopulateLevel` (`sim/level-start.md`). `pets` are the owned
    /// pet ids, added in id order. TODO(spec): pets other than none.
    pub fn start_level(
        rng: MtRand,
        global_rng: MtRand,
        tank: i32,
        level: i32,
        pets: &[i32],
    ) -> Self {
        let mut b = Board::new(rng, global_rng);
        b.tank = tank;
        b.level = level;
        b.money = 200;
        b.shop = Shop::adventure(tank, level);
        b.tutorial = tank == 1 && level == 1;
        // SetupShop's alien table, tank 1 (`alien-sylvester.md#level-setup`).
        if tank == 1 {
            (b.wave_kind, b.wave_countdown) = match level {
                1 => (0, 3000),
                2 => (1, 1750),
                3 => (2, 3000),
                _ => (3, 3000),
            };
        }
        // `Board::Board` (`0x00542150`): three background x positions,
        // `r % 640` each (cosmetic).
        for _ in 0..3 {
            b.rng.next_range(640);
        }
        // Start bubble burst: (r & 3) + 3 bubbles, two draws each.
        let n = (b.rng.next_u31() & 3) + 3;
        for _ in 0..n {
            b.rng.next_range(22);
            b.rng.next_range(6);
        }
        // Owned pets in id order (`AddPet`), before the fish.
        for &id in pets {
            b.add_pet(id);
        }
        for _ in 0..2 {
            let x = b.rng.next_range(520) as i32 + 20;
            let y = b.rng.next_range(265) as i32 + 105;
            b.add_guppy_at(x, y);
        }
        for g in &mut b.guppies {
            g.growth = 2;
        }
        b
    }

    /// `Board::AddPet(id, -1, -1)` (`0x00544a90`): 2 position draws, the
    /// pet ctor, then the shadow draw. TODO(spec): pets other than Stinky.
    pub fn add_pet(&mut self, id: i32) {
        let a = self.rng.next_range(265) as i32 + 105;
        let _b = self.rng.next_range(520) as i32 + 20;
        // The pet's Shadow ctor draws its floor y (`board.md#shadows`); a
        // type-1 shadow (Stinky) keeps it.
        if id == STINKY {
            let mut p = Pet::stinky(a, _b, &mut self.rng);
            p.shadow_y = self.rng.next_range(5) as i32 + 410;
            self.pets.push(p);
        } else {
            self.rng.next_range(5);
        }
    }

    /// `Board::AddGuppyAt` (`0x00546ef0`): constructed in place, plus shadow.
    pub fn add_guppy_at(&mut self, x: i32, y: i32) {
        let mut f = Fish::new(x, y, &mut self.rng);
        f.announces = self.tutorial;
        self.guppies.push(f);
        self.rng.next_range(5);
    }

    /// Board mouse-down, normal food-drop path. Spec: food.md#dropping-food.
    pub fn click(&mut self, x: i32, y: i32) -> ClickResult {
        // Food lockout after a kill only covers clicks within 50 px of the
        // last shot; a click elsewhere ends it (`0x00543972`).
        if self.food_lockout > 0 {
            let (dx, dy) = (self.last_shot.0 - x, self.last_shot.1 - y);
            if dx * dx + dy * dy > 2500 {
                self.food_lockout = 0;
            }
        }
        // Shop buttons are top-level widgets: they take the click first and
        // purchase on mouse-down (shop-bar.md#input-path).
        if let Some(slot) = self.shop.hit(x, y) {
            return self.purchase(slot);
        }
        // With aliens present, clicks are shots (`alien-sylvester.md#41`).
        // TODO(spec): coins inside an alien's rect re-dispatch here too.
        if !self.aliens.is_empty() {
            return self.shoot(x, y);
        }
        // Widgets on top get the click first: coins are in front of fish
        // and food (z-order), last-added on top.
        // TODO(spec): pets, fish clicks.
        if let Some(i) = self.coins.iter().rposition(|c| c.hit(x, y)) {
            let c = &mut self.coins[i];
            if !c.collected {
                c.collect();
                let kind = c.kind;
                self.play_collect_sound(kind);
            }
            return ClickResult::CoinCollected;
        }
        if !(31..=586).contains(&x) || !(61..=399).contains(&y) || self.food_lockout > 0 {
            return ClickResult::Ignored;
        }
        if self.money < self.food_price {
            // TODO(spec): buzzer, money flash and "out of money" handling.
            return ClickResult::TooPoor;
        }
        self.money -= self.food_price;
        self.add_food(x - 10, y - 10)
    }

    /// Board mouse-down with aliens present.
    fn shoot(&mut self, x: i32, y: i32) -> ClickResult {
        if y > 40 {
            for i in 0..self.aliens.len() {
                if let Some(killed) = self.aliens[i].hit(x, y, self.laser) {
                    // `EffectShot(x − 40, y − 40, 2)` and the hit sound.
                    self.add_effect_shot(x - 40, y - 40, 2);
                    self.play(0x126, Sound::Hit);
                    if killed {
                        self.kill_alien(i);
                        break;
                    }
                }
            }
        }
        // Click flash `Shot` (no RNG), then the shot sound (2 draws).
        self.last_shot = (x, y);
        self.shots.push(Shot {
            kind: 0,
            delay: 0,
            age: 0,
            frame: 0,
            r: 255,
            widget_x: x - 40,
            widget_y: y - 40,
        });
        // `0x00538ae0`: zap, maybe zzam, maybe a ricochet.
        if self.rng.next_u31() & 1 != 0 {
            self.play(0x144, Sound::Zap);
            self.play(0x145, Sound::Zzam);
        } else {
            self.play(0x144, Sound::Zap);
        }
        if self.rng.next_range(10) == 0 {
            self.play(0x132, Sound::Ricochet);
        }
        ClickResult::Ignored
    }

    /// `EffectShot` (`0x005436f0`): only while fewer than 20 shots.
    ///
    /// Deviation (implementation artifact): the original's Shot constructor
    /// reads its kind field before setting it, and draws an extra `% 3` start
    /// delay when the recycled heap block happened to hold kind 1 or 3+.
    /// The intent is no delay (the field is set right after), so we never
    /// draw it (`alien-sylvester.md#deviations`).
    fn add_effect_shot(&mut self, x: i32, y: i32, kind: i32) {
        if self.shots.len() >= 20 {
            return;
        }
        let r = self.rng.next_range(200) as i32 + 50;
        self.shots.push(Shot {
            kind,
            delay: 0,
            age: 0,
            frame: 0,
            r,
            widget_x: x,
            widget_y: y,
        });
    }

    /// `Alien` death (`0x004f9c70`), kind 1.
    fn kill_alien(&mut self, i: usize) {
        let a = self.aliens.remove(i);
        let (ix, iy) = (a.widget_x, a.widget_y);
        // Diamond drop.
        let mut c = Coin::new(ix + 25, iy + 25, crate::coin::kind::DIAMOND);
        c.vy = -1.0;
        self.coins.push(c);
        if self.aliens.is_empty() {
            self.food_lockout = 36;
            self.has_aliens = false;
            self.music = Some(Music::Tank);
        }
        // Debris (§4.3 step 5). Bubbles draw the global stream (not modelled).
        let n = self.rng.next_u31() & 1;
        for _ in 0..n {
            self.rng.next_range(70);
            self.rng.next_range(70);
            self.rng.next_range(50);
            self.rng.next_range(50);
        }
        let n = self.rng.next_range(3);
        for _ in 0..n {
            let k = self.rng.next_range(3) as i32 + 3;
            let y = self.rng.next_range(50) as i32 + 20 + iy;
            let x = self.rng.next_range(50) as i32 + 20 + ix;
            self.add_effect_shot(x, y, k);
            let k = self.rng.next_range(3) as i32 + 3;
            let y = self.rng.next_range(30) as i32 + 40 + iy;
            let x = self.rng.next_range(30) as i32 + 40 + ix;
            self.add_effect_shot(x, y, k);
        }
        let n = self.rng.next_u31() & 1;
        for _ in 0..n {
            let k = (self.rng.next_u31() & 1) as i32 + 6;
            let y = self.rng.next_range(50) as i32 + 20 + iy;
            let x = self.rng.next_range(50) as i32 + 20 + ix;
            self.add_effect_shot(x, y, k);
            let k = (self.rng.next_u31() & 1) as i32 + 6;
            let y = self.rng.next_range(30) as i32 + 40 + iy;
            let x = self.rng.next_range(30) as i32 + 40 + ix;
            self.add_effect_shot(x, y, k);
        }
        // Death sounds (`0x004fa01e`, `0x004fa03e`).
        self.play(0x11d, Sound::Explode);
        self.play(0x11e, Sound::Explosion1);
        // AddDeadAlien gets trunc(x, y) (`0x004fa6ae`): the stroke animation
        // can move the alien after its widget position was set.
        self.dead_aliens.push(DeadAlien::new(
            a.x as i32,
            a.y as i32,
            a.cel,
            a.kind,
            a.vx > 0.0,
        ));
    }

    /// `DeadAlien::Update` (`0x004f6050`): debris bursts every 7 ticks.
    fn update_dead_aliens(&mut self) {
        let mut i = 0;
        while i < self.dead_aliens.len() {
            let d = &mut self.dead_aliens[i];
            if d.life < 1 {
                self.dead_aliens.remove(i);
                continue;
            }
            d.life -= 1;
            let (life, ix, iy) = (d.life, d.widget_x, d.widget_y);
            // Rumble every 5th tick above 50, from the freshest corpse only
            // (`0x004e54f0`), louder while fresh. No RNG.
            if life % 5 == 0
                && life > 50
                && self.dead_aliens.iter().all(|o| o.life <= life)
                && self.sample_ready(0x11d, 3)
            {
                let t = life - 50;
                let v = if t < 75 {
                    ((75 - t) * 25 + 100 * t) / 75
                } else {
                    100
                };
                self.sounds.push(Played {
                    sound: Sound::Explode,
                    volume: f64::from(v.clamp(25, 100)) / 100.0,
                });
            }
            if life % 7 == 0 && life > 50 {
                // Two bubbles (positions only; bubbles draw global RNG).
                self.rng.next_range(70);
                self.rng.next_range(70);
                self.rng.next_range(50);
                self.rng.next_range(50);
                for (k6, wide) in [(false, true), (false, false), (true, true), (true, false)] {
                    let k = if k6 {
                        (self.rng.next_u31() & 1) as i32 + 6
                    } else {
                        self.rng.next_range(3) as i32 + 3
                    };
                    let (m, o) = if wide { (50, 20) } else { (30, 40) };
                    let y = self.rng.next_range(m) as i32 + o + iy;
                    let x = self.rng.next_range(m) as i32 + o + ix;
                    self.add_effect_shot(x, y, k);
                }
            }
            self.dead_aliens[i].drift();
            i += 1;
        }
    }

    /// Wave countdown (`Board::Update` `0x005481c4`..`0x00548e05`), tank 1.
    fn update_waves(&mut self) {
        if self.wave_2c4 != 0 {
            self.wave_2c4 -= 1;
        }
        if !self.aliens.is_empty() || self.wave_kind == 0 {
            return;
        }
        self.wave_countdown -= 1;
        match self.wave_countdown {
            // One-time hints (`0x005483e0`): each pauses the board until
            // dismissed, then is marked shown (`+0x4b3`, `+0x4b4`, `+0x4c1`).
            // TODO(spec): the virtual-tank/mode checks on this path.
            276 => {
                let hint = match (self.tank, self.level) {
                    (1, 2) if !self.hints_shown[0] => Some((0, Dialog::AlienWarning)),
                    (1, 2) if !self.hints_shown[1] => Some((1, Dialog::BattleTip)),
                    (1, 3) if !self.hints_shown[2] => Some((2, Dialog::NewBreed)),
                    _ => None,
                };
                if let Some((i, d)) = hint {
                    self.hints_shown[i] = true;
                    self.paused = true;
                    self.dialog = Some(d);
                }
            }
            275 => {
                self.music = Some(Music::Alien);
                self.play(0x106, Sound::Awooga);
                self.wave_a = (
                    self.rng.next_range(450) as i32 + 20,
                    self.rng.next_range(195) as i32 + 105,
                );
                self.wave_b = (
                    self.rng.next_range(450) as i32 + 20,
                    self.rng.next_range(195) as i32 + 105,
                );
            }
            c if c <= 0 => {
                self.wave_2c4 = 35;
                let a = Alien::new(self.wave_a.0, self.wave_a.1, self.wave_kind, &mut self.rng);
                self.warps.push(Warp::for_alien(&a));
                self.aliens.push(a);
                // Its shadow.
                self.rng.next_range(5);
                self.has_aliens = true;
                // Arrival roar (kinds 1–3), played directly.
                self.sounds.push(Sound::Roar.into());
                if self.tank == 1 && self.level == 5 {
                    self.wave_kind = if self.rng.next_u31() & 1 != 0 { 3 } else { 9 };
                }
                self.waves += 1;
                self.wave_countdown = 3000;
            }
            _ => {}
        }
    }

    /// Coin-collect sounds (`0x004e51f0`); silver/gold pick a variant with
    /// one draw when the sample isn't throttled (`0x005383d0`).
    /// Diamonds and pearls play a fixed sample (`0x00538470`, no RNG).
    /// TODO(spec): treasure chests and the other kinds' samples.
    fn play_collect_sound(&mut self, kind: i32) {
        if matches!(kind, 1 | 2 | 3 | 8 | 9 | 10) && self.sample_ready(0x12c, 3) {
            let r = self.rng.next_u31() & 3;
            self.sounds.push(Sound::points(r).into());
        } else if matches!(kind, 4 | 5 | 11 | 12) {
            self.play(0x116, Sound::Diamond);
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

    /// `0x00538230(id, 3, 1.0)`: play a throttled sample (no RNG).
    fn play(&mut self, id: usize, sound: Sound) {
        if self.sample_ready(id, 3) {
            self.sounds.push(sound.into());
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
            self.sounds.push(Sound::DropFood.into());
        }
        ClickResult::FoodDropped
    }

    /// One logic tick (28 ms) with this tick's inputs.
    pub fn step(&mut self, inputs: &[Input]) {
        self.sounds.clear();
        self.music = None;
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
                Input::SetPaused(p) => {
                    if self.dialog.is_none() {
                        self.paused = p;
                    }
                }
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
                self.sounds.push(Sound::Buzzer.into());
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
            Slot::FoodQuality => self.buy_food_quality(price),
            Slot::FoodQuantity => self.buy_food_quantity(price),
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
            self.sounds.push(Sound::Buy.into());
        }
        let r = self.rng.next_range(3);
        self.sounds.push(Sound::splash(r).into());
        true
    }

    /// Slot 2: food quality +1 (cap 2). On 1-2 it reveals quantity and
    /// the egg (`shop-bar.md#per-slot-behaviour`). No RNG.
    fn buy_food_quality(&mut self, price: i32) -> bool {
        if self.food_level >= 2 || !self.try_spend(price) {
            return false;
        }
        self.arrow_food_quality = false;
        self.food_level += 1;
        if self.tank == 1 && self.level == 2 && !self.shop.revealed[Slot::EggPiece as usize] {
            self.shop.reveal(Slot::FoodQuantity, true);
            self.shop.reveal(Slot::EggPiece, true);
        }
        if self.sample_ready(0x111, 3) {
            self.sounds.push(Sound::Buy.into());
        }
        true
    }

    /// Slot 3: food quantity +1 (cap 9). No RNG.
    fn buy_food_quantity(&mut self, price: i32) -> bool {
        if self.max_food >= 9 || !self.try_spend(price) {
            return false;
        }
        self.max_food += 1;
        if self.sample_ready(0x111, 3) {
            self.sounds.push(Sound::Buy.into());
        }
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
            self.sounds.push(Sound::Buy.into());
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
            2 if self.tank == 1 && self.level == 2 => {
                if !self.shop.revealed[Slot::FoodQuality as usize] {
                    self.arrow_food_quality = true;
                    self.shop.reveal(Slot::FoodQuality, true);
                }
            }
            2 if self.tank == 1 && self.level >= 3 => {
                self.shop.reveal(Slot::FoodQuality, true);
                self.shop.reveal(Slot::FoodQuantity, true);
                self.shop.reveal(Slot::Carnivore, true);
            }
            // TODO(spec): tanks 2–4 reveal lists (`0x004f1840`).
            _ => {}
        }
    }

    /// `WinFishApp` dialog-button handler (vtable slot 3 @ `0x00552a8a`).
    fn dismiss_dialog(&mut self) {
        // Game over has no "continue": the board stays paused under it.
        if self.dialog == Some(Dialog::GameOver) {
            return;
        }
        match self.dialog.take() {
            Some(Dialog::LastFishDied) => {
                self.paused = false;
                self.add_random_guppy();
            }
            Some(
                Dialog::OutOfMoney | Dialog::AlienWarning | Dialog::BattleTip | Dialog::NewBreed,
            ) => self.paused = false,
            Some(Dialog::GameOver) | None => {}
        }
    }

    /// One logic tick (28 ms).
    pub fn update(&mut self) {
        // `tick` mirrors the app's update counter, which runs while the
        // board is paused; the board clock (`+0x28`) does not.
        // The board clock `+0x28` (widget update count) runs while paused.
        self.clock += 1;
        if self.paused {
            self.tick += 1;
            return;
        }
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
        // After the ambient check: `+0x2c4`, then the wave countdown.
        self.update_waves();
        // A dialog raised by Board::Update pauses the board before the
        // objects run this tick (they're separate widgets updated after).
        if self.paused {
            self.tick += 1;
            return;
        }
        if self.food_lockout > 0 {
            self.food_lockout -= 1;
        }
        // Objects update in RebuildZOrder order: dead aliens, food, guppies,
        // aliens, pets, shots, coins.
        self.update_dead_aliens();
        // Warps sit between the coin sub-list and food in z-order.
        self.warps.retain_mut(Warp::update);
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
        // Aliens (`+0xb8`, pass 1).
        let mut i = 0;
        while i < self.aliens.len() {
            let mut out = AlienOut::default();
            self.aliens[i].update(&self.guppies, &mut self.rng, &mut out);
            if let Some(g) = out.ate {
                let victim = self.guppies.remove(g);
                // Eat sound (`0x00538560`): one draw when not throttled.
                if self.sample_ready(0x113, 3) {
                    let chomp = if self.rng.next_range(3) == 0 {
                        Sound::Chomp2
                    } else {
                        Sound::Chomp
                    };
                    self.sounds.push(chomp.into());
                }
                // Gore burst (`app+0x882`, on by default): 2..4 kind-1 shots.
                // TODO(options): the original's option that clears 0x882.
                let n = self.rng.next_range(3) + 2;
                for _ in 0..n {
                    let y = self.rng.next_range(20) as i32 + 15 + victim.widget_y;
                    let x = self.rng.next_range(20) as i32 + 15 + victim.widget_x;
                    self.add_effect_shot(x, y, 1);
                }
            }
            if out.died {
                self.kill_alien(i);
                continue;
            }
            i += 1;
        }
        // Pets (`+0xb0`) update after aliens and before coins (z-order).
        for i in 0..self.pets.len() {
            let mut out = PetOut::default();
            self.pets[i].update(self.has_aliens, &self.coins, &mut self.rng, &mut out);
            if let Some(ci) = out.picked {
                // `0x004ddc40`: bank the coin, remove it, play its sound.
                let c = self.coins.remove(ci);
                self.money = (self.money + c.value() * c.multiplier).min(9_999_999);
                self.play_collect_sound(c.kind);
            }
        }
        self.shots.retain_mut(Shot::update);
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
        // Paused: clicks do nothing and nothing moves (only the clock runs).
        let before = b.clock;
        b.step(&[Input::Click { x: 300, y: 200 }]);
        assert_eq!((b.clock, b.money, b.foods.len()), (before + 1, 200, 0));
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
        assert_eq!(b.sounds, vec![Sound::Buzzer.into()]);
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
