//! Insaniquarium frontend library: game-specific asset handling shared by the
//! game binary and tools (asset browser). `scene` and `render` are
//! Bevy-free: what a frame shows, and a CPU compositor for it.

pub mod assets;
pub mod framediff;
pub mod render;
pub mod scene;
pub mod song;
pub mod text;
