//! The main menu, the tank's Options dialog with Leave Game?, the Game
//! Paused dialog and the preview's end card, as whole-screen layers.
//! Specs: `docs/re/screens/game-selector.md`, `screens/options-dialog.md`,
//! `framework/dialog.md#game-paused-dialog-0x12`. Bevy-free.

use std::collections::HashMap;
use std::sync::Arc;

use sexy_core::data_dir::DataDir;
use sexy_core::image::Rgba;

use crate::assets::GameImages;
use crate::text::ImageFont;

const W: u32 = 640;
const H: u32 = 480;

/// A screen rect (x, y, w, h).
pub type Rect = (i32, i32, i32, i32);

fn inside((x, y, w, h): Rect, (px, py): (i32, i32)) -> bool {
    (x..x + w).contains(&px) && (y..y + h).contains(&py)
}

/// Main-menu buttons (`game-selector.md#widgets`), in drawing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuWidget {
    Adventure,
    TimeTrial,
    Challenge,
    VirtualTank,
    HallOfFame,
    Options,
    Help,
    Quit,
}

pub const MENU_WIDGETS: [(MenuWidget, Rect); 8] = [
    (MenuWidget::Adventure, (357, 48, 217, 66)),
    (MenuWidget::TimeTrial, (359, 142, 213, 48)),
    (MenuWidget::Challenge, (359, 212, 213, 48)),
    (MenuWidget::VirtualTank, (357, 287, 217, 66)),
    (MenuWidget::HallOfFame, (401, 380, 127, 29)),
    (MenuWidget::Options, (325, 412, 92, 29)),
    (MenuWidget::Help, (419, 412, 93, 29)),
    (MenuWidget::Quit, (514, 412, 89, 29)),
];

/// The v0.1.0 preview acts only on these (`game-selector.md#deviations`).
pub fn menu_widget_active(w: MenuWidget) -> bool {
    matches!(w, MenuWidget::Adventure | MenuWidget::Quit)
}

pub fn menu_widget_at(p: (i32, i32)) -> Option<MenuWidget> {
    MENU_WIDGETS
        .iter()
        .find(|(_, r)| inside(*r, p))
        .map(|(w, _)| *w)
}

/// The tank's Menu button (`options-dialog.md#the-tanks-menu-button`).
pub const TANK_MENU_BUTTON: Rect = (525, 3, 101, 29);

/// Button state for drawing: normal, hovered, pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Btn {
    #[default]
    Up,
    Over,
    Down,
}

impl Btn {
    pub fn of(r: Rect, cursor: Option<(i32, i32)>, down: bool) -> Btn {
        match cursor {
            Some(p) if inside(r, p) && down => Btn::Down,
            Some(p) if inside(r, p) => Btn::Over,
            _ => Btn::Up,
        }
    }
}

/// What a menus layer shows; also its cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The main menu with one button hovered or pressed.
    Menu { hot: Option<(MenuWidget, Btn)> },
    /// "Stop the insanity?" over the menu; the hovered button (0 QUIT, 1
    /// CANCEL).
    QuitConfirm { hot: Option<(usize, Btn)> },
    /// The tank's Menu button hovered/pressed (over the menu bar).
    TankMenuButton(Btn),
    /// The Options dialog; hot = button index (0 Help, 1 Back to Main Menu,
    /// 2 OK).
    Options { hot: Option<(usize, Btn)> },
    /// Leave Game?; hot = 0 LEAVE, 1 CANCEL.
    Leave { hot: Option<(usize, Btn)> },
    /// Game Paused; Resume Game hovered or not.
    Paused(Btn),
    /// The preview's end card.
    PreviewEnd,
}

/// Fonts and images for the menus, and rendered layers by state.
pub struct MenuArt {
    /// 15 / 10 / 12 outline: dialog header, body, button; big menu labels
    /// use the 15.
    f15: Option<ImageFont>,
    f10: Option<ImageFont>,
    f12: Option<ImageFont>,
    hint: Option<ImageFont>,
    cache: HashMap<Layer, (Arc<Rgba>, (i32, i32))>,
}

/// Dialog geometry the app hit-tests against (`framework/dialog.md`).
pub const DIALOG_X: i32 = 143;
pub const DIALOG_Y: i32 = 142;
const OPTIONS_POS: (i32, i32) = (143, 26);
const OPTIONS_W: i32 = 348;
/// Options' height (slot 73 `0x00530e60`): 0x148 + 0x22 for the Help row.
const OPTIONS_H: i32 = 0x148 + 0x22;
const INSETS: (i32, i32, i32, i32) = (36, 15, 36, 36);
const BTN_H: i32 = 33;

