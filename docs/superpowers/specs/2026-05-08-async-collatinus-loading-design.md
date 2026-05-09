# Async Collatinus Loading

**Date:** 2026-05-08
**Branch:** async-loading-of-collatinus

## Problem

Collatinus (the Latin morphological dictionary) initializes lazily via `std::call_once` in C++. The first time a user opens a definition panel for a Latin word, the UI freezes while `LemCore` and `Lemmatiseur` load their data files from disk. The goal is to move this work to a background thread at app startup so the panel never blocks.

## Approach

Background thread at app startup. The thread races ahead immediately; by the time the user navigates to a book and selects a word, init is almost certainly done. The definition panel has a loading state as a safety net for the rare case where init is still in progress.

## Design

### Shared ready state

`Context` gains a new field:

```rust
pub collatinus_ready: Arc<AtomicBool>  // starts false
```

`CollatinusDictionary` holds a clone of the same `Arc`. This lets the background thread and the dictionary struct observe the same flag without locks.

### Background thread (crates/plato/src/app.rs)

After `context.load_collatinus_dictionaries()`, clone the target language string from `context.settings.dictionary.collatinus_target` and clone the `hub` sender and `collatinus_ready` Arc, then spawn a `std::thread`:

1. Call `collatinus_sys::collatinus_init()` with the cloned language string.
2. Set `collatinus_ready` to `true` (or leave `false` on failure).
3. Send `Event::CollatinusReady` through the cloned hub.

The main startup thread is never blocked.

### Dictionary lookup during loading

`CollatinusDictionary::lookup()` checks the ready flag first:

- **Not ready:** return a placeholder result — HTML like `<p>Latin dictionary is loading…</p>` — without calling into C++.
- **Ready:** proceed normally (C++ `std::call_once` ensures init runs only once regardless).
- **Failed (flag stays false after event):** return `<p>Latin dictionary unavailable.</p>`.

### Panel refresh on completion

When `Event::CollatinusReady` arrives in the Reader's event loop:

- If a `DefinitionPanel` is currently open, call `refresh()` on it.
- `refresh()` re-runs `query_to_content()` with the stored query string and re-renders the first page.
- If no panel is open, the event is a no-op.

`DefinitionPanel` needs a new `query: String` field (the raw word being looked up) to support this.

### New event

Add `CollatinusReady` to the `Event` enum.

## Files to change

| File | Change |
|------|--------|
| `crates/core/src/context.rs` | Add `collatinus_ready: Arc<AtomicBool>` field |
| `crates/core/src/dictionary/collatinus.rs` | Add `ready: Arc<AtomicBool>`; guard `lookup()` |
| `crates/core/src/dictionary/mod.rs` | Pass `Arc` when constructing `CollatinusDictionary` |
| `crates/plato/src/app.rs` | Spawn background init thread after dict loading |
| `crates/core/src/event.rs` | Add `CollatinusReady` variant |
| `crates/core/src/view/reader/definition_panel.rs` | Add `query: String`; add `refresh()` method |
| `crates/core/src/view/reader/mod.rs` | Handle `Event::CollatinusReady` |

## Edge cases

- **Init fails:** thread sends `CollatinusReady` regardless; flag stays `false`; panel shows unavailable message.
- **Panel closed before init finishes:** `CollatinusReady` arrives, Reader finds no open panel, does nothing.
- **Panel opened, closed, then init finishes:** same as above — no-op.
- Only one `DefinitionPanel` can be open at a time, so no multi-panel complexity.
