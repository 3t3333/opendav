use crate::{OpenDavApp, ActivePage, SimGitTab};
use crate::config::worksheet::{ACCENT_COLOR, SUB_ACCENT_COLOR};

impl OpenDavApp {
    pub fn draw_simgit_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let text_color = if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK };
        let panel_bg = if is_dark { egui::Color32::from_rgb(25, 25, 25) } else { egui::Color32::from_rgb(240, 240, 240) };

        // 1. TOP NAVIGATION BAR
        egui::TopBottomPanel::top("simgit_top_nav")
            .frame(egui::Frame::none().fill(panel_bg).inner_margin(8.0))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("SimGit").strong().color(ACCENT_COLOR).size(24.0));
                    ui.add_space(20.0);

                    // Tabs
                    let tabs = [
                        (SimGitTab::Dashboard, "Dashboard"),
                        (SimGitTab::Setups, "Setups & Commits"),
                        (SimGitTab::Cloud, "Cloud Sync"),
                    ];

                    for (tab, name) in tabs {
                        let is_active = self.simgit_active_tab == tab;
                        let color = if is_active { text_color } else { egui::Color32::GRAY };
                        if ui.selectable_label(is_active, egui::RichText::new(name).color(color).strong().size(18.0)).clicked() {
                            self.simgit_active_tab = tab;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // "Commit Files" is far right
                        if self.simgit_manager.active_project.is_some() {
                            if ui.button(egui::RichText::new("➕ Commit Files").strong().color(ACCENT_COLOR).size(18.0)).clicked() {
                                if let Some(files) = rfd::FileDialog::new()
                                    .add_filter("iRacing Telemetry", &["ibt"])
                                    .pick_files()
                                {
                                    crate::simgit::history::commit_files(&self.simgit_manager.root_dir.join(self.simgit_manager.active_project.as_ref().unwrap()), &files);
                                }
                            }
                        }

                        ui.add_space(10.0);

                        // Workspace Selector / Creator
                        if self.show_new_ws_popup {
                            if ui.button(egui::RichText::new("Confirm").size(16.0)).clicked() {
                                if !self.simgit_new_ws_name.is_empty() {
                                    let _ = self.simgit_manager.create_project(&self.simgit_new_ws_name);
                                    self.simgit_manager.set_active_project(&self.simgit_new_ws_name);
                                    self.simgit_new_ws_name.clear();
                                    self.show_new_ws_popup = false;
                                }
                            }
                            if ui.button(egui::RichText::new("Cancel").size(16.0)).clicked() {
                                self.show_new_ws_popup = false;
                            }
                            ui.add(egui::TextEdit::singleline(&mut self.simgit_new_ws_name).hint_text("Workspace Name..."));
                        } else {
                            if ui.button(egui::RichText::new("➕ New").size(16.0)).clicked() {
                                self.show_new_ws_popup = true;
                            }
                            
                            let mut selected_proj = self.simgit_manager.active_project.clone().unwrap_or_else(|| "Select Workspace".to_string());
                            let projects = self.simgit_manager.list_projects();
                            
                            egui::ComboBox::from_id_source("workspace_selector")
                                .selected_text(egui::RichText::new(&selected_proj).strong().color(text_color).size(16.0))
                                .show_ui(ui, |ui| {
                                    for proj in projects {
                                        if ui.selectable_value(&mut selected_proj, proj.clone(), &proj).changed() {
                                            self.simgit_manager.set_active_project(&proj);
                                        }
                                    }
                                });
                        }
                    });
                });
            });

        ui.add_space(5.0);
        ui.separator();

        // 2. CENTRAL PANEL (Routing)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(15.0))
            .show_inside(ui, |ui| {
                match self.simgit_active_tab {
                    SimGitTab::Dashboard => self.draw_simgit_dashboard(ui, is_dark, text_color),
                    SimGitTab::Setups => self.draw_simgit_setups(ui, is_dark, text_color),
                    SimGitTab::Cloud => {
                        ui.heading("Cloud Sync (Phase 2)");
                    }
                }
            });
    }

    fn draw_simgit_dashboard(&mut self, ui: &mut egui::Ui, is_dark: bool, text_color: egui::Color32) {
        ui.heading(egui::RichText::new("Recent Sessions").strong().color(text_color).size(24.0));
        ui.add_space(15.0);

        if let Some(ref proj_ref) = self.simgit_manager.active_project {
            let proj = proj_ref.clone();
            let root_dir = self.simgit_manager.root_dir.clone();
            let history = crate::simgit::history::get_history(&root_dir.join(&proj));

            if history.is_empty() {
                ui.label(egui::RichText::new("No sessions recorded. Commit some `.ibt` files!").color(egui::Color32::GRAY));
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for entry in history.iter().rev() { // Newest first
                            let card_bg = if is_dark { egui::Color32::from_rgb(40, 40, 40) } else { egui::Color32::from_rgb(220, 220, 220) };
                            
                            let current_proj = proj.clone();
                            let current_root = root_dir.clone();
                            
                            egui::Frame::none()
                                .fill(card_bg)
                                .rounding(12.0)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)))
                                .inner_margin(0.0)
                                .show(ui, |ui| {
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(250.0, 150.0), egui::Sense::hover());
                                    
                                    // 1. Draw Background Track Map
                                    if let Some(tid) = entry.track_id {
                                        if !self.simgit_track_maps.contains_key(&tid) {
                                            let json_path = std::env::current_dir().unwrap().join("exports").join("track_maps").join(format!("{}.json", tid));
                                            if json_path.exists() {
                                                if let Ok(json_str) = std::fs::read_to_string(&json_path) {
                                                    if let Ok(segments) = serde_json::from_str::<Vec<Vec<[f64; 2]>>>(&json_str) {
                                                        self.simgit_track_maps.insert(tid, segments);
                                                    }
                                                }
                                            }
                                        }

                                        let mut plot_ui_builder = ui.new_child(
                                            egui::UiBuilder::new()
                                                .max_rect(rect)
                                                .layout(egui::Layout::top_down_justified(egui::Align::Center))
                                        );
                                        let plot = egui_plot::Plot::new(format!("map_plot_{}", tid))
                                            .data_aspect(1.0)
                                            .show_axes(false)
                                            .show_grid(false)
                                            .allow_zoom(false)
                                            .allow_drag(false)
                                            .allow_scroll(false)
                                            .show_background(false);

                                        plot.show(&mut plot_ui_builder, |plot_ui| {
                                            if let Some(segments) = self.simgit_track_maps.get(&tid) {
                                                for seg_pts in segments {
                                                    plot_ui.line(egui_plot::Line::new("", egui_plot::PlotPoints::from(seg_pts.clone()))
                                                        .color(egui::Color32::from_white_alpha(30))
                                                        .width(4.0)
                                                    );
                                                }
                                            }
                                        });
                                    }
                                    
                                    // 2. Draw Text Content
                                    let inner_rect = rect.shrink(15.0);
                                    let mut child_ui = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(inner_rect)
                                            .layout(*ui.layout())
                                    );
                                    
                                    child_ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&entry.file_name).strong().color(text_color).size(16.0));
                                        ui.add_space(8.0);
                                        
                                        let summary_color = if entry.diff_summary.contains("No Changes") || entry.diff_summary.contains("Baseline") {
                                            if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY }
                                        } else {
                                            ACCENT_COLOR
                                        };
                                        ui.label(egui::RichText::new(&entry.diff_summary).color(summary_color).strong());
                                        
                                        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                            ui.horizontal(|ui| {
                                                if let Some(tid) = entry.track_id {
                                                    ui.label(egui::RichText::new(format!("Track ID: {}", tid)).color(egui::Color32::GRAY).small());
                                                } else {
                                                    ui.label(egui::RichText::new("No Track Data").color(egui::Color32::GRAY).small());
                                                }
                                                
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button(egui::RichText::new("🗑 Delete").color(egui::Color32::LIGHT_RED)).clicked() {
                                                        crate::simgit::history::remove_file(&current_root.join(&current_proj), &entry.file_name);
                                                    }
                                                    if ui.button(egui::RichText::new("▶ Load").color(egui::Color32::LIGHT_GREEN)).clicked() {
                                                        let file_path = current_root.join(&current_proj).join("setups").join(&entry.file_name);
                                                        if file_path.exists() {
                                                            self.load_telemetry_file(&file_path);
                                                        }
                                                    }
                                                });
                                            });
                                        });
                                    });
                                });
                            
                            ui.add_space(15.0); // Spacing between cards
                        }
                    });
                });
            }
        } else {
            ui.label("No active workspace selected. Create one at the top right.");
        }
    }

    fn draw_simgit_setups(&mut self, ui: &mut egui::Ui, is_dark: bool, text_color: egui::Color32) {
        ui.heading("History & Diff Engine Placeholder");
        // We will move the history list and diff grid here in the next step!
    }
}
