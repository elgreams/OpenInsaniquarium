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
    /// The main menu (`screens/game-selector.md`).
    Menu(MenuScreen),
    /// The v0.1.0 preview's end card, after the last built level's hatch
    /// screen (not in the original; any click returns to the menu).
    PreviewEnd,
}

/// Main-menu buttons the preview acts on (`game-selector.md#buttons`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuButton {
    Adventure,
    Quit,
}

/// Main-menu state. The menu draws nothing from the sim's RNG streams
/// (game-selector.md#deviations).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuScreen {
    /// "Stop the insanity?" is up (dialog 0x0b).
    pub quit_confirm: bool,
}

/// Inputs to the main menu and the end card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    Press(MenuButton),
    /// QUIT / CANCEL on the quit confirm.
    QuitYes,
    QuitNo,
    /// Any click on the preview end card.
    Click,
}

/// The tank's Options dialog (`screens/options-dialog.md`): opened by the
/// Menu button, it freezes the board; Leave Game? sits on top of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Options {
    Open,
    LeaveConfirm,
}

/// Options-dialog inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsInput {
    /// The tank's Menu button (id 0x7b).
    Open,
    Ok,
    BackToMainMenu,
    Leave,
    Cancel,
}

/// What a menu step asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOut {
    Stay,
    /// Adventure was pressed: the app supplies fresh RNG streams
    /// ([`Game::start_adventure`]).
    StartAdventure,
    Quit,
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
    /// The tank's Options dialog, when open (the board doesn't run).
    pub options: Option<Options>,
    /// The last level this build plays (tank, level): finishing it ends
    /// the preview instead of starting the next level.
    pub last_level: (i32, i32),
    /// The music interface's state (`music.rs`); presentation, but kept
    /// here so it's deterministic and comparable with captures.
    pub music: MusicState,
}

impl Game {
    /// Start Adventure at the profile's level.
    /// Starting Adventure from the menu goes through `NextLevel` too, so
    /// with more than 3 pets it opens on the pet screen.
    pub fn new(profile: Profile, rng: MtRand, global_rng: MtRand) -> Self {
        let mut g = Game::menu(profile);
        g.next_level(rng, global_rng);
        g
    }

    /// At the main menu, with the menu music (`game-selector.md#music`).
    pub fn menu(profile: Profile) -> Self {
        let mut g = Game {
            profile,
            screen: Screen::Menu(MenuScreen::default()),
            options: None,
            last_level: (1, 5),
            music: MusicState::default(),
        };
        g.show_menu();
        g
    }

    fn show_menu(&mut self) {
        self.options = None;
        self.screen = Screen::Menu(MenuScreen::default());
        self.music.stop_all();
        self.music.play(2, 0, 1.0);
    }

    /// Adventure from the menu: the profile's level, through `NextLevel`.
    /// Past the last built level (the preview's end), Adventure starts over
    /// at 1-1 with a fresh session profile.
    pub fn start_adventure(&mut self, rng: MtRand, global_rng: MtRand) {
        if (self.profile.tank, self.profile.level) > self.last_level {
            self.profile = Profile::default();
        }
        self.next_level(rng, global_rng);
    }

    /// One tick of the main menu or the end card.
    pub fn step_menu(&mut self, inputs: &[MenuInput]) -> MenuOut {
        let mut out = MenuOut::Stay;
        match &mut self.screen {
            Screen::Menu(m) => {
                for &i in inputs {
                    match (m.quit_confirm, i) {
                        (false, MenuInput::Press(MenuButton::Adventure)) => {
                            out = MenuOut::StartAdventure;
                        }
                        (false, MenuInput::Press(MenuButton::Quit)) => m.quit_confirm = true,
                        (true, MenuInput::QuitYes) => out = MenuOut::Quit,
                        (true, MenuInput::QuitNo) => m.quit_confirm = false,
                        _ => {}
                    }
                }
            }
            Screen::PreviewEnd if inputs.contains(&MenuInput::Click) => self.show_menu(),
            _ => {}
        }
        self.music.update();
        out
    }

    /// The tank's Menu button and the Options dialog
    /// (`options-dialog.md#buttons`). Applied before the tick's board step.
    pub fn options_input(&mut self, i: OptionsInput) {
        let Screen::Tank(b) = &self.screen else {
            return;
        };
        match (self.options, i) {
            // The Menu button is a board widget: dialogs and the pause
            // cover it.
            (None, OptionsInput::Open) if b.dialog.is_none() && !b.focus_paused => {
                self.options = Some(Options::Open);
            }
            (Some(Options::Open), OptionsInput::Ok) => self.options = None,
            (Some(Options::Open), OptionsInput::BackToMainMenu) => {
                if b.fed {
                    self.options = Some(Options::LeaveConfirm);
                } else {
                    self.show_menu();
                }
            }
            (Some(Options::LeaveConfirm), OptionsInput::Leave) => self.show_menu(),
            (Some(Options::LeaveConfirm), OptionsInput::Cancel) => {
                self.options = Some(Options::Open);
            }
            _ => {}
        }
    }

