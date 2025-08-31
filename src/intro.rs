use crate::net::{GenerationAbortReason, IdeaGuy, InitialGenerationStatus};
use ::rand::{
    seq::{index, SliceRandom},
    SeedableRng,
};
use macroquad::prelude::*;

pub const CHARS_PER_SECOND: f32 = 70.;
pub const CHARS_PER_SECOND_LOADING: f32 = 80.;

pub const SETTINGS: [&str; 11] = [
    "Richard Adams's Watership Down",
    "Frank Herbert's Dune",
    "Pirates, just lots of pirates",
    "Ridley Scott's 1979 Alien movie",
    "Microcenter",
    "Star Wars but everyone is a cat",
    "Lord of the Rings but everyone is a cat",
    "Bears. Just lots of bears.",
    "Nethack, but better somehow.",
    "Faster Than Light (FTL) (the popular roguelike game)",
    "Tetris",
];

pub const TIPS: [&str; 3] = [
    "think carefully about the type of your weapon, armor and the enemy type before you attack. Some monsters are resistant or completely immune to certain damage types.",
    "most food that you'll find in the world is not particularly nutritious unless cooked.",
    "narrow corridors are your friend. Try luring enemies into a narrow chokepoint to benefit from Lanchester's linear law :)"
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
        let length = (dt.unwrap() * CHARS_PER_SECOND_LOADING) as usize;
        crate::util::trim(text, length)
    }

    fn get_setting_text<'a>(&mut self, text: &'a str) -> &'a str {
        LoadingTypewriter::trim(text, &mut self.setting_dt)
    }

    fn get_monsters_text<'a>(&mut self, text: &'a str) -> &'a str {
        LoadingTypewriter::trim(text, &mut self.monsters_dt)
    }

    fn advance(&mut self) {
        self.setting_dt = self.setting_dt.map(|dt| dt + get_frame_time());
        self.areas_dt = self.areas_dt.map(|dt| dt + get_frame_time());
        self.monsters_dt = self.monsters_dt.map(|dt| dt + get_frame_time());
        self.items_dt = self.items_dt.map(|dt| dt + get_frame_time());
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PromptState {
    Welcome(usize),
    Understand,
    EnterTheme,
    Generating,
    Errored(String),
    Done,
}

pub struct IntroState {
    prompt_state: PromptState,
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
        let mut rng = ::rand::rngs::SmallRng::seed_from_u64(::rand::random());
        IntroState {
            prompt_state: PromptState::Welcome(0),
            prompt_dt: 0.,
            typewriter: LoadingTypewriter::new(),
            exit: false,
            theme: String::new(),
            ready_for_generation: false,
            chosen_tip: (*TIPS.choose(&mut rng).unwrap()).into(),
            chosen_settings: index::sample(&mut rng, SETTINGS.len(), 2)
                .iter()
                .map(|i| (*SETTINGS[i]).into())
                .collect(),
        }
    }

    pub fn reset_from_error(&mut self, msg: String, reason: GenerationAbortReason) {
        self.ready_for_generation = false;
        self.prompt_state = PromptState::Errored(msg);
        if matches!(reason, GenerationAbortReason::ServerError) {
            self.theme = "".into();
        }
    }
}

fn storyteller_window(
    egui_ctx: &egui::Context,
    text: String,
    typewriter_time: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let num_typewritten_chars = (CHARS_PER_SECOND * typewriter_time) as usize;
    let typewritten_text: String = text.chars().take(num_typewritten_chars).collect();
    let width = screen_width() * miniquad::window::dpi_scale();
    let padding = (3.0 * miniquad::window::dpi_scale()) as i8;
    egui::Window::new("StoryTeller")
        .resizable(false)
        .collapsible(false)
        .min_width(width / 2.0)
        .max_width(width)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
        .show(egui_ctx, |ui| {
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(padding, padding))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.label(egui::RichText::new(typewritten_text));
                        ui.separator();
                    });
                    add_contents(ui);
                });
        });
}

