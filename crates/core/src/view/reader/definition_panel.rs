use crate::device::CURRENT_DEVICE;
use crate::unit::scale_by_dpi;
use crate::framebuffer::{Framebuffer, Pixmap};
use crate::input::DeviceEvent;
use crate::gesture::GestureEvent;
use crate::view::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue};
use crate::view::{ViewId, EntryId, SMALL_BAR_HEIGHT, THICKNESS_MEDIUM, Align};
use crate::document::{Location, Document};
use crate::document::html::HtmlDocument;
use crate::geom::Rectangle;
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
const CHILD_IMAGE: usize = 1;
const CHILD_SCROLLBAR: usize = 2;

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
               _rq: &mut RenderQueue, context: &mut Context) -> DefinitionPanel {
        let id = ID_FEEDER.next();
        let mut children: Vec<Box<dyn View>> = Vec::new();
        let dpi = CURRENT_DEVICE.dpi;
        let small_height = scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32;
        let thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as i32;
        let scrollbar_width = small_height;

        // [0] Top separator
        let top_sep = Filler::new(
            rect![rect.min.x, rect.min.y, rect.max.x, rect.min.y + thickness],
            BLACK);
        children.push(Box::new(top_sep) as Box<dyn View>);

        // Compute sub-rects
        // content_bottom leaves room for: bot_sep, toolbar, bottom_border
        let content_top = rect.min.y + thickness;
        let content_bottom = rect.max.y - small_height - 2 * thickness;
        let content_rect = rect![rect.min.x, content_top, rect.max.x - scrollbar_width, content_bottom];
        let scrollbar_rect = rect![rect.max.x - scrollbar_width, content_top, rect.max.x, content_bottom];

        // Set up HtmlDocument
        let mut doc = HtmlDocument::new_from_memory("");
        doc.layout(content_rect.width(), content_rect.height(),
                   context.settings.dictionary.font_size, dpi);
        doc.set_margin_width(context.settings.dictionary.margin_width);
        doc.set_viewer_stylesheet(VIEWER_STYLESHEET);
        doc.set_user_stylesheet(USER_STYLESHEET);

        // Render content — show a loading placeholder if Collatinus is still initializing
        // to avoid blocking the UI. refresh() is called once CollatinusReady fires.
        let language_string = language.to_string();
        let target_string = target.map(|t| t.to_string());
        let collatinus_ready = context.collatinus_ready.load(std::sync::atomic::Ordering::Acquire);
        let content = if collatinus_ready {
            query_to_content(query, &language_string, false, target_string.as_ref(), context)
        } else {
            "<p class=\"info\">Latin dictionary is loading\u{2026}</p>".to_string()
        };
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

        // Toolbar: sits between bot_sep and bottom_border
        let toolbar_top = content_bottom + thickness;
        let toolbar_bottom = rect.max.y - thickness;
        let toolbar_side = small_height;
        let vsep_x = rect.max.x - toolbar_side - thickness;

        // [4] Dict picker label
        let target_name = target.unwrap_or("All Dictionaries").to_string();
        let picker_rect = rect![rect.min.x, toolbar_top, vsep_x, toolbar_bottom];
        let picker_label = Label::new(picker_rect, target_name, Align::Center)
            .event(Some(Event::ToggleNear(ViewId::DefinitionDictPicker, picker_rect)));
        children.push(Box::new(picker_label) as Box<dyn View>);

        // [5] Vertical separator between picker and open button
        let vsep = Filler::new(
            rect![vsep_x, toolbar_top, vsep_x + thickness, toolbar_bottom],
            BLACK);
        children.push(Box::new(vsep) as Box<dyn View>);

        // [6] Open-in-dictionary icon
        let open_rect = rect![vsep_x + thickness, toolbar_top, rect.max.x, toolbar_bottom];
        let open_icon = Icon::new("search", open_rect,
                                  Event::Select(EntryId::OpenDictionaryFromPanel));
        children.push(Box::new(open_icon) as Box<dyn View>);

        // [7] Bottom border — separates panel from book content when panel is at top of screen
        let bottom_border = Filler::new(
            rect![rect.min.x, rect.max.y - thickness, rect.max.x, rect.max.y],
            BLACK);
        children.push(Box::new(bottom_border) as Box<dyn View>);

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
    }

    pub(super) fn go_to_page(&mut self, page: usize, rq: &mut RenderQueue) {
        if page >= self.page_locations.len() || page == self.current_page {
            return;
        }
        self.current_page = page;
        let loc = self.page_locations[page];
        if let Some((pixmap, _)) = self.doc.pixmap(Location::Exact(loc), 1.0, CURRENT_DEVICE.color_samples()) {
            if let Some(image) = self.children[CHILD_IMAGE].downcast_mut::<Image>() {
                image.update(pixmap, rq);
            }
        }
        if let Some(sb) = self.children[CHILD_SCROLLBAR].downcast_mut::<ScrollBar>() {
            sb.update(page, self.page_locations.len(), rq);
        }
    }

    pub(super) fn refresh(&mut self, rq: &mut RenderQueue, context: &mut Context) {
        let content = query_to_content(&self.query, &self.language, false,
                                       self.target.as_ref(), context);
        self.doc.update(&content);
        self.page_locations = collect_page_locations(&mut self.doc);
        self.current_page = 0;

        if let Some((pixmap, _)) = self.doc.pixmap(Location::Exact(0), 1.0, CURRENT_DEVICE.color_samples()) {
            if let Some(image) = self.children[CHILD_IMAGE].downcast_mut::<Image>() {
                image.update(pixmap, rq);
            }
        }
        if let Some(sb) = self.children[CHILD_SCROLLBAR].downcast_mut::<ScrollBar>() {
            sb.update(0, self.page_locations.len(), rq);
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
