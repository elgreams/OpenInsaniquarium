//! Tracker music (`.mo3`) decoding via libopenmpt, loaded at runtime.
//! Spec: `docs/re/assets/music.md`.
//!
//! libopenmpt is opened dynamically so builds never require it; without it
//! music is simply unavailable ([`Openmpt::load`] returns an error).

// FFI to a C library is inherently unsafe. Every unsafe block is confined to
// this module and wraps libopenmpt's documented C API.
#![allow(unsafe_code)]

use std::ffi::{c_char, c_double, c_int, c_void};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum MusicError {
    #[error("libopenmpt not available: {0}")]
    Library(String),
    #[error("libopenmpt could not load the module")]
    BadModule,
}

type CreateFn = unsafe extern "C" fn(
    *const c_void,
    usize,
    *const c_void,
    *mut c_void,
    *const c_void,
    *mut c_void,
    *mut c_int,
    *mut *const c_char,
    *const c_void,
) -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
/// `openmpt_module_read_interleaved_stereo(mod, samplerate, count, interleaved_stereo*)`.
type ReadStereoFn = unsafe extern "C" fn(*mut c_void, c_int, usize, *mut i16) -> usize;
type SetRepeatFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type SetOrderRowFn = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_double;
type GetIntFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type GetDoubleFn = unsafe extern "C" fn(*mut c_void) -> c_double;

/// A loaded libopenmpt shared library.
pub struct Openmpt {
    _lib: libloading::Library,
    create: CreateFn,
    destroy: DestroyFn,
    read_stereo: ReadStereoFn,
    set_repeat: SetRepeatFn,
    set_order_row: SetOrderRowFn,
    num_orders: GetIntFn,
    duration: GetDoubleFn,
    current_order: GetIntFn,
}

const LIB_NAMES: &[&str] = &[
    "libopenmpt.so.0",
    "libopenmpt.so",
    "libopenmpt.dll",
    "openmpt.dll",
    "libopenmpt.0.dylib",
    "libopenmpt.dylib",
];

impl Openmpt {
    pub fn load() -> Result<Arc<Self>, MusicError> {
        let mut last = String::from("not found");
        for name in LIB_NAMES {
            // SAFETY: loading libopenmpt runs only its (trivial) initializers.
            match unsafe { libloading::Library::new(*name) } {
                Ok(lib) => return Self::from_lib(lib).map(Arc::new),
                Err(e) => last = e.to_string(),
            }
        }
        Err(MusicError::Library(last))
    }

    fn from_lib(lib: libloading::Library) -> Result<Self, MusicError> {
        // SAFETY: symbol types match libopenmpt's C API (libopenmpt.h, API 0.x).
        unsafe {
            let sym = |name: &[u8]| -> Result<*const c_void, MusicError> {
                lib.get::<*const c_void>(name)
                    .map(|s| *s)
                    .map_err(|e| MusicError::Library(e.to_string()))
            };
            Ok(Self {
                create: std::mem::transmute::<*const c_void, CreateFn>(sym(
                    b"openmpt_module_create_from_memory2\0",
                )?),
                destroy: std::mem::transmute::<*const c_void, DestroyFn>(sym(
                    b"openmpt_module_destroy\0",
                )?),
                read_stereo: std::mem::transmute::<*const c_void, ReadStereoFn>(sym(
                    b"openmpt_module_read_interleaved_stereo\0",
                )?),
                set_repeat: std::mem::transmute::<*const c_void, SetRepeatFn>(sym(
                    b"openmpt_module_set_repeat_count\0",
                )?),
                set_order_row: std::mem::transmute::<*const c_void, SetOrderRowFn>(sym(
                    b"openmpt_module_set_position_order_row\0",
                )?),
                num_orders: std::mem::transmute::<*const c_void, GetIntFn>(sym(
                    b"openmpt_module_get_num_orders\0",
                )?),
                duration: std::mem::transmute::<*const c_void, GetDoubleFn>(sym(
                    b"openmpt_module_get_duration_seconds\0",
                )?),
                current_order: std::mem::transmute::<*const c_void, GetIntFn>(sym(
                    b"openmpt_module_get_current_order\0",
                )?),
                _lib: lib,
            })
        }
    }

    pub fn open(self: &Arc<Self>, data: &[u8]) -> Result<Module, MusicError> {
        // SAFETY: data is valid for the call; libopenmpt copies what it needs.
        let m = unsafe {
            (self.create)(
                data.as_ptr().cast(),
                data.len(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if m.is_null() {
            return Err(MusicError::BadModule);
        }
        Ok(Module {
            lib: Arc::clone(self),
            ptr: m,
        })
    }
}

/// A decoded tracker module.
pub struct Module {
    lib: Arc<Openmpt>,
    ptr: *mut c_void,
}

// SAFETY: a libopenmpt module handle may be used from any one thread at a
// time; `Module` is not `Sync`, and `&mut self` methods serialize access.
unsafe impl Send for Module {}

impl Module {
    pub fn num_orders(&self) -> i32 {
        // SAFETY: ptr is a live module.
        unsafe { (self.lib.num_orders)(self.ptr) }
    }

    pub fn duration_seconds(&self) -> f64 {
        // SAFETY: ptr is a live module.
        unsafe { (self.lib.duration)(self.ptr) }
    }

    /// The order being played.
    pub fn current_order(&self) -> i32 {
        // SAFETY: ptr is a live module.
        unsafe { (self.lib.current_order)(self.ptr) }
    }

    /// `-1` loops forever (the game's default), `0` plays once.
    pub fn set_repeat_count(&mut self, n: i32) {
        // SAFETY: ptr is a live module.
        unsafe { (self.lib.set_repeat)(self.ptr, n) };
    }

    /// Seek to the start of an order (the game selects songs this way).
    pub fn seek_order(&mut self, order: i32) {
        // SAFETY: ptr is a live module.
        unsafe { (self.lib.set_order_row)(self.ptr, order, 0) };
    }

    /// Render interleaved stereo i16 into `buf`; returns frames written
    /// (0 = end of song).
    pub fn read_stereo(&mut self, sample_rate: u32, buf: &mut [i16]) -> usize {
        let frames = buf.len() / 2;
        // SAFETY: buf holds 2 * frames samples, as libopenmpt requires.
        unsafe { (self.lib.read_stereo)(self.ptr, sample_rate as c_int, frames, buf.as_mut_ptr()) }
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: ptr came from create and is destroyed exactly once.
        unsafe { (self.lib.destroy)(self.ptr) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_game_music() {
        let (Ok(lib), Some(d)) = (Openmpt::load(), crate::data_dir::test_data_dir()) else {
            return;
        };
        for (file, min_orders) in [
            ("music/Insaniq2.mo3", 60),
            ("music/Alien.mo3", 1),
            ("music/Lullaby.mo3", 1),
        ] {
            let bytes = std::fs::read(d.find(file).unwrap()).unwrap();
            let mut m = lib.open(&bytes).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert!(m.num_orders() >= min_orders, "{file}");
            m.seek_order(m.num_orders().min(12));
            let mut buf = vec![0i16; 2 * 4410];
            assert_eq!(m.read_stereo(44100, &mut buf), 4410);
            assert!(buf.iter().any(|&s| s != 0), "{file} rendered silence");
        }
    }
}
