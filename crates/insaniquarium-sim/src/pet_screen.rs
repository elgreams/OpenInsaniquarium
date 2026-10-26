//! The pet screen ("Choose Your Pets", `Sexy::PetsScreen`): pick up to 3
//! owned pets for the next level. Spec: `docs/re/screens/pet-screen.md`.
//! No RNG anywhere on this screen.

use crate::profile::{PET_COUNT, Profile};

/// Ticks a not-chosen pet's info lingers after the mouse leaves it.
const FADE: i32 = 7;
/// Continue goes straight on with at least this many pets chosen.
const ENOUGH: i32 = 3;

/// Pet-screen input, in the order the original handles them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetInput {
    /// The mouse entered pet `id`'s bubble.
    Enter(i32),
    /// The mouse left pet `id`'s bubble.
    Leave(i32),
    /// A click (mouse-up) on pet `id`'s bubble.
    Toggle(i32),
    /// Continue (mouse-up on button 99).
    Continue,
    /// "Select More Pets?" answered Yes / No.
    ConfirmYes,
    ConfirmNo,
    /// Menu (button 100).
    Menu,
}

/// What a pet-screen step asks the game to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PetOutcome {
    #[default]
    Stay,
    /// Start the next level with the profile's chosen pets.
    StartLevel,
    /// Back to the main menu.
    Menu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetScreen {
    /// Pets owned (disabled bubbles are the rest).
    pub owned: [bool; PET_COUNT],
    /// Each bubble's chosen flag.
    pub chosen: [bool; PET_COUNT],
    /// `+0x120`
    pub ticks: i32,
    /// `+0x124`
    pub count: i32,
    /// `+0x128`, `+0x12c`: −1 = none.
    pub shown: i32,
    pub fallback: i32,
    /// `+0x130`
    pub max: i32,
    /// `+0x134`
    pub fade: i32,
    /// Dialog 0x14 "Select More Pets?" is open.
    pub confirm_open: bool,
}

impl PetScreen {
    /// `PetsScreen::PetsScreen` (`0x00527820`), Adventure. The caller clears
    /// the profile's chosen pets first (`NextLevel`).
    pub fn new(profile: &Profile) -> Self {
        let owned_count = profile.pet_count() as i32;
        PetScreen {
            owned: profile.pets,
            chosen: [false; PET_COUNT],
            ticks: 0,
            count: 0,
            shown: -1,
            fallback: -1,
            max: profile.pet_slots.min(owned_count).min(4),
            fade: 0,
            confirm_open: false,
        }
    }

    /// Inputs (mouse handlers run between ticks), then one Update.
    pub fn step(&mut self, inputs: &[PetInput], profile: &mut Profile) -> PetOutcome {
        for &i in inputs {
            let out = self.input(i, profile);
            if out != PetOutcome::Stay {
                return out;
            }
        }
        // `PetsScreen::Update` (`0x00518b00`).
        if self.fade != 0 {
            self.fade -= 1;
            if self.fade == 0 {
                self.shown = self.fallback;
            }
        }
        self.ticks += 1;
        PetOutcome::Stay
    }

    fn input(&mut self, i: PetInput, profile: &mut Profile) -> PetOutcome {
        // The dialog is modal: only its buttons answer.
        if self.confirm_open {
            return match i {
                PetInput::ConfirmYes => {
                    self.confirm_open = false;
                    PetOutcome::StartLevel
                }
                PetInput::ConfirmNo => {
                    self.confirm_open = false;
                    PetOutcome::Stay
                }
                _ => PetOutcome::Stay,
            };
        }
        let pet = |id: i32| (0..PET_COUNT as i32).contains(&id).then_some(id as usize);
        match i {
            PetInput::Enter(id) => {
                // The handler doesn't check for disabled (unowned) bubbles;
                // the widget manager doesn't send them mouse events.
                if pet(id).is_some_and(|p| self.owned[p]) {
                    self.shown = id;
                    self.fade = 0;
                }
            }
            PetInput::Leave(id) => {
                if pet(id).is_some_and(|p| self.owned[p] && !self.chosen[p]) {
                    self.fade = FADE;
                }
            }
            PetInput::Toggle(id) => {
                if let Some(p) = pet(id).filter(|&p| self.owned[p]) {
                    if self.chosen[p] {
                        self.chosen[p] = false;
                        self.count -= 1;
                    } else if self.count < self.max {
                        self.chosen[p] = true;
                        self.count += 1;
                    }
                }
            }
            PetInput::Continue => {
                // Written before the count check, dialog or not.
                profile.chosen = self.chosen;
                if self.count >= ENOUGH {
                    return PetOutcome::StartLevel;
                }
                self.confirm_open = true;
            }
            PetInput::Menu => return PetOutcome::Menu,
            PetInput::ConfirmYes | PetInput::ConfirmNo => {}
        }
        PetOutcome::Stay
    }

