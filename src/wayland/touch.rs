use smithay_client_toolkit::seat::touch::TouchHandler;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::wl_touch::WlTouch;
use wayland_client::{Connection, QueueHandle};

use crate::Button;
use crate::input::{PIXELS_PER_LINE, Scroll};

use super::WaylandState;

pub struct ActiveTouch {
    pub surface: WlSurface,
    pub start_pos: (f64, f64),
    pub last_pos: (f64, f64),
    pub is_scrolling: bool,
}

impl TouchHandler for WaylandState {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        surface: WlSurface,
        id: i32,
        position: (f64, f64),
    ) {
        let (x, y) = (position.0 as f32, position.1 as f32);

        if let Some(window) = self.window(&surface) {
            window.pointer.move_to(x, y);
            window.pointer.press();
            let _ = window.pointer.start_drag();
        }
        self.request_frame_on(&surface);

        self.active_touches.insert(
            id,
            ActiveTouch {
                surface,
                start_pos: position,
                last_pos: position,
                is_scrolling: false,
            },
        );
    }

    fn up(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        id: i32,
    ) {
        if let Some(touch) = self.active_touches.remove(&id) {
            if let Some(window) = self.window(&touch.surface) {
                if !touch.is_scrolling {
                    window.pointer.stop_drag();
                    window.pointer.release(Button::Left);
                } else {
                    window.pointer.stop_drag();
                    window.pointer.cancel_press();
                }
                window.pointer.leave();
            }
            self.request_frame_on(&touch.surface);
        }
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _time: u32,
        id: i32,
        position: (f64, f64),
    ) {
        let Some(touch_info) = self.active_touches.get(&id) else {
            return;
        };

        let surface = touch_info.surface.clone();
        let start_pos = touch_info.start_pos;
        let last_pos = touch_info.last_pos;
        let was_scrolling = touch_info.is_scrolling;

        let (x, y) = (position.0 as f32, position.1 as f32);
        let dx = position.0 - start_pos.0;
        let dy = position.1 - start_pos.1;
        let step_x = (position.0 - last_pos.0) as f32;
        let step_y = (position.1 - last_pos.1) as f32;

        let mut now_scrolling = was_scrolling;

        if let Some(window) = self.window(&surface) {
            if was_scrolling {
                // Finger moved while scrolling: pan scroll_area
                let scroll = Scroll {
                    x: -step_x / PIXELS_PER_LINE,
                    y: -step_y / PIXELS_PER_LINE,
                };
                window.pointer.scroll(scroll);
            } else if window.pointer.is_dragging() {
                // Active drag on a draggable widget (like slider knob)
                window.pointer.move_to(x, y);
                window.pointer.drag();
            } else {
                // If moved more than touch slop (8 pixels), transition to drag-scroll
                if dx.abs() > 8.0 || dy.abs() > 8.0 {
                    now_scrolling = true;
                    window.pointer.cancel_press();
                    window.pointer.stop_drag();

                    let scroll = Scroll {
                        x: -step_x / PIXELS_PER_LINE,
                        y: -step_y / PIXELS_PER_LINE,
                    };
                    window.pointer.scroll(scroll);
                } else {
                    window.pointer.move_to(x, y);
                }
            }
        }

        self.request_frame_on(&surface);

        if let Some(touch) = self.active_touches.get_mut(&id) {
            touch.last_pos = position;
            touch.is_scrolling = now_scrolling;
        }
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }

    fn cancel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _touch: &WlTouch) {
        let touches: Vec<_> = std::mem::take(&mut self.active_touches).into_values().collect();
        for touch in touches {
            if let Some(window) = self.window(&touch.surface) {
                window.pointer.stop_drag();
                window.pointer.cancel_press();
                window.pointer.leave();
            }
            self.request_frame_on(&touch.surface);
        }
    }
}
