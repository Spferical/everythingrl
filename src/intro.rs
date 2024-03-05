use egui;
use macroquad::prelude::*;

pub const CHARS_PER_SECOND: f32 = 25.;

pub const PROMPTS: [&str; 6] = [
    "Welcome, traveler. I am the Storyteller of this roguelike game.",
    "My objective is to create the world of the game which you are about to play. From the inhabitants of this virtual dungeon, to their implements and attire, to their demeanor and persuasion, to the very earth they step foot upon....",
    "... they will be invented by yours truly. With a bit of help from you of course.",
    "As you might have guessed by this point, the game you are about to play includes AI-generated elements. Despite the implemented safety features, it is entirely possible for the underlying system to produce inaccurate or offensive content. Click \"I understand\" if you understand these risks and wish to continue, otherwise click Exit to exit the game.",
    "Very well. Please describe the setting of the game which you would like to
play. For example, you could say \"Richard Adams Watership Down\" or \"Frank Herbert Dune\" to generate fantasy/sci-fi worlds in those settings.",
    "Thank you for listening to me. Please wait a moment as the game world is generated.",
];

pub struct IntroState {
    step: usize,
    dt: f32,
    pub exit: bool,
    prompt: String,
}

impl IntroState {
    pub fn new() -> IntroState {
        IntroState {
            step: 0,
            dt: 0.,
            exit: false,
            prompt: String::new(),
        }
    }
}

pub fn create_info_prompt(
    egui_ctx: &egui::Context,
    intro_state: &mut IntroState,
    prompt: &str,
    yes_no: bool,
    edit_text_box: bool,
) {
    let num_typewritten_chars = (CHARS_PER_SECOND * intro_state.dt) as usize;
    let typewritten_prompt: String = prompt.chars().take(num_typewritten_chars).collect();
    let width = screen_width().min(screen_height()) * 2.0;
    egui::Window::new("StoryTeller")
        .resizable(false)
        .collapsible(false)
        .min_width(width)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
        .show(egui_ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(typewritten_prompt));
                ui.separator();
                if yes_no {
                    if ui.button("I understand").clicked() {
                        intro_state.step += 1;
                    }
                    if ui.button("Exit").clicked() {
                        intro_state.exit = true;
                    }
                } else {
                    ui.label(
                        egui::RichText::new("(Press Enter to continue)")
                            .small()
                            .color(egui::Color32::from_rgb(100, 100, 100)),
                    );

                    if edit_text_box {
                        ui.add(
                            egui::widgets::TextEdit::singleline(&mut intro_state.prompt)
                                .desired_width(width * 0.8),
                        );
                    }
                    if ui.button("OK").clicked() {
                        if edit_text_box && intro_state.prompt.len() == 0 {
                            return;
                        }
                        intro_state.step += 1;
                        intro_state.dt = 0.;
                    }
                }
            });
        });

    if let Some(key) = get_last_key_pressed() {
        if key == KeyCode::Enter {
            if !yes_no {
                if edit_text_box {
                    // Don't continue if nothing is written
                    if intro_state.prompt.len() == 0 {
                        return;
                    }
                }
                intro_state.step += 1;
                intro_state.dt = 0.;
            } else {
                intro_state.dt = 1000.;
            }
        }
    }
}

pub fn intro_loop(state: &mut IntroState) -> bool {
    let mut continuing = true;
    state.dt += get_frame_time();

    egui_macroquad::ui(|egui_ctx| {
        if state.step < PROMPTS.len() {
            create_info_prompt(
                egui_ctx,
                state,
                PROMPTS[state.step],
                state.step == 3,
                state.step == 4,
            );
        } else {
            continuing = false;
        }
    });

    egui_macroquad::draw();
    continuing
}
