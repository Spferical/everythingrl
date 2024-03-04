use macroquad::prelude::*;
use macroquad::text::Font;
use macroquad::ui::{
    hash, root_ui,
    widgets::{self, Group, Window},
};

use crate::{
    grid::Rect,
    world::{MobKind, TileKind},
};

pub struct Ui {
    grid_size: usize,
    font: Font,
    ui_selected: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    character: char,
    color: Color,
    location: (usize, usize),
    layer: usize,
}

impl Ui {
    pub fn new(grid_size: Option<usize>, font: Font) -> Ui {
        Ui {
            grid_size: grid_size.unwrap_or(32),
            font,
            ui_selected: false,
        }
    }
    pub fn toggle_ui(&mut self) {
        self.ui_selected = !self.ui_selected;
    }

    fn render_inventory(&self) {
        let game_size = screen_width().min(screen_height());
        let offset_x = screen_width() / 2. - game_size / 8.;
        let offset_y = game_size / 3.;

        let window_width = (game_size / 4.).max(160.);
        let window_height = (game_size / 2.).max(200.);

        let element_width = window_width - 20.;
        let element_height = (window_height / 6.).max(50.);

        Window::new(
            hash!(),
            vec2(offset_x, offset_y),
            vec2(window_width, window_height),
        )
        .label("Inventory")
        .titlebar(true)
        .ui(&mut *root_ui(), |ui| {
            for i in 0..8 {
                Group::new(hash!("shop", i), Vec2::new(element_width, element_height)).ui(
                    ui,
                    |ui| {
                        ui.label(
                            Vec2::new(element_width * 0.1, element_height * 0.1),
                            &format!("Item N {}", i),
                        );
                        ui.label(
                            Vec2::new(element_width * 0.2, element_height * 0.4),
                            "Type: Ice",
                        );
                        ui.label(
                            Vec2::new(element_width * 0.2, element_height * 0.6),
                            &format!("Attack {}", 2),
                        );
                        if ui.button(
                            Vec2::new(element_width * 0.7, element_height * 0.1),
                            "Equip",
                        ) {
                            println!("Test!");
                        }
                    },
                );
            }
        });
    }

    pub fn render(&mut self, sim: &crate::world::World, memory: &crate::world::Memory) {
        if self.ui_selected {
            self.render_inventory();
        }
        let player_pos = sim.get_player_pos();
        let grid_rect =
            Rect::new_centered(player_pos, self.grid_size as i32, self.grid_size as i32);
        let upper_left = grid_rect.topleft();
        let mut glyphs = vec![Glyph {
            character: '@',
            color: WHITE,
            location: (player_pos.x as usize, player_pos.y as usize),
            layer: 1,
        }];
        for pos in grid_rect {
            let tile = memory.tile_map[pos];
            if let Some(tile) = tile {
                let (character, color) = match tile.kind {
                    TileKind::Floor => ('.', LIGHTGRAY),
                    TileKind::Wall => ('#', WHITE),
                    TileKind::YellowFloor => ('.', YELLOW),
                    TileKind::YellowWall => ('#', YELLOW),
                    TileKind::BloodyFloor => ('.', RED),
                };
                glyphs.push(Glyph {
                    character,
                    color,
                    location: (pos.x as usize, pos.y as usize),
                    layer: 0,
                });
                if let Some(item) = tile.item {
                    let (character, color) = match item {
                        crate::world::Item::Corpse(_) => ('%', MAROON),
                    };
                    glyphs.push(Glyph {
                        character,
                        color,
                        location: (pos.x as usize, pos.y as usize),
                        layer: 0,
                    });
                }
            }
            if let Some(mob) = memory.mobs.get(&pos) {
                let (character, color) = match mob.kind {
                    MobKind::Cat => ('c', YELLOW),
                    MobKind::Alien => ('a', PURPLE),
                };
                glyphs.push(Glyph {
                    character,
                    color,
                    location: (pos.x as usize, pos.y as usize),
                    layer: 1,
                });
            }
        }
        self.render_glyphs(
            &glyphs
                .iter()
                .map(|&glyph| {
                    (
                        (
                            glyph.location.0 as i32 - upper_left.x,
                            glyph.location.1 as i32 - upper_left.y,
                        ),
                        glyph,
                    )
                })
                .filter(|&(pos, _)| {
                    pos.0 >= 0
                        && pos.0 < self.grid_size as i32
                        && pos.1 >= 0
                        && pos.1 < self.grid_size as i32
                })
                .map(|(pos, glyph)| Glyph {
                    character: glyph.character,
                    color: glyph.color,
                    location: (pos.0 as usize, pos.1 as usize),
                    layer: glyph.layer,
                })
                .collect::<Vec<Glyph>>(),
        );

        if macroquad::ui::root_ui().button(None, "Inventory") {
            self.toggle_ui();
        }
    }

    fn render_glyphs(&self, glyphs: &[Glyph]) {
        let game_size = screen_width().min(screen_height());
        let offset_x = (screen_width() - game_size) / 2. + 10.;
        let offset_y = (screen_height() - game_size) / 2. + 10.;
        let sq_size = (screen_height() - offset_y * 2.) / self.grid_size as f32;

        // First, set the actual background of the grid to black
        draw_rectangle(offset_x, offset_y, game_size - 20., game_size - 20., BLACK);

        // Next, draw the grid in.
        for i in 1..self.grid_size {
            draw_line(
                offset_x,
                offset_y + sq_size * i as f32,
                screen_width() - offset_x,
                offset_y + sq_size * i as f32,
                1.,
                DARKGRAY,
            );
        }

        for i in 1..self.grid_size {
            draw_line(
                offset_x + sq_size * i as f32,
                offset_y,
                offset_x + sq_size * i as f32,
                screen_height() - offset_y,
                1.,
                DARKGRAY,
            );
        }

        // Quick check to ensure that the foreground replaces the background.
        let mut z_buffer = vec![vec![0; self.grid_size]; self.grid_size];
        for glyph in glyphs {
            z_buffer[glyph.location.0][glyph.location.1] =
                z_buffer[glyph.location.0][glyph.location.1].max(glyph.layer);
        }

        for glyph in glyphs {
            if glyph.layer >= z_buffer[glyph.location.0][glyph.location.1] {
                draw_text_ex(
                    &format!("{}", glyph.character),
                    offset_x + sq_size * (glyph.location.0 as f32 + 0.25),
                    offset_y + sq_size * (glyph.location.1 as f32 + 0.75),
                    TextParams {
                        font_size: (sq_size * 0.8) as u16,
                        font: Some(&self.font),
                        color: glyph.color,
                        ..Default::default()
                    },
                )
            }
        }
    }
}
