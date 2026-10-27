//! Presto (pet 19) and the PRESTO CHANGE-O dialog. Spec:
//! `docs/re/objects/pet-presto.md`.

use crate::profile::PET_COUNT;

/// Pet id 19.
pub const PRESTO: i32 = 19;

/// A changed pet's recharge before it can be right-clicked again (ticks).
pub const RECHARGE: i32 = 360;

/// Ticks the smoke puff lasts over a new Presto pet.
pub const PUFF: i32 = 20;

/// Pet button size in the dialog.
pub const BUTTON: i32 = 50;

/// The CANCEL button: (x, y, w); its height is the button image's.
pub const CANCEL: (i32, i32, i32) = (270, 30, 100);

/// The `dialogbutton2` cel height (`framework/dialog.md#creation`).
pub const CANCEL_H: i32 = 33;

/// Top-left of pet button `i` (`PetDialog` ctor `0x005355c0`): a column
/// down the left, a row along the bottom, a column up the right.
pub fn button_pos(i: usize) -> (i32, i32) {
    let i = i as i32;
    match i {
        0..=6 => (10, 120 + 50 * i),
        7..=17 => (62 + 52 * (i - 7), 420),
        _ => (582, 370 - 50 * (i - 18)),
    }
}

/// The open PRESTO CHANGE-O dialog (id `0x1f`). It doesn't pause the
/// board and has no button delay (pet-presto.md#the-petdialog-0x005355c0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PetDialog {
    /// `+0x15c`: the pet button pressed (−1 none).
    pub pressed: i32,
    /// The CANCEL button is held down.
    pub cancel_down: bool,
}

/// What a mouse-up on the dialog did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogPick {
    /// Nothing; the dialog stays up.
    None,
    /// Closed without a change.
    Cancel,
    /// Closed; change Presto into this pet.
    Pet(i32),
}

impl PetDialog {
    pub fn new() -> Self {
        PetDialog {
            pressed: -1,
            cancel_down: false,
        }
    }

    /// The hit test (`0x005329f0`): the first button whose rect holds the
    /// point; −1 if none, or if it isn't owned.
    pub fn hit(owned: &[bool; PET_COUNT], x: i32, y: i32) -> i32 {
        for (i, &own) in owned.iter().enumerate() {
            let (bx, by) = button_pos(i);
            if (bx..bx + BUTTON).contains(&x) && (by..by + BUTTON).contains(&y) {
                return if own { i as i32 } else { -1 };
            }
        }
        -1
    }

    fn on_cancel(x: i32, y: i32) -> bool {
        let (cx, cy, cw) = CANCEL;
        (cx..cx + cw).contains(&x) && (cy..cy + CANCEL_H).contains(&y)
    }

    /// Mouse-down, either button (slot 55 `0x00532a60`; the CANCEL
    /// button is a child widget and takes clicks on itself).
    pub fn down(&mut self, owned: &[bool; PET_COUNT], x: i32, y: i32) {
        if Self::on_cancel(x, y) {
            self.cancel_down = true;
        } else {
            self.pressed = Self::hit(owned, x, y);
        }
    }

    /// Mouse-up (slot 57 `0x00532a90`): a pick when it lands on the
    /// button that was pressed.
    pub fn up(&mut self, owned: &[bool; PET_COUNT], x: i32, y: i32) -> DialogPick {
        if std::mem::take(&mut self.cancel_down) {
            return if Self::on_cancel(x, y) {
                DialogPick::Cancel
            } else {
                DialogPick::None
            };
        }
        let hit = Self::hit(owned, x, y);
        let pressed = std::mem::replace(&mut self.pressed, -1);
        if hit == pressed && pressed != -1 {
            DialogPick::Pet(pressed)
        } else {
            DialogPick::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_layout() {
        assert_eq!(button_pos(0), (10, 120));
        assert_eq!(button_pos(6), (10, 420));
        assert_eq!(button_pos(7), (62, 420));
        assert_eq!(button_pos(17), (582, 420));
        assert_eq!(button_pos(18), (582, 370));
        assert_eq!(button_pos(23), (582, 120));
    }

    #[test]
    fn pick_needs_down_and_up_on_an_owned_button() {
        let mut owned = [false; PET_COUNT];
        owned[2] = true;
        let mut d = PetDialog::new();
        let (x, y) = button_pos(2);
        d.down(&owned, x + 5, y + 5);
        assert_eq!(d.up(&owned, x + 40, y + 40), DialogPick::Pet(2));
        // Unowned: nothing.
        let (x, y) = button_pos(3);
        d.down(&owned, x + 5, y + 5);
        assert_eq!(d.up(&owned, x + 5, y + 5), DialogPick::None);
        // Released elsewhere: nothing.
        let (x, y) = button_pos(2);
        d.down(&owned, x + 5, y + 5);
        assert_eq!(d.up(&owned, 300, 300), DialogPick::None);
        d.down(&owned, 280, 40);
        assert_eq!(d.up(&owned, 300, 50), DialogPick::Cancel);
    }
}
