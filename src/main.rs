use chargen::Chargen;
use macroquad::prelude::*;
use net::{GameDefs, IdeaGuy};
use std::collections::HashMap;
use world::PlayerAction;

mod chargen;
mod fov;
mod grid;
mod intro;
mod main_menu;
mod map_gen;
mod net;
mod ratelimit;
mod render;
mod save;
mod util;
mod world;

use crate::grid::{EAST, NORTH, SOUTH, WEST};

enum GameState {
    Menu(main_menu::Menu),
    Intro(intro::IntroState),
    Chargen(chargen::Chargen),
    Play(PlayState),
}

struct PlayState {
    sim: world::World,
    memory: world::Memory,
    ui: render::Ui,
    pressed_keys: HashMap<KeyCode, f32>,
}

const KEYS_WITH_REPEAT: &[KeyCode] = &[
    KeyCode::L,
    KeyCode::Right,
    KeyCode::H,
    KeyCode::Left,
    KeyCode::J,
    KeyCode::Down,
    KeyCode::K,
    KeyCode::Up,
];

const INIT_KEY_REPEAT: f32 = 0.5;
const KEY_REPEAT_DELAY: f32 = 1.0 / 30.0;

pub fn random() -> u64 {
    ::rand::random()
}

impl PlayState {
    pub fn new(font: Font, ig: &mut IdeaGuy, ch: net::Character) -> Self {
        let mut sim = world::World::new();
        sim.update_defs(ig);
        map_gen::generate_world(&mut sim, random());
        sim.apply_character(ch);
        let memory = world::Memory::new();
        let ui = render::Ui::new(None, font);
        sim.post_init();
        let pressed_keys = HashMap::new();
        let mut slf = Self {
            sim,
            ui,
            memory,
            pressed_keys,
        };
        slf.update_memory();

        slf
    }

    pub fn equip(&mut self) -> bool {
        let mut tick = false;
        if let Some(&min) = self.ui.inventory_selected.iter().min() {
            tick |= self.sim.do_player_action(PlayerAction::Use(min));
            self.ui.inventory_selected.remove(&min);
        }
        tick
    }

    pub fn craft(&mut self) -> bool {
        let mut tick = false;
        let mut selected = self
            .ui
            .inventory_selected
            .iter()
            .copied()
            .collect::<Vec<_>>();
        selected.sort();
        if selected.len() >= 2 {
            tick |= self
                .sim
                .do_player_action(PlayerAction::Craft(selected[0], selected[1]));
            self.ui.inventory_selected.remove(&selected[0]);
            self.ui.inventory_selected.remove(&selected[1]);
        }
        tick
    }

    pub fn drop(&mut self) -> bool {
        let mut tick = false;
        if let Some(&min) = self.ui.inventory_selected.iter().min() {
            tick |= self.sim.do_player_action(PlayerAction::Drop(min));
            self.ui.inventory_selected.remove(&min);
        }
        tick
    }

    pub fn inspect(&mut self) {
        for item in self.ui.inventory_selected.iter() {
            if let Some(item) = self.sim.inventory.items.get(*item).map(|x| &x.item) {
                self.sim.log_message(vec![match item {
                    world::Item::Instance(ii) => (
                        format!("{}: {}", ii.info.name, ii.info.description.clone()),
                        ii.info.ty.get_color(),
                    ),
                    world::Item::PendingCraft(_, _) => {
                        ("Crafting in progress...".into(), net::Color::Pink)
                    }
                }]);
            }
        }
    }

    pub fn handle_buttons(&mut self) {
        // Handle in-game UI button presses.
        let tick = if let Some(ui_button) = self.ui.ui_button {
            match ui_button {
                render::UiButton::Equip => {
                    self.equip();
                    true
                }
                render::UiButton::Drop => {
                    self.drop();
                    true
                }
                render::UiButton::Craft => {
                    self.craft();
                    true
                }
                render::UiButton::Inspect => {
                    self.inspect();
                    false
                }
            }
        } else {
            false
        };
        self.ui.ui_button = None;
        if tick {
            self.tick();
        }
    }

