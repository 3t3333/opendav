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
            egui::RichText::new("Sync Foundation")
                .color(theme.text_primary)
                .size(25.0),
        );
        ui.label(
            egui::RichText::new(
                "Supabase transport is not enabled in this MVP. Repository objects are already prepared for it.",
            )
            .color(theme.text_secondary),
        );
        ui.add_space(18.0);
        for (title, detail) in [
            (
                "Compressed objects",
                "Each IBT is stored once as a Zstandard object.",
            ),
            (
                "Stable identity",
                "BLAKE3 IDs make uploads and downloads safely deduplicated.",
            ),
            (
                "Portable metadata",
                "Repository records and contextual notes use a versioned JSON manifest.",
            ),
            (
                "Verified downloads",
                "Received objects are decompressed and hash-checked before analysis.",
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
