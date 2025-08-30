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
            let padding = (3.0 * miniquad::window::dpi_scale()) as i8;
            egui::Window::new("Choose your character")
                .resizable(false)
                .collapsible(false)
                .min_width(width / 2.0)
                .max_width(width)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(padding, padding))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(self.defs.setting_desc.clone().unwrap_or("".into()));
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        for (i, c) in self.defs.characters.iter().enumerate() {
                                            let name_text = egui::RichText::new(&c.name).heading();
                                            ui.radio_value(&mut self.selected, Some(i), name_text);
                                        }
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
