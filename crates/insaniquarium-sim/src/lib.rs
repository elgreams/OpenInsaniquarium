//! Deterministic Insaniquarium simulation. No Bevy, no I/O, no wall clock:
//! state advances only via fixed ticks and input events.

pub mod alien;
pub mod board;
pub mod coin;
pub mod fish;
pub mod food;
pub mod game;
pub mod hatch;
pub mod itchy;
pub mod message;
pub mod missile;
pub mod music;
pub mod pet;
pub mod pet_screen;
pub mod presto;
pub mod profile;
pub mod shop;
pub mod sound;

pub use board::Board;
pub use coin::Coin;
pub use fish::Fish;
pub use food::Food;