    /// GAME OVER's button: back to the main menu, after the dialog's button
    /// delay (`framework/dialog.md#game-over-button`).
    fn game_over_continue(&mut self, inputs: &[Input]) -> bool {
        let Screen::Tank(b) = &self.screen else {
            return false;
        };
        b.dialog == Some(crate::board::Dialog::GameOver)
            && b.dialog_age >= crate::board::DIALOG_BUTTON_DELAY
            && inputs.contains(&Input::DismissDialog)
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
            PetOutcome::Menu => self.show_menu(),
        }
    }

    /// One tick of the tank. After the last egg piece, the board is
    /// replaced by the hatch screen (`Board::BuyEggPiece` → `ShowHatchScreen`,
    /// within the same click; no RNG).
    pub fn step_tank(&mut self, inputs: &[Input]) {
        if self.game_over_continue(inputs) {
            self.show_menu();
            return;
        }
        if self.options.is_some() {
            // Frozen under the Options dialog (`options-dialog.md#dialog`).
            return;
        }
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
            let (t, l) = self.last_level;
            if (self.profile.tank, self.profile.level) > (t, l) {
                self.screen = Screen::PreviewEnd;
                return HatchOut::default();
            }
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

    fn feed_until_fed(g: &mut Game) {
        for _ in 0..3000 {
            if g.board().unwrap().fed {
                return;
            }
            g.step_tank(&[Input::Click { x: 300, y: 250 }]);
        }
        panic!("no fish ate");
    }

    #[test]
    fn menu_starts_adventure_and_quit_asks_first() {
        let mut g = Game::menu(Profile::default());
        assert_eq!(
            g.step_menu(&[MenuInput::Press(MenuButton::Quit)]),
            MenuOut::Stay
        );
        assert!(matches!(&g.screen, Screen::Menu(m) if m.quit_confirm));
        // Adventure is covered by the confirm.
        assert_eq!(
            g.step_menu(&[MenuInput::Press(MenuButton::Adventure)]),
            MenuOut::Stay
        );
        g.step_menu(&[MenuInput::QuitNo]);
        assert_eq!(
            g.step_menu(&[MenuInput::Press(MenuButton::Adventure)]),
            MenuOut::StartAdventure
        );
        g.start_adventure(MtRand::new(1), MtRand::new(2));
        assert_eq!(g.board().map(|b| (b.tank, b.level)), Some((1, 1)));
        g.show_menu();
        g.step_menu(&[MenuInput::Press(MenuButton::Quit)]);
        assert_eq!(g.step_menu(&[MenuInput::QuitYes]), MenuOut::Quit);
    }

    #[test]
    fn options_freeze_the_board_and_leave_asks_only_once_fed() {
        let mut g = Game::new(Profile::default(), MtRand::new(1), MtRand::new(2));
        g.options_input(OptionsInput::Open);
        assert_eq!(g.options, Some(Options::Open));
        let tick = g.board().unwrap().tick;
        g.step_tank(&[]);
        assert_eq!(g.board().unwrap().tick, tick, "frozen under Options");
        g.options_input(OptionsInput::Ok);
        g.step_tank(&[]);
        assert_eq!(g.board().unwrap().tick, tick + 1);
        // Nothing has eaten: Back to Main Menu leaves at once.
        g.options_input(OptionsInput::Open);
        g.options_input(OptionsInput::BackToMainMenu);
        assert!(matches!(g.screen, Screen::Menu(_)));
        assert_eq!(g.options, None);
        // Once fed, it asks; Cancel returns to Options, Leave to the menu.
        g.start_adventure(MtRand::new(1), MtRand::new(2));
        feed_until_fed(&mut g);
        g.options_input(OptionsInput::Open);
        g.options_input(OptionsInput::BackToMainMenu);
        assert_eq!(g.options, Some(Options::LeaveConfirm));
        g.options_input(OptionsInput::Cancel);
        assert_eq!(g.options, Some(Options::Open));
        g.options_input(OptionsInput::BackToMainMenu);
        g.options_input(OptionsInput::Leave);
        assert!(matches!(g.screen, Screen::Menu(_)));
    }

    #[test]
    fn game_over_button_goes_to_the_menu_after_its_delay() {
        let mut p = Profile::default();
        p.finish_level(1, 1);
        let mut g = Game::new(p, MtRand::new(1), MtRand::new(2));
        if let Screen::Tank(b) = &mut g.screen {
            b.guppies.clear();
        }
        g.step_tank(&[]);
        assert_eq!(
            g.board().unwrap().dialog,
            Some(crate::board::Dialog::GameOver)
        );
        g.step_tank(&[Input::DismissDialog]);
        assert!(g.board().is_some(), "button delay");
        for _ in 0..crate::board::DIALOG_BUTTON_DELAY {
            g.step_tank(&[]);
        }
        g.step_tank(&[Input::DismissDialog]);
        assert!(matches!(g.screen, Screen::Menu(_)));
    }

    #[test]
    fn finishing_the_last_built_level_ends_the_preview() {
        let mut p = Profile::default();
        for l in 1..5 {
            p.finish_level(1, l);
        }
        let mut g = Game::new(p, MtRand::new(1), MtRand::new(2));
        if let Screen::Pets(_) = g.screen {
            g.step_pets(&[PetInput::Continue]);
            g.step_pets(&[PetInput::ConfirmYes]);
        }
        let Screen::Tank(b) = &mut g.screen else {
            panic!("expected 1-5")
        };
        b.money = 100_000;
        b.egg_count = 3;
        b.shop.reveal(Slot::EggPiece, false);
        g.step_tank(&[Input::Buy(Slot::EggPiece)]);
        while !matches!(&g.screen, Screen::Hatch(h) if h.hatch.can_continue()) {
            g.step_hatch(&[]);
        }
        g.step_hatch(&[HatchInput::Continue]);
        assert!(matches!(g.screen, Screen::PreviewEnd));
        g.step_menu(&[MenuInput::Click]);
        assert!(matches!(g.screen, Screen::Menu(_)));
        // Adventure starts over at 1-1.
        g.start_adventure(MtRand::new(3), MtRand::new(4));
        assert_eq!(g.board().map(|b| (b.tank, b.level)), Some((1, 1)));
    }
}
