//! Tracker songs streamed live through libopenmpt, as BASS plays them in the
//! original: from a start order, looping forever inside the module (repeat
//! −1), so the song follows its own order list (intro once, then its loop
//! section) instead of restarting a fixed recording. Spec:
//! `docs/re/assets/music.md`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::asset::Asset;
use bevy::audio::{ChannelCount, Decodable, Sample, SampleRate, Source};
use bevy::reflect::TypePath;
use sexy_core::music::{Module, Openmpt};

const RATE: u32 = 44_100;
/// Frames rendered per refill.
const CHUNK: usize = 1024;

/// A song: a module file and the order to start from.
#[derive(Asset, TypePath, Clone)]
pub struct Song {
    lib: Arc<Openmpt>,
    bytes: Arc<[u8]>,
    order: i32,
}

impl Song {
    /// `None` without libopenmpt or if the module doesn't load.
    pub fn new(lib: &Arc<Openmpt>, bytes: Arc<[u8]>, order: i32) -> Option<Self> {
        lib.open(&bytes).ok()?;
        Some(Song {
            lib: Arc::clone(lib),
            bytes,
            order,
        })
    }
}

/// The playing module. Each `AudioPlayer` gets its own (restarting a song
/// = spawning a new player).
pub struct SongDecoder {
    module: Arc<Mutex<Module>>,
    buf: Vec<i16>,
    pos: usize,
}

impl Decodable for Song {
    type Decoder = SongDecoder;

    fn decoder(&self) -> SongDecoder {
        let mut m = self
            .lib
            .open(&self.bytes)
            .expect("module loaded once already");
        m.set_repeat_count(-1);
        m.seek_order(self.order);
        SongDecoder {
            module: Arc::new(Mutex::new(m)),
            buf: Vec::new(),
            pos: 0,
        }
    }
}

impl Iterator for SongDecoder {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if self.pos >= self.buf.len() {
            self.buf.resize(CHUNK * 2, 0);
            let frames = self
                .module
                .lock()
                .map_or(0, |mut m| m.read_stereo(RATE, &mut self.buf));
            if frames == 0 {
                return None;
            }
            self.buf.truncate(frames * 2);
            self.pos = 0;
        }
        let s = self.buf[self.pos];
        self.pos += 1;
        Some(Sample::from(s) / 32768.0)
    }
}

impl Source for SongDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).expect("2 > 0")
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(RATE).expect("rate > 0")
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
