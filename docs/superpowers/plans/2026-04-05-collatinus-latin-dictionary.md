# Collatinus Latin Dictionary Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate Collatinus as a static C++ library providing live Latin morphological analysis, exposed as virtual dictionaries (`Collatinus (la→fr)`, `Collatinus (la→en)`, etc.) in Plato's existing dictionary system.

**Architecture:** A thin C++ wrapper around Collatinus (with an internal lazy singleton) is compiled into a static lib, mirroring the MuPDF wrapper pattern. On the Rust side, the existing `Dictionary` struct is renamed `DictdDictionary` and a `Dictionary` trait is introduced so both backends share a common interface. Virtual `CollatinusDictionary` instances are registered at startup alongside the existing dictd dictionaries.

**Tech Stack:** Rust (FFI via `extern "C"`), C++17 (Collatinus wrapper), FreeType/HarfBuzz already present, Cargo build.rs for linking.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `crates/core/src/dictionary/mod.rs` | Add `Dictionary` trait, rename struct → `DictdDictionary` |
| Create | `crates/core/src/dictionary/collatinus_sys.rs` | Raw `extern "C"` FFI declarations |
| Create | `crates/core/src/dictionary/collatinus.rs` | Safe `CollatinusDictionary` implementing `Dictionary` trait |
| Modify | `crates/core/src/context.rs` | Change map type, add `load_collatinus_dictionaries()` |
| Modify | `crates/plato/src/app.rs` | Call `load_collatinus_dictionaries()` at startup |
| Modify | `crates/core/build.rs` | Link `libcollatinus_wrapper` |
| Create | `collatinus_wrapper/collatinus_wrapper.h` | C header for the wrapper |
| Create | `collatinus_wrapper/collatinus_wrapper.cpp` | C++ wrapper implementation |
| Create | `collatinus_wrapper/build.sh` | Build script (Linux/macOS) |
| Create | `collatinus_wrapper/build-kobo.sh` | Cross-compile build script |
| Modify | `thirdparty/download.sh` | Add Collatinus download entry |

---

## Task 1: Introduce `Dictionary` trait and rename existing struct

**Files:**
- Modify: `crates/core/src/dictionary/mod.rs`
- Modify: `crates/core/src/context.rs`

The existing `Dictionary` struct becomes `DictdDictionary`. A `pub trait Dictionary` is introduced with one method. `context.dictionaries` becomes `Box<dyn Dictionary>`.

- [ ] **Step 1: Write a failing compile test**

Add this at the bottom of `crates/core/src/dictionary/mod.rs` inside `#[cfg(test)]`:

```rust
#[cfg(test)]
mod trait_tests {
    use super::*;

    fn _assert_trait_object(_d: &dyn Dictionary) {}
}
```

This will fail to compile until the `Dictionary` trait exists.

- [ ] **Step 2: Introduce the `Dictionary` trait**

In `crates/core/src/dictionary/mod.rs`, add above the existing `Dictionary` struct definition:

```rust
pub trait Dictionary {
    fn lookup(&mut self, word: &str, fuzzy: bool) -> Result<Vec<[String; 2]>, errors::DictError>;
}
```

- [ ] **Step 3: Rename `Dictionary` struct to `DictdDictionary`**

In `crates/core/src/dictionary/mod.rs`, rename every occurrence of `pub struct Dictionary` and `impl Dictionary` (the struct impl blocks) to `DictdDictionary`. There are three places:
- The struct definition: `pub struct Dictionary {` → `pub struct DictdDictionary {`
- The impl block: `impl Dictionary {` → `impl DictdDictionary {`
- The return type of `load_dictionary`: `-> Dictionary` → `-> DictdDictionary`
- The return type of `load_dictionary_from_file`: `-> Result<Dictionary,` → `-> Result<DictdDictionary,`

- [ ] **Step 4: Implement `Dictionary` trait for `DictdDictionary`**

Add this impl block in `crates/core/src/dictionary/mod.rs` after the `impl DictdDictionary` block:

```rust
impl Dictionary for DictdDictionary {
    fn lookup(&mut self, word: &str, fuzzy: bool) -> Result<Vec<[String; 2]>, errors::DictError> {
        self.lookup(word, fuzzy)
    }
}
```

Wait — this creates an infinite recursion because the method name collides. Instead, rename the inherent `lookup` method on `DictdDictionary` to `lookup_inner`, and have both the trait impl and the old call sites use the right one.

Actually the cleaner approach: keep the inherent method as-is, but name the trait impl to delegate — the trait impl can call the inherent method because inside the `impl Dictionary for DictdDictionary` block, `self.lookup(...)` refers to the inherent method, not the trait method. This works in Rust when the trait method has the same signature. Use UFCS to be explicit:

