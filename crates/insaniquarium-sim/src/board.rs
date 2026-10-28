//! The tank (`Sexy::Board`). Spec: `docs/re/sim/board.md`.
//!
//! Objects live in typed, insertion-ordered vectors that mirror the
//! original's Board vectors; update order follows `Board::RebuildZOrder`.

use sexy_core::rand::MtRand;

use crate::alien::{Alien, AlienOut, DeadAlien, Shot, Warp};
use crate::coin::{Coin, CoinStep};
use crate::fish::{DeadFish, Fish, FishEnv, World};
use crate::food::{Food, FoodStep};
use crate::message::{self, HINT_BATTLE_TIP, HINT_DANGER, HINT_NEW_BREED, HINTS, Message};
use crate::pet::{NIKO, Pet, PetOut, STINKY};
use crate::shop::{EGG_DONE, Shop, Slot};
use crate::sound::{Music, Played, Sound};

/// Player input, applied at the start of a tick.
/// A board dialog's button stays disabled until the dialog has updated this
/// many times (`MoneyDialog` slot 76 @ `0x00530a30` with 30 from
/// `WinFishApp::DoDialog`; dialog.md#button-delay).
pub const DIALOG_BUTTON_DELAY: i32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Mouse-down at board coordinates.
    Click { x: i32, y: i32 },
    /// Right-button down at board coordinates (pet-presto.md).
    RightClick { x: i32, y: i32 },
    /// Left-button up at board coordinates: closes a board dialog through
    /// its button, or picks in the PRESTO CHANGE-O dialog.
    Release { x: i32, y: i32 },
    /// Right-button up: only the PRESTO CHANGE-O dialog takes it.
    RightRelease { x: i32, y: i32 },
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
    /// Dialog 0x0e "WARNING! A new breed of alien…" (tank 2-3).
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

/// Which sample Prego's births play (`0x005382c0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PregoSound {
    /// `SOUND_BABY` (0x108): the default.
    #[default]
    Baby,
    /// `SOUND_FART` (0x120): profile bit 1, "Original Prego Sound".
    Fart,
    /// `SOUND_FART2` (0x121): profile bit 6, "Ultra Prego Sound".
    Fart2,
}

#[derive(Debug, Clone)]
pub struct Board {
    pub rules: Rules,
    /// Replay-only: which of this tick's effect shots draw the original's
    /// stale start delay (`alien::ArtifactPlan`). Empty in play.
    pub artifacts: crate::alien::ArtifactPlan,
    /// Profile flags `+0x84` bits 1 and 6 (cheat toggles "Original Prego
    /// Sound" and "Ultra Prego Sound"): Prego's birth sound
    /// (`0x005382c0`, pet-prego.md#update). Off by default.
    pub prego_sound: PregoSound,
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
    /// Board updates since the dialog opened: its button ignores clicks
    /// until [`DIALOG_BUTTON_DELAY`] (dialog.md#button-delay).
    pub dialog_age: i32,
    /// The PRESTO CHANGE-O dialog (id `0x1f`), if open. The board keeps
    /// running under it (pet-presto.md#the-petdialog-0x005355c0).
    pub pet_dialog: Option<crate::presto::PetDialog>,
    /// The profile's owned pets (`[app+0x8b8] + id`): the dialog's
    /// enabled buttons.
    pub owned: [bool; crate::profile::PET_COUNT],
    /// Paused by focus loss or the menu ("Game Paused", dialog 18): board
    /// clicks are swallowed until it's resumed.
    pub focus_paused: bool,
    /// One-time hint bytes `+0x4b0..` (message.rs): each hint dialog or
    /// one-time message is shown once per level.
    pub hints: [bool; HINTS],
    /// The message line (`board+0x118`, board.md#message-line).
    pub message: Message,
    /// `board+0xa0`: guppies in insertion order.
    pub guppies: Vec<Fish>,
    /// `board+0xa4`: carnivores (`Sexy::Oscar`), updated right after the
    /// guppies (carnivore.md).
    pub carnivores: Vec<Fish>,
    /// `+0xb4`: fish-shaped pets (`FishTypePet`); Itchy only for now.
    pub fish_pets: Vec<Fish>,
    /// `board+0xac`: food in insertion order.
    pub foods: Vec<Food>,
    /// `board+0xa8`: coins in insertion order.
    pub coins: Vec<Coin>,
    /// `board+0xec`: pearls (coin kind 15). Last in z-order, so in front of
    /// everything and updated after the other coins (pet-niko.md#the-pearl).
    pub pearls: Vec<Coin>,
    /// Meryl's rising music notes (`Coin` kind 16, list `+0xe8`).
    pub notes: Vec<Coin>,
    /// Meryl's song flag (global `0x005e89cc`): guppies drop coins every
    /// 35 ticks while it is set (pet-meryl.md).
    pub song: bool,
    /// Wadsworth's shelter (globals `0x005e89d0..d8`): his widget position
    /// as of his last update, while sheltering (pet-wadsworth.md).
    pub shelter: Option<(i32, i32)>,
    /// Guppies Angie revived this tick (indices): they join the end of the
    /// widget list and update after the pets.
    pub revived: Vec<usize>,
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
    /// The next missile id (for a fish's `+0x98` link).
    pub next_missile_id: u32,
    /// `board+0xd0`: Starcatchers (`Sexy::Penta`, starcatcher.md).
    pub starcatchers: Vec<crate::starcatcher::Starcatcher>,
    /// `board+0xdc`: missiles (Stanley's balls for now).
    pub missiles: Vec<crate::missile::Missile>,
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
    /// `board+0x98`: dead guppies sinking (cosmetic, no RNG).
    pub dead_fish: Vec<DeadFish>,
    /// `board+0x440`: a star potion is armed (the next tank click drops
    /// it, free); `+0x441`: its first-buy hint was shown.
    pub potion_armed: bool,
    pub potion_hint_shown: bool,
    /// `board+0x2b4`: screen-shake timer (−1 per update while non-zero).
    pub shake: i32,
    /// `+0x3d8..+0x3e0`: x of the three tank-light bands (cosmetic;
    /// board.md#tank-lighting).
    pub light_x: [f32; 3],
}

