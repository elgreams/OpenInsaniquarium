//! Adventure progress (the parts of the player profile the game flow needs).
//! Spec: `docs/re/screens/hatch-screen.md#1-from-the-last-egg-piece-to-the-hatch-screen`.
//! Session-only for now: TODO(M4) our own save format and the `.dat` importer.

/// Pets in profile order (id = index). Ids 20–23 aren't won from eggs.
pub const PET_COUNT: usize = 24;

/// Hatch id for the tank 4-5 boss reveal (no pet).
pub const BOSS_REVEAL: i32 = 999;

/// Pet 19 (PRESTO), awarded with 5000 shells for beating the final boss.
pub const PRESTO: i32 = 0x13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// `+0x1c`, `+0x20`: the next Adventure level.
    pub tank: i32,
    pub level: i32,
    /// `+0x00 + id`: pets owned.
    pub pets: [bool; PET_COUNT],
    /// `+0x48`: Virtual Tank shells.
    pub shells: i32,
    /// `+0x50`: times the game was beaten.
    pub times_beaten: i32,
    /// `+0x59`: game completed.
    pub completed: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            tank: 1,
            level: 1,
            pets: [false; PET_COUNT],
            shells: 0,
            times_beaten: 0,
            completed: false,
        }
    }
}

impl Profile {
    /// Owned pets `PopulateLevel` adds when the player owns at most 3
    /// (no pet screen): ids 0..=18 in id order.
    pub fn level_pets(&self) -> Vec<i32> {
        (0..19).filter(|&id| self.pets[id as usize]).collect()
    }

    /// Number of owned pets (`+0x18`).
    pub fn pet_count(&self) -> usize {
        self.pets.iter().filter(|&&p| p).count()
    }

    /// The end of `Board::BuyEggPiece` (`0x00545960`) after the last egg
    /// piece of board level `tank`-`level`, normal Adventure path: advance
    /// the profile, then award the level's pet. Returns the hatch-screen id.
    /// No RNG.
    pub fn finish_level(&mut self, tank: i32, level: i32) -> i32 {
        self.advance();
        let id = if tank == 4 && level == 5 {
            BOSS_REVEAL
        } else if tank == 5 {
            PRESTO
        } else {
            tank * 5 + level - 6
        };
        if (0..PET_COUNT as i32).contains(&id) {
            self.pets[id as usize] = true;
            if id == PRESTO {
                self.shells = (self.shells + 5000).clamp(0, 9_999_999);
            }
        }
        id
    }

    /// `Profile::AdvanceLevel` (`0x00501370`).
    pub fn advance(&mut self) {
        self.level += 1;
        if self.tank == 4 && self.level == 6 {
            // No bonus round after tank 4: straight to the final boss.
            self.tank = 5;
            self.level = 1;
        }
        if self.tank == 5 && self.level >= 2 {
            self.times_beaten += 1;
            self.completed = true;
        }
        if self.level > 6 || (self.completed && self.level > 5) {
            self.tank += 1;
            self.level = 1;
        }
    }

    /// `App::IsBonusRound` (`0x0054b2a0`), Adventure.
    pub fn is_bonus_round(&self) -> bool {
        self.level == 6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tank_1_awards_and_advance() {
        let mut p = Profile::default();
        assert_eq!(p.finish_level(1, 1), 0, "Stinky");
        assert_eq!((p.tank, p.level), (1, 2));
        assert_eq!(p.level_pets(), vec![0]);
        assert_eq!(p.finish_level(1, 2), 1, "Niko");
        for l in 3..=5 {
            p.finish_level(1, l);
        }
        assert_eq!((p.tank, p.level), (1, 6));
        assert!(p.is_bonus_round());
        assert_eq!(p.pet_count(), 5);
    }

    #[test]
    fn tank_4_skips_its_bonus_round() {
        let mut p = Profile {
            tank: 4,
            level: 5,
            ..Profile::default()
        };
        assert_eq!(p.finish_level(4, 5), BOSS_REVEAL);
        assert_eq!((p.tank, p.level), (5, 1));
        assert_eq!(p.finish_level(5, 1), PRESTO);
        assert!(p.completed && p.shells == 5000);
    }
}