impl MenuArt {
    pub fn load(data: &DataDir) -> Self {
        let f = |p: &str| ImageFont::load(data, p).ok();
        MenuArt {
            f15: f("data/JungleFever15outline.txt"),
            f10: f("data/JungleFever10outline.txt"),
            f12: f("data/JungleFever12outline.txt"),
            hint: f("data/BlambotPro8.txt"),
            cache: HashMap::new(),
        }
    }

    /// The layer image and where it goes.
    pub fn layer(&mut self, img: &GameImages, l: &Layer) -> Option<(Arc<Rgba>, (i32, i32))> {
        if let Some(v) = self.cache.get(l) {
            return Some(v.clone());
        }
        let v = match l {
            Layer::Menu { hot } => (self.render_menu(img, *hot)?, (0, 0)),
            Layer::QuitConfirm { hot } => {
                let (im, _) = self.two_button_dialog(
                    img,
                    "Quit",
                    "Stop the insanity?",
                    ["QUIT", "CANCEL"],
                    *hot,
                )?;
                (im, (DIALOG_X, DIALOG_Y))
            }
            Layer::TankMenuButton(b) => {
                let id = match b {
                    Btn::Up => return None,
                    Btn::Over => "IMAGE_OPTIONSBUTTON",
                    Btn::Down => "IMAGE_OPTIONSBUTTOND",
                };
                let im = img.get(id)?.clone();
                (im, (TANK_MENU_BUTTON.0, TANK_MENU_BUTTON.1))
            }
            Layer::Options { hot } => (self.render_options(img, *hot)?, OPTIONS_POS),
            Layer::Leave { hot } => {
                let (im, _) = self.two_button_dialog(
                    img,
                    "Leave Game?",
                    "Do you want to return to the\nmain menu?\n\nYour game will be saved.",
                    ["LEAVE", "CANCEL"],
                    *hot,
                )?;
                (im, (DIALOG_X, DIALOG_Y))
            }
            Layer::Paused(b) => {
                let hot = (*b != Btn::Up).then_some((0, *b));
                (
                    self.one_button_dialog(
                        img,
                        "GAME PAUSED",
                        "Click to resume game",
                        "Resume Game",
                        hot,
                    )?,
                    (DIALOG_X, DIALOG_Y),
                )
            }
            Layer::PreviewEnd => (self.render_end(img)?, (0, 0)),
        };
        let v = (Arc::new(v.0), v.1);
        self.cache.insert(l.clone(), v.clone());
        Some(v)
    }