```rust
impl Dictionary for DictdDictionary {
    fn lookup(&mut self, word: &str, fuzzy: bool) -> Result<Vec<[String; 2]>, errors::DictError> {
        DictdDictionary::lookup(self, word, fuzzy)
    }
}
```

- [ ] **Step 5: Update `context.rs`**

In `crates/core/src/context.rs`:

Change the import:
```rust
// Before:
use crate::dictionary::{Dictionary, load_dictionary_from_file};
// After:
use crate::dictionary::{Dictionary, DictdDictionary, load_dictionary_from_file};
```

Change the field type:
```rust
// Before:
pub dictionaries: BTreeMap<String, Dictionary>,
// After:
pub dictionaries: BTreeMap<String, Box<dyn Dictionary>>,
```

In `load_dictionaries()`, wrap the inserted dict in `Box::new`:
```rust
// Before:
self.dictionaries.insert(name, dict);
// After:
self.dictionaries.insert(name, Box::new(dict));
```

- [ ] **Step 6: Verify it compiles**

```bash
cd /path/to/plato
cargo build -p plato-core 2>&1 | head -40
```

Expected: compiles cleanly (zero errors).

- [ ] **Step 7: Run existing dictionary tests**

```bash
cargo test -p plato-core dictionary 2>&1
```

Expected: all tests pass (same tests as before, now using `DictdDictionary`).

- [ ] **Step 8: Commit**

```bash
git add crates/core/src/dictionary/mod.rs crates/core/src/context.rs
git commit -m "refactor: introduce Dictionary trait, rename struct to DictdDictionary"
```

---

## Task 2: Add Collatinus to thirdparty download script

**Files:**
- Modify: `thirdparty/download.sh`

- [ ] **Step 1: Research the Collatinus daemon branch tarball URL**

Look in the Collatinus repository (the daemon branch) for the correct GitHub archive URL. It follows the pattern:
```
https://github.com/<owner>/collatinus/archive/refs/heads/daemon.tar.gz
```

Find the actual owner/repo from the source you intend to use, then update the URL below.

- [ ] **Step 2: Add entry to `thirdparty/download.sh`**

In `thirdparty/download.sh`, add `"collatinus"` to the `urls` associative array:

```bash
# Latin morphological analysis
["collatinus"]="https://github.com/<owner>/collatinus/archive/refs/heads/daemon.tar.gz"
```

Replace `<owner>` with the actual GitHub org/user once confirmed.

- [ ] **Step 3: Test the download**

```bash
cd thirdparty
bash download.sh collatinus
ls collatinus/
```

Expected: directory populated with Collatinus source files.

- [ ] **Step 4: Identify the supported language list**

In the downloaded `thirdparty/collatinus/` source, search for the language definitions:

```bash
grep -r "francais\|français\|english\|latin\|langue\|language\|lang" thirdparty/collatinus/src/ | grep -i "enum\|const\|list\|array" | head -20
```

Note the language codes/identifiers — these will be used in Task 6 to register virtual dictionaries.

- [ ] **Step 5: Commit**

```bash
git add thirdparty/download.sh
git commit -m "build: add Collatinus to thirdparty download script"
```

---

## Task 3: Create the C++ wrapper

**Files:**
- Create: `collatinus_wrapper/collatinus_wrapper.h`
- Create: `collatinus_wrapper/collatinus_wrapper.cpp`
- Create: `collatinus_wrapper/build.sh`
- Create: `collatinus_wrapper/build-kobo.sh`

This task depends on Task 2 (Collatinus source must be downloaded first).

- [ ] **Step 1: Create the header**

Create `collatinus_wrapper/collatinus_wrapper.h`:

```c
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Look up a Latin word and return an HTML morphological analysis.
 * lang: ISO 639-1 output language code (e.g. "fr", "en")
 * Returns a heap-allocated UTF-8 string. Caller must call collatinus_free_result().
 * Returns NULL on failure.
 */
char *collatinus_lookup(const char *word, const char *lang);

/* Free a string returned by collatinus_lookup. */
void collatinus_free_result(char *result);

#ifdef __cplusplus
}
#endif
```

- [ ] **Step 2: Study Collatinus's public API**

Before writing the wrapper implementation, read the Collatinus daemon branch headers to find the correct classes and methods for lemmatization + HTML output. Look for:

```bash
ls thirdparty/collatinus/src/
grep -r "html\|HTML\|lemmatise\|lemmatize\|analyse" thirdparty/collatinus/src/*.h | head -20
```

