use crate::OpenDavApp;
use crate::ActivePage;
use crate::config::worksheet::{WorksheetTab, WorksheetConfig, ACCENT_COLOR, SUB_ACCENT_COLOR};
use crate::signals::processing::{
    get_fastest_lap, format_lap_time, format_sector_time, get_lap_time_at_distance
};

impl OpenDavApp {
    pub fn draw_dashboard_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        if !self.session_loaded || self.session.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                    ui.label(egui::RichText::new("Please load an iRacing .ibt file from the top taskbar.").color(egui::Color32::GRAY));
                });
            });
            return;
        }

        // SAFE IMMUTABLE CLONING TO RESOLVE RUST BORROW CHECKER CLOSURE LOCKS
        let session_ref = self.session.as_ref().unwrap();
        let car = session_ref.car.clone();
        let venue = session_ref.venue.clone();
        let air_temp = session_ref.air_temp.clone();
        let surface_temp = session_ref.surface_temp.clone();
        let total_session_time = session_ref.total_session_time;
        let lap_times = session_ref.lap_times.clone();

        ui.heading(egui::RichText::new("Dashboard").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
        ui.add_space(8.0);

        // 1. Session Metadata Grid Card
        ui.group(|ui| {
            ui.columns(4, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(egui::RichText::new("VEHICLE").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.label(egui::RichText::new(&car).strong().color(ACCENT_COLOR));
                });
                cols[1].vertical(|ui| {
                    ui.label(egui::RichText::new("VENUE").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.label(egui::RichText::new(&venue).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                });
                cols[2].vertical(|ui| {
                    ui.label(egui::RichText::new("AIR TEMP").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.label(egui::RichText::new(&air_temp).strong().color(egui::Color32::from_rgb(50, 205, 50)));
                });
                cols[3].vertical(|ui| {
                    ui.label(egui::RichText::new("TRACK TEMP").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.label(egui::RichText::new(&surface_temp).strong().color(egui::Color32::from_rgb(50, 205, 50)));
                });
            });
        });

        ui.add_space(10.0);

        // 2. High-End Session Statistics Cards
        ui.columns(3, |cols| {
            cols[0].group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("TOTAL SESSION TIME").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.add_space(4.0);
                    ui.heading(egui::RichText::new(format_lap_time(total_session_time)).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                });
            });

            cols[1].group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("TOTAL VALID LAPS").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.add_space(4.0);
                    ui.heading(egui::RichText::new(format!("{} Laps", lap_times.len())).strong().color(ACCENT_COLOR));
                });
            });

            cols[2].group(|ui| {
                ui.vertical_centered(|ui| {
                    let avg_lap = {
                        let filtered: Vec<&(i32, f64)> = lap_times.iter()
                            .filter(|(lap_num, _)| *lap_num > 3)
                            .collect();
                        if !filtered.is_empty() {
                            let sum: f64 = filtered.iter().map(|val| val.1).sum();
                            sum / filtered.len() as f64
                        } else if !lap_times.is_empty() {
                            let sum: f64 = lap_times.iter().map(|val| val.1).sum();
                            sum / lap_times.len() as f64
                        } else {
                            0.0
                        }
                    };
                    ui.label(egui::RichText::new("AVERAGE LAP TIME").color(egui::Color32::DARK_GRAY).small().strong());
                    ui.add_space(4.0);
                    ui.heading(egui::RichText::new(format_lap_time(avg_lap)).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                });
            });
        });

        ui.add_space(15.0);

        let fastest_lap = get_fastest_lap(&lap_times);

        // 3. Stacked lower Dashboard (Top: Laps List, Bottom: Huge Track Map SVG!)
        ui.vertical(|ui| {
            let sheet_hdr_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
            ui.label(egui::RichText::new("VALID LAP SHEET").color(sheet_hdr_color).strong().size(11.0));
            ui.add_space(4.0);

            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                egui::Grid::new("valid_laps_grid")
                    .striped(true)
                    .min_col_width(200.0)
                    .spacing([24.0, 10.0])
                    .show(ui, |ui| {
                        let cols = 4;
                        let mut col_count = 0;
                        for (lap_num, duration) in &lap_times {
                            let is_fastest = *lap_num == fastest_lap;
                            let is_selected = self.selected_lap == Some(*lap_num);
                            
                            let mut row_text = format!("Lap {} : {}", lap_num, format_lap_time(*duration));
                            if is_fastest {
                                row_text += " ★ FASTEST";
                            }
                            
                            let row_color = if is_selected {
                                ACCENT_COLOR
                            } else if is_fastest {
                                SUB_ACCENT_COLOR
                            } else {
                                if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }
                            };
                            
                            let border_color = if is_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };
                            let btn_resp = egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, border_color))
                                .rounding(4.0)
                                .inner_margin(egui::Margin::symmetric(6, 3))
                                .show(ui, |ui| {
                                    ui.selectable_label(false, egui::RichText::new(row_text).color(row_color).strong())
                                }).inner;

                            if btn_resp.clicked() {
                                self.selected_lap = Some(*lap_num);
                                if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == *lap_num) {
                                    let (_, start_t, _end_t) = self.lap_ranges[pos];
                                    self.cursor_x = Some(start_t);
                                    self.reset_bounds_flag = true;
                                }
                                self.update_sector_deltas();
                            }
                            
                            col_count += 1;
                            if col_count >= cols {
                                ui.end_row();
                                col_count = 0;
                            }
                        }
                    });
            });
        });

        ui.add_space(15.0);
        let map_hdr_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
        ui.label(egui::RichText::new(venue.to_uppercase()).color(map_hdr_color).strong().size(11.0));
        ui.add_space(4.0);

        ui.group(|ui| {
            self.draw_interactive_track_map(ui, 340.0);
        });

        ui.add_space(15.0);
        ui.vertical_centered(|ui| {
            if ui.button(egui::RichText::new("📈 OPEN GRAPHS WORKSPACE").strong().size(12.0).color(if is_dark { egui::Color32::from_rgb(10, 10, 10) } else { egui::Color32::WHITE })).clicked() {
                self.active_page = ActivePage::Graphs;
                // Default to fastest lap on first entering graphs page
                if self.selected_lap.is_none() {
                    self.selected_lap = Some(fastest_lap);
                    self.rebuild_points_cache();
                }
            }
        });
    }

    pub fn draw_graphs_page(&mut self, ui: &mut egui::Ui) {
        if !self.session_loaded || self.session.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                    ui.label(egui::RichText::new("Please load an iRacing .ibt file from the top taskbar to view graphs.").color(egui::Color32::GRAY));
                });
            });
            return;
        }

        // 1. HORIZONTAL MOTEC WORKSHEET TABS AT THE TOP!
        ui.horizontal(|ui| {
            let tab_style = ui.style_mut();
            tab_style.spacing.button_padding = egui::vec2(12.0, 8.0); // Perfect, professional tab sizing

            ui.selectable_value(&mut self.active_worksheet, WorksheetTab::Basic, "1. Basic (Inputs)");
            ui.selectable_value(&mut self.active_worksheet, WorksheetTab::DynamicRake, "2. Dynamic Rake");
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Calculate if tab, page, or layout was switched this frame to trigger shared viewport boundary syncing!
        let mut is_tab_switch = false;
        if Some(self.active_worksheet) != self.previous_worksheet {
            is_tab_switch = true;
            self.previous_worksheet = Some(self.active_worksheet);
        }
        if Some(self.active_page) != self.previous_page {
            is_tab_switch = true;
            self.previous_page = Some(self.active_page);
        }
        if Some(self.show_graphs_track_map) != self.previous_show_graphs_track_map {
            is_tab_switch = true;
            self.previous_show_graphs_track_map = Some(self.show_graphs_track_map);
        }

        // 2. ACTIVE WORKSHEET PLOTTING AREA (SINGLE INTEGRATED HIGH-PERFORMANCE PLOT ENVIRONMENT!)
        let config = match self.active_worksheet {
            WorksheetTab::Basic => Some(WorksheetConfig::basic()),
            WorksheetTab::DynamicRake => Some(WorksheetConfig::rake()),
            _ => None,
        };

        if self.show_graphs_track_map {
            let total_h = ui.available_height();
            let half_h = (total_h - 20.0) / 2.0;

            ui.allocate_ui(egui::vec2(ui.available_width(), half_h), |ui| {
                if let Some(cfg) = &config {
                    let canvas_id = match self.active_worksheet {
                        WorksheetTab::Basic => "basic_worksheet_canvas",
                        _ => "rake_worksheet_canvas",
                    };
                    self.draw_motec_plot(ui, canvas_id, cfg, is_tab_switch);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Worksheet Active").heading().color(ACCENT_COLOR));
                            ui.label(egui::RichText::new("D3 D&D replacement plotters are standing by for this section...").color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new("(Phase 2 Rewrite Roadmap placeholder)").small().color(egui::Color32::DARK_GRAY));
                        });
                    });
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.group(|ui| {
                self.draw_interactive_track_map(ui, half_h - 10.0);
            });
        } else {
            if let Some(cfg) = &config {
                let canvas_id = match self.active_worksheet {
                    WorksheetTab::Basic => "basic_worksheet_canvas",
                    _ => "rake_worksheet_canvas",
                };
                self.draw_motec_plot(ui, canvas_id, cfg, is_tab_switch);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Worksheet Active").heading().color(ACCENT_COLOR));
                        ui.label(egui::RichText::new("D3 D&D replacement plotters are standing by for this section...").color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new("(Phase 2 Rewrite Roadmap placeholder)").small().color(egui::Color32::DARK_GRAY));
                    });
                });
            }
        }
    }

    pub fn draw_reports_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        if !self.session_loaded || self.session.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                    ui.label(egui::RichText::new("Please load an iRacing .ibt file from the top taskbar to view reports.").color(egui::Color32::GRAY));
                });
            });
            return;
        }

        let session_ref = self.session.as_ref().unwrap();
        let venue = session_ref.venue.clone();

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Sector Analysis Report").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(venue.to_uppercase()).strong().color(ACCENT_COLOR));
            });
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        if self.lap_data_cache.is_empty() || self.sectors.is_empty() {
            ui.label("No sector or lap data available for report.");
        } else {
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    let mut visible_laps: Vec<&crate::signals::processing::LapData> = self.lap_data_cache.iter()
                        .filter(|lap| lap.lap_num > 3)
                        .collect();
                    if visible_laps.is_empty() {
                        visible_laps = self.lap_data_cache.iter().collect();
                    }

                    let best_total_time = visible_laps.iter()
                        .map(|lap| lap.time.last().copied().unwrap_or(0.0))
                        .filter(|&t| t > 0.0)
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(0.0);

                    egui::ScrollArea::both()
                        .id_source("sector_report_scroll")
                        .show(ui, |ui| {
                            egui::Grid::new("sector_report_grid")
                                .striped(true)
                                .min_col_width(85.0)
                                .spacing([20.0, 12.0])
                                .show(ui, |ui| {
                                    // Header Row
                                    ui.label(egui::RichText::new("Sector / Corner").strong());
                                    for lap in &visible_laps {
                                        ui.label(egui::RichText::new(format!("Lap {}", lap.lap_num)).strong());
                                    }
                                    ui.label(egui::RichText::new("Optimal").strong().color(ACCENT_COLOR));
                                    ui.end_row();

                                    // Sector split rows
                                    for (s_idx, sector) in self.sectors.iter().enumerate() {
                                        ui.label(egui::RichText::new(&sector.name).strong());

                                        let best_s_time = self.sector_bests.get(s_idx).copied().unwrap_or(0.0);

                                        for lap in &visible_laps {
                                            let t_start = get_lap_time_at_distance(&lap.dist, &lap.time, sector.start_dist);
                                            let t_end = get_lap_time_at_distance(&lap.dist, &lap.time, sector.end_dist);
                                            let s_time = t_end - t_start;

                                            let is_session_best = s_time > 0.0 && (s_time - best_s_time).abs() < 1e-4;
                                            let is_near_best = s_time > 0.0 && s_time <= best_s_time * 1.015;

                                            let cell_color = if is_session_best {
                                                if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) }
                                            } else if is_near_best {
                                                if is_dark { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(34, 112, 34) }
                                            } else {
                                                if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY }
                                            };

                                            let mut text = egui::RichText::new(format_sector_time(s_time)).color(cell_color);
                                            if is_session_best || is_near_best {
                                                text = text.strong();
                                            }
                                            ui.label(text);
                                        }

                                        let opt_color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                                        ui.label(egui::RichText::new(format_sector_time(best_s_time)).color(opt_color).strong());
                                        ui.end_row();
                                    }

                                    // Totals Row
                                    ui.label(egui::RichText::new("TOTAL").strong().color(ACCENT_COLOR));
                                    for lap in &visible_laps {
                                        let total_time = lap.time.last().copied().unwrap_or(0.0);

                                        let is_total_best = total_time > 0.0 && (total_time - best_total_time).abs() < 1e-4;
                                        let is_total_near_best = total_time > 0.0 && total_time <= best_total_time * 1.015;

                                        let total_color = if is_total_best {
                                            if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) }
                                        } else if is_total_near_best {
                                            if is_dark { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(34, 112, 34) }
                                        } else {
                                            if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }
                                        };

                                        let mut text = egui::RichText::new(format_lap_time(total_time)).color(total_color);
                                        if is_total_best || is_total_near_best {
                                            text = text.strong();
                                        }
                                        ui.label(text);
                                    }

                                    let optimal_total = self.sector_bests.iter().sum::<f64>();
                                    let opt_total_color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                                    ui.label(egui::RichText::new(format_lap_time(optimal_total)).color(opt_total_color).strong());
                                    ui.end_row();
                                });
                        });
                });

                cols[1].vertical(|ui| {
                    ui.heading(egui::RichText::new("Track Layout Map").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        self.draw_interactive_track_map(ui, 450.0);
                    });
                });
            });
        }
    }
}
