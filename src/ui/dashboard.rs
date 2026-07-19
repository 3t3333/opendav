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
            
            let bg_bytes = include_bytes!("../../assets/splash_bg.jpg");
            let bg_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size);
            
            let bg_img = egui::Image::from_bytes("bytes://splash_bg.jpg", bg_bytes.to_vec())
                .fit_to_exact_size(size);
            ui.put(bg_rect, bg_img);
            
            // Dark overlay
            ui.painter().rect_filled(bg_rect, 0.0, egui::Color32::from_black_alpha(220));
            
            // Center the logo and loading bar vertically and horizontally
            let logo2_width = 550.0;
            let logo2_height = logo2_width * (1440.0 / 2560.0); // 2560x1440 ratio
            let group_height = logo2_height + 40.0 + 4.0;
            let start_y = (size.y - group_height) / 2.0;
            
            let logo2_rect = egui::Rect::from_min_size(
                egui::pos2((size.x - logo2_width) / 2.0, start_y),
                egui::vec2(logo2_width, logo2_height)
            );

            let logo1_width = 300.0;
            let logo1_rect = egui::Rect::from_center_size(
                logo2_rect.center(),
                egui::vec2(logo1_width, logo1_width)
            );
            
            let logo1_bytes = include_bytes!("../../assets/logo_transparent_lighttext.png");
            let logo2_bytes = include_bytes!("../../assets/opendav_transparent_lighttext.png");
            
            // Fading logic based on progress (0.0 to 1.0)
            let alpha1 = if progress < 0.45 {
                1.0 - (progress / 0.45)
            } else {
                0.0
            };
            
            let alpha2 = if progress > 0.55 {
                (progress - 0.55) / 0.45
            } else {
                0.0
            };
            
            if alpha1 > 0.0 {
                let img = egui::Image::from_bytes("bytes://logo_transparent_lighttext_splash.png", logo1_bytes.to_vec())
                    .show_loading_spinner(false)
                    .tint(egui::Color32::from_white_alpha((alpha1 * 255.0) as u8));
                ui.put(logo1_rect, img);
            }
            
            if alpha2 > 0.0 {
                let img = egui::Image::from_bytes("bytes://opendav_transparent_lighttext_splash.png", logo2_bytes.to_vec())
                    .show_loading_spinner(false)
                    .tint(egui::Color32::from_white_alpha((alpha2 * 255.0) as u8));
                ui.put(logo2_rect, img);
            }
            
            // Draw the loading progress bar underneath the logo
            let bar_width = 300.0;
            let bar_height = 3.0; // Thin and elegant
            let bar_rect = egui::Rect::from_center_size(
                egui::pos2(size.x / 2.0, logo2_rect.max.y + 40.0),
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
                    ActivePage::OpenDav | ActivePage::Reports | ActivePage::SimGit | ActivePage::Settings => {
                        // 1. CUSTOM CORNER LOGO HEADER
                        let corner_bytes = include_bytes!("../../assets/logo_transparent_lighttext.png");
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Image::from_bytes("bytes://logo_transparent_lighttext.png", corner_bytes.to_vec())
                                    .show_loading_spinner(false)
                                    .max_width(150.0) // Scaled down to fit better as a corner logo
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
                            
                            ui.add_space(5.0);
                            let img_db = egui::Image::from_bytes("bytes://button_dashboard.png", db_bytes.to_vec())
                                .max_width(240.0)
                                .rounding(8.0)
                                .sense(egui::Sense::click());
                            let resp = ui.add(img_db);
                            
                            let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                            let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_db_selected);
                            let color = egui::Rgba::from_rgba_premultiplied(
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).r() * sel_f,
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).g() * sel_f,
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).b() * sel_f,
                                hover_f * (1.0 - sel_f) + sel_f
                            );
                            if color.a() > 0.01 {
                                ui.painter().rect_stroke(resp.rect.expand(1.0), 8.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Inside);
                            }
                            if resp.clicked() {
                                self.active_page = ActivePage::OpenDav;
                            }

                            ui.add_space(15.0);

                            // 2. Graphs Workspace Image Button
                            let gr_bytes = include_bytes!("../../assets/button_graphs.png");
                            let is_gr_selected = self.active_page == ActivePage::Graphs;
                            
                            let img_gr = egui::Image::from_bytes("bytes://button_graphs.png", gr_bytes.to_vec())
                                .max_width(240.0)
                                .rounding(8.0)
                                .sense(egui::Sense::click());
                            let resp = ui.add(img_gr);
                            
                            let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                            let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_gr_selected);
                            let color = egui::Rgba::from_rgba_premultiplied(
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).r() * sel_f,
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).g() * sel_f,
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).b() * sel_f,
                                hover_f * (1.0 - sel_f) + sel_f
                            );
                            if color.a() > 0.01 {
                                ui.painter().rect_stroke(resp.rect.expand(1.0), 8.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Inside);
                            }
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

                            ui.add_space(15.0);

                            // 3. Reports Image Button
                            let rep_bytes = include_bytes!("../../assets/button_reports.png");
                            let is_rep_selected = self.active_page == ActivePage::Reports;

                            let img_rep = egui::Image::from_bytes("bytes://button_reports.png", rep_bytes.to_vec())
                                .max_width(240.0)
                                .rounding(8.0)
                                .sense(egui::Sense::click());
                            let resp = ui.add(img_rep);

                            let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                            let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_rep_selected);
                            let color = egui::Rgba::from_rgba_premultiplied(
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).r() * sel_f,
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).g() * sel_f,
                                1.0 * hover_f * (1.0 - sel_f) + egui::Rgba::from(ACCENT_COLOR).b() * sel_f,
                                hover_f * (1.0 - sel_f) + sel_f
                            );
                            if color.a() > 0.01 {
                                ui.painter().rect_stroke(resp.rect.expand(1.0), 8.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Inside);
                            }
                            if resp.clicked() {
                                self.active_page = ActivePage::Reports;
                            }

                            ui.add_space(15.0);

                            // 4. SimGit Image Button (HIDDEN FOR PRE-RELEASE)
                            // let simgit_bytes = include_bytes!("../../assets/button_simgit.png");
                            // let is_simgit_selected = self.active_page == ActivePage::SimGit;
                            // let border_color_simgit = if is_simgit_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };
                            // 
                            // egui::Frame::none()
                            //     .stroke(egui::Stroke::new(2.0, border_color_simgit))
                            //     .rounding(8.0)
                            //     .inner_margin(1.0)
                            //     .show(ui, |ui| {
                            //         let img_simgit = egui::Image::from_bytes("bytes://button_simgit.png", simgit_bytes.to_vec())
                            //             .max_width(240.0)
                            //             .rounding(8.0)
                            //             .sense(egui::Sense::click());
                            //         let resp = ui.add(img_simgit);
                            //         if resp.clicked() {
                            //             self.active_page = ActivePage::SimGit;
                            //         }
                            //     });

                            ui.add_space(15.0);
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
                                let mut session_to_remove = None;

                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        for (s_idx, loaded_session) in self.sessions.iter().enumerate() {
                                            let is_primary = self.primary_session_idx == s_idx;
                                            let header_color = if is_primary { ACCENT_COLOR } else { if is_dark { egui::Color32::from_rgb(40,40,40) } else { egui::Color32::from_rgb(210,210,210) } };
                                            let text_color = if is_primary { egui::Color32::BLACK } else { if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK } };

                                            let mut local_remove = false;
                                            let header_btn = egui::Frame::none()
                                                .fill(header_color)
                                                .corner_radius(4.0)
                                                .inner_margin(egui::Margin::symmetric(6, 4))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        let btn = ui.selectable_label(is_primary, egui::RichText::new(&loaded_session.file_name).color(text_color).strong());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.button(egui::RichText::new("🗑").color(text_color)).clicked() {
                                                                local_remove = true;
                                                            }
                                                        });
                                                        btn
                                                    }).inner
                                                }).inner;

                                            if local_remove {
                                                session_to_remove = Some(s_idx);
                                            } else if header_btn.clicked() {
                                                new_primary_idx = Some(s_idx);
                                            }
                                            
                                            ui.add_space(4.0);

                                            let lap_times = &loaded_session.session.lap_times;
                                            let fastest_lap = get_fastest_lap(lap_times);

                                            egui::Frame::none()
                                                .stroke(egui::Stroke::new(1.0, if is_dark { egui::Color32::from_rgb(50,50,50) } else { egui::Color32::from_rgb(200,200,200) }))
                                                .corner_radius(4.0)
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
                                                            .corner_radius(4.0)
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
                                                            .corner_radius(4.0)
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
                                                            .corner_radius(4.0)
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
                                
                                if let Some(idx) = session_to_remove {
                                    self.sessions.remove(idx);
                                    if self.sessions.is_empty() {
                                        self.session_loaded = false;
                                        self.primary_session_idx = 0;
                                        self.selected_lap = None;
                                        self.ref_lap_cyan = None;
                                        self.ref_lap_white = None;
                                    } else {
                                        if self.primary_session_idx == idx {
                                            self.primary_session_idx = 0;
                                        } else if self.primary_session_idx > idx {
                                            self.primary_session_idx -= 1;
                                        }
                                        
                                        let mut handle_ref_lap = |r: &mut Option<(usize, i32)>| {
                                            if let Some((s_idx, lap)) = *r {
                                                if s_idx == idx {
                                                    *r = None;
                                                } else if s_idx > idx {
                                                    *r = Some((s_idx - 1, lap));
                                                }
                                            }
                                        };
                                        handle_ref_lap(&mut self.ref_lap_cyan);
                                        handle_ref_lap(&mut self.ref_lap_white);
                                        
                                        if let Some((s_idx, lap)) = self.selected_lap {
                                            if s_idx == idx {
                                                self.selected_lap = None;
                                            } else if s_idx > idx {
                                                self.selected_lap = Some((s_idx - 1, lap));
                                            }
                                        }
                                        state_changed = true;
                                    }
                                }
                                
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
                                if state_changed && !self.sessions.is_empty() {
                                    self.update_sector_deltas();
                                }
                            }
                        });
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let is_settings = self.active_page == ActivePage::Settings;
                        
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                        let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                        let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_settings);
                        
                        let base = egui::Rgba::from(egui::Color32::GRAY);
                        let acc = egui::Rgba::from(ACCENT_COLOR);
                        let gear_color = egui::Rgba::from_rgba_premultiplied(
                            base.r() * (1.0 - hover_f - sel_f).max(0.0) + 1.0 * hover_f * (1.0 - sel_f) + acc.r() * sel_f,
                            base.g() * (1.0 - hover_f - sel_f).max(0.0) + 1.0 * hover_f * (1.0 - sel_f) + acc.g() * sel_f,
                            base.b() * (1.0 - hover_f - sel_f).max(0.0) + 1.0 * hover_f * (1.0 - sel_f) + acc.b() * sel_f,
                            base.a() * (1.0 - hover_f - sel_f).max(0.0) + hover_f * (1.0 - sel_f) + sel_f
                        ).into();
                        
                        ui.painter().text(
                            rect.center(), 
                            egui::Align2::CENTER_CENTER, 
                            "⚙", 
                            egui::FontId::proportional(22.0), 
                            gear_color
                        );
                        
                        if resp.on_hover_text("Settings").clicked() {
                            self.active_page = ActivePage::Settings;
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("v0.9.0-rs").color(egui::Color32::DARK_GRAY).small());
                        });
                    });
                });
            });
    }

    pub fn draw_top_panel(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            egui::menu::bar(ui, |ui| {
                if ui.button("Load IBT Telemetry").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("iRacing Telemetry", &["ibt"])
                        .set_title("Select Telemetry File")
                        .pick_file() 
                    {
                        self.load_telemetry_file(path.as_path());
                    }
                }

                if self.session_loaded && !self.sessions.is_empty() {
                    ui.add_space(8.0);
                    if ui.button("Export CSV").clicked() {
                        let primary = &mut self.sessions[self.primary_session_idx];
                        let default_name = format!("{}.csv", primary.file_name.replace(".ibt", ""));
                        if let Some(path) = FileDialog::new()
                            .add_filter("CSV File", &["csv"])
                            .set_file_name(&default_name)
                            .set_title("Export Telemetry to CSV")
                            .save_file()
                        {
                            let session = &mut primary.session;
                            if let Ok(mut file) = std::fs::File::create(&path) {
                                use std::io::Write;
                                let _ = writeln!(file, "# Source: {}", session.source_file);
                                let _ = writeln!(file, "# Car: {}", session.car);
                                let _ = writeln!(file, "# Venue: {}", session.venue);
                                let _ = writeln!(file, "# Air Temp: {}", session.air_temp);
                                let _ = writeln!(file, "# Surface Temp: {}", session.surface_temp);
                                let _ = writeln!(file, "# Timestamp: {}", session.timestamp);
                                let _ = writeln!(file, "# Total Time: {:.3}s", session.total_session_time);
                                let _ = writeln!(file, "# Laps: {}", session.lap_times.len());
                                
                                use polars::prelude::SerWriter;
                                let _ = polars::prelude::CsvWriter::new(&mut file)
                                    .include_header(true)
                                    .finish(&mut session.dataframe);
                            }
                        }
                    }
                }

                if !self.sessions.is_empty() {
                    ui.separator();
                    let primary_file_name = &self.sessions[self.primary_session_idx].file_name;
                    
                    if self.sessions.len() == 1 {
                        ui.label(egui::RichText::new(format!("File: {}", primary_file_name)).color(if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY }).small());
                    } else {
                        let mut new_primary = None;
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Primary File:").color(if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY }).small());
                            egui::ComboBox::from_id_source("top_primary_session_dropdown")
                                .selected_text(egui::RichText::new(primary_file_name).small())
                                .show_ui(ui, |ui| {
                                    for (idx, session) in self.sessions.iter().enumerate() {
                                        if ui.selectable_label(self.primary_session_idx == idx, &session.file_name).clicked() {
                                            new_primary = Some(idx);
                                        }
                                    }
                                });
                        });
                        
                        if let Some(idx) = new_primary {
                            self.primary_session_idx = idx;
                            self.update_sector_deltas();
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.session_loaded {
                        if ui.button("🔄 Reset Zoom").clicked() {
                            self.reset_bounds_flag = true;
                        }
                        ui.separator();
                    }
                    let theme_icon = if self.settings.dark_mode { "🌙" } else { "☀️" };
                    if ui.button(theme_icon).on_hover_text("Toggle Theme").clicked() {
                        self.settings.dark_mode = !self.settings.dark_mode;
                        self.settings.save();
                    }
                    
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
