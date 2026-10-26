//! Versioned per-tick state traces and first-divergence diffing.
//! Format: `crates/harness/README.md#traces-format-v1`.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FORMAT: &str = "insaniquarium-trace";
pub const VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("line {line}: {source}")]
    Json {
        line: usize,
        source: serde_json::Error,
    },
    #[error("not an insaniquarium trace")]
    NotATrace,
    #[error("unsupported trace version {0}")]
    Version(u32),
    #[error("empty trace")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RngState {
    /// Freshly seeded generator (`mti == 624`, state == `SRand(seed)`).
    Seed {
        seed: u32,
    },
    Full {
        mt: Vec<u32>,
        mti: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub format: String,
    pub version: u32,
    /// "original" or "sim".
    pub source: String,
    pub build: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub demo: Option<String>,
    pub start_tick: u32,
    pub rng: BTreeMap<String, RngState>,
    #[serde(default)]
    pub fields: Vec<String>,
}

impl Header {
    pub fn new(source: &str, build: &str, start_tick: u32) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            source: source.into(),
            build: build.into(),
            demo: None,
            start_tick,
            rng: BTreeMap::new(),
            fields: Vec::new(),
        }
    }
}

/// One tick of captured state. Kept as structured JSON so captures from the
/// original (field sets grow as RE progresses) and sim output can be
/// compared without both sides sharing Rust types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    #[serde(flatten)]
    pub state: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub header: Header,
    pub snapshots: Vec<Snapshot>,
}

impl Trace {
    /// Read a trace file, plain (`*.jsonl`) or zstd-compressed (the same
    /// name plus `.zst`, preferred when both exist). `None` if neither
    /// exists. Captures are stored compressed: they are 90% redundant JSON.
    pub fn open(path: impl AsRef<std::path::Path>) -> Option<Result<Self, TraceError>> {
        let path = path.as_ref();
        let mut zst = path.as_os_str().to_owned();
        zst.push(".zst");
        if let Ok(f) = std::fs::File::open(&zst) {
            return Some(
                zstd::Decoder::new(f)
                    .map_err(TraceError::from)
                    .and_then(|d| Self::read(std::io::BufReader::new(d))),
            );
        }
        let f = std::fs::File::open(path).ok()?;
        Some(Self::read(std::io::BufReader::new(f)))
    }

    pub fn read(r: impl BufRead) -> Result<Self, TraceError> {
        let mut lines = r.lines().enumerate();
        let (_, first) = lines.next().ok_or(TraceError::Empty)?;
        let header: Header =
            serde_json::from_str(&first?).map_err(|source| TraceError::Json { line: 1, source })?;
        if header.format != FORMAT {
            return Err(TraceError::NotATrace);
        }
        if header.version != VERSION {
            return Err(TraceError::Version(header.version));
        }
        let mut snapshots = Vec::new();
        for (i, line) in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            snapshots.push(
                serde_json::from_str(&line).map_err(|source| TraceError::Json {
                    line: i + 1,
                    source,
                })?,
            );
        }
        Ok(Self { header, snapshots })
    }

    pub fn write(&self, mut w: impl Write) -> Result<(), TraceError> {
        serde_json::to_writer(&mut w, &self.header).map_err(io_err)?;
        writeln!(w)?;
        for s in &self.snapshots {
            serde_json::to_writer(&mut w, s).map_err(io_err)?;
            writeln!(w)?;
        }
        Ok(())
    }
}

fn io_err(e: serde_json::Error) -> std::io::Error {
    std::io::Error::other(e)
}

/// First point where two traces disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct Divergence {
    pub tick: u32,
    pub path: String,
    pub expected: Value,
    pub actual: Value,
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tick {} {}: expected {} got {}",
            self.tick, self.path, self.expected, self.actual
        )
    }
}

/// Compare `actual` against `expected` tick by tick. Only keys present in
/// *both* snapshots are compared, so a capture can cover a subset of fields.
/// Ticks present in only one trace are skipped. Returns the first divergence.
pub fn diff(expected: &Trace, actual: &Trace) -> Option<Divergence> {
    let by_tick: BTreeMap<u32, &Snapshot> = actual.snapshots.iter().map(|s| (s.tick, s)).collect();
    for e in &expected.snapshots {
        let Some(a) = by_tick.get(&e.tick) else {
            continue;
        };
        for (k, ev) in &e.state {
            if let Some(av) = a.state.get(k)
                && let Some((path, x, y)) = first_diff(ev, av, k.clone())
            {
                return Some(Divergence {
                    tick: e.tick,
                    path,
                    expected: x,
                    actual: y,
                });
            }
        }
    }
    None
}

