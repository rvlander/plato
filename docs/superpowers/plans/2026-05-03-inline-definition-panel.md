# Inline Definition Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a word is long-pressed in the Plato reader, show an inline definition panel (33% screen height, auto-positioned top/bottom based on word position) alongside the selection menu, without navigating away from the book.

**Architecture:** A new `DefinitionPanel` child view is pushed into the Reader's `children` vec alongside `SelectionMenu`. It owns an `HtmlDocument` for rendering the definition and a `ScrollBar` for navigation. Both panel and menu are dismissed together via `Event::Close(ViewId::SelectionMenu)`.

**Tech Stack:** Rust, Plato's `View` trait, `HtmlDocument` renderer, existing `Menu`/`Label`/`Icon`/`Image`/`Filler` components.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `crates/core/src/view/mod.rs` | Add `ViewId`, `EntryId` variants; add `pub mod scroll_bar`; remove `DefineSelection` |
| Create | `crates/core/src/view/scroll_bar.rs` | Vertical draggable scrollbar, emits `Event::Scroll(page_index)` |
| Modify | `crates/core/src/view/dictionary/mod.rs` | Make `query_to_content` `pub(crate)` |
| Create | `crates/core/src/view/reader/definition_panel.rs` | `DefinitionPanel` view: HtmlDoc content + scrollbar + toolbar |
| Modify | `crates/core/src/view/reader/mod.rs` | Wire panel into Reader: show/dismiss, new event handlers, remove Define menu entry |

---

## Task 1: Add new identifiers, remove DefineSelection

**Files:**
- Modify: `crates/core/src/view/mod.rs`

- [ ] **Step 1: Add `pub mod scroll_bar` to the module list**

In `crates/core/src/view/mod.rs`, after `pub mod slider;` (line 19), add:

```rust
pub mod scroll_bar;
```

- [ ] **Step 2: Add new ViewId variants**

In the `pub enum ViewId` block (starting at line 382), after `SelectionMenu,` add:

```rust
    DefinitionPanel,
    DefinitionDictPicker,
```

- [ ] **Step 3: Add new EntryId variants and remove DefineSelection**

In the `pub enum EntryId` block (starting at line 508):

Remove this line (line 541):
```rust
    DefineSelection,
```

After `AdjustSelection,` (line 543) add:
```rust
    OpenDictionaryFromPanel,
    SwitchDictionary(String),
```

- [ ] **Step 4: Compile-check**

```bash
cd /Users/rvlander/CloudStation/WIP/plato && cargo check -p plato-core 2>&1 | grep "^error"
```

Expected: errors about `DefineSelection` being missing in reader/mod.rs — that's fine, they'll be fixed in Task 5. Any other errors must be investigated.

- [ ] **Step 5: Commit**

```bash
cd /Users/rvlander/CloudStation/WIP/plato
git add crates/core/src/view/mod.rs
git commit -m "feat: add DefinitionPanel ViewIds and EntryIds, remove DefineSelection"
```

---

## Task 2: Implement `ScrollBar`

**Files:**
- Create: `crates/core/src/view/scroll_bar.rs`

- [ ] **Step 1: Create the file**

Create `crates/core/src/view/scroll_bar.rs` with this content:

