//! SexyApp ImageFont descriptor (`data/*.txt`) parsing and text metrics.
//! Format notes: `docs/re/assets/overview.md#imagefont-descriptors-datatxt`.
//!
//! Descriptors are Latin-1 text. Characters are stored as `u8` (the original
//! is a narrow-char build), so a glyph table is indexed by byte value.

use std::collections::{BTreeMap, HashMap};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FontError {
    #[error("line {line}: {msg}")]
    Syntax { line: usize, msg: String },
    #[error("line {line}: undefined name `{name}`")]
    Undefined { line: usize, name: String },
    #[error("line {line}: unknown layer `{name}`")]
    UnknownLayer { line: usize, name: String },
    #[error("line {line}: {cmd}: {msg}")]
    BadArgs {
        line: usize,
        cmd: String,
        msg: String,
    },
}

/// A parsed value in the descriptor language.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    /// `'c'` character literal or `"str"` string (bytes, Latin-1).
    Str(Vec<u8>),
    /// Bare identifier, resolved via `Define` at command time.
    Ident(String),
    List(Vec<Value>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Glyph {
    /// Source rect on the layer image.
    pub rect: Rect,
    /// Draw offset relative to the pen position.
    pub offset: (i32, i32),
    /// Pen advance.
    pub width: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontLayer {
    pub name: String,
    /// Image stem relative to the descriptor's directory (no extension).
    pub image: Option<String>,
    pub glyphs: BTreeMap<u8, Glyph>,
    pub kerning: HashMap<(u8, u8), i32>,
    pub ascent: i32,
    pub ascent_padding: i32,
    pub line_spacing_offset: i32,
    pub point_size: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontData {
    pub layers: Vec<FontLayer>,
    pub default_point_size: i32,
}

impl FontData {
    /// Parse a descriptor. `src` is raw bytes (Latin-1).
    pub fn parse(src: &[u8]) -> Result<Self, FontError> {
        let stmts = parse_statements(src)?;
        let mut defs: HashMap<String, Value> = HashMap::new();
        let mut font = FontData::default();
        for (line, words) in stmts {
            let Some(Value::Ident(cmd)) = words.first() else {
                return Err(FontError::Syntax {
                    line,
                    msg: "statement must start with a command".into(),
                });
            };
            let args = &words[1..];
            let bad = |msg: &str| FontError::BadArgs {
                line,
                cmd: cmd.clone(),
                msg: msg.into(),
            };
            let resolve = |v: &Value| -> Result<Value, FontError> {
                match v {
                    Value::Ident(n) => defs.get(n).cloned().ok_or(FontError::Undefined {
                        line,
                        name: n.clone(),
                    }),
                    other => Ok(other.clone()),
                }
            };
            if cmd == "Define" {
                let [Value::Ident(name), v] = args else {
                    return Err(bad("expected `Define <name> <value>`"));
                };
                let v = resolve(v)?;
                defs.insert(name.clone(), v);
                continue;
            }
            if cmd == "SetDefaultPointSize" {
                font.default_point_size = as_int(&resolve(arg(args, 0, &bad)?)?, &bad)?;
                continue;
            }
            if cmd == "CreateLayer" {
                let Value::Ident(name) = arg(args, 0, &bad)? else {
                    return Err(bad("layer name must be an identifier"));
                };
                font.layers.push(FontLayer {
                    name: name.clone(),
                    ..Default::default()
                });
                continue;
            }
            // All remaining commands are `LayerSet* <layer> ...`.
            let Value::Ident(lname) = arg(args, 0, &bad)? else {
                return Err(bad("expected layer name"));
            };
            let layer = font.layers.iter_mut().find(|l| &l.name == lname).ok_or(
                FontError::UnknownLayer {
                    line,
                    name: lname.clone(),
                },
            )?;
            let a1 = || -> Result<Value, FontError> { resolve(arg(args, 1, &bad)?) };
            let a2 = || -> Result<Value, FontError> { resolve(arg(args, 2, &bad)?) };
            match cmd.as_str() {
                "LayerSetImage" => {
                    let Value::Str(s) = a1()? else {
                        return Err(bad("expected image name string"));
                    };
                    layer.image = Some(latin1(&s));
                }
                "LayerSetAscent" => layer.ascent = as_int(&a1()?, &bad)?,
                "LayerSetAscentPadding" => layer.ascent_padding = as_int(&a1()?, &bad)?,
                "LayerSetLineSpacingOffset" => layer.line_spacing_offset = as_int(&a1()?, &bad)?,
                "LayerSetPointSize" => layer.point_size = as_int(&a1()?, &bad)?,
                "LayerSetCharWidths" | "LayerSetImageMap" | "LayerSetCharOffsets" => {
                    let chars = as_chars(&a1()?, &bad)?;
                    let vals = as_list(&a2()?, &bad)?;
                    if chars.len() != vals.len() {
                        return Err(bad(&format!(
                            "{} chars but {} values",
                            chars.len(),
                            vals.len()
                        )));
                    }
                    for (c, v) in chars.into_iter().zip(vals) {
                        let g = layer.glyphs.entry(c).or_default();
                        match cmd.as_str() {
                            "LayerSetCharWidths" => g.width = as_int(&v, &bad)?,
                            "LayerSetImageMap" => {
                                let r = as_ints::<4>(&v, &bad)?;
                                g.rect = Rect {
                                    x: r[0],
                                    y: r[1],
                                    w: r[2],
                                    h: r[3],
                                };
                            }
                            _ => {
                                let o = as_ints::<2>(&v, &bad)?;
                                g.offset = (o[0], o[1]);
                            }
                        }
                    }
                }
                "LayerSetKerningPairs" => {
                    let pairs = as_list(&a1()?, &bad)?;
                    let vals = as_list(&a2()?, &bad)?;
                    if pairs.len() != vals.len() {
                        return Err(bad("pair/value count mismatch"));
                    }
                    for (p, v) in pairs.into_iter().zip(vals) {
                        let Value::Str(s) = p else {
                            return Err(bad("kerning pair must be a 2-char string"));
                        };
                        let [a, b] = s[..] else {
                            return Err(bad("kerning pair must be a 2-char string"));
                        };
                        layer.kerning.insert((a, b), as_int(&v, &bad)?);
                    }
                }
                other => {
                    return Err(FontError::Syntax {
                        line,
                        msg: format!("unknown command `{other}`"),
                    });
                }
            }
        }
        Ok(font)
    }

    /// Pen advance of `text` on the first layer, applying kerning.
    /// TODO(re): confirm against `ImageFont::StringWidth` (multi-layer max, spacing).
    pub fn string_width(&self, text: &[u8]) -> i32 {
        let Some(layer) = self.layers.first() else {
            return 0;
        };
        let mut w = 0;
        let mut prev: Option<u8> = None;
        for &c in text {
            if let Some(p) = prev {
                w += layer.kerning.get(&(p, c)).copied().unwrap_or(0);
            }
            w += layer.glyphs.get(&c).map_or(0, |g| g.width);
            prev = Some(c);
        }
        w
    }
}

fn arg<'a>(
    args: &'a [Value],
    i: usize,
    bad: &impl Fn(&str) -> FontError,
) -> Result<&'a Value, FontError> {
    args.get(i)
        .ok_or_else(|| bad(&format!("missing argument {}", i + 1)))
}

fn as_int(v: &Value, bad: &impl Fn(&str) -> FontError) -> Result<i32, FontError> {
    match v {
        Value::Int(i) => Ok(*i),
        _ => Err(bad("expected integer")),
    }
}

fn as_list(v: &Value, bad: &impl Fn(&str) -> FontError) -> Result<Vec<Value>, FontError> {
    match v {
        Value::List(l) => Ok(l.clone()),
        _ => Err(bad("expected list")),
    }
}

fn as_ints<const N: usize>(
    v: &Value,
    bad: &impl Fn(&str) -> FontError,
) -> Result<[i32; N], FontError> {
    let l = as_list(v, bad)?;
    if l.len() != N {
        return Err(bad(&format!("expected {N}-tuple")));
    }
    let mut out = [0; N];
    for (o, x) in out.iter_mut().zip(&l) {
        *o = as_int(x, bad)?;
    }
    Ok(out)
}

fn as_chars(v: &Value, bad: &impl Fn(&str) -> FontError) -> Result<Vec<u8>, FontError> {
    as_list(v, bad)?
        .into_iter()
        .map(|x| match x {
            Value::Str(s) if s.len() == 1 => Ok(s[0]),
            _ => Err(bad("expected single-character list")),
        })
        .collect()
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// Statements are sequences of values terminated by `;`.
fn parse_statements(src: &[u8]) -> Result<Vec<(usize, Vec<Value>)>, FontError> {
    let mut p = Lexer {
        src,
        pos: 0,
        line: 1,
    };
    let mut out = Vec::new();
    loop {
        p.skip_ws();
        if p.pos >= src.len() {
            return Ok(out);
        }
        let line = p.line;
        let mut words = Vec::new();
        loop {
            p.skip_ws();
            match p.peek() {
                None => {
                    return Err(FontError::Syntax {
                        line,
                        msg: "missing `;`".into(),
                    });
                }
                Some(b';') => {
                    p.pos += 1;
                    break;
                }
                Some(_) => words.push(p.value()?),
            }
        }
        out.push((line, words));
    }
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c == b'\n' {
                self.line += 1;
            }
            if c.is_ascii_whitespace() {
                self.pos += 1;
            } else if c == b'/' && self.src.get(self.pos + 1) == Some(&b'/') {
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn err(&self, msg: &str) -> FontError {
        FontError::Syntax {
            line: self.line,
            msg: msg.into(),
        }
    }

    fn value(&mut self) -> Result<Value, FontError> {
        match self.peek().ok_or_else(|| self.err("unexpected end"))? {
            b'(' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek() {
                        Some(b')') => {
                            self.pos += 1;
                            return Ok(Value::List(items));
                        }
                        Some(b',') => self.pos += 1,
                        Some(_) => items.push(self.value()?),
                        None => return Err(self.err("unterminated list")),
                    }
                }
            }
            q @ (b'\'' | b'"') => {
                // `\x` escapes the next byte (seen as `"\""` and `'\''`).
                self.pos += 1;
                let mut s = Vec::new();
                loop {
                    match self.peek() {
                        None => return Err(self.err("unterminated string")),
                        Some(c) if c == q => break,
                        Some(b'\\') => {
                            self.pos += 1;
                            s.push(self.peek().ok_or_else(|| self.err("unterminated string"))?);
                        }
                        Some(c) => s.push(c),
                    }
                    self.pos += 1;
                }
                self.pos += 1;
                Ok(Value::Str(s))
            }
            c if c == b'-' || c.is_ascii_digit() => {
                let start = self.pos;
                self.pos += 1;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                }
                let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
                s.parse()
                    .map(Value::Int)
                    .map_err(|_| self.err(&format!("bad integer `{s}`")))
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.pos;
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
                {
                    self.pos += 1;
                }
                Ok(Value::Ident(latin1(&self.src[start..self.pos])))
            }
            c => Err(self.err(&format!("unexpected character `{}`", c as char))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"Define CharList ('A', 'V', '\xe9');\r\n\
Define WidthList (7, 8, 6);\r\n\
Define RectList ((0, 0, 5, 11), (6, 0, 6, 11), (13, 0, 4, 11));\r\n\
Define OffsetList ((0, 0), (-1, 0), (0, -2));\r\n\
Define KerningPairs (\"AV\", \"\\\"/\", \"//\");\r\n\
Define KerningValues (-2, -1, -3);\r\n\
CreateLayer Main;\r\n\
LayerSetImage Main 'Pix118';\r\n\
LayerSetAscent Main 9;\r\n\
LayerSetCharWidths Main CharList WidthList;\r\n\
LayerSetCharWidths Main (' ') (3);\r\n\
LayerSetImageMap Main CharList RectList;\r\n\
LayerSetCharOffsets Main CharList OffsetList;\r\n\
LayerSetKerningPairs Main KerningPairs KerningValues;\r\n\
LayerSetLineSpacingOffset Main -5;\r\n\
LayerSetPointSize Main 8;\r\n\
SetDefaultPointSize 8;\r\n";

    #[test]
    fn parses_descriptor() {
        let f = FontData::parse(SAMPLE).unwrap();
        assert_eq!(f.default_point_size, 8);
        let l = &f.layers[0];
        assert_eq!(l.image.as_deref(), Some("Pix118"));
        assert_eq!((l.ascent, l.line_spacing_offset), (9, -5));
        assert_eq!(l.glyphs[&b' '].width, 3);
        let v = l.glyphs[&b'V'];
        assert_eq!(
            v.rect,
            Rect {
                x: 6,
                y: 0,
                w: 6,
                h: 11
            }
        );
        assert_eq!((v.offset, v.width), ((-1, 0), 8));
        assert_eq!(l.glyphs[&0xe9].offset, (0, -2));
        assert_eq!(l.kerning[&(b'A', b'V')], -2);
        assert_eq!(l.kerning[&(b'"', b'/')], -1);
        assert_eq!(l.kerning[&(b'/', b'/')], -3);
    }

    #[test]
    fn string_width_applies_kerning() {
        let f = FontData::parse(SAMPLE).unwrap();
        assert_eq!(f.string_width(b"AV"), 7 + 8 - 2);
        assert_eq!(f.string_width(b"A V"), 7 + 3 + 8);
    }

    #[test]
    fn errors_have_lines() {
        let e = FontData::parse(b"CreateLayer Main;\nLayerSetAscent Nope 3;").unwrap_err();
        assert_eq!(
            e,
            FontError::UnknownLayer {
                line: 2,
                name: "Nope".into()
            }
        );
    }

    #[test]
    fn parses_all_shipped_fonts() {
        let Some(d) = crate::data_dir::test_data_dir() else {
            return;
        };
        let mut n = 0;
        for e in std::fs::read_dir(d.join("data")).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "txt") {
                let f = FontData::parse(&std::fs::read(&p).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                let l = &f.layers[0];
                assert!(l.glyphs.len() > 60, "{}", p.display());
                // Some glyph sheets ship only as an alpha mask (`_name`).
                let img = l.image.as_ref().unwrap();
                let found = [format!("data/{img}"), format!("data/_{img}")]
                    .iter()
                    .any(|p| d.find_with_ext(p, &["gif", "png"]).is_some());
                assert!(found, "{img}");
                n += 1;
            }
        }
        assert_eq!(n, 15);
    }
}
