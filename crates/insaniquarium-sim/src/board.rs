//! The tank (`Sexy::Board`). Spec: `docs/re/sim/board.md`.
//!
//! Objects live in typed, insertion-ordered vectors that mirror the
//! original's Board vectors; update order follows `Board::RebuildZOrder`.

use sexy_core::rand::MtRand;

use crate::fish::{Fish, FishEnv};

#[derive(Debug, Clone)]
pub struct Board {
    /// `board+0x3f0`
    pub money: i32,
    /// `board+0x94`
    pub paused: bool,
    /// `board+0xa0`: guppies in insertion order.
    pub guppies: Vec<Fish>,
    /// Aliens present (`board+0xb8`/`+0xf4` non-empty). Aliens aren't
    /// modelled yet; this flag lets traces with aliens be represented.
    pub has_aliens: bool,
    /// App RNG (`[app+0x7b0]`) — almost all gameplay randomness.
    pub rng: MtRand,
    /// Global RNG (`0x005eb928`).
    pub global_rng: MtRand,
    /// Logic ticks executed (`app+0x47c`).
    pub tick: u32,
}

impl Board {
    pub fn new(rng: MtRand, global_rng: MtRand) -> Self {
        Self {
            money: 0,
            paused: false,
            guppies: Vec::new(),
            has_aliens: false,
            rng,
            global_rng,
            tick: 0,
        }
    }

    /// `Board::AddRandomGuppy` (`0x00546d70`): x, y, then the 6 ctor draws,
    /// then two more.
    pub fn add_random_guppy(&mut self) {
        let x = self.rng.next_range(520) as i32 + 20;
        let y = self.rng.next_range(265) as i32 + 105;
        let mut f = Fish::new(x, y, &mut self.rng);
        // TODO(spec): AddRandomGuppy then sets two more fields
        // (`%5+18` and `%10+45`); the decompiler offsets are ambiguous, so
        // we only consume the draws for now (fish-guppy.md open question).
        self.rng.next_range(5);
        self.rng.next_range(10);
        f.vy_damp_timer = 0;
        self.guppies.push(f);
    }

    /// One logic tick (28 ms).
    pub fn update(&mut self) {
        if self.paused {
            return;
        }
        // Board::Update ambient check: one app-RNG draw per tick
        // (`Next() % 1000 == 0` → ambient bubbles; confirmed live: exactly
        // one draw per idle tick). TODO(spec): bubble burst draws when it
        // hits, and the rest of Board::Update (board-update.md).
        if self.rng.next_range(1000) == 0 {
            let n = (self.rng.next_u31() & 3) + 2;
            let _ = n;
        }
        // Objects update in RebuildZOrder order; guppies are the only kind
        // modelled so far.
        let env = FishEnv {
            paused: self.paused,
            has_aliens: self.has_aliens,
        };
        for f in &mut self.guppies {
            f.update(env, &mut self.rng, &mut self.global_rng);
        }
        self.tick += 1;
    }
}
