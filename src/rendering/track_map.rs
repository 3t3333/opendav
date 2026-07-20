use egui_plot::{Plot, Line, Text, Points, PlotPoint, PlotPoints};
use crate::OpenDavApp;
use crate::config::worksheet::{ACCENT_COLOR};
use crate::signals::processing::{
    get_lap_segments, get_sector_segments, get_lap_coord_at_distance, get_lap_coord_at_time, get_fastest_lap,
    get_magnified_lap_segments, get_lap_distance_at_time, get_magnified_lap_coord
};

impl OpenDavApp {
    pub fn draw_interactive_track_map(&mut self, ui: &mut egui::Ui, height: f32) {
        if self.sessions.is_empty() || self.sessions[self.primary_session_idx].lap_data_cache.is_empty() {
            ui.label("No track map coordinates precomputed.");
            return;
        }

        let initial_reset_bounds = self.reset_bounds_flag;

        if self.enable_satellite_map && self.sessions[self.primary_session_idx].bg_image_bytes.is_none() {
            let api_key = self.settings.mapbox_api_key.clone();
            self.sessions[self.primary_session_idx].fetch_satellite_maps(&api_key);
        }
        
        if self.sessions[self.primary_session_idx].bg_texture.is_none() {
            if let Some(bytes) = &self.sessions[self.primary_session_idx].bg_image_bytes {
                if let Ok(mut image) = image::load_from_memory(bytes) {
                    let max_dim = 8192;
                    if image.width() > max_dim || image.height() > max_dim {
                        image = image.resize(max_dim, max_dim, image::imageops::FilterType::Triangle);
                    }
                    let size = [image.width() as _, image.height() as _];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.to_rgba8().as_flat_samples().as_slice());
                    let texture = ui.ctx().load_texture("mapbox_bg_track_map", color_image, egui::TextureOptions::default());
                    self.sessions[self.primary_session_idx].bg_texture = Some(texture);
                }
            }
        }
        
