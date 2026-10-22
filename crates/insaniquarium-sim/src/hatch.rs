//! The hatch screen after a level's last egg piece (`Sexy::HatchScreen`).
//! Spec: `docs/re/screens/hatch-screen.md`. It is presentation, but its
//! update draws the app RNG every tick, so it lives in the sim: the next
//! level's RNG state depends on how long the player stayed here.

use sexy_core::rand::MtRand;

use crate::profile::BOSS_REVEAL;

/// Update count at which the hatch sound plays (§4 step 4).
pub const SOUND_AT: i32 = 141;
/// Update count at which the Continue button becomes clickable.
pub const CONTINUE_AT: i32 = 170;
/// A held mouse button skips ahead to here (§4 step 4).
const SKIP_TO: i32 = 140;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hatch {
    /// `+0x98`: pet id (0..23) or [`BOSS_REVEAL`].
    pub pet: i32,
    /// `+0x94`: updates so far.
    pub t: i32,
    /// `+0x9c`, `+0xa0`: egg shake (x 0..3, y 0..1).
    pub jitter: (i32, i32),
    /// `+0xa4`: the boss's y (boss reveal only).
    pub boss_y: i32,
    /// The profile is at 5-1 (the boss reveal after 4-5).
    pub boss: bool,
}

/// What a hatch-screen tick asks the frontend to do.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HatchOut {
    /// Play `SOUND_HATCH` (or `SOUND_EVILLAFF` for the boss).
    pub sound: bool,
}

impl Hatch {
    /// `HatchScreen::HatchScreen` (`0x0051f450`). The bubble pre-warm uses
    /// only the global stream, and the bubbles are never drawn: not modelled.
    pub fn new(pet: i32) -> Self {
        Hatch {
            pet,
            t: 0,
            jitter: (0, 0),
            boss_y: 480,
            boss: pet == BOSS_REVEAL,
        }
    }

    /// `HatchScreen::Update` (`0x00520760`). `held`: the mouse button is
    /// down on the screen itself (not on a button).
    pub fn update(&mut self, rng: &mut MtRand, held: bool) -> HatchOut {
        let mut out = HatchOut::default();
        // Two app draws every tick, unconditionally.
        self.jitter.0 = (rng.next_u31() & 3) as i32;
        self.jitter.1 = (rng.next_u31() & 1) as i32;
        let t = self.t;
        if held && t < SKIP_TO {
            self.t = SKIP_TO;
        } else if t == SOUND_AT {
            out.sound = true;
        }
        if self.boss {
            let t = self.t;
            self.boss_y += match t {
                140..=160 => -12,
                161..=170 => -8,
                171..=180 => -5,
                181..=190 => -2,
                191..=219 => -1,
                220.. => match t % 16 {
                    0..=3 => -1,
                    8..=11 => 1,
                    _ => 0,
                },
                _ => 0,
            };
        }
        self.t += 1;
        out
    }

    /// The Continue button is clickable (from update 171 on).
    pub fn can_continue(&self) -> bool {
        self.t > CONTINUE_AT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_draws_per_tick_and_timeline() {
        let mut rng = MtRand::new(1);
        let mut probe = MtRand::new(1);
        let mut h = Hatch::new(0);
        let mut sound_at = None;
        for i in 0..200 {
            if h.update(&mut rng, false).sound {
                sound_at = Some(i);
            }
            probe.next_u31();
            probe.next_u31();
        }
        assert_eq!(rng.state(), probe.state());
        assert_eq!(sound_at, Some(SOUND_AT));
        assert!(h.can_continue());
    }

    #[test]
    fn holding_the_mouse_skips_to_the_reveal() {
        let mut rng = MtRand::new(1);
        let mut h = Hatch::new(0);
        h.update(&mut rng, true);
        assert_eq!(h.t, SKIP_TO + 1);
        assert!(!h.can_continue());
    }
}
