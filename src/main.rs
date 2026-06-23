pub mod data;
pub mod signals;
pub mod config;
pub mod rendering;
pub mod ui;

use crate::config::worksheet::{WorksheetTab, WorksheetConfig, ACCENT_COLOR, DARK_BG_COLOR, LIGHT_BG_COLOR};
use crate::signals::processing::{
    LapData, TrackSector, detect_track_sectors, get_lap_time_at_distance, get_fastest_lap
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ActivePage {
    OpenDav,  // Main Dashboard
    Graphs,   // Telemetry Plots
    Reports,  // Sector Reports
}

#[derive(Clone, Debug)]
pub enum AppState {
    Splash { progress: f32 },
    Main,
}

pub struct OpenDavApp {
    pub app_state: AppState,
    pub active_page: ActivePage,
    pub active_worksheet: WorksheetTab,
    pub session_loaded: bool,
    pub active_file: Option<String>,
    pub session: Option<crate::data::ibt_parser::IbtSession>,
    pub fade_value: f32, // Smooth UI fade-in animation tracker

    // Caching compiled, scaled PlotPoints for 300+ FPS zero-allocation rendering!
    pub front_pts_cache: Vec<[f64; 2]>,
    pub rear_pts_cache: Vec<[f64; 2]>,
    pub rake_pts_cache: Vec<[f64; 2]>,
    pub speed_pts_cache: Vec<[f64; 2]>,

    // Basic Driver Inputs Caches
    pub throttle_pts_cache: Vec<[f64; 2]>,
    pub brake_pts_cache: Vec<[f64; 2]>,
    pub steering_pts_cache: Vec<[f64; 2]>,
    pub rpm_pts_cache: Vec<[f64; 2]>,
    pub gear_pts_cache: Vec<[f64; 2]>,

    // Precomputed Lap boundary start/end time markers (relative to stint start in seconds)
    pub lap_ranges: Vec<(i32, f64, f64)>,

    // Precomputed Lap start timestamps (relative to stint start in seconds) for dotted red line dividers
    pub lap_markers: Vec<f64>,

    // Selected active Lap
    pub selected_lap: Option<i32>,

    // MoTeC Style Locked Playback Cursor Tracker (Time in seconds)
    pub cursor_x: Option<f64>,

    // Trigger flag to reset native plot boundaries
    pub reset_bounds_flag: bool,

    // Track if click drag initiated inside the bottom time-stamp ticker zone
    pub is_dragging_ticker: bool,

    // --- MOTEC DUAL-CLICK HIGHLIGHT ZOOM STATE ---
    pub is_highlight_active: bool,
    pub highlight_start: Option<f64>,

    // --- MOTEC MULTI-LAP REFERENCE OVERLAY STATE ---
    pub ref_lap_white: Option<i32>,
    pub ref_lap_cyan: Option<i32>,

    // Shared horizontal view bounds to perfectly synchronize Zoom/Pan/Scroll across different tabs!
    pub visible_x_range: Option<(f64, f64)>,

    // Tracks worksheet changes to execute tab-sync bounds on switch frames cleanly!
    pub previous_worksheet: Option<WorksheetTab>,

    // Sector reports caches
    pub sectors: Vec<TrackSector>,
    pub sector_bests: Vec<f64>,
    pub lap_data_cache: Vec<LapData>,
    pub show_graphs_track_map: bool,
    pub previous_page: Option<ActivePage>,
    pub previous_show_graphs_track_map: Option<bool>,
    pub show_sector_deltas: bool,
    pub sector_deltas: Vec<Option<f64>>,
}

impl Default for OpenDavApp {
    fn default() -> Self {
        Self {
            app_state: AppState::Splash { progress: 0.0 },
            active_page: ActivePage::OpenDav,
            active_worksheet: WorksheetTab::Basic,
            session_loaded: false,
            active_file: None,
            session: None,
            fade_value: 0.0,
            front_pts_cache: Vec::new(),
            rear_pts_cache: Vec::new(),
            rake_pts_cache: Vec::new(),
            speed_pts_cache: Vec::new(),
            throttle_pts_cache: Vec::new(),
            brake_pts_cache: Vec::new(),
            steering_pts_cache: Vec::new(),
            rpm_pts_cache: Vec::new(),
            gear_pts_cache: Vec::new(),
            lap_ranges: Vec::new(),
            lap_markers: Vec::new(),
            selected_lap: None,
            cursor_x: None,
            reset_bounds_flag: false,
            is_dragging_ticker: false,
            is_highlight_active: false,
            highlight_start: None,
            ref_lap_white: None,
            ref_lap_cyan: None,
            visible_x_range: None,
            previous_worksheet: None,
            sectors: Vec::new(),
            sector_bests: Vec::new(),
            lap_data_cache: Vec::new(),
            show_graphs_track_map: false,
            previous_page: None,
            previous_show_graphs_track_map: None,
            show_sector_deltas: false,
            sector_deltas: Vec::new(),
        }
    }
}

impl OpenDavApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders
        egui_extras::install_image_loaders(&_cc.egui_ctx);

        let mut style = egui::Style::default();
        style.visuals = egui::Visuals::dark(); 
        
        // Brand color customizations matching requested #0A0A0A Obsidian theme
        style.visuals.selection.bg_fill = ACCENT_COLOR;
        style.visuals.window_fill = DARK_BG_COLOR;
        style.visuals.panel_fill = DARK_BG_COLOR;
        
        // Custom widget rounding and borders
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(25, 35, 38);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(12, 20, 22);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(8, 14, 16);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 6));

        _cc.egui_ctx.set_style(style);

        Self::default()
    }

    // Precomputes and caches ALL points of the entire session plotted along relative SessionTime (seconds)
    // Pre-scales and normalizes Y-axis values once on load to ensure absolute ZERO heap allocations in drawing hot-loop!
    pub fn rebuild_points_cache(&mut self) {
        if let Some(session) = &mut self.session {
            let n = session.distance.len();
            
            // Build base points
            let mut front = Vec::with_capacity(n);
            let mut rear = Vec::with_capacity(n);
            let mut rake = Vec::with_capacity(n);
            let mut speed = Vec::with_capacity(n);
            let mut throttle = Vec::with_capacity(n);
            let mut brake = Vec::with_capacity(n);
            let mut steering = Vec::with_capacity(n);
            let mut rpm = Vec::with_capacity(n);
            let mut gear = Vec::with_capacity(n);

            let time_col = session.dataframe.column("SessionTime").unwrap().f64().unwrap();
            let session_start = time_col.get(0).unwrap_or(0.0);

            // Fetch columns safely with fallback defaults to prevent schema panics!
            let speed_col = session.dataframe.column("Speed").ok().map(|c| c.f64().ok()).flatten();
            let throttle_col = session.dataframe.column("Throttle").ok().map(|c| c.f64().ok()).flatten();
            let brake_col = session.dataframe.column("Brake").ok().map(|c| c.f64().ok()).flatten();
            let steering_col = session.dataframe.column("SteeringWheelAngle").ok().map(|c| c.f64().ok()).flatten();
            let rpm_col = session.dataframe.column("RPM").ok().map(|c| c.f64().ok()).flatten();
            let gear_col = session.dataframe.column("Gear").ok().map(|c| c.f64().ok()).flatten();

            let is_on_track_col = session.dataframe.column("IsOnTrack").ok().map(|c| c.f64().ok()).flatten();
            let in_pit_stall_col = session.dataframe.column("PlayerCarInPitStall").ok().map(|c| c.f64().ok()).flatten();

            // 1. Compile entire session relative seconds [0.0 to stint duration]
            for i in 0..n {
                let is_on_track = is_on_track_col.as_ref().map(|c| c.get(i).unwrap_or(1.0)).unwrap_or(1.0);
                let in_pit_stall = in_pit_stall_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0);

                if is_on_track < 1.0 || in_pit_stall > 0.0 {
                    continue; // Skip off-track or pit stall samples cleanly!
                }

                let s_time = time_col.get(i).unwrap_or(0.0);
                let rel_time = s_time - session_start;
                
                front.push([rel_time, session.front_smooth[i]]);
                rear.push([rel_time, session.rear_smooth[i]]);
                rake.push([rel_time, session.rake[i]]);
                
                let raw_speed = speed_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 3.6; // convert m/s to km/h
                speed.push([rel_time, raw_speed]);

                let raw_thr = throttle_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 100.0; // convert 0..1 to 0..100%
                throttle.push([rel_time, raw_thr]);

                let raw_brk = brake_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 100.0; // convert 0..1 to 0..100%
                brake.push([rel_time, raw_brk]);

                let raw_steer = steering_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 57.2958; // convert rad to deg
                steering.push([rel_time, raw_steer]);

                let raw_rpm = rpm_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0);
                rpm.push([rel_time, raw_rpm]);

                let raw_gear = gear_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0);
                gear.push([rel_time, raw_gear]);
            }

            // 2. Precompute mathematical dynamic scaling factors across the whole stint to map into stacked Lanes:
            // Lane 1: Ground Speed : [76.0, 98.0]
            // Lane 2: RPM          : [52.0, 72.0]
            // Lane 3: Throttle/Brake: [28.0, 48.0]
            // Lane 4: Steering     : [10.0, 24.0]

            // Lane 1: Speed Scaling
            let min_spd = speed.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_spd = speed.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let pad_spd = (max_spd - min_spd) * 0.1;
            let scale_speed = |val: f64| -> f64 {
                if max_spd == min_spd { return 87.0; }
                let pct = ((val - (min_spd - pad_spd)) / ((max_spd + pad_spd) - (min_spd - pad_spd))).clamp(0.0, 1.0);
                76.0 + pct * (98.0 - 76.0)
            };

            // Lane 2: RPM Scaling
            let min_r = rpm.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_r = rpm.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let pad_r = (max_r - min_r) * 0.1;
            let scale_rpm = |val: f64| -> f64 {
                if max_r == min_r { return 62.0; }
                let pct = ((val - (min_r - pad_r)) / ((max_r + pad_r) - (min_r - pad_r))).clamp(0.0, 1.0);
                52.0 + pct * (72.0 - 52.0)
            };

            // Lane 3: Throttle and Brake percentage Scaling (Directly fits [28.0, 48.0])
            let scale_pct = |val: f64| -> f64 {
                28.0 + (val / 100.0).clamp(0.0, 1.0) * (48.0 - 28.0)
            };

            // Lane 4: Steering Angle Scaling
            let min_st = steering.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_st = steering.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let pad_st = (max_st - min_st) * 0.1;
            let scale_steering = |val: f64| -> f64 {
                if max_st == min_st { return 17.0; }
                let pct = ((val - (min_st - pad_st)) / ((max_st + pad_st) - (min_st - pad_st))).clamp(0.0, 1.0);
                10.0 + pct * (24.0 - 10.0)
            };

            // Master Scaling for Dynamic Rake (middle and bottom lanes in Tab 2)
            let min_front = front.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_front = front.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let min_rear = rear.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_rear = rear.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            
            let min_rh = f64::min(min_front, min_rear);
            let max_rh = f64::max(max_front, max_rear);
            let pad_rh = (max_rh - min_rh) * 0.1;

            let scale_rh = |val: f64| -> f64 {
                if max_rh == min_rh { return 53.0; }
                let pct = ((val - (min_rh - pad_rh)) / ((max_rh + pad_rh) - (min_rh - pad_rh))).clamp(0.0, 1.0);
                40.0 + pct * (66.0 - 40.0)
            };

            let min_rake_val = rake.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_rake_val = rake.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let pad_rk = (max_rake_val - min_rake_val) * 0.1;

            let scale_rake = |val: f64| -> f64 {
                if max_rake_val == min_rake_val { return 24.0; }
                let pct = ((val - (min_rake_val - pad_rk)) / ((max_rake_val + pad_rk) - (min_rake_val - pad_rk))).clamp(0.0, 1.0);
                12.0 + pct * (36.0 - 12.0)
            };

            // Scale and store normalized curves directly in place inside cache!
            let cached_len = front.len();
            for i in 0..cached_len {
                front[i][1] = scale_rh(front[i][1]);
                rear[i][1] = scale_rh(rear[i][1]);
                rake[i][1] = scale_rake(rake[i][1]);
                
                speed[i][1] = scale_speed(speed[i][1]);
                throttle[i][1] = scale_pct(throttle[i][1]);
                brake[i][1] = scale_pct(brake[i][1]);
                steering[i][1] = scale_steering(steering[i][1]);
                rpm[i][1] = scale_rpm(rpm[i][1]);
                gear[i][1] = scale_speed(gear[i][1]); // Gear utilizes Speed lane vertically

            }

            self.front_pts_cache = front;
            self.rear_pts_cache = rear;
            self.rake_pts_cache = rake;
            self.speed_pts_cache = speed;
            self.throttle_pts_cache = throttle;
            self.brake_pts_cache = brake;
            self.steering_pts_cache = steering;
            self.rpm_pts_cache = rpm;
            self.gear_pts_cache = gear;

            // 3. Precompute Lap Start/End Boundaries based on actual parsed lap numbers
            let mut markers = Vec::new();
            let mut ranges = Vec::new();

            let df = &session.dataframe;
            let lap_col = df.column("Lap").unwrap().f64().unwrap();
            let time_col = df.column("SessionTime").unwrap().f64().unwrap();
            let session_start = time_col.get(0).unwrap_or(0.0);

            for &(lap_num, _duration) in &session.lap_times {
                let mut start_idx = None;
                let mut end_idx = None;
                for i in 0..n {
                    if lap_col.get(i).unwrap_or(0.0) as i32 == lap_num {
                        if start_idx.is_none() {
                            start_idx = Some(i);
                        }
                        end_idx = Some(i);
                    }
                }
                if let (Some(s_idx), Some(e_idx)) = (start_idx, end_idx) {
                    let start_t = time_col.get(s_idx).unwrap_or(0.0) - session_start;
                    let end_t = time_col.get(e_idx).unwrap_or(0.0) - session_start;
                    ranges.push((lap_num, start_t, end_t));
                    markers.push(start_t);
                }
            }

            if ranges.is_empty() && !self.front_pts_cache.is_empty() {
                let end_stint = self.front_pts_cache.last().unwrap()[0];
                ranges.push((1, 0.0, end_stint));
                markers.push(0.0);
            }

            self.lap_markers = markers.clone();
            self.lap_ranges = ranges;

            // Harmonize and write the physical transition lap list back to the session struct to maintain absolute app coherence!
            let mut sync_laps = Vec::new();
            for &(lap_num, start_t, end_t) in &self.lap_ranges {
                let duration = end_t - start_t;
                if duration > 1.0 {
                    sync_laps.push((lap_num, duration));
                }
            }
            session.lap_times = sync_laps;

            // Default locked cursor to the fastest lap start on load!
            if !self.front_pts_cache.is_empty() {
                let max_time = self.front_pts_cache.last().unwrap()[0];
                if let Some(sel_lap) = self.selected_lap {
                    if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == sel_lap) {
                        let (_, start_t, end_t) = self.lap_ranges[pos];
                        self.cursor_x = Some(start_t);
                        self.visible_x_range = Some((start_t, end_t));
                    } else {
                        self.cursor_x = Some(0.0);
                        self.visible_x_range = Some((0.0, max_time));
                    }
                } else {
                    self.cursor_x = Some(0.0);
                    self.visible_x_range = Some((0.0, max_time));
                }
                self.reset_bounds_flag = true;
            }

            // Rebuild lap data cache
            let mut data_cache = Vec::new();
            let mut unique_laps = Vec::new();
            for &(lap_num, _, _) in &self.lap_ranges {
                unique_laps.push(lap_num);
            }

            let df = &session.dataframe;
            let lap_col = df.column("Lap").unwrap().f64().unwrap();
            let dist_col = df.column("Distance_Derived").unwrap().f64().unwrap();
            let time_col = df.column("SessionTime").unwrap().f64().unwrap();
            let lat_col = df.column("Lat").ok().and_then(|c| c.f64().ok());
            let lon_col = df.column("Lon").ok().and_then(|c| c.f64().ok());

            let mut lat0 = 0.0;
            let mut lon0 = 0.0;
            if let (Some(la), Some(lo)) = (lat_col.as_ref(), lon_col.as_ref()) {
                lat0 = la.get(0).unwrap_or(0.0);
                lon0 = lo.get(0).unwrap_or(0.0);
            }

            let r_earth = 6378137.0; // Earth radius in meters
            let lat0_rad = lat0 * std::f64::consts::PI / 180.0;
            let lon0_rad = lon0 * std::f64::consts::PI / 180.0;

            for &l_num in &unique_laps {
                let mut dists = Vec::new();
                let mut times = Vec::new();
                let mut xs = Vec::new();
                let mut ys = Vec::new();
                for i in 0..n {
                    if lap_col.get(i).unwrap_or(0.0) as i32 == l_num {
                        dists.push(dist_col.get(i).unwrap_or(0.0));
                        times.push(time_col.get(i).unwrap_or(0.0));
                        
                        let lat = lat_col.as_ref().and_then(|c| c.get(i)).unwrap_or(0.0);
                        let lon = lon_col.as_ref().and_then(|c| c.get(i)).unwrap_or(0.0);
                        
                        let lat_rad = lat * std::f64::consts::PI / 180.0;
                        let lon_rad = lon * std::f64::consts::PI / 180.0;
                        
                        let x = r_earth * (lon_rad - lon0_rad) * lat0_rad.cos();
                        let y = r_earth * (lat_rad - lat0_rad);
                        xs.push(x);
                        ys.push(y);
                    }
                }
                if !dists.is_empty() {
                    let base_dist = dists[0];
                    let base_time = times[0];
                    for d in &mut dists { *d -= base_dist; }
                    for t in &mut times { *t -= base_time; }
                    data_cache.push(LapData {
                        lap_num: l_num,
                        dist: dists,
                        time: times,
                        x: xs,
                        y: ys,
                    });
                }
            }
            self.lap_data_cache = data_cache;

            // Rebuild track sectors and sector bests cache using Signals Layer
            self.sectors = detect_track_sectors(session);
            
            let mut bests = vec![f64::MAX; self.sectors.len()];
            for (s_idx, sector) in self.sectors.iter().enumerate() {
                for lap in &self.lap_data_cache {
                    if lap.lap_num > 3 {
                        let t_start = get_lap_time_at_distance(&lap.dist, &lap.time, sector.start_dist);
                        let t_end = get_lap_time_at_distance(&lap.dist, &lap.time, sector.end_dist);
                        let s_time = t_end - t_start;
                        if s_time > 0.0 && s_time < bests[s_idx] {
                            bests[s_idx] = s_time;
                        }
                    }
                }
                if bests[s_idx] == f64::MAX {
                    for lap in &self.lap_data_cache {
                        let t_start = get_lap_time_at_distance(&lap.dist, &lap.time, sector.start_dist);
                        let t_end = get_lap_time_at_distance(&lap.dist, &lap.time, sector.end_dist);
                        let s_time = t_end - t_start;
                        if s_time > 0.0 && s_time < bests[s_idx] {
                            bests[s_idx] = s_time;
                        }
                    }
                }
            }
            self.sector_bests = bests;
            self.update_sector_deltas();
        }
    }

    pub fn update_sector_deltas(&mut self) {
        let ref_lap = self.ref_lap_cyan.or(self.ref_lap_white);
        let active_lap_num = self.selected_lap.or_else(|| {
            if let Some(session) = &self.session {
                Some(crate::signals::processing::get_fastest_lap(&session.lap_times))
            } else {
                None
            }
        });
        self.sector_deltas = crate::signals::processing::recalculate_sector_deltas(
            &self.lap_data_cache,
            &self.sectors,
            active_lap_num,
            ref_lap,
        );
    }
}

