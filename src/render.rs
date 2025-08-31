use std::collections::HashSet;

use ::rand::rngs::StdRng;
use ::rand::SeedableRng;
use egui::{Color32, RichText};
use macroquad::prelude::*;
use macroquad::text::Font;
use noise::{NoiseFn, Perlin};
use rand_distr::{Distribution, Normal};
use rogue_algebra::{Pos, Rect};

use crate::net::{Color, ItemKind};
use crate::world::{Item, MobKindInfo, TileKind};
use crate::INVENTORY_KEYS;

pub const FOV_BG: macroquad::color::Color = DARKGRAY;
pub const OOS_BG: macroquad::color::Color = BLACK;

const SIDEBAR_FRACTION: f32 = 0.33;

#[derive(Clone, Debug)]
pub struct ShotAnimation {
    pub cells: Vec<Pos>,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub enum Animation {
    Shot(ShotAnimation),
}

#[derive(Clone, Debug)]
pub struct AnimationState {
    time_elapsed: f32,
    duration: f32,
    animation: Animation,
}

impl AnimationState {
    pub fn new(animation: Animation, duration: f32) -> AnimationState {
        AnimationState {
            time_elapsed: 0.,
            duration,
            animation,
        }
    }
}

#[derive(Clone, Copy, Hash)]
pub enum UiButton {
    Equip,
    Drop,
    Craft,
    Inspect,
}

pub struct Ui {
    grid_size: usize,
    font: Font,
    pub ui_selected: bool,
    pub help_selected: bool,
    camera_delta: Option<(f32, f32)>,
    last_upper_left: Option<Pos>,
    pub inventory_selected: HashSet<usize>,
    animations: Vec<AnimationState>,

    pub ui_button: Option<UiButton>,
}

#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    character: char,
    color: macroquad::color::Color,
    bg: macroquad::color::Color,
    location: (usize, usize),
    layer: usize,
}
#[derive(Hash, Debug, Clone, Copy)]
pub enum ItemCondition {
    New,
    LikeNew,
    VeryGood,
    Good,
    Acceptable,
    Poor,
}

fn condition_color(condition: ItemCondition) -> Color {
    match condition {
        ItemCondition::Poor => Color::Red,
        ItemCondition::Acceptable => Color::Brown,
        ItemCondition::Good => Color::Gray,
        ItemCondition::VeryGood => Color::Green,
        ItemCondition::LikeNew => Color::Gold,
        ItemCondition::New => Color::White,
    }
}

fn get_item_condition(durability: usize) -> ItemCondition {
    match durability {
        0..=2 => ItemCondition::Poor,
        3..=6 => ItemCondition::Acceptable,
        7..=10 => ItemCondition::Good,
        11..=14 => ItemCondition::VeryGood,
        15..=18 => ItemCondition::LikeNew,
        _ => ItemCondition::New,
    }
}

fn normpdf(x: f32, mean: f32, std: f32) -> f32 {
    let var = std * std;
    let denom = f32::sqrt(2. * std::f32::consts::PI * var);
    let num = f32::exp(-f32::powf(x - mean, 2.) / (2. * var));
    num / denom
}

pub fn render_top_bar(egui_ctx: &egui::Context, scale_factor: &mut f32) {
    egui::TopBottomPanel::top("top_bar")
        .exact_height(32.0)
        .show(egui_ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(RichText::new("UI SCALE:").color(Color::White));
                let response = ui.add(egui::Slider::new(scale_factor, 0.5..=3.0).logarithmic(true));
                if response.drag_stopped() && *scale_factor != egui_ctx.zoom_factor() {
                    miniquad::info!("{}", format_args!("scale factor {scale_factor}"));
                    egui_ctx.set_zoom_factor(*scale_factor);
                }
            });
        });
}

impl Ui {
    pub fn new(grid_size: Option<usize>, font: Font) -> Ui {
        Ui {
            grid_size: grid_size.unwrap_or(32),
            font,
            ui_selected: true,
            help_selected: false,
            camera_delta: None,
            last_upper_left: None,
            inventory_selected: HashSet::new(),
            animations: Vec::new(),
            ui_button: None,
        }
    }

