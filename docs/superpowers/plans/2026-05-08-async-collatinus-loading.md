# Async Collatinus Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Collatinus (Latin dictionary) initialization to a background thread at app startup so the definition panel never freezes the UI.

**Architecture:** A shared `Arc<AtomicBool>` tracks whether Collatinus is ready. A background thread calls `collatinus_init()` at startup and sets the flag when done, then sends `Event::CollatinusReady` through the hub. `CollatinusDictionary::lookup()` returns a loading-placeholder HTML when the flag is false. When the event arrives, the Reader refreshes any open `DefinitionPanel` by re-running the query.

**Tech Stack:** Rust, `std::sync::atomic`, `std::thread`, existing Plato event hub (`Sender<Event>`)

---

## File Map

| File | Change |
|------|--------|
| `crates/core/src/view/mod.rs` | Add `CollatinusReady` variant to `Event` enum |
| `crates/core/src/context.rs` | Add `collatinus_ready: Arc<AtomicBool>` field; update `load_collatinus_dictionaries` |
| `crates/core/src/lib.rs` | Make `dictionary` module `pub` so app.rs can access `collatinus::preload` |
| `crates/core/src/dictionary/collatinus.rs` | Add `ready: Arc<AtomicBool>` to struct; guard `lookup()`; add `pub fn preload` |
| `crates/plato/src/app.rs` | Spawn background init thread after channel creation |
| `crates/core/src/view/reader/definition_panel.rs` | Add `query: String`, `language: String` fields; add `refresh()` method |
| `crates/core/src/view/reader/mod.rs` | Handle `Event::CollatinusReady` |

---

## Task 1: Add `CollatinusReady` to Event and `collatinus_ready` to Context

**Files:**
- Modify: `crates/core/src/view/mod.rs`
- Modify: `crates/core/src/context.rs`

- [ ] **Step 1: Add `CollatinusReady` to the Event enum**

Open `crates/core/src/view/mod.rs`. Find the `WakeUp,` line (currently the last variant before the closing brace of `Event`). Add after it:

```rust
    WakeUp,
    CollatinusReady,
```

- [ ] **Step 2: Add imports to context.rs**

Open `crates/core/src/context.rs`. The existing imports start with `use crate::view::keyboard::Layout;`. Add after the last `use` line at the top:

```rust
use std::sync::{Arc, atomic::AtomicBool};
```

- [ ] **Step 3: Add `collatinus_ready` field to the Context struct**

In `crates/core/src/context.rs`, find the `pub struct Context` block. Add the new field after `pub online: bool`:

```rust
    pub online: bool,
    pub collatinus_ready: Arc<AtomicBool>,
```

- [ ] **Step 4: Initialize `collatinus_ready` in `Context::new`**

Find the `Context { fb, rtc, display: Display { dims, rotation }, ...` struct literal in `Context::new`. Add the new field at the end, before the closing `}`:

```rust
        Context { fb, rtc, display: Display { dims, rotation },
                  library, settings, fonts, dictionaries: BTreeMap::new(),
                  keyboard_layouts: BTreeMap::new(), input_history: FxHashMap::default(),
                  battery, frontlight, lightsensor, notification_index: 0,
                  kb_rect: Rectangle::default(), rng, plugged: false, covered: false,
                  shared: false, online: false,
                  collatinus_ready: Arc::new(AtomicBool::new(false)) }
```

- [ ] **Step 5: Build to check for compile errors so far**

```bash
cargo build -p plato-core 2>&1 | grep "^error" | head -20
```

Expected: errors about `CollatinusDictionary::new` call in `load_collatinus_dictionaries` — that is expected and will be fixed in Task 2. No other errors expected.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/view/mod.rs crates/core/src/context.rs
git commit -m "feat: add CollatinusReady event and collatinus_ready flag to Context"
```

---

## Task 2: Update `CollatinusDictionary` and expose `preload` function

**Files:**
- Modify: `crates/core/src/dictionary/collatinus.rs`
- Modify: `crates/core/src/context.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Open `crates/core/src/dictionary/collatinus.rs`. Find the `#[cfg(test)]` block at the bottom. Replace it with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    use crate::dictionary::Dictionary;

    fn _assert_impl_dictionary(_: &dyn Dictionary) {}

    #[test]
    fn collatinus_dictionary_implements_trait() {
        let ready = Arc::new(AtomicBool::new(false));
        let d = CollatinusDictionary::new("fr", Arc::clone(&ready));
        _assert_impl_dictionary(&d);
    }

    #[test]
    fn lookup_returns_loading_placeholder_when_not_ready() {
        let ready = Arc::new(AtomicBool::new(false));
        let mut d = CollatinusDictionary::new("fr", Arc::clone(&ready));
        let result = d.lookup("amor", false).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0][1].contains("loading"));
    }

}
```

- [ ] **Step 2: Run the tests to confirm they fail (compile error expected)**

```bash
cargo test -p plato-core -- dictionary::collatinus 2>&1 | grep "^error" | head -10
```

Expected: compile error because `CollatinusDictionary::new` signature doesn't match yet.

- [ ] **Step 3: Rewrite `CollatinusDictionary` with the ready flag**

Replace the entire content of `crates/core/src/dictionary/collatinus.rs` (keep the tests block you just wrote at the bottom):

```rust
use std::ffi::{CString, CStr};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use super::collatinus_sys;
use super::Dictionary;
use super::DictError;