    /// Returns true if the game should be restarted.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        let mut tick = false;
        match key {
            KeyCode::L | KeyCode::Right => {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    tick |= self.sim.do_player_action(PlayerAction::Fire(EAST));
                } else {
                    tick |= self.sim.do_player_action(PlayerAction::Move(EAST));
                }
            }
            KeyCode::H | KeyCode::Left => {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    tick |= self.sim.do_player_action(PlayerAction::Fire(WEST));
                } else {
                    tick |= self.sim.do_player_action(PlayerAction::Move(WEST));
                }
            }
            KeyCode::J | KeyCode::Down => {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    tick |= self.sim.do_player_action(PlayerAction::Fire(SOUTH));
                } else {
                    tick |= self.sim.do_player_action(PlayerAction::Move(SOUTH));
                }
            }
            KeyCode::K | KeyCode::Up => {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    tick |= self.sim.do_player_action(PlayerAction::Fire(NORTH));
                } else {
                    tick |= self.sim.do_player_action(PlayerAction::Move(NORTH));
                }
            }
            KeyCode::I => {
                self.ui.toggle_ui();
                tick = false
            }
            KeyCode::Comma | KeyCode::G => {
                tick |= self.sim.do_player_action(PlayerAction::PickUp);
            }
            KeyCode::Period | KeyCode::Space => {
                tick |= self.sim.do_player_action(PlayerAction::Wait);
            }
            KeyCode::E | KeyCode::A => tick |= self.equip(),
            KeyCode::C => tick |= self.craft(),
            KeyCode::D => tick |= self.drop(),
            KeyCode::Q => self.ui.toggle_help(),
            KeyCode::R => {
                if self.sim.player_is_dead() {
                    return true;
                }
            }
            KeyCode::Slash | KeyCode::Semicolon => {
                if matches!(key, KeyCode::Slash)
                    && (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift))
                {
                    // If they're actually pressing ?
                    self.ui.toggle_help();
                } else {
                    self.inspect();
                }
            }
            KeyCode::Escape => {
                self.ui.ui_selected = false;
            }
            _ => {
                let key = key as usize;
                if key >= KeyCode::Key0 as usize && key <= KeyCode::Key9 as usize {
                    // Change this so that we only open the UI if a real
                    // inventory item is selected.
                    self.ui.ui_selected = true;
                    let key = key - KeyCode::Key0 as usize;
                    if self.ui.inventory_selected.contains(&key) {
                        self.ui.inventory_selected.remove(&key);
                    } else {
                        self.ui.inventory_selected.insert(key);
                    }
                    tick = false
                }
            }
        }

        if tick {
            self.tick();
        }
        false
    }

    fn update_memory(&mut self) {
        let seen = fov::calculate_fov(self.sim.get_player_pos(), world::FOV_RANGE, &self.sim);
        self.memory.mobs.clear();
        for pos in seen {
            self.memory.tile_map[pos] = Some(self.sim.get_tile(pos));
            if let Some(mob) = self.sim.get_mob(pos) {
                self.memory.mobs.insert(pos, mob.clone());
            }
        }
    }

    fn tick(&mut self) {
        self.update_memory()
    }
}

fn egui_startup() {
    egui_macroquad::ui(|egui_ctx| {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "DejaVuSansMono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/DejaVuSansMono.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "DejaVuSansMono".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("DejaVuSansMono".to_owned());

        egui_ctx.set_fonts(fonts);
    });
}

fn egui_update_scaling(user_scale: f32) {
    egui_macroquad::ui(|egui_ctx| {
        let game_scale = screen_width().min(screen_height());
        let scale_factor = miniquad::window::dpi_scale() * (game_scale / 1200.) * user_scale;
        use egui::FontFamily::*;
        use egui::TextStyle::*;
        let mut style = (*egui_ctx.style()).clone();
        style.text_styles = [
            (
                Heading,
                egui::FontId::new(20.0 * scale_factor, Proportional),
            ),
            (
                heading2(),
                egui::FontId::new(25.0 * scale_factor, Proportional),
            ),
            (
                heading3(),
                egui::FontId::new(23.0 * scale_factor, Proportional),
            ),
            (Body, egui::FontId::new(18.0 * scale_factor, Proportional)),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(14.0 * scale_factor, Proportional),
            ),
            (Button, egui::FontId::new(14.0 * scale_factor, Proportional)),
            (Small, egui::FontId::new(10.0 * scale_factor, Proportional)),
        ]
        .into();
        egui_ctx.set_style(style);

        let mut visuals = egui::Visuals::default();
        visuals.window_shadow.extrusion = 0.;
        visuals.popup_shadow.extrusion = 0.;
        egui_ctx.set_visuals(visuals);
    });
}

