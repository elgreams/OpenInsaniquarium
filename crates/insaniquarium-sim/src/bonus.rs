//! The bonus screen after each tank's bonus round (`Sexy::BonusScreen`).
//! Spec: `docs/re/screens/bonus-screen.md`. Sells bonus pets 20–23 in a
//! fixed order, then two pets-per-level upgrades. **No RNG**: the app and
//! global streams pass through untouched.

use crate::profile::Profile;

/// What the bonus screen offers at purchase count `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// Own pet `20 + n` (Brinkley, Nostradamus, Stanley, Walter).
    Pet(i32),
    /// Raise `pet_slots` to at least this value.
    Upgrade(i32),
    /// Nothing left to buy (`n >= 6`).
    None,
}

impl Offer {
    pub fn for_purchase(n: i32) -> Self {
        match n {
            0..=3 => Offer::Pet(20 + n),
            4 => Offer::Upgrade(4),
            5 => Offer::Upgrade(7),
            _ => Offer::None,
        }
    }
}

/// Price in shells for bonus purchase `n` (`bonus-screen.md#the-offer`).
/// `min((n + 4) * 5000, 50_000)`, or 0 when `n >= 6`.
pub fn bonus_price(n: i32) -> i32 {
    if n >= 6 { 0 } else { ((n + 4) * 5000).min(50_000) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BonusScreen {
    /// Widget update counter (`+0x28`), incremented each tick.
    pub tick: i32,
    /// Price in shells (`+0xd0`); 0 when nothing is for sale.
    pub price: i32,
    /// What's being offered this visit.
    pub offer: Offer,
    /// Animation counter (`+0xd4`): set to 1 on buy; mouse-down skip
    /// jumps it to 140 when `0 < anim < 140`.
    pub anim: i32,
    /// A purchase was made this visit (`+0xe0`).
    pub purchased: bool,
}

/// Inputs to the bonus screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusInput {
    /// Continue button (id 0).
    Continue,
    /// Menu button (id 1).
    Menu,
    /// Buy confirmed (dialog 0x23 → BUY).
    Buy,
    /// Mouse held down on the screen (skips the count-up animation).
    Held,
}

/// What a bonus-screen tick asks the game to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BonusOutcome {
    #[default]
    Stay,
    /// Continue pressed → `App::NextLevel(1, 1)`.
    NextLevel,
    /// Menu pressed → save + main menu.
    Menu,
}

impl BonusScreen {
    /// `BonusScreen` ctor + offer setup (`0x00516f70`).
    pub fn new(profile: &Profile) -> Self {
        let n = profile.bonus_purchases;
        BonusScreen {
            tick: 0,
            price: bonus_price(n),
            offer: Offer::for_purchase(n),
            anim: 0,
            purchased: false,
        }
    }

    /// `BonusScreen::Update` (`0x00517090`). No RNG.
    pub fn update(&mut self, held: bool) {
        self.tick += 1;
        // Mouse-down skip: held and 0 < anim < 140 → anim = 140.
        if held && self.anim > 0 && self.anim < 140 {
            self.anim = 140;
        }
    }

    /// Execute a buy: deducts shells, owns the pet or raises pet_slots.
    /// Returns `true` on success (`bonus-screen.md#buying`).
    pub fn buy(&mut self, profile: &mut Profile) -> bool {
        if self.purchased || self.price == 0 || profile.shells < self.price {
            return false;
        }
        match self.offer {
            Offer::Pet(id) => {
                if (0..24).contains(&id) {
                    profile.pets[id as usize] = true;
                }
            }
            Offer::Upgrade(slots) => {
                profile.pet_slots = profile.pet_slots.max(slots);
            }
            Offer::None => return false,
        }
        profile.shells = (profile.shells - self.price).clamp(0, 9_999_999);
        profile.bonus_purchases += 1;
        self.anim = 1;
        self.purchased = true;
        true
    }

