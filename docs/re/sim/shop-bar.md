# Shop / control bar (tank 1-1 focus)

Status: `draft` (static). Addresses are inner-PE VAs; offsets are `Board`-relative unless noted.

## Data model

The bar is a row of up to 7 `Sexy::MenuButtonWidget` buttons. Two layers of indexing:

- **Slot** (0..11): *what* the button sells. Fixed meaning, same in every level.
- **Position** (0..6): *where* on the bar it sits.

| Field | Meaning |
|---|---|
| `+0x2e4 + 4*slot` | slot → position index, or `-1` = slot not on this level's bar |
| `+0x314 + 4*slot` | price of slot (int, clamped to ≤ 99999 by the setter) |
| `+0x3a4 + slot` (byte) | "button has been created/revealed" flag |
| `+0x404 + 4*pos` | `MenuButtonWidget*` at that position (null until created) |
| `+0x2dc` | number of positions used by this level |
| `+0x2e0` | number of buttons created so far |

Helpers:
- `Board::GetSlotButton(slot)` @ `0x00538090`: returns the widget at `+0x404[+0x2e4[slot]]`, or null if the slot is unmapped (`-1`) or not yet created. Returns **only** the widget pointer — the dispatcher keeps the slot index itself.
- `Board::SetSlotPrice(slot, price)` @ `0x005380c0`: clamps `price` to 99999, stores it, and if the button exists refreshes its price label (`0x00534380`; label is blank when price < 1).
- `Board::RevealSlotButton(slot, ?)` @ `0x005409b0`: lazily creates the button for a slot (no-op if unmapped or already created). See [Layout](#layout).

TODO: writer of `+0x2e4` / `+0x314` per level.

## Slot meanings

TODO

## Layout

TODO

## Input path

TODO

## Purchases

TODO

## Egg pieces

TODO

## Money counter

TODO
