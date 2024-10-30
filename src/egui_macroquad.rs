#![allow(static_mut_refs)]
#![allow(dead_code)]
// vendored from https://github.com/optozorax/egui-macroquad/tree/dfbdb967d6cf4e4726b84a568ec1b2bdc7e4f492

use egui_miniquad::EguiMq;
use macroquad::prelude::*;
use miniquad as mq;

pub use macroquad;

struct Egui {
    egui_mq: EguiMq,
    input_subscriber_id: usize,
}

// Global variable and global functions because it's more like macroquad way
static mut EGUI: Option<Egui> = None;
static mut EGUI_CONTEXT: Option<egui::Context> = None;

fn get_egui() -> &'static mut Egui {
    unsafe {
        if let Some(egui) = EGUI.as_mut() {
            egui
        } else {
            EGUI = Some(Egui::new());
            EGUI.as_mut().unwrap()
        }
    }
}

pub fn egui() -> &'static egui::Context {
    unsafe {
        if let Some(ctx) = EGUI_CONTEXT.as_mut() {
            ctx
        } else {
            panic!("You need to call this in a context wrapped by ui()");
        }
    }
}

impl Egui {
    fn new() -> Self {
        Self {
            egui_mq: EguiMq::new(unsafe { get_internal_gl() }.quad_context),
            input_subscriber_id: macroquad::input::utils::register_input_subscriber(),
        }
    }

    fn ui<F>(&mut self, f: F)
    where
        F: FnOnce(&mut dyn mq::RenderingBackend, &egui::Context),
    {
        let gl = unsafe { get_internal_gl() };
        macroquad::input::utils::repeat_all_miniquad_input(self, self.input_subscriber_id);
        self.egui_mq.run(gl.quad_context, f);
    }

    fn draw(&mut self) {
        let mut gl = unsafe { get_internal_gl() };
        // Ensure that macroquad's shapes are not goint to be lost, and draw them now
        gl.flush();
        self.egui_mq.draw(gl.quad_context);
    }
}

/// Calculates egui ui. Must be called once per frame.
pub fn ui<F: FnOnce(&egui::Context)>(f: F) {
    get_egui().ui(|_, ctx| {
        unsafe { EGUI_CONTEXT = Some(ctx.clone()) };
        f(ctx);
        unsafe { EGUI_CONTEXT = None };
    })
}

/// Configure egui without beginning or ending a frame.
pub fn cfg<F: FnOnce(&egui::Context)>(f: F) {
    f(get_egui().egui_mq.egui_ctx());
}

/// Draw egui ui. Must be called after `ui` and once per frame.
pub fn draw() {
    get_egui().draw()
}

// Intended to be used only if you recreate the window, making the old EGUI instance invalid.
#[doc(hidden)]
pub fn reset_egui() {
    unsafe {
        EGUI = None;
    }
}

impl mq::EventHandler for Egui {
    fn update(&mut self) {}

    fn draw(&mut self) {}

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.egui_mq.mouse_wheel_event(dx, dy);
    }

    fn mouse_button_down_event(&mut self, mb: mq::MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_down_event(mb, x, y);
    }

    fn mouse_button_up_event(&mut self, mb: mq::MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_up_event(mb, x, y);
    }

    fn char_event(&mut self, character: char, _keymods: mq::KeyMods, _repeat: bool) {
        self.egui_mq.char_event(character);
    }

    fn key_down_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods, _repeat: bool) {
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }
}
