//! What a frame shows, without Bevy: sim state → a list of cels and text
//! images with screen positions, z order, tint, alpha and blend mode. The
//! Bevy app turns a [`Frame`] into sprites; [`crate::render`] composites it
//! on the CPU (for comparing with screenshots of the original).
//!
//! Draw rules come from the specs: guppies `fish-guppy.md#animation-and-drawing`,
//! food `food.md`, coins `coin.md`, Stinky `pet-stinky.md#drawing`, aliens,
//! warps, shots and corpses `alien-sylvester.md#7-drawing-and-sound-presentation`,
//! shadows `board.md#shadows`, dialogs `framework/dialog.md`, hatch screen
//! `screens/hatch-screen.md`.

use std::collections::HashMap;
use std::sync::Arc;

use insaniquarium_sim::alien::{Alien, DeadAlien, Shot};
use insaniquarium_sim::board::Dialog;
use insaniquarium_sim::game::{Game, Screen};
use insaniquarium_sim::hatch::Hatch;
use insaniquarium_sim::pet::Pet;
use insaniquarium_sim::shop::{BUTTON_X, BUTTON_Y, Slot};
use insaniquarium_sim::{Board, Fish};
use sexy_core::anim::AnimInfo;
use sexy_core::data_dir::DataDir;
use sexy_core::image::Rgba;
use sexy_core::resources::{Anim, AnimType, Manifest, Resource};

use crate::assets::GameImages;
use crate::text::ImageFont;

/// The board's size in pixels.
pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;

/// Every sheet the tank and hatch screens draw cels from.
pub const SHEETS: &[&str] = &[
    "IMAGE_AQUARIUM1",
    "IMAGE_SMALLSWIM",
    "IMAGE_SMALLTURN",
    "IMAGE_SMALLEAT",
    "IMAGE_HUNGRYSWIM",
    "IMAGE_HUNGRYTURN",
    "IMAGE_HUNGRYEAT",
    "IMAGE_FOOD",
    "IMAGE_MONEY",
    "IMAGE_MENUBAR",
    "IMAGE_MENUBTNU",
    "IMAGE_MENUBTND",
    "IMAGE_MENUBTNO",
    "IMAGE_MENUBUTTONREFLECTION",
    "IMAGE_EGGPIECES",
    "IMAGE_HELPARROW",
    "IMAGE_MONEYFLASH",
    "IMAGE_HATCHANIMATION",
    "IMAGE_SHADOW",
    "IMAGE_STINKY",
    "IMAGE_ZZZ",
    "IMAGE_SYLV",
    "IMAGE_WARPHOLE",
    "IMAGE_WARPGLOW",
    "IMAGE_LASERS",
    "IMAGE_CHOMP",
    "IMAGE_EXPLOSION",
    "IMAGE_EXPLOSIONSMALL",
    "IMAGE_EXPLOSIONTINY",
    "IMAGE_SMOKETINY",
    "IMAGE_SMOKESMALL",
    "IMAGE_SMOKEMISSILE",
    "IMAGE_HATCHSCREEN",
    "IMAGE_HATCHREFLECTION",
    "IMAGE_EGGCRACK1",
    "IMAGE_EGGCRACK2",
    "IMAGE_EGGSHARDS",
    "IMAGE_NIKO",
    "IMAGE_ITCHY",
    "IMAGE_PREGO",
    "IMAGE_ZORF",
    "IMAGE_WAVE",
    "IMAGE_WAVESIDE",
    "IMAGE_TANKLIGHTING",
    "IMAGE_TANKMASK1",
    "IMAGE_SMALLDIE",
];

/// A sheet cels are cut from: the image, one cel's size and the grid
/// (cols, rows).
#[derive(Clone)]
pub struct Sheet {
    pub image: Arc<Rgba>,
    pub cel: (u32, u32),
    pub grid: (u32, u32),
}

impl Sheet {
    /// The source rect (x, y, w, h) of cel (`col`, `row`); `None` when out
    /// of range (`DrawImageCel` draws nothing then).
    pub fn cel_rect(&self, col: i32, row: i32) -> Option<(u32, u32, u32, u32)> {
        if col < 0 || row < 0 || col as u32 >= self.grid.0 || row as u32 >= self.grid.1 {
            return None;
        }
        let (w, h) = self.cel;
        Some((col as u32 * w, row as u32 * h, w, h))
    }
}

pub type Sheets = HashMap<&'static str, Sheet>;

/// The [`SHEETS`] with their cel size and grid. Missing images are skipped
/// (with a warning).
pub fn sheets(images: &GameImages, manifest: &Manifest) -> Sheets {
    let mut sheets = HashMap::new();
    for &id in SHEETS {
        let Some(img) = images.get(id) else {
            eprintln!("warning: missing {id}");
            continue;
        };
        let (cols, rows) = match manifest.get(id) {
            Some(Resource::Image(i)) => (i.cols.max(1), i.rows.max(1)),
            _ => (1, 1),
        };
        // Fish/food/money sheets declare no grid in the manifest; the
        // game draws them as 80×80 (fish) / 40×40 / 72×72 cels, 10 across.
        let ten_across = matches!(
            id,
            "IMAGE_SMALLSWIM"
                | "IMAGE_SMALLTURN"
                | "IMAGE_SMALLEAT"
                | "IMAGE_HUNGRYSWIM"
                | "IMAGE_HUNGRYTURN"
                | "IMAGE_HUNGRYEAT"
                | "IMAGE_SMALLDIE"
                | "IMAGE_FOOD"
                | "IMAGE_MONEY"
        );
        // Sheets whose grid the manifest omits: 17 warp frames, 20 smoke.
        let (cols, rows) = match id {
            "IMAGE_WARPHOLE" | "IMAGE_WARPGLOW" => (17, 1),
            "IMAGE_SMOKETINY" | "IMAGE_SMOKESMALL" | "IMAGE_SMOKEMISSILE" => (20, 1),
            "IMAGE_EXPLOSION" | "IMAGE_EXPLOSIONSMALL" | "IMAGE_EXPLOSIONTINY" => (10, 1),
            "IMAGE_CHOMP" => (5, 1),
            _ => (cols, rows),
        };
        let (cel, grid) = if ten_across {
            let w = img.width / 10;
            let h = w.min(img.height);
            ((w, h), (10, img.height / h))
        } else {
            ((img.width / cols, img.height / rows), (cols, rows))
        };
        sheets.insert(
            id,
            Sheet {
                image: Arc::new(img.clone()),
                cel,
                grid,
            },
        );
    }
    sheets
}

