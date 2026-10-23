//! CPU compositor for a [`Frame`]: draws it into a 640×480 image the way
//! the Bevy app shows it, so frames can be compared with screenshots of the
//! original without a GPU.
//!
//! Blending happens on sRGB values, as the original's software renderer
//! does (Bevy blends in linear space, so its tinted/translucent pixels can
//! differ slightly from these).

use sexy_core::image::Rgba;

use crate::scene::{Cel, Frame, HEIGHT, Sheets, Text, WIDTH};

/// One draw: cel or text, sorted together by z as the app's sprites are.
enum Item<'a> {
    Cel(&'a Cel),
    Text(&'a Text),
}

/// Composite `frame` onto an opaque black 640×480 image: cels and texts in
/// z order (stable: list order breaks ties, cels before texts).
pub fn render(frame: &Frame, sheets: &Sheets) -> Rgba {
    let mut out = Rgba {
        width: WIDTH,
        height: HEIGHT,
        pixels: vec![[0, 0, 0, 255]; (WIDTH * HEIGHT) as usize],
    };
    let mut items: Vec<(f32, Item)> = frame
        .cels
        .iter()
        .map(|c| (c.z, Item::Cel(c)))
        .chain(frame.texts.iter().map(|t| (t.z, Item::Text(t))))
        .collect();
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, item) in items {
        match item {
            Item::Cel(c) => draw_cel(&mut out, c, sheets),
            Item::Text(t) => draw_text(&mut out, t),
        }
    }
    out
}

/// `DrawImageCel` semantics: an unknown sheet or out-of-range cel draws
/// nothing. A cel with a `size` is stretched nearest-neighbour (the
/// original's `DrawImageMirror` with a dest rect may filter differently).
fn draw_cel(out: &mut Rgba, c: &Cel, sheets: &Sheets) {
    let Some(sheet) = sheets.get(c.sheet) else {
        return;
    };
    let Some((sx, sy, sw, sh)) = sheet.cel_rect(c.col, c.row) else {
        return;
    };
    let (dw, dh) = c.size.unwrap_or((sw as i32, sh as i32));
    if dw <= 0 || dh <= 0 {
        return;
    }
    let src = &sheet.image;
    // Fast path: unscaled, unmirrored, untinted, normal blending. Rows are
    // copied (opaque pixels) or blended per pixel.
    if (dw as u32, dh as u32) == (sw, sh)
        && !c.mirror
        && !c.additive
        && c.tint == [255; 3]
        && c.alpha == 255
    {
        blit_rows(out, src, (sx, sy, sw, sh), (c.x, c.y));
        return;
    }
    for j in 0..dh {
        let y = c.y + j;
        if y < 0 || y >= out.height as i32 {
            continue;
        }
        let v = sy + (j as u32 * sh) / dh as u32;
        for i in 0..dw {
            let x = c.x + i;
            if x < 0 || x >= out.width as i32 {
                continue;
            }
            let mut u = (i as u32 * sw) / dw as u32;
            if c.mirror {
                u = sw - 1 - u;
            }
            let (u, v) = (sx + u, v);
            if u >= src.width || v >= src.height {
                continue;
            }
            let s = src.pixels[(v * src.width + u) as usize];
            let d = &mut out.pixels[(y as u32 * out.width + x as u32) as usize];
            blend(d, s, c.tint, c.alpha, c.additive);
        }
    }
}

/// Copy `src`'s rect to `(x, y)` with normal alpha blending, clipped.
fn blit_rows(
    out: &mut Rgba,
    src: &Rgba,
    (sx, sy, sw, sh): (u32, u32, u32, u32),
    (x, y): (i32, i32),
) {
    let (ow, oh) = (out.width as i32, out.height as i32);
    let x0 = x.max(0);
    let x1 = (x + sw as i32).min(ow);
    if x0 >= x1 {
        return;
    }
    for j in 0..sh as i32 {
        let dy = y + j;
        let v = sy as i32 + j;
        if dy < 0 || dy >= oh || v as u32 >= src.height {
            continue;
        }
        let u0 = (sx as i32 + x0 - x) as u32;
        let n = (x1 - x0) as u32;
        let n = n.min(src.width.saturating_sub(u0));
        let s = &src.pixels[(v as u32 * src.width + u0) as usize..][..n as usize];
        let d = &mut out.pixels[(dy as u32 * out.width + x0 as u32) as usize..][..n as usize];
        for (d, &s) in d.iter_mut().zip(s) {
            match s[3] {
                0 => {}
                255 => *d = s,
                _ => blend(d, s, [255; 3], 255, false),
            }
        }
    }
}