impl eframe::App for OpenDavApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- DYNAMIC BRAND THEMING SWITCHER ---
        let mut style = (*ctx.style()).clone();
        let is_dark = style.visuals.dark_mode;
        
        if is_dark {
            style.visuals.window_fill = DARK_BG_COLOR;
            style.visuals.panel_fill = DARK_BG_COLOR;
            style.visuals.selection.bg_fill = ACCENT_COLOR;
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(20, 20, 20);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(30, 30, 30);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(15, 15, 15);
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8));
        } else {
            style.visuals.window_fill = LIGHT_BG_COLOR;
            style.visuals.panel_fill = LIGHT_BG_COLOR;
            style.visuals.selection.bg_fill = ACCENT_COLOR;
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(215, 214, 213);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(240, 239, 238);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(235, 234, 233);
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 15));
        }
        ctx.set_style(style);

        match self.app_state {
            AppState::Splash { .. } => {
                ctx.request_repaint();
                let mut finished = false;
                let mut current_progress = 0.0;
                if let AppState::Splash { ref mut progress } = self.app_state {
                    *progress += 0.0035;
                    current_progress = *progress;
                    if *progress >= 1.0 {
                        finished = true;
                    }
                }
                if finished {
                    self.app_state = AppState::Main;
                } else {
                    self.draw_splash_screen(ctx, current_progress);
                }
            }
            AppState::Main => {
                if self.fade_value < 1.0 {
                    ctx.request_repaint();
                    self.fade_value += 0.02;
                }

                self.draw_sidebar(ctx);
                self.draw_top_panel(ctx);

                // --- CENTRAL PAGE RENDERER ---
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_opacity(self.fade_value);

                    match self.active_page {
                        ActivePage::OpenDav => {
                            self.draw_dashboard_page(ui, is_dark);
                        }
                        ActivePage::Graphs => {
                            self.draw_graphs_page(ui);
                        }
                        ActivePage::Reports => {
                            self.draw_reports_page(ui, is_dark);
                        }
                    }
                });
            }
        }
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

fn main() -> eframe::Result<()> {
    // Load window icon from assets
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon_data = if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1150.0, 720.0]) 
        .with_min_inner_size([800.0, 500.0])
        .with_title("OpenDav");

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "OpenDav",
        native_options,
        Box::new(|cc| Ok(Box::new(OpenDavApp::new(cc)))),
    )
}