fn draw_background(state: &mut IntroState, ig: &IdeaGuy) {
    let font_size = screen_width() / 100.;
    let spacing = screen_width() / 90.;

    if let Some(setting) = &ig.game_defs.setting_desc {
        let setting = state.typewriter.get_setting_text(setting);
        for (i, line) in textwrap::wrap(setting, (screen_width() / (font_size * 2.)) as usize)
            .iter()
            .enumerate()
        {
            draw_text(
                line,
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

    if !ig.game_defs.monsters.is_empty() {
        let monsters: Vec<_> = ig
            .game_defs
            .monsters
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
                line,
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

pub fn intro_loop(state: &mut IntroState, ig: &Option<IdeaGuy>, egui_ctx: &egui::Context) -> bool {
    state.prompt_dt += get_frame_time();

    if let Some(ig) = ig {
        draw_background(state, ig);
    }
    state.typewriter.advance();

    let setting1 = &state.chosen_settings[0];
    let setting2 = &state.chosen_settings[1];
    let tip = &state.chosen_tip;

    let text = match state.prompt_state {
        PromptState::Welcome(0) => "Welcome, traveler. I am the Storyteller of this roguelike game.".into(),
        PromptState::Welcome(1) => "My objective is to create the world of the game which you are about to play. From the inhabitants of this virtual dungeon, to their implements and attire, to their demeanor and persuasion, to the very earth they step foot upon....".into(),
        PromptState::Welcome(_) => "... they will be invented by yours truly. With a bit of help from you of course.".into(),
        PromptState::Understand => "As you might have guessed by this point, the game you are about to play includes AI-generated elements. Despite the implemented safety features, it is entirely possible for the underlying system to produce inaccurate or offensive content. Click \"I understand\" if you understand these risks and wish to continue, otherwise click Exit to exit the game.".into(),
        PromptState::EnterTheme => format!("Very well. Please describe the setting of the game which you would like to play. It can be literally anything. For example, you could say \"{setting1}\" or \"{setting2}\" to generate fantasy/sci-fi worlds in those settings."),
        PromptState::Generating => format!("Good. It'll take around 60 seconds to generate your prompt. In the meantime, a couple small notes.\n\nCONTROLS\n\nPress 'q' at any time to see a summary of these controls.\nThe movement keys are hjkl/arrows.\nHold down shift and move to use your ranged weapon.\n\'i\' opens inventory\n\'.\' waits for a moment\n\',\' picks up an item\n\'0-9\' multi-selects inventory items\n\'e\' equips/eats an item.\n\'d\' drops selected items\n\'c\' combines/cooks items\n\';\' or \'/\' will inspect an item.\n\nSome other notes --\n\nCrafting improves the quality of items in your inventory, and makes food more nutritious.\nMake sure you have both items selected before crafting.\nYou can craft any two items together as long as they are the same level -- even if they have different purposes.\nAll items have a type which influences how they interact with other items.\nWeapons and equipment degrade over time, you can see their current condition in the inventory.\n\nIf this is a lot to remember, press \'q\' for a quick summary.\n\nIf the fonts are rendering too small or large, there is a font scale slider on the bottom left.\n\nA quick tip -- {tip}\n\nThank you for listening to me. Please wait a moment as the game world is generated."),
        PromptState::Errored(ref msg) => format!("Error: {msg}. Please try again."),
        PromptState::Done => "Starting the game!".into(),
        };

    storyteller_window(egui_ctx, text, state.prompt_dt, |ui| {
        let old_prompt_state = state.prompt_state.clone();
        match state.prompt_state {
            PromptState::Welcome(n) => {
                ui.label(
                    egui::RichText::new("(Press Enter to continue)")
                        .small()
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                );
                if let Some(KeyCode::Enter) = get_last_key_pressed() {
                    state.prompt_state = PromptState::Welcome(n + 1);
                    if n >= 2 {
                        state.prompt_state = PromptState::Understand;
                    }
                }
            }
            PromptState::Understand => {
                if ui.button("I understand").clicked() {
                    state.prompt_state = PromptState::EnterTheme;
                }
                if ui.button("Exit").clicked() {
                    state.exit = true;
                }
            }
            PromptState::EnterTheme | PromptState::Errored(..) => {
                state.ready_for_generation = false;
                ui.add(
                    egui::widgets::TextEdit::singleline(&mut state.theme)
                        .desired_width(f32::INFINITY),
                );
                if let Some(KeyCode::Enter) = get_last_key_pressed() {
                    if !state.theme.is_empty() {
                        state.prompt_state = PromptState::Generating;
                    }
                }
            }
            PromptState::Generating => {
                state.ready_for_generation = true;
                match ig.as_ref().map(|ig| ig.get_state()) {
                    Some(InitialGenerationStatus::Done) => {
                        ui.label(
                            egui::RichText::new("Done! Press Enter to continue!")
                                .color(egui::Color32::from_rgb(0, 255, 0)),
                        );
                        if let Some(KeyCode::Enter) = get_last_key_pressed() {
                            state.prompt_state = PromptState::Done;
                        }
                    }
                    Some(InitialGenerationStatus::Generating { msg, .. }) => {
                        ui.label(&msg);
                    }
                    Some(InitialGenerationStatus::ErroredOut { msg, .. }) => {
                        ui.label(&msg);
                    }
                    None => {}
                }
            }
            PromptState::Done => {}
        }
        if state.prompt_state != old_prompt_state {
            state.prompt_dt = 0.0;
        } else if let Some(KeyCode::Enter) = get_last_key_pressed() {
            state.prompt_dt += 1000.0;
        }
    });

    !matches!(state.prompt_state, PromptState::Done)
}
