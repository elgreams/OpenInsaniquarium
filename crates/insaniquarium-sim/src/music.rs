//! Music state as the original's `BassMusicInterface` keeps it: per track a
//! volume, a per-update fade, a stop-at-zero flag and whether it's playing,
//! plus the app's last request (track, order). Deterministic and
//! frontend-free, so the harness compares it with captures and the app only
//! mirrors it onto players. Spec: `docs/re/assets/music.md`,
//! `docs/re/framework/presentation-capture.md`.

use crate::sound::Music as Cue;

/// Loaded tracks (`LoadingThreadProc` → `LoadMusic(id, path)`). Requested
/// ids 0 and 2 are remapped onto track 4 at an order offset.
pub const ALIEN: i32 = 1;
pub const LULLABY: i32 = 3;
pub const MAIN: i32 = 4;

/// Fade speeds per update: out `0x005a1a00`, in `0x005a19f8`.
pub const FADE_OUT: f64 = 0.008;
pub const FADE_IN: f64 = 0.002;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Track {
    /// The BASS channel is playing.
    pub playing: bool,
    /// Order the track was last started from (after remapping).
    pub order: i32,
    /// Bumped on every (re)start, so a frontend knows to restart.
    pub start: u32,
    /// `+0x20`
    pub volume: f64,
    /// `+0x28`: added to the volume every update.
    pub fade: f64,
    /// `+0x38`: stop when a fade-out reaches 0 (also the no-loop flag).
    pub stop_flag: bool,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            playing: false,
            order: 0,
            start: 0,
            volume: 1.0,
            fade: 0.0,
            stop_flag: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MusicState {
    /// Tracks 0..=4 (0 and 2 are never played: remapped to 4).
    pub tracks: [Track; 5],
    /// `app+0x88c`, `+0x890`: the last request, before remapping.
    pub requested: (i32, i32),
}

impl Default for MusicState {
    fn default() -> Self {
        MusicState {
            tracks: [Track::default(); 5],
            requested: (-1, 0),
        }
    }
}

/// `WinFishApp::RemapMusicTrack` (`0x0054b090`).
pub fn remap(track: i32, order: i32) -> (i32, i32) {
    match (track, order) {
        (0, 0x16) => (MAIN, 0),
        (0, 0) => (MAIN, 0xc),
        (0, 0xd) => (MAIN, 0x3a),
        (2, 0x20) => (MAIN, 0x19),
        (2, 0x2d) => (MAIN, 0x25),
        (2, 0) => (MAIN, 0x2d),
        (2, 0x36) => (MAIN, 0x31),
        (2, 5) => (MAIN, 0x32),
        (0 | 2, o) => (MAIN, o),
        other => other,
    }
}

impl MusicState {
    fn track(&mut self, id: i32) -> &mut Track {
        &mut self.tracks[remap(id, 0).0 as usize]
    }

    /// Interface slot 2 (`0x00498d10`): volume to max, no fade, play from
    /// `order`.
    fn slot_play(&mut self, id: i32, order: i32, no_loop: bool) {
        let t = &mut self.tracks[id as usize];
        t.volume = 1.0;
        t.fade = 0.0;
        t.stop_flag = no_loop;
        t.order = order.max(0);
        t.start += 1;
        t.playing = true;
    }

    /// Interface slot 3 (`0x00498e40`): volume 0 and stop.
    fn slot_stop(&mut self, id: i32) {
        let t = &mut self.tracks[id as usize];
        t.volume = 0.0;
        t.playing = false;
    }

    /// Interface slot 11 (`0x004992e0`): set the fade, then restart the
    /// channel at its current volume (from `order`, or from the top for −1).
    fn slot_fade_in(&mut self, id: i32, order: i32, speed: f64, no_loop: bool) {
        let t = &mut self.tracks[id as usize];
        t.fade = speed;
        t.stop_flag = no_loop;
        t.order = order.max(0);
        t.start += 1;
        t.playing = true;
    }

    /// `WinFishApp::StopAllMusic` (`0x0054b020`): slot 3 on ids 0..=4.
    pub fn stop_all(&mut self) {
        for id in 0..5 {
            self.slot_stop(id);
        }
        self.requested.0 = -1;
    }

    /// `0x0054b1a0(track, order, no_loop)`: record and play at once.
    pub fn play_now(&mut self, track: i32, order: i32) {
        self.requested = (track, order);
        let (id, o) = remap(track, order);
        self.slot_play(id, o, false);
    }

    /// `WinFishApp::PlayMusic(track, order, fade, no_loop)` (`0x0054b1e0`).
    pub fn play(&mut self, track: i32, order: i32, fade: f64) {
        self.requested = (track, order);
        let (id, o) = if order < 0 {
            (remap(track, 0).0, -1)
        } else {
            remap(track, order)
        };
        self.slot_fade_in(id, o, fade, false);
    }

    /// `WinFishApp::FadeOutMusic(track, stop, speed)` (`0x0054b230`) →
    /// slot 12 (`0x00499420`): only a track whose volume isn't 0 fades.
    pub fn fade_out(&mut self, track: i32, stop: bool, speed: f64) {
        if self.requested.0 == track {
            self.requested.0 = -1;
        }
        let t = self.track(track);
        if t.volume != 0.0 {
            t.fade = -speed;
            t.stop_flag = stop;
        }
    }

    /// `WinFishApp::PlayAlienMusic(true)` (`0x0054c480`), tanks 1–2: fade
    /// the tank song out (without stopping it); play the alien song, stop
    /// it (volume 0), and restart it from the top fading in at 0.002.
    pub fn alien(&mut self) {
        self.fade_out(0, false, FADE_OUT);
        self.play_now(ALIEN, 0);
        self.slot_stop(ALIEN);
        self.play(ALIEN, -1, FADE_IN);
    }

    /// `WinFishApp::PlayTankMusic(fade)` (`0x0054c390`). With `fade`: fade
    /// the alien song out and stop it, and restart the tank song at its
    /// current volume fading in at 0.002. Without: stop all and restart it
    /// with fade 1.0 (full volume after one update).
    pub fn tank(&mut self, tank: i32, fade: bool) {
        let speed = if fade {
            self.fade_out(ALIEN, true, FADE_OUT);
            FADE_IN
        } else {
            self.stop_all();
            1.0
        };
        let (track, order) = match tank {
            1 => (0, 0x16),
            2 => (0, 0),
            3 => (2, 0x20),
            _ => (2, 0x2d),
        };
        self.play(track, order, speed);
    }

    /// The hatch screen's song (`hatch-screen.md#3`): stop all, then
    /// (2, 54) at once.
    pub fn hatch(&mut self) {
        self.stop_all();
        self.play_now(2, 0x36);
    }

    /// Apply a board's music request (`board.music`).
    pub fn cue(&mut self, cue: Option<Cue>, tank: i32) {
        match cue {
            Some(Cue::Alien) => self.alien(),
            Some(Cue::Tank) => self.tank(tank, true),
            None => {}
        }
    }

    /// Slot 19 (`0x004997d0`), once per update, before the widgets update.
    pub fn update(&mut self) {
        for t in &mut self.tracks {
            if t.fade == 0.0 {
                continue;
            }
            t.volume += t.fade;
            if t.volume > 1.0 {
                t.volume = 1.0;
                t.fade = 0.0;
            } else if t.volume < 0.0 {
                t.volume = 0.0;
                t.fade = 0.0;
                if t.stop_flag {
                    t.playing = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alien_song_fades_in_while_the_tank_song_fades_out() {
        let mut m = MusicState::default();
        m.tank(1, false);
        m.update();
        assert_eq!(m.tracks[MAIN as usize].volume, 1.0);
        m.alien();
        let a = m.tracks[ALIEN as usize];
        assert!(a.playing && a.volume == 0.0 && a.order == 0);
        for _ in 0..125 {
            m.update();
        }
        assert!(m.tracks[MAIN as usize].volume.abs() < 1e-9);
        assert!(m.tracks[MAIN as usize].playing, "faded, not stopped");
        assert!((m.tracks[ALIEN as usize].volume - 0.25).abs() < 1e-9);
        for _ in 0..400 {
            m.update();
        }
        assert_eq!(m.tracks[ALIEN as usize].volume, 1.0);
        // Last alien gone: the alien song fades out and stops; the tank
        // song restarts from its order, fading in from where it was (0).
        m.tank(1, true);
        let t = m.tracks[MAIN as usize];
        assert_eq!((t.order, t.volume, t.fade), (0, 0.0, FADE_IN));
        for _ in 0..200 {
            m.update();
        }
        assert!(!m.tracks[ALIEN as usize].playing);
    }

    #[test]
    fn hatch_song_is_track_4_order_49_at_full_volume() {
        let mut m = MusicState::default();
        m.hatch();
        let t = m.tracks[MAIN as usize];
        assert!(t.playing && t.order == 49 && t.volume == 1.0);
        assert_eq!(m.requested, (2, 0x36));
    }
}
