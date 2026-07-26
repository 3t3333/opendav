use crate::config::theme::AppTheme;
use crate::config::worksheet::{ACCENT_COLOR, DARK_BG_COLOR};
use crate::signals::processing::{format_lap_time, get_fastest_lap};
use crate::ActivePage;
use crate::OpenDavApp;
use rfd::FileDialog;

impl OpenDavApp {
    pub fn draw_splash_screen(&mut self, ctx: &egui::Context, progress: f32) {
        // Render splash screen with a sleek obsidian backdrop
        let panel_frame = egui::Frame::central_panel(&ctx.style())
            .fill(DARK_BG_COLOR)
            .inner_margin(egui::Margin::same(0));
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
            let size = ui.available_size();
            
            let bg_bytes = include_bytes!("../../assets/splash_bg.jpg");
            let bg_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size);
            
            let bg_img = egui::Image::from_bytes("bytes://splash_bg.jpg", bg_bytes.to_vec())
                .fit_to_exact_size(size);
            ui.put(bg_rect, bg_img);
            
            // Dark overlay
                ui.painter()
                    .rect_filled(bg_rect, 0.0, egui::Color32::from_black_alpha(220));
            
            // Center the logo and loading bar vertically and horizontally
            let logo2_width = 550.0;
            let logo2_height = logo2_width * (1440.0 / 2560.0); // 2560x1440 ratio
            let group_height = logo2_height + 40.0 + 4.0;
            let start_y = (size.y - group_height) / 2.0;
            
            let logo2_rect = egui::Rect::from_min_size(
                egui::pos2((size.x - logo2_width) / 2.0, start_y),
                    egui::vec2(logo2_width, logo2_height),
            );

            let logo1_width = 300.0;
            let logo1_rect = egui::Rect::from_center_size(
                logo2_rect.center(),
                    egui::vec2(logo1_width, logo1_width),
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
                    let img = egui::Image::from_bytes(
                        "bytes://logo_transparent_lighttext_splash.png",
                        logo1_bytes.to_vec(),
                    )
                    .show_loading_spinner(false)
                    .tint(egui::Color32::from_white_alpha((alpha1 * 255.0) as u8));
                ui.put(logo1_rect, img);
            }
            
            if alpha2 > 0.0 {
                    let img = egui::Image::from_bytes(
                        "bytes://opendav_transparent_lighttext_splash.png",
                        logo2_bytes.to_vec(),
                    )
                    .show_loading_spinner(false)
                    .tint(egui::Color32::from_white_alpha((alpha2 * 255.0) as u8));
                ui.put(logo2_rect, img);
            }
            
