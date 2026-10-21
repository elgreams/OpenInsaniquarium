//! The shop/control bar. Spec: `docs/re/sim/shop-bar.md`.
//!
//! Only what tank 1 uses is modelled: guppy, food upgrades, weapon and egg
//! piece slots. Other slots exist in the table so level setup is complete.

/// What a slot sells (the slot index is the original's button id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Guppy = 0,
    Breeder = 1,
    FoodQuality = 2,
    FoodQuantity = 3,
    Carnivore = 4,
    StarPotion = 5,
    Starcatcher = 6,
    Guppycruncher = 7,
    Beetlemuncher = 8,
    Ultravore = 9,
    Weapon = 10,
    EggPiece = 11,
}

pub const SLOTS: usize = 12;
/// Button x positions on the bar (`RevealSlotButton`); all 58×60 at y = 3.
pub const BUTTON_X: [i32; 7] = [18, 87, 144, 217, 290, 363, 436];
pub const BUTTON_Y: i32 = 3;
pub const BUTTON_W: i32 = 58;
pub const BUTTON_H: i32 = 60;
/// Egg counter value at which the level completes.
pub const EGG_DONE: i32 = 4;

impl Slot {
    pub const ALL: [Slot; SLOTS] = [
        Slot::Guppy,
        Slot::Breeder,
        Slot::FoodQuality,
        Slot::FoodQuantity,
        Slot::Carnivore,
        Slot::StarPotion,
        Slot::Starcatcher,
        Slot::Guppycruncher,
        Slot::Beetlemuncher,
        Slot::Ultravore,
        Slot::Weapon,
        Slot::EggPiece,
    ];
}

/// Per-level shop state (`board+0x2e4..+0x404`).
#[derive(Debug, Clone, PartialEq)]
pub struct Shop {
    /// Bar position per slot, or `None` if unused on this level.
    pub position: [Option<usize>; SLOTS],
    pub price: [i32; SLOTS],
    /// Button shown (it appears on stage-up triggers, with a pop-in).
    pub revealed: [bool; SLOTS],
    /// Pop-in countdown per slot (8 → 0, one per running tick).
    pub pop: [i32; SLOTS],
}

impl Shop {
    /// `Board::SetupShop` (`0x00541200`), Adventure. No RNG on tank 1.
    /// TODO(spec): tanks 2–4 layouts and their level-2+ RNG draw.
    pub fn adventure(tank: i32, level: i32) -> Self {
        let mut s = Shop {
            position: [None; SLOTS],
            price: [0; SLOTS],
            revealed: [false; SLOTS],
            pop: [0; SLOTS],
        };
        if tank == 1 {
            let mut set = |slot: Slot, pos: usize, price: i32| {
                s.position[slot as usize] = Some(pos);
                s.price[slot as usize] = price;
            };
            set(Slot::Guppy, 0, 100);
            set(Slot::FoodQuality, 1, 200);
            set(Slot::FoodQuantity, 2, 300);
            set(Slot::Carnivore, 3, 1000);
            set(Slot::Weapon, 5, 1000);
            let egg = match level {
                1 => 150,
                2 => 500,
                3 => 2000,
                4 => 3000,
                _ => 5000,
            };
            set(Slot::EggPiece, 6, egg);
            let unused: &[Slot] = match level {
                1 => &[
                    Slot::FoodQuality,
                    Slot::FoodQuantity,
                    Slot::Carnivore,
                    Slot::Weapon,
                ],
                2 => &[Slot::Carnivore, Slot::Weapon],
                _ => &[],
            };
            for &u in unused {
                s.position[u as usize] = None;
            }
        }
        s
    }

    /// `RevealSlotButton` (`0x005409b0`): show the button if the slot is used.
    pub fn reveal(&mut self, slot: Slot, pop: bool) {
        let i = slot as usize;
        if self.position[i].is_some() && !self.revealed[i] {
            self.revealed[i] = true;
            self.pop[i] = if pop { 8 } else { 0 };
        }
    }

    /// The revealed slot whose button rect contains (`x`, `y`).
    pub fn hit(&self, x: i32, y: i32) -> Option<Slot> {
        Slot::ALL.into_iter().find(|&s| {
            let i = s as usize;
            self.revealed[i]
                && self.position[i].is_some_and(|p| {
                    let bx = BUTTON_X[p];
                    (bx..bx + BUTTON_W).contains(&x) && (BUTTON_Y..BUTTON_Y + BUTTON_H).contains(&y)
                })
        })
    }

    /// `MenuButtonWidget::Update`: pop-in countdowns (skipped while paused).
    pub fn tick(&mut self) {
        for p in &mut self.pop {
            if *p > 0 {
                *p -= 1;
            }
        }
    }
}
