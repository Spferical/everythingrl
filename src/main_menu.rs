use chrono::TimeZone;
use macroquad::prelude::*;

use crate::save::Defs;

pub enum Choice {
    Play,
    Load(Defs),
}

pub struct Menu {
    defs: Option<Vec<Defs>>,
}

impl Menu {
    pub fn new() -> Menu {
        Menu { defs: None }
    }

    pub fn load_popup(&mut self, ui: &mut egui::Ui) -> Option<Defs> {
        let target_width = screen_width() * miniquad::window::dpi_scale() / 3.0;
        let mut ret = None;

        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    let defs = self.defs.get_or_insert_with(crate::save::load_defs);
                    if defs.is_empty() {
                        ui.label("No existing saved game definitions found");
                    } else {
                        egui_extras::TableBuilder::new(ui)
                            .striped(true)
                            .column(egui_extras::Column::auto())
                            .column(egui_extras::Column::initial(target_width))
                            .column(egui_extras::Column::auto())
                            .resizable(true)
                            .auto_shrink([false, true])
                            .header(20.0, |mut header| {
                                header.col(|_ui| {});
                                header.col(|ui| {
                                    ui.strong("Theme");
                                });
                                header.col(|ui| {
                                    ui.strong("Created");
                                });
                            })
                            .body(|mut body| {
                                for def in defs {
                                    body.row(20.0, |mut row| {
                                        row.col(|ui| {
                                            if ui.button("Play!").clicked() {
                                                ret = Some(def.clone());
                                            }
                                        });
                                        row.col(|ui| {
                                            ui.add(
                                                egui::Label::new(&def.metadata.theme)
                                                    .truncate(true),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.label(
                                                chrono::Local
                                                    .timestamp_opt(def.metadata.created, 0)
                                                    .single()
                                                    .map(|t| {
                                                        t.format("%Y-%m-%d %H:%M:%S").to_string()
                                                    })
                                                    .unwrap_or("".into()),
                                            );
                                        });
                                    });
                                }
                            });
                    }
                    if ui.button("Close").clicked() {
                        ui.close_menu();
                    }
                })
            });
        });
        ret
    }

    pub fn tick(&mut self) -> Option<Choice> {
        let mut choice = None;
        egui_macroquad::ui(|egui_ctx| {
            let width = screen_width() * miniquad::window::dpi_scale();
            let padding = 3.0 * miniquad::window::dpi_scale();
            egui::Window::new("EverythingRL")
                .resizable(false)
                .collapsible(false)
                .min_width(width / 2.0)
                .max_width(width)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    egui::Frame::none()
                        .inner_margin(egui::style::Margin::symmetric(padding, padding))
                        .show(ui, |ui| {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                if ui.button("Play in a new world").clicked() {
                                    choice = Some(Choice::Play);
                                }
                                ui.menu_button("Load an existing world", |ui| {
                                    // TODO: when we rev the game data format,
                                    // we'll need to indicate/fail somewhere.
                                    if let Some(def) = self.load_popup(ui) {
                                        choice = Some(Choice::Load(def));
                                    }
                                });
                            });
                        });
                });
        });
        egui_macroquad::draw();
        choice
    }
}
