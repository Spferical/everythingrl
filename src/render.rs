use egui::Color32;
use macroquad::prelude::*;
use macroquad::text::Font;
use std::collections::HashSet;

use crate::net::Color;
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
    color: macroquad::color::Color,
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
                                    Item::Equipment(item) => {
                                        let item_desc = &sim.get_equipmentkind_info(item.kind);
                                        name = item_desc.name.clone();
                                        types.push(item_desc.ty);
                                        display_slot = match item_desc.slot {
                                            EquipmentSlot::Weapon => "Weapon",
                                            EquipmentSlot::Armor => "Equipment",
                                        };
                                        if slot.equipped {
                                            display_equipped = "YES";
                                        } else {
                                            display_equipped = "";
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
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    if ui.button("Equip/Unequip (e)").clicked() {
                        println!("Equipped {:?}", self.inventory_selected);
                    }
                    if ui.button("Drop (d)").clicked() {
                        println!("Dropped {:?}", self.inventory_selected);
                    };
                    if ui.button("Combine (c)").clicked() {
                        println!("Combined {:?}", self.inventory_selected);
                    };
                    if ui.button("What is this? (/)").clicked() {
                        println!("What is {:?}", self.inventory_selected);
                    }
                });
            });
    }

    pub fn render(&mut self, sim: &crate::world::World, memory: &crate::world::Memory) {
        egui_macroquad::ui(|egui_ctx| {
            if self.ui_selected {
                self.render_inventory(egui_ctx, sim);
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
                        TileKind::Stairs => ('>', LIGHTGRAY),
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
                                let equip_def = sim.get_equipmentkind_info(ek.kind);
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
            self.render_side_ui(egui_ctx, sim, screen_width() * (1. / 4.));
        });

        egui_macroquad::draw();
    }

    fn render_side_ui(
        &self,
        egui_ctx: &egui::Context,
        sim: &crate::world::World,
        right_offset: f32,
    ) {
        let game_width = screen_width() - right_offset;
        let game_size = game_width.min(screen_height());

        let offset_x = (screen_width() + game_size - right_offset) / 2. + 5.;
        let offset_y = (screen_height() - game_size) / 2. + 10.;

        let panel_height = game_size - 20.;
        let panel_width = screen_width() - offset_x - 10.0;

        let mobs_lower_bound = offset_y + panel_height * 0.3;

        let game_scale = screen_width().min(screen_height());
        let scale_factor = miniquad::window::dpi_scale() * (game_scale / 1200.);

        let font_scale_base = 22. * scale_factor;
        let font_scale_details = 18. * scale_factor;

        let to_egui = |c: &Color| {
            let color = macroquad::color::Color::from(*c);
            let [r, g, b, _a] = color.into();
            Color32::from_rgb(r, g, b)
        };

        let pokedex_width = panel_width * miniquad::window::dpi_scale();
        let pokedex_height = (mobs_lower_bound - offset_y) * miniquad::window::dpi_scale();
        egui::Window::new("Pokédex")
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::Vec2::new(pokedex_width, pokedex_height))
            .max_size(egui::Vec2::new(pokedex_width, pokedex_height))
            .fixed_pos(egui::Pos2::new(
                offset_x * miniquad::window::dpi_scale(),
                offset_y * miniquad::window::dpi_scale(),
            ))
            .show(egui_ctx, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::style::Margin::symmetric(
                        pokedex_width * 0.02,
                        pokedex_height * miniquad::window::dpi_scale() * 0.02,
                    ))
                    .show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for (i, mob) in sim.get_visible_mobs().iter().enumerate() {
                                    let damage = mob.damage;
                                    let mob_kind = mob.kind;
                                    let mob_kind_def = sim.get_mobkind_info(mob_kind);

                                    let MobKindInfo {
                                        char,
                                        color,
                                        name,
                                        attack_type,
                                        type1,
                                        type2,
                                        description,
                                        level,
                                        ..
                                    } = mob_kind_def;

                                    let mut job = egui::text::LayoutJob::default();
                                    job.append(
                                        char,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_base * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: to_egui(color),
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        &format!(" - {name}\n    "),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_base * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: to_egui(color),
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        &format!("{} ", type1),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_details * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: to_egui(&type1.get_color()),
                                            ..Default::default()
                                        },
                                    );
                                    if let Some(type2) = type2 {
                                        job.append(
                                            &format!("{} ", type2),
                                            0.0,
                                            egui::TextFormat {
                                                font_id: egui::FontId::new(
                                                    font_scale_details * scale_factor,
                                                    egui::FontFamily::Proportional,
                                                ),
                                                color: to_egui(&type2.get_color()),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    job.append(
                                        "| ATT ",
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_details * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: Color32::WHITE,
                                            ..Default::default()
                                        },
                                    );

                                    job.append(
                                        &format!("{} ", attack_type),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_details * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: to_egui(&attack_type.get_color()),
                                            ..Default::default()
                                        },
                                    );

                                    job.append(
                                        &format!("| Level {} | HP ", level),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_details * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: Color32::WHITE,
                                            ..Default::default()
                                        },
                                    );

                                    let max_hp = mob_kind_def.max_hp();
                                    let hp = max_hp - damage;
                                    let hp_color = if hp < max_hp / 5 {
                                        Color32::RED
                                    } else if hp < max_hp / 2 {
                                        Color32::YELLOW
                                    } else {
                                        Color32::WHITE
                                    };
                                    job.append(
                                        &format!("{}", hp),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_details * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: hp_color,
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        &format!("/ {}", max_hp),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::new(
                                                font_scale_details * scale_factor,
                                                egui::FontFamily::Proportional,
                                            ),
                                            color: Color32::WHITE,
                                            ..Default::default()
                                        },
                                    );

                                    ui.label(job);

                                    ui.push_id(i, |ui| {
                                        egui::CollapsingHeader::new("Details...").show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(description).italics(),
                                                )
                                                .wrap(true),
                                            )
                                        });
                                    });
                                    ui.separator();
                                }
                            });
                        });
                    });
            });

        let log_upper_bound = offset_y + panel_height * 0.35;
        let log_lower_bound = offset_y + panel_height * 0.85;
        let log_height = log_lower_bound - log_upper_bound;

        let log_width = panel_width * miniquad::window::dpi_scale();
        let log_height = log_height * miniquad::window::dpi_scale();

        egui::Window::new("Logs")
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::Vec2::new(log_width, log_height))
            .fixed_pos(egui::Pos2::new(
                offset_x * miniquad::window::dpi_scale(),
                log_upper_bound * miniquad::window::dpi_scale(),
            ))
            .show(egui_ctx, |ui| {
                ui.set_height(ui.available_height());
                ui.set_width(ui.available_width());
                egui::Frame::none()
                    .inner_margin(egui::style::Margin::symmetric(
                        log_width * 0.02,
                        log_height * 0.02,
                    ))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let start_index = sim.log.len() as i64 - 100;
                                let start_index = (start_index.max(0)) as usize;
                                for log_entry in sim.log.iter().skip(start_index) {
                                    let mut job = egui::text::LayoutJob::default();
                                    for (log_entry_str, log_entry_color) in log_entry {
                                        job.append(
                                            log_entry_str,
                                            0.0,
                                            egui::TextFormat {
                                                font_id: egui::FontId::new(
                                                    font_scale_base * scale_factor,
                                                    egui::FontFamily::Proportional,
                                                ),
                                                color: to_egui(log_entry_color),
                                                ..Default::default()
                                            },
                                        );
                                    }
                                    ui.label(job);
                                }
                            });
                    });
            });
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