fn draw_text(out: &mut Rgba, t: &Text) {
    let (x0, y0) = (t.x.round() as i32, t.y.round() as i32);
    let img = &t.image;
    for j in 0..img.height as i32 {
        let y = y0 + j;
        if y < 0 || y >= out.height as i32 {
            continue;
        }
        for i in 0..img.width as i32 {
            let x = x0 + i;
            if x < 0 || x >= out.width as i32 {
                continue;
            }
            let s = img.pixels[(j as u32 * img.width + i as u32) as usize];
            let d = &mut out.pixels[(y as u32 * out.width + x as u32) as usize];
            blend(d, s, t.colour, 255, false);
        }
    }
}

/// One source pixel onto an opaque destination: colour × `tint`, alpha ×
/// `alpha`; normal blending (`src·a + dst·(1−a)`) or additive (`dst +
/// src·a`, clamped). The destination stays opaque.
fn blend(d: &mut [u8; 4], s: [u8; 4], tint: [u8; 3], alpha: u8, additive: bool) {
    let a = u32::from(s[3]) * u32::from(alpha) / 255;
    if a == 0 {
        return;
    }
    for k in 0..3 {
        let c = u32::from(s[k]) * u32::from(tint[k]) / 255;
        let dk = u32::from(d[k]);
        d[k] = if additive {
            (dk + c * a / 255).min(255)
        } else {
            (c * a + dk * (255 - a)) / 255
        } as u8;
    }
    d[3] = 255;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;
    use crate::scene::Sheet;

    /// A 2×1-cel sheet: left cel red, right cel half-transparent green.
    fn sheets() -> Sheets {
        let image = Rgba {
            width: 4,
            height: 2,
            pixels: vec![
                [255, 0, 0, 255],
                [200, 0, 0, 255],
                [0, 255, 0, 128],
                [0, 255, 0, 128],
                [255, 0, 0, 255],
                [200, 0, 0, 255],
                [0, 255, 0, 128],
                [0, 255, 0, 128],
            ],
        };
        HashMap::from([(
            "S",
            Sheet {
                image: Arc::new(image),
                cel: (2, 2),
                grid: (2, 1),
            },
        )])
    }

    fn px(img: &Rgba, x: u32, y: u32) -> [u8; 4] {
        img.pixels[(y * img.width + x) as usize]
    }

    #[test]
    fn cels_mirror_stretch_and_blend() {
        let s = sheets();
        let frame = Frame {
            cels: vec![
                Cel::new("S", 0, 0, 0, 0, 0.0),
                Cel::new("S", 0, 0, 10, 0, 0.0).mirror(true),
                Cel::new("S", 1, 0, 20, 0, 0.0),
                Cel::new("S", 2, 0, 30, 0, 0.0),
                Cel::new("S", 0, 0, 40, 0, 0.0).additive().alpha(255),
                Cel::new("S", 0, 0, 40, 0, 1.0).additive(),
            ],
            texts: vec![],
        };
        let mut stretched = Cel::new("S", 0, 0, 50, 0, 0.0);
        stretched.size = Some((4, 4));
        let mut frame = frame;
        frame.cels.push(stretched);
        let img = render(&frame, &s);
        assert_eq!((img.width, img.height), (640, 480));
        assert_eq!(px(&img, 0, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 1, 0), [200, 0, 0, 255]);
        assert_eq!(px(&img, 10, 0), [200, 0, 0, 255]);
        assert_eq!(px(&img, 11, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 20, 0), [0, 128, 0, 255]);
        // Out-of-range column: nothing.
        assert_eq!(px(&img, 30, 0), [0, 0, 0, 255]);
        // Two additive passes of 255 clamp.
        assert_eq!(px(&img, 40, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 41, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 51, 3), [255, 0, 0, 255]);
        assert_eq!(px(&img, 52, 3), [200, 0, 0, 255]);
    }

    #[test]
    fn z_order_and_text_colour() {
        let s = sheets();
        let text = Rgba {
            width: 1,
            height: 1,
            pixels: vec![[255, 255, 255, 255]],
        };
        let frame = Frame {
            cels: vec![Cel::new("S", 0, 0, 0, 0, 2.0)],
            texts: vec![
                Text {
                    image: Arc::new(text.clone()),
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                    colour: [0, 0, 255],
                },
                Text {
                    image: Arc::new(text),
                    x: 5.0,
                    y: 0.0,
                    z: 1.0,
                    colour: [10, 20, 30],
                },
            ],
        };
        let img = render(&frame, &s);
        // The cel at z 2 covers the text at z 1.
        assert_eq!(px(&img, 0, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 5, 0), [10, 20, 30, 255]);
    }
}