#[inline]
fn heading2() -> egui::TextStyle {
    egui::TextStyle::Name("Heading2".into())
}

#[inline]
fn heading3() -> egui::TextStyle {
    egui::TextStyle::Name("ContextHeading".into())
}

fn window_conf() -> Conf {
    Conf {
        window_title: "EverythingRL".to_owned(),
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font("assets/DejaVuSansMono.ttf").await.unwrap();
    egui_startup();
    egui_update_scaling(1.0);
    quad_storage::STORAGE.lock().unwrap().set("test1", "test2");

    let mut last_size = (screen_width(), screen_height());
    let mut last_user_scale_factor = 1.0;
    let mut gs = GameState::Menu(main_menu::Menu::new());
    let mut ig: Option<IdeaGuy> = None;

    loop {
        if let Some(ref mut ig) = ig {
            ig.tick()
        };
        clear_background(GRAY);

        let user_scale = if let GameState::Play(ref ps) = gs {
            ps.ui.user_scale_factor
        } else {
            1.0
        };
        if (screen_width(), screen_height()) != last_size || last_user_scale_factor != user_scale {
            egui_update_scaling(user_scale);
            last_size = (screen_width(), screen_height());
            last_user_scale_factor = user_scale;
        }

        let mut restart = false;
        gs = match gs {
            GameState::Chargen(ref mut chargen) => {
                if let Some(c) = chargen.tick() {
                    GameState::Play(PlayState::new(font.clone(), ig.as_mut().unwrap(), c))
                } else {
                    gs
                }
            }
            GameState::Menu(ref mut menu) => match menu.tick() {
                None => gs,
                Some(main_menu::Choice::Play) => GameState::Intro(intro::IntroState::new()),
                Some(main_menu::Choice::Load(def)) => {
                    let defs: GameDefs = serde_json::from_value(def.defs).unwrap();
                    ig = Some(IdeaGuy::from_saved(defs.clone()));
                    GameState::Chargen(crate::chargen::Chargen::new(defs))
                }
            },
            GameState::Intro(ref mut intro) => {
                if intro.ready_for_generation && ig.is_none() {
                    ig = Some(IdeaGuy::new(&intro.theme));
                }
                let intro_waiting = intro::intro_loop(intro, &ig);
                if intro.exit {
                    return;
                }
                let mut new_gs = gs;
                if !intro_waiting {
                    if let Some(ig) = ig.as_mut() {
                        if ig.game_defs.boss.is_some() {
                            crate::save::save_def(&ig.game_defs);
                            new_gs = GameState::Chargen(crate::chargen::Chargen::new(
                                ig.game_defs.clone(),
                            ));
                        }
                    }
                }
                new_gs
            }
            GameState::Play(ref mut ps) => {
                let ig = ig.as_mut().unwrap();
                ps.sim.update_defs(ig);
                if let Some(key) = get_last_key_pressed() {
                    restart |= ps.handle_key(key);
                    if KEYS_WITH_REPEAT.contains(&key) {
                        ps.pressed_keys.insert(key, 0.0);
                    }
                }
                // Key repeat, once per second
                ps.pressed_keys.retain(|k, _v| is_key_down(*k));
                let mut keys_to_repeat = vec![];
                for (k, v) in ps.pressed_keys.iter_mut() {
                    let old_v = *v;
                    *v += get_frame_time();
                    if *v >= INIT_KEY_REPEAT
                        && (((old_v - INIT_KEY_REPEAT) / KEY_REPEAT_DELAY).floor()
                            != ((*v - INIT_KEY_REPEAT) / KEY_REPEAT_DELAY).floor())
                    {
                        keys_to_repeat.push(*k);
                    }
                }
                for k in keys_to_repeat {
                    ps.handle_key(k);
                }

                ps.handle_buttons();

                // Handle animations
                for untriggered_animation in ps.sim.untriggered_animations.iter() {
                    ps.ui.add_animation(untriggered_animation.clone());
                }
                ps.sim.untriggered_animations.clear();

                ps.ui.render(&ps.sim, &ps.memory);

                if restart {
                    GameState::Chargen(Chargen::new(ig.game_defs.clone()))
                } else {
                    gs
                }
            }
        };

        next_frame().await
    }
}
