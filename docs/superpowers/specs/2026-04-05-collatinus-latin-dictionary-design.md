# Collatinus Latin Dictionary Integration

**Date:** 2026-04-05
**Branch:** add-decent-latin-support-to-plato
**Status:** Draft

## Goal

Add live Latin morphological analysis and declension-aware dictionary lookup to Plato by integrating Collatinus as a static C++ library. Latin words in any inflected form are looked up directly — no pre-generated dictionary files needed.

## Architecture

Four layers:

1. **Collatinus source** — downloaded into `thirdparty/collatinus/` via `thirdparty/download.sh` (daemon branch tarball from GitHub)
2. **C wrapper** — `collatinus_wrapper/collatinus_wrapper.cpp` exposes a plain C API over Collatinus's C++ internals
3. **Rust FFI** — `crates/core/src/dictionary/collatinus_sys.rs` (raw bindings) and `crates/core/src/dictionary/collatinus.rs` (safe wrapper struct)
4. **Virtual dictionaries** — `CollatinusDictionary` instances registered into `context.dictionaries` at startup, one per supported output language

## C Wrapper API

```c
// collatinus_wrapper.h

#ifdef __cplusplus
extern "C" {
#endif

// Look up a Latin word and return an HTML analysis in the given output language.
// lang: ISO 639-1 code ("fr", "en", "de", etc.)
// Returns a heap-allocated UTF-8 string. Caller must call collatinus_free_result().
// Returns NULL on failure.
char *collatinus_lookup(const char *word, const char *lang);

// Free a result returned by collatinus_lookup.
void collatinus_free_result(char *result);

#ifdef __cplusplus
}
#endif
```

Each call is self-contained from the Rust side — no init/destroy in the public API. Internally, the wrapper holds a lazy singleton: the Collatinus engine is initialized on the first call and kept alive for the process lifetime. Memory management is handled internally by the wrapper.

## Rust Integration

### Dictionary trait

Introduce a `Dictionary` trait in `crates/core/src/dictionary/`:

```rust
pub trait Dictionary {
    fn lookup(&mut self, word: &str, fuzzy: bool) -> Result<Vec<[String; 2]>, DictError>;
}
```

The existing `Dictionary` struct is renamed to `DictdDictionary` and implements this trait. `CollatinusDictionary` also implements it (`fuzzy` is ignored — Collatinus handles morphological matching internally).

`context.dictionaries` type changes from `BTreeMap<String, Dictionary>` to `BTreeMap<String, Box<dyn Dictionary>>`. No changes to `query_to_content` in the dictionary view.

### CollatinusDictionary

```rust
pub struct CollatinusDictionary {
    lang: String, // e.g. "fr", "en"
}

impl CollatinusDictionary {
    pub fn new(lang: &str) -> Self { ... }
}

impl Dictionary for CollatinusDictionary {
    fn lookup(&mut self, word: &str, _fuzzy: bool) -> Result<Vec<[String; 2]>, DictError> {
        // calls collatinus_sys::collatinus_lookup(word, lang)
        // returns vec with single entry: [word, html]
        // returns empty vec on NULL result
    }
}
```

### Virtual dictionary registration

At startup, one `CollatinusDictionary` is registered per supported output language:

```rust
for lang in COLLATINUS_LANGUAGES {
    let name = format!("Collatinus (la→{})", lang);
    context.dictionaries.insert(name, Box::new(CollatinusDictionary::new(lang)));
}
```

**TODO:** Derive `COLLATINUS_LANGUAGES` from Collatinus source code once vendored (likely an enum or data file listing supported languages). Do not hardcode.

The existing language-filtering logic in `query_to_content` means these virtual dictionaries will only activate when the reader's language is set to Latin (`la`), using the standard `dictionary.languages` settings.

## HTML Output

Collatinus returns HTML. The existing dictionary view already handles this: if the definition string starts with `<`, it is injected directly into the `HtmlDocument`. No UI changes needed.

## Build System

Mirrors the MuPDF wrapper pattern:

| File | Purpose |
|------|---------|
| `collatinus_wrapper/collatinus_wrapper.h` | C header |
| `collatinus_wrapper/collatinus_wrapper.cpp` | C wrapper implementation |
| `collatinus_wrapper/build.sh` | Builds for Linux and macOS |
| `collatinus_wrapper/build-kobo.sh` | Cross-compiles for `arm-unknown-linux-gnueabihf` |
| `crates/core/build.rs` | Extended to link `libcollatinus_wrapper` |
| `thirdparty/download.sh` | Extended with Collatinus tarball URL |

## Data Files

**TODO:** Determine where Collatinus stores its lexicon and morphological data files once the source is vendored. These will need to be:
- Bundled in Plato's data directory on Kobo (alongside `fonts/`, `dictionaries/`, etc.)
- Resolvable at a conventional path relative to the binary on Linux/macOS

## Out of Scope

- Lemma extraction + re-lookup in `.dict` files (possible future enhancement)
- Fuzzy matching (Collatinus handles this internally)
- Any changes to the dictionary popup UI
