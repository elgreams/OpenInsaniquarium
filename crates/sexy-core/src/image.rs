//! Image loading with the original's alpha-mask rules.
//! Spec: `docs/re/assets/images.md`.

use std::path::Path;

use crate::data_dir::DataDir;
use crate::resources::ImageRes;

/// Extension probe order of `ImageLib::GetImage` (tga/j2k/jp2 omitted: unused
/// by the game data and unsupported here).
pub const IMAGE_EXTS: &[&str] = &["jpg", "png", "gif"];

/// Default RGB for mask-only images (`alphacolor`).
const DEFAULT_ALPHA_COLOR: u32 = 0xFF_FFFF;

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("failed to load image: {0}")]
    NotFound(String),
    #[error("{path}: {source}")]
    Decode {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("alpha image size mismatch between {image} and {mask}")]
    MaskSize { image: String, mask: String },
}

/// 32-bit RGBA image, row-major, `[r, g, b, a]` per pixel.
#[derive(Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

impl std::fmt::Debug for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rgba({}x{})", self.width, self.height)
    }
}

impl Rgba {
    fn same_size(&self, o: &Rgba) -> bool {
        self.width == o.width && self.height == o.height
    }

    /// The original copies the mask's low byte (blue) into alpha.
    fn apply_mask(&mut self, mask: &Rgba) {
        for (p, m) in self.pixels.iter_mut().zip(&mask.pixels) {
            p[3] = m[2];
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.pixels.as_flattened()
    }

    /// A transparent black image (fresh MemoryImage contents are 0x00000000).
    pub fn new_transparent(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0; 4]; (width * height) as usize],
        }
    }

    fn px(&self, x: u32, y: u32) -> [u8; 4] {
        self.pixels[(y * self.width + x) as usize]
    }

    /// Copy the `w × h` region at `(x, y)`, clipped to the image.
    pub fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> Rgba {
        let w = w.min(self.width.saturating_sub(x));
        let h = h.min(self.height.saturating_sub(y));
        let mut out = Rgba::new_transparent(w, h);
        for j in 0..h {
            for i in 0..w {
                out.pixels[(j * w + i) as usize] = self.px(x + i, y + j);
            }
        }
        out
    }

    /// Repeat the image along x and y until it fills `w × h` (copies start at
    /// multiples of the source size, the last one clipped).
    pub fn tile(&self, w: u32, h: u32) -> Rgba {
        let mut out = Rgba::new_transparent(w, h);
        for j in 0..h {
            for i in 0..w {
                out.pixels[(j * w + i) as usize] = self.px(i % self.width, j % self.height);
            }
        }
        out
    }

    /// Replace alpha with `mask`'s alpha, pixel by pixel in linear order
    /// (`WinFish::CreateMaskedCutout`; see docs/re/assets/sprite-variants.md).
    pub fn with_alpha_of(mut self, mask: &Rgba) -> Rgba {
        for (p, m) in self.pixels.iter_mut().zip(&mask.pixels) {
            p[3] = m[3];
        }
        self
    }

    /// Area-average (box) downscale of `src` into `dst` at `(dx, dy, dw, dh)`,
    /// then composited onto `dst` the way the original's slow stretch does
    /// onto an empty canvas (`c·a >> 8`, alpha kept). Approximates
    /// `MemoryImage::SlowStretchBlt` (exact fixed-point weights are TODO).
    pub fn box_downscale_into(&self, dst: &mut Rgba, dx: u32, dy: u32, dw: u32, dh: u32) {
        let (sw, sh) = (self.width as f64, self.height as f64);
        let (fx, fy) = (sw / dw as f64, sh / dh as f64);
        for j in 0..dh {
            let (y0, y1) = (j as f64 * fy, (j + 1) as f64 * fy);
            for i in 0..dw {
                let (x0, x1) = (i as f64 * fx, (i + 1) as f64 * fx);
                let mut acc = [0f64; 4];
                let mut area = 0.0;
                let mut sy = y0.floor() as u32;
                while (sy as f64) < y1 && sy < self.height {
                    let cy = (y1.min(sy as f64 + 1.0) - y0.max(sy as f64)).max(0.0);
                    let mut sx = x0.floor() as u32;
                    while (sx as f64) < x1 && sx < self.width {
                        let cx = (x1.min(sx as f64 + 1.0) - x0.max(sx as f64)).max(0.0);
                        let w = cx * cy;
                        let p = self.px(sx, sy);
                        for c in 0..4 {
                            acc[c] += p[c] as f64 * w;
                        }
                        area += w;
                        sx += 1;
                    }
                    sy += 1;
                }
                if area == 0.0 {
                    continue;
                }
                let avg = acc.map(|v| (v / area) as u32);
                let a = avg[3];
                if a == 0 {
                    continue;
                }
                let out = &mut dst.pixels[((dy + j) * dst.width + dx + i) as usize];
                for c in 0..3 {
                    out[c] = ((avg[c] * a) >> 8) as u8;
                }
                out[3] = a as u8;
            }
        }
    }
}

