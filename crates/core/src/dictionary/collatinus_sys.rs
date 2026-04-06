//! Raw FFI bindings to the Collatinus C wrapper.

use std::os::raw::c_char;

extern "C" {
    /// On ARM Linux (Kobo), loads Qt5Core into the process via dlopen(RTLD_GLOBAL)
    /// so that Qt symbols are available before any Collatinus function is called.
    /// Must be called once before `collatinus_lookup`. No-op on other platforms.
    pub fn collatinus_preload();

    /// Look up a Latin word, returning a heap-allocated HTML string.
    /// Returns null on failure. Free with `collatinus_free_result`.
    pub fn collatinus_lookup(word: *const c_char, lang: *const c_char) -> *mut c_char;

    /// Free a string returned by `collatinus_lookup`.
    pub fn collatinus_free_result(result: *mut c_char);
}
