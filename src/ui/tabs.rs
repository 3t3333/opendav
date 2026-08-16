use crate::config::theme::AppTheme;
use crate::config::worksheet::WorksheetConfig;
use crate::signals::processing::{
    format_lap_time, format_sector_time, get_fastest_lap, get_lap_time_at_distance,
};
use crate::ActivePage;
use crate::OpenDavApp;

pub struct TreeBehavior<'a> {
    pub app: &'a mut OpenDavApp,
    pub is_tab_switch: bool,
}

impl<'a> egui_tiles::Behavior<crate::config::workbook::Pane> for TreeBehavior<'a> {
    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile_id: egui_tiles::TileId, pane: &mut crate::config::workbook::Pane) -> egui_tiles::UiResponse {
        match pane {
            crate::config::workbook::Pane::TimeSeries { id, config } => {
                self.app.draw_motec_plot(ui, id, config, self.is_tab_switch);
            }
        }
        egui_tiles::UiResponse::None
    }
    
    fn tab_title_for_pane(&mut self, pane: &crate::config::workbook::Pane) -> egui::WidgetText {
        match pane {
            crate::config::workbook::Pane::TimeSeries { .. } => "Time Series".into(),
        }
    }
}

fn report_time_cell(
    ui: &mut egui::Ui,
    theme: AppTheme,
    text: Option<String>,
    is_best: bool,
    is_near_best: bool,
) {
    let (fill, text_color, border_color) = match (text.is_some(), is_best, is_near_best) {
        (false, _, _) => (
            theme.surface_panel,
            theme.text_disabled,
            theme.border_subtle,
        ),
        (true, true, _) => (
            theme.surface_elevated,
            theme.brand_secondary,
            theme.brand_secondary,
        ),
        (true, false, true) => (theme.surface_elevated, theme.success, theme.success),
        (true, false, false) => (theme.surface_input, theme.text_primary, theme.border_subtle),
    };

    egui::Frame::NONE
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, border_color))
        .corner_radius(4.0)
        .inner_margin(egui::vec2(6.0, 4.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text.unwrap_or_else(|| "-".to_owned()))
                    .color(text_color)
                    .strong(),
            );
        });
}