/// One cel to draw: column/row in `sheet`, top-left at game pixel (`x`, `y`).
/// Art faces left; `mirror` flips it per cel.
#[derive(Clone, Debug, PartialEq)]
pub struct Cel {
    pub sheet: &'static str,
    pub col: i32,
    pub row: i32,
    pub x: i32,
    pub y: i32,
    /// Draw order: higher is in front.
    pub z: f32,
    pub mirror: bool,
    /// 0..=255 opacity (colourised draw in the original).
    pub alpha: u8,
    /// Colourise tint (RGB); white = none.
    pub tint: [u8; 3],
    /// Draw mode 1 (additive).
    pub additive: bool,
    /// Destination size, if stretched (`DrawImageMirror` with a dest rect).
    pub size: Option<(i32, i32)>,
}

impl Cel {
    pub fn new(sheet: &'static str, col: i32, row: i32, x: i32, y: i32, z: f32) -> Self {
        Cel {
            sheet,
            col,
            row,
            x,
            y,
            z,
            mirror: false,
            alpha: 255,
            tint: [255; 3],
            additive: false,
            size: None,
        }
    }

    pub fn mirror(mut self, m: bool) -> Self {
        self.mirror = m;
        self
    }

    pub fn alpha(mut self, a: i32) -> Self {
        self.alpha = a.clamp(0, 255) as u8;
        self
    }

    pub fn tint(mut self, rgb: [u8; 3]) -> Self {
        self.tint = rgb;
        self
    }

    pub fn additive(mut self) -> Self {
        self.additive = true;
        self
    }
}

/// A pre-rendered text image with its top-left at (`x`, `y`), multiplied
/// by `colour` (white for images that are already coloured: dialogs and
/// the hatch-screen layers). The image is shared with the [`Fonts`] cache,
/// so the same `Arc` comes back while the text is unchanged.
#[derive(Clone, Debug)]
pub struct Text {
    pub image: Arc<Rgba>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub colour: [u8; 3],
}

/// Everything one frame shows. Cels and texts share one z order.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub cels: Vec<Cel>,
    pub texts: Vec<Text>,
}

/// The fonts the tank and hatch screens use, and their rendered text,
/// re-rasterised only when it changes.
pub struct Fonts {
    /// `FONT_CONTINUUMBOLD12`: the money counter.
    font: Option<ImageFont>,
    /// Dialog header / body / button fonts (`framework/dialog.md`).
    dialog_fonts: Option<[ImageFont; 3]>,
    /// Hatch-screen fonts: 17, 15, 12 and 10 outline.
    hatch_fonts: Option<[ImageFont; 4]>,
    /// `FONT_TINY` for button prices.
    tiny: Option<ImageFont>,
    /// `FONT_CONTINUUMBOLD12OUTLINE` for button secondary text (the food
    /// quantity number).
    outline12: Option<ImageFont>,
    /// `FONT_JUNGLEFEVER10OUTLINE`: help-arrow captions and the tank label.
    small: Option<ImageFont>,
    /// `FONT_CONTINUUMBOLD14OUTLINE` and `FONT_CONTINUUMBOLD14`: the
    /// message line (board.md#message-line).
    message_fonts: Option<[ImageFont; 2]>,
    message: Option<((String, bool), Arc<Rgba>)>,
    /// "Tank t-l" in the bottom-left corner.
    tank_label: Option<((i32, i32), Arc<Rgba>)>,
    money: Option<(i32, Arc<Rgba>)>,
    dialog: Option<(Dialog, Arc<Rgba>)>,
    /// The dialog's button rect in screen pixels (x, y, w, h).
    dialog_button: Option<(i32, i32, i32, i32)>,
    /// Hatch screen for a pet: title bar with "You have found:", the name,
    /// the description (each a full 640×480 overlay), and the Continue
    /// button cut out as "Please Wait..." and "Click Here to Continue".
    hatch: Option<(i32, [Arc<Rgba>; 5])>,
    labels: HashMap<String, Arc<Rgba>>,
}

