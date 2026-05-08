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
