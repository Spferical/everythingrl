use egui::Color32;
use macroquad::prelude::*;
use macroquad::text::Font;
use std::collections::HashSet;

use crate::world::{EquipmentSlot, Item, MobKindInfo};
use crate::{grid::Pos, grid::Rect, world::TileKind};

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

    fn render_inventory(&mut self, egui_ctx: &egui::Context, sim: &crate::world::World) {
        egui::Window::new("Inventory")
            .resizable(false)
            .collapsible(false)
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
                        .column(egui_extras::Column::auto())
                        .sense(egui::Sense::click());
                    table
                        .header(text_height, |mut header| {
                            header.col(|ui| {
                                ui.strong("Key");
                            });
                            header.col(|ui| {
                                ui.strong("Name");
                            });
                            header.col(|ui| {
                                ui.strong("Type");
                            });
                            header.col(|ui| {
                                ui.strong("Level");
                            });
                            header.col(|ui| {
                                ui.strong("Slot");
                            });
                            header.col(|ui| {
                                ui.strong("Equipped");
                            });
                        })
                        .body(|body| {
                            body.rows(text_height, sim.inventory.items.len(), |mut row| {
                                let row_index = row.index();
                                row.set_selected(self.inventory_selected.contains(&row_index));
                                let slot = &sim.inventory.items[row_index];
                                let name;
                                let display_slot;
                                let display_equipped;
                                let level;
                                let mut types = vec![];
                                match slot.item {
                                    Item::Corpse(mob_kind) => {
                                        let mob_desc = &sim.get_mobkind_info(mob_kind);
                                        name = format!("{} Corpse", mob_desc.name);
                                        types.push(mob_desc.type1);
                                        types.extend(mob_desc.type2);
                                        display_slot = "";
                                        display_equipped = "";
                                        level = mob_desc.level.to_string();
                                    }
                                    Item::Equipment(item_kind) => {
                                        let item_desc = &sim.get_equipmentkind_info(item_kind);
                                        name = item_desc.name.clone();
                                        types.push(item_desc.ty);
                                        display_slot = match item_desc.slot {
                                            EquipmentSlot::Weapon => "Weapon",
                                            EquipmentSlot::Armor => "Equipment",
                                        };
                                        if slot.equipped {
                                            display_equipped = "YES";
                                        } else {
                                            display_equipped = "CAN";
                                        }
                                        level = item_desc.level.to_string();
                                    }
                                }

                                row.col(|ui| {
                                    ui.label(row_index.to_string());
                                });
                                row.col(|ui| {
                                    ui.label(name);
                                });
                                row.col(|ui| {
                                    for ty in types {
                                        let mq_color =
                                            macroquad::color::Color::from(ty.get_color());
                                        let [r, g, b, _a] = mq_color.into();
                                        let color = Color32::from_rgb(r, g, b);
                                        ui.colored_label(color, ty.to_string());
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(level);
                                });
                                row.col(|ui| {
                                    ui.label(display_slot);
                                });
                                row.col(|ui| {
                                    ui.label(display_equipped);
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
    }

    pub fn render(&mut self, sim: &crate::world::World, memory: &crate::world::Memory) {
        egui_macroquad::ui(|egui_ctx| {
            if self.ui_selected {
                self.render_inventory(&egui_ctx, &sim);
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
                            Item::Equipment(ek) => {
                                let equip_def = sim.get_equipmentkind_info(ek);
                                let char = match equip_def.slot {
                                    EquipmentSlot::Weapon => '/',
                                    EquipmentSlot::Armor => '[',
                                };
                                let color = equip_def.ty.get_color().into();
                                (char, color)
                            }
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
                    let mob_kind_info = sim.get_mobkind_info(mob.kind);
                    glyphs.push(Glyph {
                        character: mob_kind_info.char.chars().next().unwrap(),
                        color: mob_kind_info.color.into(),
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
                screen_width() * (1. / 4.),
            );
            self.render_side_ui(sim, screen_width() * (1. / 4.));
        });

        egui_macroquad::draw();
    }

    fn render_side_ui(&self, sim: &crate::world::World, right_offset: f32) {
        let game_width = screen_width() - right_offset;
        let game_size = game_width.min(screen_height());
        let game_height = game_size;

        let offset_x = (screen_width() + game_size - right_offset) / 2. + 10.;
        let offset_y = (screen_height() - game_size) / 2. + 10.;
        let window_lower_bound = offset_y + (game_size - 20.);

        let sq_size = (game_height - offset_y * 2.) / self.grid_size as f32;

        let window_width = screen_width() - offset_x - 10.0;

        // Rectangle for mob info.
        draw_rectangle(
            offset_x,
            offset_y,
            window_width,
            game_size / 2. - 20.,
            BLACK,
        );

        let mob_texts: Vec<(String, macroquad::color::Color)> = sim
            .get_visible_mobs()
            .iter()
            .map(|mob| {
                let damage = mob.damage;
                let mob_kind = mob.kind;
                let mob_kind_def = sim.get_mobkind_info(mob_kind);
                let MobKindInfo {
                    char, color, name, ..
                } = mob_kind_def;
                (
                    format!("{} - {:?}. DAM: {:?}", char, name, damage),
                    color.clone().into(),
                )
            })
            .collect();

        let mut mob_text_buffer: Vec<(String, macroquad::color::Color)> = Vec::new();
        for (text, color) in mob_texts {
            let wrapped = textwrap::wrap(&text, (window_width / (sq_size * 0.5)) as usize);
            for text in wrapped {
                mob_text_buffer.push((text.into(), color));
            }
        }
        mob_text_buffer.truncate(10);

        for (i, (line, color)) in mob_text_buffer.iter().enumerate() {
            draw_text_ex(
                &line,
                offset_x + 2.,
                offset_y + (i + 1) as f32 * sq_size,
                TextParams {
                    font_size: (sq_size * 0.8) as u16,
                    font: Some(&self.font),
                    color: *color,
                    ..Default::default()
                },
            )
        }

        let log_offset_y_base = offset_y + game_size / 2. - 10.;
        let mut log_offset_y = log_offset_y_base;
        let lower_bound = window_lower_bound - sq_size.max(30.0) - 10.;
        let height = lower_bound - log_offset_y;

        draw_rectangle(offset_x, log_offset_y, window_width, height, BLACK);

        for (log_message, log_color) in sim.log.iter().rev().take(10) {
            let wrapped_text =
                textwrap::wrap(&log_message, (window_width / (sq_size * 0.5)) as usize);
            for (i, substr) in wrapped_text.iter().enumerate() {
                let y = log_offset_y + (i + 1) as f32 * sq_size;
                draw_text_ex(
                    &substr,
                    offset_x + 2.,
                    y,
                    TextParams {
                        font_size: (sq_size * 0.8) as u16,
                        font: Some(&self.font),
                        color: *log_color,
                        ..Default::default()
                    },
                );
            }
            log_offset_y += wrapped_text.len() as f32 * sq_size;
        }

        let lower_bound = window_lower_bound;
        let height = sq_size.max(30.0);
        let offset_y = window_lower_bound - height;
        draw_rectangle(offset_x, offset_y, window_width, height, BLACK);

        let status = "10 HP 10 AMMO";
        draw_text_ex(
            status,
            offset_x + 5.,
            lower_bound - height / 4.,
            TextParams {
                font_size: (sq_size * 0.8) as u16,
                font: Some(&self.font),
                color: RED,
                ..Default::default()
            },
        );
    }

    fn render_glyphs(&self, glyphs: &[Glyph], right_offset: f32) {
        let width = screen_width() - right_offset;
        let game_size = width.min(screen_height());
        let offset_x = (screen_width() - game_size - right_offset) / 2. + 10.;
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
