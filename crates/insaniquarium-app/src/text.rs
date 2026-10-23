//! ImageFont text, rasterised to an [`Rgba`] with the same layout the asset
//! browser uses: pen advance, kerning pairs and per-glyph offsets.
//! Spec: `docs/re/assets/overview.md` (ImageFont); single-layer fonts only.

use sexy_core::data_dir::DataDir;
use sexy_core::font::{FontData, FontLayer};
use sexy_core::image::{Rgba, get_image};

#[derive(Debug, thiserror::Error)]
pub enum TextError {
    #[error("font file {0} missing")]
    Missing(String),
    #[error("font: {0}")]
    Font(String),
    #[error("font has no layers")]
    NoLayers,
    #[error("font image: {0}")]
    Image(String),
}

/// A loaded single-layer ImageFont.
pub struct ImageFont {
    layer: FontLayer,
    sheet: Rgba,
}

impl ImageFont {
    /// Load a font descriptor (e.g. `data/ContinuumBold12.txt`) and its sheet.
    pub fn load(data: &DataDir, path: &str) -> Result<Self, TextError> {
        let bytes = data
            .find(path)
            .and_then(|p| std::fs::read(p).ok())
            .ok_or_else(|| TextError::Missing(path.into()))?;
        let font = FontData::parse(&bytes).map_err(|e| TextError::Font(e.to_string()))?;
        let layer = font.layers.into_iter().next().ok_or(TextError::NoLayers)?;
        let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
        let img = layer.image.as_deref().unwrap_or("");
        let sheet = get_image(data, &format!("{dir}/{img}"))
            .map_err(|e| TextError::Image(e.to_string()))?;
        Ok(Self { layer, sheet })
    }

    /// `Font::GetAscent`.
    pub fn ascent(&self) -> i32 {
        self.layer.ascent
    }

    /// `Font::GetAscentPadding` (`LayerSetAscentPadding`).
    pub fn ascent_padding(&self) -> i32 {
        self.layer.ascent_padding
    }

    /// `Font::GetHeight`: the tallest glyph, rect height plus its y offset
    /// (computed in `LayerSetImageMap` @ `0x0045a71f`).
    pub fn glyph_height(&self) -> i32 {
        self.layer
            .glyphs
            .values()
            .map(|g| g.rect.h + g.offset.1)
            .max()
            .unwrap_or(0)
    }

    /// `Font::GetLineSpacing`: height + `LayerSetLineSpacingOffset`.
    pub fn line_spacing(&self) -> i32 {
        self.glyph_height() + self.layer.line_spacing_offset
    }

    /// Greedy word wrap to `width` pixels.
    pub fn wrap(&self, text: &str, width: i32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if self.width(candidate.as_bytes()) > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// Draw `text` onto `dst` with its baseline at `y` and its left edge at
    /// `x`, tinted `colour` (the glyph sheet is white).
    pub fn draw(&self, dst: &mut Rgba, text: &[u8], x: i32, y: i32, colour: [u8; 3]) {
        let top = y - self.layer.ascent;
        let mut pen = 0;
        let mut prev = None;
        let full = (0, 0, dst.width as i32, dst.height as i32);
        for &c in text {
            if let Some(p) = prev {
                pen += self.layer.kerning.get(&(p, c)).copied().unwrap_or(0);
            }
            if let Some(g) = self.layer.glyphs.get(&c) {
                let r = g.rect;
                dst.blit(
                    &self.sheet,
                    (r.x, r.y, r.w, r.h),
                    (x + pen + g.offset.0, top + g.offset.1),
                    colour,
                    full,
                );
                pen += g.width;
            }
            prev = Some(c);
        }
    }

    /// Line height used for the bitmap.
    pub fn height(&self) -> i32 {
        (self.layer.ascent + self.layer.line_spacing_offset).max(1)
    }

    /// Pen advance of `text` in pixels. `ImageFont::StringWidth` (slot 7 @
    /// `0x0045bf50`) sums `CharWidth(c, prev)` (slot 9 @ `0x0045bfd0`):
    /// the glyph width plus the kerning of the pair with the previous char.
    pub fn width(&self, text: &[u8]) -> i32 {
        let mut pen = 0;
        let mut prev = None;
        for &c in text {
            if let Some(p) = prev {
                pen += self.layer.kerning.get(&(p, c)).copied().unwrap_or(0);
            }
            if let Some(g) = self.layer.glyphs.get(&c) {
                pen += g.width;
            }
            prev = Some(c);
        }
        pen
    }

    /// Rasterise `text` onto a transparent image, top-left aligned.
    pub fn render(&self, text: &[u8]) -> Rgba {
        let pad = 4;
        let w = (self.width(text) + 2 * pad).max(1) as u32;
        let h = (self.height() + 2 * pad).max(1) as u32;
        let mut out = Rgba::new_transparent(w, h);
        let mut pen = 0;
        let mut prev = None;
        for &c in text {
            if let Some(p) = prev {
                pen += self.layer.kerning.get(&(p, c)).copied().unwrap_or(0);
            }
            if let Some(g) = self.layer.glyphs.get(&c) {
                let r = g.rect;
                for gy in 0..r.h {
                    for gx in 0..r.w {
                        let (sx, sy) = (r.x + gx, r.y + gy);
                        let (dx, dy) = (pad + pen + g.offset.0 + gx, pad + g.offset.1 + gy);
                        if sx < 0
                            || sy < 0
                            || dx < 0
                            || dy < 0
                            || sx as u32 >= self.sheet.width
                            || sy as u32 >= self.sheet.height
                            || dx as u32 >= w
                            || dy as u32 >= h
                        {
                            continue;
                        }
                        let src =
                            self.sheet.pixels[(sy as u32 * self.sheet.width + sx as u32) as usize];
                        let dst = &mut out.pixels[(dy as u32 * w + dx as u32) as usize];
                        if src[3] > dst[3] {
                            *dst = src;
                        }
                    }
                }
                pen += g.width;
            }
            prev = Some(c);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sexy_core::data_dir::test_data_dir;

    #[test]
    fn renders_money_text() {
        let Some(d) = test_data_dir() else { return };
        let f = ImageFont::load(&d, "data/ContinuumBold12.txt").unwrap();
        let img = f.render(b"$200");
        assert!(img.width > 20 && img.height > 8);
        assert!(img.pixels.iter().any(|p| p[3] > 0));
        assert_eq!(f.width(b""), 0);
        assert!(f.width(b"200") > f.width(b"2"));
    }
}