    /// `GameSelector::Draw`: the backdrop holds the mermaid, the panel and
    /// the purple pills; a hovered big button draws its over image, a
    /// pressed one its down image; small buttons are DialogButtons.
    fn render_menu(&self, img: &GameImages, hot: Option<(MenuWidget, Btn)>) -> Option<Rgba> {
        let (f15, f10, hint) = (self.f15.as_ref()?, self.f10.as_ref()?, self.hint.as_ref()?);
        let mut out = img.get("IMAGE_SELECTORSCREEN")?.clone();
        for (w, r) in MENU_WIDGETS {
            let state = hot.filter(|h| h.0 == w).map_or(Btn::Up, |h| h.1);
            let big = matches!(w, MenuWidget::Adventure | MenuWidget::VirtualTank);
            let label = match w {
                MenuWidget::Adventure => "Start Adventure",
                MenuWidget::TimeTrial => "Time Trial",
                MenuWidget::Challenge => "Challenge",
                MenuWidget::VirtualTank => "Virtual Tank",
                MenuWidget::HallOfFame => "Hall of Fame",
                MenuWidget::Options => "Options",
                MenuWidget::Help => "Help",
                MenuWidget::Quit => "Quit",
            };
            if matches!(
                w,
                MenuWidget::Adventure
                    | MenuWidget::TimeTrial
                    | MenuWidget::Challenge
                    | MenuWidget::VirtualTank
            ) {
                let art = match (state, big) {
                    (Btn::Over, true) => Some("IMAGE_BATTLETANKBUTTON"),
                    (Btn::Down, true) => Some("IMAGE_BATTLETANKBUTTOND"),
                    (Btn::Over, false) => Some("IMAGE_MIDDLEBUTTON"),
                    (Btn::Down, false) => Some("IMAGE_MIDDLEBUTTOND"),
                    _ => None,
                };
                if let Some(a) = art.and_then(|a| img.get(a)) {
                    out.blit(
                        a,
                        (0, 0, a.width as i32, a.height as i32),
                        (r.0, r.1),
                        [255; 3],
                        (0, 0, W as i32, H as i32),
                    );
                }
                // Lavender when hovered/pressed, yellow otherwise (ctor
                // colours (255, 240, 0) / (200, 200, 255)); label centred.
                let colour = if state == Btn::Up {
                    [255, 240, 0]
                } else {
                    [200, 200, 255]
                };
                let (lx, ly) = label_origin(f15, label.as_bytes(), r);
                f15.draw(&mut out, label.as_bytes(), lx, ly, colour);
            } else {
                let id = match w {
                    MenuWidget::HallOfFame => "IMAGE_MAINBUTTON",
                    MenuWidget::Options => "IMAGE_LEFTBUTTON",
                    MenuWidget::Help => "IMAGE_CENTERBUTTON",
                    _ => "IMAGE_RIGHTBUTTON",
                };
                let b = img.get(id)?;
                let cel = match state {
                    Btn::Up => 0,
                    Btn::Over => 1,
                    Btn::Down => 2,
                };
                let third = b.crop(cel * b.width / 3, 0, b.width / 3, b.height);
                out.draw_image_box(&third, r);
                let colour = if state == Btn::Up {
                    [255, 240, 0]
                } else {
                    [255, 255, 255]
                };
                let (lx, ly) = label_origin(f10, label.as_bytes(), r);
                f10.draw(&mut out, label.as_bytes(), lx, ly, colour);
            }
        }
        // The speech bubble with the hovered mode's hint
        // (`game-selector.md#drawing`); no profile, so no welcome.
        let text = hot.and_then(|(w, _)| match w {
            MenuWidget::Adventure => {
                Some("New to Insaniquarium? Click here to start your aquatic adventure!")
            }
            MenuWidget::TimeTrial => Some("How much money can you earn before time runs out?"),
            MenuWidget::Challenge => Some("Can you fend off the increasingly difficult aliens?"),
            MenuWidget::VirtualTank => {
                Some("Buy and raise your own custom fish, then use them as a screensaver!")
            }
            _ => None,
        });
        if let (Some(t), Some(bubble)) = (text, img.get("IMAGE_SELECTORSPEECHBUBBLE")) {
            out.blit(
                bubble,
                (0, 0, bubble.width as i32, bubble.height as i32),
                (45, 20),
                [255; 3],
                (0, 0, W as i32, H as i32),
            );
            let (rx, ry, rw, rh) = (64, 40, 214, 60);
            let lines = hint.wrap(t, rw);
            let lh = hint.line_spacing();
            let mut y = ry + (rh - lines.len() as i32 * lh) / 2 + hint.ascent();
            for l in &lines {
                let x = rx + (rw - hint.width(l.as_bytes())) / 2;
                hint.draw(&mut out, l.as_bytes(), x, y, [0, 0, 100]);
                y += lh;
            }
        }
        Some(out)
    }

    /// The standard dialog panel with header and body
    /// (`framework/dialog.md`), height for `buttons` button rows.
    fn panel(&self, img: &GameImages, w: i32, h: i32, header: &str, body: &str) -> Option<Rgba> {
        let (head, f10) = (self.f15.as_ref()?, self.f10.as_ref()?);
        let panel = img.get("IMAGE_DIALOG")?;
        let (left, top, right, _) = INSETS;
        let inner = w - left - right;
        let mut out = Rgba::new_transparent(w as u32, h as u32);
        out.draw_image_box(panel, (0, 0, w, h));
        let hy = top - head.ascent_padding() + head.ascent();
        head.draw(
            &mut out,
            header.as_bytes(),
            left + (inner - head.width(header.as_bytes())) / 2,
            hy,
            [255, 200, 0],
        );
        let mut y =
            hy - head.ascent() + head.glyph_height() + 15 + f10.ascent() - f10.ascent_padding();
        for para in body.split('\n') {
            for l in f10.wrap(para, inner - 4) {
                let x = left + 2 + (inner - 4 - f10.width(l.as_bytes())) / 2;
                f10.draw(&mut out, l.as_bytes(), x, y, [255; 3]);
                y += f10.line_spacing();
            }
            if para.is_empty() {
                y += f10.line_spacing();
            }
        }
        Some(out)
    }

    fn body_lines(&self, body: &str, inner: i32) -> i32 {
        let Some(f10) = &self.f10 else { return 0 };
        body.split('\n')
            .map(|p| {
                if p.is_empty() {
                    1
                } else {
                    f10.wrap(p, inner - 4).len() as i32
                }
            })
            .sum()
    }

