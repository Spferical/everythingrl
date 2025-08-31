use chrono::TimeZone;
use macroquad::prelude::*;

use crate::save::Defs;

pub enum Choice {
    Play,
    Load(Defs),
}

pub enum MenuState {
    Main,
    Load,
}

pub struct Menu {
    defs: Option<Vec<Defs>>,
    import_tx: std::sync::mpsc::Sender<Defs>,
    import_rx: std::sync::mpsc::Receiver<Defs>,
    state: MenuState,
}

impl Menu {
    pub fn new() -> Menu {
        let (import_tx, import_rx) = std::sync::mpsc::channel();

        Menu {
            defs: None,
            import_tx,
            import_rx,
            state: MenuState::Main,
        }
    }

    pub fn show_load_menu(&mut self, ui: &mut egui::Ui) -> Option<Defs> {
        let width = ui.available_width() / 2.0;
        let mut ret = None;

        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                if ui.button("Load game from file").clicked() {
                    let tx = self.import_tx.clone();
                    crate::util::spawn(async move {
                        if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                            let data = handle.read().await;
                            match serde_json::from_slice::<Defs>(&data) {
                                Ok(defs) => {
                                    tx.send(defs).ok();
                                }
                                Err(e) => {
                                    // TODO: when we rev the game data format,
                                    // we'll need to indicate/fail somewhere.
                                    macroquad::miniquad::error!("{}", e);
                                }
                            }
                        }
                    })
                }
                if ui.button("Close").clicked() {
                    self.state = MenuState::Main;
                }
                let defs = self.defs.get_or_insert_with(crate::save::load_defs);
                while let Ok(def) = self.import_rx.try_recv() {
                    defs.push(def);
                    crate::save::write_defs(defs);
                }
                if defs.is_empty() {
                    ui.label("No existing saved game definitions found");
                } else {
                    egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .column(egui_extras::Column::auto())
                        .column(egui_extras::Column::auto())
                        .column(egui_extras::Column::initial(width))
                        .column(egui_extras::Column::auto())
                        .resizable(true)
                        .auto_shrink([true, true])
                        .header(20.0, |mut header| {
                            header.col(|_ui| {});
                            header.col(|_ui| {});
                            header.col(|ui| {
                                ui.strong("Theme");
                            });
                            header.col(|ui| {
                                ui.strong("Created");
                            });
                        })
                        .body(|mut body| {
                            for def in defs.iter().rev() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        if ui.button("Play!").clicked() {
                                            ret = Some(def.clone());
                                        }
                                    });
                                    row.col(|ui| {
                                        if ui.button("Download!").clicked() {
                                            crate::util::download_file(
                                                "game.json".into(),
                                                serde_json::to_string_pretty(def).unwrap(),
                                            );
                                        }
                                    });
                                    row.col(|ui| {
                                        ui.add(egui::Label::new(&def.metadata.theme).truncate());
                                    });
                                    row.col(|ui| {
                                        ui.label(
                                            chrono::Local
                                                .timestamp_opt(def.metadata.created, 0)
                                                .single()
                                                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                                                .unwrap_or("".into()),
                                        );
                                    });
                                });
                            }
                        });
                }
                if ui.button("Close").clicked() {
                    self.state = MenuState::Main;
                }
            })
        });
        ret
    }

    pub fn tick(&mut self) -> Option<Choice> {
        let mut choice = None;
        egui_macroquad::ui(|egui_ctx| {
            let width = screen_width() * miniquad::window::dpi_scale();
            let padding = (3.0 * miniquad::window::dpi_scale()) as i8;
            let title = match self.state {
                MenuState::Main => "EverythingRL 005",
                MenuState::Load => "Load an existing EverythingRL world",
            };
            egui::Window::new(title)
                .resizable(false)
                .collapsible(false)
                .min_width(width / 2.0)
                .max_width(width)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(padding, padding))
                        .show(ui, |ui| match self.state {
                            MenuState::Main => {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                    if ui.button("Play in a new world").clicked() {
                                        choice = Some(Choice::Play);
                                    }
                                    if ui.button("Load an existing world").clicked() {
                                        self.state = MenuState::Load;
                                    };
                                });
                            }
                            MenuState::Load => {
                                if let Some(def) = self.show_load_menu(ui) {
                                    choice = Some(Choice::Load(def));
                                }
                            }
                        });
                });
        });
        egui_macroquad::draw();
        choice
    }
}