pub struct CollatinusDictionary {
    lang: CString,
    ready: Arc<AtomicBool>,
}

impl CollatinusDictionary {
    pub fn new(lang: &str, ready: Arc<AtomicBool>) -> Self {
        CollatinusDictionary {
            lang: CString::new(lang).expect("lang must not contain null bytes"),
            ready,
        }
    }
}

/// Call this from a background thread to initialize Collatinus.
/// Sets `ready` to true on success (leaves it false on failure).
pub fn preload(lang: &str, ready: Arc<AtomicBool>) {
    let c_lang = match CString::new(lang) {
        Ok(s) => s,
        Err(_) => return,
    };
    let rc = unsafe { collatinus_sys::collatinus_init(c_lang.as_ptr()) };
    if rc == 0 {
        ready.store(true, Ordering::Release);
    }
}

impl Dictionary for CollatinusDictionary {
    fn lookup(&mut self, word: &str, _fuzzy: bool) -> Result<Vec<[String; 2]>, DictError> {
        if !self.ready.load(Ordering::Acquire) {
            return Ok(vec![
                ["".to_string(),
                 "<p class=\"info\">Latin dictionary is loading\u{2026}</p>".to_string()],
            ]);
        }
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
            let s = CStr::from_ptr(raw).to_string_lossy().into_owned();
            collatinus_sys::collatinus_free_result(raw);
            s
        };
        Ok(vec![[word.to_string(), html]])
    }
}
```

Note: `collatinus_init()` is no longer called inside `lookup()`. The background thread (`preload`) is the only caller.

- [ ] **Step 4: Update `load_collatinus_dictionaries` in context.rs**

Find the `load_collatinus_dictionaries` method in `crates/core/src/context.rs` and replace it:

```rust
    pub fn load_collatinus_dictionaries(&mut self) {
        let lang = self.settings.dictionary.collatinus_target.clone();
        let name = format!("Collatinus (la→{})", lang);
        self.dictionaries.insert(name, Box::new(
            CollatinusDictionary::new(&lang, Arc::clone(&self.collatinus_ready))
        ));
    }
```

- [ ] **Step 5: Make `dictionary` module public in lib.rs**

Open `crates/core/src/lib.rs`. Find `mod dictionary;` and change it to:

```rust
pub mod dictionary;
```

This lets `crates/plato/src/app.rs` import `plato_core::dictionary::collatinus::preload`.

- [ ] **Step 6: Run the tests**

```bash
cargo test -p plato-core -- dictionary::collatinus 2>&1 | tail -20
```

Expected output:
```
test dictionary::collatinus::tests::collatinus_dictionary_implements_trait ... ok
test dictionary::collatinus::tests::lookup_returns_loading_placeholder_when_not_ready ... ok
```

- [ ] **Step 7: Full build check**

```bash
cargo build -p plato-core 2>&1 | grep "^error" | head -20
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add crates/core/src/dictionary/collatinus.rs crates/core/src/context.rs crates/core/src/lib.rs
git commit -m "feat: CollatinusDictionary checks ready flag, returns loading placeholder; expose preload fn"
```

---

## Task 3: Spawn background Collatinus init thread at startup

**Files:**
- Modify: `crates/plato/src/app.rs`

- [ ] **Step 1: Add import for collatinus preload**

Open `crates/plato/src/app.rs`. Find the block of `use plato_core::...` imports. Add:

```rust
use plato_core::dictionary::collatinus::preload as collatinus_preload;
use std::sync::Arc;
```

- [ ] **Step 2: Spawn the init thread after the channel is created**

Find the block starting at `let (tx, rx) = mpsc::channel();` (around line 259). After the last existing `thread::spawn` block (the `auto_suspend` one, around line 299), add:

```rust
    {
        let lang = context.settings.dictionary.collatinus_target.clone();
        let ready = Arc::clone(&context.collatinus_ready);
        let tx_collatinus = tx.clone();
        thread::spawn(move || {
            collatinus_preload(&lang, ready);
            tx_collatinus.send(Event::CollatinusReady).ok();
        });
    }
