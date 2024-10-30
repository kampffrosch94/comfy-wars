/*
// Game Loop
pub use comfy::delta;
pub use comfy::simple_game;
pub use comfy::EngineContext;
pub use comfy::EngineState;
pub use comfy::GameLoop;

// Math
pub use comfy::ivec2;
pub use comfy::vec2;
pub use comfy::world_to_screen;
pub use comfy::IRect;
pub use comfy::IVec2;
pub use comfy::Vec2;
pub use comfy::Vec2EngineExtensions;

// Color Constants
pub use macroquad::color::GRAY;
pub use macroquad::color::WHITE;
//pub use macroquad::color::TEAL;

// Drawing
pub use comfy::clear_background;
pub use comfy::commands;
pub use comfy::draw_rect;
pub use comfy::draw_sprite_ex;
pub use comfy::draw_text;
pub use comfy::main_camera_mut;
pub use comfy::texture_id;
pub use comfy::Color;
pub use comfy::DrawTextureParams;
pub use comfy::Sprite;
pub use comfy::TextAlign;
pub use comfy::Transform;

// Input
pub use comfy::is_key_pressed;
pub use comfy::is_mouse_button_pressed;
pub use comfy::is_mouse_button_released;
pub use comfy::mouse_world;
pub use comfy::KeyCode;
pub use comfy::MouseButton;

// Egui
pub use comfy::egui;
pub use comfy::epaint;

*/

pub use macroquad::math::ivec2;
pub use macroquad::math::IVec2;
pub use macroquad::math::Vec2;

// TODO
pub fn mouse_world() -> Vec2 {
    Vec2 { x: 0., y: 0. }
}

// TODO
pub fn world_to_screen(pos: Vec2) -> Vec2 {
    Vec2 { x: 0., y: 0. }
}

// TODO
pub fn egui() -> &'static egui::Context {
    crate::egui_macroquad::egui()
}

pub fn delta() -> f32 {
    macroquad::time::get_frame_time()
}