        if self.sessions[self.primary_session_idx].fg_texture.is_none() {
            if let Some(bytes) = &self.sessions[self.primary_session_idx].fg_image_bytes {
                if let Ok(mut image) = image::load_from_memory(bytes) {
                    let max_dim = 8192;
                    if image.width() > max_dim || image.height() > max_dim {
                        image = image.resize(max_dim, max_dim, image::imageops::FilterType::Triangle);
                    }
                    let size = [image.width() as _, image.height() as _];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.to_rgba8().as_flat_samples().as_slice());
                    let texture = ui.ctx().load_texture("mapbox_fg_track_map", color_image, egui::TextureOptions::default());
                    self.sessions[self.primary_session_idx].fg_texture = Some(texture);
                }
            }
        }
        
        let loaded = &self.sessions[self.primary_session_idx];

        // Draw checkbox bar at the top of the track map card
        ui.horizontal_wrapped(|ui| {
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
            ui.add_space(15.0);
            if ui.checkbox(&mut self.enable_satellite_map, "Satellite Map (Beta)").changed() {
                // Fetch is triggered automatically at the top of this function
            }
            ui.add_space(15.0);
            ui.checkbox(&mut self.auto_follow_track_map, "Auto-Follow Car");
            ui.add_space(15.0);
            ui.checkbox(&mut self.auto_rotate_track_map, "Auto-Rotate");
            if ref_active {
                ui.add_space(15.0);
                ui.checkbox(&mut self.magnify_line_deltas, "Magnifier");
                if self.magnify_line_deltas {
                    ui.add_space(5.0);
                    ui.add(egui::Slider::new(&mut self.magnifier_multiplier, 1.0..=20.0).text("x").show_value(true));
                }
            }
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
                
                if reset_bounds_flag {
                    // Compute raw unrotated bounds to find aspect ratio of track
                    let mut raw_min_x = f64::MAX;
                    let mut raw_max_x = f64::MIN;
                    let mut raw_min_y = f64::MAX;
                    let mut raw_max_y = f64::MIN;
                    for i in 0..active_lap.x.len() {
                        let px = active_lap.x[i];
                        let py = active_lap.y[i];
                        if px.is_nan() || py.is_nan() { continue; }
                        raw_min_x = raw_min_x.min(px);
                        raw_max_x = raw_max_x.max(px);
                        raw_min_y = raw_min_y.min(py);
                        raw_max_y = raw_max_y.max(py);
                    }
                    let data_w = raw_max_x - raw_min_x;
                    let data_h = raw_max_y - raw_min_y;
                    
                    let phys_w = plot_width.max(100.0) as f64;
                    let phys_h = height as f64;
                    
                    let data_aspect = data_w / data_h.max(0.0001);
                    let phys_aspect = phys_w / phys_h.max(0.0001);
                    
                    if (phys_aspect > 1.0 && data_aspect < 1.0) || (phys_aspect < 1.0 && data_aspect > 1.0) {
                        self.track_map_rotation = std::f64::consts::PI / 2.0; // Rotate 90 degrees
                    } else {
                        self.track_map_rotation = 0.0;
                    }
                    self.auto_rotate_track_map = false;
                }

                let mut lap_rel_time = 0.0;
                if let Some(cx) = self.cursor_x {
                    if let Some(pos) = loaded.lap_ranges.iter().position(|r| r.0 == active_lap_num) {
                        let (_, start_t, end_t) = loaded.lap_ranges[pos];
                        if cx >= start_t && cx <= end_t {
                            lap_rel_time = cx - start_t;
                        } else if cx > end_t {
                            lap_rel_time = end_t - start_t;
                        }
                    }
                    
                    if self.auto_rotate_track_map {
                        let (cx_x1, cx_y1) = get_lap_coord_at_time(active_lap, lap_rel_time);
                        let (cx_x2, cx_y2) = get_lap_coord_at_time(active_lap, lap_rel_time + 0.1);
                        let dx = cx_x2 - cx_x1;
                        let dy = cx_y2 - cx_y1;
                        if dx.abs() > 1e-4 || dy.abs() > 1e-4 {
                            let heading = dy.atan2(dx);
                            self.track_map_rotation = std::f64::consts::PI / 2.0 - heading;
                        }
                    }
                }

                let rot = self.track_map_rotation;
                let cos_a = rot.cos();
                let sin_a = rot.sin();
                let rotate_point = |x: f64, y: f64| -> [f64; 2] {
                    [x * cos_a - y * sin_a, x * sin_a + y * cos_a]
                };
                let rotate_segments = |segs: Vec<Vec<[f64; 2]>>| -> Vec<Vec<[f64; 2]>> {
                    segs.into_iter().map(|line| line.into_iter().map(|p| rotate_point(p[0], p[1])).collect()).collect()
                };

                let mut min_x = f64::MAX;
                let mut max_x = f64::MIN;
                let mut min_y = f64::MAX;
                let mut max_y = f64::MIN;
                for i in 0..active_lap.x.len() {
                    let px = active_lap.x[i];
                    let py = active_lap.y[i];
                    if px.is_nan() || py.is_nan() { continue; }
                    let p = rotate_point(px, py);
                    min_x = min_x.min(p[0]);
                    max_x = max_x.max(p[0]);
                    min_y = min_y.min(p[1]);
                    max_y = max_y.max(p[1]);
                }
                
                // Initialize the egui_plot
                let plot = Plot::new("interactive_track_map_plot")
                    .height(height)
                    .show_axes(false)
                    .show_grid(false)
                    .allow_zoom(true)
                    .allow_drag(true)
                    .data_aspect(1.0)
                    .allow_double_click_reset(false)
                    .auto_bounds(egui::Vec2b::new(false, false));

                let plot_resp = plot.show(ui, |plot_ui| {
                    if initial_reset_bounds {
                        if min_x < max_x && min_y < max_y {
                            // Give the exact un-padded bounding box of the track trace to egui_plot.
                            // Because we use `.data_aspect(1.0)`, egui_plot will automatically 
                            // expand the bounds to seamlessly fit the physical aspect ratio of the window,
                            // without clipping any of the track.
                            
                            let center_x = (min_x + max_x) / 2.0;
                            let center_y = (min_y + max_y) / 2.0;
                            
                            let data_w = max_x - min_x;
                            let data_h = max_y - min_y;
                            
                            let phys_w = plot_width.max(100.0) as f64;
                            let phys_h = height as f64;
                            
                            let mut target_w = data_w;
                            let mut target_h = data_h;
                            
                            if data_w * phys_h > data_h * phys_w {
                                target_h = data_w * phys_h / phys_w;
                            } else {
                                target_w = data_h * phys_w / phys_h;
                            }
                            
                            target_w *= 1.05;
                            target_h *= 1.05;
                            
                            plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                                [center_x - target_w / 2.0, center_y - target_h / 2.0],
                                [center_x + target_w / 2.0, center_y + target_h / 2.0],
                            ));
                        }
                    }
                    if self.enable_satellite_map {
                        // 0. Draw Mapbox Track Map (BG Layer)
                        if let (Some(texture), Some(bounds)) = (&loaded.bg_texture, &loaded.bg_bounds) {
                            let (min_x, min_y) = crate::signals::mapbox::wgs84_to_web_mercator(bounds[0], bounds[1]);
                            let (max_x, max_y) = crate::signals::mapbox::wgs84_to_web_mercator(bounds[2], bounds[3]);
                            
                            let mut center_x = (min_x + max_x) / 2.0;
                            let mut center_y = (min_y + max_y) / 2.0;
                            if let Some(origin) = loaded.map_origin {
                                center_x -= origin[0];
                                center_y -= origin[1];
                            }
                            
                            let size_x = max_x - min_x;
                            let size_y = max_y - min_y;
                            let rot_center = rotate_point(center_x, center_y);
                            
                            plot_ui.image(
                                egui_plot::PlotImage::new(
                                    "mapbox_bg_layer",
                                    texture,
                                    egui_plot::PlotPoint::new(rot_center[0], rot_center[1]),
                                    egui::vec2(size_x as f32, size_y as f32),
                                ).rotate(self.track_map_rotation)
                            );
                        }
                        
                        // 0.5. Draw Mapbox Track Map (FG Layer)
                        if let (Some(texture), Some(bounds)) = (&loaded.fg_texture, &loaded.fg_bounds) {
                            let (min_x, min_y) = crate::signals::mapbox::wgs84_to_web_mercator(bounds[0], bounds[1]);
                            let (max_x, max_y) = crate::signals::mapbox::wgs84_to_web_mercator(bounds[2], bounds[3]);
                            
                            let mut center_x = (min_x + max_x) / 2.0;
                            let mut center_y = (min_y + max_y) / 2.0;
                            
                            if let Some(origin) = loaded.map_origin {
                                center_x -= origin[0];
                                center_y -= origin[1];
                            }
                            
                            let size_x = max_x - min_x;
                            let size_y = max_y - min_y;
                            
                            let rot_center = rotate_point(center_x, center_y);
                            
                            plot_ui.image(
                                egui_plot::PlotImage::new(
                                    "mapbox_fg_layer",
                                    texture,
                                    egui_plot::PlotPoint::new(rot_center[0], rot_center[1]),
                                    egui::vec2(size_x as f32, size_y as f32), // Standard Web Mercator Y-up mapping
                                ).rotate(self.track_map_rotation)
                            );
                        }
                    }

                    // 1. Draw Reference Laps (underneath)
                    if let Some(lap) = ref_cyan_lap {
                        let color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                        let segments = if self.magnify_line_deltas {
                            rotate_segments(get_magnified_lap_segments(lap, active_lap, self.magnifier_multiplier))
                        } else {
                            rotate_segments(get_lap_segments(lap))
                        };
                        for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                            plot_ui.line(Line::new(format!("Ref Lap {} (Cyan) - Seg {}", self.ref_lap_cyan.unwrap().1, seg_idx), PlotPoints::from(seg_pts))
                                .color(color)
                                .width(2.0)
                            );
                        }
                    }

                    if let Some(lap) = ref_white_lap {
                        let color = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 100) };
                        let segments = if self.magnify_line_deltas {
                            rotate_segments(get_magnified_lap_segments(lap, active_lap, self.magnifier_multiplier))
                        } else {
                            rotate_segments(get_lap_segments(lap))
                        };
                        for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                            plot_ui.line(Line::new(format!("Ref Lap {} (White) - Seg {}", self.ref_lap_white.unwrap().1, seg_idx), PlotPoints::from(seg_pts))
                                .color(color)
                                .width(2.0)
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

                            let sector_segments = rotate_segments(get_sector_segments(active_lap, sector.start_dist, sector.end_dist));
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
                        let active_segments = rotate_segments(get_lap_segments(active_lap));
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
                            let p0 = rotate_point(x0 - nx * sf_width, y0 - ny * sf_width);
                            let p1 = rotate_point(x0 + nx * sf_width, y0 + ny * sf_width);
                            let sf_pts = vec![p0, p1];
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
                        let p_label = rotate_point(label_x, label_y);

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
                            PlotPoint::new(p_label[0], p_label[1]),
                            egui::RichText::new(box_text)
                                .color(text_color)
                                .background_color(bg_color)
                                .strong()
                                .size(dynamic_font_size)
                        ));
                    }

                    // 5. Draw Live Car Playback Position Dot (locked to cursor_x)
                    if let Some(cx) = self.cursor_x {
                        let (cx_x, cx_y) = get_lap_coord_at_time(active_lap, lap_rel_time);
                        let p_car = rotate_point(cx_x, cx_y);
                        
                        if let Some(w_lap) = ref_white_lap {
                            let (wx, wy) = if self.magnify_line_deltas {
                                let ref_dist = get_lap_distance_at_time(w_lap, lap_rel_time);
                                get_magnified_lap_coord(w_lap, active_lap, ref_dist, self.magnifier_multiplier)
                            } else {
                                get_lap_coord_at_time(w_lap, lap_rel_time)
                            };
                            let pw = rotate_point(wx, wy);
                            
                            // Rubber band
                            plot_ui.line(Line::new("White Rubber Band", vec![p_car, pw])
                                .color(egui::Color32::from_white_alpha(100))
                                .style(egui_plot::LineStyle::Dashed { length: 4.0 })
                                .width(1.0)
                            );
                            
                            plot_ui.points(Points::new("White Ref Position", vec![pw])
                                .color(egui::Color32::WHITE)
                                .radius(8.0)
                            );
                        }
                        
                        if let Some(c_lap) = ref_cyan_lap {
                            let (cx_coord, cy_coord) = if self.magnify_line_deltas {
                                let ref_dist = get_lap_distance_at_time(c_lap, lap_rel_time);
                                get_magnified_lap_coord(c_lap, active_lap, ref_dist, self.magnifier_multiplier)
                            } else {
                                get_lap_coord_at_time(c_lap, lap_rel_time)
                            };
                            let pc = rotate_point(cx_coord, cy_coord);
                            
                            // Rubber band
                            plot_ui.line(Line::new("Cyan Rubber Band", vec![p_car, pc])
                                .color(egui::Color32::from_rgba_premultiplied(0, 150, 150, 100)) // Faint cyan
                                .style(egui_plot::LineStyle::Dashed { length: 4.0 })
                                .width(1.0)
                            );
                            
                            plot_ui.points(Points::new("Cyan Ref Position", vec![pc])
                                .color(egui::Color32::CYAN)
                                .radius(8.0)
                            );
                        }

                        plot_ui.points(Points::new("Current Position", vec![p_car])
                            .color(ACCENT_COLOR)
                            .radius(8.0)
                        );

                        if self.auto_follow_track_map && !reset_bounds_flag {
                            let bounds = plot_ui.plot_bounds();
                            let w = bounds.max()[0] - bounds.min()[0];
                            let h = bounds.max()[1] - bounds.min()[1];
                            plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                                [p_car[0] - w / 2.0, p_car[1] - h / 2.0],
                                [p_car[0] + w / 2.0, p_car[1] + h / 2.0],
                            ));
                        }
                    }
                });

                if plot_resp.response.dragged() {
                    self.auto_follow_track_map = false;
                }

                if plot_resp.response.double_clicked() {
                    self.reset_bounds_flag = true;
                    self.reset_bounds_next_frame = 3;
                    self.auto_follow_track_map = false;
                }

            });
        });

        if initial_reset_bounds {
            if self.reset_bounds_next_frame > 0 {
                self.reset_bounds_next_frame -= 1;
                ui.ctx().request_repaint();
            } else {
                self.reset_bounds_flag = false;
            }
        }
    }
}
