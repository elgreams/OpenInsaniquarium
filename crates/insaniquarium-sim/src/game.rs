//! The Adventure flow across screens: a tank level, then the hatch screen,
//! then the next level. Owns the profile and carries the RNG streams from
//! one screen to the next (`docs/re/screens/hatch-screen.md`).

use sexy_core::rand::MtRand;

use crate::board::{Board, Input};
use crate::hatch::{Hatch, HatchOut};
use crate::music::MusicState;
use crate::profile::Profile;

#[derive(Debug, Clone)]
pub enum Screen {
    Tank(Box<Board>),
    /// The hatch screen and the streams it keeps drawing from.
    Hatch(Box<HatchScreen>),
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
    pub fn new(profile: Profile, rng: MtRand, global_rng: MtRand) -> Self {
        let board = Self::level_board(&profile, rng, global_rng);
        let mut music = MusicState::default();
        // `StartLevel` → `PlayTankMusic(false)`.
        music.tank(profile.tank, false);
        Game {
            profile,
            screen: Screen::Tank(Box::new(board)),
            music,
        }
    }

    fn level_board(p: &Profile, rng: MtRand, global_rng: MtRand) -> Board {
        // TODO(spec): bonus rounds (`0x00537e20`) and the pet screen for
        // more than 3 owned pets; neither happens before 1-5.
        Board::start_level(rng, global_rng, p.tank, p.level, &p.level_pets())
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
            let board = Self::level_board(&self.profile, rng, global_rng);
            self.screen = Screen::Tank(Box::new(board));
            self.music.tank(self.profile.tank, false);
            return HatchOut::default();
        }
        let out = hatch.update(rng, inputs.contains(&HatchInput::Held));
        self.music.update();
        out
    }

    pub fn board(&self) -> Option<&Board> {
        match &self.screen {
            Screen::Tank(b) => Some(b),
            Screen::Hatch(_) => None,
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