```

- [ ] **Step 3: Build check**

```bash
cargo build -p plato 2>&1 | grep "^error" | head -20
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/plato/src/app.rs
git commit -m "feat: spawn background thread to preload Collatinus at app startup"
```

---

## Task 4: Add `query`/`language` fields and `refresh()` to DefinitionPanel

**Files:**
- Modify: `crates/core/src/view/reader/definition_panel.rs`

- [ ] **Step 1: Add `query` and `language` fields to the struct**

Find the `pub struct DefinitionPanel` definition. Add two fields after `pub target: Option<String>`:

```rust
pub struct DefinitionPanel {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    doc: HtmlDocument,
    page_locations: Vec<usize>,
    current_page: usize,
    pub target: Option<String>,
    query: String,
    language: String,
}
```

- [ ] **Step 2: Store `query` and `language` in the constructor**

Find the `DefinitionPanel { id, rect, children, doc, page_locations, current_page: 0, target: target.map(String::from), }` struct literal at the end of `DefinitionPanel::new`. Replace it with:

```rust
        DefinitionPanel {
            id,
            rect,
            children,
            doc,
            page_locations,
            current_page: 0,
            target: target.map(String::from),
            query: query.to_string(),
            language: language.to_string(),
        }
```

- [ ] **Step 3: Add the `refresh()` method**

In the `impl DefinitionPanel` block, add `refresh` after the existing `go_to_page` method:

```rust
    pub fn refresh(&mut self, rq: &mut RenderQueue, context: &mut Context) {
        let content = query_to_content(&self.query, &self.language, false,
                                       self.target.as_ref(), context);
        self.doc.update(&content);
        self.page_locations = collect_page_locations(&mut self.doc);
        self.current_page = 0;

        let pixmap = self.doc.pixmap(Location::Exact(0), 1.0, CURRENT_DEVICE.color_samples())
                             .map(|(pm, _)| pm)
                             .unwrap_or_else(|| Pixmap::new(1, 1, 1));
        if let Some(image) = self.children[1].downcast_mut::<Image>() {
            image.update(pixmap, rq);
        }
        if let Some(sb) = self.children[2].downcast_mut::<ScrollBar>() {
            sb.update(0, self.page_locations.len(), rq);
        }
    }
```

- [ ] **Step 4: Build check**

```bash
cargo build -p plato-core 2>&1 | grep "^error" | head -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/view/reader/definition_panel.rs
git commit -m "feat: DefinitionPanel stores query/language and exposes refresh() for post-init update"
```

---

## Task 5: Handle `Event::CollatinusReady` in the Reader

**Files:**
- Modify: `crates/core/src/view/reader/mod.rs`

- [ ] **Step 1: Add the event handler**

Open `crates/core/src/view/reader/mod.rs`. Find the `_ => false,` line near the end of the Reader's `handle_event` match (around line 4044). Insert the new arm just before it:

```rust
            Event::CollatinusReady => {
                if let Some(index) = locate_by_id(self, ViewId::DefinitionPanel) {
                    let panel_rect = *self.child(index).rect();
                    if let Some(panel) = self.child_mut(index).downcast_mut::<DefinitionPanel>() {
                        panel.refresh(rq, context);
                        rq.add(RenderData::new(panel.id(), panel_rect, UpdateMode::Gui));
                    }
                }
                true
            },
            _ => false,
```

- [ ] **Step 2: Build check**

```bash
cargo build -p plato-core 2>&1 | grep "^error" | head -20
```

Expected: no errors.

- [ ] **Step 3: Full build**

```bash
cargo build -p plato 2>&1 | grep "^error" | head -20
```

Expected: no errors.

- [ ] **Step 4: Run all tests**

```bash
cargo test -p plato-core 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/view/reader/mod.rs
git commit -m "feat: handle CollatinusReady in Reader — refresh definition panel when init completes"
```

---

## Manual Verification

With a Kobo device or emulator:

1. Open Plato and navigate to a Latin book.
2. Quickly select a word before Collatinus finishes loading — the definition panel should open immediately showing "Latin dictionary is loading…" instead of freezing.
3. Wait a moment — the panel should automatically update with the real morphological analysis.
4. Close and reopen the panel after loading is complete — results should appear instantly with no delay.