```rust
use crate::device::CURRENT_DEVICE;
use crate::unit::scale_by_dpi;
use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::input::{DeviceEvent, FingerStatus};
use crate::geom::Rectangle;
use crate::color::{BLACK, PROGRESS_EMPTY, PROGRESS_FULL};
use crate::font::Fonts;
use crate::context::Context;
use super::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData};

const THUMB_MIN_HEIGHT: f32 = 20.0;

pub struct ScrollBar {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    pub current_page: usize,
    pub total_pages: usize,
    active: bool,
    last_y: i32,
}

impl ScrollBar {
    pub fn new(rect: Rectangle, current_page: usize, total_pages: usize) -> ScrollBar {
        ScrollBar {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            current_page,
            total_pages,
            active: false,
            last_y: -1,
        }
    }

    pub fn update(&mut self, current_page: usize, total_pages: usize, rq: &mut RenderQueue) {
        if self.current_page != current_page || self.total_pages != total_pages {
            self.current_page = current_page;
            self.total_pages = total_pages;
            rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
        }
    }

    fn thumb_rect(&self) -> Rectangle {
        let dpi = CURRENT_DEVICE.dpi;
        let total = self.total_pages.max(1);
        let track_height = self.rect.height() as i32;
        let min_thumb = scale_by_dpi(THUMB_MIN_HEIGHT, dpi) as i32;
        let thumb_height = (track_height / total as i32).max(min_thumb).min(track_height);
        let usable = (track_height - thumb_height).max(1);
        let y_offset = if total > 1 {
            (usable * self.current_page as i32) / (total as i32 - 1)
        } else {
            0
        };
        rect![
            self.rect.min.x,
            self.rect.min.y + y_offset,
            self.rect.max.x,
            self.rect.min.y + y_offset + thumb_height
        ]
    }

    fn page_from_y(&self, y: i32) -> usize {
        let dpi = CURRENT_DEVICE.dpi;
        let total = self.total_pages.max(1);
        let track_height = self.rect.height() as i32;
        let min_thumb = scale_by_dpi(THUMB_MIN_HEIGHT, dpi) as i32;
        let thumb_height = (track_height / total as i32).max(min_thumb).min(track_height);
        let usable = (track_height - thumb_height).max(1);
        let y_rel = (y - self.rect.min.y).clamp(0, usable);
        if total <= 1 {
            return 0;
        }
        ((y_rel * (total as i32 - 1)) / usable).clamp(0, total as i32 - 1) as usize
    }
}

impl View for ScrollBar {
    fn handle_event(&mut self, evt: &Event, _hub: &Hub, bus: &mut Bus, rq: &mut RenderQueue, _context: &mut Context) -> bool {
        match *evt {
            Event::Device(DeviceEvent::Finger { status, position, .. }) => {
                match status {
                    FingerStatus::Down if self.rect.includes(position) => {
                        self.active = true;
                        self.last_y = position.y;
                        let page = self.page_from_y(position.y);
                        if page != self.current_page {
                            self.current_page = page;
                            rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                            bus.push_back(Event::Scroll(page as i32));
                        }
                        true
                    },
                    FingerStatus::Motion if self.active && position.y != self.last_y => {
                        self.last_y = position.y;
                        let page = self.page_from_y(position.y);
                        if page != self.current_page {
                            self.current_page = page;
                            rq.add(RenderData::no_wait(self.id, self.rect, UpdateMode::FastMono));
                            bus.push_back(Event::Scroll(page as i32));
                        }
                        true
                    },
                    FingerStatus::Up if self.active => {
                        self.active = false;
                        rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                        true
                    },
                    _ => self.active,
                }
            },
            _ => false,
        }
    }

    fn render(&self, fb: &mut dyn Framebuffer, _rect: Rectangle, _fonts: &mut Fonts) {
        fb.draw_rectangle(&self.rect, PROGRESS_EMPTY);
        if self.total_pages > 1 {
            let thumb = self.thumb_rect();
            fb.draw_rectangle(&thumb, if self.active { BLACK } else { PROGRESS_FULL });
        }
    }

    fn rect(&self) -> &Rectangle { &self.rect }
    fn rect_mut(&mut self) -> &mut Rectangle { &mut self.rect }
    fn children(&self) -> &Vec<Box<dyn View>> { &self.children }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> { &mut self.children }
    fn id(&self) -> Id { self.id }
}
```

- [ ] **Step 2: Compile-check**

```bash
cd /Users/rvlander/CloudStation/WIP/plato && cargo check -p plato-core 2>&1 | grep "^error"
```

Expected: no new errors from scroll_bar.rs.

- [ ] **Step 3: Commit**

```bash
cd /Users/rvlander/CloudStation/WIP/plato
git add crates/core/src/view/scroll_bar.rs
git commit -m "feat: add ScrollBar view for definition panel"
```

