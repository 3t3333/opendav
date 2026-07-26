use crate::config::theme::AppTheme;
use crate::config::worksheet::{CacheSelector, WorksheetConfig};
use crate::signals::processing::{format_lap_time, get_closest_index, get_lap_points_slice};
use crate::OpenDavApp;
use egui_plot::{
    Axis, HLine, Line, Plot, PlotPoint, PlotPoints, Points, Polygon, Span, Text, VLine,
};

pub struct ChartTrace<'a> {
    pub name: &'static str,
    pub cache: CacheSelector,
    pub scaled_pts: &'a [[f64; 2]],
    pub color: egui::Color32,
    pub width: f32,
    pub raw_val: f64,
    pub cyan_ref_val: Option<f64>,
    pub secondary_ref_val: Option<f64>,
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
        if self.sessions.is_empty() {
            return &[];
        }
        let loaded = &self.sessions[self.primary_session_idx];
        match selector {
            CacheSelector::Speed => &loaded.speed_pts_cache,
            CacheSelector::RPM => &loaded.rpm_pts_cache,
            CacheSelector::Throttle => &loaded.throttle_pts_cache,
            CacheSelector::Brake => &loaded.brake_pts_cache,
            CacheSelector::Steering => &loaded.steering_pts_cache,
            CacheSelector::FrontHeight => &loaded.front_raw_pts_cache,
            CacheSelector::RearHeight => &loaded.rear_raw_pts_cache,
            CacheSelector::Rake => &loaded.rake_pts_cache,
            CacheSelector::LatG => &loaded.lat_g_pts_cache,
            CacheSelector::LongG => &loaded.long_g_pts_cache,
            CacheSelector::Gear => &loaded.gear_pts_cache,
            CacheSelector::Clutch => &loaded.clutch_pts_cache,
            CacheSelector::DistanceDelta => &loaded.distance_delta_pts_cache,
            CacheSelector::TimeDelta => &loaded.time_delta_pts_cache,
        }
    }

    pub fn get_raw_value(&self, session_idx: usize, selector: CacheSelector, idx: usize) -> f64 {
        if session_idx < self.sessions.len() {
            let session = &self.sessions[session_idx].session;
            match selector {
                CacheSelector::Speed => session
                    .dataframe
                    .column("Speed")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 3.6)
                    .unwrap_or(0.0),
                CacheSelector::RPM => session
                    .dataframe
                    .column("RPM")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0))
                    .unwrap_or(0.0),
                CacheSelector::Throttle => session
                    .dataframe
                    .column("Throttle")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 100.0)
                    .unwrap_or(0.0),
                CacheSelector::Brake => session
                    .dataframe
                    .column("Brake")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 100.0)
                    .unwrap_or(0.0),
                CacheSelector::Steering => session
                    .dataframe
                    .column("SteeringWheelAngle")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 57.2958)
                    .unwrap_or(0.0),
                CacheSelector::LatG => session
                    .dataframe
                    .column("LatAccel")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) / 9.80665)
                    .unwrap_or(0.0),
                CacheSelector::LongG => session
                    .dataframe
                    .column("LongAccel")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) / 9.80665)
                    .unwrap_or(0.0),
                CacheSelector::FrontHeight => {
                    if idx < session.front_raw.len() {
                        session.front_raw[idx]
                    } else {
                        0.0
                    }
                }
                CacheSelector::RearHeight => {
                    if idx < session.rear_raw.len() {
                        session.rear_raw[idx]
                    } else {
                        0.0
                    }
                }
                CacheSelector::Rake => {
                    if idx < session.rake.len() {
                        session.rake[idx]
                    } else {
                        0.0
                    }
                }
                CacheSelector::Gear => session
                    .dataframe
                    .column("Gear")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0))
                    .unwrap_or(0.0),
                CacheSelector::Clutch => session
                    .dataframe
                    .column("ClutchRaw")
                    .ok()
                        .and_then(|c| c.f64().ok())
                        .map(|c| c.get(idx).unwrap_or(0.0) * 100.0)
                    .unwrap_or(0.0),
                CacheSelector::DistanceDelta => {
                    let loaded = &self.sessions[session_idx];
                    if idx < loaded.distance_delta_pts_cache.len() {
                        loaded.distance_delta_pts_cache[idx][1]
                    } else {
                        0.0
                    }
                }
                CacheSelector::TimeDelta => {
                    let loaded = &self.sessions[session_idx];
                    if idx < loaded.time_delta_pts_cache.len() {
                        loaded.time_delta_pts_cache[idx][1]
                    } else {
                        0.0
                    }
                }
            }
        } else {
            0.0
        }
    }

    pub fn draw_motec_plot(
        &mut self,
        ui: &mut egui::Ui,
        plot_id: &str,
        config: &WorksheetConfig,
        is_tab_switch: bool,
    ) {
        #[cfg(feature = "dev_tools")]
        let debug_start_time = std::time::Instant::now();
        #[cfg(feature = "dev_tools")]
        let debug_pts_rendered = std::rc::Rc::new(std::cell::Cell::new(0usize));
        #[cfg(feature = "dev_tools")]
        let debug_pts_culled = std::rc::Rc::new(std::cell::Cell::new(0usize));

        if self.sessions.is_empty() {
            return;
        }
        let loaded = &self.sessions[self.primary_session_idx];
        if loaded.front_pts_cache.is_empty() {
            return;
        }
        
        let max_time = loaded.front_pts_cache.last().unwrap()[0];
        let is_dark = ui.style().visuals.dark_mode;
        let theme = AppTheme::for_mode(is_dark);
        // Dynamic HUD labels may overflow a narrow workspace, but must not resize the plot.
        let plot_width = ui.available_width();

        // 1. EXTRACT RAW HUD METRICS AT PLAYBACK CURSOR INDEX (EXCLUSIVE ZERO-CONFLICT SCOPE!)
        let mut df_idx = 0;
        let mut has_cursor = false;
        if let Some(cx) = self.cursor_x {
            let cache_idx = get_closest_index(
                &loaded
                    .speed_pts_cache
                    .iter()
                    .map(|p| p[0])
                    .collect::<Vec<f64>>(),
                cx,
            );
            if cache_idx < loaded.cache_to_df_index.len() {
                df_idx = loaded.cache_to_df_index[cache_idx];
                has_cursor = true;
            }
        }

        // 2. CONSTRUCT RUNTIME LANES FROM STATIC CONFIG SPECIFICATION
        let mut lanes = Vec::new();
        for lane_spec in &config.lanes {
            let mut traces = Vec::new();
            for trace_spec in &lane_spec.traces {
                let mut raw_val = if has_cursor {
                    self.get_raw_value(self.primary_session_idx, trace_spec.cache, df_idx)
                } else {
                    0.0
                };
                
                let mut unit = trace_spec.unit;
                if !self.settings.use_metric {
                    match trace_spec.cache {
                        CacheSelector::Speed => {
                            raw_val *= 0.621371; // km/h to mph
                            unit = " mph";
                        }
                        CacheSelector::FrontHeight
                        | CacheSelector::RearHeight
                        | CacheSelector::Rake => {
                            raw_val *= 0.0393701; // mm to inches
                            unit = " in";
                        }
                        _ => {}
                    }
                }
                let reference_value =
                    |cache: Option<&crate::signals::comparison::ComparisonCache>| {
                        let mut value = cache
                            .and_then(|cache| cache.channel(trace_spec.cache))
                            .and_then(|channel| {
                                self.cursor_x.and_then(|cx| channel.raw_value_at(cx))
                            })?;
                                        if !self.settings.use_metric {
                                            match trace_spec.cache {
                                CacheSelector::Speed => value *= 0.621371,
                                CacheSelector::FrontHeight
                                | CacheSelector::RearHeight
                                | CacheSelector::Rake => value *= 0.0393701,
                                                _ => {}
                                            }
                                        }
                        Some(value)
                    };
                let cyan_ref_val = reference_value(self.comparison_cyan.as_ref());
                let secondary_ref_val = reference_value(self.comparison_secondary.as_ref());

                let scaled_pts = self.get_cache_slice(trace_spec.cache);
                traces.push(ChartTrace {
                    name: trace_spec.name,
                    cache: trace_spec.cache,
                    scaled_pts,
                    color: trace_spec.color,
                    width: trace_spec.width,
                    raw_val,
                    cyan_ref_val,
                    secondary_ref_val,
                    unit,
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
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(cx) = self.cursor_x {
                    ui.colored_label(
                        theme.accent_text,
                        format!("PLAYBACK @ {}", format_lap_time(cx)),
                    );
                    for lane in &lanes {
                        for trace in &lane.traces {
                            ui.separator();
                            let val_fmt = if trace.name == "Gear" {
                                format!("{:.0}", trace.raw_val)
                            } else {
                                format!("{:.1}", trace.raw_val)
                            };
                            ui.colored_label(
                                trace.color,
                                format!("{}: {}{}", trace.name, val_fmt, trace.unit),
                            );
                        }
                    }
                }
            });

            if self.comparison_cyan.is_some() {
                ui.horizontal_wrapped(|ui| {
                    if self.cursor_x.is_some() {
                        ui.colored_label(theme.reference_primary, "CYAN REFERENCE");
                        for lane in &lanes {
                            for trace in &lane.traces {
                                ui.separator();
                                if let Some(r_val) = trace.cyan_ref_val {
                                    let ref_val_fmt = if trace.name == "Gear" {
                                        format!("{:.0}", r_val)
                                } else {
                                        format!("{:.1}", r_val)
                                    };
                                    ui.colored_label(
                                        theme.reference_primary,
                                        format!("{}: {}{}", trace.name, ref_val_fmt, trace.unit),
                                    );
                                } else {
                                    ui.colored_label(
                                        theme.text_disabled,
                                        format!("{}: N/A{}", trace.name, trace.unit),
                                    );
                                }
                            }
                        }
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    if self.cursor_x.is_some() {
                        ui.colored_label(theme.reference_primary, "CYAN DELTA");
                        for lane in &lanes {
                            for trace in &lane.traces {
                                ui.separator();
                                if let Some(r_val) = trace.cyan_ref_val {
                                    let delta = trace.raw_val - r_val; // base - reference
                                    let sign = if delta > 0.0 { "+" } else { "" };
                                    let color = if delta <= 0.0 {
                                        theme.success
                                    } else {
                                        theme.danger
                                    };
                                    ui.colored_label(
                                        color,
                                        format!(
                                            "{}: {}{:.1}{}",
                                            trace.name, sign, delta, trace.unit
                                        ),
                                    );
                                } else {
                                    ui.colored_label(
                                        theme.text_disabled,
                                        format!("{}: N/A{}", trace.name, trace.unit),
                                    );
                                }
                            }
                        }
                    }
                });
            }

            if self.comparison_secondary.is_some() {
                ui.horizontal_wrapped(|ui| {
                    if self.cursor_x.is_some() {
                        ui.colored_label(theme.reference_secondary, "SECONDARY REFERENCE");
                        for lane in &lanes {
                            for trace in &lane.traces {
                                ui.separator();
                                if let Some(value) = trace.secondary_ref_val {
                                    let value = if trace.name == "Gear" {
                                        format!("{value:.0}")
                                    } else {
                                        format!("{value:.1}")
                                    };
                                    ui.colored_label(
                                        theme.reference_secondary,
                                        format!("{}: {}{}", trace.name, value, trace.unit),
                                    );
                                } else {
                                    ui.colored_label(
                                        theme.text_disabled,
                                        format!("{}: N/A{}", trace.name, trace.unit),
                                    );
                                }
                            }
                        }
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    if self.cursor_x.is_some() {
                        ui.colored_label(theme.reference_secondary, "SECONDARY DELTA");
                        for lane in &lanes {
                            for trace in &lane.traces {
                                ui.separator();
                                if let Some(reference) = trace.secondary_ref_val {
                                    let delta = trace.raw_val - reference;
                                    let sign = if delta > 0.0 { "+" } else { "" };
                                    let color = if delta <= 0.0 {
                                        theme.success
                                    } else {
                                        theme.danger
                                    };
                                    ui.colored_label(
                                        color,
                                        format!(
                                            "{}: {}{:.1}{}",
                                            trace.name, sign, delta, trace.unit
                                        ),
                                    );
                                } else {
                                    ui.colored_label(
                                        theme.text_disabled,
                                        format!("{}: N/A{}", trace.name, trace.unit),
                                    );
                                }
                            }
                        }
                    }
                });
            }
        });
        ui.add_space(4.0);

        // 4. INITIALIZE UNIFIED PLOT CANVAS
        let mut plot_height = ui.available_height() - 10.0;
        if plot_height < 300.0 {
            plot_height = 300.0;
        }

        let mut plot = Plot::new(plot_id)
            .width(plot_width)
            .height(plot_height)
            .allow_zoom([false, false])
            .allow_scroll([false, false])
            .allow_drag([false, false])
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false)
            .show_grid(false)
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
        let lap_markers = &loaded.lap_markers;
        let show_chart_deltas = self.show_chart_deltas;
        let sector_deltas = self.sector_deltas.clone();

        #[cfg(feature = "dev_tools")]
        let debug_pts_rendered_clone = debug_pts_rendered.clone();
        #[cfg(feature = "dev_tools")]
        let debug_pts_culled_clone = debug_pts_culled.clone();

        plot.show(ui, |plot_ui| {
            // B. READ ACTIVE VIEWPORT COORDS
            let active_bounds = plot_ui.plot_bounds();
            let min_visible_x = active_bounds.min()[0];
            let max_visible_x = active_bounds.max()[0];
            let visible_width = max_visible_x - min_visible_x;

            // --- MOTEC STYLE DOUBLE-CLICK HIGHLIGHT ZOOM STATE MACHINE ---
            if plot_ui.response().double_clicked() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    let d_click_x = pointer_pos.x.clamp(min_visible_x, max_visible_x);
                    highlight_start = Some(d_click_x);
                    cursor_x = Some(d_click_x);
                    is_highlight_active = true;
                }
            }

            // A. HANDLE VIEWPORT SYNC & LAP FOCUSING
            if reset_bounds_flag || is_tab_switch {
                if let Some(sel_lap) = selected_lap {
                    if let Some(pos) = lap_ranges
                        .iter()
                        .position(|r| r.0 == sel_lap.1 && sel_lap.0 == self.primary_session_idx)
                    {
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

            // Commit viewport sync metrics back to local copy state
            visible_x_range = Some((min_visible_x, max_visible_x));

            // Handle pointer input before rendering so manual scrubbing updates the cursor
            // in the same frame, matching the playback update path.
            if plot_ui.response().drag_started() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    is_dragging_ticker = pointer_pos.y < 9.5;
                }
            }

            if plot_ui.response().dragged() {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    let click_pos = pointer_pos.x.clamp(min_visible_x, max_visible_x);
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
                        let new_min =
                            (min_visible_x - seconds_delta).clamp(0.0, max_time - visible_width);
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
                    let click_pos = pointer_pos.x.clamp(min_visible_x, max_visible_x);
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

            if plot_ui.response().hovered() {
                let scroll = plot_ui.ctx().input(|i| i.smooth_scroll_delta);
                if scroll.y.abs() > 1.5 {
                    let is_zooming_in = scroll.y > 0.0;
                    let zoom_factor = if is_zooming_in { 0.925 } else { 1.075 };
                    let mut target_width = visible_width * zoom_factor;
                    target_width = target_width.clamp(1.5, max_time);
                    let center = if is_zooming_in {
                        cursor_x.unwrap_or((min_visible_x + max_visible_x) / 2.0)
                    } else {
                        (min_visible_x + max_visible_x) / 2.0
                    };
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

            // High-performance MinMax decimator closure (prevents aliasing/jumping spikes)
            let decimate_points = |pts: &[[f64; 2]]| -> PlotPoints {
                if pts.is_empty() {
                    return PlotPoints::default();
                }
                let start_idx = match pts.binary_search_by(|p| {
                    p[0].partial_cmp(&min_visible_x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    Ok(idx) => idx,
                    Err(idx) => idx,
                }
                .saturating_sub(1);
                let end_idx = match pts.binary_search_by(|p| {
                    p[0].partial_cmp(&max_visible_x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    Ok(idx) => idx,
                    Err(idx) => idx,
                }
                .min(pts.len());
                let slice = &pts[start_idx..end_idx];
                let m = slice.len();
                
                #[cfg(feature = "dev_tools")]
                {
                    debug_pts_culled_clone.set(debug_pts_culled_clone.get() + (pts.len() - m));
                }

                if m <= 2000 {
                    #[cfg(feature = "dev_tools")]
                    debug_pts_rendered_clone.set(debug_pts_rendered_clone.get() + m);

                    slice.to_vec().into()
                } else {
                    let stride = m / 1000;
                    let mut downsampled = Vec::with_capacity(2002);
                    downsampled.push(slice[0]);
                    let mut idx = 1;
                    while idx < m - 1 {
                        let chunk_end = (idx + stride).min(m - 1);
                        let chunk = &slice[idx..chunk_end];
                        
                        if !chunk.is_empty() {
                            let mut min_idx = 0;
                            let mut max_idx = 0;
                            let mut min_val = chunk[0][1];
                            let mut max_val = chunk[0][1];
                            
                            for (i, p) in chunk.iter().enumerate() {
                                if p[1] < min_val {
                                    min_val = p[1];
                                    min_idx = i;
                                }
                                if p[1] > max_val {
                                    max_val = p[1];
                                    max_idx = i;
                                }
                            }
                            
                            if min_idx < max_idx {
                                downsampled.push(chunk[min_idx]);
                                downsampled.push(chunk[max_idx]);
                            } else if max_idx < min_idx {
                                downsampled.push(chunk[max_idx]);
                                downsampled.push(chunk[min_idx]);
                            } else {
                                downsampled.push(chunk[min_idx]);
                            }
                        }
                        idx += stride;
                    }
                    downsampled.push(slice[m - 1]);

                    #[cfg(feature = "dev_tools")]
                    {
                        debug_pts_rendered_clone
                            .set(debug_pts_rendered_clone.get() + downsampled.len());
                        debug_pts_culled_clone
                            .set(debug_pts_culled_clone.get() + (m - downsampled.len()));
                    }

                    downsampled.into()
                }
            };

            // C. DRAW SECTOR DELTA SHADING (if enabled)
            if show_chart_deltas && !sector_deltas.is_empty() {
                if let Some((_, sel_lap_num)) = selected_lap {
                    if let Some(lap_data) = loaded
                        .lap_data_cache
                        .iter()
                        .find(|l| l.lap_num == sel_lap_num)
                    {
                        if let Some(pos) = loaded.lap_ranges.iter().position(|r| r.0 == sel_lap_num)
                        {
                            let start_t = loaded.lap_ranges[pos].1;
                        
                            for (s_idx, sector) in loaded.sectors.iter().enumerate() {
                                let delta = sector_deltas.get(s_idx).copied().flatten();
                                if let Some(d) = delta {
                                    let sector_start_t =
                                        crate::signals::processing::get_lap_time_at_distance(
                                            &lap_data.dist,
                                            &lap_data.time,
                                            sector.start_dist,
                                        );
                                    let sector_end_t =
                                        crate::signals::processing::get_lap_time_at_distance(
                                            &lap_data.dist,
                                            &lap_data.time,
                                            sector.end_dist,
                                        );
                                    
                                    let abs_start = start_t + sector_start_t;
                                    let abs_end = start_t + sector_end_t;

                                    if abs_end >= min_visible_x && abs_start <= max_visible_x {
                                        let bg_color = if d < 0.0 {
                                            theme.success.gamma_multiply(if is_dark {
                                                0.16
                                            } else {
                                                0.22
                                            })
                                        } else {
                                            theme.danger.gamma_multiply(if is_dark {
                                                0.16
                                            } else {
                                                0.22
                                            })
                                        };

                                        plot_ui.polygon(
                                            Polygon::new(
                                            format!("ChartSectorDeltaBg_{}", sector.name),
                                            PlotPoints::from(vec![
                                                [abs_start, 10.0],
                                                [abs_end, 10.0],
                                                [abs_end, 1000.0],
                                                [abs_start, 1000.0],
                                                ]),
                                            )
                                            .fill_color(bg_color)
                                            .width(0.0),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // D. DRAW AXIS DIVIDER LANES DYNAMICALLY
            let div_color = theme.plot_divider;
            plot_ui.hline(
                HLine::new("Bottom Ticker Divider", 9.5)
                    .color(div_color)
                    .width(1.0),
            );
            for lane in &lanes {
                plot_ui.hline(
                    HLine::new(format!("Divider_{}", lane.title), lane.y_min - 2.0)
                        .color(div_color)
                        .width(1.0),
                );
            }

            // D. DRAW TICKER TIMELINE TRACK
            let track_color = theme.surface_elevated;
            plot_ui.hline(
                HLine::new("Timeline Track", 4.75)
                    .color(track_color)
                    .width(9.5),
            );

            // E. DRAW MAIN LANES AND COMPILING TRACES
            for lane in &lanes {
                for trace in &lane.traces {
                    // Automatically draw smoothed variant behind raw data for Ride Heights and Rake
                    let smooth_cache_name = match trace.name {
                        "Front Height" | "Front RH" | "CFSRH" => Some("Ride Height (F) Smooth"),
                        "Rear Height" | "Rear RH" => Some("Ride Height (R) Smooth"),
                        "Dynamic Rake" => Some("Rake Angle Smooth"),
                        _ => None,
                    };
                    
                    if let Some(smooth_name) = smooth_cache_name {
                        let smooth_pts =
                            self.sessions[self.primary_session_idx].get_cache_slice(smooth_name);
                        if !smooth_pts.is_empty() {
                            let mut scaled_smooth = Vec::with_capacity(smooth_pts.len());
                            let active_lap_num = selected_lap.map(|(_, l)| l);
                            for &(l_num, st, et) in lap_ranges {
                                if et >= min_visible_x && st <= max_visible_x {
                                    if visible_width > 200.0 && active_lap_num != Some(l_num) {
                                        continue;
                                    }
                                    let lap_slice =
                                        get_lap_points_slice(lap_ranges, smooth_pts, l_num);
                                    for p in lap_slice {
                                        scaled_smooth.push(*p);
                                    }
                                }
                            }
                            let dec_smooth = decimate_points(&scaled_smooth);
                            plot_ui.line(
                                Line::new(format!("{}_Smooth", trace.name), dec_smooth)
                                    .color(trace.color.linear_multiply(0.45))
                                    .width(trace.width),
                            );
                        }
                    }

                    let dec_pts = decimate_points(trace.scaled_pts);
                    plot_ui.line(
                        Line::new(trace.name, dec_pts)
                            .color(trace.color)
                            .width(trace.width),
                    );
            }
                            }
                            
            // F. DRAW PRECOMPUTED, DISTANCE-ALIGNED REFERENCE OVERLAYS
            if let Some(cache) = self.comparison_cyan.as_ref() {
                for lane in &lanes {
                    for trace in &lane.traces {
                        if let Some(channel) = cache.channel(trace.cache) {
                            let points = decimate_points(&channel.scaled_points);
                            plot_ui.line(
                                Line::new("", points)
                                    .color(theme.reference_primary)
                                    .width(1.5),
                            );
                        }
                    }
                }
            }

            if let Some(cache) = self.comparison_secondary.as_ref() {
                for lane in &lanes {
                    for trace in &lane.traces {
                        if let Some(channel) = cache.channel(trace.cache) {
                            let points = decimate_points(&channel.scaled_points);
                            plot_ui.line(
                                Line::new("", points)
                                    .color(theme.reference_secondary)
                                    .width(1.5),
                            );
                        }
                    }
                }
            }

            // G. DRAW LAP BOUNDARY LINES
            for &lap_start_time in lap_markers {
                if lap_start_time > 0.0 {
                    plot_ui.vline(
                        VLine::new(format!("LapSeparator_{}", lap_start_time), lap_start_time)
                            .color(theme.danger.gamma_multiply(0.55))
                        .style(egui_plot::LineStyle::dotted_dense())
                            .width(1.0),
                    );
                }
            }

            // I. DRAW OUTLAP/LAP OUTLINE LABELS
            for (_lap_idx, &(lap_num, start_t, end_t)) in lap_ranges.iter().enumerate() {
                if end_t >= min_visible_x && start_t <= max_visible_x {
                    let center = (start_t + end_t) / 2.0;
                    let label_str = if lap_num == 0 {
                        "Outlap".to_string()
                    } else {
                        format!("Lap {}", lap_num)
                    };
                    let label_txt_color = theme.text_secondary;
                    plot_ui.text(Text::new(
                        format!("LapLabelMarker_{}", lap_num),
                        PlotPoint::new(center, 98.0),
                        egui::RichText::new(label_str)
                            .color(label_txt_color)
                            .size(10.0)
                            .strong(),
                    ));
                    
                    // I2. DRAW SECTOR LABELS ON TICKER
                    if let Some(lap_data) =
                        loaded.lap_data_cache.iter().find(|l| l.lap_num == lap_num)
                    {
                        for (sec_idx, sector) in loaded.sectors.iter().enumerate() {
                            let sector_start_t =
                                crate::signals::processing::get_lap_time_at_distance(
                                    &lap_data.dist,
                                    &lap_data.time,
                                    sector.start_dist,
                                );
                            let sector_end_t = crate::signals::processing::get_lap_time_at_distance(
                                &lap_data.dist,
                                &lap_data.time,
                                sector.end_dist,
                            );
                            
                            let abs_start = start_t + sector_start_t;
                            let abs_end = start_t + sector_end_t;
                            
                            if abs_end >= min_visible_x && abs_start <= max_visible_x {
                                let sec_center = (abs_start + abs_end) / 2.0;
                                let sector_width_s = abs_end - abs_start;
                                let ratio = sector_width_s / visible_width;
                                
                                let is_even = sec_idx % 2 == 0;
                                let bg_color = if is_even { 
                                    if is_dark {
                                        egui::Color32::from_rgba_unmultiplied(200, 200, 200, 4)
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(20, 20, 20, 4)
                                    }
                                } else { 
                                    if is_dark {
                                        egui::Color32::from_rgba_unmultiplied(200, 200, 200, 16)
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(20, 20, 20, 16)
                                    }
                                };
                                
                                plot_ui.polygon(
                                    Polygon::new(
                                    format!("TickerSectorBg_{}_{}", lap_num, sector.name),
                                    PlotPoints::from(vec![
                                        [abs_start, 0.0],
                                        [abs_end, 0.0],
                                        [abs_end, 9.5],
                                        [abs_start, 9.5],
                                        ]),
                                    )
                                    .fill_color(bg_color)
                                    .width(0.0),
                                );

                                let parts: Vec<&str> = sector.name.split(" - ").collect();
                                let short_name = parts[0];
                                
                                let est_px_width = ratio * 1200.0;
                                let req_px_width = short_name.len() as f64 * 7.5;
                                
                                if est_px_width > req_px_width {
                                    let dynamic_font_size =
                                        (ratio * 400.0).clamp(10.0, 12.0) as f32;
                                    let text_color = theme.text_secondary;
                                    
                                    plot_ui.text(Text::new(
                                        format!("TickerSector_{}_{}", lap_num, sector.name),
                                        PlotPoint::new(sec_center, 4.75),
                                        egui::RichText::new(short_name)
                                            .color(text_color)
                                            .size(dynamic_font_size)
                                            .strong(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            // J. DRAW PLAYBACK CURSOR DOTS
            if let Some(cx) = cursor_x {
                plot_ui.vline(VLine::new("CursorLine", cx).color(theme.accent).width(1.5));
                let p_idx = self.primary_session_idx;
                let idx = get_closest_index(
                    &self.sessions[p_idx]
                        .front_pts_cache
                        .iter()
                        .map(|p| p[0])
                        .collect::<Vec<f64>>(),
                    cx,
                );
                
                for lane in &lanes {
                    for trace in &lane.traces {
                        if idx < trace.scaled_pts.len() {
                            let scaled_y = trace.scaled_pts[idx][1];
                            plot_ui.points(
                                Points::new(
                                    format!("Dot_{}", trace.name),
                                    PlotPoints::from(vec![[cx, scaled_y]]),
                                )
                                .color(trace.color)
                                .radius(5.0),
                            );
                        }
                    }
                }

                plot_ui.points(
                    Points::new("Stamp Ticker", PlotPoints::from(vec![[cx, 4.75]]))
                        .color(theme.accent)
                        .shape(egui_plot::MarkerShape::Up)
                        .radius(10.0),
                );
            }

            // K. DOUBLE-CLICK HIGHLIGHT ZOOM
            if is_highlight_active {
                if let Some(x_start) = highlight_start {
                    let current_x = plot_ui
                        .pointer_coordinate()
                        .map(|p| p.x.clamp(0.0, max_time))
                        .unwrap_or_else(|| cursor_x.unwrap_or(0.0));
                    let start = f64::min(x_start, current_x);
                    let end = f64::max(x_start, current_x);
                    plot_ui.span(
                        Span::new("ZoomHighlight", start..=end)
                            .axis(Axis::X)
                            .fill(egui::Color32::from_rgba_unmultiplied(242, 82, 37, 32))
                            .border_width(1.0)
                            .border_color(egui::Color32::from_rgba_unmultiplied(242, 82, 37, 120)),
                    );
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

        #[cfg(feature = "dev_tools")]
        {
            self.dev_metrics.graph_render_time_ms =
                debug_start_time.elapsed().as_secs_f32() * 1000.0;
            self.dev_metrics.points_rendered = debug_pts_rendered.get();
            self.dev_metrics.points_culled = debug_pts_culled.get();
        }
    }
}
