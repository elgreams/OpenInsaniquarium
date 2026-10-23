//! Frame comparison: our CPU rendering of a tick vs a screenshot of the
//! original at the same tick (`docs/re/framework/frame-capture.md`).
//!
//! Pixel-exact equality isn't expected (blend rounding, filtered stretches),
//! so the diff works on a grid of regions: a region "differs" when enough of
//! its pixels are off by more than a tolerance. The report names the worst
//! regions, and a side-by-side sheet (ours | original | difference) is
//! written for looking at.

use sexy_core::image::Rgba;

/// Per-channel difference that counts as a mismatched pixel.
pub const PIXEL_TOLERANCE: i32 = 40;
/// Region size (pixels).
pub const CELL: u32 = 32;
/// A region differs when more than this fraction of its pixels mismatch.
pub const REGION_THRESHOLD: f32 = 0.05;

#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    /// Fraction of mismatched pixels.
    pub bad: f32,
}

#[derive(Debug, Clone)]
pub struct Report {
    /// Fraction of all pixels that mismatch.
    pub bad: f32,
    /// Regions over the threshold, worst first.
    pub regions: Vec<Region>,
    /// Per-pixel mismatch mask (true = differs).
    pub mask: Vec<bool>,
}

impl Report {
    pub fn matches(&self) -> bool {
        self.regions.is_empty()
    }

    /// One line: overall mismatch and the worst regions.
    pub fn summary(&self) -> String {
        let worst: Vec<String> = self
            .regions
            .iter()
            .take(6)
            .map(|r| format!("({},{}) {:.0}%", r.x, r.y, r.bad * 100.0))
            .collect();
        format!(
            "{:.2}% pixels differ, {} regions over {:.0}%: {}",
            self.bad * 100.0,
            self.regions.len(),
            REGION_THRESHOLD * 100.0,
            worst.join(" ")
        )
    }
}

fn px(img: &Rgba, x: u32, y: u32) -> [u8; 4] {
    img.pixels[(y * img.width + x) as usize]
}

/// Compare two 640×480 images (alpha ignored), skipping pixels where
/// `ignore` is true (e.g. the cursor).
pub fn compare(ours: &Rgba, original: &Rgba, ignore: impl Fn(u32, u32) -> bool) -> Report {
    let (w, h) = (
        ours.width.min(original.width),
        ours.height.min(original.height),
    );
    let mut mask = vec![false; (w * h) as usize];
    let mut total_bad = 0u32;
    let mut counted = 0u32;
    for y in 0..h {
        for x in 0..w {
            if ignore(x, y) {
                continue;
            }
            counted += 1;
            let (a, b) = (px(ours, x, y), px(original, x, y));
            let off = (0..3).any(|k| (i32::from(a[k]) - i32::from(b[k])).abs() > PIXEL_TOLERANCE);
            if off {
                mask[(y * w + x) as usize] = true;
                total_bad += 1;
            }
        }
    }
    let mut regions = Vec::new();
    for ry in (0..h).step_by(CELL as usize) {
        for rx in (0..w).step_by(CELL as usize) {
            let (mut n, mut bad) = (0u32, 0u32);
            for y in ry..(ry + CELL).min(h) {
                for x in rx..(rx + CELL).min(w) {
                    if ignore(x, y) {
                        continue;
                    }
                    n += 1;
                    bad += u32::from(mask[(y * w + x) as usize]);
                }
            }
            let f = if n == 0 { 0.0 } else { bad as f32 / n as f32 };
            if f > REGION_THRESHOLD {
                regions.push(Region {
                    x: rx,
                    y: ry,
                    bad: f,
                });
            }
        }
    }
    regions.sort_by(|a, b| b.bad.total_cmp(&a.bad));
    Report {
        bad: if counted == 0 {
            0.0
        } else {
            total_bad as f32 / counted as f32
        },
        regions,
        mask,
    }
}

/// Ours | original | difference (mismatches in red over a dimmed original,
/// differing regions outlined in yellow), 1920×480.
pub fn sheet(ours: &Rgba, original: &Rgba, report: &Report) -> Rgba {
    let (w, h) = (640u32, 480u32);
    let mut out = Rgba::new_transparent(w * 3, h);
    for y in 0..h.min(ours.height).min(original.height) {
        for x in 0..w.min(ours.width).min(original.width) {
            let a = px(ours, x, y);
            let b = px(original, x, y);
            let set = |out: &mut Rgba, ox: u32, c: [u8; 4]| {
                out.pixels[(y * w * 3 + ox + x) as usize] = [c[0], c[1], c[2], 255];
            };
            set(&mut out, 0, a);
            set(&mut out, w, b);
            let d = if report.mask[(y * w + x) as usize] {
                [255, 0, 0, 255]
            } else {
                [b[0] / 3, b[1] / 3, b[2] / 3, 255]
            };
            set(&mut out, 2 * w, d);
        }
    }
    for r in &report.regions {
        for i in 0..CELL {
            for (x, y) in [
                (r.x + i, r.y),
                (r.x + i, r.y + CELL - 1),
                (r.x, r.y + i),
                (r.x + CELL - 1, r.y + i),
            ] {
                if x < w && y < h {
                    out.pixels[(y * w * 3 + 2 * w + x) as usize] = [255, 255, 0, 255];
                }
            }
        }
    }
    out
}

/// Decode a PNG (a screenshot) into an [`Rgba`].
pub fn load_png(path: &std::path::Path) -> Option<Rgba> {
    let img = image::open(path).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(Rgba {
        width: w,
        height: h,
        pixels: img.pixels().map(|p| p.0).collect(),
    })
}

/// Encode an [`Rgba`] as PNG.
pub fn save_png(img: &Rgba, path: &std::path::Path) -> Result<(), String> {
    image::save_buffer(
        path,
        img.as_bytes(),
        img.width,
        img.height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(c: [u8; 4]) -> Rgba {
        Rgba {
            width: 640,
            height: 480,
            pixels: vec![c; 640 * 480],
        }
    }

    #[test]
    fn identical_frames_match_and_a_patch_is_found() {
        let a = solid([10, 20, 30, 255]);
        let r = compare(&a, &a, |_, _| false);
        assert!(r.matches() && r.bad == 0.0);
        let mut b = a.clone();
        for y in 100..140 {
            for x in 200..240 {
                b.pixels[y * 640 + x] = [200, 0, 0, 255];
            }
        }
        let r = compare(&a, &b, |_, _| false);
        assert!(!r.matches());
        assert!(r.regions.iter().any(|g| g.x == 192 && g.y == 96));
        // Ignoring the patch makes them match again.
        let r = compare(&a, &b, |x, y| {
            (200..240).contains(&x) && (100..140).contains(&y)
        });
        assert!(r.matches());
    }
}