impl Board {
    pub fn new(rng: MtRand, global_rng: MtRand) -> Self {
        Self {
            rules: Rules::default(),
            artifacts: Default::default(),
            prego_sound: PregoSound::Baby,
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
            dialog_age: 0,
            pet_dialog: None,
            owned: [false; crate::profile::PET_COUNT],
            focus_paused: false,
            hints: [false; HINTS],
            message: Message::default(),
            guppies: Vec::new(),
            carnivores: Vec::new(),
            fish_pets: Vec::new(),
            foods: Vec::new(),
            coins: Vec::new(),
            pearls: Vec::new(),
            notes: Vec::new(),
            song: false,
            shelter: None,
            revived: Vec::new(),
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
            dead_fish: Vec::new(),
            shake: 0,
            potion_armed: false,
            potion_hint_shown: false,
            missiles: Vec::new(),
            starcatchers: Vec::new(),
            next_missile_id: 1,
            light_x: [0.0; 3],
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
        // `Board::Board` (`0x00542150`): the three light-band x positions,
        // `r % 640` each (cosmetic, board.md#tank-lighting).
        for x in &mut b.light_x {
            *x = b.rng.next_range(640) as f32;
        }
        // `InitLevel` → `SetupShop`, after the ctor's draws: tank 2's wave
        // table (shop-bar.md#shop-setup); 2-5 draws its kind.
        if tank == 2 {
            b.wave_countdown = 3000;
            b.wave_kind = match level {
                1 => 2,
                2 => 3,
                3 => 4,
                4 => 5,
                _ => (b.rng.next_u31() & 1) as i32 | 4,
            };
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
        // The welcome line (`PopulateLevel` @ `0x00549be1`, Adventure).
        // TODO(spec): `profile+0x59` (replays after beating the game).
        let welcome = match (tank, level) {
            (1, 1) => Some(("Welcome to the Insaniquarium!", 0)),
            (1, 2) => Some(("Welcome to the next level!", 0x10)),
            (2, 1) => Some(("Welcome to the second tank! Now things get tricky!", 0x13)),
            _ => None,
        };
        if let Some((text, id)) = welcome {
            b.show_message(text, false, id);
        }
        b
    }

    /// `Board::ShowMessage` (`0x0053e950`, board.md#message-line).
    pub fn show_message(&mut self, text: &str, flash: bool, id: i32) {
        message::show(&mut self.message, &mut self.hints, text, flash, id);
    }

    /// `Board::AddPet(id, -1, -1)` (`0x00544a90`): 2 position draws, the
    /// pet ctor, then the shadow draw. Presto (19) is always flagged.
    pub fn add_pet(&mut self, id: i32) {
        self.add_pet_at(id, None, id == crate::presto::PRESTO);
    }

    /// `Board::AddPet(id, x, y, presto)`: with a position there are no
    /// position draws (Presto's change, pet-presto.md#the-change). A
    /// flagged pet keeps its position (no floor or fixed spot), starts with
    /// the 360-tick recharge (0 for Presto) and the smoke puff.
    pub fn add_pet_at(&mut self, id: i32, at: Option<(i32, i32)>, presto: bool) {
        use crate::presto::{PRESTO, RECHARGE};
        let (a, _b) = at.unwrap_or_else(|| {
            let a = self.rng.next_range(265) as i32 + 105;
            let b = self.rng.next_range(520) as i32 + 20;
            (a, b)
        });
        let recharge = if presto && id != PRESTO { RECHARGE } else { 0 };
        if matches!(
            id,
            STINKY | NIKO | crate::pet::CLYDE | crate::pet::RUFUS | crate::pet::RHUBARB
        ) {
            let mut p = match id {
                // The backdrop `+0x3e8` SetupShop picks per tank (`0x00541200`).
                NIKO => {
                    let backdrop = match self.tank {
                        3 => 4,
                        4 => 5,
                        5 => 6,
                        t => t,
                    };
                    Pet::niko(a, _b, backdrop, &mut self.rng)
                }
                crate::pet::CLYDE => Pet::clyde(a, _b, &mut self.rng),
                crate::pet::RUFUS => Pet::rufus(a, _b, &mut self.rng),
                crate::pet::RHUBARB => Pet::rhubarb(a, _b, &mut self.rng),
                _ => Pet::stinky(a, _b, &mut self.rng),
            };
            if presto {
                // The ctor's floor y and Niko's fixed spot (`0x004d89f0`)
                // only apply to unflagged pets.
                (p.x, p.y) = (f64::from(a), f64::from(_b));
                (p.widget_x, p.widget_y) = (a, _b);
                p.presto = true;
                p.countdown_1ac = recharge;
            }
            // Niko gets no shadow (`0x00544800` skips pet id 1), so no draw:
            // `tank1-3_full_01` starts with 3 shadows for 2 guppies + Stinky.
            // The others' type-1 Shadow ctor draws its floor y (`board.md#shadows`).
            if id != NIKO {
                p.shadow_y = self.rng.next_range(5) as i32 + 410;
            }
            self.pets.push(p);
            return;
        }
        // `FishTypePet` (pet-itchy.md#construction): `Fish::Init`'s 6 draws,
        // the id's ctor branch, then the shadow (one draw for every type).
        let rng = &mut self.rng;
        let mut f = match id {
            crate::itchy::PREGO => Fish::new_prego(a, _b, rng),
            crate::itchy::VERT => Fish::new_vert(a, _b, rng),
            crate::itchy::ZORF => Fish::new_zorf(a, _b, rng),
            crate::itchy::AMP => Fish::new_amp(a, _b, rng),
            crate::itchy::GASH => Fish::new_gash(a, _b, rng, presto),
            crate::itchy::ANGIE => Fish::new_angie(a, _b, rng),
            crate::itchy::BLIP => Fish::new_blip(a, _b, rng),
            crate::itchy::NIMBUS => Fish::new_nimbus(a, _b, rng),
            crate::itchy::SEYMOUR => Fish::new_seymour(a, _b, rng),
            crate::itchy::SHRAPNEL => Fish::new_shrapnel(a, _b, rng),
            crate::itchy::GUMBO => Fish::new_gumbo(a, _b, rng),
            crate::itchy::WADSWORTH => Fish::new_wadsworth(a, _b, rng),
            crate::itchy::MERYL => Fish::new_meryl(a, _b, rng),
            PRESTO => Fish::new_presto(a, _b, rng),
            crate::itchy::ITCHY => Fish::new_itchy(a, _b, rng),
            crate::itchy::BRINKLEY => Fish::new_brinkley(a, _b, rng),
            crate::itchy::NOSTRADAMUS => Fish::new_nostradamus(a, _b, rng),
            crate::itchy::STANLEY => Fish::new_stanley(a, _b, rng),
            crate::itchy::WALTER => Fish::new_walter(a, _b, rng),
            // TODO(spec): pets 20–23 (bonus-pets-plan.md).
            _ => {
                self.rng.next_range(5);
                return;
            }
        };
        f.presto = presto;
        f.recharge = recharge;
        self.fish_pets.push(f);
        self.rng.next_range(5);
    }

    /// The pet id of fish pet `f` (`+0x234`).
    fn fish_pet_id(f: &Fish) -> i32 {
        use crate::fish::FishKind as K;
        use crate::itchy::*;
        match f.kind {
            K::Prego => PREGO,
            K::Vert => VERT,
            K::Zorf => ZORF,
            K::Meryl => MERYL,
            K::Wadsworth => WADSWORTH,
            K::Seymour => SEYMOUR,
            K::Shrapnel => SHRAPNEL,
            K::Gumbo => GUMBO,
            K::Blip => BLIP,
            K::Nimbus => NIMBUS,
            K::Amp => AMP,
            K::Gash => GASH,
            K::Angie => ANGIE,
            K::Presto => crate::presto::PRESTO,
            K::Brinkley => BRINKLEY,
            K::Nostradamus => NOSTRADAMUS,
            K::Stanley => STANLEY,
            K::Walter => WALTER,
            _ => ITCHY,
        }
    }

    /// `0x004f2650`: false (and BUZZER + "Recharging..." over the widget)
    /// while a Presto pet is recharging.
    fn presto_ready(&mut self, recharge: i32, wx: i32, wy: i32, w: i32) -> bool {
        if recharge < 1 {
            return true;
        }
        self.play(0x112, Sound::Buzzer);
        // `0x005444e0`: a kind-16 rising `Coin` carrying the text in
        // colour 2, which puts it on the `+0xec` list (`AddGameObject`
        // @ `0x00543182`). It isn't mouse-visible.
        let mut c = Coin::new(wx + w / 2 - 20, wy - 10, 16);
        c.text = Some("Recharging...");
        c.colour = 2;
        self.pearls.push(c);
        false
    }

    /// A right-click in the tank (`0x0053bfb0`, pet-presto.md#how-a-right-
    /// click-reaches-him): the first Presto-flagged pet under the point
    /// gets it, else the glass is tapped.
    fn right_click(&mut self, x: i32, y: i32) {
        // TODO(spec): a right-click on a shop button.
        if self.shop.hit(x, y).is_some() {
            return;
        }
        let hit =
            |wx: i32, wy: i32, w: i32| (wx..wx + w).contains(&x) && (wy..wy + 80).contains(&y);
        if let Some(i) = self
            .fish_pets
            .iter()
            .position(|f| f.presto && hit(f.widget_x, f.widget_y, f.widget_w()))
        {
            let f = &self.fish_pets[i];
            let (r, wx, wy, w) = (f.recharge, f.widget_x, f.widget_y, f.widget_w());
            if self.presto_ready(r, wx, wy, w) {
                self.pet_dialog = Some(crate::presto::PetDialog::new());
            }
            return;
        }
        if let Some(i) = self
            .pets
            .iter()
            .position(|p| p.presto && hit(p.widget_x, p.widget_y, 80))
        {
            let p = &self.pets[i];
            let (r, wx, wy) = (p.countdown_1ac, p.widget_x, p.widget_y);
            if self.presto_ready(r, wx, wy, 80) {
                self.pet_dialog = Some(crate::presto::PetDialog::new());
            }
            return;
        }
        // The board itself only passes right-clicks below y 40
        // (`0x00543f00`); objects pass theirs wherever they are.
        if y > 40 {
            self.sounds.push(Sound::TapGlass.into());
        }
    }

    /// A pick in the PRESTO CHANGE-O dialog (`0x0054b450` → slot 73,
    /// pet-presto.md#the-change): the first Presto pet (`0x00539540`) is
    /// replaced by a new, flagged pet `n` at its widget position.
    pub fn presto_change(&mut self, n: i32) {
        let fish = self.fish_pets.iter().position(|f| f.presto);
        let (old_id, recharge, wx, wy, w) = if let Some(i) = fish {
            let f = &self.fish_pets[i];
            (
                Self::fish_pet_id(f),
                f.recharge,
                f.widget_x,
                f.widget_y,
                f.widget_w(),
            )
        } else if let Some(i) = self.pets.iter().position(|p| p.presto) {
            let p = &self.pets[i];
            (p.id, p.countdown_1ac, p.widget_x, p.widget_y, 80)
        } else {
            return;
        };
        if n == old_id || !self.presto_ready(recharge, wx, wy, w) {
            return;
        }
        self.play(0x122, Sound::Grow);
        self.add_pet_at(n, Some((wx, wy)), true);
        // Remove the old pet (`0x00541d90`); its shadow goes with it.
        if let Some(i) = fish {
            let old = self.fish_pets.remove(i);
            // Between Presto and Walter the punch count and tired timer
            // carry over (`0x004f8a56`, pet-walter.md#construction).
            use crate::fish::FishKind as K;
            if matches!(
                (old.kind, n),
                (K::Walter, crate::presto::PRESTO) | (K::Presto, crate::itchy::WALTER)
            ) && let Some(new) = self.fish_pets.last_mut()
            {
                new.punches = old.punches;
                new.tired = old.tired;
            }
            // `0x004d6250`: a sheltering Wadsworth releases his guppies,
            // Meryl's song stops.
            match old.kind {
                crate::fish::FishKind::Wadsworth if old.blink => self.shelter = None,
                crate::fish::FishKind::Meryl => self.song = false,
                _ => {}
            }
        } else if let Some(i) = self.pets.iter().position(|p| p.presto) {
            self.pets.remove(i);
        }
    }

    /// Amp's zap (`0x004fb680`, pet-amp.md#the-zap): every live guppy dies
    /// (a corpse that updates this tick) and drops a $200 kind-5 coin (up
    /// to the cap), with a spark each; a free guppy if the tank is empty.
    fn amp_zap(&mut self, amp: usize) {
        let n = self.guppies.len() as i32;
        let cap = ((n - 20) / 2 + 20).max(20);
        let victims = std::mem::take(&mut self.guppies);
        for (i, g) in victims.iter().enumerate() {
            // Made in input handling, before the object pass: the pass gives
            // it its first update this tick.
            self.dead_fish.push(DeadFish::from_fish(g));
            if (i as i32) < cap {
                self.coins
                    .push(Coin::new((g.x + 15.0) as i32, (g.y + 10.0) as i32, 5));
            }
            let s = self.rng.next_range(3) as i32 + 3;
            self.add_effect_shot(g.x as i32, g.y as i32, s);
        }
        // TODO(spec): id-21 pets the zap also takes.
        if self.guppies.is_empty() && self.carnivores.is_empty() {
            self.add_random_guppy();
        }
        self.play(0x119, Sound::Eel1);
        if !victims.is_empty() {
            self.play(0x117, Sound::Die);
        }
        let f = &mut self.fish_pets[amp];
        f.coin_interval += 200;
        f.coin_timer = -20;
        f.clicks = 0;
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
        // Shop buttons are top-level widgets: they take the click first and
        // purchase on mouse-down (shop-bar.md#input-path).
        if let Some(slot) = self.shop.hit(x, y) {
            return self.purchase(slot);
        }
        // Coins are above the pets in z-order: a coin under the cursor takes
        // the click before a charged Amp does.
        let coin_on_top = self.coins.iter().any(|c| !c.collected && c.hit(x, y));
        if !coin_on_top {
            // A charged Amp is mouse-visible and sits above the guppies
            // (`+0xb4`; pet-amp.md#clicks): clicks 1 and 2 crackle, the third
            // zaps the tank.
            if let Some(i) = self
                .fish_pets
                .iter()
                .rposition(|f| f.amp_charged() && f.amp_hit(x, y))
            {
                if self.fish_pets[i].clicks < 2 {
                    self.fish_pets[i].clicks += 1;
                    self.play(0x11b, Sound::Eel3);
                } else {
                    self.amp_zap(i);
                }
                return ClickResult::Ignored;
            }
            // Walter is mouse-visible: a click on him punches, unless he's
            // busy, turning or tired, when it falls through to the tank
            // (pet-walter.md#clicks-slot-54-0x004fbe20--0x004fb680).
            if let Some(i) = self
                .fish_pets
                .iter()
                .rposition(|f| f.kind == crate::fish::FishKind::Walter && f.walter_hit(x, y))
                && let Some(punched) = self.fish_pets[i].walter_punch()
            {
                if punched {
                    self.play(0x136, Sound::Sfx);
                }
                return ClickResult::Ignored;
            }
        }
        // With aliens present, a click on a coin collects it, and is also a
        // shot if it lands inside an alien (`alien-sylvester.md#41`). With
        // only missiles left (the Destructor dead), clicks stay shots
        // (alien-kind5.md#41).
        if !self.aliens.is_empty() || self.missiles.iter().any(|m| m.kind == 0) {
            if let Some(i) = self.coins.iter().rposition(|c| !c.collected && c.hit(x, y)) {
                if self.aliens.iter().any(|a| a.contains(x, y)) {
                    self.shoot(x, y);
                }
                let c = &mut self.coins[i];
                if !c.collected {
                    c.collect();
                    let kind = c.kind;
                    self.play_collect_sound(kind);
                }
                return ClickResult::CoinCollected;
            }
            return self.shoot(x, y);
        }
        // Pearls are the front-most widgets (pet-niko.md#the-pearl).
        if let Some(i) = self
            .pearls
            .iter()
            .rposition(|c| !c.collected && c.kind != 16 && c.hit(x, y))
        {
            if !self.pearls[i].collected {
                self.pearls[i].collect();
                for p in self.pets.iter_mut().filter(|p| p.id == NIKO) {
                    p.pearl_taken = true;
                }
                self.play(Sound::Pearl.sample_id(), Sound::Pearl);
            }
            return ClickResult::CoinCollected;
        }
        // Widgets on top get the click first: coins are in front of fish
        // and food (z-order), last-added on top. A collected coin no longer
        // takes clicks (`0x004e51f0` clears its widget's mouse flag `+0x55`;
        // `zoo_e_01` tick 5383: a click on a flying coin fed the guppy
        // under it).
        // TODO(spec): pets, fish clicks.
        if let Some(i) = self.coins.iter().rposition(|c| !c.collected && c.hit(x, y)) {
            let c = &mut self.coins[i];
            if !c.collected {
                c.collect();
                let kind = c.kind;
                self.play_collect_sound(kind);
            }
            return ClickResult::CoinCollected;
        }
        // A potion (food kind 2) is mouse-visible: in front of the board,
        // behind the guppies in z-order; a guppy click over it also picks
        // it up first (`0x00539f30`). Either way the first potion in list
        // order under the point (pet-nostradamus.md#the-potion-food-kind-2).
        if let Some(i) = self
            .foods
            .iter()
            .position(|f| f.kind == 2 && !f.carried && f.hit(x, y))
        {
            self.foods[i].carried = true;
            self.play_collect_sound(1);
            return ClickResult::CoinCollected;
        }
        // Food lockout after a kill only covers clicks within 50 px of the
        // last shot; a click elsewhere ends it (`0x00543972`). This is in the
        // board's own handler: clicks a coin or pearl takes don't reach it.
        // TODO(spec): whether a guppy's own handler checks it.
        if self.food_lockout > 0 {
            let (dx, dy) = (self.last_shot.0 - x, self.last_shot.1 - y);
            if dx * dx + dy * dy > 2500 {
                self.food_lockout = 0;
            }
        }
        // Guppies are next in z-order (the last one on top): a click on one
        // drops food through its own handler (food.md#clicking-on-a-guppy).
        if let Some(i) = self.guppies.iter().rposition(|f| {
            (0..80).contains(&(x - f.widget_x)) && (0..80).contains(&(y - f.widget_y))
        }) {
            return self.click_guppy(i, x, y);
        }
        if !(31..=586).contains(&x) || !(61..=399).contains(&y) || self.food_lockout > 0 {
            return ClickResult::Ignored;
        }
        // An armed star potion drops for free (`0x00543b68`).
        if !self.potion_armed {
            if self.money < self.food_price {
                // TODO(spec): buzzer, money flash and "out of money" handling.
                return ClickResult::TooPoor;
            }
            self.money -= self.food_price;
        }
        self.add_food_delay(x - 10, y - 10, 0)
    }

    /// `Fish` mouse-down (slot 55 @ `0x004e4c10`, food.md#clicking-on-a-
    /// guppy): drop food where the guppy was clicked; a click on its body
    /// makes the pellet inedible for a while.
    fn click_guppy(&mut self, i: usize, x: i32, y: i32) -> ClickResult {
        // No pellet while a missile (a Stanley ball) is in the tank
        // (`food.md#clicking-on-a-guppy-fish-mouse-down-slot-55-0x004e4c10`).
        if !self.missiles.is_empty() {
            return ClickResult::Ignored;
        }
        let f = &self.guppies[i];
        let (lx, ly) = (x - f.widget_x, y - f.widget_y);
        let (ax, ay) = (f64::from(lx) + f.x, f64::from(ly) + f.y);
        if !(ax < 587.0 && ax > 30.0 && ay < 400.0 && ay > 60.0) {
            return ClickResult::Ignored;
        }
        let (fx, fy) = ((ax - 10.0) as i32, (ay - 10.0) as i32);
        let delay = if (10..=70).contains(&lx) && (10..=70).contains(&ly) {
            (70 - ly) / 2
        } else {
            0
        };
        if self.money < self.food_price {
            // TODO(spec): buzzer, money flash and "out of money" handling.
            return ClickResult::TooPoor;
        }
        self.money -= self.food_price;
        self.add_food_delay(fx, fy, delay)
    }

    /// A click with Gus first: no price, a pellet at (x − 10, y − 10),
    /// edible after 20 ticks if dropped near his middle (`0x004d46f0`).
    fn feed_gus(&mut self, x: i32, y: i32) -> ClickResult {
        if !(31..=586).contains(&x) || !(61..=379).contains(&y) {
            return ClickResult::Ignored;
        }
        let (acx, acy) = (self.aliens[0].widget_x + 80, self.aliens[0].widget_y + 80);
        let near = x - 35 < acx && acx < x + 75 && y - 35 < acy && acy < y + 75;
        self.add_food_delay(x - 10, y - 10, if near { 20 } else { 0 })
    }

    /// Board mouse-down with aliens present.
    fn shoot(&mut self, x: i32, y: i32) -> ClickResult {
        // Gus first in the list: the click drops a free pellet instead
        // (alien-kind4.md#41-input-routing-board-mouse-down--0x00543920).
        if self.aliens.first().is_some_and(|a| a.kind == 4) {
            return self.feed_gus(x, y);
        }
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
            // The missile walk (`0x004d82a0`): the first kind-0 missile in
            // the box explodes, plus a hit spark (alien-kind5.md#64).
            if let Some(m) = self.missiles.iter().position(|m| m.clicked(x, y)) {
                self.explode_missile(m);
                self.add_effect_shot(x - 40, y - 40, 2);
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

    /// The first fish a bomb centred at (`cx`, `cy`) touches: guppies, then
    /// carnivores (the other fish lists aren't modelled yet). Index into
    /// guppies, then carnivores.
    fn bomb_victim(&self, cx: f64, cy: f64) -> Option<usize> {
        let touches = |f: &Fish| {
            let (fx, fy) = (f64::from(f.widget_x), f64::from(f.widget_y));
            !f.dead && fx + 10.0 < cx && cx < fx + 70.0 && fy + 10.0 < cy && cy < fy + 70.0
        };
        self.guppies
            .iter()
            .chain(&self.carnivores)
            .position(touches)
    }

    /// The bomb's victim dies the starvation death (slot 88, argument 1):
    /// DIE (gap 3) and a corpse that updates this tick (board.md).
    fn bomb_kill(&mut self, victim: usize) {
        let n = self.guppies.len();
        let f = if victim < n {
            &mut self.guppies[victim]
        } else {
            &mut self.carnivores[victim - n]
        };
        f.dead = true;
        let mut corpse = DeadFish::from_fish(f);
        if victim >= n {
            self.carnivore_died();
        }
        if self.sample_ready(0x117, 3) {
            self.sounds.push(Sound::Die.into());
        }
        corpse.update();
        self.dead_fish.push(corpse);
    }

    /// A bomb removed without being banked (`0x004d5510`): EXPLODE, then
    /// `r % 3 + 2` effect shots, each with its kind, y and x drawn.
    fn explode_bomb(&mut self, c: &Coin) {
        self.play(0x11d, Sound::Explode);
        let n = self.rng.next_range(3) + 2;
        for _ in 0..n {
            let s = self.rng.next_range(3) as i32 + 3;
            let y = self.rng.next_range(30) as i32 - 10 + c.widget_y;
            let x = self.rng.next_range(30) as i32 - 10 + c.widget_x;
            self.add_effect_shot(x, y, s);
        }
    }

    /// `EffectShot` (`0x005436f0`, see `alien::add_effect_shot`).
    fn add_effect_shot(&mut self, x: i32, y: i32, kind: i32) {
        crate::alien::add_effect_shot_with(
            &mut self.shots,
            &mut self.rng,
            x,
            y,
            kind,
            &mut self.artifacts,
        );
    }

    /// The Starcatchers (`+0xd0`, after the warps, before food and the
    /// guppies; starcatcher.md#per-tick-update-slot-22--0x004f94e0).
    fn update_starcatchers(&mut self) {
        let mut i = 0;
        while i < self.starcatchers.len() {
            let has_aliens = !self.aliens.is_empty();
            let out = self.starcatchers[i].update(has_aliens, &self.coins, &mut self.rng);
            if let Some((ci, x, y)) = out.ate {
                // The star goes unbanked; a rising $200 coin (kind 5) and BUY.
                self.coins.remove(ci);
                self.coins.push(Coin::new(x, y, 5));
                self.play(0x111, Sound::Buy);
            }
            if out.died {
                // Starved (`0x004f3c60`): DIE (pitch +4), a kind-8 corpse
                // with vy 0, the 2-2 message chain. The rest of the update
                // already ran this tick (its reroll draw is real).
                let s = self.starcatchers.remove(i);
                self.play(0x117, Sound::Die);
                let mut d = crate::fish::DeadFish::starcatcher(&s);
                d.update();
                self.dead_fish.push(d);
                if (self.tank, self.level) == (2, 2) {
                    self.starcatcher_died_messages();
                }
                continue;
            }
            i += 1;
        }
    }

    /// The 2-2 chain when a starving Starcatcher is lost (`0x004f3ad0`):
    /// each line shows if its one-time byte is clear, in this order, the
    /// later ones only once the earlier ones were shown.
    fn starcatcher_died_messages(&mut self) {
        if self.hints[0x23] {
            self.show_message(
                "Hint: Star potions allow guppies to produce stars!",
                false,
                0x24,
            );
        }
        if self.hints[0x22] {
            self.show_message("Hint: Starcatchers need stars to stay alive!", false, 0x23);
        }
        self.show_message("Warning! Your starcatcher has died!", false, 0x22);
    }

    /// A star potion exploding (at the bottom, or in a small/medium
    /// guppy, food.md): EXPLODE, then 2..4 sparks at (wx ± 10.., wy ± 10..).
    fn star_explosion(&mut self, wx: i32, wy: i32) {
        self.play(0x11d, Sound::Explode);
        let n = self.rng.next_range(3) + 2;
        for _ in 0..n {
            let k = self.rng.next_range(3) as i32 + 3;
            let y = wy + self.rng.next_range(30) as i32 - 10;
            let x = wx + self.rng.next_range(30) as i32 - 10;
            self.add_effect_shot(x, y, k);
        }
    }

    /// The missile pass (`+0xdc`, right after the fish pets in z-order).
    /// Stanley's ball: a hit does 15 HP; at 0 the alien dies inside the
    /// ball's update (pet-stanley.md#the-ball-missle-kind-2).
    fn update_missiles(&mut self) {
        let mut i = 0;
        while i < self.missiles.len() {
            if self.missiles[i].kind == 0 {
                if self.update_missile_kind0(i) {
                    i += 1;
                }
                continue;
            }
            let target = self.missiles[i]
                .target
                .and_then(|t| self.aliens.get(t))
                .map(|a| (a.widget_x, a.widget_y));
            let aliens = !self.aliens.is_empty();
            match self.missiles[i].update(target, aliens) {
                crate::missile::MissileStep::HitFish | crate::missile::MissileStep::Explode => {
                    i += 1;
                }
                crate::missile::MissileStep::Keep => i += 1,
                crate::missile::MissileStep::Remove => {
                    self.missiles.remove(i);
                    self.end_of_wave_check();
                }
                crate::missile::MissileStep::Hit { target, damage } => {
                    i += 1;
                    self.play(0x136, Sound::Sfx);
                    self.aliens[target].health -= f64::from(damage);
                    if self.aliens[target].health <= 0.0 {
                        let n = self.dead_aliens.len();
                        self.kill_alien(target);
                        self.update_dead_aliens_from(n);
                    }
                }
            }
        }
    }

    /// A punched Presto pet `i` turns back into Presto (slot 73 with 19,
    /// pet-presto.md#the-change): GROW, a new flagged Presto at its widget
    /// spot (no position draws), the old pet removed.
    fn presto_revert(&mut self, i: usize) {
        let (wx, wy) = (self.fish_pets[i].widget_x, self.fish_pets[i].widget_y);
        self.play(0x122, Sound::Grow);
        self.add_pet_at(crate::presto::PRESTO, Some((wx, wy)), true);
        let old = self.fish_pets.remove(i);
        match old.kind {
            crate::fish::FishKind::Wadsworth if old.blink => self.shelter = None,
            crate::fish::FishKind::Meryl => self.song = false,
            _ => {}
        }
    }

    /// The glove's hit pass (`0x004e9a60` → `0x004e0ed0`, pet-walter.md):
    /// every guppy, carnivore and fish pet (but Prego and Walter `w`
    /// himself) whose centre is within 40 px of the fist, not already
    /// pushed, is knocked aside: push timer 50, push ±15 or ±10 (one global
    /// draw each), its coin clock put k ticks short of its interval.
    ///
    /// Deviation: the original walks its master set in heap-address order
    /// (`pet-walter.md#deviations`); this walks guppies, carnivores, then
    /// the fish pets.
    fn glove_hits(&mut self, w: usize, px: i32, py: i32) {
        use crate::fish::FishKind as K;
        let Some(g) = self.fish_pets[w].glove else {
            return;
        };
        let s = if g.right { 1.0 } else { -1.0 };
        let hit_by = |f: &Fish| {
            let (cx, cy) = (f.widget_x + 40, f.widget_y + 40);
            f.push_timer <= 0 && px - 40 < cx && cx < px + 40 && py - 40 < cy && cy < py + 40
        };
        // Candidates: (list, index, k): 0 guppies, 1 carnivores, 2 fish pets.
        let mut hits: Vec<(u8, usize, i32)> = Vec::new();
        for (j, f) in self.guppies.iter().enumerate() {
            if hit_by(f) {
                hits.push((0, j, 5));
            }
        }
        for (j, f) in self.carnivores.iter().enumerate() {
            if hit_by(f) {
                hits.push((1, j, 5));
            }
        }
        for (j, f) in self.fish_pets.iter().enumerate() {
            if j == w || f.kind == K::Prego || !hit_by(f) {
                continue;
            }
            // A pet Presto made fewer than 30 updates ago can't be hit.
            if f.presto && f.kind != K::Presto && f.recharge > crate::presto::RECHARGE - 30 {
                continue;
            }
            let k = match f.kind {
                K::Vert | K::Shrapnel | K::Amp => 5,
                K::Nostradamus => 10,
                _ => -1,
            };
            hits.push((2, j, k));
        }
        if self.artifacts.glove_reversed {
            hits.reverse();
        }
        let hit = !hits.is_empty();
        for (list, j, k) in hits {
            let f = match list {
                0 => &mut self.guppies[j],
                1 => &mut self.carnivores[j],
                _ => &mut self.fish_pets[j],
            };
            if k > 0 {
                f.coin_timer = f.coin_interval - k;
            }
            f.push_timer = 50;
            f.misc_timer = 180;
            f.push = if self.global_rng.next_u31() & 1 == 0 {
                s * 15.0
            } else {
                s * 10.0
            };
        }
        let glove = self.fish_pets[w].glove.as_mut().expect("glove");
        if hit && glove.sound_cooldown == 0 {
            glove.sound_cooldown = 5;
            self.play(0x130, Sound::Punch);
        }
    }

    /// The fish (guppy or carnivore) a kind-0 missile homes on.
    fn missile_target(&self, id: u32) -> Option<(bool, usize)> {
        if let Some(j) = self.guppies.iter().position(|f| f.missile == Some(id)) {
            return Some((false, j));
        }
        self.carnivores
            .iter()
            .position(|f| f.missile == Some(id))
            .map(|j| (true, j))
    }

    /// One kind-0 missile's update (alien-kind5.md#52). Returns false if it
    /// was removed.
    fn update_missile_kind0(&mut self, i: usize) -> bool {
        let id = self.missiles[i].id;
        let target = self.missile_target(id);
        let pos = target.map(|(c, j)| {
            let f = if c {
                &self.carnivores[j]
            } else {
                &self.guppies[j]
            };
            (f.widget_x, f.widget_y)
        });
        match self.missiles[i].update_missile(pos) {
            crate::missile::MissileStep::HitFish => {
                // The fish dies (DIE, a corpse), then the missile explodes.
                let (c, j) = target.expect("target");
                let f = if c {
                    &mut self.carnivores[j]
                } else {
                    &mut self.guppies[j]
                };
                f.missile = None;
                f.dead = true;
                let corpse = crate::fish::DeadFish::from_fish(f);
                if self.sample_ready(0x117, 3) {
                    self.sounds.push(Sound::Die.into());
                }
                let mut d = corpse;
                d.update();
                self.dead_fish.push(d);
                if c {
                    self.carnivores.remove(j);
                    self.carnivore_died();
                } else {
                    self.guppies.remove(j);
                }
                self.explode_missile(i);
                false
            }
            crate::missile::MissileStep::Explode => {
                self.explode_missile(i);
                false
            }
            _ => {
                // Smoke (kind 0): `r % 3`; on 0 two `r % 5` and a puff.
                if self.rng.next_range(3) == 0 {
                    let a = self.rng.next_range(5) as i32;
                    let b = self.rng.next_range(5) as i32;
                    let m = &self.missiles[i];
                    let x = m.widget_x - 20 * (m.vx as i32) + 17 + a;
                    let y = m.widget_y - 16 * (m.vy as i32) + 17 + b;
                    self.add_effect_shot(x, y, 8);
                }
                true
            }
        }
    }

    /// Missile removal (slot 76, alien-kind5.md#63): unlink its fish, the
    /// end-of-wave check, EXPLODE, 2..4 sparks round it.
    fn explode_missile(&mut self, i: usize) {
        let m = self.missiles.remove(i);
        for f in self.guppies.iter_mut().chain(self.carnivores.iter_mut()) {
            if f.missile == Some(m.id) {
                f.missile = None;
            }
        }
        self.end_of_wave_check();
        self.play(0x11d, Sound::Explode);
        let n = self.rng.next_range(3) + 2;
        for _ in 0..n {
            let k = self.rng.next_range(3) as i32 + 3;
            let y = m.widget_y + self.rng.next_range(30) as i32 - 10;
            let x = m.widget_x + self.rng.next_range(30) as i32 - 10;
            self.add_effect_shot(x, y, k);
        }
    }

    /// The Destructor's volley (alien-kind5.md#32): up to three missiles,
    /// each at the farthest eligible fish without one; MISSLE once.
    fn fire_volley(&mut self, a: usize, points: [(i32, i32); 3]) -> bool {
        let (acx, acy) = (self.aliens[a].widget_x + 80, self.aliens[a].widget_y + 80);
        let mut fired = false;
        for (n, &(x, y)) in points.iter().enumerate() {
            // PickTarget (`0x004e8df0`): the farthest, list order on ties.
            let sheltered = self.shelter.is_some();
            let mut best = 0;
            let mut pick: Option<(bool, usize)> = None;
            let cands = self
                .guppies
                .iter()
                .enumerate()
                .map(|(j, f)| (false, j, f))
                .chain(
                    self.carnivores
                        .iter()
                        .enumerate()
                        .map(|(j, f)| (true, j, f)),
                );
            for (c, j, f) in cands {
                if f.missile.is_some() || (!c && sheltered && f.stage < 2) {
                    continue;
                }
                let dx = acx - (f.widget_x + 40);
                let dy = acy - (f.widget_y + 40);
                let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
                if best < d {
                    best = d;
                    pick = Some((c, j));
                }
            }
            let Some((c, j)) = pick else { break };
            let id = self.next_missile_id;
            self.next_missile_id += 1;
            let m = crate::missile::Missile::new_missile(x, y, id, &mut self.global_rng);
            self.missiles.push(m);
            if c {
                self.carnivores[j].missile = Some(id);
            } else {
                self.guppies[j].missile = Some(id);
            }
            fired = true;
            if n == 0 {
                self.play(0x128, Sound::Missle);
            }
        }
        fired
    }

    /// `0x00539c90`: with no missile, alien or Bilaterus left, the wave is
    /// over (food lockout 36, tank music, guppies mouse-visible again).
    fn end_of_wave_check(&mut self) {
        if self.missiles.is_empty() && self.aliens.is_empty() {
            self.food_lockout = 36;
            self.has_aliens = false;
            self.music = Some(Music::Tank);
        }
    }

    /// `Alien` death (`0x004f9c70`), kind 1.
    fn kill_alien(&mut self, i: usize) {
        let a = self.aliens.remove(i);
        // The alien's attached missile goes with it (slot 76); later aliens
        // shift down one in the list.
        self.missiles.retain(|m| m.target != Some(i));
        for m in &mut self.missiles {
            if let Some(t) = &mut m.target
                && *t > i
            {
                *t -= 1;
            }
        }
        let (ix, iy) = (a.widget_x, a.widget_y);
        // Diamond drop.
        let mut c = Coin::new(ix + 25, iy + 25, crate::coin::kind::DIAMOND);
        c.vy = -1.0;
        self.coins.push(c);
        // `0x00539c90`: the wave ends once no alien and no missile is left.
        self.end_of_wave_check();
        // Debris (§4.3 step 5). Bubbles draw the global stream (not modelled).
        // Gus (kind 4) bursts bigger: 2–3, 4–6 and 2–3 rounds
        // (alien-kind4.md#43-death-0x004f9c701-kind-4).
        let gus = a.kind == 4;
        let n = (self.rng.next_u31() & 1) + if gus { 2 } else { 0 };
        for _ in 0..n {
            self.rng.next_range(70);
            self.rng.next_range(70);
            self.rng.next_range(50);
            self.rng.next_range(50);
        }
        let n = self.rng.next_range(3) + if gus { 4 } else { 0 };
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
        let n = (self.rng.next_u31() & 1) + if gus { 2 } else { 0 };
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
        // Gus leaves no corpse.
        if gus {
            return;
        }
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
        self.update_dead_aliens_from(0);
    }

    /// [`Self::update_dead_aliens`] for the corpses from index `start` on.
    fn update_dead_aliens_from(&mut self, start: usize) {
        let mut i = start;
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

    /// The Sneeze of Power (`0x00543f40`, pet-nostradamus.md#sneeze-of-power-0x00543f40).
    /// All draws on the global stream; the bubbles are cosmetic (their own
    /// draws aren't modelled, as for every board bubble).
    fn sneeze(&mut self) {
        if self.global_rng.next_range(100) >= 50 {
            return;
        }
        let mut noses = 0;
        for f in self
            .fish_pets
            .iter()
            .filter(|f| f.kind == crate::fish::FishKind::Nostradamus)
        {
            noses += 1;
            let _ = (f.widget_x, f.widget_y);
            for _ in 0..5 {
                self.global_rng.next_range(21);
                self.global_rng.next_range(21);
            }
        }
        self.shake = 30;
        self.play(0x110, Sound::Bubbles);
        for _ in 0..noses.min(5) {
            // A queued SOUND_SNEEZE, delay `r % 150 + 100` (not modelled).
            self.global_rng.next_range(150);
        }
        self.wave_countdown = 635;
        self.music = Some(Music::Tank);
        self.show_message("Attack Postponed by Sneeze of Power!", false, -1);
    }

    /// Wave countdown (`Board::Update` `0x005481c4`..`0x00548e05`), tank 1.
    fn update_waves(&mut self) {
        if self.wave_2c4 != 0 {
            self.wave_2c4 -= 1;
        }
        // Skipped while any alien or missile (Stanley's balls) exists
        // (`alien-sylvester.md#per-tick-countdown`, step 2).
        if !self.aliens.is_empty() || !self.missiles.is_empty() || self.wave_kind == 0 {
            return;
        }
        self.wave_countdown -= 1;
        match self.wave_countdown {
            // One-time hints (`0x005483e0`): each pauses the board until
            // dismissed, then is marked shown (`+0x4b3`, `+0x4b4`, `+0x4c1`).
            // TODO(spec): the virtual-tank/mode checks on this path.
            276 => {
                let hint = match (self.tank, self.level) {
                    (1, 2) if !self.hints[HINT_DANGER] => Some((HINT_DANGER, Dialog::AlienWarning)),
                    (1, 2) if !self.hints[HINT_BATTLE_TIP] => {
                        Some((HINT_BATTLE_TIP, Dialog::BattleTip))
                    }
                    (2, 3) if !self.hints[HINT_NEW_BREED] => {
                        Some((HINT_NEW_BREED, Dialog::NewBreed))
                    }
                    _ => None,
                };
                if let Some((i, d)) = hint {
                    self.hints[i] = true;
                    self.paused = true;
                    self.dialog = Some(d);
                }
            }
            275 => {
                // The warning line (board.md#the-alien-warning-line).
                // TODO(spec): kinds 4–8 and 0x15 (tanks 2–5), and 4-4's
                // "FOURTEEN ALIEN SIGNATURES".
                let text = match self.wave_kind {
                    9..=12 => "MULTIPLE ALIEN SIGNATURES DETECTED",
                    4 => "ALIEN SIGNATURE TYPE-G DETECTED",
                    5 => "ALIEN SIGNATURE TYPE-D DETECTED",
                    _ => "ENEMY APPROACHING",
                };
                message::show_warning(&mut self.message, text);
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
            // Sneeze of Power (`0x00543f40`, pet-nostradamus.md): with a
            // nose in, a 50 % chance (global stream) to postpone the wave.
            30 if self.tank != 5
                && self
                    .fish_pets
                    .iter()
                    .any(|f| f.kind == crate::fish::FishKind::Nostradamus) =>
            {
                self.sneeze();
            }
            // `0x0054b1a0(1, 1, 0)`: the alien song moves on to its fight
            // section as the alien arrives.
            1 => self.music = Some(Music::AlienFight),
            c if c <= 0 => {
                self.wave_2c4 = 35;
                // `AddAlien` (`0x00545620`): wave kind 9 is two aliens,
                // Sylvester at A then Balrog at B, each added in full.
                // TODO(spec): wave kinds 8 (Bilaterus) and 10..12.
                let spawns: &[(i32, (i32, i32))] = match self.wave_kind {
                    9 => &[(1, self.wave_a), (3, self.wave_b)],
                    k => &[(k, self.wave_a)],
                };
                for &(kind, (x, y)) in spawns {
                    let a = Alien::new(x, y, kind, &mut self.rng);
                    self.warps.push(Warp::for_alien(&a));
                    self.aliens.push(a);
                    // Its shadow.
                    self.rng.next_range(5);
                    // Arrival roar (kinds 1–3) or Gus's guffaw, played
                    // directly.
                    let arrival = match kind {
                        4 => Sound::Guffaw,
                        5 => Sound::Interfer,
                        _ => Sound::Roar,
                    };
                    self.sounds.push(arrival.into());
                }
                self.has_aliens = true;
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
        // `0x005383d0` (POINTS family) also for Shrapnel's bomb (0x11).
        if matches!(kind, 1 | 2 | 3 | 8 | 9 | 10 | 0x11) && self.sample_ready(0x12c, 3) {
            let r = self.rng.next_u31() & 3;
            self.sounds.push(Sound::points(r).into());
        } else if matches!(kind, 4 | 5 | 11 | 12) {
            self.play(0x116, Sound::Diamond);
        } else if matches!(kind, 6 | 13) {
            // The chest plays the pearl sample (`0x004e51f0`).
            self.play(0x12b, Sound::Pearl);
        } else if matches!(kind, 7 | 14) {
            // TREASURE (BONUSCOLLECT in the Virtual Tank and with
            // `board+0x2a5`, not modelled).
            self.play(0x142, Sound::Treasure);
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
        self.play_throttled(id, 3, sound);
    }

    /// `0x00538230(id, gap, 1.0)`.
    fn play_throttled(&mut self, id: usize, gap: i32, sound: Sound) {
        if self.sample_ready(id, gap) {
            self.sounds.push(sound.into());
        }
    }

    /// `Board::AddFood` (`0x00543280`) for a player pellet.
    /// `Board::AddFood(x, y, 0, 0, delay, −1)` for a player drop. With a
    /// star potion armed the pellet is the potion: level 3, disarmed, no
    /// DROPFOOD (`0x00543469`). A guppy click pays the price even then.
    fn add_food_delay(&mut self, x: i32, y: i32, delay: i32) -> ClickResult {
        let extra = self
            .foods
            .iter()
            .filter(|f| f.motion != 0 || f.kind != 0)
            .count() as i32;
        if self.foods.len() as i32 >= self.max_food + extra {
            if !self.has_aliens {
                self.money += self.food_price;
            }
            // `AddFood` (`0x00543386`): on 1-1 the one-pellet hint; on 1-2,
            // once food quantity is for sale, the upgrade hint (id 0xd).
            if self.message.idle() {
                if self.tutorial {
                    self.show_message(
                        "You can only drop 1 food pellet at a time for now",
                        false,
                        6,
                    );
                } else if (self.tank, self.level) == (1, 2)
                    && self.shop.revealed[Slot::FoodQuantity as usize]
                {
                    self.show_message(
                        "Upgrade Food Quantity to drop more food at once!",
                        false,
                        0xd,
                    );
                }
            }
            return ClickResult::FoodLimit;
        }
        let mut food = Food::new_pellet(x, y, self.food_level, &mut self.rng);
        food.delay = delay;
        if self.potion_armed {
            food.level = 3;
            self.potion_armed = false;
        } else if self.sample_ready(0x118, 3) {
            self.sounds.push(Sound::DropFood.into());
        }
        self.foods.push(food);
        ClickResult::FoodDropped
    }

    /// `AddFood(x, y, 0, tossed 1, 0, level)` for Nimbus: the player's drop
    /// path (limit check, one draw, DROPFOOD) with the tossed flag.
    fn add_tossed_food(&mut self, x: i32, y: i32, level: i32) {
        let extra = self
            .foods
            .iter()
            .filter(|f| f.motion != 0 || f.kind != 0)
            .count() as i32;
        if self.foods.len() as i32 >= self.max_food + extra {
            // TODO(spec): the limit path's refund and hint (pet-nimbus.md
            // open question).
            return;
        }
        let mut food = Food::new_pellet(x, y, level, &mut self.rng);
        food.tossed = true;
        // Made during the object pass: it updates this tick (board.md;
        // `zoo_d_02` tick 2665 shows anim 1 and y − 8 on its first read).
        food.update();
        self.foods.push(food);
        if self.sample_ready(0x118, 3) {
            self.sounds.push(Sound::DropFood.into());
        }
    }

    /// One logic tick (28 ms) with this tick's inputs.
    pub fn step(&mut self, inputs: &[Input]) {
        self.apply_inputs(inputs);
        self.update();
    }

    /// The input half of [`Self::step`]: clears this step's sounds and
    /// music request, then applies `inputs`. [`Self::update`] is the rest.
    pub fn apply_inputs(&mut self, inputs: &[Input]) {
        self.artifacts.made = 0;
        self.sounds.clear();
        self.music = None;
        for i in inputs {
            // The PRESTO CHANGE-O dialog covers the screen: it takes every
            // mouse event while it is up (pet-presto.md#the-petdialog-0x005355c0).
            if let Some(d) = &mut self.pet_dialog {
                match *i {
                    Input::Click { x, y } | Input::RightClick { x, y } => d.down(&self.owned, x, y),
                    Input::Release { x, y } | Input::RightRelease { x, y } => {
                        use crate::presto::DialogPick;
                        match d.up(&self.owned, x, y) {
                            DialogPick::None => {}
                            DialogPick::Cancel => self.pet_dialog = None,
                            DialogPick::Pet(n) => {
                                self.pet_dialog = None;
                                self.presto_change(n);
                            }
                        }
                    }
                    Input::SetPaused(p) => {
                        self.paused = p;
                        self.focus_paused = p;
                    }
                    Input::DismissDialog | Input::Buy(_) => {}
                }
                continue;
            }
            match *i {
                // The focus-loss pause is the "Game Paused" dialog (id 18,
                // `zoo_b_01`): clicks go to it, not the board.
                Input::Click { x, y } if self.dialog.is_none() && !self.focus_paused => {
                    self.click(x, y);
                }
                Input::RightClick { x, y } if self.dialog.is_none() && !self.focus_paused => {
                    self.right_click(x, y);
                }
                Input::Click { .. } | Input::RightClick { .. } => {}
                // The "Game Paused" dialog's button resumes the board (the
                // button rect isn't checked yet, as for board dialogs).
                Input::DismissDialog | Input::Release { .. }
                    if self.focus_paused && self.dialog.is_none() =>
                {
                    self.paused = false;
                    self.focus_paused = false;
                }
                Input::DismissDialog | Input::Release { .. } => self.dismiss_dialog(),
                Input::RightRelease { .. } => {}
                Input::Buy(slot) if self.dialog.is_none() => {
                    self.purchase(slot);
                }
                Input::Buy(_) => {}
                Input::SetPaused(p) => {
                    if self.dialog.is_none() {
                        self.paused = p;
                        self.focus_paused = p;
                    }
                }
            }
        }
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
            Slot::Carnivore => self.buy_carnivore(price),
            Slot::Weapon => self.buy_weapon(price),
            Slot::StarPotion => self.buy_star_potion(price),
            Slot::Starcatcher => self.buy_starcatcher(price),
            // TODO(spec): other slots.
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
        // Shown before TrySpend, even when pending coins then cover it.
        if self.tutorial && self.money < price {
            self.show_message(
                "You can't afford new fish yet! Collect more money!",
                false,
                0xe,
            );
        }
        if !self.try_spend(price) {
            return false;
        }
        self.add_random_guppy();
        self.arrow_buy_fish = false;
        // Purchase sounds: a throttled `SOUND_GROW` (`0x0054941c`), then a
        // splash variant picked with one unthrottled draw (`0x00538360`).
        self.play(0x122, Sound::Grow);
        let r = self.rng.next_range(3);
        self.sounds.push(Sound::splash(r).into());
        true
    }

    /// Slot 4 (`0x00549380` case 4): spend, `BuyCarnivore`, reveal 10 and
    /// 11 (tank 1), `SOUND_GROW`, then the splash draw.
    fn buy_carnivore(&mut self, price: i32) -> bool {
        if !self.try_spend(price) {
            return false;
        }
        self.add_carnivore();
        self.shop.reveal(Slot::Weapon, true);
        self.shop.reveal(Slot::EggPiece, true);
        self.play(0x122, Sound::Grow);
        let r = self.rng.next_range(3);
        self.sounds.push(Sound::splash(r).into());
        true
    }

    /// Slot 5 (`0x00549380` case 5, shop-bar.md#per-slot-behaviour): arms
    /// the potion (`+0x440`); the next tank click drops it. The first buy
    /// on 2-1/2-2 shows a hint. No RNG.
    fn buy_star_potion(&mut self, price: i32) -> bool {
        if self.potion_armed || !self.try_spend(price) {
            return false;
        }
        if !self.potion_hint_shown {
            self.potion_hint_shown = true;
            if self.level < 3 {
                self.show_message("Click on tank to drop star potion!", false, -1);
                self.message.timer = 180;
            }
        }
        self.potion_armed = true;
        if self.sample_ready(0x111, 3) {
            self.sounds.push(Sound::Buy.into());
        }
        true
    }

    /// Slot 6 (`0x00549380` case 6): spend, `BuyStarcatcher` (x, 4 ctor
    /// draws, shadow), reveal 10 and 11, GROW, the splash draw.
    fn buy_starcatcher(&mut self, price: i32) -> bool {
        if !self.try_spend(price) {
            return false;
        }
        let mut s = crate::starcatcher::Starcatcher::bought(&mut self.rng);
        s.shadow_y = self.rng.next_range(5) as i32 + 410;
        self.starcatchers.push(s);
        self.shop.reveal(Slot::Weapon, true);
        self.shop.reveal(Slot::EggPiece, true);
        self.play(0x122, Sound::Grow);
        let r = self.rng.next_range(3);
        self.sounds.push(Sound::splash(r).into());
        true
    }

    /// Slot 10: laser +1, cap 12 (`shop-bar.md#per-slot-behaviour`). No
    /// RNG. At 12 the original also shows "Hold down mouse button for Rapid
    /// Fire!" (static; not reached in a capture yet).
    fn buy_weapon(&mut self, price: i32) -> bool {
        if self.laser >= 12 || !self.try_spend(price) {
            return false;
        }
        self.laser += 1;
        if self.laser == 12 {
            self.show_message("Hold down mouse button for Rapid Fire!", false, -1);
        }
        if self.sample_ready(0x111, 3) {
            self.sounds.push(Sound::Buy.into());
        }
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
            self.show_message(
                "Upgrade Food Quantity to drop more food at once!",
                false,
                0xc,
            );
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
        if self.tank == 1 && self.max_food == 1 {
            self.show_message("Hold down mouse button to Auto-Feed!", false, -1);
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
        if self.tutorial && self.egg_count == 1 && self.message.idle() {
            self.show_message("Collect 2 more egg pieces to finish level!", false, 0xa);
        }
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
                    if self.message.idle() {
                        self.show_message("Your fish has grown! Good work!", false, 9);
                    }
                }
            }
            2 if first_level && !self.shop.revealed[Slot::EggPiece as usize] => {
                self.shop.reveal(Slot::EggPiece, true);
                self.arrow_buy_fish = false;
                self.arrow_buy_egg = true;
                if self.tutorial && self.message.idle() {
                    self.show_message("Buy 3 egg pieces to complete level!", false, 7);
                }
            }
            2 if self.tank == 1 && self.level == 2 => {
                if !self.shop.revealed[Slot::FoodQuality as usize] {
                    self.arrow_food_quality = true;
                    self.shop.reveal(Slot::FoodQuality, true);
                    self.show_message(
                        "Upgrade Food Quality to make food more nourishing!",
                        false,
                        0xb,
                    );
                }
            }
            2 if self.tank == 1 && self.level >= 3 => {
                self.shop.reveal(Slot::FoodQuality, true);
                self.shop.reveal(Slot::FoodQuantity, true);
                self.shop.reveal(Slot::Carnivore, true);
            }
            // Tank 2 (`0x004f1840`, shop-bar.md#reveal): quality, quantity,
            // star potion, then the egg on 2-1 or the Starcatcher after.
            2 if self.tank == 2 => {
                self.shop.reveal(Slot::FoodQuality, true);
                self.shop.reveal(Slot::FoodQuantity, true);
                self.shop.reveal(Slot::StarPotion, true);
                if self.level == 1 {
                    self.shop.reveal(Slot::EggPiece, true);
                } else {
                    self.shop.reveal(Slot::Starcatcher, true);
                }
            }
            // TODO(spec): tanks 3–4 reveal lists (`0x004f1840`).
            _ => {}
        }
    }

    /// `WinFishApp` dialog-button handler (vtable slot 3 @ `0x00552a8a`).
    fn dismiss_dialog(&mut self) {
        // Game over has no "continue": the board stays paused under it.
        // A new dialog's button is disabled for its first updates.
        if self.dialog == Some(Dialog::GameOver) || self.dialog_age < DIALOG_BUTTON_DELAY {
            return;
        }
        self.dialog_age = 0;
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

    /// The light bands drift by −1.6, +1.8 and +2.2 px per tick (f32), then
    /// wrap: past 640 → 0, below 0 → 640 (`Board::Update` @ `0x0054805d`;
    /// board.md#tank-lighting).
    fn move_lights(&mut self) {
        self.light_x[0] = (f64::from(self.light_x[0]) - 1.6) as f32;
        self.light_x[1] = (f64::from(self.light_x[1]) + 1.8) as f32;
        self.light_x[2] = (f64::from(self.light_x[2]) + 2.2) as f32;
        for x in &mut self.light_x {
            if *x > 640.0 {
                *x = 0.0;
            } else if *x < 0.0 {
                *x = 640.0;
            }
        }
    }

    /// Carnivore remove (slot 87 @ `0x004f2d30`): the 1-3 hint chain
    /// (carnivore.md#carnivore-remove), for starving and being eaten alike.
    fn carnivore_died(&mut self) {
        if (self.tank, self.level) != (1, 3) {
            return;
        }
        if self.hints[0x1c] {
            self.show_message("Try feeding small guppies to your carnivores!", false, 0x1d);
        }
        if self.hints[0x1b] {
            self.show_message("Hint: Carnivores won't eat fish food!", false, 0x1c);
        }
        self.show_message("Warning! Your carnivore has died!", false, 0x1b);
    }

    /// `Board::BuyCarnivore` (`0x00544c00`, carnivore.md#spawn): 12 draws.
    pub fn add_carnivore(&mut self) {
        let x = self.rng.next_range(520) as i32 + 20;
        let y = self.rng.next_range(265) as i32 + 105;
        let mut f = Fish::new_carnivore(x, y, &mut self.rng);
        f.vy = f64::from(self.rng.next_range(5) + 23);
        f.y = 40.0;
        f.widget_y = 40;
        f.vy_damp_timer = self.rng.next_range(10) as i32 + 45;
        self.carnivores.push(f);
        // Shadow ctor draw.
        self.rng.next_range(5);
    }

    /// One pet's update and what it asks the board to do.
    fn update_pet(&mut self, i: usize) {
        let mut out = PetOut::default();
        let fish: Vec<(i32, i32)> = if self.pets[i].id == crate::pet::RHUBARB {
            self.guppies
                .iter()
                .chain(&self.carnivores)
                .map(|f| (f.widget_x, f.widget_y))
                .collect()
        } else {
            Vec::new()
        };
        self.pets[i].update(
            self.has_aliens,
            &self.aliens,
            &self.coins,
            &fish,
            &mut self.rng,
            &mut out,
        );
        // Rhubarb's flings (`0x004e7e5a`): vy and the damp timer.
        let n = self.guppies.len();
        for &(f, vy) in &out.flings {
            let fish = if f < n {
                &mut self.guppies[f]
            } else {
                &mut self.carnivores[f - n]
            };
            fish.vy = vy;
            fish.vy_damp_timer = 50;
        }
        // TODO(spec): Rhubarb's snap click (`out.click`, SOUND_BUTTONCLICK
        // through the app, no board throttle) isn't in the sound list yet.
        // Rufus's hits (`0x004e7c30`): health only; the alien dies in its
        // own update.
        for &(a, damage) in &out.hits {
            self.aliens[a].health -= damage;
            self.play_throttled(0x130, 10, Sound::Punch);
        }
        if let Some(ci) = out.picked {
            // `0x004ddc40`: bank the coin, remove it, play its sound.
            let c = self.coins.remove(ci);
            self.money = (self.money + c.value() * c.multiplier).min(9_999_999);
            self.play_collect_sound(c.kind);
        }
        if let Some(sound) = out.sound {
            self.play(sound.sample_id(), sound);
        }
        if let Some((x, y)) = out.pearl {
            // The pearl list updates later this tick (Niko is first).
            self.pearls.push(Coin::new_pearl(x, y));
        }
    }

    /// A `Fish::Update` pass over guppies made during this tick's object
    /// pass (Prego's births), in order.
    fn update_newborns(&mut self, born: &[usize]) {
        let env = FishEnv {
            paused: self.paused,
            has_aliens: self.has_aliens,
            gus_first: self.aliens.first().is_some_and(|a| a.kind == 4),
        };
        let mut stage_ups = Vec::new();
        let mut new_dead = Vec::new();
        let mut noses = Vec::new();
        let mut world = World {
            foods: &mut self.foods,
            coins: &mut self.coins,
            rng: &mut self.rng,
            global: &mut self.global_rng,
            clock: self.clock,
            sample_last: &mut self.sample_last,
            sounds: &mut self.sounds,
            stage_ups: &mut stage_ups,
            message: &mut self.message,
            hints: &mut self.hints,
            tutorial: self.tutorial,
            dead_fish: &mut new_dead,
            guppies: &[],
            eaten: &mut Vec::new(),
            potions: &mut noses,
            star_hints: self.tank == 2 && self.level < 3,
            shots: &mut self.shots,
            artifacts: &mut self.artifacts,
            song: self.song,
            shelter: self.shelter,
            lure: self
                .fish_pets
                .iter()
                .find(|f| f.kind == crate::fish::FishKind::Gumbo)
                .map(|f| (f.widget_x, f.widget_y)),
        };
        for &i in born {
            self.guppies[i].update(env, &mut world);
        }
        for s in stage_ups {
            self.on_stage_up(s);
        }
        self.fish_pets.append(&mut noses);
        self.dead_fish.extend(new_dead);
    }

    /// One logic tick (28 ms).
    pub fn update(&mut self) {
        // `tick` mirrors the app's update counter, which runs while the
        // board is paused; the board clock (`+0x28`) does not.
        // The board clock `+0x28` (widget update count) runs while paused.
        self.clock += 1;
        if self.dialog.is_some() {
            self.dialog_age += 1;
        }
        if self.paused {
            self.tick += 1;
            return;
        }
        self.level_ticks += 1;
        // Level-tick-150 line (`Board::Update` @ `0x0054913b`, Adventure).
        if self.level_ticks == 150 {
            match (self.tank, self.level) {
                (1, 1) => self.show_message(
                    "Here are your first fish! Take good care of them!",
                    false,
                    1,
                ),
                (1, 2) => self.show_message(
                    "On this level you will meet your 1st alien opponent!",
                    false,
                    0x14,
                ),
                _ => {}
            }
        }
        if self.money_flash > 0 {
            self.money_flash -= 1;
        }
        // TODO(spec): the shake decrement (`0x0054786c`) runs earlier in
        // the update than the flash; no RNG either way.
        if self.shake != 0 {
            self.shake -= 1;
        }
        self.move_lights();
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
            self.play(0x110, Sound::Bubbles);
        }
        // After the ambient check: `+0x2c4`, then the wave countdown.
        self.update_waves();
        // Blip's reticle (pet-blip.md, board draw `0x0053ffaa`): while the
        // countdown is 1..225 the alien will appear at y 280. The original
        // does this in its draw; the sim applies it once per tick.
        if (1..=225).contains(&self.wave_countdown)
            && self.tank != 5
            && self
                .fish_pets
                .iter()
                .any(|f| f.kind == crate::fish::FishKind::Blip)
        {
            self.wave_a.1 = 280;
            if self.wave_kind == 11 {
                self.wave_b.1 = 280;
            }
            if self.wave_countdown == 225 {
                self.play(0x13c, Sound::Sonar);
            }
        }
        // Board::Update (`0x00548e29`, right after the waves and before the
        // objects update): no fish left → pause and raise the
        // tutorial's second-chance dialog, or game over. (Coins still on the
        // board are banked first in the original via `0x0053c2d0`; that only
        // matters once coin values on-screen are tracked separately.)
        // TODO(spec): other fish kinds and pets count as fish (`0x005392a0`).
        if self.guppies.is_empty()
            && self.carnivores.is_empty()
            && self.starcatchers.is_empty()
            && self.dialog.is_none()
        {
            self.paused = true;
            self.dialog = Some(if self.tutorial {
                Dialog::LastFishDied
            } else {
                Dialog::GameOver
            });
        }
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
        // Niko is put behind the board overlay, not brought to the front
        // with the other pets (`RebuildZOrder` @ `0x0053b604`): he is the
        // back-most object and updates first (pet-niko.md#update-order).
        for i in 0..self.pets.len() {
            if self.pets[i].id == NIKO {
                self.update_pet(i);
            }
        }
        // Dead fish come right after shadows (cosmetic, no RNG).
        // A corpse Angie revived (`DeadFish::Update` `0x004f64e0`): HEAL,
        // remove it, and re-create the fish, which updates this tick.
        let mut revived = Vec::new();
        self.dead_fish.retain_mut(|d| {
            let keep = d.update();
            if !keep && d.revive_due() {
                revived.push((d.widget_x, d.widget_y, d.kind, d.facing_right));
            }
            keep
        });
        let mut reborn = Vec::new();
        for (x, y, kind, right) in revived {
            self.play(0x125, Sound::Heal);
            if (0..=4).contains(&kind) {
                // `0x00546fc0`: `Fish::Init` (6 draws), the stage and facing,
                // then its shadow (1 draw).
                let mut f = Fish::new(x, y, &mut self.rng);
                f.stage = kind;
                f.vx = if right { 1.0 } else { -1.0 };
                f.facing = f.vx;
                f.announces = self.tutorial;
                self.guppies.push(f);
                self.rng.next_range(5);
                reborn.push(self.guppies.len() - 1);
            }
            // TODO(spec): other fish kinds (not on tank 1).
        }
        self.revived = reborn;
        // Meryl's notes (`+0xe8`, `Coin::Update` kind 16): rise 1 px a
        // tick; gone after 100 ticks or above y 50.
        self.notes.retain_mut(|n| {
            n.floor_time += 1;
            if n.floor_time > 100 || n.y < 50.0 {
                return false;
            }
            n.y -= 1.0;
            n.widget_y = n.y as i32;
            true
        });
        self.update_dead_aliens();
        // Warps sit between the coin sub-list and food in z-order.
        self.warps.retain_mut(Warp::update);
        self.update_starcatchers();
        let mut potion_money = 0;
        let mut i = 0;
        while i < self.foods.len() {
            match self.foods[i].update() {
                FoodStep::Keep => i += 1,
                FoodStep::Remove => {
                    self.foods.remove(i);
                }
                FoodStep::Bank => {
                    potion_money += 1;
                    self.foods.remove(i);
                }
                FoodStep::Explode => {
                    let f = self.foods.remove(i);
                    self.star_explosion(f.widget_x, f.widget_y);
                }
            }
        }
        self.money = (self.money + potion_money).min(9_999_999);
        let env = FishEnv {
            paused: self.paused,
            has_aliens: self.has_aliens,
            gus_first: self.aliens.first().is_some_and(|a| a.kind == 4),
        };
        let mut stage_ups = Vec::new();
        let mut new_dead = Vec::new();
        let mut noses = Vec::new();
        let mut world = World {
            foods: &mut self.foods,
            coins: &mut self.coins,
            rng: &mut self.rng,
            global: &mut self.global_rng,
            clock: self.clock,
            sample_last: &mut self.sample_last,
            sounds: &mut self.sounds,
            stage_ups: &mut stage_ups,
            message: &mut self.message,
            hints: &mut self.hints,
            tutorial: self.tutorial,
            dead_fish: &mut new_dead,
            guppies: &[],
            eaten: &mut Vec::new(),
            potions: &mut noses,
            star_hints: self.tank == 2 && self.level < 3,
            shots: &mut self.shots,
            artifacts: &mut self.artifacts,
            song: self.song,
            shelter: self.shelter,
            lure: self
                .fish_pets
                .iter()
                .find(|f| f.kind == crate::fish::FishKind::Gumbo)
                .map(|f| (f.widget_x, f.widget_y)),
        };
        // Guppies Angie revived this tick sit at the end of the widget
        // list: they update after the pets (with the newborns).
        let n_old = self.guppies.len() - self.revived.len();
        for f in &mut self.guppies[..n_old] {
            f.update(env, &mut world);
        }
        // Carnivores (`+0xa4`) right after the guppies. They read the guppy
        // list for prey; a guppy eaten here is gone for the next carnivore.
        let mut eaten = Vec::new();
        world.guppies = &self.guppies;
        world.eaten = &mut eaten;
        let mut carnivore_died = 0;
        for f in &mut self.carnivores {
            f.update(env, &mut world);
            if f.dead {
                carnivore_died += 1;
            }
        }
        for i in eaten {
            // Fish slot 87 (`true`): removed, no corpse; `board+0x488 += 1`.
            self.guppies[i].dead = true;
            self.guppies[i].eaten = true;
        }
        for s in stage_ups {
            self.on_stage_up(s);
        }
        // Noses made from guppies join `+0xb4` and update in this pass
        // (pet-nostradamus.md#the-potion-food-kind-2). If no fish is left,
        // a random guppy is added (`0x005392a0`).
        let made_nose = !noses.is_empty();
        self.fish_pets.append(&mut noses);
        if made_nose {
            let left = self.guppies.iter().filter(|f| !f.dead).count() + self.carnivores.len();
            if left == 0 {
                self.add_random_guppy();
            }
        }
        // Starved guppies leave a corpse (`Fish::Die` → `AddDeadFish`).
        // New corpses join the end of the update list, so they also update
        // this tick (confirmed: life 124 on the first snapshot).
        for mut d in new_dead {
            d.update();
            self.dead_fish.push(d);
        }
        // A fish leaving with a missile on it takes the missile with it
        // (`Fish` slot 87 releases its `+0x98`: the missile explodes).
        let orphans: Vec<u32> = self
            .guppies
            .iter()
            .chain(&self.carnivores)
            .filter(|f| f.dead)
            .filter_map(|f| f.missile)
            .collect();
        self.guppies.retain(|f| !f.dead);
        self.carnivores.retain(|f| !f.dead);
        for id in orphans {
            if let Some(m) = self.missiles.iter().position(|m| m.kind == 0 && m.id == id) {
                self.explode_missile(m);
            }
        }
        for _ in 0..carnivore_died {
            self.carnivore_died();
        }
        // Aliens (`+0xb8`, pass 1).
        let mut i = 0;
        while i < self.aliens.len() {
            let mut out = AlienOut::default();
            // Wadsworth's shelter hides small and medium guppies from every
            // alien (`0x00500ff0`). Indices stay those of the full list.
            let sheltered = |f: &Fish| self.shelter.is_some() && !f.is_carnivore() && f.stage < 2;
            // Starcatchers are prey too (types 5..10,
            // alien-sylvester.md#31-nearest-prey-0x004e8b60); the alien only
            // reads a prey's widget position, so they go in as stand-ins.
            let stand_ins: Vec<Fish> = self
                .starcatchers
                .iter()
                .map(|s| {
                    let mut f = Fish::new(s.widget_x, s.widget_y, &mut MtRand::new(0));
                    (f.widget_x, f.widget_y, f.stage) = (s.widget_x, s.widget_y, 5);
                    f
                })
                .collect();
            let prey: Vec<&Fish> = self
                .guppies
                .iter()
                .chain(&self.carnivores)
                .chain(&stand_ins)
                .collect();
            let prey_mask: Vec<bool> = prey.iter().map(|f| sheltered(f)).collect();
            // Replay-only: the original's address-order tie break
            // (`ArtifactPlan::eat_choice`); `None` in play.
            let prefer = self.artifacts.eat_choice;
            let foods: Vec<crate::alien::FoodView> = self
                .foods
                .iter()
                .map(|f| crate::alien::FoodView {
                    widget_x: f.widget_x,
                    widget_y: f.widget_y,
                    delay: f.delay,
                    kind: f.kind,
                    level: f.level,
                })
                .collect();
            self.aliens[i].update_with_food(
                &prey,
                &prey_mask,
                prefer,
                &foods,
                &mut self.rng,
                &mut out,
            );
            if let Some(points) = out.volley {
                let fired = self.fire_volley(i, points);
                self.aliens[i].finish_volley(fired, &mut self.rng);
                out.died = self.aliens[i].health <= 0.0;
            }
            if let Some(fi) = out.ate_food {
                // Gus ate a pellet: a star potion explodes first, then the
                // slurp, then the pellet is removed (alien-kind4.md#32).
                if self.foods[fi].level == 3 {
                    self.play(0x11d, Sound::Explode);
                    let (ix, iy) = (self.aliens[i].widget_x, self.aliens[i].widget_y);
                    let n = self.rng.next_range(3) + 1;
                    for _ in 0..n {
                        let k = self.rng.next_range(3) as i32 + 3;
                        let y = self.rng.next_range(40) as i32 + 10 + iy;
                        let x = self.rng.next_range(40) as i32 + 10 + ix;
                        self.add_effect_shot(x, y, k);
                        let k = self.rng.next_range(3) as i32 + 3;
                        let y = self.rng.next_range(20) as i32 + 20 + iy;
                        let x = self.rng.next_range(20) as i32 + 20 + ix;
                        self.add_effect_shot(x, y, k);
                    }
                }
                if self.sample_ready(0x138, 3) {
                    let s = Sound::slurp(self.rng.next_range(5));
                    self.sounds.push(s.into());
                }
                self.foods.remove(fi);
            }
            if let Some(g) = out.ate {
                let (ng, nc) = (self.guppies.len(), self.carnivores.len());
                let victim = if g < ng {
                    self.guppies.remove(g)
                } else if g < ng + nc {
                    let c = self.carnivores.remove(g - ng);
                    self.carnivore_died();
                    c
                } else {
                    // A Starcatcher eaten (slot 75 → remove(true): no
                    // corpse; messages only if it was starving).
                    let s = self.starcatchers.remove(g - ng - nc);
                    if s.hunger <= 0 && (self.tank, self.level) == (2, 2) {
                        self.starcatcher_died_messages();
                    }
                    stand_ins[g - ng - nc].clone()
                };
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
                // A corpse made during the object pass joins the end of the
                // update list and updates this tick (`tank1-4_full_01`: life
                // 124 on the kill tick when Itchy's stab finished the alien).
                let n = self.dead_aliens.len();
                self.kill_alien(i);
                self.update_dead_aliens_from(n);
                continue;
            }
            i += 1;
        }
        // Pets (`+0xb0`) update after aliens and before coins (z-order).
        for i in 0..self.pets.len() {
            if self.pets[i].id != NIKO {
                self.update_pet(i);
            }
        }
        // Fish-shaped pets (`+0xb4`) right after (pet-itchy.md#update-order).
        // Guppies born this tick: they join the end of the widget list and
        // update after the pets in the same pass (`tank1-5_full_01`: a
        // newborn's hunger is already one down on its first snapshot).
        let mut born = Vec::new();
        let mut i = 0;
        while i < self.fish_pets.len() {
            if self.paused {
                break;
            }
            // Zorf's hungry check (`0x004e5b8e`): guppies, then breeders
            // (none yet), with hunger below 300.
            let hungry = self.guppies.iter().any(|g| g.hunger < 300);
            let hidden = self
                .shop
                .position
                .iter()
                .zip(&self.shop.revealed)
                .any(|(p, &r)| p.is_some() && !r);
            let balled: Vec<usize> = self.missiles.iter().filter_map(|m| m.target).collect();
            let ctx = crate::itchy::PetCtx {
                aliens: &self.aliens,
                hungry,
                guppies: &self.guppies,
                coins: &self.coins,
                foods: &self.foods,
                reveal_due: hidden && self.level_ticks > 200,
                corpses: &self.dead_fish,
                brinkley: self
                    .fish_pets
                    .iter()
                    .any(|f| f.kind == crate::fish::FishKind::Brinkley),
                balled: &balled,
            };
            // The gate publishes the position from before his move.
            let (pre_wx, pre_wy) = (self.fish_pets[i].widget_x, self.fish_pets[i].widget_y);
            let out = self.fish_pets[i].update_fish_pet(ctx, &mut self.rng);
            if out.revert {
                // Slot 73 with 19: Presto is added at the end of `+0xb4`
                // (he updates later in this pass) and this pet is removed.
                self.presto_revert(i);
                continue;
            }
            if let Some((x, y, motion)) = out.food {
                // Food made in the object pass updates in it (board.md).
                let mut f = crate::food::Food::new_thrown(x, y, motion, &mut self.rng);
                f.update();
                self.foods.push(f);
            }
            match out.shelter {
                Some(true) => self.shelter = Some((pre_wx, pre_wy)),
                Some(false) => {
                    // Release: two bubbles (global stream, not modelled).
                    self.shelter = None;
                }
                None => {}
            }
            if let Some(on) = out.song {
                self.song = on;
                if on {
                    let (wx, wy) = (self.fish_pets[i].widget_x, self.fish_pets[i].widget_y);
                    self.notes.push(Coin::new(wx + 15, wy - 5, 16));
                    self.play(0x137, Sound::Sing);
                }
            }
            if let Some(a) = out.stabbed {
                crate::itchy::stab(&mut self.aliens[a]);
                // `0x00538340(11)`: PUNCH, throttled to one per 11 ticks.
                self.play_throttled(0x130, 11, Sound::Punch);
            }
            if let Some((x, y)) = out.coin {
                // Coins come after `+0xb4` in z-order: it updates this tick.
                self.coins.push(Coin::new(x, y, crate::coin::kind::GOLD));
            }
            if let Some(a) = out.bite {
                // Gash (`0x004fb5b0`): 3 HP (0.5 for kinds 4/5), PUNCH gap
                // 9; at 0 the alien dies on the spot.
                let dmg = if matches!(self.aliens[a].kind, 4 | 5) {
                    0.5
                } else {
                    3.0
                };
                self.aliens[a].health -= dmg;
                if self.aliens[a].health <= 0.0 {
                    let n = self.dead_aliens.len();
                    self.kill_alien(a);
                    self.update_dead_aliens_from(n);
                }
                self.play_throttled(0x130, 9, Sound::Punch);
            }
            if let Some(g) = out.swallow {
                // Gash swallows a guppy: removed, no corpse; the chomp
                // sound draws `r % 3` unless throttled.
                self.guppies.remove(g);
                if self.sample_ready(0x113, 3) {
                    let s = if self.rng.next_range(3) == 0 {
                        Sound::Chomp2
                    } else {
                        Sound::Chomp
                    };
                    self.sounds.push(s.into());
                }
            }
            if let Some(c) = out.revive
                && self.dead_fish[c].turn == 100
            {
                // Angie (`0x004d5630`): the countdown starts.
                self.dead_fish[c].turn = 10;
            }
            if out.reveal_shop {
                // Blip (`0x004f7b01`): SONAR, then every slot revealed.
                self.play(0x13c, Sound::Sonar);
                for slot in Slot::ALL {
                    self.shop.reveal(slot, true);
                }
            }
            if let Some((ci, y)) = out.toss_coin {
                // Nimbus: the coin is replaced by its rising kind.
                let c = self.coins.remove(ci);
                let kind = if c.kind <= 7 { c.kind + 7 } else { c.kind };
                self.coins.push(Coin::new(c.widget_x, y, kind));
            }
            if let Some((fi, y)) = out.toss_food {
                // Nimbus: the pellet is removed and dropped again, tossed
                // (the normal drop path: limit check, 1 draw, DROPFOOD).
                let f = self.foods.remove(fi);
                self.add_tossed_food(f.widget_x, y, f.level);
            }
            if out.charged {
                self.play(0x11a, Sound::Eel2);
            }
            if let Some(fi) = out.eat {
                // Brinkley (`0x004fe740`, pet-brinkley.md#eat): slurp (one
                // draw unless throttled), the pellet goes, every 3rd is a
                // rising coin. Coins come after `+0xb4`: it updates this tick.
                if self.sample_ready(0x138, 3) {
                    let s = Sound::slurp(self.rng.next_range(5));
                    self.sounds.push(s.into());
                }
                self.foods.remove(fi);
                if let Some((x, y, kind)) = self.fish_pets[i].brinkley_ate(pre_wx, pre_wy) {
                    self.coins.push(Coin::new(x, y, kind));
                }
            }
            if let Some((x, y)) = out.potion {
                // Straight onto `+0xac` (no AddFood limit or sound). Made in
                // the object pass, it joins the end of the widget list and
                // updates this tick (`nostradamus_01`).
                let mut f = crate::food::Food::new_potion(x, y, &mut self.rng);
                f.update();
                self.foods.push(f);
            }
            if let Some((x, y, t)) = out.ball {
                // `0x00544130`: onto `+0xdc` (after `+0xb4` in z-order, so it
                // updates this tick), the alien's `+0x98` set; GROW gap 3.
                self.missiles.push(crate::missile::Missile::new_ball(
                    x,
                    y,
                    t,
                    &mut self.global_rng,
                ));
                self.play(0x122, Sound::Grow);
            }
            if let Some((px, py)) = out.fist {
                self.glove_hits(i, px, py);
            }
            if let Some((x, y)) = out.bomb {
                self.coins.push(Coin::new(x, y, crate::coin::kind::BOMB));
                self.play(0x143, Sound::Unleash);
            }
            if self.fish_pets[i].is_prego() {
                // Slot 81, id 3: the newborn (`AddGuppyAt`, pet-prego.md),
                // then the birth sound (`0x005382c0`: BABY by default).
                if let Some((x, y)) = out.birth {
                    self.add_guppy_at(x, y);
                    match self.prego_sound {
                        PregoSound::Baby => self.play(0x108, Sound::Baby),
                        PregoSound::Fart => self.play(0x120, Sound::Fart),
                        PregoSound::Fart2 => self.play(0x121, Sound::Fart2),
                    }
                    born.push(self.guppies.len() - 1);
                }
                let n = self.guppies.len();
                self.fish_pets[i].finish_prego(out.birth.is_some(), n);
            }
            i += 1;
        }
        self.update_missiles();
        // Revived guppies are the last ones before this tick's births.
        let n_rev = std::mem::take(&mut self.revived).len();
        let first_born = born.first().copied().unwrap_or(self.guppies.len());
        let mut late: Vec<usize> = (first_born.saturating_sub(n_rev)..first_born).collect();
        late.extend(born);
        if !late.is_empty() {
            self.update_newborns(&late);
        }
        self.shots.retain_mut(Shot::update);
        let mut banked = 0;
        let long_rest = self.tutorial;
        let seymour = self
            .fish_pets
            .iter()
            .any(|f| f.kind == crate::fish::FishKind::Seymour);
        // Coins in list order; a bomb (kind 0x11) can kill a fish and
        // explode inside its update (pet-shrapnel.md#coinupdate-0x004f5050-kind-0x11-normal-game).
        let mut i = 0;
        while i < self.coins.len() {
            let step = self.coins[i].update_with(long_rest, seymour);
            match step {
                CoinStep::Keep => {
                    let c = &self.coins[i];
                    if c.kind == crate::coin::kind::BOMB
                        && !c.collected
                        && c.age >= 31
                        && let Some(victim) = self.bomb_victim(c.x + 36.0, c.y + 36.0)
                    {
                        self.bomb_kill(victim);
                        let c = self.coins.remove(i);
                        self.explode_bomb(&c);
                        let s = self.rng.next_range(3) as i32 + 3;
                        self.add_effect_shot(c.x as i32, c.y as i32, s);
                        continue;
                    }
                    i += 1;
                }
                CoinStep::Remove => {
                    self.coins.remove(i);
                }
                CoinStep::Explode => {
                    let c = self.coins.remove(i);
                    self.explode_bomb(&c);
                }
                CoinStep::Bank(v) => {
                    banked += v;
                    self.coins.remove(i);
                }
            }
        }
        self.guppies.retain(|f| !f.dead);
        self.carnivores.retain(|f| !f.dead);
        self.pearls.retain_mut(|c| match c.update(long_rest) {
            CoinStep::Keep => true,
            CoinStep::Remove => false,
            CoinStep::Bank(v) => {
                banked += v;
                false
            }
            CoinStep::Explode => false,
        });
        // `0x0053c1e0`: add money, capped at 9,999,999.
        self.money = (self.money + banked).min(9_999_999);
        // The message widget updates after the board and its objects (it
        // is later in the widget list), so a message shown this tick counts
        // down once already (captures show 184 on the first tick).
        self.message.update();
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
    fn walter_punches_a_guppy_and_tires_after_five() {
        let mut b = Board::new(MtRand::new(1), MtRand::new(2));
        b.add_pet(crate::itchy::WALTER);
        b.add_random_guppy();
        let w = 0;
        // Facing right, a guppy just right of him, both at rest.
        b.fish_pets[w].vx = 0.0;
        b.fish_pets[w].facing = 1.0;
        let (wx, wy) = (b.fish_pets[w].widget_x, b.fish_pets[w].widget_y);
        let g = &mut b.guppies[0];
        (g.x, g.y, g.vy, g.vy_damp_timer) = (f64::from(wx + 40), f64::from(wy), 0.0, 0);
        (g.widget_x, g.widget_y) = (wx + 40, wy);
        b.step(&[Input::Click {
            x: wx + 40,
            y: wy + 40,
        }]);
        assert!(b.fish_pets[w].glove.is_some());
        for _ in 0..40 {
            b.step(&[]);
        }
        assert!(b.fish_pets[w].glove.is_none());
        assert!(
            b.guppies[0].push_timer > 0 || b.guppies[0].push != 0.0,
            "guppy knocked"
        );
        for _ in 0..4 {
            let (wx, wy) = (b.fish_pets[w].widget_x, b.fish_pets[w].widget_y);
            b.fish_pets[w].turn_timer = 0;
            b.step(&[Input::Click {
                x: wx + 40,
                y: wy + 40,
            }]);
            for _ in 0..41 {
                b.step(&[]);
            }
        }
        assert!(b.fish_pets[w].tired > 0 && b.fish_pets[w].punches == 0);
    }

    #[test]
    fn presto_changes_into_an_owned_pet_and_recharges() {
        use crate::presto::{PRESTO, button_pos};
        let mut b = Board::new(MtRand::new(1), MtRand::new(2));
        b.owned[PRESTO as usize] = true;
        b.owned[crate::itchy::VERT as usize] = true;
        b.add_pet(PRESTO);
        b.add_random_guppy();
        let p = &b.fish_pets[0];
        assert!(p.presto && p.recharge == 0);
        let (x, y) = (p.widget_x + 40, p.widget_y + 40);
        b.step(&[Input::RightClick { x, y }]);
        assert!(b.pet_dialog.is_some());
        // The board keeps running under the dialog.
        let clock = b.clock;
        b.step(&[]);
        assert!(!b.paused && b.clock == clock + 1);
        let (bx, by) = button_pos(crate::itchy::VERT as usize);
        let draws = b.rng.state().1;
        b.step(&[
            Input::Click {
                x: bx + 5,
                y: by + 5,
            },
            Input::Release {
                x: bx + 5,
                y: by + 5,
            },
        ]);
        assert!(b.pet_dialog.is_none());
        assert_eq!(b.fish_pets.len(), 1);
        let v = &b.fish_pets[0];
        assert!(v.is_vert() && v.presto);
        // `Fish::Init`'s 6 draws and the shadow's, no position draws.
        assert_ne!(b.rng.state().1, draws);
        assert_eq!(v.recharge, crate::presto::RECHARGE - 1);
        // Too early: BUZZER and "Recharging...", no dialog.
        let (x, y) = (v.widget_x + 40, v.widget_y + 40);
        b.step(&[Input::RightClick { x, y }]);
        assert!(b.pet_dialog.is_none());
        assert_eq!(b.pearls.last().and_then(|n| n.text), Some("Recharging..."));
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
        // The button ignores clicks until the dialog has updated 30 times.
        b.step(&[Input::DismissDialog]);
        assert!(b.dialog.is_some());
        while b.dialog_age < DIALOG_BUTTON_DELAY {
            b.step(&[]);
        }
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