Note the class name, constructor signature, and method for producing HTML output. You will need this for Step 3.

- [ ] **Step 3: Create the wrapper implementation**

Create `collatinus_wrapper/collatinus_wrapper.cpp`. The wrapper uses a lazy singleton to avoid reloading data files on every call:

```cpp
#include "collatinus_wrapper.h"
#include <cstring>
#include <mutex>
#include <memory>

// TODO: replace these includes with the actual Collatinus headers
// found in thirdparty/collatinus/src/
#include "../thirdparty/collatinus/src/lemmatiseur.h"

static std::unique_ptr<Lemmat> g_lemmat;  // TODO: replace Lemmat with actual class name
static std::once_flag g_init_flag;

static void ensure_initialized() {
    std::call_once(g_init_flag, []() {
        // TODO: construct with correct arguments (data path, etc.)
        // based on Collatinus API discovered in Step 2
        g_lemmat = std::make_unique<Lemmat>();
    });
}

extern "C" {

char *collatinus_lookup(const char *word, const char *lang) {
    if (!word || !lang) return nullptr;
    try {
        ensure_initialized();
        // TODO: call the correct method on g_lemmat
        // returning HTML for the given word in the given language.
        // Replace the line below with the actual API call:
        std::string result = g_lemmat->lemmatiseHtml(word, lang);
        if (result.empty()) return nullptr;
        char *out = static_cast<char *>(malloc(result.size() + 1));
        if (!out) return nullptr;
        memcpy(out, result.c_str(), result.size() + 1);
        return out;
    } catch (...) {
        return nullptr;
    }
}

void collatinus_free_result(char *result) {
    free(result);
}

} // extern "C"
```

- [ ] **Step 4: Create `build.sh`**

Create `collatinus_wrapper/build.sh`:

```sh
#! /bin/sh

set -e

CXX=${CXX:-g++}
AR=${AR:-ar}

TARGET_OS=${TARGET_OS:-$(uname -s)}
BUILD_DIR=../target/collatinus_wrapper/${TARGET_OS}
mkdir -p ${BUILD_DIR}

# TODO: add all Collatinus .cpp source files that need to be compiled.
# Discover them with: find ../thirdparty/collatinus/src -name "*.cpp" | head -20
# Then list the ones required (lemmatiseur, lexique, etc.) explicitly here.
COLLATINUS_SOURCES=""  # TODO: fill in

${CXX} ${CPPFLAGS} ${CXXFLAGS} -std=c++17 \
    -I../thirdparty/collatinus/src \
    -c collatinus_wrapper.cpp \
    -o ${BUILD_DIR}/collatinus_wrapper.o

# TODO: compile each Collatinus source file and add its .o to the archive
${AR} -rcs ${BUILD_DIR}/libcollatinus_wrapper.a \
    ${BUILD_DIR}/collatinus_wrapper.o
    # TODO: add compiled Collatinus object files here
```

- [ ] **Step 5: Create `build-kobo.sh`**

Create `collatinus_wrapper/build-kobo.sh`:

```sh
#! /bin/sh

TARGET_OS=Kobo CXX=arm-linux-gnueabihf-g++ AR=arm-linux-gnueabihf-ar ./build.sh
```

- [ ] **Step 6: Fill in the TODO stubs**

Now use your research from Step 2 to:
1. Replace `Lemmat` with the actual Collatinus class name in `collatinus_wrapper.cpp`
2. Replace the `lemmatiseHtml` call with the actual method
3. Fill in `COLLATINUS_SOURCES` in `build.sh` with the Collatinus `.cpp` files needed
4. Add the compiled object files to the `ar` archive in `build.sh`

- [ ] **Step 7: Build and test the wrapper**

```bash
cd collatinus_wrapper
bash build.sh
ls ../target/collatinus_wrapper/$(uname -s)/
```

Expected: `libcollatinus_wrapper.a` present.

- [ ] **Step 8: Commit**

```bash
git add collatinus_wrapper/
git commit -m "build: add Collatinus C++ wrapper"
```

---

## Task 4: Update `build.rs` to link the wrapper

**Files:**
- Modify: `crates/core/build.rs`

- [ ] **Step 1: Add collatinus_wrapper link directives**

Edit `crates/core/build.rs`. Add the collatinus search path and lib directive alongside the mupdf ones:

