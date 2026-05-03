# Inline Definition Panel — Design Spec

**Date:** 2026-05-03
**Branch:** sane-build-environment
**Status:** Approved

## Goal

Replace Plato's current "Define → full-screen Dictionary app" flow with a Kobo-style inline definition panel that appears automatically when the user selects a word, without leaving the reader.

## Current Behaviour

1. Long-press a word → word is selected → contextual `SelectionMenu` appears (Highlight, Add Note, Define, Search, Adjust Selection)
2. Tap "Define" → fires `AppCmd::Dictionary` → entire screen replaced by full-screen Dictionary app
3. User presses Back to return to the book

## Target Behaviour

1. Long-press a word → word is selected → `SelectionMenu` appears (Highlight, Add Note, Search, Adjust Selection) **and simultaneously** a `DefinitionPanel` appears
2. Panel shows the definition inline — no navigation, no full-screen takeover
3. Panel dismisses together with the selection menu (tap elsewhere, highlight, etc.)

## Components

### New file: `crates/core/src/view/reader/definition_panel.rs`

A `DefinitionPanel` struct following the same child-view pattern as other Reader overlays.

Owns:
- A small `HtmlDocument` (same renderer as the Dictionary app) for definition content
- A bottom toolbar with two controls:
  - Dictionary picker button → opens a `Menu` dropdown listing available dictionaries
  - "Open Dictionary" button → launches the full Dictionary app

### New identifiers (in `crates/core/src/view/mod.rs`)

| Identifier | Kind | Purpose |
|---|---|---|
| `ViewId::DefinitionPanel` | ViewId | Locate/toggle the panel in the child tree |
| `ViewId::DefinitionDictPicker` | ViewId | Locate the picker button within the panel |
| `EntryId::SwitchDictionary(String)` | EntryId | Pick a dictionary from the dropdown |
| `EntryId::OpenDictionaryFromPanel` | EntryId | Open full Dictionary app from the panel |

`EntryId::DefineSelection` is removed (no longer used).

## Panel Layout

Fixed height: **33% of screen height**.

Position depends on where the selected word sits (determined by `selection_rect()` midpoint Y vs. reader rect vertical midpoint):
- Word in **top half** → panel anchors to the **bottom** of the screen
- Word in **bottom half** → panel anchors to the **top** of the screen

```
┌──────────────────────────────────┬──┐
│  separator (1px black line)      │  │
├──────────────────────────────────┤  │
│  HtmlDocument content area       │▓▓│ ← scrollbar (right edge)
│                                  │  │
├──────────────────────────────────┤  │
│  separator (1px black line)      │  │
├──────────────────────────────────┴──┤
│  [Dict Name ▼]    [Open Dictionary] │  ← toolbar (SMALL_BAR_HEIGHT)
└─────────────────────────────────────┘
```

The content area supports vertical scrolling via a **scrollbar on the right edge of the panel** when the definition overflows the available height. The scrollbar is a thin draggable track — tapping or dragging it updates the `HtmlDocument` location (page offset) and re-renders the content area. No swipe gestures.

There is no existing scrollbar component in Plato — a minimal `ScrollBar` view must be implemented as part of this feature (new file: `crates/core/src/view/scroll_bar.rs`). It only needs to handle vertical drag and tap, and report `Event::Scroll(delta)` onto the bus.

Uses existing constants: `SMALL_BAR_HEIGHT`, `THICKNESS_MEDIUM`.
Uses existing components: `RoundedButton` / `LabeledIcon`.
Uses existing CSS: `css/dictionary.css`.

## Data Flow

### On word selection (finger up)

The `FingerStatus::Up` handler in `reader/mod.rs` currently calls `toggle_selection_menu(...)`. We add immediately after:

```
toggle_definition_panel(query, language, Some(true), rq, context)
```

This:
1. Extracts the selected text and trims non-alphanumeric chars (same as current `DefineSelection` path)
2. Picks the best-match dictionary using the existing language-matching logic from `query_to_content`
3. Renders the HTML definition into the panel's `HtmlDocument`
4. Determines panel position (top/bottom) from selection rect midpoint
5. Pushes `DefinitionPanel` into `self.children`

### On selection dismiss

Every path that dismisses the selection (tap elsewhere, Highlight, Add Note, Search, Adjust Selection, Close SelectionMenu) also calls `toggle_definition_panel(..., Some(false), ...)`. Panel and menu always die together.

### Dictionary switching

Tap picker → `Event::ToggleNear(ViewId::DefinitionDictPicker, rect)` → `Menu` lists all dictionaries → user picks → `Event::Select(EntryId::SwitchDictionary(name))` → Reader calls `toggle_definition_panel` with the new target dictionary, replacing the panel.

### Open in Dictionary

Tap "Open Dictionary" → `Event::Select(EntryId::OpenDictionaryFromPanel)` → Reader fires `hub.send(Event::Select(EntryId::Launch(AppCmd::Dictionary { query, language })))` — identical to the old `DefineSelection` path.

## Changes to Existing Files

### `crates/core/src/view/mod.rs`
- Add `ViewId::DefinitionPanel`, `ViewId::DefinitionDictPicker`
- Add `EntryId::SwitchDictionary(String)`, `EntryId::OpenDictionaryFromPanel`
- Remove `EntryId::DefineSelection`

### `crates/core/src/view/reader/mod.rs`
- Add `mod definition_panel;` and import `DefinitionPanel`
- Add `toggle_definition_panel(query, language, target, enable, rq, context)` method
- Call it alongside `toggle_selection_menu` in `FingerStatus::Up` handler
- Call `toggle_definition_panel(..., Some(false), ...)` in all selection-dismiss paths
- Remove `"Define"` entry from `toggle_selection_menu`
- Remove `Event::Select(EntryId::DefineSelection)` handler
- Add `Event::Select(EntryId::SwitchDictionary(name))` handler
- Add `Event::Select(EntryId::OpenDictionaryFromPanel)` handler

## Error Handling

If no dictionary matches or lookup returns empty, the panel still appears and shows "No definitions found." — same message as the full Dictionary app. This avoids a jarring absent-panel experience on unknown words.

## Out of Scope

- Animations or transitions
- Any change to the standalone Dictionary app