            // Draw the loading progress bar underneath the logo
            let bar_width = 300.0;
            let bar_height = 3.0; // Thin and elegant
            let bar_rect = egui::Rect::from_center_size(
                egui::pos2(size.x / 2.0, logo2_rect.max.y + 40.0),
                    egui::vec2(bar_width, bar_height),
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
        let theme = AppTheme::for_mode(is_dark);

        // 1. GRAPHS PAGE: WORKSPACE TABS + INDEPENDENT MAP TOGGLE
        if self.active_page == ActivePage::Graphs {
            let active_tab = self.active_sidebar_tab;

            // A. Fixed 38px Far-Left Strip for rotated tab buttons
            egui::SidePanel::left("details_tab_strip")
                .resizable(false)
                .exact_width(38.0)
                .frame(egui::Frame::none().fill(theme.surface_panel))
                .show(ctx, |ui| {
                    ui.add_space(15.0);
                    
                    let tab_configs = [
                        (crate::GraphsSidebarTab::Details, "DETAILS"),
                        (crate::GraphsSidebarTab::Values, "VALUES"),
                    ];

                    for (tab_type, label_text) in tab_configs {
                        let is_tab_active = active_tab == Some(tab_type);
                        let btn_size = egui::vec2(30.0, 95.0);
                        let (rect, response) =
                            ui.allocate_exact_size(btn_size, egui::Sense::click());
                        let is_hovered = response.hovered();
                        
                        let bg_color = if is_tab_active {
                            theme.surface_elevated
                        } else if is_hovered {
                            theme.surface_card
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let stroke_color = if is_tab_active {
                            ACCENT_COLOR
                        } else if is_hovered {
                            theme.border_strong
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        ui.painter().rect_filled(rect, 4.0, bg_color);
                        if stroke_color != egui::Color32::TRANSPARENT {
                            ui.painter().rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(1.5, stroke_color),
                                egui::StrokeKind::Inside,
                            );
                        }

                        let text_color = if is_tab_active {
                            theme.accent_text
                        } else if is_hovered {
                            theme.text_primary
                        } else {
                            theme.text_tertiary
                        };

                        let font_id = egui::FontId::proportional(11.0);
                        let galley = ui.painter().layout_no_wrap(
                            label_text.to_string(),
                            font_id,
                            text_color,
                        );
                        let text_width = galley.size().x;
                        let text_height = galley.size().y;

                        let rotated_origin = egui::pos2(
                            rect.center().x - text_height / 2.0,
                            rect.center().y + text_width / 2.0,
                        );
                        let mut shape =
                            egui::epaint::TextShape::new(rotated_origin, galley, text_color);
                        shape.angle = -std::f32::consts::FRAC_PI_2;
                        ui.painter().add(egui::Shape::Text(shape));

                        if response.clicked() {
                            if is_tab_active {
                                self.active_sidebar_tab = None;
                                self.is_details_sidebar_open = false;
                            } else {
                                self.active_sidebar_tab = Some(tab_type);
                                self.is_details_sidebar_open = true;
                            }
                        }

                        ui.add_space(8.0);
                    }

                    let is_map_active = self.show_graphs_track_map;
                    let btn_size = egui::vec2(30.0, 95.0);
                    let (rect, response) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                    let is_hovered = response.hovered();

                    let bg_color = if is_map_active {
                        theme.surface_elevated
                    } else if is_hovered {
                        theme.surface_card
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let stroke_color = if is_map_active {
                        ACCENT_COLOR
                    } else if is_hovered {
                        theme.border_strong
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    ui.painter().rect_filled(rect, 4.0, bg_color);
                    if stroke_color != egui::Color32::TRANSPARENT {
                        ui.painter().rect_stroke(
                            rect,
                            4.0,
                            egui::Stroke::new(1.5, stroke_color),
                            egui::StrokeKind::Inside,
                        );
                    }

                    let text_color = if is_map_active {
                        theme.accent_text
                    } else if is_hovered {
                        theme.text_primary
                    } else {
                        theme.text_tertiary
                    };
                    let font_id = egui::FontId::proportional(11.0);
                    let galley = ui
                        .painter()
                        .layout_no_wrap("MAP".to_owned(), font_id, text_color);
                    let rotated_origin = egui::pos2(
                        rect.center().x - galley.size().y / 2.0,
                        rect.center().y + galley.size().x / 2.0,
                    );
                    let mut shape =
                        egui::epaint::TextShape::new(rotated_origin, galley, text_color);
                    shape.angle = -std::f32::consts::FRAC_PI_2;
                    ui.painter().add(egui::Shape::Text(shape));

                    if response.on_hover_text("Toggle Track Map").clicked() {
                        self.show_graphs_track_map = !self.show_graphs_track_map;
                        if self.show_graphs_track_map {
                            self.graphs_track_map_rotation = std::f64::consts::FRAC_PI_2;
                            self.reset_track_map_bounds_flag = true;
                            self.reset_track_map_bounds_next_frame = 3;
                        }
                    }
                });

            // B. Collapsible Drawer Panel
            let window_width = ctx.screen_rect().width();
            let narrow_workspace = window_width < 1280.0;
            if let Some(tab) =
                active_tab.filter(|_| !self.show_graphs_track_map || !narrow_workspace)
            {
                egui::SidePanel::left("graphs_lap_sidebar")
                    .resizable(false)
                    .default_width(270.0) 
                    .show(ctx, |ui| {
                        ui.add_space(15.0);
                        match tab {
                            crate::GraphsSidebarTab::Details => {
                                self.draw_graphs_sidebar_content(ui, is_dark)
                            }
                            crate::GraphsSidebarTab::Values => {
                                self.draw_values_sidebar_content(ui, is_dark)
                            }
                        }
                    });
            }

            if self.show_graphs_track_map {
                let map_panel_resp = egui::SidePanel::left("graphs_track_map_panel")
                    .resizable(true)
                    .default_width(self.graphs_track_map_width)
                    .width_range(240.0..=820.0)
                    .frame(
                        egui::Frame::none()
                            .fill(theme.surface_panel)
                            .inner_margin(egui::Margin::same(8)),
                    )
                    .show(ctx, |ui| {
                        let height = ui.available_height();
                        self.draw_interactive_track_map(
                            ui,
                            height,
                            crate::rendering::track_map::TrackMapPlacement::GraphsSidebar,
                        );
                    });

                let is_mouse_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                let actual_w = map_panel_resp.response.rect.width();
                if is_mouse_down && actual_w >= 240.0 && actual_w <= 820.0 {
                    self.graphs_track_map_width = actual_w;
                }
                ctx.data_mut(|d| {
                    d.insert_persisted(egui::Id::new("graphs_track_map_panel"), self.graphs_track_map_width);
                });
            }
            return;
        }

        // 2. NON-GRAPHS PAGES: RENDER STANDARD NAVIGATION SIDEBAR
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .default_width(260.0) 
            .show(ctx, |ui| {
                ui.add_space(15.0);
                self.draw_main_nav_sidebar_content(ui, is_dark);
            });
    }

    fn draw_main_nav_sidebar_content(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let theme = AppTheme::for_mode(is_dark);
        let (logo_id, corner_bytes): (&str, &[u8]) = if is_dark {
            (
                "bytes://logo_dark_lighttext.png",
                include_bytes!("../../assets/logo_dark_lighttext.png"),
            )
        } else {
            (
                "bytes://logo_light_darktext.png",
                include_bytes!("../../assets/logo_light_darktext.png"),
            )
        };
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Image::from_bytes(logo_id, corner_bytes.to_vec())
                    .show_loading_spinner(false)
                    .max_width(150.0)
                    .maintain_aspect_ratio(true),
            );
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(15.0);

        let sidebar_style = ui.style_mut();
        sidebar_style.spacing.button_padding = egui::vec2(16.0, 12.0);

        ui.vertical(|ui| {
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
                hover_f * (1.0 - sel_f) + sel_f,
            );
            if color.a() > 0.01 {
                ui.painter().rect_stroke(
                    resp.rect.expand(1.0),
                    8.0,
                    egui::Stroke::new(2.0, color),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                self.active_page = ActivePage::OpenDav;
            }

            ui.add_space(15.0);

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
                hover_f * (1.0 - sel_f) + sel_f,
            );
            if color.a() > 0.01 {
                ui.painter().rect_stroke(
                    resp.rect.expand(1.0),
                    8.0,
                    egui::Stroke::new(2.0, color),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                self.active_page = ActivePage::Graphs;
                if !self.sessions.is_empty() && self.selected_lap.is_none() {
                    let p_idx = self.primary_session_idx;
                    let session = &self.sessions[p_idx].session;
                    let fastest_lap = get_fastest_lap(&session.lap_times);
                    self.selected_lap = if fastest_lap > 0 {
                        Some((p_idx, fastest_lap))
                    } else {
                        None
                    };
                }
            }

            ui.add_space(15.0);

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
                hover_f * (1.0 - sel_f) + sel_f,
            );
            if color.a() > 0.01 {
                ui.painter().rect_stroke(
                    resp.rect.expand(1.0),
                    8.0,
                    egui::Stroke::new(2.0, color),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                self.active_page = ActivePage::Reports;
            }

            ui.add_space(15.0);

            let simgit_bytes = include_bytes!("../../assets/button_simgit.png");
            let is_simgit_selected = self.active_page == ActivePage::SimGit;
            let simgit =
                egui::Image::from_bytes("bytes://button_simgit.png", simgit_bytes.to_vec())
                    .max_width(240.0)
                    .corner_radius(8.0)
                    .sense(egui::Sense::click());
            let response = ui.add(simgit).on_hover_text("SimGit workspaces");
            let hover = ui
                .ctx()
                .animate_bool(response.id.with("hover"), response.hovered());
            let selected = ui
                .ctx()
                .animate_bool(response.id.with("sel"), is_simgit_selected);
            let stroke = if selected > 0.0 {
                egui::Stroke::new(2.0, theme.accent)
            } else if hover > 0.0 {
                egui::Stroke::new(1.5, theme.border_strong)
            } else {
                egui::Stroke::NONE
            };
            if stroke != egui::Stroke::NONE {
                ui.painter().rect_stroke(
                    response.rect.expand(1.0),
                    8.0,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            if response.clicked() {
                self.active_page = ActivePage::SimGit;
            }

            ui.add_space(15.0);
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let is_settings = self.active_page == ActivePage::Settings;
                
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_settings);
                let acc = egui::Rgba::from(ACCENT_COLOR);
                let base = egui::Rgba::from(egui::Color32::DARK_GRAY);
                let gear_color: egui::Color32 = egui::Rgba::from_rgba_premultiplied(
                    base.r() * (1.0 - hover_f - sel_f).max(0.0)
                        + 1.0 * hover_f * (1.0 - sel_f)
                        + acc.r() * sel_f,
                    base.g() * (1.0 - hover_f - sel_f).max(0.0)
                        + 1.0 * hover_f * (1.0 - sel_f)
                        + acc.g() * sel_f,
                    base.b() * (1.0 - hover_f - sel_f).max(0.0)
                        + 1.0 * hover_f * (1.0 - sel_f)
                        + acc.b() * sel_f,
                    base.a() * (1.0 - hover_f - sel_f).max(0.0) + hover_f * (1.0 - sel_f) + sel_f,
                )
                .into();
                
                ui.painter().text(
                    rect.center(), 
                    egui::Align2::CENTER_CENTER, 
                    "⚙", 
                    egui::FontId::proportional(22.0), 
                    gear_color,
                );
                
                if resp.on_hover_text("Settings").clicked() {
                    self.active_page = ActivePage::Settings;
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("v0.9.0-rs")
                            .color(egui::Color32::DARK_GRAY)
                            .small(),
                    );
                });
            });
        });
    }

    fn draw_graphs_sidebar_content(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let theme = AppTheme::for_mode(is_dark);
        ui.vertical(|ui| {
            ui.add_space(5.0);
            if ui
                .button(
                    egui::RichText::new("⬅  Back to OpenDAV")
                        .strong()
                        .color(theme.accent_text),
                )
                .clicked()
            {
                self.active_page = ActivePage::OpenDav;
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            let select_hdr_color = theme.text_secondary;
            ui.label(
                egui::RichText::new("LAP TIMELINE SELECT")
                    .color(select_hdr_color)
                    .size(10.0)
                    .strong(),
            );
            ui.add_space(8.0);

                            if self.sessions.is_empty() {
                ui.label(
                    egui::RichText::new("No Session Active")
                        .color(theme.text_tertiary)
                        .small(),
                );
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
                            let header_color = if is_primary {
                                theme.accent
                            } else {
                                theme.surface_elevated
                            };
                            let text_color = if is_primary {
                                theme.on_accent
                            } else {
                                theme.text_primary
                            };

                                            let mut local_remove = false;
                                            let header_btn = egui::Frame::none()
                                                .fill(header_color)
                                                .corner_radius(4.0)
                                                .inner_margin(egui::Margin::symmetric(6, 4))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        let display_name = if loaded_session.file_name.len() > 22 {
                                                            format!("{}...", &loaded_session.file_name[..19])
                                                        } else {
                                                            loaded_session.file_name.clone()
                                                        };
                                        let btn = ui
                                            .selectable_label(
                                                is_primary,
                                                egui::RichText::new(&display_name)
                                                    .color(text_color)
                                                    .strong(),
                                            )
                                                            .on_hover_text(&loaded_session.file_name);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        egui::RichText::new("🗑").color(text_color),
                                                    )
                                                    .clicked()
                                                {
                                                                local_remove = true;
                                                            }
                                            },
                                        );
                                                        btn
                                    })
                                    .inner
                                })
                                .inner;

                                            if local_remove {
                                                session_to_remove = Some(s_idx);
                                            } else if header_btn.clicked() {
                                                new_primary_idx = Some(s_idx);
                                            }
                                            
                                            ui.add_space(4.0);

                                            let lap_times = &loaded_session.session.lap_times;
                                            let fastest_lap = get_fastest_lap(lap_times);

                                            egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                                .corner_radius(4.0)
                                                .inner_margin(egui::Margin::symmetric(6, 4))
                                                .show(ui, |ui| {
                                                for (lap_num, duration) in lap_times {
                                        let is_selected =
                                            self.selected_lap == Some((s_idx, *lap_num));
                                                    let is_fastest = *lap_num == fastest_lap && *lap_num > 0;

                                                    let is_cyan = self.ref_lap_cyan == Some((s_idx, *lap_num));
                                        let is_white =
                                            self.ref_lap_white == Some((s_idx, *lap_num));

                                                    let label_color = if is_selected {
                                            theme.accent_text
                                                    } else if is_fastest {
                                            theme.brand_secondary
                                                    } else {
                                            theme.text_primary
                                                    };

                                                    ui.horizontal(|ui| {
                                                        // 1. Cyan Reference Toggle Box (Left)
                                            let active_cyan = theme.reference_primary;
                                            let border_color_c = if is_cyan {
                                                active_cyan
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            };
                                                        
                                                        let btn_c = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_c))
                                                            .corner_radius(4.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                                            .show(ui, |ui| {
                                                    ui.selectable_label(
                                                        false,
                                                        egui::RichText::new("C")
                                                            .color(if is_cyan {
                                                                active_cyan
                                                            } else {
                                                                theme.text_disabled
                                                            })
                                                            .strong(),
                                                    )
                                                })
                                                .inner;
                                                        
                                                        if btn_c.clicked() {
                                                            if is_cyan {
                                                                toggle_cyan_off = true;
                                                            } else {
                                                                new_ref_cyan = Some((s_idx, *lap_num));
                                                            }
                                                        }

                                                        // 2. White Reference Toggle Box (Right)
                                            let active_white = theme.reference_secondary;
                                            let border_color_w = if is_white {
                                                active_white
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            };
                                                        
                                                        let btn_w = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_w))
                                                            .corner_radius(4.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                                            .show(ui, |ui| {
                                                    ui.selectable_label(
                                                        false,
                                                        egui::RichText::new("S")
                                                            .color(if is_white {
                                                                active_white
                                                            } else {
                                                                theme.text_disabled
                                                            })
                                                            .strong(),
                                                    )
                                                })
                                                .inner;
                                                        
                                                        if btn_w.clicked() {
                                                            if is_white {
                                                                toggle_white_off = true;
                                                            } else {
                                                                new_ref_white = Some((s_idx, *lap_num));
                                                            }
                                                        }

                                                        // 3. Main Lap Timeline Selection Selector
                                            let mut text = format!(
                                                "Lap {} : {}",
                                                lap_num,
                                                format_lap_time(*duration)
                                            );
                                                        if is_fastest {
                                                            text += " ★";
                                                        }

                                            let border_color_l = if is_selected {
                                                theme.accent
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            };
                                                        
                                                        let btn_l = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_l))
                                                            .corner_radius(4.0)
                                                            .inner_margin(egui::Margin::symmetric(6, 3))
                                                            .show(ui, |ui| {
                                                    ui.selectable_label(
                                                        false,
                                                        egui::RichText::new(text)
                                                            .color(label_color)
                                                            .strong(),
                                                    )
                                                })
                                                .inner;

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
                                        
                        let handle_ref_lap = |r: &mut Option<(usize, i32)>| {
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
                                    self.reset_track_map_bounds_flag = true;
                                    self.reset_track_map_bounds_next_frame = 3;
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
                    if self.primary_session_idx != sl.0 {
                        self.primary_session_idx = sl.0;
                    }
                                    self.selected_lap = Some(sl);
                    if let Some(pos) = self.sessions[sl.0]
                        .lap_ranges
                        .iter()
                        .position(|r| r.0 == sl.1)
                    {
                                        let (_, start_t, _end_t) = self.sessions[sl.0].lap_ranges[pos];
                                        self.cursor_x = Some(start_t);
                                        self.reset_bounds_flag = true;
                                        self.reset_bounds_next_frame = 3;
                                        self.reset_track_map_bounds_flag = true;
                                        self.reset_track_map_bounds_next_frame = 3;
                                    }
                                    state_changed = true;
                                }
                                if state_changed && !self.sessions.is_empty() {
                                    self.update_sector_deltas();
                    self.update_lap_deltas();
                                }
                            }
                        });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let is_settings = self.active_page == ActivePage::Settings;
                        
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                        let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                        let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_settings);
                        
                        let base = egui::Rgba::from(egui::Color32::GRAY);
                        let acc = egui::Rgba::from(ACCENT_COLOR);
                        let gear_color = egui::Rgba::from_rgba_premultiplied(
                    base.r() * (1.0 - hover_f - sel_f).max(0.0)
                        + 1.0 * hover_f * (1.0 - sel_f)
                        + acc.r() * sel_f,
                    base.g() * (1.0 - hover_f - sel_f).max(0.0)
                        + 1.0 * hover_f * (1.0 - sel_f)
                        + acc.g() * sel_f,
                    base.b() * (1.0 - hover_f - sel_f).max(0.0)
                        + 1.0 * hover_f * (1.0 - sel_f)
                        + acc.b() * sel_f,
                    base.a() * (1.0 - hover_f - sel_f).max(0.0) + hover_f * (1.0 - sel_f) + sel_f,
                )
                .into();
                        
                        ui.painter().text(
                            rect.center(), 
                            egui::Align2::CENTER_CENTER, 
                            "⚙", 
                            egui::FontId::proportional(22.0), 
                    gear_color,
                        );
                        
                        if resp.on_hover_text("Settings").clicked() {
                            self.active_page = ActivePage::Settings;
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("v0.9.0-rs")
                            .color(egui::Color32::DARK_GRAY)
                            .small(),
                    );
                        });
                    });
                });
    }

    fn draw_values_sidebar_content(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let theme = AppTheme::for_mode(is_dark);
        ui.vertical(|ui| {
            ui.add_space(5.0);
            if ui
                .button(
                    egui::RichText::new("⬅  Back to OpenDAV")
                        .strong()
                        .color(theme.accent_text),
                )
                .clicked()
            {
                self.active_page = ActivePage::OpenDav;
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            let select_hdr_color = theme.text_secondary;
            ui.label(
                egui::RichText::new("LIVE TELEMETRY VALUES")
                    .color(select_hdr_color)
                    .size(10.0)
                    .strong(),
            );
            ui.add_space(6.0);

            // Channel Search Bar
            ui.add(
                egui::TextEdit::singleline(&mut self.channel_search_query)
                    .hint_text("🔍 Search channels...")
                    .desired_width(ui.available_width()),
            );
            ui.add_space(8.0);

            if !self.session_loaded || self.sessions.is_empty() {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("No Telemetry Session Active")
                        .color(theme.text_tertiary)
                        .small(),
                );
                ui.label(
                    egui::RichText::new("Load an IBT file to inspect live channel values.")
                        .color(theme.text_tertiary)
                        .small(),
                );
            } else {
                let p_idx = self.primary_session_idx;
                let session = &self.sessions[p_idx].session;
                let df = &session.dataframe;

                let row_idx = if let Ok(col) = df.column("SessionTime") {
                    if let Ok(ca) = col.f64() {
                        let cx = self.cursor_x.unwrap_or(0.0);
                        match ca.cont_slice() {
                            Ok(slice) => crate::signals::processing::get_closest_index(slice, cx),
                            Err(_) => 0,
                        }
                    } else {
                        0
                    }
                } else {
                    0
                };

                let query_lower = self.channel_search_query.to_lowercase();

                let mut channel_names: Vec<String> = df
                    .get_column_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .filter(|name| {
                        query_lower.is_empty() || name.to_lowercase().contains(&query_lower)
                    })
                    .collect();
                channel_names.sort_by_key(|name| name.to_lowercase());

                let total_cols = df.width();
                let filtered_count = channel_names.len();

                ui.label(
                    egui::RichText::new(format!(
                        "Showing {} of {} channels",
                        filtered_count, total_cols
                    ))
                    .color(theme.text_tertiary)
                    .small(),
                );
                if filtered_count == 0 {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("No matching channels").color(theme.text_secondary),
                    );
                    if ui.small_button("Clear search").clicked() {
                        self.channel_search_query.clear();
                    }
                }
                let use_metric = self.settings.use_metric;
                let empty_unit = String::new();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical(|ui| {
                        for col_name in channel_names {
                            let raw_unit =
                                session.channel_units.get(&col_name).unwrap_or(&empty_unit);

                            if let Ok(col) = df.column(&col_name) {
                                let (val_str, unit_display) = if row_idx < col.len() {
                                    match col.get(row_idx) {
                                        Ok(polars::prelude::AnyValue::Float64(f)) => {
                                            format_channel_unit_val(
                                                &col_name, f, raw_unit, use_metric,
                                            )
                                        }
                                        Ok(polars::prelude::AnyValue::Float32(f)) => {
                                            format_channel_unit_val(
                                                &col_name, f as f64, raw_unit, use_metric,
                                            )
                                        }
                                        Ok(polars::prelude::AnyValue::Int64(i)) => {
                                            format_channel_unit_val(
                                                &col_name, i as f64, raw_unit, use_metric,
                                            )
                                        }
                                        Ok(polars::prelude::AnyValue::Int32(i)) => {
                                            format_channel_unit_val(
                                                &col_name, i as f64, raw_unit, use_metric,
                                            )
                                        }
                                        Ok(polars::prelude::AnyValue::Int16(i)) => {
                                            format_channel_unit_val(
                                                &col_name, i as f64, raw_unit, use_metric,
                                            )
                                        }
                                        Ok(polars::prelude::AnyValue::Int8(i)) => {
                                            format_channel_unit_val(
                                                &col_name, i as f64, raw_unit, use_metric,
                                            )
                                        }
                                        Ok(polars::prelude::AnyValue::Boolean(b)) => (
                                            if b {
                                                "true".to_string()
                                            } else {
                                                "false".to_string()
                                            },
                                            raw_unit.to_string(),
                                        ),
                                        Ok(polars::prelude::AnyValue::String(s)) => {
                                            (s.to_string(), raw_unit.to_string())
                                        }
                                        Ok(other) => (format!("{:?}", other), raw_unit.to_string()),
                                        Err(_) => ("N/A".to_string(), raw_unit.to_string()),
                                    }
                                } else {
                                    ("N/A".to_string(), raw_unit.to_string())
                                };

                                egui::Frame::none()
                                    .fill(theme.surface_card)
                                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                    .corner_radius(4.0)
                                    .inner_margin(egui::Margin::symmetric(8, 5))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let name = ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&col_name)
                                                        .small()
                                                        .strong()
                                                        .color(theme.text_primary),
                                                )
                                                .truncate(),
                                            );
                                            name.on_hover_text(&col_name);
                                            if !unit_display.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "[{}]",
                                                        unit_display
                                                    ))
                                                    .size(9.0)
                                                    .color(theme.text_tertiary),
                                                );
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(val_str)
                                                            .small()
                                                            .strong()
                                                            .color(theme.accent_text),
                                                    );
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(3.0);
                            }
                        }
                    });
                });
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let is_settings = self.active_page == ActivePage::Settings;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                    let hover_f = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                    let sel_f = ui.ctx().animate_bool(resp.id.with("sel"), is_settings);
                    let acc = egui::Rgba::from(ACCENT_COLOR);
                    let base = egui::Rgba::from(theme.text_tertiary);
                    let gear_color: egui::Color32 = egui::Rgba::from_rgba_premultiplied(
                        base.r() * (1.0 - hover_f - sel_f).max(0.0)
                            + 1.0 * hover_f * (1.0 - sel_f)
                            + acc.r() * sel_f,
                        base.g() * (1.0 - hover_f - sel_f).max(0.0)
                            + 1.0 * hover_f * (1.0 - sel_f)
                            + acc.g() * sel_f,
                        base.b() * (1.0 - hover_f - sel_f).max(0.0)
                            + 1.0 * hover_f * (1.0 - sel_f)
                            + acc.b() * sel_f,
                        base.a() * (1.0 - hover_f - sel_f).max(0.0)
                            + hover_f * (1.0 - sel_f)
                            + sel_f,
                    )
                    .into();
                    
                    ui.painter().text(
                        rect.center(), 
                        egui::Align2::CENTER_CENTER, 
                        "⚙", 
                        egui::FontId::proportional(22.0), 
                        gear_color,
                    );
                    
                    if resp.on_hover_text("Settings").clicked() {
                        self.active_page = ActivePage::Settings;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("v0.9.0-rs")
                                .color(egui::Color32::DARK_GRAY)
                                .small(),
                        );
                    });
                });
            });
        });
    }

    pub fn draw_top_panel(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        let theme = AppTheme::for_mode(is_dark);
        
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
                                let _ = writeln!(
                                    file,
                                    "# Total Time: {:.3}s",
                                    session.total_session_time
                                );
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
                    let display_name = if primary_file_name.len() > 22 {
                        format!("{}...", &primary_file_name[..19])
                    } else {
                        primary_file_name.clone()
                    };
                    
                    if self.sessions.len() == 1 {
                        ui.label(
                            egui::RichText::new(format!("File: {}", display_name))
                                .color(theme.text_secondary)
                                .small(),
                        )
                            .on_hover_text(primary_file_name);
                    } else {
                        let mut new_primary = None;
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Primary File:")
                                    .color(theme.text_secondary)
                                    .small(),
                            );
                            egui::ComboBox::from_id_source("top_primary_session_dropdown")
                                .selected_text(egui::RichText::new(display_name).small())
                                .show_ui(ui, |ui| {
                                    for (idx, session) in self.sessions.iter().enumerate() {
                                        let s_display_name = if session.file_name.len() > 22 {
                                            format!("{}...", &session.file_name[..19])
                                        } else {
                                            session.file_name.clone()
                                        };
                                        if ui
                                            .selectable_label(
                                                self.primary_session_idx == idx,
                                                &s_display_name,
                                            )
                                            .on_hover_text(&session.file_name)
                                            .clicked()
                                        {
                                            new_primary = Some(idx);
                                        }
                                    }
                                });
                        });
                        
                        if let Some(idx) = new_primary {
                            self.primary_session_idx = idx;
                            let fastest = get_fastest_lap(&self.sessions[idx].session.lap_times);
                            self.selected_lap = (fastest > 0).then_some((idx, fastest));
                            self.update_sector_deltas();
                            self.update_lap_deltas();
                            self.reset_bounds_flag = true;
                            self.reset_bounds_next_frame = 3;
                            self.reset_track_map_bounds_flag = true;
                            self.reset_track_map_bounds_next_frame = 3;
                        }
                    }

                    // --- TELEMETRY PLAYBACK CONTROLS (TOP PANEL NEXT TO FILE DETAILS) ---
                    ui.separator();
                    let play_icon = if self.is_playing {
                        "⏸ Pause"
                    } else {
                        "▶ Play"
                    };
                    let play_color = if self.is_playing {
                        theme.danger
                    } else {
                        theme.accent_text
                    };
                    
                    let play_btn = ui.add(egui::Button::new(
                        egui::RichText::new(play_icon)
                            .strong()
                            .color(play_color)
                            .size(12.0),
                    ));
                    if play_btn.clicked() {
                        self.is_playing = !self.is_playing
                            && !self.sessions.is_empty()
                            && self.selected_lap.is_some();
                    }

                    let mut speed_text = "1.0x";
                    if self.playback_speed == 0.5 {
                        speed_text = "0.5x";
                    } else if self.playback_speed == 2.0 {
                        speed_text = "2.0x";
                    }
                    
                    egui::ComboBox::from_id_source("top_playback_speed")
                        .selected_text(egui::RichText::new(speed_text).size(12.0))
                        .width(48.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.playback_speed, 0.5, "0.5x");
                            ui.selectable_value(&mut self.playback_speed, 1.0, "1.0x");
                            ui.selectable_value(&mut self.playback_speed, 2.0, "2.0x");
                        });
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.session_loaded {
                        if ui.button("🔄 Reset Zoom").clicked() {
                            self.reset_bounds_flag = true;
                            self.reset_bounds_next_frame = 3;
                            self.reset_track_map_bounds_flag = true;
                            self.reset_track_map_bounds_next_frame = 3;
                        }
                        ui.separator();
                    }
                    let theme_icon = if self.settings.dark_mode {
                        "🌙"
                    } else {
                        "☀️"
                    };
                    if ui
                        .button(theme_icon)
                        .on_hover_text("Toggle Theme")
                        .clicked()
                    {
                        self.settings.dark_mode = !self.settings.dark_mode;
                        self.settings.save();
                    }
                });
            });
            ui.add_space(6.0);
        });
    }
}

