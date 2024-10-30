pub use macroquad::math::ivec2;
pub use macroquad::math::IVec2;
pub use macroquad::math::Vec2;
use macroquad::prelude::*;

pub fn egui() -> &'static egui::Context {
    crate::egui_macroquad::egui()
}

pub fn delta() -> f32 {
    macroquad::time::get_frame_time()
}
