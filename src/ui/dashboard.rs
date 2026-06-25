use rfd::FileDialog;
use crate::OpenDavApp;
use crate::ActivePage;
use crate::config::worksheet::{WorksheetTab, ACCENT_COLOR, SUB_ACCENT_COLOR, DARK_BG_COLOR, LIGHT_BG_COLOR};
use crate::signals::processing::{get_fastest_lap, format_lap_time, trigger_track_map_download};

impl OpenDavApp {
    pub fn draw_splash_screen(&mut self, ctx: &egui::Context, progress: f32) {
        // Render splash screen with a sleek obsidian backdrop
        let panel_frame = egui::Frame::central_panel(&ctx.style())
            .fill(DARK_BG_COLOR)
            .inner_margin(egui::Margin::same(0));
        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            let size = ui.available_size();
            
            // Center the logo and loading bar vertically and horizontally
            let logo_width = 550.0;
            let logo_height = logo_width * (1440.0 / 2560.0); // 2560x1440 ratio
            
            let group_height = logo_height + 40.0 + 4.0;
            let start_y = (size.y - group_height) / 2.0;
            
            let logo_rect = egui::Rect::from_min_size(
                egui::pos2((size.x - logo_width) / 2.0, start_y),
                egui::vec2(logo_width, logo_height)
            );
            
            let logo_bytes = include_bytes!("../../assets/transparent_full_opendav_logo.png");
            
            ui.put(
                logo_rect,
                egui::Image::from_bytes("bytes://transparent_full_opendav_logo.png", logo_bytes.to_vec())
            );
            
            // Draw the loading progress bar underneath the logo
            let bar_width = 300.0;
            let bar_height = 3.0; // Thin and elegant
            let bar_rect = egui::Rect::from_center_size(
                egui::pos2(size.x / 2.0, logo_rect.max.y + 40.0),
                egui::vec2(bar_width, bar_height)
            );
            
            let progress_bg = egui::Color32::from_rgb(25, 25, 25);
            ui.painter().rect_filled(bar_rect, 1.5, progress_bg);

            let active_width = bar_width * progress;
            let mut active_rect = bar_rect;
            active_rect.max.x = active_rect.min.x + active_width;

            ui.painter().rect_filled(active_rect, 1.5, ACCENT_COLOR);
        });
    }

    pub fn draw_sidebar(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .default_width(260.0) 
            .show(ctx, |ui| {
                ui.add_space(15.0);
                
                match self.active_page {
                    ActivePage::OpenDav | ActivePage::Reports => {
                        // 1. CUSTOM CORNER LOGO HEADER
                        let corner_bytes = include_bytes!("../../assets/corner_logo.png");
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Image::from_bytes("bytes://corner_logo.png", corner_bytes.to_vec())
                                    .max_width(240.0) // Fills the sidebar width beautifully!
                                    .maintain_aspect_ratio(true)
                            );
                        });

                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(15.0);

                        let sidebar_style = ui.style_mut();
                        sidebar_style.spacing.button_padding = egui::vec2(16.0, 12.0); // 35% larger padding

                        ui.vertical(|ui| {
                            // 1. Dashboard Image Button (Full width, padded with selection glow border)
                            let db_bytes = include_bytes!("../../assets/button_dashboard.png");
                            let is_db_selected = self.active_page == ActivePage::OpenDav;
                            let border_color_db = if is_db_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };
                            
                            ui.add_space(5.0);
                            egui::Frame::none()
                                .stroke(egui::Stroke::new(2.0, border_color_db))
                                .rounding(8.0)
                                .inner_margin(1.0)
                                .show(ui, |ui| {
                                    let img_db = egui::Image::from_bytes("bytes://button_dashboard.png", db_bytes.to_vec())
                                        .max_width(240.0)
                                        .rounding(8.0)
                                        .sense(egui::Sense::click());
                                    let resp = ui.add(img_db);
                                    if resp.clicked() {
                                        self.active_page = ActivePage::OpenDav;
                                    }
                                });

                            ui.add_space(15.0);

                            // 2. Graphs Workspace Image Button
                            let gr_bytes = include_bytes!("../../assets/button_graphs.png");
                            let is_gr_selected = self.active_page == ActivePage::Graphs;
                            let border_color_gr = if is_gr_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };

                            egui::Frame::none()
                                .stroke(egui::Stroke::new(2.0, border_color_gr))
                                .rounding(8.0)
                                .inner_margin(1.0)
                                .show(ui, |ui| {
                                    let img_gr = egui::Image::from_bytes("bytes://button_graphs.png", gr_bytes.to_vec())
                                        .max_width(240.0)
                                        .rounding(8.0)
                                        .sense(egui::Sense::click());
                                    let resp = ui.add(img_gr);
                                    if resp.clicked() {
                                        self.active_page = ActivePage::Graphs;
                                        // Default to fastest lap on first entering graphs page
                                        if !self.sessions.is_empty() && self.selected_lap.is_none() {
                                            let p_idx = self.primary_session_idx;
                                            let session = &self.sessions[p_idx].session;
                                            let fastest_lap = get_fastest_lap(&session.lap_times);
                                            self.selected_lap = if fastest_lap > 0 { Some((p_idx, fastest_lap)) } else { None };
                                        }
                                    }
                                });

                            ui.add_space(15.0);

                            // 3. Reports Image Button
                            let rep_bytes = include_bytes!("../../assets/button_reports.png");
                            let is_rep_selected = self.active_page == ActivePage::Reports;
                            let border_color_rep = if is_rep_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };

                            egui::Frame::none()
                                .stroke(egui::Stroke::new(2.0, border_color_rep))
                                .rounding(8.0)
                                .inner_margin(1.0)
                                .show(ui, |ui| {
                                    let img_rep = egui::Image::from_bytes("bytes://button_reports.png", rep_bytes.to_vec())
                                        .max_width(240.0)
                                        .rounding(8.0)
                                        .sense(egui::Sense::click());
                                    let resp = ui.add(img_rep);
                                    if resp.clicked() {
                                        self.active_page = ActivePage::Reports;
                                    }
                                });
                        });
                    }
                    ActivePage::Graphs => {
                        // 2. COMPACT MOTEC SIDEBAR CUT-OFF (LAP SELECTION EXCLUSIVE)
                        ui.vertical(|ui| {
                            ui.add_space(5.0);
                            if ui.button(egui::RichText::new("⬅  Back to OpenDAV").strong().color(ACCENT_COLOR)).clicked() {
                                self.active_page = ActivePage::OpenDav;
                            }
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            // --- PLAYBACK CONTROLS ---
                            let select_hdr_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
                            ui.label(egui::RichText::new("TELEMETRY PLAYBACK").color(select_hdr_color).size(10.0).strong());
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                let play_icon = if self.is_playing { "⏸ Pause " } else { "▶ Play  " };
                                let play_color = if self.is_playing { egui::Color32::from_rgb(200, 50, 50) } else { ACCENT_COLOR };
                                
                                let play_btn = ui.add_sized([100.0, 32.0], egui::Button::new(egui::RichText::new(play_icon).strong().color(play_color).size(16.0)));
                                if play_btn.clicked() {
                                    if !self.is_playing && !self.sessions.is_empty() && self.selected_lap.is_some() {
                                        self.is_playing = true;
                                    } else {
                                        self.is_playing = false;
                                    }
                                }

                                ui.add_space(5.0);
                                
                                let mut speed_text = "1.0x";
                                if self.playback_speed == 0.5 { speed_text = "0.5x"; }
                                else if self.playback_speed == 2.0 { speed_text = "2.0x"; }
                                
                                egui::ComboBox::from_id_source("playback_speed")
                                    .selected_text(egui::RichText::new(speed_text).size(14.0))
                                    .width(60.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.playback_speed, 0.5, "0.5x");
                                        ui.selectable_value(&mut self.playback_speed, 1.0, "1.0x");
                                        ui.selectable_value(&mut self.playback_speed, 2.0, "2.0x");
                                    });
                            });

                            ui.add_space(15.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.label(egui::RichText::new("LAP TIMELINE SELECT").color(select_hdr_color).size(10.0).strong());
                            ui.add_space(8.0);

                            if self.sessions.is_empty() {
                                ui.label(egui::RichText::new("No Session Active").color(egui::Color32::GRAY).small());
                            } else {
                                let sidebar_style = ui.style_mut();
                                sidebar_style.spacing.button_padding = egui::vec2(12.0, 8.0);

                                let mut new_primary_idx = None;
                                let mut new_ref_cyan = None;
                                let mut toggle_cyan_off = false;
                                let mut new_ref_white = None;
                                let mut toggle_white_off = false;
                                let mut new_selected_lap = None;

                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        for (s_idx, loaded_session) in self.sessions.iter().enumerate() {
                                            let is_primary = self.primary_session_idx == s_idx;
                                            let header_color = if is_primary { ACCENT_COLOR } else { if is_dark { egui::Color32::from_rgb(40,40,40) } else { egui::Color32::from_rgb(210,210,210) } };
                                            let text_color = if is_primary { egui::Color32::BLACK } else { if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK } };

                                            let header_btn = egui::Frame::none()
                                                .fill(header_color)
                                                .rounding(4.0)
                                                .inner_margin(egui::Margin::symmetric(6, 4))
                                                .show(ui, |ui| {
                                                    ui.selectable_label(is_primary, egui::RichText::new(&loaded_session.file_name).color(text_color).strong())
                                                }).inner;

                                            if header_btn.clicked() {
                                                new_primary_idx = Some(s_idx);
                                            }
                                            
                                            ui.add_space(4.0);

                                            let lap_times = &loaded_session.session.lap_times;
                                            let fastest_lap = get_fastest_lap(lap_times);

                                            egui::Frame::none()
                                                .stroke(egui::Stroke::new(1.0, if is_dark { egui::Color32::from_rgb(50,50,50) } else { egui::Color32::from_rgb(200,200,200) }))
                                                .rounding(4.0)
                                                .inner_margin(egui::Margin::symmetric(6, 4))
                                                .show(ui, |ui| {
                                                for (lap_num, duration) in lap_times {
                                                    let is_selected = self.selected_lap == Some((s_idx, *lap_num));
                                                    let is_fastest = *lap_num == fastest_lap && *lap_num > 0;

                                                    let is_cyan = self.ref_lap_cyan == Some((s_idx, *lap_num));
                                                    let is_white = self.ref_lap_white == Some((s_idx, *lap_num));

                                                    let label_color = if is_selected {
                                                        ACCENT_COLOR
                                                    } else if is_fastest {
                                                        SUB_ACCENT_COLOR
                                                    } else {
                                                        if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }
                                                    };

                                                    ui.horizontal(|ui| {
                                                        // 1. Cyan Reference Toggle Box (Left)
                                                        let active_cyan = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 136, 170) };
                                                        let border_color_c = if is_cyan { active_cyan } else { egui::Color32::TRANSPARENT };
                                                        
                                                        let btn_c = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_c))
                                                            .rounding(4.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                                            .show(ui, |ui| {
                                                                ui.selectable_label(false, egui::RichText::new("C").color(if is_cyan { active_cyan } else { egui::Color32::DARK_GRAY }).strong())
                                                            }).inner;
                                                        
                                                        if btn_c.clicked() {
                                                            if is_cyan {
                                                                toggle_cyan_off = true;
                                                            } else {
                                                                new_ref_cyan = Some((s_idx, *lap_num));
                                                            }
                                                        }

                                                        // 2. White Reference Toggle Box (Right)
                                                        let active_white = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(40, 40, 40) };
                                                        let border_color_w = if is_white { active_white } else { egui::Color32::TRANSPARENT };
                                                        
                                                        let btn_w = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_w))
                                                            .rounding(4.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                                            .show(ui, |ui| {
                                                                ui.selectable_label(false, egui::RichText::new("W").color(if is_white { active_white } else { egui::Color32::DARK_GRAY }).strong())
                                                            }).inner;
                                                        
                                                        if btn_w.clicked() {
                                                            if is_white {
                                                                toggle_white_off = true;
                                                            } else {
                                                                new_ref_white = Some((s_idx, *lap_num));
                                                            }
                                                        }

                                                        // 3. Main Lap Timeline Selection Selector
                                                        let mut text = format!("Lap {} : {}", lap_num, format_lap_time(*duration));
                                                        if is_fastest {
                                                            text += " ★";
                                                        }

                                                        let border_color_l = if is_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };
                                                        
                                                        let btn_l = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_l))
                                                            .rounding(4.0)
                                                            .inner_margin(egui::Margin::symmetric(6, 3))
                                                            .show(ui, |ui| {
                                                                ui.selectable_label(false, egui::RichText::new(text).color(label_color).strong())
                                                            }).inner;

                                                        if btn_l.clicked() {
                                                            new_selected_lap = Some((s_idx, *lap_num));
                                                        }
                                                    });
                                                }
                                            });
                                            ui.add_space(8.0);
                                        }
                                    });
                                });

                                let mut state_changed = false;
                                if let Some(idx) = new_primary_idx {
                                    self.primary_session_idx = idx;
                                    state_changed = true;
                                }
                                if toggle_cyan_off {
                                    self.ref_lap_cyan = None;
                                    state_changed = true;
                                } else if let Some(c) = new_ref_cyan {
                                    self.ref_lap_cyan = Some(c);
                                    state_changed = true;
                                }
                                if toggle_white_off {
                                    self.ref_lap_white = None;
                                    state_changed = true;
                                } else if let Some(w) = new_ref_white {
                                    self.ref_lap_white = Some(w);
                                    state_changed = true;
                                }
                                if let Some(sl) = new_selected_lap {
                                    self.selected_lap = Some(sl);
                                    if let Some(pos) = self.sessions[sl.0].lap_ranges.iter().position(|r| r.0 == sl.1) {
                                        let (_, start_t, _end_t) = self.sessions[sl.0].lap_ranges[pos];
                                        self.cursor_x = Some(start_t);
                                        self.reset_bounds_flag = true;
                                    }
                                    state_changed = true;
                                }
                                if state_changed {
                                    self.update_sector_deltas();
                                }
                            }
                        });
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("v0.4.0-rs").color(egui::Color32::DARK_GRAY).small());
                });
            });
    }

    pub fn draw_top_panel(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            egui::menu::bar(ui, |ui| {
                if ui.button("📂 Load IBT Telemetry").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("iRacing Telemetry", &["ibt"])
                        .set_title("Select Telemetry File")
                        .pick_file() 
                    {
                        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                        self.active_file = Some(file_name.clone());
                        match crate::data::ibt_parser::parse_ibt_file(&path) {
                            Ok(parsed_session) => {
                                self.session_loaded = true;
                                
                                // Trigger the asynchronous background SVG track map downloader
                                trigger_track_map_download(parsed_session.track_id);
                                
                                let new_session = crate::LoadedSession::new(file_name, parsed_session);
                                self.sessions.push(new_session);
                                
                                let new_idx = self.sessions.len() - 1;
                                self.primary_session_idx = new_idx;
                                
                                // Auto-load fastest lap in caching layer on file load
                                let fastest = get_fastest_lap(&self.sessions[new_idx].session.lap_times);
                                self.selected_lap = if fastest > 0 { Some((new_idx, fastest)) } else { None };
                                
                                self.cursor_x = None;
                                self.update_sector_deltas();
                            }
                            Err(e) => {
                                eprintln!("Error parsing .ibt file: {}", e);
                            }
                        }
                    }
                }

                if let Some(file) = &self.active_file {
                    ui.separator();
                    ui.label(egui::RichText::new(format!("File: {}", file)).color(if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY }).small());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.session_loaded {
                        if ui.button("🔄 Reset Zoom").clicked() {
                            self.reset_bounds_flag = true;
                        }
                        ui.separator();
                    }
                    egui::widgets::global_theme_preference_switch(ui);
                    
                    // Tiny little uppercase letter T button right next to the theme switcher, only visible in Graphs page
                    if self.active_page == ActivePage::Graphs {
                        let t_text = if self.show_graphs_track_map {
                            egui::RichText::new("T").strong().color(ACCENT_COLOR)
                        } else {
                            egui::RichText::new("T").strong()
                        };
                        if ui.add(egui::Button::new(t_text).frame(true)).on_hover_text("Toggle Track Map View").clicked() {
                            self.show_graphs_track_map = !self.show_graphs_track_map;
                        }
                    }
                });
            });
            ui.add_space(6.0);
        });
    }
}
