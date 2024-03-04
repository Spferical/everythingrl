use macroquad::prelude::*;
use world::PlayerAction;

mod fov;
mod grid;
mod map_gen;
mod render;
mod world;

use crate::grid::{EAST, NORTH, SOUTH, WEST};

struct GameState {
    sim: world::World,
    memory: world::Memory,
    ui: render::Ui,
}

impl GameState {
    pub fn new(font: Font) -> Self {
        let mut sim = world::World::new();
        map_gen::generate_world(&mut sim, 0x11_22_33_44_55_66_77_88);
        let memory = world::Memory::new();
        let ui = render::Ui::new(None, font);
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
            },
            KeyCode::Comma | KeyCode::G => {
                tick |= self.sim.do_player_action(PlayerAction::PickUp);
            }
            KeyCode::E => {
                tick |= self.sim.do_player_action(PlayerAction::Equip(0));
            }
            KeyCode::U => {
                tick |= self.sim.do_player_action(PlayerAction::Unequip(0));
            }
            KeyCode::D => {
                tick |= self.sim.do_player_action(PlayerAction::Drop(0));
            }
            _ => {

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

#[macroquad::main("game")]
async fn main() {
    let font = load_ttf_font("assets/DejaVuSansMono.ttf").await.unwrap();
    let mut gs = GameState::new(font);
    loop {
        clear_background(GRAY);

        if let Some(key) = get_last_key_pressed() {
            eprintln!("{key:?}");
            gs.handle_key(key);
        }

        gs.ui.render(&gs.sim, &gs.memory);

        next_frame().await
    }
}