---

## Task 3: Make `query_to_content` pub(crate)

**Files:**
- Modify: `crates/core/src/view/dictionary/mod.rs`

- [ ] **Step 1: Change visibility**

In `crates/core/src/view/dictionary/mod.rs` at line 45, change:

```rust
fn query_to_content(query: &str, language: &String, fuzzy: bool, target: Option<&String>, context: &mut Context) -> String {
```

to:

```rust
pub(crate) fn query_to_content(query: &str, language: &String, fuzzy: bool, target: Option<&String>, context: &mut Context) -> String {
```

- [ ] **Step 2: Compile-check**

```bash
cd /Users/rvlander/CloudStation/WIP/plato && cargo check -p plato-core 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd /Users/rvlander/CloudStation/WIP/plato
git add crates/core/src/view/dictionary/mod.rs
git commit -m "feat: expose query_to_content as pub(crate) for definition panel"
```

---

## Task 4: Implement `DefinitionPanel`

**Files:**
- Create: `crates/core/src/view/reader/definition_panel.rs`

The panel has these children (indices shown for downcast access):
- `[0]` `Filler` — top separator (BLACK, `thickness` px tall, full width)
- `[1]` `Image` — definition content (full width minus scrollbar)
- `[2]` `ScrollBar` — right edge of content area
- `[3]` `Filler` — bottom separator (BLACK, `thickness` px tall, full width)
- `[4]` `Label` — dictionary picker (fires `Event::ToggleNear(ViewId::DefinitionDictPicker, rect)`)
- `[5]` `Icon` — "open in dictionary" button (fires `Event::Select(EntryId::OpenDictionaryFromPanel)`)

- [ ] **Step 1: Create the file**

Create `crates/core/src/view/reader/definition_panel.rs`:

