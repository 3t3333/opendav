use crate::config::theme::AppTheme;
use crate::{OpenDavApp, SimGitTab};

impl OpenDavApp {
    pub fn draw_simgit_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let theme = AppTheme::for_mode(is_dark);

        egui::Panel::top("simgit_top_nav")
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface_panel)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .inner_margin(10.0),
            )
            .show_inside(ui, |ui| {
                let is_narrow = ui.available_width() < 760.0;

                if is_narrow {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("SimGit")
                                .strong()
                                .color(theme.accent_text)
                                .size(23.0),
                        );
                        ui.add_space(10.0);
                        self.draw_simgit_tabs(ui, theme);
                    });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        self.draw_simgit_workspace_controls(ui, theme);
                    });
                } else {
                ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("SimGit")
                                .strong()
                                .color(theme.accent_text)
                                .size(23.0),
                        );
                        ui.add_space(18.0);
                        self.draw_simgit_tabs(ui, theme);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.draw_simgit_workspace_controls(ui, theme)
                        });
                    });
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface_root)
                    .inner_margin(15.0),
            )
            .show_inside(ui, |ui| match self.simgit_active_tab {
                SimGitTab::Dashboard => self.draw_simgit_dashboard(ui, theme),
                SimGitTab::Setups => self.draw_simgit_setups(ui, theme),
                SimGitTab::Cloud => {
                    ui.heading(
                        egui::RichText::new("Cloud Sync")
                            .color(theme.text_primary)
                            .size(24.0),
                    );
                    ui.label(
                        egui::RichText::new("Cloud synchronization is planned for Phase 2.")
                            .color(theme.text_secondary),
                    );
                }
            });
    }

    fn draw_simgit_tabs(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
                    let tabs = [
                        (SimGitTab::Dashboard, "Dashboard"),
                        (SimGitTab::Setups, "Setups & Commits"),
                        (SimGitTab::Cloud, "Cloud Sync"),
                    ];

                    for (tab, name) in tabs {
                        let is_active = self.simgit_active_tab == tab;
            let color = if is_active {
                theme.text_primary
            } else {
                theme.text_tertiary
            };
            if ui
                .selectable_label(
                    is_active,
                    egui::RichText::new(name).color(color).strong().size(16.0),
                )
                .clicked()
            {
                            self.simgit_active_tab = tab;
                        }
                    }
    }

    fn draw_simgit_workspace_controls(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
                        if self.simgit_manager.active_project.is_some() {
            let commit_button = egui::Button::new(
                egui::RichText::new("+ Commit Files")
                    .strong()
                    .color(theme.on_accent)
                    .size(15.0),
            )
            .fill(theme.accent)
            .stroke(egui::Stroke::new(1.0, theme.accent_text));
            if ui.add(commit_button).clicked() {
                                if let Some(files) = rfd::FileDialog::new()
                                    .add_filter("iRacing Telemetry", &["ibt"])
                                    .pick_files()
                                {
                    if let Some(project) = self.simgit_manager.active_project.as_ref() {
                        crate::simgit::history::commit_files(
                            &self.simgit_manager.root_dir.join(project),
                            &files,
                        );
                    }
                                }
                            }
                        }

                        if self.show_new_ws_popup {
            let confirm = egui::Button::new(
                egui::RichText::new("Confirm")
                    .color(theme.success)
                    .strong()
                    .size(14.0),
            )
            .fill(theme.surface_elevated)
            .stroke(egui::Stroke::new(1.0, theme.success));
            if ui.add(confirm).clicked() && !self.simgit_new_ws_name.is_empty() {
                                    let _ = self.simgit_manager.create_project(&self.simgit_new_ws_name);
                self.simgit_manager
                    .set_active_project(&self.simgit_new_ws_name);
                                    self.simgit_new_ws_name.clear();
                                    self.show_new_ws_popup = false;
                                }

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Cancel")
                            .color(theme.text_secondary)
                            .size(14.0),
                    )
                    .fill(theme.surface_elevated)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle)),
                )
                .clicked()
            {
                                self.show_new_ws_popup = false;
                            }
            ui.add(
                egui::TextEdit::singleline(&mut self.simgit_new_ws_name)
                    .hint_text("Workspace name")
                    .desired_width(190.0),
            );
                        } else {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("+ New")
                            .color(theme.accent_text)
                            .strong()
                            .size(14.0),
                    )
                    .fill(theme.surface_elevated)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle)),
                )
                .clicked()
            {
                                self.show_new_ws_popup = true;
                            }
                            
            let mut selected_proj = self
                .simgit_manager
                .active_project
                .clone()
                .unwrap_or_else(|| "Select Workspace".to_string());
                            let projects = self.simgit_manager.list_projects();
                            
            egui::ComboBox::from_id_salt("workspace_selector")
                .width(180.0)
                .selected_text(
                    egui::RichText::new(&selected_proj)
                        .strong()
                        .color(theme.text_primary)
                        .size(14.0),
                )
                                .show_ui(ui, |ui| {
                                    for proj in projects {
                        if ui
                            .selectable_value(&mut selected_proj, proj.clone(), &proj)
                            .changed()
                        {
                                            self.simgit_manager.set_active_project(&proj);
                                        }
                                    }
                                });
                        }
    }

    fn draw_simgit_dashboard(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        ui.heading(
            egui::RichText::new("Recent Sessions")
                .strong()
                .color(theme.text_primary)
                .size(24.0),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Telemetry committed to the active workspace.")
                .color(theme.text_secondary)
                .size(14.0),
        );
        ui.add_space(16.0);

        if let Some(ref proj_ref) = self.simgit_manager.active_project {
            let proj = proj_ref.clone();
            let root_dir = self.simgit_manager.root_dir.clone();
            let history = crate::simgit::history::get_history(&root_dir.join(&proj));

            if history.is_empty() {
                egui::Frame::NONE
                    .fill(theme.surface_panel)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(10.0)
                    .inner_margin(18.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(
                                "No sessions recorded yet. Commit one or more .ibt files to begin.",
                            )
                            .color(theme.text_secondary),
                        );
                    });
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("simgit_session_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let gap = 12.0;
                        let available_width = ui.available_width();
                        let columns = ((available_width + gap) / (280.0 + gap))
                            .floor()
                            .max(1.0);
                        let card_width =
                            ((available_width - gap * (columns - 1.0)) / columns).max(1.0);
                            
                        ui.spacing_mut().item_spacing.x = gap;
                        ui.horizontal_wrapped(|ui| {
                            for entry in history.iter().rev() {
                            let current_proj = proj.clone();
                            let current_root = root_dir.clone();
                            
                                egui::Frame::NONE
                                    .fill(theme.surface_card)
                                    .corner_radius(10.0)
                                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                .inner_margin(0.0)
                                .show(ui, |ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(card_width, 168.0),
                                            egui::Sense::hover(),
                                        );
                                    
                                    if let Some(tid) = entry.track_id {
                                            if let std::collections::hash_map::Entry::Vacant(entry) =
                                                self.simgit_track_maps.entry(tid)
                                            {
                                                let json_path = std::env::current_dir()
                                                    .unwrap()
                                                    .join("exports")
                                                    .join("track_maps")
                                                    .join(format!("{}.json", tid));
                                            if json_path.exists() {
                                                    if let Ok(json_str) =
                                                        std::fs::read_to_string(&json_path)
                                                    {
                                                        if let Ok(segments) =
                                                            serde_json::from_str::<
                                                                Vec<Vec<[f64; 2]>>,
                                                            >(&json_str)
                                                        {
                                                            entry.insert(segments);
                                                    }
                                                }
                                            }
                                        }

                                        let mut plot_ui_builder = ui.new_child(
                                                egui::UiBuilder::new().max_rect(rect).layout(
                                                    egui::Layout::top_down_justified(
                                                        egui::Align::Center,
                                                    ),
                                                ),
                                        );
                                            let plot = egui_plot::Plot::new(format!(
                                                "map_plot_{}_{}",
                                                tid, entry.file_name
                                            ))
                                            .data_aspect(1.0)
                                            .show_axes(false)
                                            .show_grid(false)
                                            .allow_zoom(false)
                                            .allow_drag(false)
                                            .allow_scroll(false)
                                            .show_background(false);

                                        plot.show(&mut plot_ui_builder, |plot_ui| {
                                                if let Some(segments) =
                                                    self.simgit_track_maps.get(&tid)
                                                {
                                                for seg_pts in segments {
                                                        plot_ui.line(
                                                            egui_plot::Line::new(
                                                                "",
                                                                egui_plot::PlotPoints::from(
                                                                    seg_pts.clone(),
                                                                ),
                                                            )
                                                            .color(
                                                                theme.reference_primary_faint,
                                                            )
                                                            .width(3.0),
                                                    );
                                                }
                                            }
                                        });
                                    }
                                    
                                        let inner_rect = rect.shrink(14.0);
                                    let mut child_ui = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(inner_rect)
                                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                                    );
                                    
                                    child_ui.vertical(|ui| {
                                            let file_response = ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&entry.file_name)
                                                        .strong()
                                                        .color(theme.text_primary)
                                                        .size(15.0),
                                                )
                                                .truncate(),
                                            );
                                            file_response.on_hover_text(&entry.file_name);
                                        ui.add_space(8.0);
                                        
                                            let has_no_changes = entry
                                                .diff_summary
                                                .contains("No Changes")
                                                || entry.diff_summary.contains("Baseline");
                                            let summary_color = if has_no_changes {
                                                theme.text_secondary
                                        } else {
                                                theme.accent_text
                                        };
                                            let summary_response = ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&entry.diff_summary)
                                                        .color(summary_color)
                                                        .strong(),
                                                )
                                                .truncate(),
                                            );
                                            summary_response.on_hover_text(&entry.diff_summary);
                                        
                                            ui.with_layout(
                                                egui::Layout::bottom_up(egui::Align::LEFT),
                                                |ui| {
                                            ui.horizontal(|ui| {
                                                        let track_label = entry.track_id.map_or_else(
                                                            || "No track data".to_string(),
                                                            |tid| format!("Track ID: {}", tid),
                                                        );
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(track_label)
                                                                    .color(theme.text_tertiary)
                                                                    .small(),
                                                            )
                                                            .truncate(),
                                                        );
                                                
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                let delete = egui::Button::new(
                                                                    egui::RichText::new("Delete")
                                                                        .color(theme.danger)
                                                                        .strong(),
                                                                )
                                                                .fill(theme.surface_elevated)
                                                                .stroke(egui::Stroke::new(
                                                                    1.0,
                                                                    theme.danger,
                                                                ));
                                                                if ui.add(delete).clicked() {
                                                                    crate::simgit::history::remove_file(
                                                                        &current_root
                                                                            .join(&current_proj),
                                                                        &entry.file_name,
                                                                    );
                                                    }

                                                                let load = egui::Button::new(
                                                                    egui::RichText::new("Load")
                                                                        .color(theme.success)
                                                                        .strong(),
                                                                )
                                                                .fill(theme.surface_elevated)
                                                                .stroke(egui::Stroke::new(
                                                                    1.0,
                                                                    theme.success,
                                                                ));
                                                                if ui.add(load).clicked() {
                                                                    let file_path = current_root
                                                                        .join(&current_proj)
                                                                        .join("setups")
                                                                        .join(&entry.file_name);
                                                        if file_path.exists() {
                                                                        self.load_telemetry_file(
                                                                            &file_path,
                                                                        );
                                                        }
                                                    }
                                                            },
                                                        );
                                                });
                                                },
                                            );
                                            });
                                        });
                        }
                    });
                });
            }
        } else {
            egui::Frame::NONE
                .fill(theme.surface_panel)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(10.0)
                .inner_margin(18.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "No active workspace. Select one above or create a new workspace.",
                        )
                        .color(theme.text_secondary),
                    );
                });
        }
    }

    fn draw_simgit_setups(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        ui.heading(
            egui::RichText::new("Setups & Commits")
                .color(theme.text_primary)
                .size(24.0),
        );
        ui.label(
            egui::RichText::new("History and diff tools are planned for a future update.")
                .color(theme.text_secondary),
        );
    }
}
