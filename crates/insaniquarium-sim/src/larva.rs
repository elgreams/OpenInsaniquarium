//! Larva/Beetle (`Sexy::Beetle`, type `0x1d`, board list `+0xc8`).
//! Dropped by the Guppycruncher, eaten by the Beetlemuncher.
//!
//! TODO(spec): full Larva behaviour (walking, growth, animation).

/// A minimal Larva: enough for the Beetlemuncher's prey chase and bite
/// check. The full update (walk, grow, animate) is not yet implemented.
#[derive(Debug, Clone, PartialEq)]
pub struct Larva {
    pub x: f64,
    pub y: f64,
    /// `+0x30`, `+0x34`: truncated position.
    pub widget_x: i32,
    pub widget_y: i32,
    /// `+0x174`: 0 while alive.
    pub dead: bool,
}

impl Larva {
    pub fn new(x: i32, y: i32) -> Self {
        Larva {
            x: f64::from(x),
            y: f64::from(y),
            widget_x: x,
            widget_y: y,
            dead: false,
        }
    }
}