/// Decode a single file by its extension.
pub fn decode_file(path: &Path) -> Result<Rgba, ImageError> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    let err = |source: Box<dyn std::error::Error + Send + Sync>| ImageError::Decode {
        path: path.display().to_string(),
        source,
    };
    let bytes = std::fs::read(path).map_err(|e| err(e.into()))?;
    match ext.as_deref() {
        Some("gif") => decode_gif(&bytes).map_err(|e| err(e.into())),
        _ => {
            let img = image::load_from_memory(&bytes).map_err(|e| err(e.into()))?;
            let rgba = img.to_rgba8();
            Ok(Rgba {
                width: rgba.width(),
                height: rgba.height(),
                pixels: rgba.pixels().map(|p| p.0).collect(),
            })
        }
    }
}

/// First frame only. Transparent palette index → `[0, 0, 0, 0]`, as in the
/// original's GIF decoder.
fn decode_gif(bytes: &[u8]) -> Result<Rgba, gif::DecodingError> {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::Indexed);
    let mut dec = opts.read_info(bytes)?;
    let global = dec.global_palette().map(<[u8]>::to_vec);
    let (width, height) = (dec.width() as u32, dec.height() as u32);
    let mut pixels = vec![[0u8; 4]; (width * height) as usize];
    if let Some(frame) = dec.read_next_frame()? {
        let pal = frame
            .palette
            .as_deref()
            .or(global.as_deref())
            .unwrap_or(&[]);
        let (fx, fy, fw) = (frame.left as u32, frame.top as u32, frame.width as u32);
        for (i, &idx) in frame.buffer.iter().enumerate() {
            let (x, y) = (fx + i as u32 % fw, fy + i as u32 / fw);
            if x >= width || y >= height {
                continue;
            }
            let px = if Some(idx) == frame.transparent {
                [0, 0, 0, 0]
            } else {
                let o = idx as usize * 3;
                match pal.get(o..o + 3) {
                    Some(c) => [c[0], c[1], c[2], 255],
                    None => [0, 0, 0, 255],
                }
            };
            pixels[(y * width + x) as usize] = px;
        }
    }
    Ok(Rgba {
        width,
        height,
        pixels,
    })
}

fn load_stem(data: &DataDir, stem: &str) -> Result<Option<Rgba>, ImageError> {
    match data.find_with_ext(stem, IMAGE_EXTS) {
        Some(p) => decode_file(&p).map(Some),
        None => Ok(None),
    }
}

/// `ImageLib::GetImage(path, lookForAlpha = true)`: colour image plus implicit
/// `_stem` / `stem_` mask.
pub fn get_image(data: &DataDir, path: &str) -> Result<Rgba, ImageError> {
    let color = load_stem(data, path)?;
    let (dir, stem) = match path.rsplit_once('/') {
        Some((d, s)) => (format!("{d}/"), s),
        None => (String::new(), path),
    };
    let mask = match load_stem(data, &format!("{dir}_{stem}"))? {
        Some(m) => Some(m),
        None => load_stem(data, &format!("{path}_"))?,
    };
    match (color, mask) {
        (Some(mut c), Some(m)) => {
            if c.same_size(&m) {
                c.apply_mask(&m);
            }
            Ok(c)
        }
        (Some(c), None) => Ok(c),
        (None, Some(m)) => {
            let [r, g, b] = rgb(DEFAULT_ALPHA_COLOR);
            Ok(Rgba {
                pixels: m.pixels.iter().map(|p| [r, g, b, p[2]]).collect(),
                ..m
            })
        }
        (None, None) => Err(ImageError::NotFound(path.to_string())),
    }
}