    /// The prompt's first line (`pet-screen.md#layout-and-drawing`).
    pub fn prompt(&self) -> Option<String> {
        match self.max - self.count {
            n if n > 1 => Some(format!("Choose {n} more pets")),
            1 => Some("Choose 1 more pet".into()),
            _ => None,
        }
    }
}

/// Bubble rect for pet `id` (90×83).
pub fn bubble(id: i32) -> (i32, i32, i32, i32) {
    let (x, row) = match id {
        0..=4 => (25, id),
        5..=9 => (119, id - 5),
        10..=14 => (425, id - 10),
        15..=19 => (519, id - 15),
        20 | 21 => (226, id - 17),
        _ => (323, id - 19),
    };
    (x, 41 + 83 * row, 90, 83)
}

/// The bubble under (x, y), if any.
pub fn bubble_at(x: i32, y: i32) -> Option<i32> {
    (0..PET_COUNT as i32).find(|&id| {
        let (bx, by, w, h) = bubble(id);
        (bx..bx + w).contains(&x) && (by..by + h).contains(&y)
    })
}

/// Continue and Menu button rects.
pub const CONTINUE: (i32, i32, i32, i32) = (225, 250, 186, 29);
pub const MENU: (i32, i32, i32, i32) = (525, 4, 80, 29);

#[cfg(test)]
mod tests {
    use super::*;

    fn five_pets() -> Profile {
        let mut p = Profile {
            tank: 2,
            level: 1,
            ..Profile::default()
        };
        for id in 0..5 {
            p.pets[id] = true;
        }
        p
    }

    #[test]
    fn three_picks_then_continue() {
        let mut p = five_pets();
        let mut s = PetScreen::new(&p);
        assert_eq!(s.max, 3);
        for id in [4, 2, 0, 3] {
            s.step(&[PetInput::Toggle(id)], &mut p);
        }
        assert_eq!(s.count, 3, "a 4th pick is refused");
        assert!(!s.chosen[3]);
        s.step(&[PetInput::Toggle(2), PetInput::Toggle(3)], &mut p);
        assert_eq!(
            s.step(&[PetInput::Continue], &mut p),
            PetOutcome::StartLevel
        );
        assert_eq!(p.level_pets(), vec![0, 3, 4]);
    }

    #[test]
    fn continue_short_asks_first() {
        let mut p = five_pets();
        let mut s = PetScreen::new(&p);
        assert_eq!(s.step(&[PetInput::Continue], &mut p), PetOutcome::Stay);
        assert!(s.confirm_open);
        s.step(&[PetInput::Toggle(0)], &mut p);
        assert_eq!(s.count, 0, "modal");
        s.step(&[PetInput::ConfirmNo], &mut p);
        assert!(!s.confirm_open);
        s.step(&[PetInput::Continue], &mut p);
        assert_eq!(
            s.step(&[PetInput::ConfirmYes], &mut p),
            PetOutcome::StartLevel
        );
        assert!(p.level_pets().is_empty());
    }

    #[test]
    fn info_fades_back_after_leaving() {
        let mut p = five_pets();
        let mut s = PetScreen::new(&p);
        s.step(&[PetInput::Enter(4)], &mut p);
        assert_eq!(s.shown, 4);
        s.step(&[PetInput::Leave(4)], &mut p);
        for _ in 0..5 {
            s.step(&[], &mut p);
        }
        assert_eq!(s.shown, 4);
        s.step(&[], &mut p);
        assert_eq!(s.shown, -1);
    }

    #[test]
    fn bubbles_map_back() {
        for id in 0..24 {
            let (x, y, _, _) = bubble(id);
            assert_eq!(bubble_at(x + 45, y + 40), Some(id));
        }
    }
}