impl Fonts {
    /// Load the fonts from the install; missing ones just draw nothing.
    pub fn load(data: &DataDir) -> Self {
        let font = |p: &str| ImageFont::load(data, p).ok();
        Fonts {
            font: font("data/ContinuumBold12.txt"),
            dialog_fonts: [
                "data/JungleFever15outline.txt",
                "data/JungleFever10outline.txt",
                "data/JungleFever12outline.txt",
            ]
            .map(font)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok()),
            hatch_fonts: [
                "data/JungleFever17outline.txt",
                "data/JungleFever15outline.txt",
                "data/JungleFever12outline.txt",
                "data/JungleFever10outline.txt",
            ]
            .map(font)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok()),
            tiny: font("data/Pix118.txt"),
            outline12: font("data/ContinuumBold12outline.txt"),
            small: font("data/JungleFever10outline.txt"),
            message_fonts: [
                "data/ContinuumBold14outback.txt",
                "data/ContinuumBold14.txt",
            ]
            .map(font)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok()),
            message: None,
            tank_label: None,
            money: None,
            dialog: None,
            dialog_button: None,
            hatch: None,
            labels: HashMap::new(),
        }
    }

    /// The open dialog's button rect (x, y, w, h) in screen pixels, once
    /// its image has been rendered.
    pub fn dialog_button(&self) -> Option<(i32, i32, i32, i32)> {
        self.dialog_button
    }

    /// Render whatever text the current screen needs and isn't cached yet.
    fn update(&mut self, game: &Game, images: &GameImages) {
        if let Screen::Hatch(h) = &game.screen {
            let hatch = &h.hatch;
            if self.hatch.as_ref().is_none_or(|h| h.0 != hatch.pet)
                && let Some(fonts) = &self.hatch_fonts
                && let Some(imgs) = render_hatch_text(images, fonts, hatch.pet)
            {
                self.hatch = Some((hatch.pet, imgs.map(Arc::new)));
            }
            return;
        }
        let Some(b) = game.board() else { return };
        if let Some(t) = quantity_text(b)
            && !self.labels.contains_key(&t)
            && let Some(f) = &self.outline12
        {
            let img = f.render(&t.as_bytes()[1..]);
            self.labels.insert(t, Arc::new(img));
        }
        let mut want: Vec<(String, bool)> = Slot::ALL
            .into_iter()
            .filter(|&s| b.shop.revealed[s as usize])
            .map(|s| (price_text(b, s), true))
            .collect();
        for (_, lines) in arrows(b) {
            for (t, _, _) in lines {
                want.push((t.to_string(), false));
            }
        }
        for (text, tiny) in want {
            if text.is_empty() || self.labels.contains_key(&text) {
                continue;
            }
            let font = if tiny { &self.tiny } else { &self.small };
            if let Some(f) = font {
                let img = f.render(text.as_bytes());
                self.labels.insert(text, Arc::new(img));
            }
        }
        let key = (b.message.text.clone(), b.message.warning);
        if self.message.as_ref().is_none_or(|m| m.0 != key)
            && let Some(fonts) = &self.message_fonts
        {
            let img = render_message(fonts, &b.message.text, b.message.warning);
            self.message = Some((key, Arc::new(img)));
        }
        let tl = (b.tank, b.level);
        if self.tank_label.as_ref().is_none_or(|l| l.0 != tl)
            && let Some(f) = &self.font
        {
            let text = format!("Tank {}-{}", b.tank, b.level);
            self.tank_label = Some((tl, Arc::new(f.render(text.as_bytes()))));
        }
        let money = b.money;
        if self.money.as_ref().is_none_or(|m| m.0 != money)
            && let Some(f) = &self.font
        {
            self.money = Some((money, Arc::new(f.render(money.to_string().as_bytes()))));
        }
        match b.dialog {
            Some(d) if self.dialog.as_ref().is_none_or(|x| x.0 != d) => {
                if let Some((img, button)) = self
                    .dialog_fonts
                    .as_ref()
                    .and_then(|f| render_dialog(images, f, d))
                {
                    self.dialog = Some((d, Arc::new(img)));
                    self.dialog_button = Some(button);
                }
            }
            None => {
                self.dialog = None;
                self.dialog_button = None;
            }
            _ => {}
        }
    }

    fn tank_text(&self, b: &Board) -> Vec<Text> {
        let mut out = Vec::new();
        let mut text = |image: &Arc<Rgba>, x: f32, y: f32, z: f32, colour: [u8; 3]| {
            out.push(Text {
                image: image.clone(),
                x,
                y,
                z,
                colour,
            })
        };
        // Money label: rect (535, 40, 80, h), right-aligned → right edge 615
        // (shop-bar.md#bar-money-counter-flash-help-arrows). The rendered
        // bitmap has a 4 px pad on each side.
        if let Some((_, img)) = &self.money {
            let x = 615.0 + 4.0 - img.width as f32;
            text(img, x, 40.0 - 4.0, 11.6, [180, 255, 90]);
        }
        // Button prices: FONT_TINY centred on the 58-wide button, baseline
        // ascent + 45 (shop-bar.md#button-art); shown once the pop-in ends.
        for slot in Slot::ALL {
            let i = slot as usize;
            let (Some(pos), true) = (b.shop.position[i], b.shop.revealed[i]) else {
                continue;
            };
            if let Some(img) = self.labels.get(&price_text(b, slot)) {
                let x = BUTTON_X[pos] as f32 + (58.0 - img.width as f32) / 2.0;
                text(img, x, BUTTON_Y as f32 + 45.0 - 4.0, 11.25, [110, 250, 110]);
            }
        }
        // Food quantity number: centred, baseline ascent + 9, (110, 250, 110),
        // drawn while the button isn't popping in.
        let qi = Slot::FoodQuantity as usize;
        if let (Some(t), Some(pos), true) =
            (quantity_text(b), b.shop.position[qi], b.shop.pop[qi] < 7)
            && let Some(img) = self.labels.get(&t)
        {
            let x = BUTTON_X[pos] as f32 + (58.0 - img.width as f32) / 2.0;
            // Baseline at ascent + 9 puts the line's top 9 px below the button
            // top; the bitmap has a 4 px pad.
            let y = (BUTTON_Y + 9) as f32 - 4.0;
            text(img, x, y, 11.15, [110, 250, 110]);
        }
        // Help-arrow captions (JungleFever10outline, (110, 250, 110)) at their
        // baselines; the bitmap's top is ascent above the baseline.
        for (_, lines) in arrows(b) {
            for (t, x, y) in lines {
                if let Some(img) = self.labels.get(t) {
                    let (x, y) = (x as f32 - 4.0, (y - 11) as f32 - 4.0);
                    text(img, x, y, 11.55, [110, 250, 110]);
                }
            }
        }
        // "Tank %d-%d" (`Board::Draw` @ `0x0053fcd8`): FONT_CONTINUUMBOLD12,
        // baseline (15, 470), colour by backdrop (tank 1: (165, 140, 80)).
        // TODO(spec): bonus rounds and other modes print other labels.
        if let (Some((_, img)), Some(f)) = (&self.tank_label, &self.font) {
            let colour = match b.tank {
                2 => [220, 150, 150],
                3 => [125, 200, 215],
                4 => [60, 180, 80],
                _ => [165, 140, 80],
            };
            text(
                img,
                15.0 - 4.0,
                (470 - f.ascent()) as f32 - 4.0,
                11.5,
                colour,
            );
        }
        if b.message.drawn()
            && let Some((_, img)) = &self.message
        {
            text(
                img,
                MESSAGE_RECT.0 as f32,
                MESSAGE_RECT.1 as f32,
                11.7,
                [255; 3],
            );
        }
        if let Some((_, img)) = &self.dialog {
            text(img, DIALOG_POS.0, DIALOG_POS.1, 12.0, [255; 3]);
        }
        out
    }

    fn hatch_text(&self, h: &Hatch) -> Vec<Text> {
        let Some((_, [title, name, desc, wait, cont])) = &self.hatch else {
            return Vec::new();
        };
        let at = |image: &Arc<Rgba>, x: i32, y: i32, z: f32| Text {
            image: image.clone(),
            x: x as f32,
            y: y as f32,
            z,
            colour: [255; 3],
        };
        let mut out = vec![at(title, 0, 0, 5.0)];
        if h.t > 140 {
            out.push(at(name, 0, 0, 5.1));
        }
        if h.t > 170 {
            out.push(at(desc, 0, 0, 5.2));
        }
        // The button: "Please Wait..." until clickable, then the real label.
        let (x, y, _, _) = HATCH_CONTINUE;
        out.push(at(if h.can_continue() { cont } else { wait }, x, y, 19.0));
        out
    }
}

/// Everything the app draws for the current screen (tank or hatch).
/// `cursor` is the mouse in board pixels and `mouse_down` the left button
/// (button hover/press art). Renders and caches any new text in `fonts`.
pub fn frame(
    game: &Game,
    cursor: Option<(i32, i32)>,
    mouse_down: bool,
    fonts: &mut Fonts,
    images: &GameImages,
) -> Frame {
    fonts.update(game, images);
    match &game.screen {
        Screen::Tank(b) => Frame {
            cels: tank_cels(b, cursor, mouse_down),
            texts: fonts.tank_text(b),
        },
        Screen::Hatch(h) => Frame {
            cels: hatch_cels(&h.hatch),
            texts: fonts.hatch_text(&h.hatch),
        },
    }
}

