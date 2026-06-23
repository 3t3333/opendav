use egui_plot::{Plot, Line, Text, Points, PlotPoint, PlotPoints};
use crate::OpenDavApp;
use crate::config::worksheet::{ACCENT_COLOR};
use crate::signals::processing::{
    get_lap_segments, get_sector_segments, get_lap_coord_at_distance, get_lap_coord_at_time, get_fastest_lap
};

impl OpenDavApp {
    pub fn draw_interactive_track_map(&mut self, ui: &mut egui::Ui, height: f32) {
        if self.lap_data_cache.is_empty() {
            ui.label("No track map coordinates precomputed.");
            return;
        }

        // Draw checkbox bar at the top of the track map card
        ui.horizontal(|ui| {
            let ref_active = self.ref_lap_cyan.or(self.ref_lap_white).is_some();
            if ref_active {
                ui.checkbox(&mut self.show_sector_deltas, egui::RichText::new("Sector Delta Overlays").strong());
            } else {
                ui.add_enabled_ui(false, |ui| {
                    let mut dummy = false;
                    ui.checkbox(&mut dummy, egui::RichText::new("Sector Delta Overlays (Select Reference Lap in Graphs)").small());
                });
            }
        });

        let is_dark = ui.style().visuals.dark_mode;
        let active_lap_num = self.selected_lap.unwrap_or_else(|| {
            if let Some(session) = &self.session {
                get_fastest_lap(&session.lap_times)
            } else {
                0
            }
        });

        // Find the active lap data
        let active_lap = self.lap_data_cache.iter().find(|l| l.lap_num == active_lap_num);
        if active_lap.is_none() {
            ui.label("Active lap data not found in cache.");
            return;
        }
        let active_lap = active_lap.unwrap();

        // Let's get the reference overlay laps if selected
        let ref_cyan_lap = self.ref_lap_cyan.and_then(|num| self.lap_data_cache.iter().find(|l| l.lap_num == num));
        let ref_white_lap = self.ref_lap_white.and_then(|num| self.lap_data_cache.iter().find(|l| l.lap_num == num));

        // Initialize the egui_plot
        let plot = Plot::new("interactive_track_map_plot")
            .data_aspect(1.0) // Lock aspect ratio 1:1 to prevent coordinate stretching!
            .height(height)
            .show_axes(false)
            .show_grid(false)
            .legend(egui_plot::Legend::default())
            .allow_zoom(true)
            .allow_drag(true);

        let ref_active = self.ref_lap_cyan.or(self.ref_lap_white).is_some();
        let show_deltas = self.show_sector_deltas && ref_active;

        plot.show(ui, |plot_ui| {
            // --- Dynamic Zoom Font Scaling ---
            let bounds = plot_ui.plot_bounds();
            let view_width = bounds.width() as f32;
            let clamped_width = view_width.clamp(100.0, 800.0);
            let scale_factor = 1.0 - ((clamped_width - 100.0) / 700.0); // 1.0 at 100m view, 0.0 at 800m view
            let dynamic_font_size = 12.0 + (scale_factor * 16.0); // scales smoothly from 12.0pt to 28.0pt
            // ---------------------------------

            // 1. Draw Reference Laps (underneath)
            if let Some(lap) = ref_cyan_lap {
                let color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                let segments = get_lap_segments(lap);
                for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                    plot_ui.line(Line::new(format!("Ref Lap {} (Cyan) - Seg {}", self.ref_lap_cyan.unwrap(), seg_idx), PlotPoints::from(seg_pts))
                        .color(color)
                        .width(1.2)
                    );
                }
            }

            if let Some(lap) = ref_white_lap {
                let color = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 100) };
                let segments = get_lap_segments(lap);
                for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                    plot_ui.line(Line::new(format!("Ref Lap {} (White) - Seg {}", self.ref_lap_white.unwrap(), seg_idx), PlotPoints::from(seg_pts))
                        .color(color)
                        .width(1.2)
                    );
                }
            }

            // 2. Draw Active Lap (color-coded by sector if show_deltas is true)
            if show_deltas {
                for (s_idx, sector) in self.sectors.iter().enumerate() {
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
                    for (seg_idx, seg_pts) in sector_segments.into_iter().enumerate() {
                        plot_ui.line(Line::new(format!("Sector {} - Seg {}", s_idx + 1, seg_idx), PlotPoints::from(seg_pts))
                            .color(seg_color)
                            .width(2.0)
                        );
                    }

                    // Removed separate delta label rendering - integrated into Turn labels below
                }
            } else {
                let active_color = ACCENT_COLOR;
                let active_segments = get_lap_segments(active_lap);
                for (seg_idx, seg_pts) in active_segments.into_iter().enumerate() {
                    plot_ui.line(Line::new(format!("Lap {} - Seg {}", active_lap_num, seg_idx), PlotPoints::from(seg_pts))
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
                    plot_ui.line(Line::new("Start/Finish Line", sf_pts)
                        .color(egui::Color32::RED)
                        .width(3.5)
                    );
                }
            }

            // 4. Draw Turn Labels and Sector Times at corner midpoints
            for (s_idx, sector) in self.sectors.iter().enumerate() {
                if sector.name.starts_with("Turn") {
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
                    
                    // Extract turn number string from name (e.g. "Turn 3" -> "3")
                    let turn_num = sector.name.replace("Turn ", "");
                    
                    // Offset the text slightly by 18 meters along normal to fit the box
                    let offset_dist = 18.0;
                    let label_x = tx + nx * offset_dist;
                    let label_y = ty + ny * offset_dist;

                    let mut text_color = if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK };
                    let mut bg_color = if is_dark { egui::Color32::from_black_alpha(180) } else { egui::Color32::from_white_alpha(180) };
                    let box_text;
                    
                    if ref_active {
                        if let Some(d) = self.sector_deltas.get(s_idx).copied().flatten() {
                            if d <= 0.0 {
                                text_color = egui::Color32::WHITE;
                                bg_color = if is_dark { egui::Color32::from_rgb(34, 139, 34) } else { egui::Color32::from_rgb(0, 120, 0) };
                                box_text = format!(" T{} | -{:.3}s ", turn_num, d.abs());
                            } else {
                                text_color = egui::Color32::WHITE;
                                bg_color = if is_dark { egui::Color32::from_rgb(200, 40, 0) } else { egui::Color32::from_rgb(180, 0, 0) };
                                box_text = format!(" T{} | +{:.3}s ", turn_num, d);
                            }
                        } else {
                            box_text = format!(" T{} | -- ", turn_num);
                        }
                    } else {
                        // Raw sector time
                        let act_start = crate::signals::processing::get_lap_time_at_distance(&active_lap.dist, &active_lap.time, sector.start_dist);
                        let act_end = crate::signals::processing::get_lap_time_at_distance(&active_lap.dist, &active_lap.time, sector.end_dist);
                        box_text = format!(" T{} | {:.3}s ", turn_num, act_end - act_start);
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
            }

            // 5. Draw Live Car Playback Position Dot (locked to cursor_x)
            if let Some(cx) = self.cursor_x {
                let mut lap_rel_time = 0.0;
                if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == active_lap_num) {
                    let (_, start_t, end_t) = self.lap_ranges[pos];
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
    }
}
