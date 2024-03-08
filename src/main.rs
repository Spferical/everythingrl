use macroquad::prelude::*;
use net::IdeaGuy;
use world::PlayerAction;

mod fov;
mod grid;
mod intro;
mod map_gen;
mod net;
mod render;
#[cfg(target_family = "wasm")]
mod wasm;
mod world;

use crate::grid::{EAST, NORTH, SOUTH, WEST};

enum GameState {
    Intro(intro::IntroState),
    Startup,
    Play(PlayState),
}

struct PlayState {
    sim: world::World,
    memory: world::Memory,
    ui: render::Ui,
}

impl PlayState {
    pub fn new(font: Font, ig: &mut IdeaGuy) -> Self {
        assert!(ig.monsters.is_some());
        assert!(ig.items.is_some());
        let mut sim = world::World::new();
        sim.update_defs(ig);
        map_gen::generate_world(&mut sim, 0x11_22_33_44_55_66_77_88);
        let memory = world::Memory::new();
        let ui = render::Ui::new(None, font);
        sim.post_init();
        let mut slf = Self { sim, ui, memory };
        slf.update_memory();

        slf
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        let mut tick = false;
        match key {
            KeyCode::L | KeyCode::Right => {
                tick |= self.sim.do_player_action(PlayerAction::Move(EAST));
            }
            KeyCode::H | KeyCode::Left => {
                tick |= self.sim.do_player_action(PlayerAction::Move(WEST));
            }
            KeyCode::J | KeyCode::Down => {
                tick |= self.sim.do_player_action(PlayerAction::Move(SOUTH));
            }
            KeyCode::K | KeyCode::Up => {
                tick |= self.sim.do_player_action(PlayerAction::Move(NORTH));
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
            KeyCode::E => {
                if let Some(&min) = self.ui.inventory_selected.iter().min() {
                    tick |= self.sim.do_player_action(PlayerAction::ToggleEquip(min));
                    self.ui.inventory_selected.remove(&min);
                }
            }
            KeyCode::C => {
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
            }
            KeyCode::D => {
                if let Some(&min) = self.ui.inventory_selected.iter().min() {
                    tick |= self.sim.do_player_action(PlayerAction::Drop(min));
                    self.ui.inventory_selected.remove(&min);
                }
            }
            KeyCode::Slash | KeyCode::Semicolon => {
                for item in self.ui.inventory_selected.iter() {
                    let item = self.sim.inventory.items[*item].item;
                    self.sim.log_message(vec![match item {
                        world::Item::Corpse(item) => {
                            let mob_desc = &self.sim.get_mobkind_info(item);
                            (format!("{} Corpse", mob_desc.name), net::Color::Maroon)
                        }
                        world::Item::Equipment(item) => {
                            let item_desc = &self.sim.get_equipmentkind_info(item.kind);
                            (item_desc.description.clone(), item_desc.color)
                        }
                    }]);
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

fn egui_setup() {
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

        let game_scale = screen_width().min(screen_height());
        let scale_factor = miniquad::window::dpi_scale() * (game_scale / 1200.);
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

#[macroquad::main("game")]
async fn main() {
    let font = load_ttf_font("assets/DejaVuSansMono.ttf").await.unwrap();
    egui_setup();

    let mut last_size = (screen_width(), screen_height());
    let mut gs = GameState::Intro(intro::IntroState::new());
    let mut ig: Option<IdeaGuy> = None;
    loop {
        if let Some(ref mut ig) = ig {
            ig.tick()
        };
        clear_background(GRAY);

        if (screen_width(), screen_height()) != last_size {
            egui_setup();
            last_size = (screen_width(), screen_height());
        }

        gs = match gs {
            GameState::Intro(ref mut intro) => {
                if intro.ready_for_generation && ig.is_none() {
                    ig = Some(IdeaGuy::new(&intro.theme));
                }
                if !intro::intro_loop(intro) {
                    GameState::Startup
                } else {
                    if intro.exit {
                        return;
                    }
                    gs
                }
            }
            GameState::Startup => {
                let ig = ig.as_mut().unwrap();
                if ig.monsters.is_some() {
                    if ig.items.is_some() {
                        GameState::Play(PlayState::new(font.clone(), ig))
                    } else {
                        GameState::Startup
                    }
                } else {
                    GameState::Startup
                }
            }
            GameState::Play(ref mut ps) => {
                let ig = ig.as_mut().unwrap();
                ps.sim.update_defs(ig);
                if let Some(key) = get_last_key_pressed() {
                    ps.handle_key(key);
                }

                ps.ui.render(&ps.sim, &ps.memory);
                gs
            }
        };

        next_frame().await
    }
}
