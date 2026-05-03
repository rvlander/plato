//! Raw FFI bindings to the Collatinus C wrapper.

use std::os::raw::{c_char, c_int};

extern "C" {
    /// Initialize Collatinus for a single target language.
    /// Must be called before the first `collatinus_lookup`.
    /// Returns 0 on success, -1 on failure.
    pub fn collatinus_init(lang: *const c_char) -> c_int;

    /// Look up a Latin word, returning a heap-allocated HTML string.
    /// Returns null on failure. Free with `collatinus_free_result`.
    pub fn collatinus_lookup(word: *const c_char, lang: *const c_char) -> *mut c_char;

    /// Free a string returned by `collatinus_lookup`.
    pub fn collatinus_free_result(result: *mut c_char);
}