    /// Can the player afford what's on offer?
    pub fn can_afford(&self, profile: &Profile) -> bool {
        self.price > 0 && !self.purchased && profile.shells >= self.price
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_sequence() {
        assert_eq!(Offer::for_purchase(0), Offer::Pet(20));
        assert_eq!(Offer::for_purchase(1), Offer::Pet(21));
        assert_eq!(Offer::for_purchase(2), Offer::Pet(22));
        assert_eq!(Offer::for_purchase(3), Offer::Pet(23));
        assert_eq!(Offer::for_purchase(4), Offer::Upgrade(4));
        assert_eq!(Offer::for_purchase(5), Offer::Upgrade(7));
        assert_eq!(Offer::for_purchase(6), Offer::None);
    }

    #[test]
    fn price_sequence() {
        assert_eq!(bonus_price(0), 20_000);
        assert_eq!(bonus_price(1), 25_000);
        assert_eq!(bonus_price(2), 30_000);
        assert_eq!(bonus_price(3), 35_000);
        assert_eq!(bonus_price(4), 40_000);
        assert_eq!(bonus_price(5), 45_000);
        assert_eq!(bonus_price(6), 0);
    }

    #[test]
    fn buy_pet_brinkley() {
        let mut p = Profile {
            shells: 25_000,
            ..Profile::default()
        };
        let mut s = BonusScreen::new(&p);
        assert_eq!(s.offer, Offer::Pet(20));
        assert_eq!(s.price, 20_000);
        assert!(s.can_afford(&p));
        assert!(s.buy(&mut p));
        assert!(p.pets[20]);
        assert_eq!(p.shells, 5_000);
        assert_eq!(p.bonus_purchases, 1);
        assert!(s.purchased);
        assert_eq!(s.anim, 1);
        // Can't buy twice.
        assert!(!s.buy(&mut p));
    }

    #[test]
    fn buy_upgrade_raises_pet_slots() {
        let mut p = Profile {
            shells: 100_000,
            bonus_purchases: 4,
            ..Profile::default()
        };
        let mut s = BonusScreen::new(&p);
        assert_eq!(s.offer, Offer::Upgrade(4));
        assert_eq!(s.price, 40_000);
        assert!(s.buy(&mut p));
        assert_eq!(p.pet_slots, 4);
        assert_eq!(p.bonus_purchases, 5);
        // Next visit: upgrade to 7.
        let mut s2 = BonusScreen::new(&p);
        assert_eq!(s2.offer, Offer::Upgrade(7));
        assert!(s2.buy(&mut p));
        assert_eq!(p.pet_slots, 7);
    }

    #[test]
    fn not_enough_shells() {
        let mut p = Profile {
            shells: 10_000,
            ..Profile::default()
        };
        let mut s = BonusScreen::new(&p);
        assert!(!s.can_afford(&p));
        assert!(!s.buy(&mut p));
        assert!(!s.purchased);
    }

    #[test]
    fn nothing_for_sale_after_six_purchases() {
        let p = Profile {
            shells: 999_999,
            bonus_purchases: 6,
            ..Profile::default()
        };
        let s = BonusScreen::new(&p);
        assert_eq!(s.offer, Offer::None);
        assert_eq!(s.price, 0);
    }

    #[test]
    fn mouse_skip_animation() {
        let mut p = Profile {
            shells: 50_000,
            ..Profile::default()
        };
        let mut s = BonusScreen::new(&p);
        s.buy(&mut p);
        assert_eq!(s.anim, 1);
        s.update(true);
        assert_eq!(s.anim, 140, "skip to 140");
    }

    #[test]
    fn no_rng_consumed() {
        // The spec says Update makes no RNG calls. Verify there's no
        // RNG parameter on update at all.
        let p = Profile::default();
        let mut s = BonusScreen::new(&p);
        for _ in 0..200 {
            s.update(false);
        }
        assert_eq!(s.tick, 200);
    }
}
