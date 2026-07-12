use egui_plot::{Plot, Line, Text, Points, PlotPoint, PlotPoints};
use crate::OpenDavApp;
use crate::config::worksheet::{ACCENT_COLOR};
use crate::signals::processing::{
    get_lap_segments, get_sector_segments, get_lap_coord_at_distance, get_lap_coord_at_time, get_fastest_lap
};

impl OpenDavApp {
    pub fn draw_interactive_track_map(&mut self, ui: &mut egui::Ui, height: f32) {
        if self.sessions.is_empty() || self.sessions[self.primary_session_idx].lap_data_cache.is_empty() {
            ui.label("No track map coordinates precomputed.");
            return;
        }
        
        let loaded = &self.sessions[self.primary_session_idx];

        // Draw checkbox bar at the top of the track map card
        ui.horizontal(|ui| {
            let ref_active = self.ref_lap_cyan.or(self.ref_lap_white).is_some();
            if ref_active {
                ui.checkbox(&mut self.show_sector_deltas, egui::RichText::new("Sector Delta Overlays").strong());
                ui.add_space(15.0);
                ui.checkbox(&mut self.show_chart_deltas, egui::RichText::new("Time Series Charts Deltas").strong());
            } else {
                ui.add_enabled_ui(false, |ui| {
                    let mut dummy = false;
                    ui.checkbox(&mut dummy, egui::RichText::new("Sector Delta Overlays (Select Reference Lap in Graphs)").small());
                    ui.add_space(15.0);
                    ui.checkbox(&mut dummy, egui::RichText::new("Time Series Charts Deltas").small());
                });
            }
            ui.add_space(15.0);
            ui.checkbox(&mut self.show_all_splits, "Toggle All Splits");
        });

        let is_dark = ui.style().visuals.dark_mode;
        let active_lap_num = self.selected_lap.map(|(_, lap)| lap).unwrap_or_else(|| {
            get_fastest_lap(&loaded.session.lap_times)
        });

        // Find the active lap data
        let active_lap = loaded.lap_data_cache.iter().find(|l| l.lap_num == active_lap_num);
        if active_lap.is_none() {
            ui.label("Active lap data not found in cache.");
            return;
        }
        let active_lap = active_lap.unwrap();

        // Let's get the reference overlay laps if selected
        let ref_cyan_lap = self.ref_lap_cyan.and_then(|(s_idx, num)| self.sessions[s_idx].lap_data_cache.iter().find(|l| l.lap_num == num));
        let ref_white_lap = self.ref_lap_white.and_then(|(s_idx, num)| self.sessions[s_idx].lap_data_cache.iter().find(|l| l.lap_num == num));
        
        let ref_active = self.ref_lap_cyan.or(self.ref_lap_white).is_some();
        let show_deltas = self.show_sector_deltas && ref_active;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            let plot_width = ui.available_width() - 200.0;
            ui.allocate_ui(egui::vec2(190.0, height), |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Legend");
                        ui.add_space(8.0);
                        
                        if let Some(lap) = ref_cyan_lap {
                            ui.horizontal(|ui| {
                                let (r,g,b) = if is_dark { (0, 255, 255) } else { (0, 120, 136) };
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(r,g,b));
                                ui.label(format!("Ref Lap {} (Cyan)", lap.lap_num));
                            });
                        }
                        if let Some(lap) = ref_white_lap {
                            ui.horizontal(|ui| {
                                let color = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 100) };
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, color);
                                ui.label(format!("Ref Lap {} (White)", lap.lap_num));
                            });
                        }
                        
                        ui.horizontal(|ui| {
                            let (rect, _response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, egui::Color32::RED);
                            ui.label("Start/Finish Line");
                        });
                        
                        ui.horizontal(|ui| {
                            let (rect, _response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, ACCENT_COLOR);
                            ui.label("Current Position");
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        
                        ui.label(egui::RichText::new("Track Splits").strong());
                        ui.add_space(4.0);
                        
                        for (s_idx, sector) in loaded.sectors.iter().enumerate() {
                            let mut is_visible = !self.hidden_splits.contains(&sector.name);
                            ui.horizontal(|ui| {
                                // Draw color swatch
                                let swatch_color = if show_deltas {
                                    let delta = self.sector_deltas.get(s_idx).copied().flatten();
                                    if let Some(d) = delta {
                                        if d <= 0.0 {
                                            if is_dark { egui::Color32::from_rgb(50, 205, 50) } else { egui::Color32::from_rgb(34, 139, 34) }
                                        } else {
                                            if is_dark { egui::Color32::from_rgb(255, 69, 0) } else { egui::Color32::from_rgb(200, 40, 0) }
                                        }
                                    } else {
                                        if is_dark { egui::Color32::from_rgb(150, 150, 150) } else { egui::Color32::from_rgb(100, 100, 100) }
                                    }
                                } else {
                                    ACCENT_COLOR
                                };
                                
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, swatch_color);
                                
                                if ui.checkbox(&mut is_visible, &sector.name).changed() {
                                    if is_visible {
                                        self.hidden_splits.remove(&sector.name);
                                    } else {
                                        self.hidden_splits.insert(sector.name.clone());
                                    }
                                }
                            });
                        }
                    });
                });
            });
                });

            // Plot taking the remaining space on the left
            ui.allocate_ui(egui::vec2(plot_width.max(100.0), height), |ui| {
                let reset_bounds_flag = self.reset_bounds_flag;

                // Initialize the egui_plot
                let plot = Plot::new("interactive_track_map_plot")
                    .height(height)
                    .show_axes(false)
                    .show_grid(false)
                    .allow_zoom(true)
                    .allow_drag(true)
                    .auto_bounds(egui::Vec2b::new(false, false));

                plot.show(ui, |plot_ui| {
                    if reset_bounds_flag {
                        let mut min_x = f64::MAX;
                        let mut max_x = f64::MIN;
                        let mut min_y = f64::MAX;
                        let mut max_y = f64::MIN;
                        for &x in &active_lap.x {
                            min_x = min_x.min(x);
                            max_x = max_x.max(x);
                        }
                        for &y in &active_lap.y {
                            min_y = min_y.min(y);
                            max_y = max_y.max(y);
                        }
                        
                        if min_x < max_x && min_y < max_y {
                            // Calculate data dimensions
                            let data_w = max_x - min_x;
                            let data_h = max_y - min_y;
                            
                            // Calculate physical dimensions
                            let phys_w = plot_width.max(100.0) as f64;
                            let phys_h = height as f64;
                            
                            // To maintain a 1:1 aspect ratio manually, we pad the bounds so that 
                            // data_w / data_h equals phys_w / phys_h
                            let mut target_w = data_w;
                            let mut target_h = data_h;
                            
                            if data_w * phys_h > data_h * phys_w {
                                // Data is wider relative to physical, so we pad data height
                                target_h = data_w * phys_h / phys_w;
                            } else {
                                // Data is taller relative to physical, so we pad data width
                                target_w = data_h * phys_w / phys_h;
                            }
                            
                            // Add an extra 5% margin
                            target_w *= 1.05;
                            target_h *= 1.05;
                            
                            let center_x = (min_x + max_x) / 2.0;
                            let center_y = (min_y + max_y) / 2.0;
                            
                            plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                                [center_x - target_w / 2.0, center_y - target_h / 2.0],
                                [center_x + target_w / 2.0, center_y + target_h / 2.0],
                            ));
                        }
                    }

                    // 1. Draw Reference Laps (underneath)
                    if let Some(lap) = ref_cyan_lap {
                        let color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                        let segments = get_lap_segments(lap);
                        for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                            plot_ui.line(Line::new(format!("Ref Lap {} (Cyan) - Seg {}", self.ref_lap_cyan.unwrap().1, seg_idx), PlotPoints::from(seg_pts))
                                .color(color)
                                .width(1.2)
                            );
                        }
                    }

                    if let Some(lap) = ref_white_lap {
                        let color = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 100) };
                        let segments = get_lap_segments(lap);
                        for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                            plot_ui.line(Line::new(format!("Ref Lap {} (White) - Seg {}", self.ref_lap_white.unwrap().1, seg_idx), PlotPoints::from(seg_pts))
                                .color(color)
                                .width(1.2)
                            );
                        }
                    }

                    // 2. Draw Active Lap (color-coded by sector if show_deltas is true)
                    if show_deltas {
                        for (s_idx, sector) in loaded.sectors.iter().enumerate() {
                            let delta = self.sector_deltas.get(s_idx).copied().flatten();
                            let seg_color = if let Some(d) = delta {
                                if d <= 0.0 {
                                    if is_dark { egui::Color32::from_rgb(50, 205, 50) } else { egui::Color32::from_rgb(34, 139, 34) } // LimeGreen vs ForestGreen
                                } else {
                                    if is_dark { egui::Color32::from_rgb(255, 69, 0) } else { egui::Color32::from_rgb(200, 40, 0) } // OrangeRed vs DarkRed
                                }
                            } else {
                                if is_dark { egui::Color32::from_rgb(150, 150, 150) } else { egui::Color32::from_rgb(100, 100, 100) }
                            };

                            let sector_segments = get_sector_segments(active_lap, sector.start_dist, sector.end_dist);
                            for seg_pts in sector_segments.into_iter() {
                                // Empty name because we render the labels manually below, AND we have the side panel legend!
                                plot_ui.line(Line::new("", PlotPoints::from(seg_pts))
                                    .color(seg_color)
                                    .width(2.0)
                                );
                            }
                        }
                    } else {
                        let active_color = ACCENT_COLOR;
                        let active_segments = get_lap_segments(active_lap);
                        for seg_pts in active_segments.into_iter() {
                            plot_ui.line(Line::new("", PlotPoints::from(seg_pts))
                                .color(active_color)
                                .width(2.0)
                            );
                        }
                    }

                    // 3. Draw Start/Finish Line (perpendicular red tick at first coordinate)
                    if active_lap.x.len() > 1 {
                        let x0 = active_lap.x[0];
                        let y0 = active_lap.y[0];
                        let x1 = active_lap.x[1];
                        let y1 = active_lap.y[1];
                        
                        // Direction vector of the track at start/finish
                        let dx = x1 - x0;
                        let dy = y1 - y0;
                        let len = (dx*dx + dy*dy).sqrt();
                        if len > 0.0 {
                            // Normal vector (perpendicular to direction)
                            let nx = -dy / len;
                            let ny = dx / len;
                            
                            // Draw a red line segment of length 16 meters centered on the S/F point
                            let sf_width = 8.0;
                            let sf_pts = vec![
                                [x0 - nx * sf_width, y0 - ny * sf_width],
                                [x0 + nx * sf_width, y0 + ny * sf_width],
                            ];
                            plot_ui.line(Line::new("", sf_pts)
                                .color(egui::Color32::RED)
                                .width(3.5)
                            );
                        }
                    }

                    // 4. Draw Sector Labels and Sector Times at sector midpoints
                    for (s_idx, sector) in loaded.sectors.iter().enumerate() {
                        if !self.show_all_splits || self.hidden_splits.contains(&sector.name) {
                            continue;
                        }

                        let mid_dist = (sector.start_dist + sector.end_dist) / 2.0;
                        let (tx, ty) = get_lap_coord_at_distance(active_lap, mid_dist);
                        
                        // Find normal vector at this midpoint to offset the label slightly outwards
                        let mid_idx = match active_lap.dist.binary_search_by(|val| val.partial_cmp(&mid_dist).unwrap_or(std::cmp::Ordering::Equal)) {
                            Ok(i) => i,
                            Err(i) => i.clamp(0, active_lap.dist.len() - 1),
                        };
                        
                        let mut nx = 0.0;
                        let mut ny = 0.0;
                        if mid_idx > 0 && mid_idx < active_lap.x.len() - 1 {
                            let dx = active_lap.x[mid_idx + 1] - active_lap.x[mid_idx - 1];
                            let dy = active_lap.y[mid_idx + 1] - active_lap.y[mid_idx - 1];
                            let len = (dx*dx + dy*dy).sqrt();
                            if len > 0.0 {
                                nx = -dy / len;
                                ny = dx / len;
                            }
                        }
                        
                        // Shortened name (T1, Str 1-2)
                        let mut short_name = sector.name.clone();
                        if short_name.starts_with("Turn ") {
                            short_name = short_name.replace("Turn ", "T");
                        } else if short_name.starts_with("Straight ") {
                            short_name = short_name.replace("Straight ", "Str ");
                        }
                        
                        // Offset the text slightly by 18 meters along normal to fit the box
                        let offset_dist = 18.0;
                        let label_x = tx + nx * offset_dist;
                        let label_y = ty + ny * offset_dist;

                        // Dynamic text scaling depending on zoom (bounds relative to coordinate space)
                        let b = plot_ui.plot_bounds();
                        let view_width = b.max()[0] - b.min()[0];
                        let dynamic_font_size = (3750.0 / view_width).clamp(10.0, 20.0) as f32;
                        
                        let mut text_color = if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK };
                        let mut bg_color = if is_dark { egui::Color32::from_black_alpha(180) } else { egui::Color32::from_white_alpha(180) };
                        let box_text;
                        
                        if ref_active {
                            if let Some(d) = self.sector_deltas.get(s_idx).copied().flatten() {
                                if d <= 0.0 {
                                    text_color = egui::Color32::WHITE;
                                    bg_color = if is_dark { egui::Color32::from_rgb(34, 139, 34) } else { egui::Color32::from_rgb(0, 120, 0) };
                                    box_text = format!(" {} | -{:.3}s ", short_name, d.abs());
                                } else {
                                    text_color = egui::Color32::WHITE;
                                    bg_color = if is_dark { egui::Color32::from_rgb(200, 40, 0) } else { egui::Color32::from_rgb(180, 0, 0) };
                                    box_text = format!(" {} | +{:.3}s ", short_name, d);
                                }
                            } else {
                                box_text = format!(" {} | -- ", short_name);
                            }
                        } else {
                            // Raw sector time
                            let act_start = crate::signals::processing::get_lap_time_at_distance(&active_lap.dist, &active_lap.time, sector.start_dist);
                            let act_end = crate::signals::processing::get_lap_time_at_distance(&active_lap.dist, &active_lap.time, sector.end_dist);
                            box_text = format!(" {} | {:.3}s ", short_name, act_end - act_start);
                        }

                        plot_ui.text(Text::new(
                            &sector.name,
                            PlotPoint::new(label_x, label_y),
                            egui::RichText::new(box_text)
                                .color(text_color)
                                .background_color(bg_color)
                                .strong()
                                .size(dynamic_font_size)
                        ));
                    }

                    // 5. Draw Live Car Playback Position Dot (locked to cursor_x)
                    if let Some(cx) = self.cursor_x {
                        let mut lap_rel_time = 0.0;
                        if let Some(pos) = loaded.lap_ranges.iter().position(|r| r.0 == active_lap_num) {
                            let (_, start_t, end_t) = loaded.lap_ranges[pos];
                            if cx >= start_t && cx <= end_t {
                                lap_rel_time = cx - start_t;
                            } else if cx > end_t {
                                lap_rel_time = end_t - start_t;
                            }
                        }

                        let (cx_x, cx_y) = get_lap_coord_at_time(active_lap, lap_rel_time);
                        plot_ui.points(Points::new("Current Position", vec![[cx_x, cx_y]])
                            .color(ACCENT_COLOR)
                            .radius(8.0)
                        );
                    }
                });
            });
        });

        if self.reset_bounds_flag {
            self.reset_bounds_flag = false;
        }
    }
}