    pub fn add_animation(&mut self, animation: AnimationState) {
        self.animations.push(animation);
    }

    pub fn toggle_ui(&mut self) {
        self.ui_selected = !self.ui_selected;
    }

    pub fn toggle_help(&mut self) {
        self.help_selected = !self.help_selected;
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

    fn render_help(&mut self, egui_ctx: &egui::Context) {
        egui::Window::new("Help")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
            .show(egui_ctx, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        let mut basic_label = |start_text: &str, end_text: &str| {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                start_text,
                                0.0,
                                egui::TextFormat {
                                    color: Color::Gold.into(),
                                    italics: true,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &format!("{}{}", " ".repeat(17 - start_text.len()), end_text)
                                    .to_owned(),
                                0.0,
                                egui::TextFormat {
                                    color: Color::White.into(),
                                    ..Default::default()
                                },
                            );
                            ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                        };
                        basic_label("hjkl or arrows", "Movement");
                        basic_label("SHIFT + move", "Fire weapon");
                        basic_label("i", "Show inventory.");
                        basic_label(".", "Wait a turn.");
                        basic_label(",", "Pick up item.");
                        basic_label("0-9, misc", "Multi-select inventory item");
                        basic_label("e", "Equip/eat selected item(s).");
                        basic_label("d", "Drop selected item(s).");
                        basic_label("c", "Combine/cook selected item(s).");
                        basic_label("; or /", "Inspect selected item(s).");
                        basic_label("q or ?", "Request help.");
                        ui.separator();
                        ui.label("Click on 'details' in the upper right panel to get more info about that monster.");
                    });
            });
    }

    fn render_inventory(&mut self, egui_ctx: &egui::Context, sim: &crate::world::World) {
        egui::Window::new("Inventory")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0))
            .show(egui_ctx, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
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
                        .column(egui_extras::Column::auto())
                        .column(egui_extras::Column::auto())
                        .column(egui_extras::Column::auto())
                        .sense(egui::Sense::click());
                    table
                        .header(text_height, |mut header| {
                            header.col(|ui| {
                                ui.strong("Key");
                            });
                            header.col(|_| {});
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
                                ui.strong("Kind");
                            });
                            header.col(|ui| {
                                ui.strong("Equipped");
                            });
                            header.col(|ui| {
                                ui.strong("Condition");
                            });
                            header.col(|ui| {
                                ui.strong("Modifiers");
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
                                let cond;
                                let mut types = vec![];
                                let mut modifiers = vec![];
                                match &slot.item {
                                    Item::PendingCraft(..) => {
                                        name = "Crafting in progress...".into();
                                        display_slot = "";
                                        display_equipped = "";
                                        level = "".into();
                                        cond = ItemCondition::New;
                                    }
                                    Item::Instance(item) => {
                                        name = match item.info.kind {
                                            ItemKind::Food => format!(
                                                "{} ({}hp)",
                                                &item.info.name,
                                                item.info.get_heal_amount(&[]).abs()
                                            ),
                                            _ => item.info.name.clone(),
                                        };
                                        types.push(item.info.ty);
                                        for modifier in &item.info.modifiers {
                                            modifiers.push(format!("{:?}", modifier));
                                        }
                                        display_slot = match item.info.kind {
                                            ItemKind::MeleeWeapon => "Melee",
                                            ItemKind::RangedWeapon => "Ranged",
                                            ItemKind::Armor => "Equipment",
                                            ItemKind::Food => "Food",
                                        };
                                        if slot.equipped {
                                            display_equipped = "YES";
                                        } else {
                                            display_equipped = "";
                                        }
                                        level = item.info.level.to_string();
                                        cond = get_item_condition(item.item_durability);
                                    }
                                }

                                row.col(|ui| {
                                    let key_char = INVENTORY_KEYS
                                        .get(row_index)
                                        .map(|(c, _)| c)
                                        .unwrap_or(&'?');
                                    ui.label(key_char.to_string());
                                });
                                row.col(|ui| {
                                    let (ch, color) = get_item_glyph(&slot.item);
                                    ui.colored_label(color, ch.to_string());
                                });
                                row.col(|ui| {
                                    ui.label(name);
                                });
                                row.col(|ui| {
                                    for ty in types {
                                        ui.colored_label(ty.get_color(), ty.to_string());
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
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:?}", cond))
                                            .color(condition_color(cond)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(modifiers.join(", "));
                                });

                                self.toggle_row_selection(row_index, &row.response());
                            });
                        });
                });
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    if ui.button("Equip/Unequip/Eat (e)").clicked() {
                        self.ui_button = Some(UiButton::Equip);
                        println!("Equipped {:?}", self.inventory_selected);
                    }
                    if ui.button("Drop (d)").clicked() {
                        self.ui_button = Some(UiButton::Drop);
                        println!("Dropped {:?}", self.inventory_selected);
                    };
                    if ui.button("Combine/Cook (c)").clicked() {
                        self.ui_button = Some(UiButton::Craft);
                        println!("Combined {:?}", self.inventory_selected);
                    };
                    if ui.button("What is this? (; or /)").clicked() {
                        self.ui_button = Some(UiButton::Inspect);
                        println!("What is {:?}", self.inventory_selected);
                    }
                });
            });
    }

    pub fn render(
        &mut self,
        sim: &crate::world::World,
        memory: &crate::world::Memory,
        egui_ctx: &egui::Context,
    ) {
        if self.ui_selected {
            self.render_inventory(egui_ctx, sim);
        }
        if self.help_selected {
            self.render_help(egui_ctx);
        }
        let bottom_bar_height = 32.0;
        let player_pos = sim.get_player_pos();
        let grid_rect =
            Rect::new_centered(player_pos, self.grid_size as i32, self.grid_size as i32);
        let world_offset = grid_rect.bottomleft();

        // Handle smooth camera movement.
        self.camera_delta = Some(match self.camera_delta {
            None => (0., 0.),
            Some(old_delta) => {
                let added_delta = match self.last_upper_left {
                    None => (0., 0.),
                    Some(old_upper_left) => (
                        (world_offset.x - old_upper_left.x) as f32,
                        (world_offset.y - old_upper_left.y) as f32,
                    ),
                };
                (
                    (old_delta.0 + added_delta.0) * 0.9,
                    (old_delta.1 + added_delta.1) * 0.9,
                )
            }
        });
        self.last_upper_left = Some(world_offset);

        // Render mobs.
        let mut glyphs = vec![Glyph {
            character: '@',
            color: WHITE,
            bg: FOV_BG,
            location: (player_pos.x as usize, player_pos.y as usize),
            layer: 2,
        }];
        let fov = sim.get_fov(sim.player_pos);
        for pos in grid_rect {
            let tile = &memory.tile_map[pos];
            let bg = if fov.contains(&pos) { FOV_BG } else { OOS_BG };
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
                    bg,
                    location: (pos.x as usize, pos.y as usize),
                    layer: 0,
                });
                if let Some(ref item) = tile.item {
                    let (character, color) = get_item_glyph(item);
                    glyphs.push(Glyph {
                        character,
                        color: color.into(),
                        bg,
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
                    bg,
                    location: (pos.x as usize, pos.y as usize),
                    layer: 2,
                });
            }
        }
        let bottom_bar_height_mq =
            bottom_bar_height * (screen_height() / egui_ctx.screen_rect().height());
        let glyph_rect = macroquad::math::Rect {
            x: 0.0,
            y: 0.0,
            w: screen_width() * (1.0 - SIDEBAR_FRACTION),
            h: screen_height() - bottom_bar_height_mq,
        };
        self.render_glyphs(&glyphs, glyph_rect, world_offset);

        // Draw side panel UI.
        self.render_side_ui(egui_ctx, sim);
        self.render_bottom_bar(egui_ctx, sim, bottom_bar_height);
    }

    fn render_bottom_bar(
        &mut self,
        egui_ctx: &egui::Context,
        sim: &crate::world::World,
        height: f32,
    ) {
        egui::TopBottomPanel::bottom("bottom_bar")
            .exact_height(height)
            .show(egui_ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (height * 0.5) as i8,
                        (height * 0.1) as i8,
                    ))
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::left_to_right(egui::Align::Center)
                                .with_cross_align(egui::Align::Center),
                            |ui| {
                                let white = &Color::White;
                                ui.label(RichText::new("HEALTH:").color(white));
                                let player_hp = usize::saturating_sub(
                                    crate::world::PLAYER_MAX_HEALTH,
                                    sim.player_damage,
                                );
                                let red = &Color::Red;
                                ui.label(RichText::new(format!("{player_hp}")).color(red));
                                ui.separator();
                                if ui.button("Help (q)").clicked() {
                                    self.toggle_help();
                                }
                            },
                        );
                    });
            });
    }

    fn render_side_ui(&self, egui_ctx: &egui::Context, sim: &crate::world::World) {
        let egui_width = egui_ctx.available_rect().width();
        let egui_height = egui_ctx.available_rect().height();
        let offset_x = egui_width * (1.0 - SIDEBAR_FRACTION);
        let offset_y = 10.;

        let panel_height = egui_height - 20.;
        let panel_width = egui_width - offset_x - 10.0;

        let mobs_lower_bound = offset_y + panel_height * 0.3;

        let pokedex_width = panel_width;
        let pokedex_height = mobs_lower_bound - offset_y;

        let log_upper_bound = offset_y + panel_height * 0.35;
        let log_lower_bound = offset_y + panel_height * 0.9;
        let log_height = log_lower_bound - log_upper_bound;
        let log_width = panel_width;

        miniquad::info!("{}", format_args!(
            "EGUI dims: total {egui_width} {egui_height} offset {offset_x} {offset_y} pokedex {pokedex_width} {pokedex_height}, log {log_width} {log_height}"
        ));

        egui::Window::new("Pokédex")
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::Vec2::new(pokedex_width, pokedex_height))
            .fixed_pos(egui::Pos2::new(offset_x, offset_y))
            .show(egui_ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (pokedex_width * 0.02) as i8,
                        (pokedex_height * 0.02) as i8,
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
                                    let base_format = egui::TextFormat {
                                        color: color.into(),
                                        ..Default::default()
                                    };
                                    let with_color =
                                        |format: &egui::TextFormat, color| egui::TextFormat {
                                            color,
                                            ..format.clone()
                                        };
                                    let details_format = egui::TextFormat {
                                        color: Color32::WHITE,
                                        ..Default::default()
                                    };
                                    job.append(char, 0.0, base_format.clone());
                                    job.append(
                                        &format!(" - {name}\n    "),
                                        0.0,
                                        base_format.clone(),
                                    );
                                    let type_format =
                                        with_color(&base_format, type1.get_color().into());
                                    job.append(&format!("{} ", type1), 0.0, type_format.clone());
                                    if let Some(type2) = type2 {
                                        let type2_format =
                                            with_color(&base_format, type2.get_color().into());
                                        job.append(&format!("{} ", type2), 0.0, type2_format);
                                    }

                                    job.append("| ATT ", 0.0, details_format.clone());

                                    let attack_type_format =
                                        with_color(&details_format, attack_type.get_color().into());
                                    job.append(
                                        &format!("{} ", attack_type),
                                        0.0,
                                        attack_type_format,
                                    );

                                    job.append(
                                        &format!("| Level {} | HP ", level),
                                        0.0,
                                        details_format.clone(),
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
                                    let hp_format = with_color(&details_format, hp_color);
                                    job.append(&format!("{}", hp), 0.0, hp_format);
                                    job.append(
                                        &format!("/ {}", max_hp),
                                        0.0,
                                        details_format.clone(),
                                    );

                                    ui.label(job);

                                    if !mob_kind_def.modifiers.is_empty() {
                                        let mut modifier_job = egui::text::LayoutJob::default();
                                        for modifier in mob_kind_def.modifiers.iter() {
                                            let modifier_format = with_color(
                                                &details_format,
                                                modifier.color().into(),
                                            );
                                            modifier_job.append(
                                                &format!("{:?} ", modifier),
                                                0.0,
                                                modifier_format,
                                            );
                                        }
                                        for status in mob.status_effects.iter() {
                                            let format = with_color(
                                                &details_format,
                                                status.effect.color().into(),
                                            );
                                            modifier_job.append(
                                                &format!("{:?} ", status.effect),
                                                0.0,
                                                format,
                                            );
                                        }
                                        ui.label(modifier_job);
                                    }

                                    ui.push_id(i, |ui| {
                                        egui::CollapsingHeader::new("Details...").show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(description).italics(),
                                                )
                                                .wrap(),
                                            )
                                        });
                                    });
                                    ui.separator();
                                }
                            });
                        });
                    });
            });

        egui::Window::new("Logs")
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::Vec2::new(log_width, log_height))
            .fixed_pos(egui::Pos2::new(offset_x, log_upper_bound))
            .show(egui_ctx, |ui| {
                ui.set_height(ui.available_height());
                ui.set_width(ui.available_width());
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (log_width * 0.02) as i8,
                        (log_height * 0.02) as i8,
                    ))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let start_index = sim.log.len() as i64 - 100;
                                let start_index = (start_index.max(0)) as usize;
                                let last_step = sim.log.iter().map(|(_, step)| step).max();
                                for (log_entry, step) in sim.log.iter().skip(start_index) {
                                    let is_new = match last_step {
                                        None => true,
                                        Some(last_step) => *step >= *last_step,
                                    };
                                    let mut job = egui::text::LayoutJob::default();
                                    for (log_entry_str, log_entry_color) in log_entry {
                                        let mut color = egui::Color32::from(log_entry_color);
                                        if !is_new {
                                            color = color.gamma_multiply(0.5);
                                        }
                                        job.append(
                                            log_entry_str,
                                            0.0,
                                            egui::TextFormat {
                                                color,
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

    fn render_glyphs(
        &mut self,
        glyphs: &[Glyph],
        mut mq_rect: macroquad::math::Rect,
        world_offset: Pos,
    ) {
        let glyphs = glyphs
            .iter()
            .map(|&glyph| {
                (
                    (
                        glyph.location.0 as i32 - world_offset.x,
                        glyph.location.1 as i32 - world_offset.y,
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
                bg: glyph.bg,
                location: (pos.0 as usize, pos.1 as usize),
                layer: glyph.layer,
            })
            .collect::<Vec<_>>();

        // fit a square into the available screen rect
        if mq_rect.w > mq_rect.h {
            let diff = mq_rect.w - mq_rect.h;
            mq_rect.x += diff / 2.;
            mq_rect.w -= diff;
        } else {
            let diff = mq_rect.h - mq_rect.w;
            mq_rect.y += diff / 2.;
            mq_rect.h -= diff;
        }
        let game_size = mq_rect.w.min(mq_rect.h);
        let sq_size = mq_rect.w / self.grid_size as f32;

        let delta = self.camera_delta.unwrap_or((0.0, 0.0));
        let delta = (delta.0 * sq_size, delta.1 * sq_size);

        let translate_coords = |x, y, font_offset| {
            let off = if font_offset {
                (0.25, 0.75)
            } else {
                (0.5, 0.6)
            };
            (
                delta.0 + mq_rect.x + sq_size * (x as f32 + off.0),
                delta.1 + mq_rect.y + sq_size * (y as f32 + off.1),
            )
        };

        // First, set the actual background of the grid to black
        draw_rectangle(mq_rect.x, mq_rect.y, mq_rect.w, mq_rect.h, BLACK);

        // Quick check to ensure that the foreground replaces the background.
        let mut z_buffer = vec![vec![0; self.grid_size]; self.grid_size];
        for glyph in &glyphs {
            z_buffer[glyph.location.0][glyph.location.1] =
                z_buffer[glyph.location.0][glyph.location.1].max(glyph.layer);
        }

        let mut flicker_rng = StdRng::seed_from_u64(0);
        let flicker_dist = Perlin::new(1);
        for glyph in &glyphs {
            if glyph.layer >= z_buffer[glyph.location.0][glyph.location.1] {
                let (x, y) =
                    translate_coords(glyph.location.0 as i32, glyph.location.1 as i32, true);
                if x >= mq_rect.x
                    && x < game_size + mq_rect.x - 20.0
                    && y >= mq_rect.y
                    && y < game_size + mq_rect.y - 20.0
                {
                    let off_from_center = (
                        glyph.location.0 as i32 - (self.grid_size as i32 / 2),
                        glyph.location.1 as i32 - (self.grid_size as i32 / 2),
                    );
                    let dist_from_center_sq =
                        ((off_from_center.0.pow(2) + off_from_center.1.pow(2)) as f32).powf(0.5);

                    let flicker = flicker_dist.get([
                        get_time() / 2.0,
                        Normal::new(0.0, 1.0).unwrap().sample(&mut flicker_rng),
                    ]) as f32
                        * 0.4
                        + 1.;
                    let attenuation = 1. / (dist_from_center_sq * flicker).clamp(2.0, 4.0);
                    let bg_hsl = macroquad::color::rgb_to_hsl(glyph.bg);
                    let bg_rgb =
                        macroquad::color::hsl_to_rgb(bg_hsl.0, bg_hsl.1, bg_hsl.2 * attenuation);
                    let (sq_x, sq_y) =
                        translate_coords(glyph.location.0 as i32, glyph.location.1 as i32, false);
                    draw_rectangle(
                        sq_x - sq_size / 2.,
                        sq_y - sq_size / 2. - 3.,
                        sq_size,
                        sq_size,
                        bg_rgb,
                    );
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
                    );
                }
            }
        }

        // Handle animations.
        for animation in self.animations.iter_mut() {
            match &animation.animation {
                Animation::Shot(shot_animation) => {
                    let interp = animation.time_elapsed / animation.duration;
                    for (i, cell) in shot_animation.cells.iter().skip(1).enumerate() {
                        let progress = (i as f32 + 0.5) / shot_animation.cells.len() as f32;
                        let intensity = normpdf(interp, progress, 0.05) * 0.1;
                        let intensity = f32::clamp(intensity, 0., 1.);
                        let cell = *cell - world_offset;

                        // Don't render animations out of bounds!
                        if cell.x < 0
                            || cell.x >= self.grid_size as i32
                            || cell.y < 0
                            || cell.y >= self.grid_size as i32
                        {
                            continue;
                        }

                        // Little hack to show blood.
                        let (color, size) = if z_buffer[cell.x as usize][cell.y as usize] == 2 {
                            (Color::Red, 2.0 * intensity * sq_size)
                        } else {
                            (shot_animation.color, intensity * sq_size)
                        };

                        draw_circle(
                            translate_coords(cell.x, cell.y, false).0,
                            translate_coords(cell.x, cell.y, false).1,
                            size,
                            color.into(),
                        );
                    }
                }
            };
            animation.time_elapsed += get_frame_time();
        }
        self.animations.retain(|a| a.time_elapsed < a.duration);
    }
}

fn get_item_glyph(item: &Item) -> (char, Color) {
    match item {
        Item::PendingCraft(..) => ('?', Color::Pink),
        Item::Instance(ii) => {
            let char = match ii.info.kind {
                ItemKind::MeleeWeapon => ')',
                ItemKind::RangedWeapon => '/',
                ItemKind::Armor => '[',
                ItemKind::Food => '%',
            };
            let color = ii.info.ty.get_color();
            (char, color)
        }
    }
}