/// Guppy sheet, cel, row and mirroring (fish-guppy.md#animation-and-drawing).
fn guppy_cel(f: &Fish) -> (&'static str, i32, i32, bool) {
    guppy_cel_as(f, f.looks_hungry())
}

/// As [`guppy_cel`], but with the hungry/fed sheet forced.
fn guppy_cel_as(f: &Fish, hungry: bool) -> (&'static str, i32, i32, bool) {
    let sheet = match (
        f.turn_timer != 0,
        f.eat_timer >= 1 || f.timer_108 >= 101,
        hungry,
    ) {
        (true, _, false) => "IMAGE_SMALLTURN",
        (true, _, true) => "IMAGE_HUNGRYTURN",
        (false, true, false) => "IMAGE_SMALLEAT",
        (false, true, true) => "IMAGE_HUNGRYEAT",
        (false, false, false) => "IMAGE_SMALLSWIM",
        (false, false, true) => "IMAGE_HUNGRYSWIM",
    };
    let row = match f.stage {
        3 => 2,
        4 => 3,
        s => s.clamp(0, 2),
    };
    let mirror = match f.turn_timer {
        t if t > 0 => true,
        t if t < 0 => false,
        _ => f.vx > 0.0 || (f.vx == 0.0 && f.facing > 0.0),
    };
    (sheet, f.cel, row, mirror)
}

/// Shop buttons, help arrows and money flash (shop-bar.md#button-art).
fn shop_cels(b: &Board, cursor: Option<(i32, i32)>, down: bool, cels: &mut Vec<Cel>) {
    // Button art by state (ButtonWidget::Draw): pressed (held on it) →
    // MENUBTND, hovered → MENUBTNO, else MENUBTNU. MAXed slots are
    // disabled and always draw the up image.
    let hovered = cursor
        .and_then(|(x, y)| b.shop.hit(x, y))
        .filter(|&s| !maxed(b, s) && b.dialog.is_none());
    for slot in Slot::ALL {
        let i = slot as usize;
        let (Some(pos), true) = (b.shop.position[i], b.shop.revealed[i]) else {
            continue;
        };
        let (bx, by) = (BUTTON_X[pos], BUTTON_Y);
        let pop = b.shop.pop[i];
        if pop < 7 {
            let art = match (hovered == Some(slot), down) {
                (true, true) => "IMAGE_MENUBTND",
                (true, false) => "IMAGE_MENUBTNO",
                _ => "IMAGE_MENUBTNU",
            };
            cels.push(Cel::new(art, 0, 0, bx, by, 11.0));
            match slot {
                Slot::Guppy => cels.push(Cel::new(
                    "IMAGE_SMALLSWIM",
                    (b.level_ticks / 2) % 10,
                    0,
                    bx - 12,
                    by - 18,
                    11.1,
                )),
                // Food quality: the next pellet's row (`0x00540724`); at the
                // cap the button reads MAX and has no icon.
                Slot::FoodQuality if b.food_level < 2 => cels.push(Cel::new(
                    "IMAGE_FOOD",
                    0,
                    b.food_level + 1,
                    bx + 9,
                    by + 3,
                    11.1,
                )),
                Slot::EggPiece if (1..=3).contains(&b.egg_count) => cels.push(Cel::new(
                    "IMAGE_EGGPIECES",
                    b.egg_count - 1,
                    0,
                    bx + 7,
                    by + 3,
                    11.1,
                )),
                _ => {}
            }
        }
        if (1..=6).contains(&pop) {
            cels.push(Cel::new(
                "IMAGE_HATCHANIMATION",
                (6 - pop) / 2,
                0,
                bx,
                by,
                11.2,
            ));
        }
        cels.push(Cel::new("IMAGE_MENUBUTTONREFLECTION", 0, 0, bx, by, 11.3));
    }
    if b.money_flash > 0 && b.money_flash % 20 < 10 {
        cels.push(Cel::new("IMAGE_MONEYFLASH", 0, 0, 545, 39, 11.4));
    }
    for (x, _) in arrows(b) {
        cels.push(Cel::new("IMAGE_HELPARROW", 0, 0, x, 70, 11.5));
    }
}

