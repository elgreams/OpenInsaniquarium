//! Sound events the sim emits for the frontend. The sim decides *which*
//! sample plays (variant choice consumes RNG, so it must happen in the sim);
//! the app only plays them. Names are the `resources.xml` sound ids.

/// One sample to play this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    DropFood,
    Slurp,
    Slurp2,
    Slurp3,
    Points,
    Points2,
    Points3,
    Points4,
    Splash,
    Splash2,
    Splash3,
    Buy,
    Grow,
    Die,
    Buzzer,
}

impl Sound {
    /// Resource id in `resources.xml`.
    pub fn id(self) -> &'static str {
        match self {
            Sound::DropFood => "SOUND_DROPFOOD",
            Sound::Slurp => "SOUND_SLURP",
            Sound::Slurp2 => "SOUND_SLURP2",
            Sound::Slurp3 => "SOUND_SLURP3",
            Sound::Points => "SOUND_POINTS",
            Sound::Points2 => "SOUND_POINTS2",
            Sound::Points3 => "SOUND_POINTS3",
            Sound::Points4 => "SOUND_POINTS4",
            Sound::Splash => "SOUND_SPLASH",
            Sound::Splash2 => "SOUND_SPLASH2",
            Sound::Splash3 => "SOUND_SPLASH3",
            Sound::Buy => "SOUND_BUY",
            Sound::Grow => "SOUND_GROW",
            Sound::Die => "SOUND_DIE",
            Sound::Buzzer => "SOUND_BUZZER",
        }
    }

    /// Slurp variant for `r = Next() % 5` (`0x005384c0`).
    pub fn slurp(r: u32) -> Sound {
        match r {
            0 => Sound::Slurp3,
            1 => Sound::Slurp2,
            _ => Sound::Slurp,
        }
    }

    /// Coin "points" variant for `r = Next() & 3` (`0x005383d0`).
    pub fn points(r: u32) -> Sound {
        match r {
            1 => Sound::Points2,
            2 => Sound::Points3,
            3 => Sound::Points4,
            _ => Sound::Points,
        }
    }

    /// Splash variant for `r = Next() % 3` (`0x00538360`).
    pub fn splash(r: u32) -> Sound {
        match r {
            0 => Sound::Splash,
            1 => Sound::Splash2,
            _ => Sound::Splash3,
        }
    }
}
