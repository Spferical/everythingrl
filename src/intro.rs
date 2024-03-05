use egui;
use macroquad::prelude::*;

pub const CHARS_PER_SECOND: f32 = 30.;

pub struct IntroState {
    step: usize,
    dt: f32,
}

impl IntroState {
    pub fn new() -> IntroState {
        IntroState { step: 0, dt: 0. }
    }
}

pub fn create_info_prompt(egui_ctx: &egui::Context, intro_state: &mut IntroState, prompt: &str) {
    let num_typewritten_chars = (CHARS_PER_SECOND * intro_state.dt) as usize;
    let typewritten_prompt: String = prompt.chars().take(num_typewritten_chars).collect();
    egui::Window::new("Tutorial")
        .resizable(false)
        .collapsible(false)
        .min_width(screen_width() / 2.)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
        .show(egui_ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(typewritten_prompt));
                ui.separator();
                ui.label(
                    egui::RichText::new("(Press Enter to continue)")
                        .small()
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                );
                if ui.button("OK").clicked() {
                    intro_state.step += 1;
                }
            });
        });

    if let Some(key) = get_last_key_pressed() {
        if key == KeyCode::Enter {
            intro_state.step += 1;
            intro_state.dt = 0.;
        }
    }
}

pub fn intro_loop(state: &mut IntroState) -> bool {
    state.dt += get_frame_time();
    egui_macroquad::ui(|egui_ctx| match state.step {
        0 => create_info_prompt(egui_ctx, state, "Welcome to the game!"),
        _ => create_info_prompt(egui_ctx, state, "This is our game!"),
    });

    egui_macroquad::draw();
    true
}
