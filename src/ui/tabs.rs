use crate::OpenDavApp;
use crate::ActivePage;
use crate::config::worksheet::{WorksheetTab, WorksheetConfig, ACCENT_COLOR, SUB_ACCENT_COLOR};
use crate::signals::processing::{
    get_fastest_lap, format_lap_time, format_sector_time, get_lap_time_at_distance
};

impl OpenDavApp {
    pub fn draw_empty_state_drag_drop(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let ctx = ui.ctx().clone();
        
        let is_hovered = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let stroke_color = if is_hovered { ACCENT_COLOR } else { egui::Color32::from_rgb(50, 50, 50) };
        let bg_color = if is_dark { egui::Color32::from_rgb(20, 20, 20) } else { egui::Color32::from_rgb(240, 240, 240) };
        
        let available_size = ui.available_size();
        
        egui::Frame::none()
            .fill(bg_color)
            .stroke(egui::Stroke::new(3.0, stroke_color))
            .corner_radius(16.0)
            .inner_margin(egui::Margin::same(40))
            .show(ui, |ui| {
                ui.set_min_size(available_size);
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(egui::RichText::new("Drop Telemetry File Here").size(32.0).strong().color(if is_hovered { ACCENT_COLOR } else { SUB_ACCENT_COLOR }));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("Drag and drop an iRacing .ibt file onto this window,").size(18.0).color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new("or click 'Browse Files' below to search manually.").size(18.0).color(egui::Color32::GRAY));
                        
                        ui.add_space(30.0);
                        let browse_btn = egui::Button::new(egui::RichText::new("Browse Files").size(24.0).color(egui::Color32::WHITE))
                            .fill(egui::Color32::from_rgb(140, 82, 255))
                            .corner_radius(12.0)
                            .min_size(egui::vec2(220.0, 50.0));
                        
                        if ui.add(browse_btn).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("iRacing Telemetry", &["ibt"])
                                .set_title("Select Telemetry File")
                                .pick_file() 
                            {
                                self.load_telemetry_file(path.as_path());
                            }
                        }

                        if !self.settings.recent_files.is_empty() {
                            ui.add_space(50.0);
                            ui.label(egui::RichText::new("RECENT FILES").size(16.0).strong().color(egui::Color32::DARK_GRAY));
                            ui.add_space(15.0);
                            
                            let recent_files = self.settings.recent_files.clone();
                            for recent in recent_files {
                                let file_name = std::path::Path::new(&recent).file_name().unwrap_or_default().to_string_lossy();
                                let btn = egui::Button::new(egui::RichText::new(format!("📄 {}", file_name)).size(16.0))
                                    .fill(if is_dark { egui::Color32::from_rgb(30, 30, 30) } else { egui::Color32::from_rgb(220, 220, 220) })
                                    .corner_radius(8.0)
                                    .min_size(egui::vec2(300.0, 45.0));
                                    
                                if ui.add(btn).on_hover_text(&recent).clicked() {
                                    self.load_telemetry_file(std::path::Path::new(&recent));
                                }
                                ui.add_space(8.0);
                            }
                        }
                    });
                });
            });
    }

    pub fn draw_dashboard_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        if !self.session_loaded || self.sessions.is_empty() {
            self.draw_empty_state_drag_drop(ui, is_dark);
            return;
        }

        // SAFE IMMUTABLE CLONING TO RESOLVE RUST BORROW CHECKER CLOSURE LOCKS
        let session_ref = &self.sessions[self.primary_session_idx].session;
        let car = session_ref.car.clone();
        let venue = session_ref.venue.clone();
        let mut air_temp = session_ref.air_temp.clone();
        let mut surface_temp = session_ref.surface_temp.clone();

        if !self.settings.use_metric {
            let convert_temp = |s: &str| -> String {
                if let Some(val_str) = s.strip_suffix(" C") {
                    if let Ok(c) = val_str.parse::<f64>() {
                        let f = c * 9.0 / 5.0 + 32.0;
                        return format!("{:.2} F", f);
                    }
                }
                s.to_string()
            };
            air_temp = convert_temp(&air_temp);
            surface_temp = convert_temp(&surface_temp);
        }
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
                            let is_selected = self.selected_lap == Some((self.primary_session_idx, *lap_num));
                            
                            let mut row_text = format!("Lap {} : {}", lap_num, format_lap_time(*duration));
                            if is_fastest {
                                row_text += " FASTEST";
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
                                self.selected_lap = Some((self.primary_session_idx, *lap_num));
                                if let Some(pos) = self.sessions[self.primary_session_idx].lap_ranges.iter().position(|r| r.0 == *lap_num) {
                                    let (_, start_t, _end_t) = self.sessions[self.primary_session_idx].lap_ranges[pos];
                                    self.cursor_x = Some(start_t);
                                    self.reset_bounds_flag = true;
                                    self.reset_bounds_next_frame = 3;
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
                // Rebuild track sectors and sector bests cache using Signals Layer
                self.active_page = ActivePage::Graphs;
                if self.selected_lap.is_none() {
                    self.selected_lap = Some((self.primary_session_idx, fastest_lap));
                }
            }
        });
    }

    pub fn draw_graphs_page(&mut self, ui: &mut egui::Ui) {
        if !self.session_loaded || self.sessions.is_empty() {
            let is_dark = self.settings.dark_mode;
            self.draw_empty_state_drag_drop(ui, is_dark);
            return;
        }

        // 1. HORIZONTAL MOTEC WORKSHEET TABS AT THE TOP!
        ui.horizontal(|ui| {
            let tab_style = ui.style_mut();
            tab_style.spacing.button_padding = egui::vec2(12.0, 8.0); // Perfect, professional tab sizing

            ui.selectable_value(&mut self.active_worksheet, WorksheetTab::Basic, "1. Basic (Inputs)");
            ui.selectable_value(&mut self.active_worksheet, WorksheetTab::BasicVehicle, "2. Basic Vehicle");
            ui.selectable_value(&mut self.active_worksheet, WorksheetTab::DynamicRake, "3. Dynamic Rake");
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
            WorksheetTab::BasicVehicle => Some(WorksheetConfig::basic_vehicle()),
            WorksheetTab::DynamicRake => Some(WorksheetConfig::rake()),
            _ => None,
        };

        // MANUAL RESIZER (Bypasses TopBottomPanel bugs with egui::Plot)
        let mut track_map_height = ui.ctx().data_mut(|d| d.get_temp::<f32>(egui::Id::new("tm_h")).unwrap_or(300.0));
        let max_h = ui.available_height() - 150.0;
        track_map_height = track_map_height.clamp(150.0, max_h.max(150.0));

        let graphs_height = if self.show_graphs_track_map {
            (ui.available_height() - track_map_height - 8.0).max(100.0)
        } else {
            ui.available_height()
        };

        ui.allocate_ui(egui::vec2(ui.available_width(), graphs_height), |ui| {
            if let Some(cfg) = &config {
                let canvas_id = match self.active_worksheet {
                    WorksheetTab::Basic => "basic_worksheet_canvas",
                    WorksheetTab::BasicVehicle => "basic_vehicle_worksheet_canvas",
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

        if self.show_graphs_track_map {
            ui.add_space(2.0);
            let resizer = ui.allocate_response(egui::vec2(ui.available_width(), 4.0), egui::Sense::drag());
            
            // Draw a subtle line for the splitter
            let rect = resizer.rect;
            let visual_color = if resizer.hovered() || resizer.dragged() {
                egui::Color32::from_rgb(100, 150, 255)
            } else {
                egui::Color32::from_rgb(80, 80, 80)
            };
            ui.painter().rect_filled(rect, 0.0, visual_color);

            if resizer.dragged() {
                track_map_height -= resizer.drag_delta().y;
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tm_h"), track_map_height));
            }
            if resizer.hovered() || resizer.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            ui.add_space(2.0);
            
            ui.allocate_ui(egui::vec2(ui.available_width(), track_map_height), |ui| {
                self.draw_interactive_track_map(ui, track_map_height);
            });
        }
    }

    pub fn draw_reports_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        if !self.session_loaded || self.sessions.is_empty() {
            self.draw_empty_state_drag_drop(ui, is_dark);
            return;
        }

        ui.horizontal(|ui| {
            let tab_style = ui.style_mut();
            tab_style.spacing.button_padding = egui::vec2(12.0, 8.0);

            ui.selectable_value(&mut self.active_reports_tab, crate::ReportsTab::SectorAnalysis, "1. Sector Analysis");
            ui.selectable_value(&mut self.active_reports_tab, crate::ReportsTab::TimingGraphs, "2. Timing Graphs");
            

        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        match self.active_reports_tab {
            crate::ReportsTab::SectorAnalysis => {

        let has_data = {
            let loaded = &self.sessions[self.primary_session_idx];
            !loaded.lap_data_cache.is_empty() && !loaded.sectors.is_empty()
        };

        if !has_data {
            ui.label("No sector or lap data available for report.");
        } else {
            ui.vertical(|ui| {
                let loaded = &self.sessions[self.primary_session_idx];
                let mut visible_laps: Vec<&crate::signals::processing::LapData> = loaded.lap_data_cache.iter()
                    .filter(|lap| lap.lap_num > 3)
                    .collect();
                if visible_laps.is_empty() {
                    visible_laps = loaded.lap_data_cache.iter().collect();
                }

                let best_total_time = visible_laps.iter()
                    .map(|lap| lap.time.last().copied().unwrap_or(0.0))
                    .filter(|&t| t > 0.0)
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0);

                egui::ScrollArea::both()
                    .id_salt("sector_report_scroll")
                    .max_height(300.0)
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
                                    for (s_idx, sector) in loaded.sectors.iter().enumerate() {
                                        ui.label(egui::RichText::new(&sector.name).strong());

                                        let best_s_time = loaded.sector_bests.get(s_idx).copied().unwrap_or(0.0);

                                        for lap in &visible_laps {
                                            let t_start = get_lap_time_at_distance(&lap.dist, &lap.time, sector.start_dist);
                                            let t_end = get_lap_time_at_distance(&lap.dist, &lap.time, sector.end_dist);
                                            let s_time = t_end - t_start;

                                            let is_session_best = s_time > 0.0 && (s_time - best_s_time).abs() < 1e-4;
                                            let is_near_best = s_time > 0.0 && s_time <= best_s_time * 1.015;

                                            let (bg_color, text_color) = if is_session_best {
                                                (egui::Color32::from_rgb(128, 0, 128), egui::Color32::WHITE) // Purple background
                                            } else if is_near_best {
                                                (egui::Color32::from_rgb(34, 139, 34), egui::Color32::WHITE) // Green background
                                            } else {
                                                (egui::Color32::BLACK, egui::Color32::WHITE) // Normal black background
                                            };

                                            egui::Frame::none()
                                                .fill(bg_color)
                                                .corner_radius(4.0)
                                                .inner_margin(egui::vec2(6.0, 4.0))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(format_sector_time(s_time)).color(text_color).strong());
                                                });
                                        }

                                        let opt_bg = egui::Color32::from_rgb(128, 0, 128);
                                        egui::Frame::none()
                                            .fill(opt_bg)
                                            .corner_radius(4.0)
                                            .inner_margin(egui::vec2(6.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(format_sector_time(best_s_time)).color(egui::Color32::WHITE).strong());
                                            });
                                        ui.end_row();
                                    }

                                    // Totals Row
                                    ui.label(egui::RichText::new("TOTAL").strong().color(ACCENT_COLOR));
                                    for lap in &visible_laps {
                                        let total_time = lap.time.last().copied().unwrap_or(0.0);

                                        let is_total_best = total_time > 0.0 && (total_time - best_total_time).abs() < 1e-4;
                                        let is_total_near_best = total_time > 0.0 && total_time <= best_total_time * 1.015;

                                        let (bg_color, text_color) = if is_total_best {
                                            (egui::Color32::from_rgb(128, 0, 128), egui::Color32::WHITE)
                                        } else if is_total_near_best {
                                            (egui::Color32::from_rgb(34, 139, 34), egui::Color32::WHITE)
                                        } else {
                                            (egui::Color32::BLACK, egui::Color32::WHITE)
                                        };

                                        egui::Frame::none()
                                            .fill(bg_color)
                                            .corner_radius(4.0)
                                            .inner_margin(egui::vec2(6.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(format_lap_time(total_time)).color(text_color).strong());
                                            });
                                    }

                                    let optimal_total = loaded.sector_bests.iter().sum::<f64>();
                                    let opt_bg = egui::Color32::from_rgb(128, 0, 128);
                                    egui::Frame::none()
                                        .fill(opt_bg)
                                        .corner_radius(4.0)
                                        .inner_margin(egui::vec2(6.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(format_lap_time(optimal_total)).color(egui::Color32::WHITE).strong());
                                        });
                                    ui.end_row();
                            });
                    });
                
                ui.add_space(20.0);

                let venue_name = self.sessions[self.primary_session_idx].session.venue.to_uppercase();
                ui.heading(egui::RichText::new(venue_name).strong().color(crate::config::worksheet::ACCENT_COLOR));
                ui.add_space(8.0);
                ui.group(|ui| {
                    let map_height = ui.available_height().max(300.0);
                    self.draw_interactive_track_map(ui, map_height);
                });
            });
        }
    }
        crate::ReportsTab::TimingGraphs => {
            self.draw_timing_graphs_page(ui, is_dark);
        }
        }
    }
}

