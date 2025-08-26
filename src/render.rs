use std::collections::HashSet;

use ::rand::rngs::StdRng;
use ::rand::SeedableRng;
use egui::{Color32, FontId, RichText};
use macroquad::prelude::*;
use macroquad::text::Font;
use noise::{NoiseFn, Perlin};
use rand_distr::{Distribution, Normal};
use rogue_algebra::{Pos, Rect};

use crate::net::{Color, ItemKind};
use crate::world::{Item, MobKindInfo, TileKind};

pub const FOV_BG: macroquad::color::Color = DARKGRAY;
pub const OOS_BG: macroquad::color::Color = BLACK;

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
    pub user_scale_factor: f32,
    tmp_scale_factor: f32,
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
            user_scale_factor: 1.0,
            tmp_scale_factor: 1.0,
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
            .fixed_size(
                egui::Vec2::new((screen_width() / 2.) * miniquad::window::dpi_scale(),
                                (screen_height() / 2.) * miniquad::window::dpi_scale()))
            .show(egui_ctx, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        let mut basic_label = |start_text: &str, end_text: &str| {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                start_text,
                                0.0,
                                egui::TextFormat {
                                    font_id: self.get_base_font(),
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
                                    font_id: self.get_base_font(),
                                    color: Color::White.into(),
                                    ..Default::default()
                                },
                            );
                            ui.label(job);
                        };
                        basic_label("hjkl or arrows", "Movement");
                        basic_label("SHIFT + move", "Fire weapon");
                        basic_label("i", "Show inventory.");
                        basic_label(".", "Wait a turn.");
                        basic_label(",", "Pick up item.");
                        basic_label("0-9", "Multi-select inventory item");
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
                                    ui.label(row_index.to_string());
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

    pub fn render(&mut self, sim: &crate::world::World, memory: &crate::world::Memory) {
        egui_macroquad::ui(|egui_ctx| {
            if self.ui_selected {
                self.render_inventory(egui_ctx, sim);
            }
            if self.help_selected {
                self.render_help(egui_ctx);
            }
            let bottom_bar_height = 32.0 * self.scale_factor();
            let player_pos = sim.get_player_pos();
            let grid_rect =
                Rect::new_centered(player_pos, self.grid_size as i32, self.grid_size as i32);
            let upper_left = grid_rect.topleft();

            // Handle smooth camera movement.
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
            self.render_glyphs(
                &glyphs,
                screen_width() * (1. / 4.),
                bottom_bar_height,
                upper_left,
            );

            // Draw side panel UI.
            self.render_side_ui(egui_ctx, sim, screen_width() * (1. / 4.));
            self.render_bottom_bar(egui_ctx, sim, bottom_bar_height);
        });

        egui_macroquad::draw();
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
                                let font = self.get_base_font();
                                ui.label(RichText::new("HEALTH:").color(white).font(font.clone()));
                                let player_hp = usize::saturating_sub(
                                    crate::world::PLAYER_MAX_HEALTH,
                                    sim.player_damage,
                                );
                                let red = &Color::Red;
                                ui.label(
                                    RichText::new(format!("{player_hp}"))
                                        .color(red)
                                        .font(font.clone()),
                                );
                                ui.separator();
                                ui.label(
                                    RichText::new("FONT SCALE:").color(white).font(font.clone()),
                                );
                                let response = ui.add(
                                    egui::Slider::new(&mut self.tmp_scale_factor, 0.5..=3.0)
                                        .logarithmic(true),
                                );
                                if response.drag_stopped() {
                                    self.user_scale_factor = self.tmp_scale_factor;
                                }
                                if ui.button("Help (q)").clicked() {
                                    self.toggle_help();
                                }
                            },
                        );
                    });
            });
    }

    fn scale_factor(&self) -> f32 {
        let game_scale = screen_width().min(screen_height());
        miniquad::window::dpi_scale() * game_scale / 1200.0 * self.user_scale_factor
    }

    fn get_base_font(&self) -> FontId {
        egui::FontId::new(22.0 * self.scale_factor(), egui::FontFamily::Proportional)
    }
    fn get_details_font(&self) -> FontId {
        egui::FontId::new(18.0 * self.scale_factor(), egui::FontFamily::Proportional)
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
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (pokedex_width * 0.02) as i8,
                        (pokedex_height * miniquad::window::dpi_scale() * 0.02) as i8,
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
                                            font_id: self.get_base_font(),
                                            color: color.into(),
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        &format!(" - {name}\n    "),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: self.get_base_font(),
                                            color: color.into(),
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        &format!("{} ", type1),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: self.get_base_font(),
                                            color: type1.get_color().into(),
                                            ..Default::default()
                                        },
                                    );
                                    if let Some(type2) = type2 {
                                        job.append(
                                            &format!("{} ", type2),
                                            0.0,
                                            egui::TextFormat {
                                                font_id: self.get_details_font(),
                                                color: type2.get_color().into(),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    job.append(
                                        "| ATT ",
                                        0.0,
                                        egui::TextFormat {
                                            font_id: self.get_details_font(),
                                            color: Color32::WHITE,
                                            ..Default::default()
                                        },
                                    );

                                    job.append(
                                        &format!("{} ", attack_type),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: self.get_details_font(),
                                            color: attack_type.get_color().into(),
                                            ..Default::default()
                                        },
                                    );

                                    job.append(
                                        &format!("| Level {} | HP ", level),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: self.get_details_font(),
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
                                            font_id: self.get_details_font(),
                                            color: hp_color,
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        &format!("/ {}", max_hp),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: self.get_details_font(),
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

        let log_upper_bound = offset_y + panel_height * 0.35;
        let log_lower_bound = offset_y + panel_height * 0.9;
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
                                                font_id: self.get_base_font(),
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
        right_offset: f32,
        bottom_offset: f32,
        upper_left: Pos,
    ) {
        let glyphs = glyphs
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
                bg: glyph.bg,
                location: (pos.0 as usize, pos.1 as usize),
                layer: glyph.layer,
            })
            .collect::<Vec<_>>();

        let width = screen_width() - right_offset;
        let height = screen_height() - bottom_offset;
        let game_size = width.min(height);
        let offset_x = (screen_width() - game_size - right_offset) / 2. + 10.;
        let offset_y = (screen_height() - game_size) / 2. + 10.;
        let sq_size = (screen_height() - offset_y * 2.) / self.grid_size as f32;

        let delta = self.camera_delta.unwrap_or((0.0, 0.0));
        let delta = (delta.0 * sq_size, delta.1 * sq_size);

        let translate_coords = |x, y, font_offset| {
            let off = if font_offset {
                (0.25, 0.75)
            } else {
                (0.5, 0.6)
            };
            (
                delta.0 + offset_x + sq_size * (x as f32 + off.0),
                delta.1 + offset_y + sq_size * (y as f32 + off.1),
            )
        };

        // First, set the actual background of the grid to black
        draw_rectangle(offset_x, offset_y, game_size - 20., game_size - 20., BLACK);

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
                if x >= offset_x
                    && x < game_size + offset_x - 20.0
                    && y >= offset_y
                    && y < game_size + offset_y - 20.0
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
                        let cell = *cell - upper_left;

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
