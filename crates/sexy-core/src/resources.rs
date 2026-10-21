//! `properties/resources.xml` manifest parsing. See `docs/re/assets/overview.md`.
//!
//! The original's XMLParser is lenient: attribute values may be unquoted
//! (`cols=3`) or omitted entirely (`ddsurface`). We tokenize tags ourselves
//! rather than use a strict XML crate.

use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ManifestError {
    #[error("line {line}: {msg}")]
    Syntax { line: usize, msg: String },
    #[error("line {line}: <{tag}> missing required attribute `{attr}`")]
    MissingAttr {
        line: usize,
        tag: String,
        attr: &'static str,
    },
    #[error("line {line}: invalid value {value:?} for `{attr}`")]
    BadValue {
        line: usize,
        attr: String,
        value: String,
    },
    #[error("line {line}: <{tag}> outside of a <Resources> group")]
    OutsideGroup { line: usize, tag: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimType {
    #[default]
    None,
    Once,
    Loop,
    PingPong,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Anim {
    pub kind: AnimType,
    /// Ticks per frame (see `docs/re/assets/images.md` for units).
    pub frame_delay: i32,
    pub begin_delay: i32,
    pub per_frame_delay: Vec<i32>,
    /// Explicit frame order; may contain `-1` (see spec).
    pub frame_map: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageRes {
    pub id: String,
    /// Directory-qualified path without extension, `/`-separated.
    pub path: String,
    pub alpha_image: Option<String>,
    pub alpha_grid: Option<String>,
    pub cols: u32,
    pub rows: u32,
    pub anim: Anim,
    pub dd_surface: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoundRes {
    pub id: String,
    pub path: String,
    pub volume: Option<f64>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontRes {
    pub id: String,
    /// Path to the descriptor, including its extension (e.g. `data/Pix118.txt`).
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Resource {
    Image(ImageRes),
    Sound(SoundRes),
    Font(FontRes),
}

impl Resource {
    pub fn id(&self) -> &str {
        match self {
            Resource::Image(r) => &r.id,
            Resource::Sound(r) => &r.id,
            Resource::Font(r) => &r.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceGroup {
    pub id: String,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Manifest {
    /// Groups in file order (load phase order: Init, Register, LoadingThread…).
    pub groups: Vec<ResourceGroup>,
}

impl Manifest {
    pub fn parse(src: &str) -> Result<Self, ManifestError> {
        Parser::default().run(src)
    }

    pub fn group(&self, id: &str) -> Option<&ResourceGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Resource> {
        self.groups.iter().flat_map(|g| &g.resources)
    }

    pub fn get(&self, id: &str) -> Option<&Resource> {
        self.iter().find(|r| r.id() == id)
    }
}

struct Tag<'a> {
    name: &'a str,
    closing: bool,
    attrs: BTreeMap<String, Option<String>>,
    line: usize,
}

impl Tag<'_> {
    fn get(&self, k: &str) -> Option<&str> {
        self.attrs.get(k).and_then(|v| v.as_deref())
    }

    fn has(&self, k: &str) -> bool {
        self.attrs.contains_key(k)
    }

    fn req(&self, k: &'static str) -> Result<&str, ManifestError> {
        self.get(k).ok_or_else(|| ManifestError::MissingAttr {
            line: self.line,
            tag: self.name.to_string(),
            attr: k,
        })
    }

    fn int(&self, k: &str, default: i32) -> Result<i32, ManifestError> {
        match self.get(k) {
            None => Ok(default),
            Some(v) => v.trim().parse().map_err(|_| self.bad(k, v)),
        }
    }

    fn int_list(&self, k: &str) -> Result<Vec<i32>, ManifestError> {
        match self.get(k) {
            None => Ok(Vec::new()),
            Some(v) => v
                .split(',')
                .map(|s| s.trim().parse().map_err(|_| self.bad(k, v)))
                .collect(),
        }
    }

    fn bad(&self, k: &str, v: &str) -> ManifestError {
        ManifestError::BadValue {
            line: self.line,
            attr: k.to_string(),
            value: v.to_string(),
        }
    }
}

#[derive(Default)]
struct Parser {
    groups: Vec<ResourceGroup>,
    current: Option<ResourceGroup>,
    default_path: String,
    id_prefix: String,
}

impl Parser {
    fn run(mut self, src: &str) -> Result<Manifest, ManifestError> {
        for tag in tokenize(src)? {
            self.handle(tag)?;
        }
        if let Some(g) = self.current.take() {
            self.groups.push(g);
        }
        Ok(Manifest {
            groups: self.groups,
        })
    }

    fn handle(&mut self, t: Tag<'_>) -> Result<(), ManifestError> {
        match (t.name, t.closing) {
            ("ResourceManifest", _) => {}
            ("Resources", false) => {
                if let Some(g) = self.current.take() {
                    self.groups.push(g);
                }
                self.current = Some(ResourceGroup {
                    id: t.req("id")?.to_string(),
                    resources: Vec::new(),
                });
                self.default_path.clear();
                self.id_prefix.clear();
            }
            ("Resources", true) => {
                if let Some(g) = self.current.take() {
                    self.groups.push(g);
                }
            }
            ("SetDefaults", false) => {
                self.default_path = t.get("path").unwrap_or("").replace('\\', "/");
                self.id_prefix = t.get("idprefix").unwrap_or("").to_string();
            }
            ("Image" | "Sound" | "Font", false) => {
                let res = self.resource(&t)?;
                self.current
                    .as_mut()
                    .ok_or_else(|| ManifestError::OutsideGroup {
                        line: t.line,
                        tag: t.name.to_string(),
                    })?
                    .resources
                    .push(res);
            }
            (_, true) => {}
            (other, false) => {
                return Err(ManifestError::Syntax {
                    line: t.line,
                    msg: format!("unknown element <{other}>"),
                });
            }
        }
        Ok(())
    }

    fn resource(&self, t: &Tag<'_>) -> Result<Resource, ManifestError> {
        let id = format!("{}{}", self.id_prefix, t.req("id")?);
        let path = self.qualify(t.req("path")?);
        Ok(match t.name {
            "Image" => Resource::Image(ImageRes {
                id,
                path,
                alpha_image: t.get("alphaimage").map(|p| self.qualify(p)),
                alpha_grid: t.get("alphagrid").map(|p| self.qualify(p)),
                cols: t.int("cols", 1)?.max(1) as u32,
                rows: t.int("rows", 1)?.max(1) as u32,
                anim: Anim {
                    kind: match t.get("anim") {
                        None => AnimType::None,
                        Some(v) => match v.to_ascii_lowercase().as_str() {
                            "none" => AnimType::None,
                            "once" => AnimType::Once,
                            "loop" => AnimType::Loop,
                            "pingpong" => AnimType::PingPong,
                            _ => return Err(t.bad("anim", v)),
                        },
                    },
                    frame_delay: t.int("framedelay", 0)?,
                    begin_delay: t.int("begindelay", 0)?,
                    per_frame_delay: t.int_list("perframedelay")?,
                    frame_map: t.int_list("framemap")?,
                },
                dd_surface: t.has("ddsurface"),
            }),
            "Sound" => Resource::Sound(SoundRes {
                id,
                path,
                volume: match t.get("volume") {
                    None => None,
                    Some(v) => Some(v.trim().parse().map_err(|_| t.bad("volume", v))?),
                },
                alias: t.get("alias").map(str::to_string),
            }),
            "Font" => Resource::Font(FontRes { id, path }),
            _ => unreachable!(),
        })
    }

    fn qualify(&self, p: &str) -> String {
        let p = p.replace('\\', "/");
        if self.default_path.is_empty() {
            p
        } else {
            format!("{}/{}", self.default_path, p)
        }
    }
}

/// Split the document into tags, skipping comments, `<?…?>` and text.
fn tokenize(src: &str) -> Result<Vec<Tag<'_>>, ManifestError> {
    let mut tags = Vec::new();
    let mut i = 0;
    let line_at = |pos: usize| src[..pos].matches('\n').count() + 1;
    while let Some(off) = src[i..].find('<') {
        let start = i + off;
        if src[start..].starts_with("<!--") {
            let end = src[start..]
                .find("-->")
                .ok_or_else(|| ManifestError::Syntax {
                    line: line_at(start),
                    msg: "unterminated comment".into(),
                })?;
            i = start + end + 3;
            continue;
        }
        let end = start
            + src[start..]
                .find('>')
                .ok_or_else(|| ManifestError::Syntax {
                    line: line_at(start),
                    msg: "unterminated tag".into(),
                })?;
        i = end + 1;
        let body = &src[start + 1..end];
        if body.starts_with('?') || body.starts_with('!') {
            continue;
        }
        let (closing, body) = match body.strip_prefix('/') {
            Some(b) => (true, b),
            None => (false, body),
        };
        let body = body.strip_suffix('/').unwrap_or(body);
        let name_end = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        tags.push(Tag {
            name: &body[..name_end],
            closing,
            attrs: parse_attrs(&body[name_end..]),
            line: line_at(start),
        });
    }
    Ok(tags)
}

fn parse_attrs(s: &str) -> BTreeMap<String, Option<String>> {
    let mut out = BTreeMap::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let ks = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' {
            i += 1;
        }
        if ks == i {
            break;
        }
        let key = s[ks..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                let vs = i + 1;
                i = vs;
                while i < b.len() && b[i] != q {
                    i += 1;
                }
                let v = &s[vs..i];
                i += 1;
                Some(v.to_string())
            } else {
                let vs = i;
                while i < b.len() && !b[i].is_ascii_whitespace() {
                    i += 1;
                }
                Some(s[vs..i].to_string())
            }
        } else {
            None
        };
        out.insert(key, value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<ResourceManifest>
<!-- comment with <Image id="NOPE" path="x"/> inside -->
<Resources id="Init">
<SetDefaults path="images" idprefix="IMAGE_" />
<Image id="BTN"   path="btn" alphagrid="btn_" cols=3 />
<Image id="BG"    path="bg" ddsurface />
<Image id="BLINK" path="blink" cols="3" anim="once" framemap="0,1,2,1,0,-1" perframedelay="3,2,3,1,1,1" />
<Image id="WAVE"  path="wave" rows="12" anim="pingpong" framedelay="4" begindelay="6" />
</Resources>
<Resources id="Register">
<SetDefaults path="sounds" idprefix="SOUND_" />
<Sound id="ZZAM" path="zzam" volume="0.8" alias="END"/>
<SetDefaults path="data" idprefix="FONT_" />
<Font id="PIX" path="Pix118.txt" />
</Resources>
</ResourceManifest>"#;

    #[test]
    fn parses_lenient_manifest() {
        let m = Manifest::parse(SAMPLE).unwrap();
        assert_eq!(m.groups.len(), 2);
        assert_eq!(m.groups[0].id, "Init");
        assert!(m.get("NOPE").is_none());

        let Some(Resource::Image(btn)) = m.get("IMAGE_BTN") else {
            panic!()
        };
        assert_eq!(btn.path, "images/btn");
        assert_eq!(btn.alpha_grid.as_deref(), Some("images/btn_"));
        assert_eq!((btn.cols, btn.rows), (3, 1));

        let Some(Resource::Image(bg)) = m.get("IMAGE_BG") else {
            panic!()
        };
        assert!(bg.dd_surface);

        let Some(Resource::Image(blink)) = m.get("IMAGE_BLINK") else {
            panic!()
        };
        assert_eq!(blink.anim.kind, AnimType::Once);
        assert_eq!(blink.anim.frame_map, [0, 1, 2, 1, 0, -1]);
        assert_eq!(blink.anim.per_frame_delay, [3, 2, 3, 1, 1, 1]);

        let Some(Resource::Image(wave)) = m.get("IMAGE_WAVE") else {
            panic!()
        };
        assert_eq!(wave.anim.kind, AnimType::PingPong);
        assert_eq!((wave.anim.frame_delay, wave.anim.begin_delay), (4, 6));
        assert_eq!((wave.cols, wave.rows), (1, 12));

        let Some(Resource::Sound(z)) = m.get("SOUND_ZZAM") else {
            panic!()
        };
        assert_eq!(z.path, "sounds/zzam");
        assert_eq!(z.volume, Some(0.8));
        assert_eq!(z.alias.as_deref(), Some("END"));

        let Some(Resource::Font(f)) = m.get("FONT_PIX") else {
            panic!()
        };
        assert_eq!(f.path, "data/Pix118.txt");
    }

    #[test]
    fn reports_bad_values_with_line() {
        let err =
            Manifest::parse("<Resources id=\"a\">\n<Image id=\"X\" path=\"x\" cols=\"three\"/>")
                .unwrap_err();
        assert_eq!(
            err,
            ManifestError::BadValue {
                line: 2,
                attr: "cols".into(),
                value: "three".into()
            }
        );
    }

    #[test]
    fn parses_real_manifest() {
        let Some(d) = crate::data_dir::test_data_dir() else {
            return;
        };
        let src = std::fs::read_to_string(d.join("properties/resources.xml")).unwrap();
        let m = Manifest::parse(&src).unwrap();
        let ids: Vec<_> = m.groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["Init", "Register", "LoadingThread"]);
        let count = |f: fn(&Resource) -> bool| m.iter().filter(|r| f(r)).count();
        assert_eq!(count(|r| matches!(r, Resource::Image(_))), 246);
        assert_eq!(count(|r| matches!(r, Resource::Sound(_))), 65);
        assert_eq!(count(|r| matches!(r, Resource::Font(_))), 15);
        // Every referenced file exists on disk with some extension. Some images
        // ship only as an alpha mask (`_name` or `name_`), with no color file.
        let exts = &["png", "gif", "jpg"];
        for r in m.iter() {
            match r {
                Resource::Image(i) => {
                    let (dir, stem) = i.path.rsplit_once('/').unwrap();
                    let found = [
                        i.path.clone(),
                        format!("{dir}/_{stem}"),
                        format!("{}_", i.path),
                    ]
                    .iter()
                    .any(|p| d.find_with_ext(p, exts).is_some());
                    assert!(found, "missing {}", i.path);
                }
                Resource::Sound(s) => {
                    assert!(d.find_with_ext(&s.path, &["ogg", "au", "wav"]).is_some());
                }
                Resource::Font(f) => assert!(d.find(&f.path).is_some(), "{}", f.path),
            }
        }
    }
}
