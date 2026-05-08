use crate::device::CURRENT_DEVICE;
use crate::unit::scale_by_dpi;
use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::gesture::GestureEvent;
use crate::geom::Rectangle;
use crate::color::{BLACK, WHITE, TEXT_NORMAL, PROGRESS_EMPTY, PROGRESS_FULL};
use crate::font::Fonts;
use crate::context::Context;
use super::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData, SMALL_BAR_HEIGHT, THICKNESS_MEDIUM};
use super::icon::ICONS_PIXMAPS;

const INDICATOR_MIN_HEIGHT: f32 = 20.0;

pub struct ScrollBar {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    pub current_page: usize,
    pub total_pages: usize,
}

impl ScrollBar {
    pub fn new(rect: Rectangle, current_page: usize, total_pages: usize) -> ScrollBar {
        ScrollBar {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            current_page,
            total_pages,
        }
    }

    pub fn update(&mut self, current_page: usize, total_pages: usize, rq: &mut RenderQueue) {
        if self.current_page != current_page || self.total_pages != total_pages {
            self.current_page = current_page;
            self.total_pages = total_pages;
            rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
        }
    }

    fn button_size(&self) -> i32 {
        let dpi = CURRENT_DEVICE.dpi;
        scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32
    }

    fn up_rect(&self) -> Rectangle {
        let btn = self.button_size();
        rect![self.rect.min.x, self.rect.min.y, self.rect.max.x, self.rect.min.y + btn]
    }

    fn down_rect(&self) -> Rectangle {
        let btn = self.button_size();
        rect![self.rect.min.x, self.rect.max.y - btn, self.rect.max.x, self.rect.max.y]
    }

    fn indicator_rect(&self) -> Rectangle {
        let dpi = CURRENT_DEVICE.dpi;
        let total = self.total_pages.max(1);
        let up_r = self.up_rect();
        let dn_r = self.down_rect();
        let track_top = up_r.max.y;
        let track_bottom = dn_r.min.y;
        let track_height = (track_bottom - track_top).max(1);
        let min_h = scale_by_dpi(INDICATOR_MIN_HEIGHT, dpi) as i32;
        let indicator_height = (track_height / total as i32).max(min_h).min(track_height);
        let usable = (track_height - indicator_height).max(1);
        let y_offset = if total > 1 {
            (usable * self.current_page as i32) / (total as i32 - 1)
        } else {
            0
        };
        rect![
            self.rect.min.x,
            track_top + y_offset,
            self.rect.max.x,
            track_top + y_offset + indicator_height
        ]
    }
}

impl View for ScrollBar {
    fn handle_event(&mut self, evt: &Event, _hub: &Hub, bus: &mut Bus, rq: &mut RenderQueue, _context: &mut Context) -> bool {
        match *evt {
            Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => {
                if self.up_rect().includes(center) && self.current_page > 0 {
                    let page = self.current_page - 1;
                    self.current_page = page;
                    rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                    bus.push_back(Event::Scroll(page as i32));
                } else if self.down_rect().includes(center) && self.current_page + 1 < self.total_pages {
                    let page = self.current_page + 1;
                    self.current_page = page;
                    rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                    bus.push_back(Event::Scroll(page as i32));
                }
                true
            },
            _ => false,
        }
    }

    fn render(&self, fb: &mut dyn Framebuffer, _rect: Rectangle, _fonts: &mut Fonts) {
        let dpi = CURRENT_DEVICE.dpi;
        let thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as i32;

        fb.draw_rectangle(&self.rect, PROGRESS_EMPTY);

        let up_r = self.up_rect();
        let dn_r = self.down_rect();

        // Separators between buttons and track
        fb.draw_rectangle(&rect![self.rect.min.x, up_r.max.y - thickness,
                                  self.rect.max.x, up_r.max.y], BLACK);
        fb.draw_rectangle(&rect![self.rect.min.x, dn_r.min.y,
                                  self.rect.max.x, dn_r.min.y + thickness], BLACK);

        // Up button background
        fb.draw_rectangle(&up_r, WHITE);

        // Down button background
        fb.draw_rectangle(&dn_r, WHITE);

        // Up arrow icon
        if let Some(pixmap) = ICONS_PIXMAPS.get("angle-up") {
            let dx = (up_r.width() as i32 - pixmap.width as i32) / 2;
            let dy = (up_r.height() as i32 - pixmap.height as i32) / 2;
            fb.draw_blended_pixmap(pixmap, up_r.min + pt!(dx, dy), TEXT_NORMAL[1]);
        }

        // Down arrow icon
        if let Some(pixmap) = ICONS_PIXMAPS.get("angle-down") {
            let dx = (dn_r.width() as i32 - pixmap.width as i32) / 2;
            let dy = (dn_r.height() as i32 - pixmap.height as i32) / 2;
            fb.draw_blended_pixmap(pixmap, dn_r.min + pt!(dx, dy), TEXT_NORMAL[1]);
        }

        // Position indicator
        if self.total_pages > 1 {
            fb.draw_rectangle(&self.indicator_rect(), PROGRESS_FULL);
        }
    }

    fn rect(&self) -> &Rectangle { &self.rect }
    fn rect_mut(&mut self) -> &mut Rectangle { &mut self.rect }
    fn children(&self) -> &Vec<Box<dyn View>> { &self.children }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> { &mut self.children }
    fn id(&self) -> Id { self.id }
}
