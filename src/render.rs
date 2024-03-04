use macroquad::prelude::*;
use macroquad::text::Font;
use macroquad::ui::{
    hash, root_ui,
    widgets::{Group, Window},
};
use std::collections::HashSet;

use crate::world::{EquipmentKind, Item};
use crate::{
    grid::Pos,
    grid::Rect,
    world::{MobKind, TileKind},
};

pub struct Ui {
    grid_size: usize,
    font: Font,
    pub ui_selected: bool,
    camera_delta: Option<(f32, f32)>,
    last_upper_left: Option<Pos>,
    pub inventory_selected: HashSet<usize>,
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
            camera_delta: None,
            last_upper_left: None,
            inventory_selected: HashSet::new(),
        }
    }
    pub fn toggle_ui(&mut self) {
        self.ui_selected = !self.ui_selected;
    }

    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            if self.inventory_selected.contains(&row_index) {
                self.inventory_selected.remove(&row_index);
            } else {
                self.inventory_selected.insert(row_index);
            }
        }
    }

    fn render_inventory(&mut self, sim: &crate::world::World) {
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Inventory")
                .resizable(false)
                .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    let text_height = egui::TextStyle::Body
                        .resolve(ui.style())
                        .size
                        .max(ui.spacing().interact_size.y);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let table = egui_extras::TableBuilder::new(ui)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(egui_extras::Column::auto())
                            .column(egui_extras::Column::auto())
                            .column(egui_extras::Column::auto())
                            .column(egui_extras::Column::auto())
                            .column(egui_extras::Column::auto())
                            .sense(egui::Sense::click());
                        table
                            .header(text_height, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Hotkey");
                                });
                                header.col(|ui| {
                                    ui.strong("Name");
                                });
                                header.col(|ui| {
                                    ui.strong("Type");
                                });
                                header.col(|ui| {
                                    ui.strong("Attack");
                                });
                                header.col(|ui| {
                                    ui.strong("Body Part");
                                });
                            })
                            .body(|body| {
                                body.rows(text_height, sim.inventory.len(), |mut row| {
                                    let row_index = row.index();
                                    row.set_selected(self.inventory_selected.contains(&row_index));

                                    row.col(|ui| {
                                        ui.label(row_index.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:?}", sim.inventory[row_index]));
                                    });
                                    row.col(|ui| {
                                        ui.label("Ice");
                                    });
                                    row.col(|ui| {
                                        ui.label("4/4");
                                    });
                                    row.col(|ui| {
                                        ui.label("Head");
                                    });

                                    self.toggle_row_selection(row_index, &row.response());
                                });
                            });
                    });
                    if ui.button("Equip/Unequip (e)").clicked() {
                        println!("Equipped {:?}", self.inventory_selected);
                    }
                    if ui.button("Drop (d)").clicked() {
                        println!("Dropped {:?}", self.inventory_selected);
                    };
                    if ui.button("Combine (c)").clicked() {
                        println!("Combined {:?}", self.inventory_selected);
                    };
                });
        });
    }

    pub fn render(&mut self, sim: &crate::world::World, memory: &crate::world::Memory) {
        if self.ui_selected {
            self.render_inventory(&sim);
        }
        let player_pos = sim.get_player_pos();
        let grid_rect =
            Rect::new_centered(player_pos, self.grid_size as i32, self.grid_size as i32);
        let upper_left = grid_rect.topleft();

        self.camera_delta = Some(match self.camera_delta {
            None => (0., 0.),
            Some(old_delta) => {
                let added_delta = match self.last_upper_left {
                    None => (0., 0.),
                    Some(old_upper_left) => (
                        (upper_left.x - old_upper_left.x) as f32,
                        (upper_left.y - old_upper_left.y) as f32,
                    ),
                };
                (
                    (old_delta.0 + added_delta.0) * 0.9,
                    (old_delta.1 + added_delta.1) * 0.9,
                )
            }
        });
        self.last_upper_left = Some(upper_left);

        let mut glyphs = vec![Glyph {
            character: '@',
            color: WHITE,
            location: (player_pos.x as usize, player_pos.y as usize),
            layer: 2,
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
                        Item::Corpse(_) => ('%', MAROON),
                        Item::Equipment(_) => ('[', SKYBLUE),
                    };
                    glyphs.push(Glyph {
                        character,
                        color,
                        location: (pos.x as usize, pos.y as usize),
                        layer: 1,
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
                    layer: 2,
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

        egui_macroquad::draw();
    }

    fn render_glyphs(&self, glyphs: &[Glyph]) {
        let game_size = screen_width().min(screen_height());
        let offset_x = (screen_width() - game_size) / 2. + 10.;
        let offset_y = (screen_height() - game_size) / 2. + 10.;
        let sq_size = (screen_height() - offset_y * 2.) / self.grid_size as f32;

        let delta = self.camera_delta.unwrap_or((0.0, 0.0));
        let delta = (delta.0 * sq_size, delta.1 * sq_size);

        // First, set the actual background of the grid to black
        draw_rectangle(offset_x, offset_y, game_size - 20., game_size - 20., BLACK);

        // Quick check to ensure that the foreground replaces the background.
        let mut z_buffer = vec![vec![0; self.grid_size]; self.grid_size];
        for glyph in glyphs {
            z_buffer[glyph.location.0][glyph.location.1] =
                z_buffer[glyph.location.0][glyph.location.1].max(glyph.layer);
        }

        for glyph in glyphs {
            if glyph.layer >= z_buffer[glyph.location.0][glyph.location.1] {
                let x = delta.0 + offset_x + sq_size * (glyph.location.0 as f32 + 0.25);
                let y = delta.1 + offset_y + sq_size * (glyph.location.1 as f32 + 0.75);
                if x >= offset_x
                    && x < game_size + offset_x - 20.0
                    && y >= offset_y
                    && y < game_size + offset_y - 20.0
                {
                    draw_text_ex(
                        &format!("{}", glyph.character),
                        x,
                        y,
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
}
