//! Insaniquarium-specific image loading: every manifest image plus the
//! images the original derives after loading.
//! Spec: `docs/re/assets/sprite-variants.md#load-time-derived-images`.
//!
//! Mirroring is *not* done here: the original flips cels at draw time
//! (`Sprite::flip_x`), never baking mirrored copies.

use std::collections::HashMap;

use sexy_core::data_dir::DataDir;
use sexy_core::image::{ImageError, Rgba, load_image_res};
use sexy_core::resources::{Manifest, Resource};

/// Pets in manifest order (`STINKY` … `WALTER`), matching `SCL_*` order.
pub const PETS: [&str; 24] = [
    "STINKY",
    "NIKO",
    "ITCHY",
    "PREGO",
    "ZORF",
    "CLYDE",
    "VERT",
    "RUFUS",
    "MERYL",
    "WADSWORTH",
    "SEYMOUR",
    "SHRAPNEL",
    "GUMBO",
    "BLIP",
    "RHUBARB",
    "NIMBUS",
    "AMP",
    "GASH",
    "ANGIE",
    "PRESTO",
    "BRINKLEY",
    "NOSTRADAMUS",
    "STANLEY",
    "WALTER",
];

/// Loaded images keyed by full resource id (`IMAGE_…`), plus the 40×40
/// pet-dialog thumbnails keyed by the full-size pet's id (`IMAGE_STINKY` …).
#[derive(Default)]
pub struct GameImages {
    pub images: HashMap<String, Rgba>,
    pub pet_thumbnails: HashMap<String, Rgba>,
}

impl GameImages {
    pub fn load(data: &DataDir, manifest: &Manifest) -> Result<Self, ImageError> {
        let mut images = HashMap::new();
        for r in manifest.iter() {
            if let Resource::Image(i) = r {
                images.insert(i.id.clone(), load_image_res(data, i)?);
            }
        }
        let mut g = GameImages {
            images,
            pet_thumbnails: HashMap::new(),
        };
        g.derive();
        Ok(g)
    }

    pub fn get(&self, id: &str) -> Option<&Rgba> {
        self.images.get(id)
    }

    /// `WinFishApp::LoadingThreadProc` / `WinFishApp::Init` post-load steps.
    fn derive(&mut self) {
        // 1. Crease strips tiled to 8×60 / 60×8.
        for (id, w, h) in [("IMAGE_VERTCREASE", 8, 60), ("IMAGE_HORZCREASE", 60, 8)] {
            if let Some(src) = self.images.get(id) {
                let tiled = src.tile(w, h);
                self.images.insert(id.into(), tiled);
            }
        }
        // 2. Tank foreground cut-outs: AQUARIUMn region at (0,365) with
        //    TANKMASKn alpha.
        for n in 1..=6 {
            let (aq, mk) = (format!("IMAGE_AQUARIUM{n}"), format!("IMAGE_TANKMASK{n}"));
            if let (Some(src), Some(mask)) = (self.images.get(&aq), self.images.get(&mk)) {
                let cut = src
                    .crop(0, 365, mask.width, mask.height)
                    .with_alpha_of(mask);
                self.images.insert(mk, cut);
            }
        }
        // 3. Title-page cut-out at (384,388).
        if let (Some(src), Some(mask)) = (
            self.images.get("IMAGE_TITLEPAGE"),
            self.images.get("IMAGE_TITLEPAGEMASK"),
        ) {
            let cut = src
                .crop(384, 388, mask.width, mask.height)
                .with_alpha_of(mask);
            self.images.insert("IMAGE_TITLEPAGEMASK".into(), cut);
        }
        // 4. 40×40 pet thumbnails from cel 0 of SCL_<pet>.
        for pet in PETS {
            let Some(scl) = self.images.get(&format!("IMAGE_SCL_{pet}")) else {
                continue;
            };
            let mut thumb = Rgba::new_transparent(40, 40);
            if pet == "AMP" {
                scl.crop(0, 0, 64, 24)
                    .box_downscale_into(&mut thumb, 0, 10, 40, 20);
            } else {
                scl.crop(0, 0, 60, 60)
                    .box_downscale_into(&mut thumb, 0, 0, 40, 40);
            }
            self.pet_thumbnails.insert(format!("IMAGE_{pet}"), thumb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sexy_core::data_dir::test_data_dir;

    #[test]
    fn derives_load_time_images() {
        let Some(d) = test_data_dir() else { return };
        let src = std::fs::read_to_string(d.join("properties/resources.xml")).unwrap();
        let g = GameImages::load(&d, &Manifest::parse(&src).unwrap()).unwrap();

        let v = g.get("IMAGE_VERTCREASE").unwrap();
        assert_eq!((v.width, v.height), (8, 60));
        let h = g.get("IMAGE_HORZCREASE").unwrap();
        assert_eq!((h.width, h.height), (60, 8));

        let aq = g.get("IMAGE_AQUARIUM1").unwrap();
        let tm = g.get("IMAGE_TANKMASK1").unwrap();
        assert_eq!((tm.width, tm.height), (640, 80));
        // RGB from the aquarium's rows 365.., alpha from the mask.
        let opaque = tm.pixels.iter().position(|p| p[3] == 255).unwrap() as u32;
        let (x, y) = (opaque % 640, opaque / 640);
        assert_eq!(
            tm.pixels[opaque as usize][..3],
            aq.pixels[((365 + y) * aq.width + x) as usize][..3]
        );

        let tp = g.get("IMAGE_TITLEPAGEMASK").unwrap();
        assert_eq!((tp.width, tp.height), (202, 67));
        assert!(tp.pixels.iter().any(|p| p[..3] != [255, 255, 255]));

        assert_eq!(g.pet_thumbnails.len(), 24);
        let amp = &g.pet_thumbnails["IMAGE_AMP"];
        // AMP lives in the 40×20 band at y = 10; rows outside stay empty.
        assert!(amp.pixels[..400].iter().all(|p| p[3] == 0));
        assert!(amp.pixels[400..1200].iter().any(|p| p[3] > 0));
    }
}