```rust
use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    if target == "arm-unknown-linux-gnueabihf" {
        println!("cargo:rustc-env=PKG_CONFIG_ALLOW_CROSS=1");
        println!("cargo:rustc-link-search=target/mupdf_wrapper/Kobo");
        println!("cargo:rustc-link-search=target/collatinus_wrapper/Kobo");
        println!("cargo:rustc-link-search=libs");
        println!("cargo:rustc-link-lib=dylib=stdc++");
    } else {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        match target_os.as_ref() {
            "linux" => {
                println!("cargo:rustc-link-search=target/mupdf_wrapper/Linux");
                println!("cargo:rustc-link-search=target/collatinus_wrapper/Linux");
                println!("cargo:rustc-link-lib=dylib=stdc++");
            },
            "macos" => {
                println!("cargo:rustc-link-search=target/mupdf_wrapper/Darwin");
                println!("cargo:rustc-link-search=target/collatinus_wrapper/Darwin");
                println!("cargo:rustc-link-lib=dylib=c++");
            },
            _ => panic!("Unsupported platform: {}.", target_os),
        }
        println!("cargo:rustc-link-lib=mupdf-third");
    }

    println!("cargo:rustc-link-lib=collatinus_wrapper");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=bz2");
    println!("cargo:rustc-link-lib=jpeg");
    println!("cargo:rustc-link-lib=png16");
    println!("cargo:rustc-link-lib=gumbo");
    println!("cargo:rustc-link-lib=openjp2");
    println!("cargo:rustc-link-lib=jbig2dec");
}
```

- [ ] **Step 2: Verify build.rs compiles**

```bash
cargo build -p plato-core 2>&1 | head -20
```

Expected: linker may complain about missing `libcollatinus_wrapper` if the lib isn't built yet — that's fine at this stage. The Rust compile step itself should succeed.

- [ ] **Step 3: Commit**

```bash
git add crates/core/build.rs
git commit -m "build: link libcollatinus_wrapper in build.rs"
```

---

## Task 5: Create Rust FFI bindings

**Files:**
- Create: `crates/core/src/dictionary/collatinus_sys.rs`

- [ ] **Step 1: Create the FFI module**

Create `crates/core/src/dictionary/collatinus_sys.rs`:

```rust
//! Raw FFI bindings to the Collatinus C wrapper.

use std::os::raw::c_char;

extern "C" {
    /// Look up a Latin word, returning a heap-allocated HTML string.
    /// Returns null on failure. Free with `collatinus_free_result`.
    pub fn collatinus_lookup(word: *const c_char, lang: *const c_char) -> *mut c_char;

    /// Free a string returned by `collatinus_lookup`.
    pub fn collatinus_free_result(result: *mut c_char);
}
```

- [ ] **Step 2: Declare the module in `dictionary/mod.rs`**

In `crates/core/src/dictionary/mod.rs`, add at the top with the other `mod` declarations:

```rust
mod collatinus_sys;
pub mod collatinus;
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build -p plato-core 2>&1 | head -20
```

Expected: compiles (the `collatinus` module doesn't exist yet — add an empty `pub mod collatinus {}` temporarily if needed).

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/dictionary/collatinus_sys.rs crates/core/src/dictionary/mod.rs
git commit -m "feat: add Collatinus FFI bindings (collatinus_sys)"
```

---

## Task 6: Create safe Rust wrapper and implement Dictionary trait

**Files:**
- Create: `crates/core/src/dictionary/collatinus.rs`

- [ ] **Step 1: Write a compile-time test**

Before writing the implementation, write a test that verifies `CollatinusDictionary` implements `Dictionary`. Add to `crates/core/src/dictionary/collatinus.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Dictionary;

    fn _assert_impl_dictionary(_: &dyn Dictionary) {}

    #[test]
    fn collatinus_dictionary_implements_trait() {
        let d = CollatinusDictionary::new("fr");
        _assert_impl_dictionary(&d);
    }
}
```

This will fail to compile until `CollatinusDictionary` exists and implements `Dictionary`.

- [ ] **Step 2: Implement `CollatinusDictionary`**

Write the full `crates/core/src/dictionary/collatinus.rs`:

```rust
use std::ffi::{CString, CStr};
use std::os::raw::c_char;
use super::collatinus_sys;
use super::Dictionary;
use super::errors::DictError;

pub struct CollatinusDictionary {
    lang: CString,
    lang_str: String,
}

impl CollatinusDictionary {
    pub fn new(lang: &str) -> Self {
        CollatinusDictionary {
            lang: CString::new(lang).expect("lang must not contain null bytes"),
            lang_str: lang.to_string(),
        }
    }

    pub fn lang(&self) -> &str {
        &self.lang_str
    }
}

