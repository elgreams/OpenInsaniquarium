# Shop / control bar (tank 1-1 focus)

Status: per section below. Tank 1-1 and 1-2 behaviour is `traced`; everything for later levels is `static`.

Notation:
- Addresses are inner-PE VAs. Offsets are `Board`-relative unless noted.
- `app` = `[board+0x8c]` (WinFishApp). `profile` = `[app+0x8b8]`.
- `mode` = `app+0x87c`: 0 = Adventure; 1 = Time Trial; 3 = sandbox-like ("Sandbox" label); 4 = Challenge; 5 = Virtual Tank.
- `tank` / `level` = `+0x3c8` / `+0x3cc`.
- "Tutorial 1-1" = `tank == 1 && level == 1 && profile+0x59 == 0`. The predicates are `0x00537b80` (fixed 1-1) and `0x00537bb0(t, l)`. `profile+0x59` looks like "game completed once", and it suppresses tutorial hints.

## Data model

The bar holds up to 7 `Sexy::MenuButtonWidget` buttons. There are two kinds of index:

- **Slot** (0..11): *what* the button sells. The meaning is fixed and the same on every level. Slot = button id = `ButtonListener` id.
- **Position** (0..6): *where* the button sits on the bar.

| Field | Meaning |
|---|---|
| `+0x2e4 + 4*slot` | slot → position, or `-1` = slot not used on this level |
| `+0x314 + 4*slot` | current price (int, ≤ 99999) |
| `+0x344 + 4*slot` | base price (copied from `+0x314` at level init) |
| `+0x374 + 4*slot` | inflation step. If left at `-1`, it becomes `base / 100` (int division). Only used in mode 4 |
| `+0x3a4 + slot` (byte) | "button revealed" flag |
| `+0x404 + 4*pos` | `MenuButtonWidget*` at a position. Null until revealed |
| `+0x2dc` | number of mapped slots this level |
| `+0x2e0` | number of buttons revealed so far |