    fn dialog_height(&self, img: &GameImages, body: &str, extra: i32) -> Option<i32> {
        let (head, f10) = (self.f15.as_ref()?, self.f10.as_ref()?);
        let panel = img.get("IMAGE_DIALOG")?;
        let (left, top, right, bottom) = INSETS;
        let inner = 348 - left - right;
        let pref = top
            + head.line_spacing()
            + 10
            + self.body_lines(body, inner) * f10.line_spacing()
            + 8
            + BTN_H
            + bottom
            + extra;
        Some(pref.max(panel.height as i32 * 2 / 3 + 10))
    }

    fn button(
        &self,
        img: &GameImages,
        out: &mut Rgba,
        r: Rect,
        label: &str,
        state: Btn,
    ) -> Option<()> {
        let f12 = self.f12.as_ref()?;
        let b = img.get("IMAGE_DIALOGBUTTON")?;
        let cel = match state {
            Btn::Up => 0,
            Btn::Over => 1,
            Btn::Down => 2,
        };
        out.draw_image_box(&b.crop(cel * b.width / 3, 0, b.width / 3, b.height), r);
        let (lx, ly) = label_origin(f12, label.as_bytes(), r);
        f12.draw(out, label.as_bytes(), lx, ly, [255, 255, 100]);
        Some(())
    }

    fn one_button_dialog(
        &self,
        img: &GameImages,
        header: &str,
        body: &str,
        label: &str,
        hot: Option<(usize, Btn)>,
    ) -> Option<Rgba> {
        let h = self.dialog_height(img, body, 0)?;
        let mut out = self.panel(img, 348, h, header, body)?;
        let (left, _, right, bottom) = INSETS;
        let r = (left, h - bottom - BTN_H, 348 - left - right, BTN_H);
        self.button(img, &mut out, r, label, hot.map_or(Btn::Up, |h| h.1))?;
        Some(out)
    }

    /// Two side-by-side buttons, 8 px apart, 13 px taller than one
    /// (`framework/dialog.md`, two buttons). Returns the buttons' rects
    /// relative to the dialog.
    fn two_button_dialog(
        &self,
        img: &GameImages,
        header: &str,
        body: &str,
        labels: [&str; 2],
        hot: Option<(usize, Btn)>,
    ) -> Option<(Rgba, [Rect; 2])> {
        let h = self.dialog_height(img, body, 13)?;
        let mut out = self.panel(img, 348, h, header, body)?;
        let rects = two_button_rects(h);
        for (k, r) in rects.iter().enumerate() {
            let s = hot.filter(|h| h.0 == k).map_or(Btn::Up, |h| h.1);
            self.button(img, &mut out, *r, labels[k], s)?;
        }
        Some((out, rects))
    }

    /// Screen rects of a two-button dialog's buttons (QUIT/CANCEL,
    /// LEAVE/CANCEL).
    pub fn two_button_screen_rects(&self, img: &GameImages, body: &str) -> Option<[Rect; 2]> {
        let h = self.dialog_height(img, body, 13)?;
        Some(two_button_rects(h).map(|(x, y, w, hh)| (x + DIALOG_X, y + DIALOG_Y, w, hh)))
    }

    /// Screen rect of a one-button dialog's button (Game Paused).
    pub fn one_button_screen_rect(&self, img: &GameImages, body: &str) -> Option<Rect> {
        let h = self.dialog_height(img, body, 0)?;
        let (left, _, right, bottom) = INSETS;
        Some((
            DIALOG_X + left,
            DIALOG_Y + h - bottom - BTN_H,
            348 - left - right,
            BTN_H,
        ))
    }

