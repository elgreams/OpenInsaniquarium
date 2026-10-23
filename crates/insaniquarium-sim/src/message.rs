//! The board's message line (`MessageWidget` at `board+0x118`) and the
//! one-time hint bytes (`board+0x4b0..`). Presentation only: no RNG.
//! Spec: `docs/re/sim/board.md#message-line`.

/// Number of one-time hint bytes (`+0x4b0..=+0x4e5`).
pub const HINTS: usize = 0x36;

/// One-time ids of the hint dialogs (`framework/dialog.md`).
pub const HINT_DANGER: usize = 3;
pub const HINT_BATTLE_TIP: usize = 4;
pub const HINT_NEW_BREED: usize = 0x11;

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub text: String,
    /// `+0xd0`: ticks left; shown while > 0.
    pub timer: i32,
    /// `+0xd4`: blinks.
    pub flash: bool,
    /// `+0xd8`: id of the last one-time message, −1 for none.
    pub id: i32,
    /// Red alien-warning colours instead of the normal green
    /// (`+0xa8`/`+0xb8`; `ShowMessage` resets them to green).
    pub warning: bool,
}

impl Default for Message {
    fn default() -> Self {
        // The ctor: empty text, timer 185.
        Message {
            text: String::new(),
            timer: 185,
            flash: false,
            id: -1,
            warning: false,
        }
    }
}

impl Message {
    /// Showing now (`Draw`: timer > 0).
    pub fn visible(&self) -> bool {
        self.timer > 0
    }

    /// Drawn this frame: visible, and not in the off phase of a blink.
    pub fn drawn(&self) -> bool {
        self.visible() && (!self.flash || self.timer % 32 < 27)
    }

    /// Nothing is showing (the "idle" condition several callers check).
    pub fn idle(&self) -> bool {
        self.timer <= 0
    }

    /// `MessageWidget::Update` (`0x00530800`), unpaused ticks only.
    pub fn update(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
        }
    }
}

/// `Board::ShowMessage(text, flash, id)` (`0x0053e950`). `id` < 0 always
/// shows; otherwise only the first time per level (`hints[id]`).
pub fn show(msg: &mut Message, hints: &mut [bool; HINTS], text: &str, flash: bool, id: i32) {
    if id < 0 {
        msg.warning = false;
        msg.text = text.to_string();
        msg.flash = flash;
        msg.timer = if flash { 500 } else { 100 };
        msg.id = -1;
        return;
    }
    let i = id as usize;
    if hints[i] {
        return;
    }
    msg.warning = false;
    msg.text = text.to_string();
    msg.flash = flash;
    msg.timer = 185;
    msg.id = id;
    hints[i] = true;
}

/// The alien warning (`Board::Update` @ `0x005486e1`, board.md#the-alien-
/// warning-line): written directly, not through `ShowMessage`, so it shows
/// on every wave; red, no blink, 274 ticks.
pub fn show_warning(msg: &mut Message, text: &str) {
    msg.text = text.to_string();
    msg.flash = false;
    msg.timer = 274;
    msg.warning = true;
}

/// A message that can show up to three times per level: the first unused
/// id of `chain` (`Fish::UpdateCoin` / `UpdateHunger`).
pub fn show_chain(
    msg: &mut Message,
    hints: &mut [bool; HINTS],
    text: &str,
    flash: bool,
    chain: [i32; 3],
) {
    let [a, b, c] = chain;
    let id = if hints[b as usize] {
        c
    } else if hints[a as usize] {
        b
    } else {
        a
    };
    show(msg, hints, text, flash, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_time_messages_show_once_and_chains_three_times() {
        let mut m = Message::default();
        let mut h = [false; HINTS];
        show(&mut m, &mut h, "a", false, 0x1a);
        assert_eq!((m.text.as_str(), m.timer), ("a", 185));
        m.timer = 3;
        show(&mut m, &mut h, "b", false, 0x1a);
        assert_eq!((m.text.as_str(), m.timer), ("a", 3));
        for (n, id) in [(0, 5), (1, 0x19), (2, 0x1a)] {
            let mut h = [false; HINTS];
            for _ in 0..=n {
                show_chain(&mut m, &mut h, "coin", true, [5, 0x19, 0x1a]);
            }
            assert_eq!(m.id, id);
        }
        show(&mut m, &mut h, "free", true, -1);
        assert_eq!((m.timer, m.id), (500, -1));
        m.timer = 5 * 32 + 27;
        assert!(!m.drawn());
        m.timer -= 1;
        assert!(m.drawn());
    }
}
