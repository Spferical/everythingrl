use egui::Label;
use macroquad::prelude::*;

use crate::net::{Character, GameDefs};

pub struct Chargen {
    defs: GameDefs,
    selected: Option<usize>,
}

impl Chargen {
    pub fn new(defs: GameDefs) -> Chargen {
        Chargen {
            defs,
            selected: Some(0),
        }
    }

    pub fn tick(&mut self) -> Option<Character> {
        let mut choice = None;
        egui_macroquad::ui(|egui_ctx| {
            let width = screen_width() * miniquad::window::dpi_scale();
            let padding = 3.0 * miniquad::window::dpi_scale();
            egui::Window::new("Choose your character")
                .resizable(false)
                .collapsible(false)
                .min_width(width / 2.0)
                .max_width(width)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    egui::Frame::none()
                        .inner_margin(egui::style::Margin::symmetric(padding, padding))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(self.defs.setting_desc.clone().unwrap_or("".into()));
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        egui_extras::TableBuilder::new(ui)
                                            .striped(true)
                                            .column(egui_extras::Column::auto())
                                            .sense(egui::Sense::click())
                                            .header(20.0, |mut header| {
                                                header.col(|_ui| {});
                                            })
                                            .body(|mut body| {
                                                for (i, c) in
                                                    self.defs.characters.iter().enumerate()
                                                {
                                                    body.row(20.0, |mut row| {
                                                        row.set_selected(self.selected == Some(i));
                                                        row.col(|ui| {
                                                            ui.add(
                                                                Label::new(
                                                                    egui::RichText::new(&c.name)
                                                                        .heading(),
                                                                )
                                                                .wrap(false),
                                                            );
                                                        });
                                                        if row.response().clicked() {
                                                            self.selected = Some(i)
                                                        }
                                                    });
                                                }
                                            });
                                    });
                                    ui.separator();
                                    ui.vertical(|ui| {
                                        if let Some(i) = self.selected {
                                            if let Some(c) = self.defs.characters.get(i) {
                                                ui.add(Label::new(
                                                    egui::RichText::new(&c.name).heading(),
                                                ));
                                                ui.separator();

                                                ui.label(&c.backstory);
                                                for name in &c.starting_items {
                                                    if let Some(item_def) = self
                                                        .defs
                                                        .items
                                                        .iter()
                                                        .find(|i| &i.name == name)
                                                    {
                                                        ui.label(
                                                            egui::RichText::new(name)
                                                                .color(item_def.ty.get_color()),
                                                        );
                                                    }
                                                }
                                                if ui.button("Play").clicked() {
                                                    choice = Some(c.clone());
                                                }
                                            }
                                        }
                                    });
                                });
                            });
                        });
                });
        });
        egui_macroquad::draw();
        choice
    }
}