/// `ResourceManager::DoLoadImage`: implicit lookup, then explicit
/// `alphaimage` / `alphagrid`.
pub fn load_image_res(data: &DataDir, res: &ImageRes) -> Result<Rgba, ImageError> {
    let mut img = get_image(data, &res.path)?;
    if let Some(a) = &res.alpha_image {
        let mask = get_image(data, a)?;
        if !img.same_size(&mask) {
            return Err(ImageError::MaskSize {
                image: res.path.clone(),
                mask: a.clone(),
            });
        }
        img.apply_mask(&mask);
    }
    if let Some(a) = &res.alpha_grid {
        let mask = get_image(data, a)?;
        let (cw, ch) = (img.width / res.cols, img.height / res.rows);
        if mask.width != cw || mask.height != ch {
            return Err(ImageError::MaskSize {
                image: res.path.clone(),
                mask: a.clone(),
            });
        }
        for row in 0..res.rows {
            for col in 0..res.cols {
                for y in 0..ch {
                    for x in 0..cw {
                        let dst = ((row * ch + y) * img.width + col * cw + x) as usize;
                        img.pixels[dst][3] = mask.pixels[(y * cw + x) as usize][2];
                    }
                }
            }
        }
    }
    Ok(img)
}

fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_dir::test_data_dir;
    use crate::resources::{Manifest, Resource};

    fn manifest(d: &DataDir) -> Manifest {
        Manifest::parse(&std::fs::read_to_string(d.join("properties/resources.xml")).unwrap())
            .unwrap()
    }

    fn image_res<'a>(m: &'a Manifest, id: &str) -> &'a ImageRes {
        match m.get(id) {
            Some(Resource::Image(i)) => i,
            _ => panic!("{id}"),
        }
    }

    #[test]
    fn loads_every_manifest_image() {
        let Some(d) = test_data_dir() else { return };
        let m = manifest(&d);
        for r in m.iter() {
            if let Resource::Image(i) = r {
                load_image_res(&d, i).unwrap_or_else(|e| panic!("{}: {e}", i.id));
            }
        }
    }

    #[test]
    fn implicit_prefix_mask_applies() {
        let Some(d) = test_data_dir() else { return };
        // amp.gif + _amp.gif: some pixels must end up translucent.
        let img = get_image(&d, "images/amp").unwrap();
        assert!(img.pixels.iter().any(|p| p[3] == 0));
        assert!(img.pixels.iter().any(|p| p[3] == 255));
    }

    #[test]
    fn mask_only_image_is_white() {
        let Some(d) = test_data_dir() else { return };
        // TANKMASK1 ships only as _mask1.jpg.
        let img = get_image(&d, "images/mask1").unwrap();
        assert!(img.pixels.iter().all(|p| p[..3] == [255, 255, 255]));
    }

    #[test]
    fn alpha_grid_stamps_every_cel() {
        let Some(d) = test_data_dir() else { return };
        let m = manifest(&d);
        let res = image_res(&m, "IMAGE_MAINBUTTON");
        let img = load_image_res(&d, res).unwrap();
        let cw = img.width / res.cols;
        // Same alpha at the same position in each cel.
        for y in (0..img.height).step_by(7) {
            for x in (0..cw).step_by(5) {
                let a0 = img.pixels[(y * img.width + x) as usize][3];
                for c in 1..res.cols {
                    assert_eq!(img.pixels[(y * img.width + c * cw + x) as usize][3], a0);
                }
            }
        }
    }
}
