//! Thin wrapper over `xkbcommon-dl` for parsing the keymap delivered
//! by `wl_keyboard.keymap` and resolving X11 keysym names (e.g.
//! `"Left"`, `"XF86Back"`) to the runtime keycodes the compositor
//! actually uses for `tizen_keyrouter.set_keygrab`.
//!
//! Mirrors the C pattern in
//! `tizen-core-wayland/src/tizen-core-wl/tizen_core_wl_keygrab.c`
//! (`_key_string_to_keycodes`).

use std::ffi::CString;
use std::os::fd::OwnedFd;
use std::sync::OnceLock;

use memmap2::MmapOptions;
use xkbcommon_dl::{
    xkb_context, xkb_context_flags, xkb_keymap, xkb_keymap_compile_flags,
    xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1, xkb_keysym_flags, xkb_keysym_t, xkbcommon_handle,
    XkbCommon,
};

fn xkb() -> &'static XkbCommon {
    static HANDLE: OnceLock<&'static XkbCommon> = OnceLock::new();
    HANDLE.get_or_init(xkbcommon_handle)
}

/// Owned XKB context + parsed keymap loaded from a
/// `wl_keyboard.keymap` event.
pub(crate) struct XkbState {
    ctx: *mut xkb_context,
    keymap: *mut xkb_keymap,
}

// SAFETY: the C handles are only used through `&self` methods that
// don't touch shared mutable state outside the C library, which is
// itself thread-safe for these read-only queries; `Sync` is required
// so the wrapper can be shared via `Arc` between the connect-time
// snapshot on `Display` and the per-window state.
unsafe impl Send for XkbState {}
unsafe impl Sync for XkbState {}

impl XkbState {
    /// Parse the keymap fd + size delivered by `wl_keyboard.keymap`
    /// (`format = XKB_V1`). The fd is mmap'd; the resulting
    /// null-terminated string is fed to `xkb_keymap_new_from_string`.
    pub(crate) fn from_keymap_fd(fd: OwnedFd, size: u32) -> Option<Self> {
        // SAFETY: mmap the keymap region read-only. The fd is owned;
        // when this function returns, the mmap is dropped and the
        // kernel releases the mapping.
        let mmap = unsafe { MmapOptions::new().len(size as usize).map(&fd).ok()? };
        let ctx = unsafe { (xkb().xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS) };
        if ctx.is_null() {
            return None;
        }
        // The keymap string is NUL-terminated within the mmap region;
        // `xkb_keymap_new_from_string` wants a C string.
        let str_ptr = mmap.as_ptr() as *const std::os::raw::c_char;
        let keymap = unsafe {
            (xkb().xkb_keymap_new_from_string)(
                ctx,
                str_ptr,
                XKB_KEYMAP_FORMAT_TEXT_V1,
                xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
            )
        };
        if keymap.is_null() {
            unsafe { (xkb().xkb_context_unref)(ctx) };
            return None;
        }
        Some(Self { ctx, keymap })
    }

    /// Resolve a key NAME (e.g. `"Left"`, `"Return"`, `"XF86Back"`) to
    /// the keycode(s) bound to it in this keymap. The returned
    /// `Vec<u32>` may have more than one entry if the same keysym is
    /// mapped to multiple keycodes (e.g. `Tab` + `ISO_Left_Tab`).
    pub(crate) fn keycodes_for_name(&self, name: &str) -> Vec<u32> {
        let Ok(cname) = CString::new(name) else {
            return Vec::new();
        };
        let keysym: xkb_keysym_t = unsafe {
            (xkb().xkb_keysym_from_name)(cname.as_ptr(), xkb_keysym_flags::XKB_KEYSYM_NO_FLAGS)
        };
        if keysym == 0 {
            return Vec::new();
        }
        let min = unsafe { (xkb().xkb_keymap_min_keycode)(self.keymap) };
        let max = unsafe { (xkb().xkb_keymap_max_keycode)(self.keymap) };
        let mut out = Vec::new();
        for kc in min..=max {
            let mut syms_ptr: *const xkb_keysym_t = std::ptr::null();
            let n = unsafe {
                (xkb().xkb_keymap_key_get_syms_by_level)(self.keymap, kc, 0, 0, &mut syms_ptr)
            };
            if n > 0 && !syms_ptr.is_null() {
                let syms = unsafe { std::slice::from_raw_parts(syms_ptr, n as usize) };
                if syms.contains(&keysym) {
                    out.push(kc);
                }
            }
        }
        out
    }

    /// Inverse of [`Self::keycodes_for_name`]: given a keycode arriving
    /// in a `wl_keyboard.key` event, return the human-readable keysym
    /// name (`"Left"`, `"XF86Back"`, `"a"`, …) bound to it at level 0.
    /// Currently unused in this crate but kept for future debug
    /// surfacing and for the safe API the eframe layer can offer.
    #[allow(dead_code)]
    pub(crate) fn name_for_keycode(&self, keycode: u32) -> Option<String> {
        let mut syms_ptr: *const xkb_keysym_t = std::ptr::null();
        let n = unsafe {
            (xkb().xkb_keymap_key_get_syms_by_level)(self.keymap, keycode, 0, 0, &mut syms_ptr)
        };
        if n == 0 || syms_ptr.is_null() {
            return None;
        }
        let keysym = unsafe { *syms_ptr };
        if keysym == 0 {
            return None;
        }
        let mut buf = [0u8; 64];
        let written = unsafe {
            (xkb().xkb_keysym_get_name)(
                keysym,
                buf.as_mut_ptr() as *mut std::os::raw::c_char,
                buf.len() as _,
            )
        };
        if written <= 0 {
            return None;
        }
        let written = written as usize;
        std::str::from_utf8(&buf[..written]).ok().map(str::to_owned)
    }
}

impl Drop for XkbState {
    fn drop(&mut self) {
        unsafe {
            if !self.keymap.is_null() {
                (xkb().xkb_keymap_unref)(self.keymap);
            }
            if !self.ctx.is_null() {
                (xkb().xkb_context_unref)(self.ctx);
            }
        }
    }
}
