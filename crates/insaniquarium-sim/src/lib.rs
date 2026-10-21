//! Deterministic Insaniquarium simulation. No Bevy, no I/O, no wall clock:
//! state advances only via fixed ticks and input events.

pub mod board;
pub mod fish;

pub use board::Board;
pub use fish::Fish;
