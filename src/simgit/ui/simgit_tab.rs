use crate::config::theme::AppTheme;
use crate::simgit::repository::{AnalysisNote, ImportBatchSummary, ImportStatus};
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
                let narrow = ui.available_width() < 820.0;
                if narrow {
                    ui.horizontal_wrapped(|ui| {
                        self.draw_simgit_title_and_tabs(ui, theme);
                    });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| self.draw_simgit_repository_controls(ui, theme));
                } else {
                    ui.horizontal(|ui| {
                        self.draw_simgit_title_and_tabs(ui, theme);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.draw_simgit_repository_controls(ui, theme);
                        });
                    });
                }
            });
        egui::SidePanel::right("simgit_workspace_panel")
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface_panel)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .inner_margin(10.0),
            )
            .default_width(250.0)
            .show_inside(ui, |ui| {
                ui.heading(egui::RichText::new("Workspaces").color(theme.text_primary));
                ui.add_space(10.0);
                
                let mut project_to_delete = None;
                for project in self.simgit_manager.list_projects() {
                    ui.horizontal(|ui| {
                        let is_active = self.simgit_manager.active_project.as_deref() == Some(project.as_str());
                        if ui.selectable_label(is_active, &project).clicked() {
                            let _ = self.simgit_manager.set_active_project(&project);
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("🗑").color(theme.danger)).clicked() {
                                project_to_delete = Some(project.clone());
                            }
                        });
                    });
                }
                
                if let Some(proj) = project_to_delete {
                    let path = self.simgit_manager.root_dir.join(&proj);
                    let _ = std::fs::remove_dir_all(&path);
                    if self.simgit_manager.active_project.as_deref() == Some(proj.as_str()) {
                        self.simgit_manager.active_project = None;
                    }
                }
                
                if self.simgit_manager.list_projects().is_empty() {
                    ui.label(egui::RichText::new("No local workspaces.").color(theme.text_secondary));
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface_root)
                    .inner_margin(16.0),
            )
            .show_inside(ui, |ui| {
                if let Some(message) = &self.simgit_status_message {
                    egui::Frame::NONE
                        .fill(theme.surface_elevated)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .corner_radius(6.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(message).color(theme.text_secondary),
                                )
                                .wrap(),
                            );
                        });
                    ui.add_space(12.0);
                }

                match self.simgit_active_tab {
                    SimGitTab::Dashboard => self.draw_simgit_repository(ui, theme),
                    SimGitTab::Setups => self.draw_simgit_team_notes(ui, theme),
                    SimGitTab::Cloud => self.draw_simgit_sync_status(ui, theme),
                }
            });
        if self.show_simgit_analysis_builder {
            self.draw_simgit_analysis_builder(ui.ctx(), theme);
        }
    }

    pub(crate) fn poll_simgit_import(&mut self) {
        let result = self
            .simgit_import_receiver
            .as_ref()
            .map(std::sync::mpsc::Receiver::try_recv);
        match result {
            Some(Ok(summary)) => {
                self.simgit_status_message = Some(format_import_summary(&summary));
                self.simgit_import_receiver = None;
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.simgit_status_message =
                    Some("Telemetry import stopped before completing.".to_owned());
                self.simgit_import_receiver = None;
            }
            _ => {}
        }
    }

    fn draw_simgit_title_and_tabs(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        ui.label(
            egui::RichText::new("SimGit")
                .strong()
                .color(theme.accent_text)
                .size(23.0),
        );
        ui.add_space(18.0);
        for (tab, label) in [
            (SimGitTab::Dashboard, "Repository"),
            (SimGitTab::Setups, "Team Notes"),
            (SimGitTab::Cloud, "Sync"),
        ] {
            let active = self.simgit_active_tab == tab;
            let color = if active {
                theme.text_primary
            } else {
                theme.text_tertiary
            };
            if ui
                .selectable_label(
                    active,
                    egui::RichText::new(label).color(color).strong().size(15.0),
                )
                .clicked()
            {
                self.simgit_active_tab = tab;
            }
        }
    }

    fn draw_simgit_repository_controls(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        let importing = self.simgit_import_receiver.is_some();
        if self.simgit_manager.active_project.is_some() {
            let button = egui::Button::new(
                egui::RichText::new(if importing {
                    "Importing..."
                } else {
                    "+ Import IBT"
                })
                .strong()
                .color(theme.on_accent),
            )
            .fill(theme.accent);
            if ui.add_enabled(!importing, button).clicked() {
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter("iRacing Telemetry", &["ibt"])
                    .pick_files()
                {
                    self.start_simgit_import(files);
                }
            }
        }

        if self.show_new_ws_popup {
            if ui.button("Cancel").clicked() {
                self.show_new_ws_popup = false;
            }
            if ui.button("Create").clicked() {
                match self.simgit_manager.create_project(&self.simgit_new_ws_name) {
                    Ok(()) => {
                        self.simgit_status_message = Some(format!(
                            "Created local repository '{}'.",
                            self.simgit_new_ws_name.trim()
                        ));
                        self.simgit_new_ws_name.clear();
                        self.show_new_ws_popup = false;
                    }
                    Err(error) => self.simgit_status_message = Some(error.to_string()),
                }
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.simgit_new_ws_name)
                    .hint_text("Repository name")
                    .desired_width(180.0),
            );
        } else if ui.button("+ New Repository").clicked() {
            self.show_new_ws_popup = true;
        }

        let selected = self
            .simgit_manager
            .active_project
            .clone()
            .unwrap_or_else(|| "Select Repository".to_owned());
        let mut selection = None;
        egui::ComboBox::from_id_salt("simgit_repository_selector")
            .width(190.0)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for project in self.simgit_manager.list_projects() {
                    if ui.selectable_label(false, &project).clicked() {
                        selection = Some(project);
                    }
                }
            });
        if let Some(project) = selection {
            if let Err(error) = self.simgit_manager.set_active_project(&project) {
                self.simgit_status_message = Some(error.to_string());
            }
        }
    }

    fn start_simgit_import(&mut self, files: Vec<std::path::PathBuf>) {
        let Some(project) = self.simgit_manager.active_project.clone() else {
            self.simgit_status_message = Some("Select a repository before importing.".to_owned());
            return;
        };
        let root = self.simgit_manager.root_dir.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        self.simgit_import_receiver = Some(receiver);
        self.simgit_status_message =
            Some(format!("Importing {} telemetry file(s)...", files.len()));
        std::thread::spawn(move || {
            let mut summary = ImportBatchSummary::default();
            match crate::simgit::repository::SimGitRepository::open(&root, &project) {
                Ok(mut repository) => {
                    for file in files {
                        match repository.import_ibt(&file) {
                            Ok(result) if result.status == ImportStatus::Imported => {
                                summary.imported += 1;
                            }
                            Ok(_) => summary.already_present += 1,
                            Err(error) => summary.failures.push(format!(
                                "{}: {error}",
                                file.file_name().unwrap_or_default().to_string_lossy()
                            )),
                        }
                    }
                }
                Err(error) => summary.failures.push(error.to_string()),
            }
            let _ = sender.send(summary);
        });
    }

    fn draw_simgit_repository(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        ui.heading(
            egui::RichText::new("Telemetry Repository")
                .color(theme.text_primary)
                .size(25.0),
        );
        ui.label(
            egui::RichText::new(
                "Content-addressed, compressed telemetry ready for analysis and future team sync.",
            )
            .color(theme.text_secondary),
        );
        ui.add_space(16.0);

        let Some(project) = self.simgit_manager.active_project.clone() else {
            draw_empty_repository(ui, theme, "Create or select a repository to begin.");
            return;
        };
        let mut repository = match self.simgit_manager.repository(&project) {
            Ok(repository) => repository,
            Err(error) => {
                draw_empty_repository(ui, theme, &error.to_string());
                return;
            }
        };
        let mut records = repository.telemetry().to_vec();
        records.sort_by_key(|record| std::cmp::Reverse(record.imported_at));
        if records.is_empty() {
            draw_empty_repository(
                ui,
                theme,
                "Import one or more .ibt files. Duplicate content is stored only once.",
            );
            return;
        }

        let analyze_button = egui::Button::new(
            egui::RichText::new("Analyze Laps")
                .strong()
                .color(theme.on_accent)
                .size(15.0),
        )
        .fill(theme.accent)
        .min_size(egui::vec2(150.0, 34.0));
        if ui.add(analyze_button).clicked() {
            self.prepare_simgit_analysis_builder(&mut repository, &records);
        }
        ui.add_space(14.0);

        let mut open_record = None;
        let mut delete_record = None;
        egui::ScrollArea::vertical()
            .id_salt("simgit_repository_records")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for record in &records {
                    let note_count = repository.notes_for(&record.id).len();
                    egui::Frame::NONE
                        .fill(theme.surface_card)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .corner_radius(9.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&record.original_name)
                                                .strong()
                                                .color(theme.text_primary)
                                                .size(16.0),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(&record.original_name);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} | {} | Track {}",
                                            record.car, record.venue, record.track_id
                                        ))
                                        .color(theme.text_secondary),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} -> {} compressed | {} note{} | {}",
                                            format_bytes(record.uncompressed_size),
                                            format_bytes(record.compressed_size),
                                            note_count,
                                            if note_count == 1 { "" } else { "s" },
                                            format_timestamp(record.imported_at)
                                        ))
                                        .small()
                                        .color(theme.text_tertiary),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(
                                                egui::RichText::new("Delete").color(theme.danger),
                                            )
                                            .clicked()
                                        {
                                            delete_record = Some(record.id.clone());
                                        }
                                        if ui
                                            .button(
                                                egui::RichText::new("Open in Graphs")
                                                    .color(theme.success),
                                            )
                                            .clicked()
                                        {
                                            open_record = Some(record.id.clone());
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(10.0);
                }
            });

        if let Some(id) = delete_record {
            match repository.remove_telemetry(&id) {
                Ok(()) => self.simgit_status_message = Some("Telemetry removed.".to_owned()),
                Err(error) => self.simgit_status_message = Some(error.to_string()),
            }
        }
        if let Some(id) = open_record {
            if let Err(error) = self.open_simgit_telemetry(&project, &id) {
                self.simgit_status_message = Some(error);
            }
        }
    }

    fn prepare_simgit_analysis_builder(
        &mut self,
        repository: &mut crate::simgit::repository::SimGitRepository,
        records: &[crate::simgit::repository::TelemetryRecord],
    ) {
        let defaults: Vec<_> = records
            .iter()
            .take(2)
            .map(|record| record.id.clone())
            .collect();
        if defaults.is_empty() {
            self.simgit_status_message =
                Some("Import telemetry before starting analysis.".to_owned());
            return;
        }
        for telemetry_id in &defaults {
            if let Err(error) = repository.ensure_lap_summaries(telemetry_id) {
                self.simgit_status_message = Some(error.to_string());
                return;
            }
        }
        let records = repository.telemetry();
        let baseline = defaults[0].clone();
        let reference = defaults.get(1).cloned().unwrap_or_else(|| baseline.clone());
        self.simgit_analysis_draft.selected_telemetry = defaults.into_iter().collect();
        self.simgit_analysis_draft.baseline_lap = records
            .iter()
            .find(|record| record.id == baseline)
            .and_then(|record| record.fastest_lap())
            .map(|lap| lap.lap_number);
        self.simgit_analysis_draft.reference_lap = records
            .iter()
            .find(|record| record.id == reference)
            .and_then(|record| record.fastest_lap())
            .map(|lap| lap.lap_number);
        self.simgit_analysis_draft.baseline_telemetry = Some(baseline);
        self.simgit_analysis_draft.reference_telemetry = Some(reference);
        self.show_simgit_analysis_builder = true;
    }

    fn draw_simgit_analysis_builder(&mut self, ctx: &egui::Context, theme: AppTheme) {
        let Some(project) = self.simgit_manager.active_project.clone() else {
            self.show_simgit_analysis_builder = false;
            return;
        };
        let mut repository = match self.simgit_manager.repository(&project) {
            Ok(repository) => repository,
            Err(error) => {
                self.simgit_status_message = Some(error.to_string());
                self.show_simgit_analysis_builder = false;
                return;
            }
        };
        let mut records = repository.telemetry().to_vec();
        records.sort_by_key(|record| std::cmp::Reverse(record.imported_at));
        let mut window_open = self.show_simgit_analysis_builder;
        let mut cancel = false;
        let mut analyze = false;

        egui::Window::new("Start Analysis Session")
            .id(egui::Id::new("simgit_analysis_builder"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(760.0)
            .max_width(900.0)
            .collapsible(false)
            .resizable(true)
            .open(&mut window_open)
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(theme.surface_panel)
                    .stroke(egui::Stroke::new(1.0, theme.border_strong)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Choose every repository file to load, then set the initial baseline and reference laps.",
                    )
                    .color(theme.text_secondary),
                );
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("Files in this analysis")
                        .strong()
                        .color(theme.text_primary),
                );
                egui::ScrollArea::vertical()
                    .id_salt("simgit_analysis_files")
                    .max_height(190.0)
                    .show(ui, |ui| {
                        for record in &mut records {
                            let mut selected = self
                                .simgit_analysis_draft
                                .selected_telemetry
                                .contains(&record.id);
                            let response = ui.checkbox(
                                &mut selected,
                                format!(
                                    "{} | {} | {}",
                                    record.original_name, record.car, record.venue
                                ),
                            );
                            if response.changed() {
                                if selected {
                                    match repository.ensure_lap_summaries(&record.id) {
                                        Ok(laps) => {
                                            record.laps = laps;
                                            self.simgit_analysis_draft
                                                .selected_telemetry
                                                .insert(record.id.clone());
                                        }
                                        Err(error) => {
                                            self.simgit_status_message = Some(error.to_string())
                                        }
                                    }
                                } else {
                                    self.simgit_analysis_draft
                                        .selected_telemetry
                                        .remove(&record.id);
                                }
                            }
                        }
                    });

                normalize_analysis_draft(&mut self.simgit_analysis_draft, &records);
                ui.add_space(14.0);
                let role_width = ((ui.available_width() - 58.0) / 2.0).max(120.0);
                ui.horizontal(|ui| {
                    ui.allocate_ui(egui::vec2(role_width, 88.0), |ui| {
                        draw_analysis_role(
                            ui,
                            theme,
                            "Baseline",
                            "simgit_baseline_file",
                            "simgit_baseline_lap",
                            &records,
                            &self.simgit_analysis_draft.selected_telemetry,
                            &mut self.simgit_analysis_draft.baseline_telemetry,
                            &mut self.simgit_analysis_draft.baseline_lap,
                        );
                    });
                    ui.vertical(|ui| {
                        ui.add_space(27.0);
                        if ui
                            .small_button("Swap")
                            .on_hover_text("Swap baseline and reference")
                            .clicked()
                        {
                            swap_analysis_roles(&mut self.simgit_analysis_draft);
                        }
                    });
                    ui.allocate_ui(egui::vec2(role_width, 88.0), |ui| {
                        draw_analysis_role(
                            ui,
                            theme,
                            "Reference",
                            "simgit_reference_file",
                            "simgit_reference_lap",
                            &records,
                            &self.simgit_analysis_draft.selected_telemetry,
                            &mut self.simgit_analysis_draft.reference_telemetry,
                            &mut self.simgit_analysis_draft.reference_lap,
                        );
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    let ready = analysis_draft_ready(&self.simgit_analysis_draft);
                    if ui
                        .add_enabled(
                            ready,
                            egui::Button::new(
                                egui::RichText::new("Analyze")
                                    .strong()
                                    .color(theme.on_accent),
                            )
                            .fill(theme.accent)
                            .min_size(egui::vec2(110.0, 32.0)),
                        )
                        .clicked()
                    {
                        analyze = true;
                    }
                });
            });

        if cancel || !window_open {
            self.show_simgit_analysis_builder = false;
        } else if analyze {
            let selected: Vec<_> = records
                .iter()
                .filter(|record| {
                    self.simgit_analysis_draft
                        .selected_telemetry
                        .contains(&record.id)
                })
                .map(|record| record.id.clone())
                .collect();
            let baseline_id = self.simgit_analysis_draft.baseline_telemetry.clone();
            let reference_id = self.simgit_analysis_draft.reference_telemetry.clone();
            if let (
                Some(baseline_id),
                Some(baseline_lap),
                Some(reference_id),
                Some(reference_lap),
            ) = (
                baseline_id,
                self.simgit_analysis_draft.baseline_lap,
                reference_id,
                self.simgit_analysis_draft.reference_lap,
            ) {
                match self.start_simgit_analysis(
                    &project,
                    &selected,
                    (&baseline_id, baseline_lap),
                    (&reference_id, reference_lap),
                ) {
                    Ok(()) => self.show_simgit_analysis_builder = false,
                    Err(error) => self.simgit_status_message = Some(error),
                }
            }
        }
    }

    fn draw_simgit_team_notes(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        ui.heading(
            egui::RichText::new("Team Analysis Notes")
                .color(theme.text_primary)
                .size(25.0),
        );
        ui.label(
            egui::RichText::new(
                "Notes are anchored to telemetry time, lap, viewport, and worksheet.",
            )
            .color(theme.text_secondary),
        );
        ui.add_space(16.0);

        let Some(project) = self.simgit_manager.active_project.clone() else {
            draw_empty_repository(ui, theme, "Select a repository to view its notes.");
            return;
        };
        let mut repository = match self.simgit_manager.repository(&project) {
            Ok(repository) => repository,
            Err(error) => {
                draw_empty_repository(ui, theme, &error.to_string());
                return;
            }
        };
        let mut notes = repository.notes().to_vec();
        notes.sort_by_key(|note| std::cmp::Reverse(note.created_at));
        if notes.is_empty() {
            draw_empty_repository(
                ui,
                theme,
                "Open repository telemetry and use the Graphs NOTES drawer to add analysis.",
            );
            return;
        }

        let mut open_note = None;
        let mut delete_note = None;
        egui::ScrollArea::vertical()
            .id_salt("simgit_team_notes")
            .show(ui, |ui| {
                for note in &notes {
                    let is_active = self.active_simgit_note_id.as_deref() == Some(note.id.as_str());
                    let file_name = repository
                        .telemetry()
                        .iter()
                        .find(|record| record.id == note.telemetry_id)
                        .map(|record| record.original_name.as_str())
                        .unwrap_or("Unknown telemetry");
                    egui::Frame::NONE
                        .fill(if is_active {
                            theme.surface_elevated
                        } else {
                            theme.surface_card
                        })
                        .stroke(egui::Stroke::new(
                            if is_active { 2.0 } else { 1.0 },
                            if is_active {
                                note.color.display_color(theme.is_dark)
                            } else {
                                theme.border_subtle
                            },
                        ))
                        .corner_radius(8.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let view_tab = egui::Button::new(
                                    egui::RichText::new(if is_active {
                                        "Viewing note"
                                    } else {
                                        "View note"
                                    })
                                    .strong()
                                    .small()
                                    .color(if is_active {
                                        theme.on_accent
                                    } else {
                                        theme.text_primary
                                    }),
                                )
                                .fill(if is_active {
                                    theme.accent
                                } else {
                                    note.color
                                        .display_color(theme.is_dark)
                                        .gamma_multiply(0.18)
                                })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    note.color.display_color(theme.is_dark),
                                ))
                                .corner_radius(4.0)
                                .min_size(egui::vec2(92.0, 24.0));
                                if ui.add(view_tab).clicked() {
                                    open_note = Some(note.clone());
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(
                                                egui::RichText::new("Delete").color(theme.danger),
                                            )
                                            .clicked()
                                        {
                                            delete_note = Some(note.id.clone());
                                        }
                                    },
                                );
                            });
                            ui.add_space(7.0);
                            ui.horizontal(|ui| {
                                let (tag_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(14.0, 14.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    tag_rect,
                                    4.0,
                                    note.color.display_color(theme.is_dark),
                                );
                                if is_active {
                                    ui.label(
                                        egui::RichText::new("VIEWING")
                                            .strong()
                                            .small()
                                            .color(theme.accent_text),
                                    );
                                }
                                ui.label(
                                    egui::RichText::new(note.display_objective())
                                        .strong()
                                        .size(16.0)
                                        .color(theme.text_primary),
                                );
                                if let Some(delta) = note.context.section_delta {
                                    let delta_str = crate::simgit::repository::format_section_delta(delta);
                                    let delta_clr = crate::simgit::repository::section_delta_color(delta, theme.is_dark);
                                    let bg_clr = delta_clr.gamma_multiply(0.18);
                                    egui::Frame::NONE
                                        .fill(bg_clr)
                                        .stroke(egui::Stroke::new(1.0, delta_clr))
                                        .corner_radius(4.0)
                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("Delta: {delta_str}"))
                                                    .strong()
                                                    .size(13.0)
                                                    .color(delta_clr),
                                            );
                                        });
                                }
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} | {} | {} | {}",
                                    note.author,
                                    file_name,
                                    format_context(note),
                                    format_timestamp(note.created_at)
                                ))
                                .small()
                                .color(theme.text_tertiary),
                            );
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&note.body).color(theme.text_secondary),
                                )
                                .wrap(),
                            );
                            for (role, reference) in [
                                ("Cyan", note.context.cyan_reference.as_ref()),
                                ("Secondary", note.context.secondary_reference.as_ref()),
                            ] {
                                if let Some(reference) = reference {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{role} reference: {} / Lap {}",
                                            reference.file_name, reference.lap_number
                                        ))
                                        .small()
                                        .color(theme.text_tertiary),
                                    );
                                }
                            }
                        });
                    ui.add_space(10.0);
                }
            });
        if let Some(note_id) = delete_note {
            match repository.remove_note(&note_id) {
                Ok(()) => {
                    self.simgit_status_message = Some("Analysis note deleted.".to_owned());
                    if self.active_simgit_note_id.as_deref() == Some(note_id.as_str()) {
                        self.active_simgit_note_id = None;
                    }
                    if let Some(note) = notes.iter().find(|note| note.id == note_id) {
                        let source = crate::simgit::repository::RepositoryRecordRef {
                            project: project.clone(),
                            telemetry_id: note.telemetry_id.clone(),
                        };
                        let _ = self.refresh_simgit_note_cache(&source);
                    }
                }
                Err(error) => self.simgit_status_message = Some(error.to_string()),
            }
        }
        if let Some(note) = open_note {
            self.open_simgit_note(&project, &note);
        }
    }

    fn open_simgit_note(&mut self, project: &str, note: &AnalysisNote) {
        if let Err(error) = self.open_simgit_note_context(project, note) {
            self.simgit_status_message = Some(error);
        }
    }

    fn draw_simgit_sync_status(&mut self, ui: &mut egui::Ui, theme: AppTheme) {
        ui.heading(
            egui::RichText::new("SimGit Cloud Collaboration")
                .color(theme.text_primary)
                .size(25.0),
        );
        ui.label(
            egui::RichText::new(
                "Synchronize telemetry packets, vehicle setups, and coaching analysis across your entire racing team using Supabase.",
            )
            .color(theme.text_secondary),
        );
        ui.add_space(16.0);

        let configured = !self.settings.active_supabase_url().is_empty()
            && !self.settings.active_supabase_anon_key().is_empty();

        if !configured {
            egui::Frame::NONE
                .fill(theme.surface_card)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("⚠️ Cloud Sync Credentials Required")
                            .strong()
                            .size(16.0)
                            .color(theme.text_primary),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "To connect to your collaborative team repository, enter your Supabase Project URL, Anon Key, and Team Email address in the Settings tab.\n\nSimGit includes an automated SQL migration script in Settings that sets up your Postgres schema, Row Level Security, and Role-Based Access Control (RBAC) instantly on any free Supabase tier.",
                        )
                        .color(theme.text_secondary),
                    );
                });
            ui.add_space(16.0);
        } else {
            egui::Frame::NONE
                .fill(theme.surface_card)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    if self.simgit_access_token.is_none() {
                        ui.label(egui::RichText::new("Sign In to Team Repository").strong().color(theme.text_primary));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("Email:");
                            ui.text_edit_singleline(&mut self.simgit_auth_email);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Password:");
                            ui.add(egui::TextEdit::singleline(&mut self.simgit_auth_password).password(true));
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Sign In").clicked() {
                                let mut client = crate::simgit::data::client::SupabaseClient::new(
                                    self.settings.active_supabase_url(),
                                    self.settings.active_supabase_anon_key(),
                                    None,
                                    None,
                                );
                                match client.sign_in(&self.simgit_auth_email, &self.simgit_auth_password) {
                                    Ok((token, uid)) => {
                                        self.simgit_access_token = Some(token.clone());
                                        self.simgit_user_id = Some(uid.clone());
                                        client.access_token = Some(token);
                                        match client.check_connection_and_role(&uid) {
                                            Ok(role) => {
                                                self.simgit_sync_role = Some(role.display_name().to_owned());
                                                self.simgit_status_message = Some("Signed in successfully!".to_string());
                                                if role.can_manage_team() {
                                                    if let Ok(users) = Ok::<_, String>(vec![]) {
                                                        self.simgit_team_users = users;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                self.simgit_status_message = Some(e);
                                                self.simgit_sync_role = None;
                                                self.simgit_team_users.clear();
                                            }
                                        }
                                    }
                                    Err(e) => self.simgit_status_message = Some(e),
                                }
                            }
                            if ui.button("Sign Up").clicked() {
                                let mut client = crate::simgit::data::client::SupabaseClient::new(
                                    self.settings.active_supabase_url(),
                                    self.settings.active_supabase_anon_key(),
                                    None,
                                    None,
                                );
                                match client.sign_up(&self.simgit_auth_email, &self.simgit_auth_password) {
                                    Ok((token, uid)) => {
                                        self.simgit_access_token = Some(token.clone());
                                        self.simgit_user_id = Some(uid.clone());
                                        client.access_token = Some(token);
                                        match client.check_connection_and_role(&uid) {
                                            Ok(role) => {
                                                self.simgit_sync_role = Some(role.display_name().to_owned());
                                                self.simgit_status_message = Some("Account created and signed in!".to_string());
                                            }
                                            Err(e) => self.simgit_status_message = Some(e),
                                        }
                                    }
                                    Err(e) => self.simgit_status_message = Some(e),
                                }
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Connected Account:")
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                egui::RichText::new(&self.simgit_auth_email)
                                    .color(theme.accent_text),
                            );
                        });
                        ui.add_space(8.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("Sign Out").clicked() {
                                self.simgit_access_token = None;
                                self.simgit_user_id = None;
                                self.simgit_sync_role = None;
                                self.simgit_team_users.clear();
                                self.simgit_status_message = Some("Signed out.".to_string());
                            }
                            
                            if ui.button(egui::RichText::new("🗑️ Delete My Account").color(theme.danger)).clicked() {
                                let client = crate::simgit::data::client::SupabaseClient::new(self.settings.active_supabase_url(), self.settings.active_supabase_anon_key(), self.simgit_access_token.clone(), self.simgit_user_id.clone());
                                match client.delete_account() {
                                    Ok(_) => {
                                        self.simgit_access_token = None;
                                        self.simgit_user_id = None;
                                        self.simgit_sync_role = None;
                                        self.simgit_team_users.clear();
                                        self.simgit_status_message = Some("Account deleted successfully.".to_string());
                                    }
                                    Err(e) => self.simgit_status_message = Some(e),
                                }
                            }
                        });
                        
                        if let Some(role) = &self.simgit_sync_role {
                            ui.add_space(8.0);
                            let badge_color = if role.contains("Pending") {
                                egui::Color32::from_rgb(220, 160, 40)
                            } else {
                                egui::Color32::from_rgb(50, 180, 100)
                            };
                            egui::Frame::NONE
                                .fill(badge_color.gamma_multiply(0.2))
                                .stroke(egui::Stroke::new(1.0, badge_color))
                                .corner_radius(4.0)
                                .inner_margin(egui::Margin::symmetric(8, 3))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("Role: {}", role))
                                            .strong()
                                            .size(13.0)
                                            .color(badge_color),
                                    );
                                });
                        }

                        if let Some(role) = &self.simgit_sync_role {
                            if role.contains("Pending") {
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Notice: Your account is registered on Supabase but awaiting administrator approval. Ask your team admin to change your access level from 'pending' to 'editor' or 'viewer' in the user_roles table to unlock telemetry pulling and pushing.",
                                    )
                                    .small()
                                    .color(egui::Color32::from_rgb(220, 180, 70)),
                                );
                            }
                        }
                    }
                });
            ui.add_space(16.0);

            // Admin Dashboard (only displayed for verified team administrators)
            if let Some(role) = &self.simgit_sync_role {
                if role.contains("Admin") {
                    let mut role_changes: Vec<(String, crate::simgit::data::backend::BackendUserRole)> = Vec::new();
                    let mut do_refresh = false;

                    egui::Frame::NONE
                        .fill(theme.surface_card)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 130, 255)))
                        .corner_radius(8.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("👑 Admin Team Management Dashboard")
                                        .strong()
                                        .size(16.0)
                                        .color(theme.text_primary),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("🔄 Refresh Directory")
                                                    .strong()
                                                    .color(theme.on_accent),
                                            )
                                            .fill(theme.accent),
                                        )
                                        .clicked()
                                    {
                                        do_refresh = true;
                                    }
                                });
                            });
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(
                                    "As team administrator, you can approve pending accounts and adjust collaboration access levels below. Notice: The first account created on the server automatically receives Admin status.",
                                )
                                .color(theme.text_secondary)
                                .small(),
                            );
                            ui.add_space(12.0);

                            // Invite User Section
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Invite by Email:").strong().color(theme.text_primary));
                                ui.text_edit_singleline(&mut self.simgit_invite_email);
                                if ui.button("Add Editor").clicked() {
                                    if let (Some(project_name), Some(token), Some(uid)) = (&self.simgit_manager.active_project, &self.simgit_access_token, &self.simgit_user_id) {
                                        let mut client = crate::simgit::data::client::SupabaseClient::new(
                                            self.settings.active_supabase_url(),
                                            self.settings.active_supabase_anon_key(),
                                            Some(token.clone()),
                                            Some(uid.clone()),
                                        );
                                        match client.ensure_project(project_name) {
                                            Ok(proj_uuid) => {
                                                if let Err(e) = client.upsert_project_member(&proj_uuid, &self.simgit_invite_email, crate::simgit::data::backend::BackendUserRole::Editor) {
                                                    self.simgit_status_message = Some(e);
                                                } else {
                                                    self.simgit_status_message = Some(format!("Invited {} as Editor", self.simgit_invite_email));
                                                    self.simgit_invite_email.clear();
                                                    do_refresh = true;
                                                }
                                            }
                                            Err(e) => self.simgit_status_message = Some(e),
                                        }
                                    } else {
                                        self.simgit_status_message = Some("Please select an active workspace first.".to_string());
                                    }
                                }
                                if ui.button("Add Viewer").clicked() {
                                    if let (Some(project_name), Some(token), Some(uid)) = (&self.simgit_manager.active_project, &self.simgit_access_token, &self.simgit_user_id) {
                                        let mut client = crate::simgit::data::client::SupabaseClient::new(
                                            self.settings.active_supabase_url(),
                                            self.settings.active_supabase_anon_key(),
                                            Some(token.clone()),
                                            Some(uid.clone()),
                                        );
                                        match client.ensure_project(project_name) {
                                            Ok(proj_uuid) => {
                                                if let Err(e) = client.upsert_project_member(&proj_uuid, &self.simgit_invite_email, crate::simgit::data::backend::BackendUserRole::Viewer) {
                                                    self.simgit_status_message = Some(e);
                                                } else {
                                                    self.simgit_status_message = Some(format!("Invited {} as Viewer", self.simgit_invite_email));
                                                    self.simgit_invite_email.clear();
                                                    do_refresh = true;
                                                }
                                            }
                                            Err(e) => self.simgit_status_message = Some(e),
                                        }
                                    } else {
                                        self.simgit_status_message = Some("Please select an active workspace first.".to_string());
                                    }
                                }
                            });
                            ui.add_space(12.0);

                            if self.simgit_team_users.is_empty() {
                                ui.label(
                                    egui::RichText::new("No registered team accounts loaded in directory yet. Click 'Refresh Directory' above to pull the user list.")
                                        .color(theme.text_secondary)
                                        .italics(),
                                );
                            } else {
                                egui::Grid::new("admin_user_directory_grid")
                                    .num_columns(4)
                                    .spacing([16.0, 12.0])
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("User / Email").strong().color(theme.text_primary));
                                        ui.label(egui::RichText::new("Current Role").strong().color(theme.text_primary));
                                        ui.label(egui::RichText::new("Status").strong().color(theme.text_primary));
                                        ui.label(egui::RichText::new("Admin Actions / Role Modification").strong().color(theme.text_primary));
                                        ui.end_row();

                                        for u in &self.simgit_team_users {
                                            let email_disp = u.email.as_str();
                                            let is_me = u.user_id.as_deref() == self.simgit_user_id.as_deref();
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(email_disp).color(theme.text_primary));
                                                if is_me {
                                                    ui.label(
                                                        egui::RichText::new("(You - Admin)")
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(100, 210, 120))
                                                            .small(),
                                                    );
                                                }
                                            });

                                            let role_name = u.role.display_name();
                                            let badge_color = match u.role {
                                                crate::simgit::data::backend::BackendUserRole::Admin => egui::Color32::from_rgb(180, 130, 255),
                                                crate::simgit::data::backend::BackendUserRole::Editor => egui::Color32::from_rgb(70, 200, 110),
                                                crate::simgit::data::backend::BackendUserRole::Viewer => egui::Color32::from_rgb(70, 160, 255),
                                                crate::simgit::data::backend::BackendUserRole::Pending => egui::Color32::from_rgb(235, 170, 45),
                                            };
                                            ui.label(egui::RichText::new(role_name).strong().color(badge_color));

                                            if u.role == crate::simgit::data::backend::BackendUserRole::Pending {
                                                ui.label(egui::RichText::new("⏳ Awaiting Approval").strong().color(badge_color));
                                            } else {
                                                ui.label(egui::RichText::new("✅ Approved").color(theme.text_secondary));
                                            }

                                            ui.horizontal(|ui| {
                                                if is_me {
                                                    ui.label(
                                                        egui::RichText::new("Self (Cannot demote)")
                                                            .color(theme.text_secondary)
                                                            .italics(),
                                                    );
                                                } else if u.role == crate::simgit::data::backend::BackendUserRole::Pending {
                                                    if ui.add(egui::Button::new("✅ Approve Editor").fill(egui::Color32::from_rgb(50, 140, 80))).clicked() {
                                                        role_changes.push((u.user_id.clone().unwrap_or_default(), crate::simgit::data::backend::BackendUserRole::Editor));
                                                    }
                                                    if ui.add(egui::Button::new("👁️ Approve Viewer").fill(egui::Color32::from_rgb(40, 100, 180))).clicked() {
                                                        role_changes.push((u.user_id.clone().unwrap_or_default(), crate::simgit::data::backend::BackendUserRole::Viewer));
                                                    }
                                                } else {
                                                    if u.role != crate::simgit::data::backend::BackendUserRole::Editor {
                                                        if ui.button("Make Editor").clicked() {
                                                            role_changes.push((u.user_id.clone().unwrap_or_default(), crate::simgit::data::backend::BackendUserRole::Editor));
                                                        }
                                                    }
                                                    if u.role != crate::simgit::data::backend::BackendUserRole::Viewer {
                                                        if ui.button("Make Viewer").clicked() {
                                                            role_changes.push((u.user_id.clone().unwrap_or_default(), crate::simgit::data::backend::BackendUserRole::Viewer));
                                                        }
                                                    }
                                                    if u.role != crate::simgit::data::backend::BackendUserRole::Admin {
                                                        if ui.button("Make Admin").clicked() {
                                                            role_changes.push((u.user_id.clone().unwrap_or_default(), crate::simgit::data::backend::BackendUserRole::Admin));
                                                        }
                                                    }
                                                    if ui.button("Revoke (Pending)").clicked() {
                                                        role_changes.push((u.user_id.clone().unwrap_or_default(), crate::simgit::data::backend::BackendUserRole::Pending));
                                                    }
                                                }
                                            });
                                            ui.end_row();
                                        }
                                    });
                            }
                        });

                    if do_refresh {
                        self.refresh_simgit_team_users();
                    }
                    for (target_id, new_role) in role_changes {
                        self.execute_simgit_update_role(&target_id, new_role);
                    }
                    ui.add_space(16.0);
                }
            }

            // Active Repository Sync Operations
            egui::Frame::NONE
                .fill(theme.surface_card)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    let project_name = self
                        .simgit_manager
                        .active_project
                        .as_deref()
                        .unwrap_or("None selected");
                    ui.label(
                        egui::RichText::new(format!("Active Repository: {}", project_name))
                            .strong()
                            .size(16.0)
                            .color(theme.text_primary),
                    );
                    ui.add_space(12.0);
                    let can_sync = self.simgit_manager.active_project.is_some();
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                can_sync,
                                egui::Button::new(
                                    egui::RichText::new("⬆ Push Repository & Notes")
                                        .strong()
                                        .color(theme.on_accent),
                                )
                                .fill(theme.accent)
                                .min_size(egui::vec2(180.0, 34.0)),
                            )
                            .clicked()
                        {
                            match self.execute_simgit_cloud_push() {
                                Ok(count) => {
                                    self.simgit_status_message = Some(format!(
                                        "Successfully pushed {} telemetry packet(s) and team notes to Supabase cloud repository!",
                                        count
                                    ));
                                }
                                Err(error) => {
                                    self.simgit_status_message = Some(error);
                                }
                            }
                        }

                        if ui
                            .add_enabled(
                                can_sync,
                                egui::Button::new(
                                    egui::RichText::new("⬇ Pull Remote Packets")
                                        .strong()
                                        .color(theme.on_accent),
                                )
                                .fill(theme.accent)
                                .min_size(egui::vec2(180.0, 34.0)),
                            )
                            .clicked()
                        {
                            match self.execute_simgit_cloud_pull() {
                                Ok(count) => {
                                    self.simgit_status_message = Some(format!(
                                        "Successfully synced team cloud repository! Pulled and imported {} new packet(s).",
                                        count
                                    ));
                                }
                                Err(error) => {
                                    self.simgit_status_message = Some(error);
                                }
                            }
                        }
                    });
                });
            ui.add_space(18.0);

            // Remote Repositories Section
            egui::Frame::NONE
                .fill(theme.surface_card)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Remote Cloud Repositories")
                            .strong()
                            .size(16.0)
                            .color(theme.text_primary),
                    );
                    ui.add_space(8.0);
                    
                    if ui.button("🔄 Fetch Available Cloud Repositories").clicked() {
                        let mut client = crate::simgit::data::client::SupabaseClient::new(self.settings.active_supabase_url(), self.settings.active_supabase_anon_key(), self.simgit_access_token.clone(), self.simgit_user_id.clone());
                        if let Some(user_id) = &self.simgit_user_id {
                            if let Ok(_) = client.check_connection_and_role(user_id) {
                                match client.fetch_remote_projects() {
                                    Ok(projects) => {
                                        self.simgit_remote_projects = Some(projects);
                                        self.simgit_status_message = Some("Fetched remote cloud repositories.".to_string());
                                    }
                                    Err(e) => {
                                        self.simgit_status_message = Some(e);
                                    }
                                }
                            }
                        }
                    }
                    
                    if let Some(projects) = &self.simgit_remote_projects {
                        ui.add_space(12.0);
                        if projects.is_empty() {
                            ui.label(egui::RichText::new("No repositories found on Supabase.").color(theme.text_secondary));
                        } else {
                            egui::ScrollArea::vertical().id_salt("remote_repos_scroll").max_height(200.0).show(ui, |ui| {
                                for project in projects {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&project.name).strong().color(theme.text_primary));
                                        if ui.button("Set as Active Local Repository").clicked() {
                                            self.simgit_manager.create_project(&project.name).ok();
                                            self.simgit_manager.active_project = Some(project.name.clone());
                                            self.simgit_status_message = Some(format!("Set '{}' as active local repository! You can now click Pull.", project.name));
                                        }
                                    });
                                    ui.add_space(4.0);
                                }
                            });
                        }
                    }
                });
            ui.add_space(18.0);
        }

        ui.heading(
            egui::RichText::new("Cloud Sync Architecture")
                .color(theme.text_primary)
                .size(20.0),
        );
        ui.add_space(10.0);
        for (title, detail) in [
            (
                "High-Ratio Zstandard Blob Compression",
                "To stay well under the Supabase 500MB free-tier storage limit, all raw IBT telemetry packets are compressed at Zstd level 15 before uploading, achieving massive compression ratios for high-frequency sensor streams.",
            ),
            (
                "Content-Addressed BLAKE3 Deduplication",
                "Telemetry sessions are uniquely identified by their BLAKE3 content hash. Uploads and downloads are automatically deduplicated so team members never upload or store duplicate telemetry files.",
            ),
            (
                "Role-Based Access Control (RBAC)",
                "Team access is secured via Postgres Row Level Security. Admins govern team access, Editors can push setups and driving telemetry, and Viewers can inspect telemetry and coaching notes.",
            ),
            (
                "Local-First Synchronized Workspaces",
                "SimGit operates with zero latency by caching all decompressed sessions locally in your workspace. Cloud synchronization pushes diffs and pulls remote updates seamlessly in the background.",
            ),
        ] {
            egui::Frame::NONE
                .fill(theme.surface_card)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(egui::RichText::new(detail).color(theme.text_secondary));
                });
            ui.add_space(8.0);
        }
    }

    fn execute_simgit_cloud_push(&mut self) -> Result<usize, String> {
        let Some(project) = self.simgit_manager.active_project.clone() else {
            return Err("No active repository selected to push.".to_string());
        };
        let repository = self
            .simgit_manager
            .repository(&project)
            .map_err(|e| format!("Failed to open local repository: {}", e))?;

        let mut client = crate::simgit::data::client::SupabaseClient::new(self.settings.active_supabase_url(), self.settings.active_supabase_anon_key(), self.simgit_access_token.clone(), self.simgit_user_id.clone());
        let user_id = self.simgit_user_id.clone().ok_or_else(|| "Not signed in".to_string())?;
        client.check_connection_and_role(&user_id)?;
        if !client.cached_role.can_push() {
            return Err("Your current user role on Supabase is not an Editor or Admin and cannot push telemetry packets.".to_string());
        }

        let project_id = client.ensure_project(&project)?;

        let existing_remote = client.fetch_remote_packets(&project_id).unwrap_or_default();
        let mut pushed_count = 0;

        for record in repository.telemetry() {
            if existing_remote.iter().any(|r| r.telemetry_id == record.id) {
                continue; // Already present on remote repository
            }

            let ibt_path = repository
                .resolve_ibt(&record.id)
                .map_err(|e| format!("Failed to resolve local IBT file for {}: {}", record.original_name, e))?;
            let raw_bytes = std::fs::read(&ibt_path)
                .map_err(|e| format!("Failed to read file {}: {}", ibt_path.display(), e))?;

            let fastest_lap = record
                .laps
                .iter()
                .min_by(|a, b| a.duration_seconds.partial_cmp(&b.duration_seconds).unwrap_or(std::cmp::Ordering::Equal))
                .map(|l| l.duration_seconds);

            let meta = crate::simgit::data::client::RemotePacketMetadata {
            id: None,
            project_id: project_id.clone(),
            telemetry_id: record.id.clone(),
            original_name: record.original_name.clone(),
            vehicle_name: Some(record.car.clone()),
            venue_name: Some(record.venue.clone()),
            fastest_lap_seconds: record.laps.iter().map(|l| l.duration_seconds).min_by(|a, b| a.partial_cmp(b).unwrap()),
            lap_count: record.laps.len() as i32,
            uploaded_by: None,
            storage_file_path: format!("{}/{}.ibt", project_id, record.id),
        };

            let local_notes = repository.notes_for(&record.id);
            let mut remote_notes = Vec::new();
            for note in local_notes {
                let (viewport_start, viewport_end) = note.context.time_range().unwrap_or((0.0, 0.0));
                remote_notes.push(crate::simgit::data::client::RemoteAnalysisNote {
                    id: None,
                    packet_id: None,
                    note_id: note.id.clone(),
                    author: note.author.clone(),
                    objective: note.objective.clone(),
                    body: note.body.clone(),
                    color: note.color.label().to_string(),
                    lap_number: note.context.lap_number,
                    viewport_start: Some(viewport_start),
                    viewport_end: Some(viewport_end),
                    section_delta: note.context.section_delta,
                    worksheet: Some(note.context.worksheet.clone()),
                });
            }

            client.push_packet(&meta, &raw_bytes, &remote_notes)?;
            pushed_count += 1;
        }

        Ok(pushed_count)
    }

    fn execute_simgit_cloud_pull(&mut self) -> Result<usize, String> {
        let Some(project) = self.simgit_manager.active_project.clone() else {
            return Err("No active repository selected to pull into.".to_string());
        };
        let mut repository = self
            .simgit_manager
            .repository(&project)
            .map_err(|e| format!("Failed to open local repository: {}", e))?;

        let mut client = crate::simgit::data::client::SupabaseClient::new(self.settings.active_supabase_url(), self.settings.active_supabase_anon_key(), self.simgit_access_token.clone(), self.simgit_user_id.clone());
        let user_id = self.simgit_user_id.clone().ok_or_else(|| "Not signed in".to_string())?;
        client.check_connection_and_role(&user_id)?;
        if !client.cached_role.can_pull() {
            return Err("Your current user role on Supabase is Pending and lacks permission to pull telemetry packets.".to_string());
        }

        let project_id = client.ensure_project(&project)?;
        let remote_packets = client.fetch_remote_packets(&project_id)?;
        let mut pulled_count = 0;

        for packet in remote_packets {
            let exists_local = repository.telemetry().iter().any(|r| r.id == packet.telemetry_id);
            let remote_notes = if !exists_local {
                if let Some(ref pid) = packet.id {
                    let (bytes, notes) = client.pull_packet(pid, &packet.storage_file_path)?;
                    let tmp_name = format!("tmp_{}.ibt", packet.telemetry_id);
                    let tmp_path = self.simgit_manager.root().join(&tmp_name);
                    if let Err(e) = std::fs::write(&tmp_path, &bytes) {
                        let _ = std::fs::remove_file(&tmp_path);
                        return Err(format!("Failed to save temporary download file: {}", e));
                    }
                    match repository.import_ibt(&tmp_path) {
                        Ok(_) => {
                            let _ = std::fs::remove_file(&tmp_path);
                            pulled_count += 1;
                        }
                        Err(e) => {
                            let _ = std::fs::remove_file(&tmp_path);
                            return Err(format!("Failed to import downloaded packet {}: {}", packet.original_name, e));
                        }
                    }
                    notes
                } else {
                    Vec::new()
                }
            } else if let Some(ref pid) = packet.id {
                client.fetch_remote_notes(pid).unwrap_or_default()
            } else {
                Vec::new()
            };

            for r_note in remote_notes {
                let color = match r_note.color.to_lowercase().as_str() {
                    "red" => crate::simgit::repository::NoteColor::Red,
                    "yellow" => crate::simgit::repository::NoteColor::Yellow,
                    "orange" => crate::simgit::repository::NoteColor::Orange,
                    "green" => crate::simgit::repository::NoteColor::Green,
                    "purple" => crate::simgit::repository::NoteColor::Purple,
                    _ => crate::simgit::repository::NoteColor::Blue,
                };
                let context = crate::simgit::repository::AnalysisContext {
                    cursor_seconds: r_note.viewport_start,
                    viewport: match (r_note.viewport_start, r_note.viewport_end) {
                        (Some(s), Some(e)) if s < e => Some((s, e)),
                        _ => None,
                    },
                    lap_number: r_note.lap_number,
                    worksheet: r_note.worksheet.unwrap_or_else(|| "Driver".to_string()),
                    cyan_reference: None,
                    secondary_reference: None,
                    track_map: None,
                    section_delta: r_note.section_delta,
                };
                let note = crate::simgit::repository::AnalysisNote {
                    id: r_note.note_id,
                    telemetry_id: packet.telemetry_id.clone(),
                    author: r_note.author,
                    objective: r_note.objective,
                    body: r_note.body,
                    color,
                    context,
                    created_at: 0,
                    updated_at: 0,
                };
                let _ = repository.insert_note(note);
            }
        }

        Ok(pulled_count)
    }

    fn refresh_simgit_team_users(&mut self) {
        if let (Some(project_name), Some(token), Some(uid)) = (&self.simgit_manager.active_project, &self.simgit_access_token, &self.simgit_user_id) {
            let mut client = crate::simgit::data::client::SupabaseClient::new(
                self.settings.active_supabase_url(),
                self.settings.active_supabase_anon_key(),
                Some(token.clone()),
                Some(uid.clone()),
            );
            if let Ok(proj_uuid) = client.ensure_project(project_name) {
                if let Ok(users) = client.fetch_project_members(&proj_uuid) {
                    self.simgit_team_users = users;
                }
            }
        }
    }

    fn execute_simgit_update_role(&mut self, target_email: &str, new_role: crate::simgit::data::backend::BackendUserRole) {
        if let (Some(project_name), Some(token), Some(uid)) = (&self.simgit_manager.active_project, &self.simgit_access_token, &self.simgit_user_id) {
            let mut client = crate::simgit::data::client::SupabaseClient::new(
                self.settings.active_supabase_url(),
                self.settings.active_supabase_anon_key(),
                Some(token.clone()),
                Some(uid.clone()),
            );
            if let Ok(proj_uuid) = client.ensure_project(project_name) {
                if let Err(e) = client.upsert_project_member(&proj_uuid, target_email, new_role) {
                    self.simgit_status_message = Some(e);
                } else {
                    self.simgit_status_message = Some(format!("Updated role for {}", target_email));
                }
            }
        }
    }
}