/// A help arrow: its x and two caption lines (text, x, baseline y).
type Arrow = (i32, [(&'static str, i32, i32); 2]);

/// Help arrows and their two caption lines, left x first (`0x0053f6a0`):
/// tutorial "buy fish"/"buy egg piece", and 1-2's food-quality hint.
fn arrows(b: &Board) -> Vec<Arrow> {
    let mut v = Vec::new();
    if b.arrow_buy_fish {
        v.push((40, [("Click here to", 65, 95), ("buy fish!", 83, 107)]));
    } else if b.arrow_buy_egg {
        v.push((
            460,
            [("Click here to", 485, 95), ("buy egg piece!", 480, 107)],
        ));
    } else if b.arrow_food_quality && b.tank == 1 && b.level == 2 {
        v.push((110, [("Click here to", 135, 95), ("upgrade!", 152, 107)]));
    }
    v
}

/// Floor shadows (board.md#shadows): tank 1 colour (66, 48, 41).
fn shadow_cels(b: &Board, cels: &mut Vec<Cel>) {
    const TINT: [u8; 3] = [66, 48, 41];
    // Types 0 (fish) and 2 (alien) follow the owner's height and stretch.
    let mut moving = |ix: i32, iy: i32, kind2: bool| {
        let dark = (iy - 50) / 2;
        let x = ix + if kind2 { 40 } else { 0 };
        let y = 415 - ((370 - iy).max(0) * 30) / 370;
        let alpha = if kind2 {
            (2 * dark).clamp(0, 255)
        } else {
            dark.clamp(0, 150)
        };
        let mut f = ((y - iy) as f32 / 100.0).clamp(0.9, 1.3);
        if kind2 {
            f += 0.4;
        }
        let g = f - 1.0;
        let (dx, dy) = ((80.0 * g) as i32, (40.0 * g) as i32);
        let mut c = Cel::new("IMAGE_SHADOW", 0, 0, x - dx, y - dy, 0.5)
            .alpha(alpha)
            .tint(TINT);
        c.size = Some((80 + 2 * dx, 40 + 2 * dy));
        cels.push(c);
    };
    for f in &b.guppies {
        moving(f.widget_x, f.widget_y, false);
    }
    for a in &b.aliens {
        if a.warp < 10 {
            moving(a.widget_x, a.widget_y, true);
        }
    }
    // Type 1 (Stinky): fixed floor y, unstretched.
    for p in &b.pets {
        let dark = (p.widget_y - 50) / 2;
        cels.push(
            Cel::new("IMAGE_SHADOW", 0, 0, p.widget_x, p.shadow_y, 0.5)
                .alpha(dark.clamp(0, 150))
                .tint(TINT),
        );
    }
}

/// Stinky (pet-stinky.md#drawing).
fn pet_cels(p: &Pet, z: f32, cels: &mut Vec<Cel>) {
    let mirror = if p.shell > 0 {
        p.facing >= 0.0
    } else if p.turn != 0 {
        p.turn > 0
    } else {
        p.vx > 0.0 || (p.vx == 0.0 && p.facing > 0.0)
    };
    if p.slow {
        let zx = if mirror { 55 } else { 25 };
        cels.push(Cel::new(
            "IMAGE_ZZZ",
            0,
            0,
            p.widget_x + zx,
            p.widget_y - 14,
            z,
        ));
    }
    let row = if p.shell != 0 {
        2
    } else if p.turn != 0 {
        1
    } else {
        0
    };
    cels.push(
        Cel::new(
            "IMAGE_STINKY",
            p.cel,
            row,
            p.widget_x,
            p.widget_y,
            z + 0.0001,
        )
        .mirror(mirror),
    );
}

/// Sylvester (alien-sylvester.md#71-alien).
fn alien_cels(a: &Alien, z: f32, cels: &mut Vec<Cel>) {
    let (row, mirror) = if a.turn != 0 {
        (1, a.turn > 0)
    } else {
        (0, a.vx >= 0.0)
    };
    let base = Cel::new("IMAGE_SYLV", a.cel, row, a.widget_x, a.widget_y, z).mirror(mirror);
    if a.warp > 0 {
        if a.warp > 9 {
            return;
        }
        let s = (160.0 * a.warp as f32 / 10.0) as i32;
        let mut c = base;
        c.x += s / 2;
        c.y += s / 2;
        c.size = Some((160 - s, 160 - s));
        cels.push(c);
        return;
    }
    cels.push(base.clone());
    if a.cooldown > 0 {
        cels.push(
            Cel {
                z: z + 0.0001,
                ..base
            }
            .additive()
            .alpha(25 * a.cooldown),
        );
    }
}

/// Laser colours by level (`0x004d9160`, `0x004d9280`).
fn laser_colours(laser: i32) -> ([u8; 3], [u8; 3]) {
    let base = match laser {
        3 | 10 => [255, 0, 0],
        4 => [255, 55, 0],
        5 => [255, 200, 0],
        6 => [125, 255, 90],
        7 => [10, 200, 35],
        8 => [5, 150, 250],
        9 => [160, 0, 255],
        12 => [235, 0, 215],
        _ => [50, 180, 200],
    };
    let high = match laser {
        4 | 7 => [255, 255, 100],
        6 => [255, 255, 0],
        10 => [100, 255, 255],
        11 => [180, 225, 70],
        12 => [0, 255, 0],
        _ => [255, 255, 255],
    };
    (base, high)
}

/// Shots (alien-sylvester.md#73-shot). Nothing while the start delay runs.
fn shot_cels(s: &Shot, laser: i32, z: f32, cels: &mut Vec<Cel>) {
    if s.delay != 0 {
        return;
    }
    let (x, y) = (s.widget_x, s.widget_y);
    let one = |sheet| Cel::new(sheet, s.frame, 0, x, y, z).alpha(s.r);
    match s.kind {
        0 => {
            let pair = match laser {
                ..7 => 0,
                7..10 => 1,
                _ => 2,
            } * 2;
            let (base, high) = laser_colours(laser);
            let c = Cel::new("IMAGE_LASERS", s.frame, pair, x, y, z);
            cels.push(c.clone().tint(base).alpha(100));
            cels.push(
                Cel {
                    z: z + 0.0001,
                    ..c.clone()
                }
                .tint(base)
                .alpha(s.r)
                .additive(),
            );
            cels.push(
                Cel {
                    row: pair + 1,
                    z: z + 0.0002,
                    ..c
                }
                .tint(high)
                .alpha(s.r)
                .additive(),
            );
        }
        1 => cels.push(one("IMAGE_CHOMP")),
        3 => cels.push(one("IMAGE_EXPLOSION").additive()),
        4 => cels.push(one("IMAGE_EXPLOSIONSMALL").additive()),
        5 => cels.push(one("IMAGE_EXPLOSIONTINY")),
        6 => cels.push(one("IMAGE_SMOKETINY")),
        7 => cels.push(one("IMAGE_SMOKESMALL")),
        8 => cels.push(one("IMAGE_SMOKEMISSILE")),
        // Kind 2 (hit spark) draws nothing.
        _ => {}
    }
}

/// Sinking corpse (alien-sylvester.md#74-dead-alien).
fn dead_alien_cels(d: &DeadAlien, z: f32, cels: &mut Vec<Cel>) {
    let c = Cel::new("IMAGE_SYLV", d.cel, 0, d.widget_x, d.widget_y, z)
        .mirror(d.facing_right)
        .alpha((d.fade * 255.0) as i32);
    cels.push(c.clone());
    if d.life > 115 {
        // Five additive passes: a white flash fading over 10 ticks.
        let a = ((d.life - 115) * 20 * 5).min(255);
        cels.push(Cel { z: z + 0.0001, ..c }.additive().alpha(a));
    }
}

/// The food-quantity button's number: pellets after the next upgrade,
/// `min(qty + 1, 9)`; none at MAX (`shop-bar.md#button-art`). Keyed with
/// a `#` prefix in the label cache (it's a different font).
fn quantity_text(b: &Board) -> Option<String> {
    let i = Slot::FoodQuantity as usize;
    (b.shop.revealed[i] && !maxed(b, Slot::FoodQuantity))
        .then(|| format!("#{}", (b.max_food + 1).min(9)))
}

/// A slot at its cap: the button reads MAX and is disabled (`0x00534450`).
fn maxed(b: &Board, slot: Slot) -> bool {
    match slot {
        Slot::FoodQuality => b.food_level >= 2,
        Slot::FoodQuantity => b.max_food >= 9,
        _ => false,
    }
}

/// Button price text: `$n`, or MAX at a cap (`0x00534450`).
fn price_text(b: &Board, slot: Slot) -> String {
    let price = b.shop.price[slot as usize];
    if maxed(b, slot) {
        "MAX".into()
    } else if price > 0 {
        format!("${price}")
    } else {
        String::new()
    }
}

/// Header and body of a board dialog (`framework/dialog.md#board-dialogs`).
fn dialog_text(d: Dialog) -> (&'static str, &'static str) {
    match d {
        Dialog::AlienWarning => (
            "DANGER!",
            "A vicious alien is about to enter your tank! Defeat it with your laser weapon by clicking on it!",
        ),
        Dialog::LastFishDied => (
            "YOUR LAST FISH HAS DIED!",
            "Normally this would end your game, but this time we'll give you another fish to keep playing! Make sure you keep it well fed!",
        ),
        Dialog::BattleTip => (
            "BATTLE TIP",
            "Shoot the alien's head to push it downwards! Shoot its tail to deflect it upwards!",
        ),
        Dialog::NewBreed => (
            "WARNING!",
            "A new breed of alien is fast approaching!  Lasers can't hurt this baddie, so find another way to defeat it!",
        ),
        Dialog::GameOver => ("GAME OVER", "Oops!  All of your fish have died!"),
        Dialog::OutOfMoney => (
            "OUT OF MONEY!",
            "You've run out of money!  Because you're new, we'll float you a loan, but be careful next time!",
        ),
    }
}

/// A `MoneyDialog` rendered to one image (`framework/dialog.md`): the panel
/// as a 3×3 box, the header, the wrapped body and the "Click to Continue"
/// button. Drawn at `DIALOG_POS`.
fn render_dialog(
    img: &GameImages,
    [head, body, btn]: &[ImageFont; 3],
    d: Dialog,
) -> Option<(Rgba, (i32, i32, i32, i32))> {
    let panel = img.get("IMAGE_DIALOG")?;
    let button = img.get("IMAGE_DIALOGBUTTON")?;
    let (header, text) = dialog_text(d);
    let (left, top, right, bottom) = (36, 15, 36, 36);
    let w = (head.width(header.as_bytes()) + 150).max(348);
    let inner = w - left - right;
    let lines = body.wrap(text, inner - 4);
    let bh = button.height as i32;
    // Dialog::GetPreferredHeight (`0x00476570`): insets, the header line,
    // 10 px, the body lines (each line spacing apart), 8 px, the button.
    let body_h = lines.len() as i32 * body.line_spacing();
    let preferred = top + head.line_spacing() + 10 + body_h + 8 + bh + bottom;
    // `NewDialog` (`0x0054b940`): at least 2/3 of the panel image + 10.
    let h = preferred.max(panel.height as i32 * 2 / 3 + 10);
    let mut out = Rgba::new_transparent(w as u32, h as u32);
    out.draw_image_box(panel, (0, 0, w, h));
    // `Dialog::Draw` (`0x00476710`): the header centred, baseline at
    // top inset (15) − ascentPadding + ascent; the body word-wrapped into the
    // rect (left + 2, header baseline − ascent + height + 15, inner − 4),
    // each line centred, baseline at rect y + ascent − ascentPadding
    // (measured: the body's ink matches the original's to the pixel).
    let hy = top - head.ascent_padding() + head.ascent();
    let hx = left + (inner - head.width(header.as_bytes())) / 2;
    head.draw(&mut out, header.as_bytes(), hx, hy, [255, 200, 0]);
    let (bx0, bw0) = (left + 2, inner - 4);
    // + 15: the dialog's header-to-body gap (`+0x12c`).
    let mut y =
        hy - head.ascent() + head.glyph_height() + 15 + body.ascent() - body.ascent_padding();
    for l in &lines {
        let x = bx0 + (bw0 - body.width(l.as_bytes())) / 2;
        body.draw(&mut out, l.as_bytes(), x, y, [255, 255, 255]);
        y += body.line_spacing();
    }
    // The button: its normal-state third of DIALOGBUTTON as a 3×3 box.
    let normal = button.crop(0, 0, button.width / 3, button.height);
    let (bx, by, bw) = (left, h - bottom - bh, inner);
    out.draw_image_box(&normal, (bx, by, bw, bh));
    let label = b"Click to Continue";
    let (lx, ly) = button_label_origin(btn, label, (bx, by, bw, bh));
    btn.draw(&mut out, label, lx, ly, [255, 255, 100]);
    let (px, py) = (DIALOG_POS.0 as i32, DIALOG_POS.1 as i32);
    Some((out, (px + bx, py + by, bw, bh)))
}

/// The message line's widget: x, y and width; the height is the outline
/// font's height + 6 (`MessageWidget` ctor @ `0x00534490`).
const MESSAGE_RECT: (i32, i32, i32) = (90, 445, 460);

/// The message line as one image the size of its widget
/// (`MessageWidget::Draw` @ `0x00530830`, board.md#message-line): the
/// outline font in (0, 75, 0), then the plain font in (180, 250, 90), with
/// the baseline at ascent + (h − height) / 2, centred + 2.
fn render_message([outline, plain]: &[ImageFont; 2], text: &str, warning: bool) -> Rgba {
    let fh = outline.glyph_height();
    let (w, h) = (MESSAGE_RECT.2, fh + 6);
    let mut out = Rgba::new_transparent(w as u32, h as u32);
    let t = text.as_bytes();
    let x = (w - outline.width(t)) / 2 + 2;
    let y = outline.ascent() + (h - fh) / 2;
    // The alien warning is red (board.md#the-alien-warning-line).
    let (edge, fill) = if warning {
        ([100, 20, 20], [255, 50, 50])
    } else {
        ([0, 75, 0], [180, 250, 90])
    };
    outline.draw(&mut out, t, x, y, edge);
    plain.draw(&mut out, t, x, y, fill);
    out
}

/// `NewDialog` places board dialogs at (143, 142).
const DIALOG_POS: (f32, f32) = (143.0, 142.0);

/// `Board::DrawBackdrop` (`board.md#backdrop`): the tank image, the wave
/// strip, the three light bands and the foreground cut-out over them.
fn backdrop_cels(b: &Board, cels: &mut Vec<Cel>) {
    cels.push(Cel::new("IMAGE_AQUARIUM1", 0, 0, 0, 0, 0.0));
    // Wave strip at y = 88, additive: side, two centres, mirrored side.
    // The cel comes from IMAGE_WAVE's animation (rows 12, ping-pong,
    // framedelay 4, begindelay 6) at the level tick.
    static WAVE: std::sync::LazyLock<AnimInfo> = std::sync::LazyLock::new(|| {
        let anim = Anim {
            kind: AnimType::PingPong,
            frame_delay: 4,
            begin_delay: 6,
            ..Anim::default()
        };
        AnimInfo::compute(&anim, 12)
    });
    let row = WAVE.cel(b.level_ticks);
    for (sheet, x, mirror) in [
        ("IMAGE_WAVESIDE", 0, false),
        ("IMAGE_WAVE", 160, false),
        ("IMAGE_WAVE", 320, false),
        ("IMAGE_WAVESIDE", 480, true),
    ] {
        cels.push(
            Cel::new(sheet, 0, row, x, 88, 0.01)
                .mirror(mirror)
                .additive(),
        );
    }
    // Light bands at y = 365, additive at alpha 40, 100 and 70 (the third
    // mirrored), each drawn at x and x − 640 so it wraps around.
    for (i, (alpha, mirror)) in [(40, false), (100, false), (70, true)]
        .into_iter()
        .enumerate()
    {
        let x = b.light_x[i];
        for x in [x as i32, (f64::from(x) - 640.0) as i32] {
            cels.push(
                Cel::new("IMAGE_TANKLIGHTING", 0, 0, x, 365, 0.02)
                    .mirror(mirror)
                    .alpha(alpha)
                    .additive(),
            );
        }
    }
    cels.push(Cel::new("IMAGE_TANKMASK1", 0, 0, 0, 365, 0.03));
}

/// The tank: back to front as in `Board::RebuildZOrder` (board.md).
fn tank_cels(b: &Board, cursor: Option<(i32, i32)>, down: bool) -> Vec<Cel> {
    let mut cels = Vec::new();
    backdrop_cels(b, &mut cels);
    // Back to front as in Board::RebuildZOrder (board.md): shadows, dead
    // aliens, warps, food, guppies, aliens, pets, shots, coins.
    shadow_cels(b, &mut cels);
    // Dead fish (`fish-guppy.md#dead-fish`): IMAGE_SMALLDIE, row = stage
    // (stages above 2 shift down one), sinking (90 − life)/2 as it fades.
    for (i, d) in b.dead_fish.iter().enumerate() {
        let row = if d.kind > 2 { d.kind - 1 } else { d.kind };
        let sink = if d.life < 90 { (90 - d.life) / 2 } else { 0 };
        cels.push(
            Cel::new(
                "IMAGE_SMALLDIE",
                d.cel,
                row,
                d.widget_x,
                d.widget_y + sink,
                0.55 + i as f32 * 0.001,
            )
            .mirror(d.facing_right)
            .alpha((d.alpha * 255.0) as i32),
        );
    }
    for (i, d) in b.dead_aliens.iter().enumerate() {
        dead_alien_cels(d, 0.6 + i as f32 * 0.001, &mut cels);
    }
    for (i, w) in b.warps.iter().enumerate() {
        let (f, z) = (w.frame(), 0.8 + i as f32 * 0.001);
        cels.push(Cel::new(
            "IMAGE_WARPHOLE",
            f,
            0,
            w.widget_x + 20,
            w.widget_y,
            z,
        ));
        cels.push(Cel::new("IMAGE_WARPGLOW", f, 0, w.widget_x, w.widget_y, z + 0.0001).additive());
    }
    for f in &b.foods {
        cels.push(Cel::new(
            "IMAGE_FOOD",
            f.anim / f.anim_speed.max(1),
            f.level,
            f.widget_x - 5,
            f.widget_y - 4,
            1.0,
        ));
    }
    for (i, f) in b.guppies.iter().enumerate() {
        let (sheet, col, row, mirror) = guppy_cel(f);
        let z = 2.0 + i as f32 * 0.001;
        cels.push(Cel::new(sheet, col, row, f.widget_x, f.widget_y, z).mirror(mirror));
        // Hungry↔fed colour fade: the hungry sheet over the fed one at
        // fade·255/5 (fish-guppy.md#animation-and-drawing).
        if f.fade > 0 {
            let (sheet, col, row, mirror) = guppy_cel_as(f, true);
            cels.push(
                Cel::new(sheet, col, row, f.widget_x, f.widget_y, z + 0.0005)
                    .mirror(mirror)
                    .alpha(f.fade.clamp(0, 5) * 255 / 5),
            );
        }
    }
    for (i, a) in b.aliens.iter().enumerate() {
        alien_cels(a, 2.5 + i as f32 * 0.001, &mut cels);
    }
    for (i, p) in b.pets.iter().enumerate() {
        pet_cels(p, 2.7 + i as f32 * 0.001, &mut cels);
    }
    for (i, s) in b.shots.iter().enumerate() {
        shot_cels(s, b.laser, 2.8 + i as f32 * 0.001, &mut cels);
    }
    for (i, c) in b.coins.iter().enumerate() {
        // `IMAGE_MONEY` rows (`0x004dd7a0`, normal game): silver 0, gold 1,
        // star 2, diamond 3. Other kinds not drawn yet.
        let row = match c.kind {
            1 | 8 => 0,
            2 | 9 => 1,
            3 | 10 => 2,
            4 | 5 | 11 | 12 => 3,
            _ => continue,
        };
        cels.push(Cel::new(
            "IMAGE_MONEY",
            c.cel,
            row,
            c.widget_x,
            c.widget_y,
            3.0 + i as f32 * 0.001,
        ));
    }
    cels.push(Cel::new("IMAGE_MENUBAR", 0, 0, 0, 0, 10.0));
    shop_cels(b, cursor, down, &mut cels);
    cels
}

/// The hatch screen's Continue button rect (`hatch-screen.md#3`).
pub const HATCH_CONTINUE: (i32, i32, i32, i32) = (186, 445, 264, 29);

/// Name and description of a pet on the hatch screen (tank 1 for now).
/// TODO(spec): the other pets' text.
fn pet_text(pet: i32) -> (&'static str, &'static [&'static str]) {
    match pet {
        0 => (
            "STINKY the Snail",
            &[
                "STINKY roams around the",
                "bottom of your tank, catching",
                "any coins you may have missed.",
            ],
        ),
        1 => (
            "NIKO the Oyster",
            &[
                "NIKO produces pearls that",
                "you can click on for a",
                "hefty sum of money.",
            ],
        ),
        2 => (
            "ITCHY the Swordfish",
            &["ITCHY helps you by attacking", "aliens when they appear."],
        ),
        3 => (
            "PREGO the Momma Fish",
            &[
                "PREGO helps populate your",
                "tank by giving birth to a new",
                "baby guppy every so often.",
            ],
        ),
        4 => (
            "ZORF the Sea Horse",
            &["ZORF gives you a hand in", "keeping your fish fed."],
        ),
        _ => ("", &[]),
    }
}

/// The hatch screen's text layers (`hatch-screen.md#5`): three 640×480
/// layers [title bar + "You have found:", name, description + "saved"],
/// then the Continue button's two states cut to its rect ("Please Wait...",
/// "Click Here to Continue").
/// Text x is centred as `(650 − width) / 2 − 3`; y is the baseline.
fn render_hatch_text(
    img: &GameImages,
    [f17, f15, f12, f10]: &[ImageFont; 4],
    pet: i32,
) -> Option<[Rgba; 5]> {
    let centre = |f: &ImageFont, t: &str| ((650 - f.width(t.as_bytes())) >> 1) - 3;
    let layer = || Rgba::new_transparent(WIDTH, HEIGHT);
    let (name, lines) = pet_text(pet);
    let mut title = layer();
    if let Some(t) = img.get("IMAGE_SCREENTITLE") {
        title.draw_image_box(t, (20, 0, 600, t.height as i32));
    }
    f17.draw(&mut title, b"You have found:", 215, 25, [255, 200, 0]);
    // The Menu button (id 100) in its hole in the title bar
    // (`hatch-screen.md#3` step 5, `#5` step 3): (525, 4, 80, h).
    if let (Some(hole), Some(b)) = (
        img.get("IMAGE_SCREENTITLEHOLE"),
        img.get("IMAGE_MAINBUTTON"),
    ) {
        title.draw_image_box(hole, (524, 3, 82, hole.height as i32));
        let up = b.crop(0, 0, b.width / 3, b.height);
        let (mx, my, mw, mh) = (525, 4, 80, b.height as i32);
        title.draw_image_box(&up, (mx, my, mw, mh));
        // Label centred like the Continue button's (measured against the
        // original: see `button_label_origin`).
        let (lx, ly) = button_label_origin(f10, b"Menu", (mx, my, mw, mh));
        f10.draw(&mut title, b"Menu", lx, ly, [255, 240, 0]);
    }
    let mut n = layer();
    let nf = if matches!(pet, 9 | 11 | 14 | 21) {
        f12
    } else {
        f15
    };
    nf.draw(
        &mut n,
        name.as_bytes(),
        centre(nf, name),
        260,
        [255, 200, 0],
    );
    let mut desc = layer();
    for (i, l) in lines.iter().enumerate() {
        f10.draw(
            &mut desc,
            l.as_bytes(),
            centre(f10, l),
            300 + 20 * i as i32,
            [255; 3],
        );
    }
    f10.draw(
        &mut desc,
        b"Your game has been saved.",
        221,
        390,
        [255, 255, 100],
    );
    // Both button labels in one layer: "Please Wait..." on top, the
    // clickable label 30 px lower (the draw picks one by clipping).
    let mut button = layer();
    let (x, y, w, h) = HATCH_CONTINUE;
    if let Some(b) = img.get("IMAGE_MAINBUTTON") {
        let up = b.crop(0, 0, b.width / 3, b.height);
        for (k, label) in ["Please Wait...", "Click Here to Continue"]
            .iter()
            .enumerate()
        {
            let yy = y - 30 * (1 - k as i32);
            button.draw_image_box(&up, (x, yy, w, h));
            let (tx, ty) = button_label_origin(f12, label.as_bytes(), (x, yy, w, h));
            f12.draw(&mut button, label.as_bytes(), tx, ty, [255, 240, 0]);
        }
    }
    let (x, y, w, h) = (x as u32, y as u32, w as u32, h as u32);
    let wait = button.crop(x, y - 30, w, h);
    let cont = button.crop(x, y, w, h);
    Some([title, n, desc, wait, cont])
}

/// The label offset every game button gets from the button setup helper
/// (`0x005034e0`, `framework/dialog.md#buttons`).
const BUTTON_TEXT_OFFSET: (i32, i32) = (-2, 0);

/// Where a `DialogButton` draws its label (pen x, baseline y), from
/// `DialogButton::Draw` (`0x00476f70`, `framework/dialog.md#buttons`):
/// x = (w − width) / 2 + dx; baseline = (h − ascent/6 − padding − 1 +
/// ascent) / 2 + dy (C integer division).
fn button_label_origin(
    f: &ImageFont,
    label: &[u8],
    (x, y, w, h): (i32, i32, i32, i32),
) -> (i32, i32) {
    let a = f.ascent();
    let (dx, dy) = BUTTON_TEXT_OFFSET;
    (
        x + (w - f.width(label)) / 2 + dx,
        y + (h - a / 6 - f.ascent_padding() - 1 + a) / 2 + dy,
    )
}

/// Egg bob by `T % 32` (`hatch-screen.md#5` step 5).
fn egg_bob(t: i32) -> i32 {
    [0, -1, -2, -1, 0, 1, 2, 1][(t % 32 / 4) as usize]
}

/// Shard velocities (px per tick) for the 8 `IMAGE_EGGSHARDS` cels.
const SHARDS: [(i32, i32); 8] = [
    (1, -10),
    (-11, -15),
    (-14, 3),
    (12, -13),
    (-15, 10),
    (11, 5),
    (13, 14),
    (-1, 12),
];

/// The hatch screen's images (`hatch-screen.md#5`), `T` = updates so far.
fn hatch_cels(h: &Hatch) -> Vec<Cel> {
    let t = h.t;
    let (jx, jy) = h.jitter;
    let b = egg_bob(t);
    let mut cels = vec![Cel::new("IMAGE_HATCHSCREEN", 0, 0, 0, 0, 0.0)];
    if t <= 80 {
        let cel = if t < 56 { 0 } else { (t - 56) / 2 };
        cels.push(Cel::new("IMAGE_EGGCRACK1", cel, 0, 258, 60 + b, 1.0));
    } else if t < 140 {
        // (At T = 140 the original asks for cel 10 of 10: nothing drawn.)
        let cel = if t < 120 { 0 } else { (t - 120) / 2 };
        cels.push(Cel::new(
            "IMAGE_EGGCRACK2",
            cel,
            0,
            258 + jx,
            60 + jy + b,
            1.0,
        ));
    } else if t > 140 {
        // The pet swims in the incubator window: a clip of its sheet.
        if let Some(sheet) = pet_sheet(h.pet) {
            let y = match h.pet {
                0 | 5 => 100,
                1 => 70,
                _ => 90,
            };
            let f = match h.pet {
                5 | 10 | 11 | 14 | 20 => (t % 40) / 4,
                1 | 6 => {
                    let f = t % 20;
                    if f > 9 { 19 - f } else { f }
                }
                _ => (t % 20) / 2,
            };
            cels.push(Cel::new(sheet, f, 0, 278, y, 1.0));
        }
    }
    // The overlay, on top of everything: reflection, then flying shards.
    cels.push(Cel::new("IMAGE_HATCHREFLECTION", 0, 0, 240, 60, 20.0));
    if (140..200).contains(&t) {
        let s = t - 140;
        for (i, (vx, vy)) in SHARDS.iter().enumerate() {
            cels.push(Cel::new(
                "IMAGE_EGGSHARDS",
                i as i32,
                0,
                268 + vx * s,
                70 + vy * s,
                20.1,
            ));
        }
    }
    cels
}

/// A pet's full-size swim sheet (tank 1 pets for now).
fn pet_sheet(pet: i32) -> Option<&'static str> {
    Some(match pet {
        0 => "IMAGE_STINKY",
        1 => "IMAGE_NIKO",
        2 => "IMAGE_ITCHY",
        3 => "IMAGE_PREGO",
        4 => "IMAGE_ZORF",
        _ => return None,
    })
}