Helpers (our names):
- `Board::GetSlotButton(slot)` @ `0x00538090`: returns `+0x404[+0x2e4[slot]]`, or null if the slot is unmapped or not yet revealed. **This is not a hit test**, only a pointer lookup. Callers use it as "does this slot have a live button?".
- `Board::SetSlotPrice(slot, p)` @ `0x005380c0`: clamps `p` to ≤ 99999, stores it, and refreshes the price label if the button exists.
- `Board::RevealSlotButton(slot, popAnim)` @ `0x005409b0`: see [Reveal](#reveal).
- `Board::RefreshSlotButton(slot, popAnim)` @ `0x005406a0`: sets the price label, then the slot icon or "MAX" state. See [Button art](#button-art).
- `Board::SlotName(slot)` @ `0x00538100`: returns the tooltip string.

`traced` for the parts the replays compare indirectly (reveals, prices paid, money) on 1-1 and 1-2; field offsets are `static`.

## Slot table

| Slot | Name (tooltip) | Effect |
|---|---|---|
| 0 | "buy guppy" | `Board::AddRandomGuppy` @ `0x00546d70` |
| 1 | "buy breeder" | `0x005470a0` |
| 2 | "upgrade food quality" | global `food_quality` `0x005e89dc` += 1, cap 2 |
| 3 | "upgrade food quantity" | global `food_quantity` `0x005df58c` += 1, cap 9 |
| 4 | "buy carnivore" | `0x00544c00` |
| 5 | "buy star potion" | `+0x440 = 1` (potion armed; next tank click drops it) |
| 6 | "buy starcatcher" | `0x00545260` |
| 7 | "buy guppycruncher" | `0x00545430` |
| 8 | "buy beetlemuncher" | `++(+0x4a4)` (purchase stat), `0x00545080` |
| 9 | "buy ultravore" | `0x00544e60` |
| 10 | "upgrade weapon" | `+0x3e4` += 1, cap 12 |
| 11 | "buy egg piece". Mode 1: "buy random pet". Mode 3: "end level" | `Board::BuyEggPiece` @ `0x00545960` |

`static` (names and ids); slots 0, 2, 3 and 11 are exercised by the replays.

## Level init and per-level layout

`Board::InitLevel` @ `0x00541a20` does the following, in order:
1. Resets all 12 slots: position −1, price 0, inflation −1, revealed 0. Sets `+0x444 = 0` (the level tick counter that animates the guppy icon).
2. Sets **money `+0x3f0` = 200** (999999 in mode 3).
3. Sets egg counter `+0x43c = 1`, weapon `+0x3e4 = 2`, food quality = 0, food quantity = 1, potion armed `+0x440 = 0`. Zeroes the purchase stats `+0x460..+0x4a8` and the hint bytes `+0x4b0..+0x4e5`.
4. Calls `Board::SetupShop` @ `0x00541200`.
5. Counts `+0x2dc`, fills in default inflation steps, and copies the base prices.

`SetupShop` (Adventure, `app+0x880 == 0`) for tank 1:

| Slot | Pos | Price |
|---|---|---|
| 0 guppy | 0 | 100 |
| 2 food quality | 1 | 200 |
| 3 food quantity | 2 | 300 |
| 4 carnivore | 3 | 1000 |
| 10 weapon | 5 | 1000 |
| 11 egg piece | 6 | per level |

| Level | Egg price | Slots set back to −1 |
|---|---|---|
| **1** | **150** | 2, 3, 4, 10 |
| 2 | 500 | 4, 10 |
| 3 | 2000 | – |
| 4 | 3000 | – |
| 5 | 5000 | – |

**Tank 1-1 has exactly two slots: guppy (pos 0, $100) and egg piece (pos 6, $150).** `+0x2dc = 2`. Position 4 is unused on tank 1.

The same switch also writes `+0x2bc` (tank 1: 0/1/2/3/3 by level; 1-2 also sets `+0x2c0 = 1750`). These are not shop fields; they look like alien/wave parameters and belong to the alien spec.

Tank 2 (`0x00541200`, the tank-2 branch):

| Slot | Pos | Price |
|---|---|---|
| 0 guppy | 0 | 100 |
| 2 food quality | 1 | 200 |
| 3 food quantity | 2 | 300 |
| 5 star potion | 3 | 250 |
| 6 Starcatcher | 4 | 750 |
| 10 weapon | 5 | 1000 |
| 11 egg piece | 6 | per level |

| Level | Egg price | Wave kind `+0x2bc` | Slots set back to −1 | RNG |
|---|---|---|---|---|
| 2-1 | 750 | 2 | 6, 10 | – |
| 2-2 | 3000 | 3 | – | – (shares tank 1's level-4 case) |
| 2-3 | 5000 | 4 | – | – |
| 2-4 | 7500 | 5 | – | – |
| 2-5 | 10000 | `(r & 1) \| 4`: 4 or 5 | – | 1 app draw |

Other tanks, for reference:
- Tank 3: slots 0, 2, 3, 7 ($750), 8 ($2000), 10 ($2000), 11.
- Tank 4: slots 1 ($200), 2, 3, 4, 9 ($10000), 10 ($5000), 11.

RNG in `SetupShop`:
- **Tank 1 makes no RNG draws.**
- Levels 2-5, 3-2 and 3-5 each make **one app-`MTRand` draw** to set `+0x2bc`.
- With `app+0x880 != 0`, a different price table is used, every mapped slot is revealed immediately, and `0x00537c20` makes one `Sexy::Rand` draw. This path is never taken in normal Adventure.

At the end, mode 1 forces the egg price to 100. Mode 4 overrides it on tanks 2/3/4 (5000/15000/25000). `0x00538a10(n)` stores a backdrop index (`+0x3e8`) and is not shop-related.

`traced` for 1-1 and 1-2 (slots, prices and egg price drive the replayed purchases). Rows for 1-3+ and tanks 2–4 are `static`.

## Reveal

An Adventure level starts with **no buttons**.

`RevealSlotButton(slot, popAnim)` returns immediately if the slot is unmapped or already has a widget. Otherwise:
1. Allocates a `MenuButtonWidget` (ctor `0x005373c0`): id = slot, listener = `board+0x88` (the Board's ButtonListener sub-object), tooltip = `SlotName(slot)`.
2. Adds it to the **WidgetManager** (`board+0x10`, vtable +0xc). It is a top-level widget, not a child of the Board.
3. Sets button images: up `IMAGE_MENUBTNU`, over `IMAGE_MENUBTNO`, down `IMAGE_MENUBTND`.
4. Resizes it to `(X[pos], 3, 58, 60)` with **`X = {18, 87, 144, 217, 290, 363, 436}`**. The uneven 69/57/73 spacing is in the original.
5. Calls `RefreshSlotButton(slot, popAnim)`. Sets `revealed[slot] = 1` and `+0x2e0 += 1`.

Triggers on tank 1-1 come from `Fish::OnStageUp` @ `0x004f1840`. It is called from `Fish` vslot 78 @ `0x004f1ba0` immediately after a guppy's `stage (+0x1a0)` increments, and is skipped in mode 5:
- **Stage becomes 1** (small → medium) and slot 0 is not revealed: reveal slot 0.
  - In tutorial 1-1, also set `+0x4b2 = 1` (the "buy fish" help arrow).
  - If no message is showing (`[+0x118]+0xd0 < 1`), show "Your fish has grown! Good work!".
- **Stage becomes 2** (medium → large):
  - In mode 1, reveal slot 11.
  - On 1-1, if `+0x3af` (slot 11 revealed) is 0: reveal slot 11, set `+0x4b2 = 0` and `+0x4d8 = 1` ("buy egg piece" arrow). In tutorial 1-1, if no message is showing, also show "Buy 3 egg pieces to complete level!".
  - Other levels reveal their own slots here instead. For example, 1-2 reveals 2 with "Upgrade Food Quality to make food more nourishing!"; tank 1 levels 3+ reveal 2, 3 and 4.

So on 1-1, the guppy button appears when the first guppy reaches medium, and the egg button appears when the first guppy reaches large. **No reveal path uses RNG.**

`traced` for the 1-1 and 1-2 triggers (stage-up reveals and messages are compared), and for 1-3/1-4: slots 2, 3, 4 at the first large guppy, then 10 and 11 only when a carnivore is bought (`tank_1_3_from_start`, `tank_1_4_won`).

## Input path

1. `ButtonWidget::MouseDown` (vslot 54 @ `0x00475af0`) calls `listener->ButtonPress(id)` and then marks the widget dirty. `MenuButtonWidget` does not override it. **A purchase therefore fires on mouse-down.** Mouse-up (vslot 56 @ `0x00475b20`) calls `ButtonDepress`, which the Board only handles for id 0x7b (the Menu button). Clicks go through the WidgetManager, so standard widget-rect hit testing applies (58×60 at `(X[pos], 3)`; TODO: does ButtonWidget use per-pixel alpha?).
2. The Board's ButtonListener vtable is at `0x005a83e0`. Slot 0 is `0x0046ca30` and forwards to **`Board::ButtonPress(id)` @ `0x0054a370`**.
3. `ButtonPress` plays `SOUND_BUTTONCLICK` through app vtable +0xd8. Then, if `mode != 5 && id < 12`, it calls **`Board::Purchase(slot)` @ `0x00549380`**. Ids ≥ 12 are non-shop buttons. No RNG.
4. The press runs from WidgetManager mouse dispatch, **outside `Board::Update`**. Its order relative to the sim tick has to be pinned down with CE (TODO). The harness must inject purchases at the same point.

`traced` (purchases fire on mouse-down in the replays; the harness injects them at the input phase). The per-pixel hit-test question is open.

## Purchases

`Board::Purchase(slot)` @ `0x00549380`:
1. If `GetSlotButton(slot)` is null, return. **Unrevealed or unmapped slots cannot be bought.**
2. Take `price = +0x314[slot]` and switch on the slot.

`traced` for the slot-0/2/3/11 paths (see below).

### `Board::TrySpend(price, buzz)` @ `0x00540b30`

Every shop call passes `buzz = 1`. No RNG.
- Mode 5: return true and change nothing.
- If `app+0x880` is set: floor money at 5 first.
- **Fail** when `money < price` *and* `money + PendingCollectValue() < price`.
  - `PendingCollectValue` @ `0x0053a0b0` sums the values of already-clicked items still flying to the counter (`+0x198` flag on coins in `+0xa8` and `+0xec`; `+0x174` on larvae in `+0xc8`).
  - On fail:
    - If `buzz`, play `SOUND_BUZZER` (0x112).
    - Set the money-flash timer **`+0x450 = 70`**. When `buzz == 0`, this only happens if the timer is not already running.
    - If `money == 0` in tutorial 1-1, open dialog 0x13 "OUT OF MONEY!" ("…we'll float you a loan…", button "Click to Continue"). The loan itself is TODO.
    - Return false.
- **Succeed:** `money -= price`. Money **can go negative** when pending coins cover the gap; it settles when those coins land. With `app+0x880` set and `0 ≤ money < 5`, money becomes 5. Refresh the money label, then return true.

`traced` for success and pending coins. The failure path (buzzer, flash, OUT OF MONEY) is `static`; the replays have no failed purchase.

### Per-slot behaviour

Sounds go through `Board::PlaySound(id, gap=3, vol=1.0)` @ `0x00538230`. This throttles per id via `+0x1a4[id−0x106]` and uses no RNG. `Board::PlaySplash` @ `0x00538360` makes **one app-`MTRand` draw**: `Next() % 3` picks `SOUND_SPLASH` / `SPLASH2` / `SPLASH3`.

The RNG order shown is within the purchase call. Sound ids: `SOUND_BUY` = 0x111, `SOUND_GROW` = 0x122, `SOUND_SPLASHBIG` = 0x140.

- **0 guppy:**
  1. In tutorial 1-1, if `money < price` (pending coins are ignored here), show "You can't afford new fish yet! Collect more money!". This message appears **even if the TrySpend below then succeeds** on pending coins. That is original behaviour.
  2. `TrySpend`. On success: `AddRandomGuppy` (**11 draws**, see `objects/fish-guppy.md#spawn`), then `+0x4b2 = 0` (hide the arrow), then `SOUND_GROW`, then `PlaySplash` (**1 draw**). **12 app draws total**, `confirmed`.
- **1 breeder:** TrySpend → `0x005470a0` → `SOUND_GROW` → `PlaySplash`.
- **2 food quality:** only if `food_quality < 2` and TrySpend succeeds.
  1. `+0x4bf = 0`, then `food_quality += 1`, then Refresh.
  2. On 1-2 (if `+0x3a7 == 0`): reveal 3 and 11. Without `profile+0x59`, also show "Upgrade Food Quantity to drop more food at once!".
  3. `SOUND_BUY`.
- **3 food quantity:** only if `food_quantity < 9` and TrySpend succeeds.
  1. On tank 1 with quantity == 1 (and no `profile+0x59`), show "Hold down mouse button to Auto-Feed!".
  2. `+= 1`, then Refresh, then `SOUND_BUY`.
- **4 carnivore:** TrySpend → `0x00544c00` → on tank 4 with level > 1, reveal 9 → reveal 10 → reveal 11 → `SOUND_GROW` → `PlaySplash`. Tank 4 level 1 skips the reveal of 10.
- **5 star potion:** only if `+0x440 == 0` and TrySpend succeeds.
  1. On the first potion of the level (`+0x441`): if level < 3, show "Click on tank to drop star potion!" and set the message timer `[+0x118]+0xd0 = 180`.
  2. `+0x440 = 1`, then `SOUND_BUY`.
- **6 starcatcher:** TrySpend → `0x00545260` → reveal 10, 11 → `SOUND_GROW` → `PlaySplash`.
- **7 guppycruncher:** TrySpend → `0x00545430` → `SOUND_GROW` → `PlaySplash` → reveal 11 (level 1) or reveal 8 (level ≠ 1).
- **8 beetlemuncher:** TrySpend → `++(+0x4a4)` → `0x00545080` → reveal 10, 11 → `SOUND_GROW` → `PlaySplash`.
- **9 ultravore:** TrySpend → `0x00544e60` → reveal 10, 11 → `SOUND_GROW` → `SOUND_SPLASHBIG`. No `PlaySplash` here.
- **10 weapon:** only if `+0x3e4 < 12` and TrySpend succeeds. `+= 1`, then Refresh (at 12: "Hold down mouse button for Rapid Fire!", then the button goes to "MAX"), then `SOUND_BUY`. `traced` for laser 2→4 (`tank1_1.rs` `tank_1_4_won`: two upgrades, price 1000, laser compared through the alien damage); the 12 cap and its message are `static`.
- **11 egg piece:** `BuyEggPiece`, below. It does its own GetSlotButton check and TrySpend.

TODO: RNG inside the spawn functions for slots 1, 4, 6–9 (not reachable on 1-1).

`traced` for slots 0, 2, 3 and 11. Slots 1 and 4–10 are `static`.

## Egg pieces

`Board::BuyEggPiece` @ `0x00545960`. Counter = **`+0x43c`**, set to 1 by InitLevel (tank 5 forces 3). The level completes when the counter reaches **4**, so **1-1 takes 3 purchases at $150 each**. The price never changes in Adventure.

1. Return if the button doesn't exist. In mode 3, call `0x00552230` (end level) and return.
2. `TrySpend(price, 1)`. Return on failure.
3. Mode 1: the "buy random pet" variant. Out of scope here.
4. If `+0x4d8` (egg arrow) is set: clear it and set `+0x4e6 = 1`, `+0x4e8 = 0`. TODO: the follow-up hint state.
5. If the counter is 1 in tutorial 1-1 and no message is showing: show "Collect 2 more egg pieces to finish level!".
6. `counter += 1`. In mode 4 only: `+0x454 = 500` (inflation pause), and every slot except 11 is set back to its base price, with "Prices have been reset!" while the counter is < 4.
7. `RefreshSlotButton(11)` (the icon shows the new count), then `SOUND_BUY`.
8. If `counter ≥ 4` (**level complete**):
   1. Set `+0x3fc = ElapsedSeconds()` (`0x00538040`: `(now − +0x3b8) / 1000`) and `+0x4ee = 0`.
   2. **`Board::SettlePendingCoins` @ `0x0053c2d0`**: money += PendingCollectValue, clamped to 0..9999999, then the label is refreshed.
   3. Adventure without `profile+0x59`: record the level (`0x00514510`), advance the profile tank/level (`0x00501370`), unlock the pet with index `tank*5 + level − 6` (`0x00501420`; tank 5 uses 0x13 instead), and **open the hatch screen with `App::ShowHatchScreen` @ `0x0054be80`**.
   4. With `profile+0x59` set, the path goes through `0x0053dc10` instead.

**Steps 1–7 use no RNG**, so egg pieces 1 and 2 consume zero draws. Step 8 doesn't draw either. The hatch screen itself draws 2 app values per tick (`screens/hatch-screen.md#7-rng`).

`traced` (1-1 and 1-2 completions, including the third piece ending the level).

## Button art

`MenuButtonWidget` fields (widget-relative):

| Field | Meaning |
|---|---|
| `+0x124` | icon image |
| `+0x128`, `+0x12c` | icon x, y |
| `+0x130` | icon column (−1 = animated) |
| `+0x134` | icon row |
| `+0x13c` | pop-in countdown |
| `+0x140` | price text |
| `+0x15c` | secondary text |
| `+0x178` | price colour (110,250,110) |

Icons are set by `MenuButtonWidget::SetIcon` @ `0x00530410` from `RefreshSlotButton`:

| Slot | Image | x, y | col, row |
|---|---|---|---|
| 0 | `IMAGE_SMALLSWIM` | −12, −18 | animated, row 0 |
| 1 | `IMAGE_SCL_BREEDER` | 8, 2 | (−1 → animated), 0 |
| 2 | `IMAGE_FOOD` | 9, 3 | col 0, row = quality + 1 (at quality 2: MAX) |
| 3 | none | – | text `+0x15c = "%d"` of `min(qty+1, 9)` (at 9: MAX) |
| 4 | `IMAGE_SCL_OSCAR` | 8, 3 | animated |
| 5 | `IMAGE_FOOD` | 10, 1 | col 0, row 3 |
| 6 | `IMAGE_SCL_STARCATCHER` | 8, 2 | animated |
| 7 | `IMAGE_SCL_GRUBBER` | 8, 1 | animated |
| 8 | `IMAGE_SCL_GEKKO` | 10, 2 | animated |
| 9 | `IMAGE_SCL_ULTRA` | 10, 2 | animated |
| 10 | `IMAGE_LASERUPGRADES` | 7, 3 | col = weapon − 2, row 0 (at 12: MAX) |
| 11 | `IMAGE_EGGPIECES` | 7, 3 | **col = counter − 1** (0..2), row 0. Not drawn for counter outside 1..3 |

- **Animated column** = `(board+0x444 / 2) % image.numCols` (`+0x30` of the image).
- **"MAX"** (`0x00534450`): disables the button (`+0x7c = 0`, `+0x55 = 0`, vslot +0x98(0)), clears the secondary text, and sets the price text to "MAX".
- **Price text** = `"$%d"`, or empty if the price is < 1. It is only updated while `+0x55` is set.

`MenuButtonWidget::Draw` @ `0x00530540`, all relative to the button. While the pop-in countdown is < 7:
1. The ButtonWidget base draws its state image: **down** `MENUBTND` while the mouse is held on it, **over** `MENUBTNO` while hovered, else **up** `MENUBTNU`. A disabled (MAX) button always draws up.
2. The icon cel at `(x, y)`.
3. The secondary text in `FONT_CONTINUUMBOLD12OUTLINE`, colour (110,250,110), centred, baseline `ascent + 9`.

Then, in every state:
4. The price text in `FONT_TINY`, centred, baseline `ascent + 45`. Centring is integer: x = `(58 − width) / 2` truncated, so odd-width texts sit half a pixel left (`confirmed` by `tank13_carnivore_01`: "$100" and "$2000" and the "2" were 1 px off with rounding).
5. If the countdown is 1..6: `IMAGE_HATCHANIMATION` cel `(6 − c) / 2`.
6. `IMAGE_MENUBUTTONREFLECTION` at (0, 0). During pop-in it is clipped to y ≥ 40.

The countdown is set to 8 by `popAnim` and decrements once per tick in `MenuButtonWidget::Update` @ `0x00530500`, which is skipped while the board is paused (`+0x94`).

`confirmed` by the frame diff (`tank12_start_01`). Hover/pressed states are `static` (frames had the cursor outside the window).

## Bar, money counter, flash, help arrows

Drawn by `Board::Draw` @ `0x0053f5f0`, in board coordinates (640×480):
1. `IMAGE_MENUBAR` at (0, 0). Mode 5 can use an alternate bar image.
2. **Money flash:** `IMAGE_MONEYFLASH` at **(545, 39)** while `+0x450 > 0 && +0x450 % 20 < 10`. That gives 70 ticks of blinking, 10 ticks on and 10 off, starting "on". `+0x450` decrements by 1 per tick in `Board::Update` (`0x00548024`).
3. **Help arrows** (tutorial 1-1 only):
   - `+0x4b2` set: `IMAGE_HELPARROW` at (40, 70), plus "Click here to" at (65, 95) and "buy fish!" at (83, 107).
   - Else `+0x4d8` set: the arrow at (460, 70), plus "Click here to" at (485, 95) and "buy egg piece!" at (480, 107).
   - Font `FONT_JUNGLEFEVER10OUTLINE`, colour (110,250,110).
4. The "Tank %d-%d" label at (15, 470) and other overlays (not shop).

**Money counter** = a `Sexy::MyLabelWidget` at `board+0x438`, created in the Board ctor (`0x00542d06`) and added to the WidgetManager in InitLevel:
- Rect (535, 40, 80, font height); right-aligned (`+0xb4 = 2`).
- Font `FONT_CONTINUUMBOLD12`. Colour (180,255,90), or (250,155,150) in mode 5.
- Text `"%d"` of money, set by `Board::UpdateMoneyLabel` @ `0x0053a360` after every money change. Mode 5 shows a profile value instead.
- The text is drawn right-aligned to x = 615, vertically centred in the label.

The Menu button `board+0x114` (id 0x7b, `IMAGE_OPTIONSBUTTON`/`…D`) sits at (525, 3, 101, 29).

Bar and money counter: `confirmed` by the frame diff. Flash and help arrows: `static` (not in a captured frame).

## Mode notes (not used on 1-1)

- **Mode 4 price inflation** (`Board::Update` ~`0x00547bb1`): once slot 0 or 1 has been revealed, every N ticks of `+0x444` (N = 150/150/175/200 for tanks 1–4), and only while `+0x454 ≤ 0` and the counter ≤ 5, each live slot's price += its step (clamped). Mode 4 also reveals all buttons once `+0x444 > 10800`.
- **Mode 3** reveals all buttons and sets the slot 11 price to 0.
- **Tank 5** reveals slot 11 from `Board::Update` (`0x0054928e`) once its conditions hold.
- **Pets** (`FishTypePet` vslot 23 @ `0x004f7ad0`) can reveal all 12 slots after 200 ticks, with sound 0x13c.

`static`.

## Open questions

- Where a mouse-down purchase falls relative to `Board::Update` within a frame (CE).
- Whether ButtonWidget hit testing is per-pixel or rectangular.
- The loan amount and mechanism behind the OUT OF MONEY dialog (`0x00537e20` clears `+0x450`).
- RNG inside the other fish-spawn functions. (`ShowHatchScreen`: see `screens/hatch-screen.md`.)