```rust
use crate::device::CURRENT_DEVICE;
use crate::unit::scale_by_dpi;
use crate::framebuffer::{Framebuffer, UpdateMode, Pixmap};
use crate::input::DeviceEvent;
use crate::gesture::GestureEvent;
use crate::view::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData};
use crate::view::{ViewId, EntryId, SMALL_BAR_HEIGHT, THICKNESS_MEDIUM, Align};
use crate::document::{Location, Document};
use crate::document::html::HtmlDocument;
use crate::geom::{Rectangle, halves};
use crate::color::{BLACK, WHITE};
use crate::font::Fonts;
use crate::context::Context;
use crate::view::filler::Filler;
use crate::view::image::Image;
use crate::view::icon::Icon;
use crate::view::label::Label;
use crate::view::scroll_bar::ScrollBar;
use crate::view::dictionary::query_to_content;

const VIEWER_STYLESHEET: &str = "css/dictionary.css";
const USER_STYLESHEET: &str = "css/dictionary-user.css";
const SCROLLBAR_WIDTH: f32 = 14.0;

pub struct DefinitionPanel {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    doc: HtmlDocument,
    page_locations: Vec<usize>,
    current_page: usize,
    pub target: Option<String>,
}

fn collect_page_locations(doc: &mut HtmlDocument) -> Vec<usize> {
    let mut locations = vec![0usize];
    let mut loc = 0usize;
    for _ in 0..50 {
        match doc.resolve_location(Location::Next(loc)) {
            Some(next_loc) => {
                locations.push(next_loc);
                loc = next_loc;
            }
            None => break,
        }
    }
    locations
}

impl DefinitionPanel {
    pub fn new(rect: Rectangle, query: &str, language: &str, target: Option<&str>,
               rq: &mut RenderQueue, context: &mut Context) -> DefinitionPanel {
        let id = ID_FEEDER.next();
        let mut children: Vec<Box<dyn View>> = Vec::new();
        let dpi = CURRENT_DEVICE.dpi;
        let small_height = scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32;
        let thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as i32;
        let scrollbar_width = scale_by_dpi(SCROLLBAR_WIDTH, dpi) as i32;

        // [0] Top separator
        let top_sep = Filler::new(
            rect![rect.min.x, rect.min.y, rect.max.x, rect.min.y + thickness],
            BLACK);
        children.push(Box::new(top_sep) as Box<dyn View>);

        // Compute sub-rects
        let content_top = rect.min.y + thickness;
        let content_bottom = rect.max.y - small_height - thickness;
        let content_rect = rect![rect.min.x, content_top, rect.max.x - scrollbar_width, content_bottom];
        let scrollbar_rect = rect![rect.max.x - scrollbar_width, content_top, rect.max.x, content_bottom];

        // Set up HtmlDocument
        let mut doc = HtmlDocument::new_from_memory("");
        doc.layout(content_rect.width(), content_rect.height(),
                   context.settings.dictionary.font_size, dpi);
        doc.set_margin_width(context.settings.dictionary.margin_width);
        doc.set_viewer_stylesheet(VIEWER_STYLESHEET);
        doc.set_user_stylesheet(USER_STYLESHEET);

        // Render content
        let language_string = language.to_string();
        let target_string = target.map(|t| t.to_string());
        let content = query_to_content(query, &language_string, false, target_string.as_ref(), context);
        doc.update(&content);

        // Collect page locations for scrollbar
        let page_locations = collect_page_locations(&mut doc);
        let total_pages = page_locations.len();

        // [1] Image (initial render at page 0)
        let initial_pixmap = doc.pixmap(Location::Exact(0), 1.0, CURRENT_DEVICE.color_samples())
                                .map(|(pm, _)| pm)
                                .unwrap_or_else(|| Pixmap::new(1, 1, 1));
        let image = Image::new(content_rect, initial_pixmap);
        children.push(Box::new(image) as Box<dyn View>);

        // [2] ScrollBar
        let scrollbar = ScrollBar::new(scrollbar_rect, 0, total_pages);
        children.push(Box::new(scrollbar) as Box<dyn View>);

        // [3] Bottom separator
        let bot_sep = Filler::new(
            rect![rect.min.x, content_bottom, rect.max.x, content_bottom + thickness],
            BLACK);
        children.push(Box::new(bot_sep) as Box<dyn View>);

        // Toolbar
        let toolbar_top = content_bottom + thickness;
        let toolbar_side = small_height;
        let picker_rect = rect![rect.min.x, toolbar_top, rect.max.x - toolbar_side, rect.max.y];
        let open_rect = rect![rect.max.x - toolbar_side, toolbar_top, rect.max.x, rect.max.y];

        // [4] Dict picker label
        let target_name = target.unwrap_or("All").to_string();
        let picker_label = Label::new(picker_rect, target_name, Align::Center)
            .event(Some(Event::ToggleNear(ViewId::DefinitionDictPicker, picker_rect)));
        children.push(Box::new(picker_label) as Box<dyn View>);

        // [5] Open-in-dictionary icon
        let open_icon = Icon::new("search", open_rect,
                                  Event::Select(EntryId::OpenDictionaryFromPanel));
        children.push(Box::new(open_icon) as Box<dyn View>);

        rq.add(RenderData::new(id, rect, UpdateMode::Gui));

        DefinitionPanel {
            id,
            rect,
            children,
            doc,
            page_locations,
            current_page: 0,
            target: target.map(String::from),
        }
    }

    pub fn go_to_page(&mut self, page: usize, rq: &mut RenderQueue) {
        if page >= self.page_locations.len() || page == self.current_page {
            return;
        }
        self.current_page = page;
        let loc = self.page_locations[page];
        if let Some((pixmap, _)) = self.doc.pixmap(Location::Exact(loc), 1.0, CURRENT_DEVICE.color_samples()) {
            if let Some(image) = self.children[1].downcast_mut::<Image>() {
                image.update(pixmap, rq);
            }
        }
        if let Some(sb) = self.children[2].downcast_mut::<ScrollBar>() {
            sb.update(page, self.page_locations.len(), rq);
        }
    }
}

impl View for DefinitionPanel {
    fn handle_event(&mut self, evt: &Event, _hub: &Hub, _bus: &mut Bus, rq: &mut RenderQueue, _context: &mut Context) -> bool {
        match *evt {
            Event::Scroll(page) if page >= 0 => {
                self.go_to_page(page as usize, rq);
                true
            },
            // Absorb all touch/gesture events within the panel to prevent the Reader from handling them
            Event::Device(DeviceEvent::Finger { position, .. }) if self.rect.includes(position) => true,
            Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => true,
            Event::Gesture(GestureEvent::Swipe { start, .. }) if self.rect.includes(start) => true,
            Event::Gesture(GestureEvent::HoldFingerShort(center, ..)) if self.rect.includes(center) => true,
            _ => false,
        }
    }

    fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, _fonts: &mut Fonts) {
        if let Some(r) = self.rect.intersection(&rect) {
            fb.draw_rectangle(&r, WHITE);
        }
    }

    fn view_id(&self) -> Option<ViewId> {
        Some(ViewId::DefinitionPanel)
    }

    fn rect(&self) -> &Rectangle { &self.rect }
    fn rect_mut(&mut self) -> &mut Rectangle { &mut self.rect }
    fn children(&self) -> &Vec<Box<dyn View>> { &self.children }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> { &mut self.children }
    fn id(&self) -> Id { self.id }
}
```

