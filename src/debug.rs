use comfy::*;

static DEBUG_LINES: Lazy<AtomicRefCell<Vec<String>>> =
    Lazy::new(|| AtomicRefCell::new(Default::default()));

pub fn cw_debug(s: impl Into<String>) {
    DEBUG_LINES.borrow_mut().push(s.into());
}

pub fn cw_draw_debug_window() {
    let mut lines = DEBUG_LINES.borrow_mut();
    if lines.len() > 0 {
        egui::Window::new("Adhoc Debug Window").show(egui(), |ui| {
            for line in lines.drain(..) {
                ui.label(line);
            }
        });
    }
}

macro_rules! cw_debug {
    ($($arg:tt)*) => {
        cw_debug(format!($($arg)*));
    };
}