impl Dictionary for CollatinusDictionary {
    fn lookup(&mut self, word: &str, _fuzzy: bool) -> Result<Vec<[String; 2]>, DictError> {
        let c_word = match CString::new(word) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        let raw = unsafe {
            collatinus_sys::collatinus_lookup(c_word.as_ptr(), self.lang.as_ptr())
        };
        if raw.is_null() {
            return Ok(vec![]);
        }
        let html = unsafe {
            let s = CStr::from_ptr(raw as *const c_char)
                .to_string_lossy()
                .into_owned();
            collatinus_sys::collatinus_free_result(raw);
            s
        };
        Ok(vec![[word.to_string(), html]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Dictionary;

    fn _assert_impl_dictionary(_: &dyn Dictionary) {}

    #[test]
    fn collatinus_dictionary_implements_trait() {
        let d = CollatinusDictionary::new("fr");
        _assert_impl_dictionary(&d);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p plato-core dictionary 2>&1
```

Expected: all tests pass including `collatinus_dictionary_implements_trait`.

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/dictionary/collatinus.rs crates/core/src/dictionary/mod.rs
git commit -m "feat: implement CollatinusDictionary with Dictionary trait"
```

---

## Task 7: Register virtual dictionaries at startup

**Files:**
- Modify: `crates/core/src/context.rs`
- Modify: `crates/plato/src/app.rs`

- [ ] **Step 1: Research supported languages from Collatinus source**

From Task 2 Step 4 you noted the language identifiers. Confirm the correct string values to pass as `lang` to `collatinus_lookup`. Define the list as a constant in `context.rs`.

For example (replace with actual values found in the source):

```rust
const COLLATINUS_LANGUAGES: &[&str] = &["fr", "en", "de", "it", "es", "pt", "ca"];
```

- [ ] **Step 2: Add `load_collatinus_dictionaries()` to `Context`**

In `crates/core/src/context.rs`, add the import:

```rust
use crate::dictionary::collatinus::CollatinusDictionary;
```

Add the constant after the existing `DICTIONARIES_DIRNAME` constant:

```rust
/// ISO 639-1 codes for languages supported by Collatinus.
/// Derived from thirdparty/collatinus source — update if Collatinus adds languages.
const COLLATINUS_LANGUAGES: &[&str] = &["fr", "en"];  // TODO: complete list from source
```

Add the method to the `impl Context` block:

```rust
pub fn load_collatinus_dictionaries(&mut self) {
    for lang in COLLATINUS_LANGUAGES {
        let name = format!("Collatinus (la\u{2192}{})", lang);
        self.dictionaries.insert(name, Box::new(CollatinusDictionary::new(lang)));
    }
}
```

- [ ] **Step 3: Call it at startup in `app.rs`**

In `crates/plato/src/app.rs`, find the line:

```rust
context.load_dictionaries();
```

Add the Collatinus call immediately after:

```rust
context.load_dictionaries();
context.load_collatinus_dictionaries();
```

- [ ] **Step 4: Build the full project**

```bash
cargo build -p plato 2>&1 | head -40
```

Expected: compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/context.rs crates/plato/src/app.rs
git commit -m "feat: register Collatinus virtual dictionaries at startup"
```

---

## Task 8: Full integration build and smoke test

- [ ] **Step 1: Build the wrapper**

```bash
cd collatinus_wrapper
bash build.sh
cd ..
```

Expected: `target/collatinus_wrapper/$(uname -s)/libcollatinus_wrapper.a` present.

- [ ] **Step 2: Build the full project**

```bash
cargo build -p plato 2>&1
```

Expected: no errors.

- [ ] **Step 3: Run all tests**

```bash
cargo test -p plato-core 2>&1
```

Expected: all tests pass.

- [ ] **Step 4: Verify data file access**

Collatinus needs its lexicon/morphological data files at runtime. Confirm where the built Collatinus engine looks for them (check the path it uses at init time — either a compile-time constant or a relative path). On Kobo, these files must be bundled alongside the Plato binary (same directory as `fonts/`, `dictionaries/`, etc.) and the path must resolve correctly. Verify this on desktop first, then test on a Kobo device or emulator.

If the path is hardcoded in Collatinus source, patch or configure it at build time to point to the Plato data directory.

- [ ] **Step 5: Smoke test on desktop (emulator)**

Run the emulator and open the dictionary panel. Verify:
- Dictionaries named `Collatinus (la→fr)`, `Collatinus (la→en)` etc. appear in the dictionary list
- Looking up an inflected Latin word (e.g. *amaverunt*) returns HTML output from Collatinus
- The HTML renders correctly in the dictionary popup (no raw tags shown)
- Non-Latin lookups are unaffected

- [ ] **Step 6: Final commit**

```bash
git add -p  # stage any remaining changes
git commit -m "feat: Collatinus Latin dictionary integration complete"
```