- [ ] **Step 2: Compile-check**

```bash
cd /Users/rvlander/CloudStation/WIP/plato && cargo check -p plato-core 2>&1 | grep "^error"
```

Expected: errors about `definition_panel` module not declared in reader/mod.rs — that's fine. No other errors.

- [ ] **Step 3: Commit**

```bash
cd /Users/rvlander/CloudStation/WIP/plato
git add crates/core/src/view/reader/definition_panel.rs
git commit -m "feat: add DefinitionPanel view"
```

---

## Task 5: Wire up the Reader

**Files:**
- Modify: `crates/core/src/view/reader/mod.rs`

This task has many small changes. Make them in order and compile-check at the end.

- [ ] **Step 1: Declare the submodule and add imports**

At the top of `crates/core/src/view/reader/mod.rs`, after `mod results_label;` (line 6), add:

```rust
mod definition_panel;
```

After the existing `use self::results_bar::ResultsBar;` import, add:

```rust
use self::definition_panel::DefinitionPanel;
```

- [ ] **Step 2: Add `toggle_definition_panel` and `definition_panel_rect` methods**

In the `impl Reader` block, add these two methods after `toggle_selection_menu` (after line 1801):

```rust
fn definition_panel_rect(&self) -> Rectangle {
    let panel_height = self.rect.height() as i32 / 3;
    let sel_mid_y = self.selection_rect()
                       .map(|r| r.min.y + r.height() as i32 / 2)
                       .unwrap_or(self.rect.min.y + self.rect.height() as i32 / 2);
    let reader_mid_y = self.rect.min.y + self.rect.height() as i32 / 2;
    if sel_mid_y < reader_mid_y {
        rect![self.rect.min.x, self.rect.max.y - panel_height,
              self.rect.max.x, self.rect.max.y]
    } else {
        rect![self.rect.min.x, self.rect.min.y,
              self.rect.max.x, self.rect.min.y + panel_height]
    }
}

pub fn toggle_definition_panel(&mut self, enable: Option<bool>, rq: &mut RenderQueue, context: &mut Context) {
    if let Some(index) = locate_by_id(self, ViewId::DefinitionPanel) {
        if enable == Some(true) {
            return;
        }
        rq.add(RenderData::expose(*self.child(index).rect(), UpdateMode::Gui));
        self.children.remove(index);
    } else {
        if enable == Some(false) {
            return;
        }
        if let Some(text) = self.selected_text() {
            let query = text.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if query.is_empty() {
                return;
            }
            let language = self.info.language.clone();
            let panel_rect = self.definition_panel_rect();
            let panel = DefinitionPanel::new(panel_rect, &query, &language, None, rq, context);
            rq.add(RenderData::new(panel.id(), *panel.rect(), UpdateMode::Gui));
            self.children.push(Box::new(panel) as Box<dyn View>);
        }
    }
}
```

