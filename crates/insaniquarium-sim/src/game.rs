//! The Adventure flow across screens: a tank level, then the hatch screen,
//! then the next level. Owns the profile and carries the RNG streams from
//! one screen to the next (`docs/re/screens/hatch-screen.md`).

use sexy_core::rand::MtRand;

use crate::board::{Board, Input};
use crate::hatch::{Hatch, HatchOut};
use crate::music::MusicState;
use crate::pet_screen::{PetInput, PetOutcome, PetScreen};
use crate::profile::Profile;

#[derive(Debug, Clone)]
pub enum Screen {
    Tank(Box<Board>),
    /// The hatch screen and the streams it keeps drawing from.
    Hatch(Box<HatchScreen>),
    /// Choosing pets for the next level (`pet-screen.md`). No RNG: the
    /// streams wait here for `StartLevel`.
    Pets(Box<PetsScreen>),
    /// Back at the main menu (`pet-screen.md#input`, Menu). TODO(spec): the
    /// game selector screen.
    Menu,
}

#[derive(Debug, Clone)]
pub struct PetsScreen {
    pub screen: PetScreen,
    pub rng: MtRand,
    pub global_rng: MtRand,
}

#[derive(Debug, Clone)]
pub struct HatchScreen {
    pub hatch: Hatch,
    pub rng: MtRand,
    pub global_rng: MtRand,
}

/// Hatch-screen inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HatchInput {
    /// The mouse button is held down on the screen (skips the egg).
    Held,
    /// Continue button released (`ButtonDepress(99)`).
    Continue,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub profile: Profile,
    pub screen: Screen,
    /// The music interface's state (`music.rs`); presentation, but kept
    /// here so it's deterministic and comparable with captures.
    pub music: MusicState,
}

impl Game {
    /// Start Adventure at the profile's level.
    /// Starting Adventure from the menu goes through `NextLevel` too, so
    /// with more than 3 pets it opens on the pet screen.
    pub fn new(profile: Profile, rng: MtRand, global_rng: MtRand) -> Self {
        let mut g = Game {
            profile,
            screen: Screen::Menu,
            music: MusicState::default(),
        };
        g.next_level(rng, global_rng);
        g
    }

    fn level_board(p: &Profile, rng: MtRand, global_rng: MtRand) -> Board {
        // TODO(spec): bonus rounds (`0x00537e20`) after each tank's 5th level.
        let mut b = Board::start_level(rng, global_rng, p.tank, p.level, &p.level_pets());
        b.owned = p.pets;
        b
    }

    /// `App::NextLevel` (`hatch-screen.md`): clear the chosen pets, then the
    /// pet screen when more than 3 are owned, else straight into the level.
    fn next_level(&mut self, rng: MtRand, global_rng: MtRand) {
        if self.profile.is_bonus_round() {
            // TODO(spec): the bonus round (`0x00537e20`) and its results
            // screen; skipped to the next tank for now.
            self.profile.advance();
        }
        self.profile.chosen = Default::default();
        if self.profile.needs_pet_screen() {
            self.screen = Screen::Pets(Box::new(PetsScreen {
                screen: PetScreen::new(&self.profile),
                rng,
                global_rng,
            }));
        } else {
            self.start_level(rng, global_rng);
        }
    }

    fn start_level(&mut self, rng: MtRand, global_rng: MtRand) {
        let board = Self::level_board(&self.profile, rng, global_rng);
        self.screen = Screen::Tank(Box::new(board));
        self.music.tank(self.profile.tank, false);
    }

    /// One tick of the pet screen. Its music isn't touched.
    pub fn step_pets(&mut self, inputs: &[PetInput]) {
        let Screen::Pets(p) = &mut self.screen else {
            return;
        };
        match p.screen.step(inputs, &mut self.profile) {
            PetOutcome::Stay => self.music.update(),
            PetOutcome::StartLevel => {
                let (rng, global_rng) = (p.rng.clone(), p.global_rng.clone());
                self.start_level(rng, global_rng);
            }
            PetOutcome::Menu => self.screen = Screen::Menu,
        }
    }

    /// One tick of the tank. After the last egg piece, the board is
    /// replaced by the hatch screen (`Board::BuyEggPiece` → `ShowHatchScreen`,
    /// within the same click; no RNG).
    pub fn step_tank(&mut self, inputs: &[Input]) {
        let Screen::Tank(board) = &mut self.screen else {
            return;
        };
        // The board updates, then the music fades (seen in captures: a cue's
        // tick already shows one fade step).
        board.step(inputs);
        self.music.cue(board.music, board.tank);
        self.music.update();
        if board.level_complete {
            let pet = self.profile.finish_level(board.tank, board.level);
            let (rng, global_rng) = (board.rng.clone(), board.global_rng.clone());
            self.screen = Screen::Hatch(Box::new(HatchScreen {
                hatch: Hatch::new(pet),
                rng,
                global_rng,
            }));
            self.music.hatch();
        }
    }

    /// One tick of the hatch screen. Continue (once clickable) removes it
    /// and starts the next level (`App::NextLevel` → `StartLevel`).
    pub fn step_hatch(&mut self, inputs: &[HatchInput]) -> HatchOut {
        let Screen::Hatch(h) = &mut self.screen else {
            return HatchOut::default();
        };
        let HatchScreen {
            hatch,
            rng,
            global_rng,
        } = &mut **h;
        if inputs.contains(&HatchInput::Continue) && hatch.can_continue() {
            let (rng, global_rng) = (rng.clone(), global_rng.clone());
            self.next_level(rng, global_rng);
            return HatchOut::default();
        }
        let out = hatch.update(rng, inputs.contains(&HatchInput::Held));
        self.music.update();
        out
    }

    pub fn board(&self) -> Option<&Board> {
        match &self.screen {
            Screen::Tank(b) => Some(b),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shop::Slot;

    #[test]
    fn finishing_1_1_hatches_stinky_then_starts_1_2_with_him() {
        let mut g = Game::new(Profile::default(), MtRand::new(1), MtRand::new(2));
        let Screen::Tank(b) = &mut g.screen else {
            unreachable!()
        };
        // Skip to the last egg piece.
        b.money = 1000;
        b.egg_count = 3;
        b.shop.reveal(Slot::EggPiece, false);
        g.step_tank(&[Input::Buy(Slot::EggPiece)]);
        let Screen::Hatch(h) = &g.screen else {
            panic!("expected the hatch screen")
        };
        assert_eq!(h.hatch.pet, 0);
        assert_eq!((g.profile.tank, g.profile.level), (1, 2));
        // Continue is ignored until it's clickable.
        g.step_hatch(&[HatchInput::Continue]);
        assert!(g.board().is_none());
        while !matches!(&g.screen, Screen::Hatch(h) if h.hatch.can_continue()) {
            g.step_hatch(&[]);
        }
        g.step_hatch(&[HatchInput::Continue]);
        let b = g.board().expect("level 1-2");
        assert_eq!((b.tank, b.level, b.money), (1, 2, 200));
        assert_eq!(b.pets.len(), 1);
    }
}
