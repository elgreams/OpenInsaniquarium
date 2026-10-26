//! Reader for SexyApp `.dmo` input recordings.
//! Format: `docs/re/framework/demo-recording.md`.

pub const MAGIC: u32 = 0x42BE_EF78;

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum DemoError {
    #[error("not a demo file (bad magic)")]
    BadMagic,
    #[error("unsupported demo version {0}")]
    Version(u32),
    #[error("truncated demo file")]
    Truncated,
    #[error("bitstream ended mid-event at tick {0}")]
    BitstreamTruncated(u32),
    #[error("unknown demo command {cmd:#x} (long={long}) at tick {tick}")]
    UnknownCommand { cmd: u32, long: bool, tick: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemoEvent {
    /// Absolute cursor position (game pixels).
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseButton {
        button: MouseButton,
        down: bool,
        double_click: bool,
        x: i32,
        y: i32,
    },
    Activate(bool),
    Minimized(bool),
    KeyDown(u8),
    KeyUp(u8),
    Char(u16),
    MouseWheel(i32),
    MouseEnter,
    MouseLeave,
    Close,
    End,
    /// Result of a registry write (cmd 0x0C).
    RegistryWrite(bool),
    /// A registry read and the value it returned (cmd 0x0B); `None` = failed.
    RegistryRead(Option<(u32, Vec<u8>)>),
    /// Result of a registry existence/enum query (cmd 0x0E).
    RegistryQuery(bool),
    /// Contents of a data file read (cmd 0x0F); `None` = failed.
    FileRead(Option<Vec<u8>>),
    /// Result of a data file write (cmd 0x10).
    FileWrite(bool),
    /// HTTP transfer result (cmd 0x11): transfer id, result code, body.
    HttpResult {
        id: u32,
        result: u8,
        body: Vec<u8>,
    },
    /// Opaque buffer replay (cmd 0x12).
    Data(Vec<u8>),
    FullscreenToggle {
        windowed: bool,
        refresh: u8,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvent {
    /// Update count (28 ms logic ticks) at which the event is applied.
    pub tick: u32,
    pub event: DemoEvent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncPoint {
    pub name: Vec<u8>,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Demo {
    pub version: u32,
    /// Seed for the global `MTRand` (`0x005eb928`).
    pub rand_seed: u32,
    pub product_version: Vec<u8>,
    /// `DemoJumpTime` markers (usually empty).
    pub sync_points: Vec<SyncPoint>,
    pub total_ticks: u32,
    pub events: Vec<TimedEvent>,
}

struct Bytes<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DemoError> {
        let s = self
            .b
            .get(self.pos..self.pos + n)
            .ok_or(DemoError::Truncated)?;
        self.pos += n;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16, DemoError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, DemoError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

/// LSB-first bit reader matching `Buffer::ReadNumBits`.
struct Bits<'a> {
    b: &'a [u8],
    bit: usize,
}

impl Bits<'_> {
    fn read(&mut self, n: u32) -> Option<u32> {
        let mut v = 0u32;
        for i in 0..n {
            let byte = *self.b.get(self.bit / 8)?;
            if byte >> (self.bit % 8) & 1 != 0 {
                v |= 1 << i;
            }
            self.bit += 1;
        }
        Some(v)
    }

    fn read_signed(&mut self, n: u32) -> Option<i32> {
        let v = self.read(n)?;
        let shift = 32 - n;
        Some(((v << shift) as i32) >> shift)
    }
}

impl Demo {
    pub fn parse(data: &[u8]) -> Result<Self, DemoError> {
        let mut r = Bytes { b: data, pos: 0 };
        if r.u32()? != MAGIC {
            return Err(DemoError::BadMagic);
        }
        let version = r.u32()?;
        if version != 2 {
            return Err(DemoError::Version(version));
        }
        let rand_seed = r.u32()?;
        let len = r.u16()?.min(255) as usize;
        let product_version = r.take(len)?.to_vec();
        let reg_len = r.u32()? as usize;
        let sync_points = parse_sync_points(r.take(reg_len)?)?;
        let total_ticks = r.u32()?;
        // The bitstream has no length prefix: it runs to end of file.
        let events = parse_events(&data[r.pos..])?;
        Ok(Demo {
            version,
            rand_seed,
            product_version,
            sync_points,
            total_ticks,
            events,
        })
    }
}

fn parse_sync_points(b: &[u8]) -> Result<Vec<SyncPoint>, DemoError> {
    if b.is_empty() {
        return Ok(Vec::new());
    }
    let mut r = Bytes { b, pos: 0 };
    let n = r.u32()?;
    (0..n)
        .map(|_| {
            let len = r.u16()? as usize;
            let name = r.take(len)?.to_vec();
            let value = r.u32()?;
            Ok(SyncPoint { name, value })
        })
        .collect()
}

fn parse_events(b: &[u8]) -> Result<Vec<TimedEvent>, DemoError> {
    let mut bits = Bits { b, bit: 0 };
    let mut tick = 0u32;
    let (mut mx, mut my) = (0i32, 0i32);
    let mut out = Vec::new();
    // The final byte is zero-padded; stop when less than a header remains,
    // or when what's left can't hold a whole event (a stream can end
    // exactly on the last event's last bit, or inside the padding).
    while b.len() * 8 - bits.bit >= 6 {
        let mark = bits.bit;
        // A header cut off by the end of the stream is padding.
        let (Some(dt), Some(short)) = (bits.read(4), bits.read(1)) else {
            break;
        };
        let short = short == 1;
        let Some(cmd) = bits.read(if short { 1 } else { 5 }) else {
            break;
        };
        tick += dt;
        let event =
            decode_event(&mut bits, short, cmd, &mut mx, &mut my, tick).inspect_err(|_| {
                if std::env::var_os("DEMO_DEBUG").is_some() {
                    eprintln!(
                        "demo: failed at bit {mark} of {}, short {short} cmd {cmd:#x}",
                        b.len() * 8
                    );
                }
            })?;
        if let Some(event) = event {
            out.push(TimedEvent { tick, event });
        }
    }
    Ok(out)
}

fn decode_event(
    bits: &mut Bits<'_>,
    short: bool,
    cmd: u32,
    mx: &mut i32,
    my: &mut i32,
    tick: u32,
) -> Result<Option<DemoEvent>, DemoError> {
    let trunc = || DemoError::BitstreamTruncated(tick);
    // Multi-byte values are written byte-wise (LSB-first within the stream).
    let mut u = |n: u32| bits.read(n).ok_or_else(trunc);
    macro_rules! u32le {
        () => {{
            let mut v = 0u32;
            for i in 0..4 {
                v |= u(8)? << (8 * i);
            }
            v
        }};
    }
    macro_rules! blob {
        () => {{
            let n = u32le!() as usize;
            let mut v = Vec::with_capacity(n.min(1 << 20));
            for _ in 0..n {
                v.push(u(8)? as u8);
            }
            v
        }};
    }
    Ok(Some(match (short, cmd) {
        (true, 0) => {
            let dx = bits.read_signed(6).ok_or_else(trunc)?;
            let dy = bits.read_signed(6).ok_or_else(trunc)?;
            *mx += dx;
            *my += dy;
            DemoEvent::MouseMove { x: *mx, y: *my }
        }
        (true, 1) => {
            let down = bits.read(1).ok_or_else(trunc)? == 1;
            let btn = bits.read_signed(3).ok_or_else(trunc)?;
            let (button, double_click) = match btn {
                1 => (MouseButton::Left, false),
                -1 => (MouseButton::Left, true),
                2 => (MouseButton::Right, false),
                -2 => (MouseButton::Right, true),
                _ => (MouseButton::Middle, false),
            };
            DemoEvent::MouseButton {
                button,
                down,
                double_click,
                x: *mx,
                y: *my,
            }
        }
        (false, 0x00) => {
            *mx = u(12)? as i32;
            *my = u(12)? as i32;
            DemoEvent::MouseMove { x: *mx, y: *my }
        }
        (false, 0x01) => DemoEvent::Activate(u(1)? == 1),
        // WriteBoolean emits a whole byte.
        (false, 0x02) => DemoEvent::Minimized(u(8)? != 0),
        (false, 0x03) => DemoEvent::KeyDown(u(8)? as u8),
        (false, 0x04) => DemoEvent::KeyUp(u(8)? as u8),
        (false, 0x05) => {
            let wide = u(1)? == 1;
            DemoEvent::Char(u(if wide { 16 } else { 8 })? as u16)
        }
        (false, 0x06) => DemoEvent::Close,
        (false, 0x07) => DemoEvent::MouseEnter,
        (false, 0x08) => DemoEvent::MouseLeave,
        (false, 0x09) => DemoEvent::End,
        (false, 0x0B) => DemoEvent::RegistryRead(if u(1)? == 1 {
            let kind = u32le!();
            Some((kind, blob!()))
        } else {
            None
        }),
        (false, 0x0C) => DemoEvent::RegistryWrite(u(1)? == 1),
        (false, 0x0E) => DemoEvent::RegistryQuery(u(1)? == 1),
        (false, 0x0F) => DemoEvent::FileRead(if u(1)? == 1 { Some(blob!()) } else { None }),
        (false, 0x10) => DemoEvent::FileWrite(u(1)? == 1),
        (false, 0x11) => {
            let id = u32le!();
            let result = u(8)? as u8;
            DemoEvent::HttpResult {
                id,
                result,
                body: blob!(),
            }
        }
        (false, 0x12) => DemoEvent::Data(blob!()),
        (false, 0x15) => DemoEvent::MouseWheel(bits.read_signed(8).ok_or_else(trunc)?),
        (false, 0x17) => {
            let windowed = u(8)? != 0;
            DemoEvent::FullscreenToggle {
                windowed,
                refresh: u(8)? as u8,
            }
        }
        (false, 0x1F) => return Ok(None),
        (short, cmd) => {
            return Err(DemoError::UnknownCommand {
                cmd,
                long: !short,
                tick,
            });
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LSB-first bit writer mirroring `Buffer::WriteNumBits`.
    #[derive(Default)]
    struct W {
        bytes: Vec<u8>,
        bit: usize,
    }
    impl W {
        fn put(&mut self, v: i64, n: u32) {
            for i in 0..n {
                if self.bit.is_multiple_of(8) {
                    self.bytes.push(0);
                }
                if (v >> i) & 1 != 0 {
                    *self.bytes.last_mut().unwrap() |= 1 << (self.bit % 8);
                }
                self.bit += 1;
            }
        }
    }

    fn file(bitstream: &[u8], ticks: u32) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend(MAGIC.to_le_bytes());
        f.extend(2u32.to_le_bytes());
        f.extend(0x1234u32.to_le_bytes());
        f.extend(3u16.to_le_bytes());
        f.extend(b"1.1");
        f.extend(4u32.to_le_bytes());
        f.extend(0u32.to_le_bytes());
        f.extend(ticks.to_le_bytes());
        f.extend(bitstream);
        f
    }

    #[test]
    fn decodes_synthetic_stream() {
        let mut w = W::default();
        // tick 3: absolute move to (320, 200)
        w.put(3, 4);
        w.put(0, 1);
        w.put(0, 5);
        w.put(320, 12);
        w.put(200, 12);
        // tick 3+15+2 = 20 via a padding record, then left button down
        w.put(15, 4);
        w.put(0, 1);
        w.put(0x1F, 5);
        w.put(2, 4);
        w.put(1, 1);
        w.put(1, 1);
        w.put(1, 1);
        w.put(1, 3);
        // tick 21: relative move (-5, +7)
        w.put(1, 4);
        w.put(1, 1);
        w.put(0, 1);
        w.put(-5, 6);
        w.put(7, 6);
        let d = Demo::parse(&file(&w.bytes, 21)).unwrap();
        assert_eq!(d.rand_seed, 0x1234);
        assert_eq!(d.product_version, b"1.1");
        assert_eq!(
            d.events,
            vec![
                TimedEvent {
                    tick: 3,
                    event: DemoEvent::MouseMove { x: 320, y: 200 }
                },
                TimedEvent {
                    tick: 20,
                    event: DemoEvent::MouseButton {
                        button: MouseButton::Left,
                        down: true,
                        double_click: false,
                        x: 320,
                        y: 200
                    }
                },
                TimedEvent {
                    tick: 21,
                    event: DemoEvent::MouseMove { x: 315, y: 207 }
                },
            ]
        );
    }

    #[test]
    fn rejects_bad_magic() {
        assert_eq!(Demo::parse(&[0; 32]), Err(DemoError::BadMagic));
    }

    /// Recorded from the original (gitignored; skipped when absent).
    #[test]
    fn parses_recorded_demo() {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../traces/demos/tank1-1_scripted_01.dmo"
        );
        let Ok(bytes) = std::fs::read(p) else { return };
        let d = Demo::parse(&bytes).unwrap();
        assert_eq!(d.product_version, b"1.1");
        assert!(d.events.iter().all(|e| e.tick <= d.total_ticks));
        let clicks = d
            .events
            .iter()
            .filter(|e| matches!(e.event, DemoEvent::MouseButton { down: true, .. }))
            .count();
        // The scripted session: 190 left clicks, last event at the final tick.
        assert_eq!(clicks, 190);
        assert_eq!(d.events.last().unwrap().tick, d.total_ticks);
        assert!(
            d.events
                .iter()
                .any(|e| matches!(e.event, DemoEvent::FileRead(Some(_))))
        );
        for e in &d.events {
            if let DemoEvent::MouseMove { x, y } = e.event {
                assert!((0..640).contains(&x) && (0..480).contains(&y), "{e:?}");
            }
        }
    }
}
