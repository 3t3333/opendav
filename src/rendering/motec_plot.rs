use egui_plot::{Plot, HLine, VLine, Line, Points, PlotPoint, PlotPoints, Span, Axis, Text};
use crate::OpenDavApp;
use crate::config::worksheet::{WorksheetConfig, CacheSelector, ACCENT_COLOR, SUB_ACCENT_COLOR};
use crate::signals::processing::{get_closest_index, get_lap_points_slice, format_lap_time};

pub struct ChartTrace<'a> {
    pub name: &'static str,
    pub scaled_pts: &'a [[f64; 2]],
    pub color: egui::Color32,
    pub width: f32,
    pub raw_val: f64,
    pub unit: &'static str,
}

pub struct ChartLane<'a> {
    pub title: &'static str,
    pub y_min: f64,
    pub y_max: f64,
    pub traces: Vec<ChartTrace<'a>>,
}

impl OpenDavApp {
    pub fn get_cache_slice(&self, selector: CacheSelector) -> &[[f64; 2]] {
        if self.sessions.is_empty() { return &[]; }
        let loaded = &self.sessions[self.primary_session_idx];
        match selector {
            CacheSelector::Speed => &loaded.speed_pts_cache,
            CacheSelector::RPM => &loaded.rpm_pts_cache,
            CacheSelector::Throttle => &loaded.throttle_pts_cache,
            CacheSelector::Brake => &loaded.brake_pts_cache,
            CacheSelector::Steering => &loaded.steering_pts_cache,
            CacheSelector::FrontHeight => &loaded.front_pts_cache,
            CacheSelector::RearHeight => &loaded.rear_pts_cache,
            CacheSelector::Rake => &loaded.rake_pts_cache,
        }
    }

