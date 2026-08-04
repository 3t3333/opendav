use crate::config::theme::AppTheme;
use crate::OpenDavApp;

impl OpenDavApp {
    pub fn draw_settings_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let theme = AppTheme::for_mode(is_dark);

        egui::ScrollArea::vertical()
            .id_salt("settings_page_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.set_max_width(860.0);
                    ui.heading(
                        egui::RichText::new("Application Settings")
                            .color(theme.text_primary)
                            .size(30.0),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "Changes are saved automatically as soon as you make them.",
                        )
                        .color(theme.text_secondary)
                        .size(14.0),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("AUTOSAVE ON")
                            .color(theme.success)
                            .strong()
                            .size(12.0),
                    );
                    ui.add_space(24.0);

                    egui::Frame::NONE
                        .fill(theme.surface_elevated)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .corner_radius(10.0)
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                ui.heading(
                                    egui::RichText::new("Algorithm Tuning")
                                        .color(theme.text_primary)
                                        .size(21.0),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Tune how telemetry is interpreted without changing the source data.",
                                    )
                                    .color(theme.text_secondary)
                                    .size(14.0),
                                );
                                ui.add_space(18.0);

                                let is_narrow = ui.available_width() < 540.0;
                                let label = egui::RichText::new("Corner merge gap")
                                    .color(theme.text_primary)
                                    .strong()
                                    .size(15.0);
                                let draw_threshold = |ui: &mut egui::Ui, app: &mut Self| {
                                    let response = ui.add(
                                        egui::DragValue::new(
                                            &mut app.settings.corner_merge_threshold,
                                        )
                                        .speed(0.5)
                                        .range(5.0..=100.0)
                                        .suffix(" m"),
                                    );

                                    if response.changed() {
                                        let threshold = app.settings.corner_merge_threshold;
                                        for session in &mut app.sessions {
                                            session.recalculate_sectors(threshold);
                                        }
                                        app.settings.save();
                                    }
                                };

                                if is_narrow {
                                    ui.label(label);
                                    ui.add_space(6.0);
                                    draw_threshold(ui, self);
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(label);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| draw_threshold(ui, self),
                                        );
                                    });
                                }
                                ui.add_space(6.0);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(
                                            "Controls how aggressively tight chicanes are merged into a single corner.",
                                        )
                                        .color(theme.text_tertiary)
                                        .size(13.0),
                                    )
                                    .wrap(),
                                );

                                ui.add_space(18.0);
                                ui.separator();
                                ui.add_space(18.0);

                                let unit_label = egui::RichText::new("Unit system")
                                    .color(theme.text_primary)
                                    .strong()
                                    .size(15.0);
                                let draw_unit_toggle = |ui: &mut egui::Ui, app: &mut Self| {
                                    let mut use_metric = app.settings.use_metric;
                                    let response = ui.checkbox(
                                        &mut use_metric,
                                        egui::RichText::new("Metric (km/h, mm, kg)")
                                            .color(theme.text_primary)
                                            .size(14.0),
                                    );
                                    if response.changed() {
                                        app.settings.use_metric = use_metric;
                                        app.settings.save();
                                    }
                                };

                                if is_narrow {
                                    ui.label(unit_label);
                                    ui.add_space(6.0);
                                    draw_unit_toggle(ui, self);
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(unit_label);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| draw_unit_toggle(ui, self),
                                        );
                                    });
                                }
                                ui.add_space(6.0);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(
                                            "Turn this off to display imperial units such as mph, inches, and pounds.",
                                        )
                                        .color(theme.text_tertiary)
                                        .size(13.0),
                                    )
                                    .wrap(),
                                );
                            });
                        });

                    ui.add_space(16.0);

                    egui::Frame::NONE
                        .fill(theme.surface_card)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .corner_radius(10.0)
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.heading(
                                egui::RichText::new("Graph Display")
                                    .color(theme.text_primary)
                                    .size(21.0),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "Control reference grid lines across every telemetry worksheet.",
                                )
                                .color(theme.text_secondary)
                                .size(14.0),
                            );
                            ui.add_space(18.0);

                            let visibility = ui.checkbox(
                                &mut self.settings.show_graph_grid,
                                egui::RichText::new("Show graph grid lines")
                                    .color(theme.text_primary)
                                    .size(14.0),
                            );
                            if visibility.changed() {
                                self.settings.save();
                            }

                            ui.add_space(12.0);
                            ui.add_enabled_ui(self.settings.show_graph_grid, |ui| {
                                ui.label(
                                    egui::RichText::new("Grid opacity")
                                        .color(theme.text_primary)
                                        .strong()
                                        .size(15.0),
                                );
                                let opacity = ui.add(
                                    egui::Slider::new(
                                        &mut self.settings.graph_grid_opacity,
                                        0.0..=1.0,
                                    )
                                    .show_value(true),
                                );
                                if opacity.changed() {
                                    self.settings.save();
                                }
                            });
                        });

                    ui.add_space(16.0);

                    egui::Frame::NONE
                        .fill(theme.surface_card)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .corner_radius(10.0)
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                ui.heading(
                                    egui::RichText::new("Map Integration")
                                        .color(theme.text_primary)
                                        .size(21.0),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Connect Mapbox for high-resolution satellite imagery on custom tracks.",
                                    )
                                    .color(theme.text_secondary)
                                    .size(14.0),
                                );
                                ui.add_space(18.0);

                                let is_narrow = ui.available_width() < 540.0;
                                let label = egui::RichText::new("Mapbox API key (optional)")
                                    .color(theme.text_primary)
                                    .strong()
                                    .size(15.0);
                                let draw_api_key = |ui: &mut egui::Ui, app: &mut Self| {
                                    let width = ui.available_width().min(360.0);
                                    let response = ui.add_sized(
                                        [width, 26.0],
                                        egui::TextEdit::singleline(
                                            &mut app.settings.mapbox_api_key,
                                        )
                                        .password(true)
                                        .hint_text("Paste API key"),
                                    );
                                    if response.changed() {
                                        app.settings.save();
                                    }
                                };

                                if is_narrow {
                                    ui.label(label);
                                    ui.add_space(6.0);
                                    draw_api_key(ui, self);
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(label);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| draw_api_key(ui, self),
                                        );
                                    });
                                }
                                ui.add_space(8.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            "Overrides Google Maps imagery when configured.",
                                        )
                                        .color(theme.text_tertiary)
                                        .size(13.0),
                                    );
                                    ui.hyperlink_to(
                                        egui::RichText::new("Get a free Mapbox key")
                                            .color(theme.accent_text)
                                            .size(13.0),
                                        "https://account.mapbox.com/auth/signup/",
                                    );
                                });
                            });
                        });

                    ui.add_space(16.0);

                    egui::Frame::NONE
                        .fill(theme.surface_card)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .corner_radius(10.0)
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                ui.heading(
                                    egui::RichText::new("SimGit Cloud Sync (Supabase)")
                                        .color(theme.text_primary)
                                        .size(21.0),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Connect a team Supabase instance for collaborative telemetry packets, shared analysis notes, and role-based access control.",
                                    )
                                    .color(theme.text_secondary)
                                    .size(14.0),
                                );
                                ui.add_space(18.0);

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Supabase Project URL").color(theme.text_primary).strong().size(15.0));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let width = ui.available_width().min(360.0);
                                        let res = ui.add_sized([width, 26.0], egui::TextEdit::singleline(&mut self.settings.supabase_url).hint_text("https://xxxx.supabase.co"));
                                        if res.changed() { self.settings.save(); }
                                    });
                                });
                                ui.add_space(8.0);

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Supabase Anon Key").color(theme.text_primary).strong().size(15.0));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let width = ui.available_width().min(360.0);
                                        let res = ui.add_sized([width, 26.0], egui::TextEdit::singleline(&mut self.settings.supabase_anon_key).password(true).hint_text("Paste anon pub key"));
                                        if res.changed() { self.settings.save(); }
                                    });
                                });

                                ui.add_space(16.0);
                                ui.separator();
                                ui.add_space(12.0);

                                ui.label(
                                    egui::RichText::new("Database Setup & Initialization")
                                        .color(theme.text_primary)
                                        .strong()
                                        .size(15.0),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Setting up your own server is free and only takes 2 minutes. Copy the SQL setup script and execute it inside your Supabase project's SQL Editor.\n\nIMPORTANT: You must go to your Supabase dashboard -> Authentication -> Providers -> Email, and disable \"Confirm email\" so that accounts can sign up smoothly without requiring email confirmation links.\n\nNote: The very first account created on your database automatically receives the Admin role (never pending), unlocking the inline Admin Dashboard in the SimGit Sync tab to approve and manage subsequent team accounts.",
                                    )
                                    .color(theme.text_secondary)
                                    .size(13.0),
                                );
                                ui.add_space(10.0);

                                ui.horizontal_wrapped(|ui| {
                                    if ui.add(egui::Button::new("📋 Copy Setup SQL Script").min_size(egui::vec2(160.0, 28.0))).clicked() {
                                        ui.ctx().copy_text(crate::simgit::data::backend::SUPABASE_INIT_SQL.to_string());
                                    }
                                    ui.add_space(12.0);
                                    if ui.add(
                                        egui::Button::new(
                                            egui::RichText::new("🗑️ Copy Database Deletion Script")
                                                .color(egui::Color32::WHITE)
                                        )
                                        .fill(egui::Color32::from_rgb(220, 50, 50))
                                        .min_size(egui::vec2(160.0, 28.0))
                                    ).clicked() {
                                        ui.ctx().copy_text(crate::simgit::data::backend::SUPABASE_WIPE_SQL.to_string());
                                    }
                                    ui.add_space(12.0);
                                    ui.hyperlink_to(
                                        egui::RichText::new("Create Free Supabase Database ↗")
                                            .color(theme.accent_text)
                                            .size(13.0),
                                        "https://supabase.com/dashboard/projects",
                                    );
                                });
                            });
                        });

                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new("No save action is required.")
                            .color(theme.text_tertiary)
                            .italics()
                            .size(13.0),
                    );
                    ui.add_space(24.0);
                });
            });
    }
}
