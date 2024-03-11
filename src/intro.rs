use crate::net::IdeaGuy;
use ::rand::seq::{index, SliceRandom};
use macroquad::prelude::*;

pub const CHARS_PER_SECOND: f32 = 35.;
pub const CHARS_PER_SECOND_LOADING: f32 = 80.;

pub const SETTINGS: [&str; 9] = [
    "Richard Adams's Watership Down",
    "Frank Herbert's Dune",
    "Pirates, just lots of pirates",
    "Ridley Scott's 1979 Alien movie",
    "Microcenter",
    "Star Wars but everyone is a cat",
    "Lord of the Rings but everyone is a cat",
    "Bears. Just lots of bears.",
    "Nethack, but better somehow.",
];

pub const TIPS: [&str; 3] = [
    "think carefully about the type of your weapon, armor and the enemy type before you attack. Some monsters are resistant or completely immune to certain damage types.",
    "most food that you'll find in the world is not particularly nutritious unless cooked.",
    "narrow corridors are your friend. Try luring enemies into a narrow chokepoint to benefit from Lanchester's linear law :)"
];

pub const PROMPTS: [&str; 12] = [
    "Welcome, traveler. I am the Storyteller of this roguelike game.",
    "My objective is to create the world of the game which you are about to play. From the inhabitants of this virtual dungeon, to their implements and attire, to their demeanor and persuasion, to the very earth they step foot upon....",
    "... they will be invented by yours truly. With a bit of help from you of course.",
    "As you might have guessed by this point, the game you are about to play includes AI-generated elements. Despite the implemented safety features, it is entirely possible for the underlying system to produce inaccurate or offensive content. Click \"I understand\" if you understand these risks and wish to continue, otherwise click Exit to exit the game.",
    "Very well. Please describe the setting of the game which you would like to play. It can be literally anything. For example, you could say \"{setting1}\" or \"{setting2}\" to generate fantasy/sci-fi worlds in those settings.",
    "Good. It'll take around 60 seconds to generate your prompt. In the meantime, a couple small notes.",
    "The movement keys are hjkl/arrows.\nHold down shift and move to use your ranged weapon.\n\'i\' opens inventory\n\'.\' waits for a moment\n\',\' picks up an item\n\'0-9\' multi-selects inventory items\n\'e\' equips/eats an item.\n\'d\' drops selected items\n\'c\' combines/cooks items\n\';\' or \'/\' will inspect an item.",
    "Some other notes --\nCrafting improves the quality of items in your inventory, and makes food more nutritious.\nMake sure you have both items selected before crafting.\nYou can craft any two items together as long as they are the same level -- even if they have different purposes.\nAll items have a type which influences how they interact with other items.\nWeapons and equipment degrade over time, you can see their current condition in the inventory.",
    "If this is a lot to remember, press \'q\' or \'?\' for a quick summary.",
    "If the fonts are rendering too small or large, there is a font scale slider on the bottom left.",
    "A quick tip -- {tip}",
    "Thank you for listening to me. Please wait a moment as the game world is generated.",
];

pub struct LoadingTypewriter {
    setting_dt: Option<f32>,
    areas_dt: Option<f32>,
    monsters_dt: Option<f32>,
    items_dt: Option<f32>,
}

impl LoadingTypewriter {
    fn new() -> LoadingTypewriter {
        LoadingTypewriter {
            setting_dt: None,
            areas_dt: None,
            monsters_dt: None,
            items_dt: None,
        }
    }

    fn trim<'a>(text: &'a str, dt: &mut Option<f32>) -> &'a str {
        if dt.is_none() {
            *dt = Some(0.0);
        }
        if text.chars().next().is_none() {
            return "";
        }

        let length = (dt.unwrap() * CHARS_PER_SECOND_LOADING) as usize;
        let length = text.len().min(length);
        let mut iter = text.char_indices();
        let (end, _) = iter
            .nth(length)
            .unwrap_or(text.char_indices().last().unwrap());
        &text[..end]
    }

    fn get_setting_text<'a>(&mut self, text: &'a str) -> &'a str {
        LoadingTypewriter::trim(text, &mut self.setting_dt)
    }

    fn get_areas_text<'a>(&mut self, text: &'a str) -> &'a str {
        LoadingTypewriter::trim(text, &mut self.areas_dt)
    }

    fn get_monsters_text<'a>(&mut self, text: &'a str) -> &'a str {
        LoadingTypewriter::trim(text, &mut self.monsters_dt)
    }

    fn get_items_text<'a>(&mut self, text: &'a str) -> &'a str {
        LoadingTypewriter::trim(text, &mut self.items_dt)
    }

    fn advance(&mut self) {
        self.setting_dt = self.setting_dt.map(|dt| dt + get_frame_time());
        self.areas_dt = self.areas_dt.map(|dt| dt + get_frame_time());
        self.monsters_dt = self.monsters_dt.map(|dt| dt + get_frame_time());
        self.items_dt = self.items_dt.map(|dt| dt + get_frame_time());
    }
}

pub struct IntroState {
    step: usize,
    prompt_dt: f32,
    typewriter: LoadingTypewriter,
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
            prompt_dt: 0.,
            typewriter: LoadingTypewriter::new(),
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
    let num_typewritten_chars = (CHARS_PER_SECOND * intro_state.prompt_dt) as usize;
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
                        intro_state.prompt_dt = 0.;
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
                intro_state.prompt_dt = 0.;
            } else {
                intro_state.prompt_dt = 1000.;
            }
        }
    }
}

pub fn intro_loop(state: &mut IntroState, ig: &Option<IdeaGuy>) -> bool {
    let mut continuing = true;
    state.prompt_dt += get_frame_time();

    let font_size = screen_width() / 100.;
    let spacing = screen_width() / 90.;
    if let Some(ig) = ig {
        if let Some(setting) = &ig.setting {
            let setting = state.typewriter.get_setting_text(setting);
            for (i, line) in textwrap::wrap(setting, (screen_width() / (font_size * 2.)) as usize)
                .iter()
                .enumerate()
            {
                draw_text(
                    &line,
                    screen_width() * 0.1,
                    spacing * i as f32 + screen_height() * 0.1,
                    font_size,
                    BLACK,
                );
            }
        } else {
            draw_text(
                "Loading setting...",
                screen_width() * 0.1,
                screen_height() * 0.1,
                font_size,
                BLACK,
            );
        }

        if let Some(monsters) = &ig.monsters {
            let monsters: Vec<_> = monsters
                .iter()
                .map(|m| format!("{}: {}\n", m.name, m.description))
                .collect();
            let monsters = monsters.join("\n");
            let monsters = state.typewriter.get_monsters_text(&monsters);
            for (i, line) in textwrap::wrap(monsters, (screen_width() / (font_size * 2.)) as usize)
                .iter()
                .enumerate()
            {
                draw_text(
                    &line,
                    screen_width() * 0.6,
                    spacing * i as f32 + screen_height() * 0.1,
                    font_size,
                    BLACK,
                );
            }
        } else {
            draw_text(
                "Loading monsters...",
                screen_width() * 0.6,
                screen_height() * 0.1,
                font_size,
                BLACK,
            );
        }
    }
    state.typewriter.advance();

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