impl OpenDavApp {
    pub fn draw_empty_state_drag_drop(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let ctx = ui.ctx().clone();
        let theme = AppTheme::for_mode(is_dark);
        let is_hovered = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let stroke_color = if is_hovered {
            theme.accent
        } else {
            theme.border_strong
        };
        let available_size = ui.available_size();

        egui::Frame::NONE
            .fill(theme.surface_panel)
            .stroke(egui::Stroke::new(3.0, stroke_color))
            .corner_radius(16.0)
            .inner_margin(egui::Margin::same(40))
            .show(ui, |ui| {
                ui.set_min_size(available_size);
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(
                            egui::RichText::new("Drop Telemetry File Here")
                                .size(32.0)
                                .strong()
                                .color(if is_hovered {
                                    theme.accent_text
                                } else {
                                    theme.text_primary
                                }),
                        );
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(
                                "Drag and drop an iRacing .ibt file onto this window,",
                            )
                            .size(18.0)
                            .color(theme.text_secondary),
                        );
                        ui.label(
                            egui::RichText::new(
                                "or click 'Browse Files' below to search manually.",
                            )
                            .size(18.0)
                            .color(theme.text_secondary),
                        );

                        ui.add_space(30.0);
                        let browse_btn = egui::Button::new(
                            egui::RichText::new("Browse Files")
                                .size(24.0)
                                .color(theme.on_accent),
                        )
                        .fill(theme.accent)
                        .stroke(egui::Stroke::new(1.0, theme.accent_text))
                        .corner_radius(12.0)
                        .min_size(egui::vec2(220.0, 50.0));

                        if ui.add(browse_btn).clicked() {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter("iRacing Telemetry", &["ibt"])
                                .set_title("Select Telemetry Files")
                                .pick_files()
                            {
                                for path in paths {
                                    self.load_telemetry_file(path.as_path());
                                }
                            }
                        }

                        if !self.settings.recent_files.is_empty() {
                            ui.add_space(50.0);
                            ui.label(
                                egui::RichText::new("RECENT FILES")
                                    .size(16.0)
                                    .strong()
                                    .color(theme.text_tertiary),
                            );
                            ui.add_space(15.0);

                            let recent_files = self.settings.recent_files.clone();
                            for recent in recent_files {
                                let file_name = std::path::Path::new(&recent)
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy();
                                let btn = egui::Button::new(
                                    egui::RichText::new(format!("📄 {}", file_name))
                                        .size(16.0)
                                        .color(theme.text_primary),
                                )
                                .fill(theme.surface_card)
                                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
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

        let theme = AppTheme::for_mode(is_dark);

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

        egui::ScrollArea::vertical()
            .id_salt("dashboard_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.heading(
                    egui::RichText::new("Dashboard")
                        .strong()
                        .color(theme.text_primary),
                );
                ui.add_space(8.0);

                // 1. Session Metadata Grid Card
                egui::Frame::group(ui.style())
                    .fill(theme.surface_card)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .show(ui, |ui| {
                        let columns = match ui.available_width() {
                            width if width >= 720.0 => 4,
                            width if width >= 360.0 => 2,
                            _ => 1,
                        };
                        let metadata = [
                            ("VEHICLE", car.as_str(), theme.accent_text),
                            ("VENUE", venue.as_str(), theme.text_primary),
                            ("AIR TEMP", air_temp.as_str(), theme.success),
                            ("TRACK TEMP", surface_temp.as_str(), theme.success),
                        ];

                        egui::Grid::new("dashboard_metadata_grid")
                            .num_columns(columns)
                            .min_col_width(
                                (ui.available_width() - 16.0 * (columns - 1) as f32)
                                    / columns as f32,
                            )
                            .spacing([16.0, 12.0])
                            .show(ui, |ui| {
                                for (index, (label, value, color)) in metadata.iter().enumerate() {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(*label)
                                                .color(theme.text_tertiary)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(*value).strong().color(*color),
                                        );
                                    });
                                    if (index + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });

                ui.add_space(10.0);

                // 2. High-End Session Statistics Cards
                let avg_lap = {
                    let filtered: Vec<&(i32, f64)> = lap_times
                        .iter()
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
                let statistics = [
                    (
                        "TOTAL SESSION TIME",
                        format_lap_time(total_session_time),
                        theme.text_primary,
                    ),
                    (
                        "TOTAL VALID LAPS",
                        format!("{} Laps", lap_times.len()),
                        theme.accent_text,
                    ),
                    (
                        "AVERAGE LAP TIME",
                        format_lap_time(avg_lap),
                        theme.text_primary,
                    ),
                ];
                let statistic_columns = match ui.available_width() {
                    width if width >= 700.0 => 3,
                    width if width >= 420.0 => 2,
                    _ => 1,
                };

                egui::Grid::new("dashboard_statistics_grid")
                    .num_columns(statistic_columns)
                    .min_col_width(
                        (ui.available_width() - 10.0 * (statistic_columns - 1) as f32)
                            / statistic_columns as f32,
                    )
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        for (index, (label, value, color)) in statistics.iter().enumerate() {
                            egui::Frame::group(ui.style())
                                .fill(theme.surface_card)
                                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(*label)
                                                .color(theme.text_tertiary)
                                                .small()
                                                .strong(),
                                        );
                                        ui.add_space(4.0);
                                        ui.heading(
                                            egui::RichText::new(value).strong().color(*color),
                                        );
                                    });
                                });
                            if (index + 1) % statistic_columns == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(15.0);

                let fastest_lap = get_fastest_lap(&lap_times);

                // 3. Stacked lower Dashboard (Top: Laps List, Bottom: Huge Track Map SVG!)
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("VALID LAP SHEET")
                            .color(theme.text_secondary)
                            .strong()
                            .size(11.0),
                    );
                    ui.add_space(4.0);

                    let lap_columns = match ui.available_width() {
                        width if width >= 900.0 => 4,
                        width if width >= 660.0 => 3,
                        width if width >= 420.0 => 2,
                        _ => 1,
                    };
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            egui::Grid::new("valid_laps_grid")
                                .num_columns(lap_columns)
                                .min_col_width(
                                    (ui.available_width() - 24.0 * (lap_columns - 1) as f32)
                                        / lap_columns as f32,
                                )
                                .spacing([24.0, 10.0])
                                .show(ui, |ui| {
                                    let mut col_count = 0;
                                    for (lap_num, duration) in &lap_times {
                                        let is_fastest = *lap_num == fastest_lap;
                                        let is_selected = self.selected_lap
                                            == Some((self.primary_session_idx, *lap_num));

                                        let mut row_text = format!(
                                            "Lap {} : {}",
                                            lap_num,
                                            format_lap_time(*duration)
                                        );
                                        if is_fastest {
                                            row_text += " FASTEST";
                                        }

                                        let (fill_color, row_color, border_color) = if is_selected {
                                            (theme.accent, theme.on_accent, theme.accent_text)
                                        } else if is_fastest {
                                            (
                                                theme.surface_elevated,
                                                theme.brand_secondary,
                                                theme.brand_secondary,
                                            )
                                        } else {
                                            (
                                                theme.surface_card,
                                                theme.text_primary,
                                                theme.border_subtle,
                                            )
                                        };

                                        let btn_resp = egui::Frame::NONE
                                            .fill(fill_color)
                                            .stroke(egui::Stroke::new(1.0, border_color))
                                            .corner_radius(4.0)
                                            .inner_margin(egui::Margin::symmetric(6, 3))
                                            .show(ui, |ui| {
                                                ui.selectable_label(
                                                    false,
                                                    egui::RichText::new(row_text)
                                                        .color(row_color)
                                                        .strong(),
                                                )
                                            })
                                            .inner;

                                        if btn_resp.clicked() {
                                            self.selected_lap =
                                                Some((self.primary_session_idx, *lap_num));
                                            if let Some(pos) = self.sessions
                                                [self.primary_session_idx]
                                                .lap_ranges
                                                .iter()
                                                .position(|r| r.0 == *lap_num)
                                            {
                                                let (_, start_t, _end_t) = self.sessions
                                                    [self.primary_session_idx]
                                                    .lap_ranges[pos];
                                                self.cursor_x = Some(start_t);
                                                self.reset_bounds_flag = true;
                                                self.reset_bounds_next_frame = 3;
                                                self.reset_track_map_bounds_flag = true;
                                                self.reset_track_map_bounds_next_frame = 3;
                                            }
                                            self.update_sector_deltas();
                                            self.update_lap_deltas();
                                        }

                                        col_count += 1;
                                        if col_count >= lap_columns {
                                            ui.end_row();
                                            col_count = 0;
                                        }
                                    }
                                });
                        });
                });

                ui.add_space(15.0);
                ui.label(
                    egui::RichText::new(venue.to_uppercase())
                        .color(theme.text_secondary)
                        .strong()
                        .size(11.0),
                );
                ui.add_space(4.0);

                egui::Frame::group(ui.style())
                    .fill(theme.surface_card)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .show(ui, |ui| {
                        self.draw_interactive_track_map(
                            ui,
                            340.0,
                            crate::rendering::track_map::TrackMapPlacement::Inline,
                        );
                    });

                ui.add_space(15.0);
                ui.vertical_centered(|ui| {
                    let open_graphs = egui::Button::new(
                        egui::RichText::new("📈 OPEN GRAPHS WORKSPACE")
                            .strong()
                            .size(12.0)
                            .color(theme.on_accent),
                    )
                    .fill(theme.accent)
                    .stroke(egui::Stroke::new(1.0, theme.accent_text))
                    .corner_radius(6.0)
                    .min_size(egui::vec2(240.0, 36.0));

                    if ui.add(open_graphs).clicked() {
                        self.active_page = ActivePage::Graphs;
                        let p_idx = self.primary_session_idx;
                        if p_idx < self.sessions.len() {
                            let fastest = get_fastest_lap(&self.sessions[p_idx].session.lap_times);
                            self.selected_lap = if fastest > 0 {
                                Some((p_idx, fastest))
                            } else {
                                None
                            };
                            self.visible_x_range = None;
                            self.reset_bounds_flag = true;
                            self.reset_bounds_next_frame = 3;
                            self.reset_track_map_bounds_flag = true;
                            self.reset_track_map_bounds_next_frame = 3;
                            self.update_sector_deltas();
                            self.update_lap_deltas();
                        }
                    }
                });
            });
    }

    pub fn draw_graphs_page(&mut self, ui: &mut egui::Ui) {
        if !self.session_loaded || self.sessions.is_empty() {
            let is_dark = self.settings.dark_mode;
            self.draw_empty_state_drag_drop(ui, is_dark);
            return;
        }

        if self.selected_lap.is_none() {
            let p_idx = self.primary_session_idx;
            if p_idx < self.sessions.len() {
                let fastest = get_fastest_lap(&self.sessions[p_idx].session.lap_times);
                if fastest > 0 {
                    self.selected_lap = Some((p_idx, fastest));
                    self.visible_x_range = None;
                    self.reset_bounds_flag = true;
                    self.reset_bounds_next_frame = 3;
                    self.reset_track_map_bounds_flag = true;
                    self.reset_track_map_bounds_next_frame = 3;
                }
            }
        }

        // 1. WORKBOOK COMBOBOX & WORKSHEET TABS
        ui.horizontal(|ui| {
            let mut selected_wb = self.active_workbook_idx;
            let wb_name = self.workbooks.get(selected_wb).map(|w| w.name.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("workbook_selector")
                .selected_text(wb_name)
                .show_ui(ui, |ui| {
                    for (i, wb) in self.workbooks.iter().enumerate() {
                        ui.selectable_value(&mut selected_wb, i, &wb.name);
                    }
                });
            if selected_wb != self.active_workbook_idx {
                self.active_workbook_idx = selected_wb;
                self.active_worksheet_idx = 0;
            }

            ui.menu_button("⚙", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rename:");
                    ui.text_edit_singleline(&mut self.workbooks[self.active_workbook_idx].name);
                });
                if ui.button("Delete Workbook").clicked() {
                    if self.workbooks.len() > 1 {
                        self.workbooks.remove(self.active_workbook_idx);
                        self.active_workbook_idx = 0;
                        self.active_worksheet_idx = 0;
                        ui.close_menu();
                    }
                }
            });
            if ui.button("+ WB").clicked() {
                self.workbooks.push(crate::config::workbook::Workbook {
                    name: format!("Workbook {}", self.workbooks.len() + 1),
                    worksheets: vec![crate::config::workbook::Worksheet {
                        name: "Worksheet 1".to_string(),
                        tree: egui_tiles::Tree::empty("new_ws"),
                    }],
                });
                self.active_workbook_idx = self.workbooks.len() - 1;
                self.active_worksheet_idx = 0;
            }

            ui.separator();

            let tab_style = ui.style_mut();
            tab_style.spacing.button_padding = egui::vec2(12.0, 8.0);
            
            let wb = &mut self.workbooks[self.active_workbook_idx];
            let mut ws_to_delete = None;
            for (i, ws) in wb.worksheets.iter_mut().enumerate() {
                let response = ui.selectable_label(self.active_worksheet_idx == i, &ws.name);
                if response.clicked() {
                    self.active_worksheet_idx = i;
                }
                response.context_menu(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Rename:");
                        ui.text_edit_singleline(&mut ws.name);
                    });
                    if ui.button("Delete Worksheet").clicked() {
                        ws_to_delete = Some(i);
                        ui.close_menu();
                    }
                });
            }
            if let Some(i) = ws_to_delete {
                if wb.worksheets.len() > 1 {
                    wb.worksheets.remove(i);
                    if self.active_worksheet_idx >= wb.worksheets.len() {
                        self.active_worksheet_idx = wb.worksheets.len() - 1;
                    }
                }
            }
            if ui.button("+").clicked() {
                wb.worksheets.push(crate::config::workbook::Worksheet {
                    name: format!("Worksheet {}", wb.worksheets.len() + 1),
                    tree: egui_tiles::Tree::empty("new_ws"),
                });
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        let mut is_tab_switch = false;
        if Some(self.active_page) != self.previous_page {
            is_tab_switch = true;
            self.previous_page = Some(self.active_page);
        }
        if Some(self.show_graphs_track_map) != self.previous_show_graphs_track_map {
            is_tab_switch = true;
            self.previous_show_graphs_track_map = Some(self.show_graphs_track_map);
        }
        if Some(self.active_worksheet_idx) != self.previous_worksheet_idx {
            is_tab_switch = true;
            self.previous_worksheet_idx = Some(self.active_worksheet_idx);
        }

        let mut behavior = TreeBehavior {
            app: self,
            is_tab_switch,
        };
        
        let active_wb_idx = behavior.app.active_workbook_idx;
        let active_ws_idx = behavior.app.active_worksheet_idx;
        let mut tree = behavior.app.workbooks[active_wb_idx].worksheets[active_ws_idx].tree.clone();

        let response = ui.allocate_response(ui.available_size(), egui::Sense::click());
        response.context_menu(|ui| {
            ui.menu_button("Add", |ui| {
                if ui.button("Time Series Graph").clicked() {
                    let pane = crate::config::workbook::Pane::TimeSeries { 
                        id: format!("ts_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()),
                        config: crate::config::worksheet::WorksheetConfig { lanes: vec![] },
                    };
                    let tile_id = tree.tiles.insert_pane(pane);
                    if let Some(root) = tree.root {
                        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(lin))) = tree.tiles.get_mut(root) {
                            lin.add_child(tile_id);
                        } else {
                            let linear = egui_tiles::Linear::new(egui_tiles::LinearDir::Horizontal, vec![root, tile_id]);
                            let new_root = tree.tiles.insert_container(egui_tiles::Container::Linear(linear));
                            tree.root = Some(new_root);
                        }
                    } else {
                        tree.root = Some(tile_id);
                    }
                    ui.close_menu();
                }
            });
        });

        // We render the tree in the same rect we allocated above
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(response.rect).layout(*ui.layout()));
        tree.ui(&mut behavior, &mut child_ui);

        if let Some(pane_id) = behavior.app.pane_to_close.take() {
            let mut tile_to_remove = None;
            for (tile_id, tile) in tree.tiles.iter() {
                if let egui_tiles::Tile::Pane(crate::config::workbook::Pane::TimeSeries { id, .. }) = tile {
                    if id == &pane_id {
                        tile_to_remove = Some(*tile_id);
                        break;
                    }
                }
            }
            if let Some(tile_id) = tile_to_remove {
                tree.tiles.remove(tile_id);
            }
        }

        let mut close_properties = false;
        if let Some(plot_id) = behavior.app.properties_window_open.clone() {
            let mut is_open = true;
            let mut window_drawn = false;
            
            for (_tile_id, tile) in tree.tiles.iter_mut() {
                if let egui_tiles::Tile::Pane(crate::config::workbook::Pane::TimeSeries { id, config }) = tile {
                    if id == &plot_id {
                        window_drawn = true;
                        egui::Window::new(format!("Properties: {}", plot_id))
                            .open(&mut is_open)
                            .show(ui.ctx(), |ui| {
                                let select_mode_id = ui.id().with("select_mode");
                                let selected_lanes_id = ui.id().with("selected_lanes");
                                let selected_traces_id = ui.id().with("selected_traces");

                                let mut is_select_mode = ui.data_mut(|d| d.get_temp::<bool>(select_mode_id).unwrap_or(false));
                                let mut selected_lanes = ui.data_mut(|d| d.get_temp::<std::collections::HashSet<usize>>(selected_lanes_id).unwrap_or_default());
                                let mut selected_traces = ui.data_mut(|d| d.get_temp::<std::collections::HashSet<(usize, usize)>>(selected_traces_id).unwrap_or_default());
                                let mut do_copy = false;
                                let mut do_cut = false;

                                ui.horizontal(|ui| {
                                    if ui.toggle_value(&mut is_select_mode, "Select Mode").changed() {
                                        if !is_select_mode {
                                            selected_lanes.clear();
                                            selected_traces.clear();
                                        }
                                    }
                                    if is_select_mode {
                                        if ui.button("Copy").clicked() { do_copy = true; }
                                        if ui.button("Cut").clicked() { do_cut = true; }
                                    }
                                });
                                ui.separator();

                                let mut needs_redistribute = false;
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let mut i = 0;
                                    while i < config.lanes.len() {
                                        ui.horizontal(|ui| {
                                            if is_select_mode {
                                                let mut selected = selected_lanes.contains(&i);
                                                if ui.checkbox(&mut selected, "").changed() {
                                                    if selected { selected_lanes.insert(i); } else { selected_lanes.remove(&i); }
                                                }
                                            }
                                            ui.text_edit_singleline(&mut config.lanes[i].title);
                                            if ui.button("↑").clicked() && i > 0 {
                                                config.lanes.swap(i, i - 1);
                                                needs_redistribute = true;
                                            }
                                            if ui.button("↓").clicked() && i + 1 < config.lanes.len() {
                                                config.lanes.swap(i, i + 1);
                                                needs_redistribute = true;
                                            }
                                            if ui.button("Remove Group").clicked() {
                                                config.lanes.remove(i);
                                                needs_redistribute = true;
                                            } else {
                                                ui.label(format!("({} traces)", config.lanes[i].traces.len()));
                                            }
                                        });
                                        if i < config.lanes.len() {
                                            ui.indent(format!("lane_{}", i), |ui| {
                                                let mut j = 0;
                                                while j < config.lanes[i].traces.len() {
                                                    ui.horizontal(|ui| {
                                                        if is_select_mode {
                                                            let mut selected = selected_traces.contains(&(i, j));
                                                            if ui.checkbox(&mut selected, "").changed() {
                                                                if selected { selected_traces.insert((i, j)); } else { selected_traces.remove(&(i, j)); }
                                                            }
                                                        }
                                                        ui.text_edit_singleline(&mut config.lanes[i].traces[j].name);
                                                        ui.color_edit_button_srgba(&mut config.lanes[i].traces[j].color);
                                                        if ui.button("Remove").clicked() {
                                                            config.lanes[i].traces.remove(j);
                                                        } else {
                                                            j += 1;
                                                        }
                                                    });
                                                }
                                                
                                                // We use a popup_below_widget instead of menu_button to prevent auto-closing!
                                                let btn_id = ui.id().with(format!("add_channel_popup_{}", i));
                                                let add_btn = ui.button("+ Add Channel");
                                                if add_btn.clicked() {
                                                    ui.memory_mut(|mem| mem.toggle_popup(btn_id));
                                                }
                                                egui::popup_below_widget(ui, btn_id, &add_btn, egui::PopupCloseBehavior::CloseOnClickOutside, |ui: &mut egui::Ui| {
                                                    ui.set_min_width(350.0);
                                                    let mut search_query = ui.data_mut(|d| d.get_temp::<String>(ui.id().with("search")).unwrap_or_default());
                                                    ui.horizontal(|ui| {
                                                        ui.label("🔍");
                                                        let response = ui.text_edit_singleline(&mut search_query);
                                                        response.request_focus();
                                                    });
                                                    ui.data_mut(|d| d.insert_temp(ui.id().with("search"), search_query.clone()));
                                                    ui.separator();
                                                    egui::ScrollArea::vertical().max_height(200.0).auto_shrink([false, false]).show(ui, |ui| {
                                                        let channel_names: Vec<String> = if behavior.app.session_loaded && !behavior.app.sessions.is_empty() {
                                                            let p_idx = behavior.app.primary_session_idx;
                                                            let mut names: Vec<String> = behavior.app.sessions[p_idx]
                                                                .session
                                                                .dataframe
                                                                .get_column_names()
                                                                .into_iter()
                                                                .map(|s| s.to_string())
                                                                .collect();
                                                            names.sort_by_key(|n| n.to_lowercase());
                                                            names
                                                        } else {
                                                            vec![]
                                                        };
                                                        let query = search_query.to_lowercase();
                                                        for name in channel_names {
                                                            if !query.is_empty() && !name.to_lowercase().contains(&query) {
                                                                continue;
                                                            }
                                                            if ui.button(name.clone()).clicked() {
                                                                config.lanes[i].traces.push(crate::config::worksheet::TraceSpec {
                                                                    name: name.clone(),
                                                                    cache: crate::config::worksheet::CacheSelector::Speed, // Fallback
                                                                    custom_channel: Some(name.clone()),
                                                                    color: egui::Color32::WHITE,
                                                                    width: 2.2,
                                                                    unit: "".to_string(),
                                                                });
                                                                ui.memory_mut(|mem| mem.close_popup(btn_id));
                                                            }
                                                        }
                                                    });
                                                });
                                                
                                                if let Some(crate::config::worksheet::WorksheetClipboard::Traces(traces)) = &behavior.app.worksheet_clipboard {
                                                    if ui.button("Paste Traces").clicked() {
                                                        config.lanes[i].traces.extend(traces.clone());
                                                    }
                                                }
                                            });
                                            i += 1;
                                        }
                                    }
                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        if ui.button("+ Add Group").clicked() {
                                            config.lanes.push(crate::config::worksheet::LaneSpec {
                                                title: "New Lane".to_string(),
                                                y_min: 0.0,
                                                y_max: 100.0,
                                                scaling: crate::config::worksheet::LaneScaling::Mono,
                                                traces: vec![],
                                            });
                                            needs_redistribute = true;
                                        }
                                        if let Some(crate::config::worksheet::WorksheetClipboard::Lanes(lanes)) = &behavior.app.worksheet_clipboard {
                                            if ui.button("Paste Groups").clicked() {
                                                config.lanes.extend(lanes.clone());
                                                needs_redistribute = true;
                                            }
                                        }
                                    });
                                    
                                    if needs_redistribute && !config.lanes.is_empty() {
                                        let n = config.lanes.len();
                                        let gap = 4.0;
                                        let total_gaps = (n - 1) as f64 * gap;
                                        let available_height = 86.0; // 98.0 (top margin) - 12.0 (bottom ticker boundary)
                                        let height_per_lane = (available_height - total_gaps) / n as f64;
                                        for (idx, lane) in config.lanes.iter_mut().enumerate() {
                                            let reverse_idx = n - 1 - idx;
                                            lane.y_min = 12.0 + reverse_idx as f64 * (height_per_lane + gap);
                                            lane.y_max = lane.y_min + height_per_lane;
                                        }
                                    }
                                });

                                if do_copy || do_cut {
                                    if !selected_lanes.is_empty() {
                                        let mut lanes = Vec::new();
                                        let mut i_to_remove = Vec::new();
                                        for &idx in &selected_lanes {
                                            if idx < config.lanes.len() {
                                                lanes.push(config.lanes[idx].clone());
                                                i_to_remove.push(idx);
                                            }
                                        }
                                        behavior.app.worksheet_clipboard = Some(crate::config::worksheet::WorksheetClipboard::Lanes(lanes));
                                        if do_cut {
                                            i_to_remove.sort_by(|a, b| b.cmp(a));
                                            for idx in i_to_remove {
                                                config.lanes.remove(idx);
                                            }
                                            needs_redistribute = true;
                                        }
                                    } else if !selected_traces.is_empty() {
                                        let mut traces = Vec::new();
                                        let mut traces_to_remove = Vec::new();
                                        for &(i, j) in &selected_traces {
                                            if i < config.lanes.len() && j < config.lanes[i].traces.len() {
                                                traces.push(config.lanes[i].traces[j].clone());
                                                traces_to_remove.push((i, j));
                                            }
                                        }
                                        behavior.app.worksheet_clipboard = Some(crate::config::worksheet::WorksheetClipboard::Traces(traces));
                                        if do_cut {
                                            traces_to_remove.sort_by(|a, b| {
                                                if a.0 != b.0 { b.0.cmp(&a.0) } else { b.1.cmp(&a.1) }
                                            });
                                            for (i, j) in traces_to_remove {
                                                if i < config.lanes.len() && j < config.lanes[i].traces.len() {
                                                    config.lanes[i].traces.remove(j);
                                                }
                                            }
                                        }
                                    }
                                    is_select_mode = false;
                                    selected_lanes.clear();
                                    selected_traces.clear();
                                }

                                ui.data_mut(|d| d.insert_temp(select_mode_id, is_select_mode));
                                ui.data_mut(|d| d.insert_temp(selected_lanes_id, selected_lanes));
                                ui.data_mut(|d| d.insert_temp(selected_traces_id, selected_traces));
                            });
                        break;
                    }
                }
            }
            if !is_open || !window_drawn {
                close_properties = true;
            }
        }
        if close_properties {
            behavior.app.properties_window_open = None;
        }
        
        behavior.app.workbooks[active_wb_idx].worksheets[active_ws_idx].tree = tree;
    }

    pub fn draw_reports_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        if !self.session_loaded || self.sessions.is_empty() {
            self.draw_empty_state_drag_drop(ui, is_dark);
            return;
        }

        let theme = AppTheme::for_mode(is_dark);

        ui.horizontal(|ui| {
            let tab_style = ui.style_mut();
            tab_style.spacing.button_padding = egui::vec2(12.0, 8.0);
            tab_style.visuals.selection.bg_fill = theme.accent;
            tab_style.visuals.selection.stroke = egui::Stroke::new(1.0, theme.on_accent);

            ui.selectable_value(
                &mut self.active_reports_tab,
                crate::ReportsTab::SectorAnalysis,
                "1. Sector Analysis",
            );
            ui.selectable_value(
                &mut self.active_reports_tab,
                crate::ReportsTab::TimingGraphs,
                "2. Timing Graphs",
            );
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
                    ui.label(
                        egui::RichText::new("No sector or lap data available for report.")
                            .color(theme.text_secondary),
                    );
                } else {
                    ui.vertical(|ui| {
                        let loaded = &self.sessions[self.primary_session_idx];
                        let mut visible_laps: Vec<&crate::signals::processing::LapData> = loaded
                            .lap_data_cache
                            .iter()
                            .filter(|lap| lap.lap_num > 3)
                            .collect();
                        if visible_laps.is_empty() {
                            visible_laps = loaded.lap_data_cache.iter().collect();
                        }

                        let best_total_time = visible_laps
                            .iter()
                            .map(|lap| lap.time.last().copied().unwrap_or(0.0))
                            .filter(|&t| t > 0.0)
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(0.0);

                        egui::ScrollArea::both()
                            .id_salt("sector_report_scroll")
                            .max_height(300.0)
                            .show(ui, |ui| {
                                egui::Grid::new("sector_report_grid")
                                    .min_col_width(85.0)
                                    .spacing([20.0, 12.0])
                                    .show(ui, |ui| {
                                        // Header Row
                                        ui.label(
                                            egui::RichText::new("Sector / Corner")
                                                .strong()
                                                .color(theme.text_primary),
                                        );
                                        for lap in &visible_laps {
                                            ui.label(
                                                egui::RichText::new(format!("Lap {}", lap.lap_num))
                                                    .strong()
                                                    .color(theme.text_primary),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new("Optimal")
                                                .strong()
                                                .color(theme.accent_text),
                                        );
                                        ui.end_row();

                                        // Sector split rows
                                        for (s_idx, sector) in loaded.sectors.iter().enumerate() {
                                            ui.label(
                                                egui::RichText::new(&sector.name)
                                                    .strong()
                                                    .color(theme.text_secondary),
                                            );

                                            let best_s_time = loaded
                                                .sector_bests
                                                .get(s_idx)
                                                .copied()
                                                .unwrap_or(0.0);

                                            for lap in &visible_laps {
                                                let t_start = get_lap_time_at_distance(
                                                    &lap.dist,
                                                    &lap.time,
                                                    sector.start_dist,
                                                );
                                                let t_end = get_lap_time_at_distance(
                                                    &lap.dist,
                                                    &lap.time,
                                                    sector.end_dist,
                                                );
                                                let s_time = t_end - t_start;

                                                let is_session_best = s_time > 0.0
                                                    && (s_time - best_s_time).abs() < 1e-4;
                                                let is_near_best =
                                                    s_time > 0.0 && s_time <= best_s_time * 1.015;

                                                report_time_cell(
                                                    ui,
                                                    theme,
                                                    (s_time > 0.0)
                                                        .then(|| format_sector_time(s_time)),
                                                    is_session_best,
                                                    is_near_best,
                                                );
                                            }

                                            report_time_cell(
                                                ui,
                                                theme,
                                                (best_s_time > 0.0)
                                                    .then(|| format_sector_time(best_s_time)),
                                                best_s_time > 0.0,
                                                false,
                                            );
                                            ui.end_row();
                                        }

                                        // Totals Row
                                        ui.label(
                                            egui::RichText::new("TOTAL")
                                                .strong()
                                                .color(theme.accent_text),
                                        );
                                        for lap in &visible_laps {
                                            let total_time =
                                                lap.time.last().copied().unwrap_or(0.0);

                                            let is_total_best = total_time > 0.0
                                                && (total_time - best_total_time).abs() < 1e-4;
                                            let is_total_near_best = total_time > 0.0
                                                && total_time <= best_total_time * 1.015;

                                            report_time_cell(
                                                ui,
                                                theme,
                                                (total_time > 0.0)
                                                    .then(|| format_lap_time(total_time)),
                                                is_total_best,
                                                is_total_near_best,
                                            );
                                        }

                                        let optimal_total = loaded.sector_bests.iter().sum::<f64>();
                                        report_time_cell(
                                            ui,
                                            theme,
                                            (optimal_total > 0.0)
                                                .then(|| format_lap_time(optimal_total)),
                                            optimal_total > 0.0,
                                            false,
                                        );
                                        ui.end_row();
                                    });
                            });

                        ui.add_space(20.0);

                        let venue_name = self.sessions[self.primary_session_idx]
                            .session
                            .venue
                            .to_uppercase();
                        ui.heading(
                            egui::RichText::new(venue_name)
                                .strong()
                                .color(theme.accent_text),
                        );
                        ui.add_space(8.0);
                        egui::Frame::group(ui.style())
                            .fill(theme.surface_card)
                            .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                            .show(ui, |ui| {
                                let map_height = ui.available_height().max(300.0);
                                self.draw_interactive_track_map(
                                    ui,
                                    map_height,
                                    crate::rendering::track_map::TrackMapPlacement::Inline,
                                );
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
