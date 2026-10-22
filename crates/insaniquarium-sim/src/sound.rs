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
    Awooga,
    Roar,
    Zap,
    Zzam,
    Ricochet,
    Hit,
    Chomp,
    Chomp2,
    Explode,
    Explosion1,
    Diamond,
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
            Sound::Awooga => "SOUND_AWOOGA",
            Sound::Roar => "SOUND_ROAR",
            Sound::Zap => "SOUND_ZAP",
            Sound::Zzam => "SOUND_ZZAM",
            Sound::Ricochet => "SOUND_RICOCHET",
            Sound::Hit => "SOUND_HIT",
            Sound::Chomp => "SOUND_CHOMP",
            Sound::Chomp2 => "SOUND_CHOMP2",
            Sound::Explode => "SOUND_EXPLODE",
            Sound::Explosion1 => "SOUND_EXPLOSION1",
            Sound::Diamond => "SOUND_DIAMOND",
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

/// One sound event: a sample and its volume (1.0 = the sample's own level).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Played {
    pub sound: Sound,
    pub volume: f64,
}

/// Music changes the board asks for (`assets/music.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Music {
    /// `PlayAlienMusic` (`0x0054c480`): fade the tank song out, start the
    /// alien song.
    Alien,
    /// `PlayTankMusic(1)` (`0x0054c390`): fade back to the tank song.
    Tank,
}

impl From<Sound> for Played {
    fn from(sound: Sound) -> Self {
        Played { sound, volume: 1.0 }
    }
}