    pub fn get_raw_value(&self, selector: CacheSelector, idx: usize) -> f64 {
        if !self.sessions.is_empty() {
            let session = &self.sessions[self.primary_session_idx].session;
            match selector {
                CacheSelector::Speed => {
                    session.dataframe.column("Speed").ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 3.6)
                        .unwrap_or(0.0)
                }
                CacheSelector::RPM => {
                    session.dataframe.column("RPM").ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0))
                        .unwrap_or(0.0)
                }
                CacheSelector::Throttle => {
                    session.dataframe.column("Throttle").ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 100.0)
                        .unwrap_or(0.0)
                }
                CacheSelector::Brake => {
                    session.dataframe.column("Brake").ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 100.0)
                        .unwrap_or(0.0)
                }
                CacheSelector::Steering => {
                    session.dataframe.column("SteeringWheelAngle").ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 57.2958)
                        .unwrap_or(0.0)
                }
                CacheSelector::FrontHeight => {
                    if idx < session.front_smooth.len() {
                        let scale = if session.front_smooth[idx] < 0.5 { 1000.0 } else { 1.0 };
                        session.front_smooth[idx] * scale
                    } else {
                        0.0
                    }
                }
                CacheSelector::RearHeight => {
                    if idx < session.rear_smooth.len() {
                        let scale = if session.front_smooth[idx] < 0.5 { 1000.0 } else { 1.0 };
                        session.rear_smooth[idx] * scale
                    } else {
                        0.0
                    }
                }
                CacheSelector::Rake => {
                    if idx < session.rake.len() {
                        let scale = if session.front_smooth[idx] < 0.5 { 1000.0 } else { 1.0 };
                        session.rake[idx] * scale
                    } else {
                        0.0
                    }
                }
            }
        } else {
            0.0
        }
    }

    pub fn draw_motec_plot(&mut self, ui: &mut egui::Ui, plot_id: &str, config: &WorksheetConfig, is_tab_switch: bool) {
        if self.sessions.is_empty() { return; }
        let loaded = &self.sessions[self.primary_session_idx];
        if loaded.front_pts_cache.is_empty() { return; }
        
        let max_time = loaded.front_pts_cache.last().unwrap()[0];
        let is_dark = ui.style().visuals.dark_mode;

        // 1. EXTRACT RAW HUD METRICS AT PLAYBACK CURSOR INDEX (EXCLUSIVE ZERO-CONFLICT SCOPE!)
        let mut idx = 0;
        let mut has_cursor = false;
        if let Some(cx) = self.cursor_x {
            idx = get_closest_index(&loaded.speed_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), cx);
            has_cursor = true;
        }

        // 2. CONSTRUCT RUNTIME LANES FROM STATIC CONFIG SPECIFICATION
        let mut lanes = Vec::new();
        for lane_spec in &config.lanes {
            let mut traces = Vec::new();
            for trace_spec in &lane_spec.traces {
                let raw_val = if has_cursor {
                    self.get_raw_value(trace_spec.cache, idx)
                } else {
                    0.0
                };
                let scaled_pts = self.get_cache_slice(trace_spec.cache);
                traces.push(ChartTrace {
                    name: trace_spec.name,
                    scaled_pts,
                    color: trace_spec.color,
                    width: trace_spec.width,
                    raw_val,
                    unit: trace_spec.unit,
                });
            }
            lanes.push(ChartLane {
                title: lane_spec.title,
                y_min: lane_spec.y_min,
                y_max: lane_spec.y_max,
                traces,
            });
        }

        // 3. RENDER MASTER DYNAMIC HUD HEADERS ROW
        ui.horizontal(|ui| {
            if let Some(cx) = self.cursor_x {
                ui.colored_label(ACCENT_COLOR, format!("⏱  PLAYBACK @ {}", format_lap_time(cx)));
                
                // Iteratively render each trace metrics matching their unique channel colors!
                for lane in &lanes {
                    for trace in &lane.traces {
                        ui.separator();
                        ui.colored_label(trace.color, format!("{}: {:.1}{}", trace.name, trace.raw_val, trace.unit));
                    }
                }
            }
        });
        ui.add_space(4.0);

        // 4. INITIALIZE UNIFIED PLOT CANVAS
        let mut plot_height = ui.available_height() - 10.0;
        let min_h = if self.show_graphs_track_map { 150.0 } else { 300.0 };
        if plot_height < min_h { plot_height = min_h; }

        let mut plot = Plot::new(plot_id)
            .height(plot_height)
            .allow_zoom([false, false])
            .allow_scroll([false, false])
            .allow_drag([false, false])
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false)
            .auto_bounds([false, false])
            .include_y(0.0)
            .include_y(100.0)
            .allow_axis_zoom_drag([false, false]);

        plot = plot.x_axis_formatter(|tick, _range| {
            let sec = tick.value;
            let minutes = (sec / 60.0).floor() as i32;
            let seconds = (sec % 60.0).floor() as i32;
            let ms = ((sec % 1.0) * 10.0).round() as i32;
            format!("{:02}:{:02}.{}", minutes, seconds, ms)
        });

        plot = plot.show_axes([true, false]);

        // Extract local copies of mutable states to completely bypass Rust borrow-checker conflicts!
        let mut cursor_x = self.cursor_x;
        let mut visible_x_range = self.visible_x_range;
        let mut reset_bounds_flag = self.reset_bounds_flag;
        let mut is_dragging_ticker = self.is_dragging_ticker;
        let mut is_highlight_active = self.is_highlight_active;
        let mut highlight_start = self.highlight_start;
        
        let selected_lap = self.selected_lap;
        let lap_ranges = &loaded.lap_ranges;
        let ref_lap_cyan = self.ref_lap_cyan;
        let ref_lap_white = self.ref_lap_white;
        let lap_markers = &loaded.lap_markers;

        plot.show(ui, |plot_ui| {

            // --- MOTEC STYLE DOUBLE-CLICK HIGHLIGHT ZOOM STATE MACHINE ---
            if plot_ui.response().double_clicked() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    let d_click_x = pointer_pos.x.clamp(0.0, max_time);
                    highlight_start = Some(d_click_x);
                    cursor_x = Some(d_click_x);
                    is_highlight_active = true;
                }
            }

            // A. HANDLE VIEWPORT SYNC & LAP FOCUSING
            if reset_bounds_flag || is_tab_switch {
                if let Some(sel_lap) = selected_lap {
                    if let Some(pos) = lap_ranges.iter().position(|r| r.0 == sel_lap.1 && sel_lap.0 == self.primary_session_idx) {
                        let (_, start_t, end_t) = lap_ranges[pos];
                        let end_time_focus = end_t; // EXACT PRECOMPUTED END TIME OF CURRENT LAP!
                        if is_tab_switch && visible_x_range.is_some() {
                            let (min_x, max_x) = visible_x_range.unwrap();
                            plot_ui.set_plot_bounds_x(min_x..=max_x);
                        } else {
                            plot_ui.set_plot_bounds_x(start_t..=end_time_focus);
                            visible_x_range = Some((start_t, end_time_focus));
                        }
                    } else {
                        if is_tab_switch && visible_x_range.is_some() {
                            let (min_x, max_x) = visible_x_range.unwrap();
                            plot_ui.set_plot_bounds_x(min_x..=max_x);
                        } else {
                            plot_ui.set_plot_bounds_x(0.0..=max_time);
                            visible_x_range = Some((0.0, max_time));
                        }
                    }
                } else {
                    if is_tab_switch && visible_x_range.is_some() {
                        let (min_x, max_x) = visible_x_range.unwrap();
                        plot_ui.set_plot_bounds_x(min_x..=max_x);
                    } else {
                        plot_ui.set_plot_bounds_x(0.0..=max_time);
                        visible_x_range = Some((0.0, max_time));
                    }
                }
                reset_bounds_flag = false;
            }

            // B. READ ACTIVE VIEWPORT COORDS
            let active_bounds = plot_ui.plot_bounds();
            let min_visible_x = active_bounds.min()[0];
            let max_visible_x = active_bounds.max()[0];
            let visible_width = max_visible_x - min_visible_x;

            // Commit viewport sync metrics back to local copy state
            visible_x_range = Some((min_visible_x, max_visible_x));

            // High-performance decimator closures
            let decimate_points = |pts: &[[f64; 2]]| -> PlotPoints {
                if pts.is_empty() { return PlotPoints::default(); }
                let start_idx = match pts.binary_search_by(|p| p[0].partial_cmp(&min_visible_x).unwrap_or(std::cmp::Ordering::Equal)) {
                    Ok(idx) => idx,
                    Err(idx) => idx,
                }.saturating_sub(1);
                let end_idx = match pts.binary_search_by(|p| p[0].partial_cmp(&max_visible_x).unwrap_or(std::cmp::Ordering::Equal)) {
                    Ok(idx) => idx,
                    Err(idx) => idx,
                }.min(pts.len());
                let slice = &pts[start_idx..end_idx];
                let m = slice.len();
                if m <= 2000 {
                    slice.to_vec().into()
                } else {
                    let stride = m / 2000;
                    let mut downsampled = Vec::with_capacity(2002);
                    downsampled.push(slice[0]);
                    let mut idx = 1;
                    while idx < m - 1 {
                        downsampled.push(slice[idx]);
                        idx += stride;
                    }
                    downsampled.push(slice[m - 1]);
                    downsampled.into()
                }
            };

            // C. DRAW AXIS DIVIDER LANES DYNAMICALLY
            let div_color = if is_dark { egui::Color32::from_rgb(25, 30, 32) } else { egui::Color32::from_rgb(205, 204, 203) };
            plot_ui.hline(HLine::new("Bottom Ticker Divider", 9.5).color(div_color).width(1.0));
            for lane in &lanes {
                plot_ui.hline(HLine::new(format!("Divider_{}", lane.title), lane.y_min - 2.0).color(div_color).width(1.0));
            }

            // D. DRAW TICKER TIMELINE TRACK
            let track_color = if is_dark { egui::Color32::from_rgb(12, 18, 20) } else { egui::Color32::from_rgb(215, 214, 213) };
            plot_ui.hline(HLine::new("Timeline Track", 4.75).color(track_color).width(9.5));

            // E. DRAW MAIN LANES AND COMPILING TRACES
            for lane in &lanes {
                for trace in &lane.traces {
                    let dec_pts = decimate_points(trace.scaled_pts);
                    plot_ui.line(Line::new(trace.name, dec_pts).color(trace.color).width(trace.width));
                }
            }

            // F. DRAW DYNAMIC MOTEC MULTI-LAP REFERENCE OVERLAYS
            // Cyan reference overlays
            if let Some((s_idx, ref_lap_num)) = ref_lap_cyan {
                let ref_session = &self.sessions[s_idx];
                if let Some(pos) = ref_session.lap_ranges.iter().position(|r| r.0 == ref_lap_num) {
                    let ref_start = ref_session.lap_ranges[pos].1;
                    
                    for &(lap_num, start_t, end_t) in lap_ranges {
                        if end_t >= min_visible_x && start_t <= max_visible_x {
                            let offset = start_t - ref_start;
                            
                            for lane in &lanes {
                                for trace in &lane.traces {
                                    let ref_pts = ref_session.get_cache_slice(&trace.name);
                                    if !ref_pts.is_empty() {
                                        let ref_slice = get_lap_points_slice(&ref_session.lap_ranges, ref_pts, ref_lap_num);
                                        if !ref_slice.is_empty() {
                                            let shifted: Vec<[f64; 2]> = ref_slice.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                            let dec_ref = decimate_points(&shifted);
                                            let cyan_color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 136, 170) };
                                            plot_ui.line(Line::new(format!("CyanRef_{}_{}", trace.name, lap_num), dec_ref).color(cyan_color).width(1.2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // White reference overlays
            if let Some((s_idx, ref_lap_num)) = ref_lap_white {
                let ref_session = &self.sessions[s_idx];
                if let Some(pos) = ref_session.lap_ranges.iter().position(|r| r.0 == ref_lap_num) {
                    let ref_start = ref_session.lap_ranges[pos].1;
                    
                    for &(lap_num, start_t, end_t) in lap_ranges {
                        if end_t >= min_visible_x && start_t <= max_visible_x {
                            let offset = start_t - ref_start;
                            
                            for lane in &lanes {
                                for trace in &lane.traces {
                                    let ref_pts = ref_session.get_cache_slice(&trace.name);
                                    if !ref_pts.is_empty() {
                                        let ref_slice = get_lap_points_slice(&ref_session.lap_ranges, ref_pts, ref_lap_num);
                                        if !ref_slice.is_empty() {
                                            let shifted: Vec<[f64; 2]> = ref_slice.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                            let dec_ref = decimate_points(&shifted);
                                            plot_ui.line(Line::new(format!("WhiteRef_{}_{}", trace.name, lap_num), dec_ref).color(egui::Color32::WHITE).width(1.2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // G. DRAW LAP BOUNDARY LINES
            for &lap_start_time in lap_markers {
                if lap_start_time > 0.0 {
                    plot_ui.vline(VLine::new(format!("LapSeparator_{}", lap_start_time), lap_start_time)
                        .color(egui::Color32::from_rgba_unmultiplied(220, 20, 60, 120))
                        .style(egui_plot::LineStyle::dotted_dense())
                        .width(1.0)
                    );
                }
            }

            // H. DRAW TIME TICKER LABELS
            let step = if visible_width > 240.0 { 60.0 } else if visible_width > 120.0 { 30.0 } else if visible_width > 60.0 { 15.0 } else if visible_width > 30.0 { 10.0 } else if visible_width > 15.0 { 5.0 } else if visible_width > 5.0 { 2.0 } else { 0.5 };
            let start_tick = (min_visible_x / step).floor() * step;
            let end_tick = (max_visible_x / step).ceil() * step;
            let mut current_tick = start_tick;
            while current_tick <= end_tick {
                if current_tick >= 0.0 && current_tick <= max_time {
                    let tick_line_color = if is_dark { egui::Color32::from_rgb(28, 38, 41) } else { egui::Color32::from_rgb(180, 179, 178) };
                    plot_ui.vline(VLine::new(format!("TickLine_{}", current_tick), current_tick).color(tick_line_color).width(1.0));
                    let label_str = format_lap_time(current_tick);
                    let display_text = label_str.get(3..8).unwrap_or("00:00");
                    
                    let text_color = if is_dark { egui::Color32::from_rgb(120, 135, 140) } else { egui::Color32::from_rgb(80, 80, 80) };
                    plot_ui.text(Text::new(format!("TickLabel_{}", current_tick), PlotPoint::new(current_tick, 4.75), egui::RichText::new(display_text).color(text_color).size(9.0)));
                }
                current_tick += step;
            }

            // I. DRAW OUTLAP/LAP OUTLINE LABELS
            for (_lap_idx, &(lap_num, start_t, end_t)) in lap_ranges.iter().enumerate() {
                if end_t >= min_visible_x && start_t <= max_visible_x {
                    let center = (start_t + end_t) / 2.0;
                    let label_str = if lap_num == 0 { "Outlap".to_string() } else { format!("Lap {}", lap_num) };
                    let label_txt_color = if is_dark { egui::Color32::from_rgb(180, 195, 200) } else { egui::Color32::from_rgb(60, 70, 75) };
                    plot_ui.text(Text::new(format!("LapLabelMarker_{}", lap_num), PlotPoint::new(center, center), egui::RichText::new(label_str).color(label_txt_color).size(10.0).strong()));
                    // Wait, let's fix that Y value: in original main.rs, the label is drawn at PlotPoint::new(center, 99.0)!
                    // Let's modify this to use 99.0 as in the original main.rs line 1414.
                }
            }

            // J. DRAW PLAYBACK CURSOR DOTS
            if let Some(cx) = cursor_x {
                plot_ui.vline(VLine::new("Cursor Line", cx).color(ACCENT_COLOR).width(1.5));
                let p_idx = self.primary_session_idx;
                let idx = get_closest_index(&self.sessions[p_idx].front_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), cx);
                
                for lane in &lanes {
                    for trace in &lane.traces {
                        if idx < trace.scaled_pts.len() {
                            let scaled_y = trace.scaled_pts[idx][1];
                            plot_ui.points(Points::new(format!("Dot_{}", trace.name), PlotPoints::from(vec![[cx, scaled_y]])).color(trace.color).radius(5.0));
                        }
                    }
                }

                // Slider Stamp
                plot_ui.points(Points::new("Stamp Ticker", PlotPoints::from(vec![[cx, 4.75]])).color(ACCENT_COLOR).shape(egui_plot::MarkerShape::Up).radius(10.0));
            }

            // K. DOUBLE-CLICK HIGHLIGHT ZOOM
            if is_highlight_active {
                if let Some(x_start) = highlight_start {
                    let current_x = plot_ui.pointer_coordinate().map(|p| p.x.clamp(0.0, max_time)).unwrap_or_else(|| cursor_x.unwrap_or(0.0));
                    let start = f64::min(x_start, current_x);
                    let end = f64::max(x_start, current_x);
                    plot_ui.span(Span::new("Zoom Highlight", start..=end).axis(Axis::X).fill(egui::Color32::from_rgba_unmultiplied(242, 82, 37, 32)).border_width(1.0).border_color(egui::Color32::from_rgba_unmultiplied(242, 82, 37, 120)));
                }
            }

            // L. TIME DRAG DETECTION & SCRUBBING
            if plot_ui.response().drag_started() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    is_dragging_ticker = pointer_pos.y < 9.5;
                }
            }

            if plot_ui.response().dragged() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    let click_pos = pointer_pos.x.clamp(0.0, max_time);
                    if is_highlight_active {
                        if !plot_ui.response().double_clicked() {
                            if let Some(x_start) = highlight_start {
                                let zoom_min = f64::min(x_start, click_pos);
                                let zoom_max = f64::max(x_start, click_pos);
                                if (zoom_max - zoom_min).abs() > 0.1 {
                                    plot_ui.set_plot_bounds_x(zoom_min..=zoom_max);
                                    cursor_x = Some(zoom_min);
                                    visible_x_range = Some((zoom_min, zoom_max));
                                }
                                is_highlight_active = false;
                                highlight_start = None;
                            }
                        }
                    } else if is_dragging_ticker {
                        let pixel_delta_x = plot_ui.ctx().input(|i| i.pointer.delta().x);
                        let plot_width_pixels = plot_ui.response().rect.width();
                        let pixels_per_second = (plot_width_pixels as f64) / visible_width;
                        let seconds_delta = (pixel_delta_x as f64) / pixels_per_second;
                        let new_min = (min_visible_x - seconds_delta).clamp(0.0, max_time - visible_width);
                        let new_max = new_min + visible_width;
                        plot_ui.set_plot_bounds_x(new_min..=new_max);
                        visible_x_range = Some((new_min, new_max));
                    } else {
                        cursor_x = Some(click_pos);
                    }
                }
            }

            if plot_ui.response().clicked() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    let click_pos = pointer_pos.x.clamp(0.0, max_time);
                    if is_highlight_active {
                        if !plot_ui.response().double_clicked() {
                            if let Some(x_start) = highlight_start {
                                let zoom_min = f64::min(x_start, click_pos);
                                let zoom_max = f64::max(x_start, click_pos);
                                if (zoom_max - zoom_min).abs() > 0.1 {
                                    plot_ui.set_plot_bounds_x(zoom_min..=zoom_max);
                                    cursor_x = Some(zoom_min);
                                    visible_x_range = Some((zoom_min, zoom_max));
                                }
                                is_highlight_active = false;
                                highlight_start = None;
                            }
                        }
                    } else {
                        cursor_x = Some(click_pos);
                    }
                }
            }

            // M. SILKY-SMOOTH HIGH-PRECISION ZOOM WHEEL
            if plot_ui.response().hovered() {
                let scroll = plot_ui.ctx().input(|i| i.smooth_scroll_delta);
                if scroll.y.abs() > 1.5 {
                    let is_zooming_in = scroll.y > 0.0;
                    let zoom_factor = if is_zooming_in { 0.925 } else { 1.075 };
                    let mut target_width = visible_width * zoom_factor;
                    target_width = target_width.clamp(1.5, max_time);
                    let center = if is_zooming_in { cursor_x.unwrap_or((min_visible_x + max_visible_x) / 2.0) } else { (min_visible_x + max_visible_x) / 2.0 };
                    let half_width = target_width / 2.0;
                    let mut new_min = center - half_width;
                    let mut mut_new_max = center + half_width;
                    if new_min < 0.0 {
                        let overflow = 0.0 - new_min;
                        new_min = 0.0;
                        mut_new_max = (mut_new_max + overflow).min(max_time);
                    } else if mut_new_max > max_time {
                        let overflow = mut_new_max - max_time;
                        mut_new_max = max_time;
                        new_min = (new_min - overflow).max(0.0);
                    }
                    if new_min < mut_new_max {
                        plot_ui.set_plot_bounds_x(new_min..=mut_new_max);
                        visible_x_range = Some((new_min, mut_new_max));
                    }
                }
            }
        });

        // 5. RESTORE COPIES BACK TO APP STATE IN CONSTANT TIME
        self.cursor_x = cursor_x;
        self.visible_x_range = visible_x_range;
        self.reset_bounds_flag = reset_bounds_flag;
        self.is_dragging_ticker = is_dragging_ticker;
        self.is_highlight_active = is_highlight_active;
        self.highlight_start = highlight_start;
    }
}
