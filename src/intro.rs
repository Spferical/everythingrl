use ::rand::seq::{index, SliceRandom};
use macroquad::prelude::*;

pub const CHARS_PER_SECOND: f32 = 35.;

pub const SETTINGS: [&str; 7] = [
    "Richard Adams's Watership Down",
    "Frank Herbert's Dune",
    "Pirates, just lots of pirates",
    "Ridley Scott's 1979 Alien movie",
    "Microcenter",
    "Star Wars but everyone is a cat",
    "Lord of the Rings but everyone is a cat"
];

pub const TIPS: [&str; 3] = [
    "think carefully about the type of your weapon, armor and the enemy type before you attack. Some monsters are resistant or completely immune to certain damage types.",
    "most food that you'll find in the world is not particularly nutritious unless cooked.",
    "narrow corridors are your friend. Try luring enemies into a narrow chokepoint to benefit from Lanchester's linear law :)"
];

pub const PROMPTS: [&str; 20] = [
    "Welcome, traveler. I am the Storyteller of this roguelike game.",
    "My objective is to create the world of the game which you are about to play. From the inhabitants of this virtual dungeon, to their implements and attire, to their demeanor and persuasion, to the very earth they step foot upon....",
    "... they will be invented by yours truly. With a bit of help from you of course.",
    "As you might have guessed by this point, the game you are about to play includes AI-generated elements. Despite the implemented safety features, it is entirely possible for the underlying system to produce inaccurate or offensive content. Click \"I understand\" if you understand these risks and wish to continue, otherwise click Exit to exit the game.",
    "Very well. Please describe the setting of the game which you would like to play. It can be literally anything. For example, you could say \"{setting1}\" or \"{setting2}\" to generate fantasy/sci-fi worlds in those settings.",
    "Good. It'll take around 60 seconds to generate your prompt. In the meantime, let's discuss the controls and user interface of the game.",
    "Either 'hjkl' or arrow keys can be used for movement through the level. Press the '.' key to wait a turn.",
    "Press ',' to pick up an item if you are standing on top of it. Note that you are limited to 10 inventory items at a time.",
    "Press 'i' to toggle the inventory overlay. Here you can see the weapons, armor, and consumables that you have picked up as well as their associated types, level, and condition.",
    "The number keys 0-9 can be used to select items in your inventory. Pressing the key again will toggle that selection.",
    "The 'e' key can be used to equip selected items, and 'd' to drop items. The currently equipped weapon(s) and armor will appear first in this list.",
    "'e' also stands for '(e)at' -- eating consumables will increase your health.",
    "You can have at most one melee weapon, one ranged weapon equipped at a time, and two articles of armor equipped at a time.",
    "Press the c key to combine/craft selected items. Crafting takes a short amount of (real-world) time, and will combine both items to create a completely new item with different properties.",
    "'c' also stands for '(c)ook. Cooking consumable items will improve their nutritional quality.",
    "'/' or ';' can be used to describe all items that are currently selected.",
    "The upper right panel shows some information about all monsters that you can see. Click the \"Details\" header to learn more about a given monster.",
    "If the fonts are rendering too small or large, there is a font scale slider on the bottom left.",
    "A quick tip -- {tip}",
    "Thank you for listening to me. Please wait a moment as the game world is generated.",
];

pub struct IntroState {
    step: usize,
    dt: f32,
    pub exit: bool,
    pub theme: String,
    pub ready_for_generation: bool,
    chosen_tip: String,
    chosen_settings: Vec<String>,
}

impl IntroState {
    pub fn new() -> IntroState {
        IntroState {
            step: 0,
            dt: 0.,
            exit: false,
            theme: String::new(),
            ready_for_generation: false,
            chosen_tip: (*TIPS.choose(&mut ::rand::thread_rng()).unwrap()).into(),
            chosen_settings: index::sample(&mut ::rand::thread_rng(), SETTINGS.len(), 2)
                .iter()
                .map(|i| (*SETTINGS[i]).into())
                .collect(),
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
    let width = screen_width() * miniquad::window::dpi_scale();
    egui::Window::new("StoryTeller")
        .resizable(false)
        .collapsible(false)
        .min_width(width / 2.0)
        .max_width(width)
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
                            egui::widgets::TextEdit::singleline(&mut intro_state.theme)
                                .desired_width(width * 0.8),
                        );
                    }
                    if ui.button("OK").clicked() {
                        if edit_text_box && intro_state.theme.is_empty() {
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
                    if intro_state.theme.is_empty() {
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
            let mut prompt = PROMPTS[state.step].to_owned();
            if prompt.contains("{tip}") {
                prompt = prompt.replace("{tip}", &state.chosen_tip);
            }
            if prompt.contains("{setting1}") || prompt.contains("{setting2}") {
                prompt = prompt.replace("{setting1}", &state.chosen_settings[0]);
                prompt = prompt.replace("{setting2}", &state.chosen_settings[1]);
            }
            create_info_prompt(egui_ctx, state, &prompt, state.step == 3, state.step == 4);
        } else {
            continuing = false;
        }
    });
    if state.step >= 5 {
        state.ready_for_generation = true;
    }

    egui_macroquad::draw();
    continuing
}