- [ ] **Step 3: Show panel when finger lifts (initial selection)**

At line 2963, the existing code is:

```rust
self.toggle_selection_menu(Rectangle::from_disk(position, radius), Some(true), rq, context);
```

Add the panel call immediately after it:

```rust
self.toggle_selection_menu(Rectangle::from_disk(position, radius), Some(true), rq, context);
self.toggle_definition_panel(Some(true), rq, context);
```

- [ ] **Step 4: Show panel when adjust-selection finalizes**

At line 3284, the existing code is:

```rust
self.toggle_selection_menu(Rectangle::from_disk(center, radius), Some(true), rq, context);
```

Add the panel call immediately after it:

```rust
self.toggle_selection_menu(Rectangle::from_disk(center, radius), Some(true), rq, context);
self.toggle_definition_panel(Some(true), rq, context);
```

- [ ] **Step 5: Dismiss panel when SelectionMenu closes**

At line 3570, the existing handler is:

```rust
Event::Close(ViewId::SelectionMenu) => {
    if self.state == State::Idle && self.target_annotation.is_none() {
        if let Some(rect) = self.selection_rect() {
            self.selection = None;
            rq.add(RenderData::new(self.id, rect, UpdateMode::Gui));
        }
    }
    false
},
```

Replace it with:

```rust
Event::Close(ViewId::SelectionMenu) => {
    self.toggle_definition_panel(Some(false), rq, context);
    if self.state == State::Idle && self.target_annotation.is_none() {
        if let Some(rect) = self.selection_rect() {
            self.selection = None;
            rq.add(RenderData::new(self.id, rect, UpdateMode::Gui));
        }
    }
    false
},
```

- [ ] **Step 6: Remove "Define" from the selection menu**

In `toggle_selection_menu` (around line 1786), remove these two lines:

```rust
entries.push(EntryKind::Separator);
entries.push(EntryKind::Command("Define".to_string(), EntryId::DefineSelection));
```

The separator before Search stays but now groups only Search below:

After removal the block should read:
```rust
entries.push(EntryKind::Separator);
entries.push(EntryKind::Command("Search".to_string(), EntryId::SearchForSelection));
```