fn draw_empty_repository(ui: &mut egui::Ui, theme: AppTheme, message: &str) {
    egui::Frame::NONE
        .fill(theme.surface_panel)
        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
        .corner_radius(9.0)
        .inner_margin(18.0)
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(message).color(theme.text_secondary)).wrap(),
            );
        });
}

fn format_import_summary(summary: &ImportBatchSummary) -> String {
    let mut message = format!(
        "Imported {} file(s); {} duplicate(s) reused.",
        summary.imported, summary.already_present
    );
    if !summary.failures.is_empty() {
        message.push_str(&format!(
            " {} failed: {}",
            summary.failures.len(),
            summary.failures.join(" | ")
        ));
    }
    message
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn format_timestamp(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|time| time.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "Unknown time".to_owned())
}

fn format_context(note: &AnalysisNote) -> String {
    let time = note
        .context
        .cursor_seconds
        .map(|seconds| format!("{seconds:.3}s"))
        .unwrap_or_else(|| "No cursor".to_owned());
    let lap = note
        .context
        .lap_number
        .map(|lap| format!("Lap {lap}"))
        .unwrap_or_else(|| "No lap".to_owned());
    if let Some(delta) = note.context.section_delta {
        let delta_str = crate::simgit::repository::format_section_delta(delta);
        format!("{} | {} | {} | {}", note.context.worksheet, lap, time, delta_str)
    } else {
        format!("{} | {} | {}", note.context.worksheet, lap, time)
    }
}