fn first_diff(a: &Value, b: &Value, path: String) -> Option<(String, Value, Value)> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => x.iter().find_map(|(k, va)| {
            y.get(k)
                .and_then(|vb| first_diff(va, vb, format!("{path}.{k}")))
        }),
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some((
                    format!("{path}.len"),
                    Value::from(x.len()),
                    Value::from(y.len()),
                ));
            }
            x.iter()
                .zip(y)
                .enumerate()
                .find_map(|(i, (va, vb))| first_diff(va, vb, format!("{path}[{i}]")))
        }
        // Exact numeric comparison: we aim for bit-exact replication.
        (Value::Number(x), Value::Number(y)) => {
            let same = match (x.as_i64(), y.as_i64()) {
                (Some(i), Some(j)) => i == j,
                _ => x.as_f64() == y.as_f64(),
            };
            (!same).then(|| (path, a.clone(), b.clone()))
        }
        _ => (a != b).then(|| (path, a.clone(), b.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snap(tick: u32, v: Value) -> Snapshot {
        Snapshot {
            tick,
            state: serde_json::from_value(v).unwrap(),
        }
    }

    fn trace(source: &str, snaps: Vec<Snapshot>) -> Trace {
        let mut header = Header::new(source, "test", 0);
        header
            .rng
            .insert("app".into(), RngState::Seed { seed: 0x7FEF_1344 });
        Trace {
            header,
            snapshots: snaps,
        }
    }

    #[test]
    fn roundtrips_jsonl() {
        let t = trace(
            "original",
            vec![snap(
                1,
                json!({"board": {"money": 200}, "objects": [{"list": "fish", "x": 312.5}]}),
            )],
        );
        let mut buf = Vec::new();
        t.write(&mut buf).unwrap();
        assert_eq!(Trace::read(&buf[..]).unwrap(), t);
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.starts_with(r#"{"format":"insaniquarium-trace","version":1"#));
    }

    #[test]
    fn rejects_unknown_version() {
        let line = r#"{"format":"insaniquarium-trace","version":99,"source":"x","build":"x","start_tick":0,"rng":{}}"#;
        assert!(matches!(
            Trace::read(line.as_bytes()),
            Err(TraceError::Version(99))
        ));
    }

    #[test]
    fn finds_first_divergence() {
        let a = trace(
            "original",
            vec![
                snap(
                    1,
                    json!({"board": {"money": 200}, "objects": [{"x": 1.0}, {"x": 2.0}]}),
                ),
                snap(
                    2,
                    json!({"board": {"money": 210}, "objects": [{"x": 1.5}, {"x": 2.5}]}),
                ),
            ],
        );
        let b = trace(
            "sim",
            vec![
                snap(
                    1,
                    json!({"board": {"money": 200}, "objects": [{"x": 1.0}, {"x": 2.0}]}),
                ),
                snap(
                    2,
                    json!({"board": {"money": 210}, "objects": [{"x": 1.5}, {"x": 2.25}]}),
                ),
            ],
        );
        let d = diff(&a, &b).unwrap();
        assert_eq!(d.tick, 2);
        assert_eq!(d.path, "objects[1].x");
        assert_eq!(d.to_string(), "tick 2 objects[1].x: expected 2.5 got 2.25");
        assert_eq!(diff(&a, &a), None);
    }

    #[test]
    fn compares_only_shared_fields_and_flags_list_length() {
        let a = trace("original", vec![snap(5, json!({"board": {"money": 5}}))]);
        let b = trace(
            "sim",
            vec![snap(5, json!({"board": {"money": 5, "extra": 1}}))],
        );
        assert_eq!(diff(&a, &b), None);
        let c = trace("original", vec![snap(5, json!({"objects": [1, 2]}))]);
        let d = trace("sim", vec![snap(5, json!({"objects": [1]}))]);
        assert_eq!(diff(&c, &d).unwrap().path, "objects.len");
    }
}