fn format_channel_unit_val(
    name: &str,
    raw_val: f64,
    raw_unit: &str,
    use_metric: bool,
) -> (String, String) {
    let u_clean = raw_unit.trim();
    let u_lower = u_clean.to_lowercase();
    let name_lower = name.to_lowercase();

    if u_lower == "m/s" || name_lower.contains("speed") || name_lower.contains("velocity") {
        if use_metric {
            let kmh = raw_val * 3.6;
            (format!("{:.1}", kmh), "km/h".to_string())
        } else {
            let mph = raw_val * 2.23694;
            (format!("{:.1}", mph), "mph".to_string())
        }
    } else if u_lower.contains('c')
        && (u_lower.contains("deg")
            || u_lower.contains('°')
            || u_clean == "C"
            || name_lower.contains("temp"))
    {
        if use_metric {
            (format!("{:.1}", raw_val), "°C".to_string())
        } else {
            let f = raw_val * 1.8 + 32.0;
            (format!("{:.1}", f), "°F".to_string())
        }
    } else if u_lower == "kpa" || u_lower == "bar" || name_lower.contains("press") {
        if use_metric {
            if u_lower == "kpa" {
                (format!("{:.1}", raw_val), "kPa".to_string())
            } else {
                (format!("{:.2}", raw_val), "bar".to_string())
            }
        } else {
            let psi = if u_lower == "bar" {
                raw_val * 14.5038
            } else {
                raw_val * 0.145038
            };
            (format!("{:.1}", psi), "psi".to_string())
        }
    } else if u_lower == "mm"
        || (u_lower == "m"
            && (name_lower.contains("height")
                || name_lower.contains("rake")
                || name_lower.contains("dist")))
    {
        if use_metric {
            if u_lower == "m" {
                (format!("{:.1}", raw_val), "m".to_string())
            } else {
                (format!("{:.2}", raw_val), "mm".to_string())
            }
        } else {
            if u_lower == "m" {
                let ft = raw_val * 3.28084;
                (format!("{:.1}", ft), "ft".to_string())
            } else {
                let inches = raw_val * 0.0393701;
                (format!("{:.3}", inches), "in".to_string())
            }
        }
    } else if u_lower == "rad" || u_lower == "rad/s" {
        let deg = raw_val * 57.2957795;
        let unit_str = if u_lower.contains("/s") { "°/s" } else { "°" };
        (format!("{:.1}", deg), unit_str.to_string())
    } else if u_lower == "%" || u_clean == "%" {
        let val_pct = if (0.0..=1.0).contains(&raw_val) {
            raw_val * 100.0
        } else {
            raw_val
        };
        (format!("{:.1}", val_pct), "%".to_string())
    } else {
        let fmt = if raw_val.fract() == 0.0 {
            format!("{:.0}", raw_val)
        } else {
            format!("{:.2}", raw_val)
        };
        (fmt, u_clean.to_string())
    }
}