    /// The Options dialog (`options-dialog.md#dialog`): sliders and
    /// checkboxes drawn (inactive in v0.1.0), then Help, Back to Main Menu
    /// and OK stacked upward from the bottom.
    fn render_options(&self, img: &GameImages, hot: Option<(usize, Btn)>) -> Option<Rgba> {
        let f10 = self.f10.as_ref()?;
        let (w, h) = (OPTIONS_W, OPTIONS_H);
        let mut out = self.panel(img, w, h, "Options", "")?;
        let line = [255, 255, 255];
        f10.draw(&mut out, b"Music", 0x30, 0x4e, line);
        f10.draw(&mut out, b"Sound Fx", 0x30, 0x6e, line);
        if let (Some(track), Some(thumb)) =
            (img.get("IMAGE_SLIDERTRACK"), img.get("IMAGE_SLIDERTHUMB"))
        {
            for y in [0x3a, 0x5a] {
                let sw = w - 0xc0;
                let ty = y + (33 - track.height as i32) / 2;
                out.draw_image_box(track, (0x98, ty, sw, track.height as i32));
                // Thumbs at full volume (the original's defaults here).
                let tx = 0x98 + sw - thumb.width as i32;
                out.blit(
                    thumb,
                    (0, 0, thumb.width as i32, thumb.height as i32),
                    (tx, y + (33 - thumb.height as i32) / 2),
                    [255; 3],
                    (0, 0, w, h),
                );
            }
        }
        for (x, y, label, on) in [
            (0x21, 0x80, "Fullscreen", false),
            (0xa1, 0x80, "Custom Cursors", false),
            (0x21, 0xac, "Hardware Acceleration", true),
        ] {
            if let Some(c) = img.get(if on {
                "IMAGE_CHECKED"
            } else {
                "IMAGE_UNCHECKED"
            }) {
                out.blit(
                    c,
                    (0, 0, c.width as i32, c.height as i32),
                    (x, y),
                    [255; 3],
                    (0, 0, w, h),
                );
            }
            f10.draw(&mut out, label.as_bytes(), x + 0x2b, y + 0x18, line);
        }
        for (k, (r, label)) in options_buttons().iter().enumerate() {
            let s = hot.filter(|h| h.0 == k).map_or(Btn::Up, |h| h.1);
            self.button(img, &mut out, *r, label, s)?;
        }
        Some(out)
    }

    fn render_end(&self, img: &GameImages) -> Option<Rgba> {
        let f15 = self.f15.as_ref()?;
        let f10 = self.f10.as_ref()?;
        let mut out = img.get("IMAGE_SELECTORSCREEN")?.clone();
        let (pw, ph) = (420, 200);
        let mut p = self.panel(img, pw, ph, "End of the Preview", "")?;
        let lines = [
            "Thanks for playing the OpenInsaniquarium v0.1.0 preview!",
            "Tank 1 is all this build plays.",
            "Tanks 2-5 and the other modes are coming.",
            "",
            "Click to return to the main menu.",
        ];
        let mut y = 70;
        for l in lines {
            let x = (pw - f10.width(l.as_bytes())) / 2;
            f10.draw(&mut p, l.as_bytes(), x, y, [255; 3]);
            y += 22;
        }
        let _ = f15;
        out.blit(
            &p,
            (0, 0, pw, ph),
            ((W as i32 - pw) / 2, 140),
            [255; 3],
            (0, 0, W as i32, H as i32),
        );
        Some(out)
    }
}

fn two_button_rects(h: i32) -> [Rect; 2] {
    let (left, _, right, bottom) = INSETS;
    let inner = 348 - left - right;
    let bw = (inner - 8) / 2;
    let by = h - bottom - BTN_H;
    [(left, by, bw, BTN_H), (left + bw + 8, by, bw, BTN_H)]
}

/// Options' buttons relative to the dialog: Help, Back to Main Menu, OK,
/// 34 px apart upward from the footer (`OptionsDialog::Resize`).
fn options_buttons() -> [(Rect, &'static str); 3] {
    let (left, _, right, bottom) = INSETS;
    let inner = OPTIONS_W - left - right;
    let ok_y = OPTIONS_H - bottom - BTN_H;
    [
        ((left, ok_y - 0x44, inner, BTN_H), "Help"),
        ((left, ok_y - 0x22, inner, BTN_H), "Back to Main Menu"),
        ((left, ok_y, inner, BTN_H), "OK"),
    ]
}

/// Options' buttons in screen pixels: Help, Back to Main Menu, OK.
pub fn options_screen_rects() -> [Rect; 3] {
    options_buttons().map(|((x, y, w, h), _)| (x + OPTIONS_POS.0, y + OPTIONS_POS.1, w, h))
}

pub const LEAVE_BODY: &str = "Do you want to return to the\nmain menu?\n\nYour game will be saved.";
pub const QUIT_BODY: &str = "Stop the insanity?";
pub const PAUSED_BODY: &str = "Click to resume game";

/// A DialogButton's label origin (`framework/dialog.md#buttons`), text
/// offset (−2, 0).
fn label_origin(f: &ImageFont, label: &[u8], (x, y, w, h): Rect) -> (i32, i32) {
    let a = f.ascent();
    (
        x + (w - f.width(label)) / 2 - 2,
        y + (h - a / 6 - f.ascent_padding() - 1 + a) / 2,
    )
}