(The `Separator` before `Search` is kept, but check if it still makes sense — if it's the only separator-separated item, it may look odd. You may remove the separator too if Search is now the first item after the top group. Use your judgment when you see the entries list in context.)

- [ ] **Step 7: Remove the `DefineSelection` event handler**

Find and remove the entire block (around lines 3718–3726):

```rust
Event::Select(EntryId::DefineSelection) => {
    if let Some(text) = self.selected_text() {
        let query = text.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        let language = self.info.language.clone();
        hub.send(Event::Select(EntryId::Launch(AppCmd::Dictionary { query, language }))).ok();
    }
    self.selection = None;
    true
},
```

- [ ] **Step 8: Add handler for `OpenDictionaryFromPanel`**

In the event match block, after the `SearchForSelection` handler (around line 3745), add:

```rust
Event::Select(EntryId::OpenDictionaryFromPanel) => {
    if let Some(text) = self.selected_text() {
        let query = text.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        let language = self.info.language.clone();
        hub.send(Event::Select(EntryId::Launch(AppCmd::Dictionary { query, language }))).ok();
    }
    self.selection = None;
    true
},
```

- [ ] **Step 9: Add handler for `ToggleNear(ViewId::DefinitionDictPicker, rect)`**

After the `ToggleNear(ViewId::SelectionMenu, ..)` handler (search for existing ToggleNear handlers in the reader), add:

```rust
Event::ToggleNear(ViewId::DefinitionDictPicker, rect) => {
    if let Some(index) = locate_by_id(self, ViewId::DefinitionDictPicker) {
        rq.add(RenderData::expose(*self.child(index).rect(), UpdateMode::Gui));
        self.children.remove(index);
    } else {
        let current_target = locate_by_id(self, ViewId::DefinitionPanel)
            .and_then(|idx| self.child(idx).downcast_ref::<DefinitionPanel>())
            .and_then(|panel| panel.target.clone());
        let mut entries: Vec<EntryKind> = context.dictionaries.keys()
            .map(|k| EntryKind::RadioButton(
                k.to_string(),
                EntryId::SwitchDictionary(k.to_string()),
                current_target.as_deref() == Some(k.as_str())))
            .collect();
        if !entries.is_empty() {
            entries.push(EntryKind::Separator);
        }
        entries.push(EntryKind::RadioButton(
            "All".to_string(),
            EntryId::SwitchDictionary(String::new()),
            current_target.is_none()));
        let menu = Menu::new(rect, ViewId::DefinitionDictPicker, MenuKind::DropDown, entries, context);
        rq.add(RenderData::new(menu.id(), *menu.rect(), UpdateMode::Gui));
        self.children.push(Box::new(menu) as Box<dyn View>);
    }
    true
},
```

- [ ] **Step 10: Add handler for `SwitchDictionary`**

After the `OpenDictionaryFromPanel` handler, add:

```rust
Event::Select(EntryId::SwitchDictionary(ref name)) => {
    if let Some(index) = locate_by_id(self, ViewId::DefinitionPanel) {
        rq.add(RenderData::expose(*self.child(index).rect(), UpdateMode::Gui));
        self.children.remove(index);
    }
    if let Some(text) = self.selected_text() {
        let query = text.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        let language = self.info.language.clone();
        let target = if name.is_empty() { None } else { Some(name.as_str()) };
        let panel_rect = self.definition_panel_rect();
        let panel = DefinitionPanel::new(panel_rect, &query, &language, target, rq, context);
        rq.add(RenderData::new(panel.id(), *panel.rect(), UpdateMode::Gui));
        self.children.push(Box::new(panel) as Box<dyn View>);
    }
    true
},
```

- [ ] **Step 11: Final compile-check**

```bash
cd /Users/rvlander/CloudStation/WIP/plato && cargo check -p plato-core 2>&1 | grep "^error"
```

Expected: no errors.

Also check the emulator target:

```bash
cd /Users/rvlander/CloudStation/WIP/plato && cargo check -p plato 2>&1 | grep "^error"
```

- [ ] **Step 12: Commit**

```bash
cd /Users/rvlander/CloudStation/WIP/plato
git add crates/core/src/view/reader/mod.rs
git commit -m "feat: wire DefinitionPanel into Reader — show on selection, dismiss with menu, dict picker, switch dict"
```

---

## Self-Review Checklist

- [x] **Panel shows automatically on word selection** — handled in `FingerStatus::Up` (step 3) and adjust-selection finalizer (step 4)
- [x] **Panel positioned top/bottom based on word location** — `definition_panel_rect()` computes this
- [x] **Fixed 33% height** — `self.rect.height() as i32 / 3`
- [x] **Single best-match dictionary** — `query_to_content` with `target = None` uses existing language-matching logic
- [x] **Dictionary picker dropdown** — `ToggleNear(ViewId::DefinitionDictPicker)` handler creates a `Menu`
- [x] **Switch dictionary** — `SwitchDictionary(name)` handler re-creates panel with new target
- [x] **"Open in Dictionary" button** — `OpenDictionaryFromPanel` handler launches `AppCmd::Dictionary`
- [x] **"Define" removed from selection menu** — step 6
- [x] **Panel dismisses with selection menu** — `Close(ViewId::SelectionMenu)` handler calls `toggle_definition_panel(Some(false))` (step 5)
- [x] **Scrollbar** — `ScrollBar` view with `go_to_page` in `DefinitionPanel`
- [x] **"No definitions found" on empty lookup** — handled by existing `query_to_content` behaviour (returns that string)
- [x] **No swipe-to-scroll** — scrollbar only, swipe events within panel are absorbed