fn normalize_analysis_draft(
    draft: &mut crate::SimGitAnalysisDraft,
    records: &[crate::simgit::repository::TelemetryRecord],
) {
    draft
        .selected_telemetry
        .retain(|telemetry_id| records.iter().any(|record| record.id == *telemetry_id));
    normalize_analysis_role(
        records,
        &draft.selected_telemetry,
        &mut draft.baseline_telemetry,
        &mut draft.baseline_lap,
    );
    normalize_analysis_role(
        records,
        &draft.selected_telemetry,
        &mut draft.reference_telemetry,
        &mut draft.reference_lap,
    );
}

fn normalize_analysis_role(
    records: &[crate::simgit::repository::TelemetryRecord],
    selected: &std::collections::HashSet<String>,
    telemetry_id: &mut Option<String>,
    lap_number: &mut Option<i32>,
) {
    let current_is_selected = telemetry_id
        .as_ref()
        .is_some_and(|id| selected.contains(id));
    if !current_is_selected {
        *telemetry_id = records
            .iter()
            .find(|record| selected.contains(&record.id))
            .map(|record| record.id.clone());
        *lap_number = None;
    }
    let Some(record) = telemetry_id
        .as_ref()
        .and_then(|id| records.iter().find(|record| record.id == *id))
    else {
        *lap_number = None;
        return;
    };
    let lap_is_valid =
        lap_number.is_some_and(|lap| record.laps.iter().any(|summary| summary.lap_number == lap));
    if !lap_is_valid {
        *lap_number = record.fastest_lap().map(|lap| lap.lap_number);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_analysis_role(
    ui: &mut egui::Ui,
    theme: AppTheme,
    title: &str,
    file_combo_id: &str,
    lap_combo_id: &str,
    records: &[crate::simgit::repository::TelemetryRecord],
    selected: &std::collections::HashSet<String>,
    telemetry_id: &mut Option<String>,
    lap_number: &mut Option<i32>,
) {
    ui.label(
        egui::RichText::new(title)
            .strong()
            .size(16.0)
            .color(theme.text_primary),
    );
    ui.add_space(5.0);
    let selected_name = telemetry_id
        .as_ref()
        .and_then(|id| records.iter().find(|record| record.id == *id))
        .map(|record| record.original_name.as_str())
        .unwrap_or("Select file");
    let previous_id = telemetry_id.clone();
    egui::ComboBox::from_id_salt(file_combo_id)
        .width(ui.available_width())
        .selected_text(selected_name)
        .show_ui(ui, |ui| {
            for record in records
                .iter()
                .filter(|record| selected.contains(&record.id))
            {
                ui.selectable_value(telemetry_id, Some(record.id.clone()), &record.original_name);
            }
        });
    if *telemetry_id != previous_id {
        *lap_number = telemetry_id
            .as_ref()
            .and_then(|id| records.iter().find(|record| record.id == *id))
            .and_then(|record| record.fastest_lap())
            .map(|lap| lap.lap_number);
    }
    let selected_record = telemetry_id
        .as_ref()
        .and_then(|id| records.iter().find(|record| record.id == *id));
    let selected_lap_text = lap_number
        .and_then(|number| {
            selected_record.and_then(|record| {
                record
                    .laps
                    .iter()
                    .find(|lap| lap.lap_number == number)
                    .map(|lap| format!("Lap {} | {:.3}s", lap.lap_number, lap.duration_seconds))
            })
        })
        .unwrap_or_else(|| "Select lap".to_owned());
    egui::ComboBox::from_id_salt(lap_combo_id)
        .width(ui.available_width())
        .selected_text(selected_lap_text)
        .show_ui(ui, |ui| {
            if let Some(record) = selected_record {
                for lap in &record.laps {
                    ui.selectable_value(
                        lap_number,
                        Some(lap.lap_number),
                        format!("Lap {} | {:.3}s", lap.lap_number, lap.duration_seconds),
                    );
                }
            }
        });
}

fn analysis_draft_ready(draft: &crate::SimGitAnalysisDraft) -> bool {
    let baseline_ready = draft
        .baseline_telemetry
        .as_ref()
        .is_some_and(|id| draft.selected_telemetry.contains(id) && draft.baseline_lap.is_some());
    let reference_ready = draft
        .reference_telemetry
        .as_ref()
        .is_some_and(|id| draft.selected_telemetry.contains(id) && draft.reference_lap.is_some());
    baseline_ready && reference_ready
}

fn swap_analysis_roles(draft: &mut crate::SimGitAnalysisDraft) {
    std::mem::swap(
        &mut draft.baseline_telemetry,
        &mut draft.reference_telemetry,
    );
    std::mem::swap(&mut draft.baseline_lap, &mut draft.reference_lap);
}

#[cfg(test)]
mod tests {
    use super::{analysis_draft_ready, normalize_analysis_draft, swap_analysis_roles};
    use crate::simgit::repository::{LapSummary, TelemetryRecord};

    fn record(id: &str, laps: &[(i32, f64)]) -> TelemetryRecord {
        TelemetryRecord {
            id: id.to_owned(),
            original_name: format!("{id}.ibt"),
            object_name: format!("{id}.ibt.zst"),
            imported_at: 1,
            uncompressed_size: 100,
            compressed_size: 50,
            car: "GT3".to_owned(),
            venue: "Spa".to_owned(),
            track_id: 1,
            laps: laps
                .iter()
                .map(|(lap_number, duration_seconds)| LapSummary {
                    lap_number: *lap_number,
                    duration_seconds: *duration_seconds,
                })
                .collect(),
        }
    }

    #[test]
    fn analysis_defaults_each_role_to_the_selected_files_fastest_lap() {
        let records = [
            record("baseline", &[(2, 91.0), (3, 89.5)]),
            record("reference", &[(4, 90.0), (5, 88.8)]),
        ];
        let mut draft = crate::SimGitAnalysisDraft::default();
        draft.selected_telemetry = ["baseline".to_owned(), "reference".to_owned()]
            .into_iter()
            .collect();
        draft.baseline_telemetry = Some("baseline".to_owned());
        draft.reference_telemetry = Some("reference".to_owned());

        normalize_analysis_draft(&mut draft, &records);

        assert_eq!(draft.baseline_lap, Some(3));
        assert_eq!(draft.reference_lap, Some(5));
        assert!(analysis_draft_ready(&draft));
    }

    #[test]
    fn removing_a_role_file_falls_back_to_a_remaining_selection() {
        let records = [
            record("baseline", &[(2, 91.0)]),
            record("reference", &[(5, 88.8)]),
        ];
        let mut draft = crate::SimGitAnalysisDraft::default();
        draft.selected_telemetry.insert("reference".to_owned());
        draft.baseline_telemetry = Some("baseline".to_owned());
        draft.baseline_lap = Some(2);

        normalize_analysis_draft(&mut draft, &records);

        assert_eq!(draft.baseline_telemetry.as_deref(), Some("reference"));
        assert_eq!(draft.baseline_lap, Some(5));
    }

    #[test]
    fn swapping_analysis_roles_moves_each_file_and_lap_together() {
        let mut draft = crate::SimGitAnalysisDraft {
            baseline_telemetry: Some("baseline".to_owned()),
            baseline_lap: Some(3),
            reference_telemetry: Some("reference".to_owned()),
            reference_lap: Some(7),
            ..Default::default()
        };

        swap_analysis_roles(&mut draft);

        assert_eq!(
            (
                draft.baseline_telemetry.as_deref(),
                draft.baseline_lap,
                draft.reference_telemetry.as_deref(),
                draft.reference_lap,
            ),
            (Some("reference"), Some(7), Some("baseline"), Some(3))
        );
    }
}
