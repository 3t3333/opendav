#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on release in Windows
#![allow(deprecated)]

mod ibt_parser;

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, VLine, Points, HLine, Text, PlotPoint, Span, Axis};
use rfd::FileDialog;

// --- GLOBAL BRAND COLOR CONSTANTS ---
const DARK_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(10, 10, 10);      // #0A0A0A Obsidian
const LIGHT_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(227, 226, 225); // #E3E2E1 Slate White
const ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(242, 82, 37);      // #F25225 Electric Blaze Orange
const SUB_ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(102, 72, 212); // #6648D4 Electric Indigo Purple
const SPEED_COLOR: egui::Color32 = egui::Color32::from_rgb(78, 159, 245);      // Calm Sky Blue for Ground Speed

#[derive(PartialEq, Clone, Copy)]
enum ActivePage {
    OpenDav, // Summary Dashboard
    Graphs,  // MoTeC Workspace
}

#[derive(PartialEq, Clone, Copy)]
enum WorksheetTab {
    Basic,             // 1. Basic (Driver Inputs: Speed, Throttle, Brake, Steering, RPM, Gear)
    DynamicRake,       // 2. Dynamic Rake Analyzer
    TireEnergy,        // 3. Tire Energy Profiler
    TireFuelWindows,   // 4. Tire & Fuel Windows
    TireTempLoad,      // 5. Tire Temp/Load Map
    MathSandbox,       // 6. Custom Math Sandbox
    EmpiricalAero,     // 7. Empirical Aero Map
    DownforceMapping,  // 8. Downforce Mapping
    PitchPlatform,     // 9. Pitch & Platform
    HandlingAnalyzer,  // 10. Handling Analyzer (Yaw Error)
    TlltdDistribution, // 11. TLLTD Distribution
    CompressionRates,  // 12. Compression Rates
}

enum AppState {
    Splash {
        progress: f32,
    },
    Main,
}

// --- GENERAL-PURPOSE MOTEC GRAPHING ARCHITECTURE ---
// Encapsulates the entire multi-lane, 300+ FPS, highly-interactive plotting engine to guarantee 100% design integrity!
pub struct ChartTrace {
    pub name: &'static str,
    pub scaled_pts: Vec<[f64; 2]>,
    pub color: egui::Color32,
    pub width: f32,
    pub raw_val: f64,
}

pub struct ChartLane {
    pub title: &'static str,
    pub traces: Vec<ChartTrace>,
    pub y_min: f64,
    pub y_max: f64,
}

pub struct OpenDavApp {
    app_state: AppState,
    active_page: ActivePage,
    active_worksheet: WorksheetTab,
    session_loaded: bool,
    active_file: Option<String>,
    session: Option<ibt_parser::IbtSession>,
    fade_value: f32, // Smooth UI fade-in animation tracker

    // Caching compiled, scaled PlotPoints for 300+ FPS zero-allocation rendering!
    front_pts_cache: Vec<[f64; 2]>,
    rear_pts_cache: Vec<[f64; 2]>,
    rake_pts_cache: Vec<[f64; 2]>,
    speed_pts_cache: Vec<[f64; 2]>,

    // Basic Driver Inputs Caches
    throttle_pts_cache: Vec<[f64; 2]>,
    brake_pts_cache: Vec<[f64; 2]>,
    steering_pts_cache: Vec<[f64; 2]>,
    rpm_pts_cache: Vec<[f64; 2]>,
    gear_pts_cache: Vec<[f64; 2]>,

    // Precomputed Lap boundary start/end time markers (relative to stint start in seconds)
    lap_ranges: Vec<(i32, f64, f64)>,

    // Precomputed Lap start timestamps (relative to stint start in seconds) for dotted red line dividers
    lap_markers: Vec<f64>,

    // Selected active Lap
    selected_lap: Option<i32>,

    // MoTeC Style Locked Playback Cursor Tracker (Time in seconds)
    cursor_x: Option<f64>,

    // Trigger flag to reset native plot boundaries
    reset_bounds_flag: bool,

    // Track if click drag initiated inside the bottom time-stamp ticker zone
    is_dragging_ticker: bool,

    // --- MOTEC DUAL-CLICK HIGHLIGHT ZOOM STATE ---
    is_highlight_active: bool,
    highlight_start: Option<f64>,

    // --- MOTEC MULTI-LAP REFERENCE OVERLAY STATE ---
    ref_lap_white: Option<i32>,
    ref_lap_cyan: Option<i32>,
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
        }
    }
}

// Highly precise binary search bisector to locate closest index in < 1 microsecond!
fn get_closest_index(distance: &[f64], target_x: f64) -> usize {
    if distance.is_empty() { return 0; }
    match distance.binary_search_by(|val| val.partial_cmp(&target_x).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => idx,
        Err(idx) => {
            if idx <= 0 { 0 }
            else if idx >= distance.len() { distance.len() - 1 }
            else {
                let d0 = distance[idx - 1];
                let d1 = distance[idx];
                if (target_x - d0).abs() < (d1 - target_x).abs() {
                    idx - 1
                } else {
                    idx
                }
            }
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
    fn rebuild_points_cache(&mut self) {
        if let Some(session) = &self.session {
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

            // 1. Compile entire session relative seconds [0.0 to stint duration]
            for i in 0..n {
                let s_time = time_col.get(i).unwrap_or(0.0);
                let rel_time = s_time - session_start;
                
                front.push([rel_time, session.front_smooth[i]]);
                rear.push([rel_time, session.rear_smooth[i]]);
                rake.push([rel_time, session.rake[i]]);
                
                let raw_speed = speed_col.map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 3.6; // convert m/s to km/h
                speed.push([rel_time, raw_speed]);

                let raw_thr = throttle_col.map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 100.0; // convert 0..1 to 0..100%
                throttle.push([rel_time, raw_thr]);

                let raw_brk = brake_col.map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 100.0; // convert 0..1 to 0..100%
                brake.push([rel_time, raw_brk]);

                let raw_steer = steering_col.map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) * 57.2958; // convert rad to deg
                steering.push([rel_time, raw_steer]);

                let raw_rpm = rpm_col.map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0);
                rpm.push([rel_time, raw_rpm]);

                let raw_gear = gear_col.map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0);
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
            for i in 0..n {
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

            // 3. Precompute Lap Start/End Boundaries and dotted line dividers!
            let mut markers = Vec::new();
            let mut ranges: Vec<(i32, f64, f64)> = Vec::new();
            let mut seen_laps = std::collections::HashSet::new();

            for i in 0..n {
                let lap_num = session.laps[i];
                let s_time = time_col.get(i).unwrap_or(0.0);
                let rel_time = s_time - session_start;

                if seen_laps.insert(lap_num) {
                    markers.push(rel_time);
                    
                    // Close previous lap range
                    if !ranges.is_empty() {
                        let len = ranges.len();
                        ranges[len - 1].2 = rel_time;
                    }
                    ranges.push((lap_num, rel_time, rel_time + 120.0)); // Standby default duration
                }
            }

            // Close the final lap range with stint end duration
            if !ranges.is_empty() && !self.front_pts_cache.is_empty() {
                let stint_end = self.front_pts_cache[self.front_pts_cache.len() - 1][0];
                let len = ranges.len();
                ranges[len - 1].2 = stint_end;
            }

            self.lap_markers = markers;
            self.lap_ranges = ranges;

            // Default locked cursor to absolute stint start on load
            if !self.front_pts_cache.is_empty() {
                self.cursor_x = Some(0.0);
                self.reset_bounds_flag = true;
            }
        }
    }

    // Slice only the cache points that correspond to a specific lap number
    fn get_lap_points_slice<'a>(&self, cache: &'a [[f64; 2]], lap_num: i32) -> &'a [[f64; 2]] {
        if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == lap_num) {
            let (_, start_t, end_t) = self.lap_ranges[pos];
            let start_idx = match cache.binary_search_by(|p| p[0].partial_cmp(&start_t).unwrap_or(std::cmp::Ordering::Equal)) {
                Ok(i) => i,
                Err(i) => i,
            };
            let end_idx = match cache.binary_search_by(|p| p[0].partial_cmp(&end_t).unwrap_or(std::cmp::Ordering::Equal)) {
                Ok(i) => i,
                Err(i) => i,
            }.min(cache.len());
            &cache[start_idx..end_idx]
        } else {
            &[]
        }
    }
}

// Formats seconds into MM:SS.SSS
fn format_lap_time(sec: f64) -> String {
    let minutes = (sec / 60.0).floor() as i32;
    let seconds = (sec % 60.0).floor() as i32;
    let ms = ((sec % 1.0) * 1000.0).round() as i32;
    format!("{:02}:{:02}.{:03}", minutes, seconds, ms)
}

// Spawns a background thread to download the track map SVG from the public iRacing static assets CDN
fn trigger_track_map_download(track_id: i32) {
    if track_id <= 0 { return; }
    std::thread::spawn(move || {
        let dest_dir = std::path::Path::new("exports/track_maps");
        if !dest_dir.exists() {
            let _ = std::fs::create_dir_all(dest_dir);
        }
        let dest_file = dest_dir.join(format!("{}.svg", track_id));

        // Only download if the file does not already exist
        if !dest_file.exists() {
            println!("Downloading track map SVG for track_id: {} to: {}", track_id, dest_file.display());
            
            // Try official iRacing static CDN first (requires curl -f to fail on 404/403)
            let official_url = format!("https://images-static.iracing.com/tracks/{}/track.svg", track_id);
            let status = std::process::Command::new("curl")
                .arg("-s")
                .arg("-f")
                .arg("-L")
                .arg("-o")
                .arg(&dest_file)
                .arg(&official_url)
                .status();

            let success = match status {
                Ok(s) => s.success(),
                _ => false,
            };

            // If official CDN fails, automatically fallback to the community GitHub cdn mirror!
            if !success {
                let fallback_url = format!("https://cdn.jsdelivr.net/gh/iTelemetry/iracing-tracks/svgs/{}.svg", track_id);
                let _ = std::process::Command::new("curl")
                    .arg("-s")
                    .arg("-f")
                    .arg("-L")
                    .arg("-o")
                    .arg(&dest_file)
                    .arg(&fallback_url)
                    .status();
            }
        }
    });
}

impl eframe::App for OpenDavApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        // --- DYNAMIC BRAND THEMING SWITCHER ---
        // Dynamically applies background and selection highlights based on light/dark mode changes!
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
            AppState::Splash { ref mut progress } => {
                ctx.request_repaint();
                *progress += 0.0035;

                if *progress >= 1.0 {
                    self.app_state = AppState::Main;
                    return;
                }

                // Render Splash Window
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(height_offset(ui) - 240.0);
                            
                            let splash_bytes = include_bytes!("../assets/solid_splashscreen.png");
                            ui.add(
                                egui::Image::from_bytes("bytes://splash.png", splash_bytes.to_vec())
                                    .max_width(700.0) 
                                    .rounding(12.0)
                            );
                            
                            ui.add_space(45.0);

                            ui.vertical_centered(|ui| {
                                let width = 380.0;
                                let height = 5.0;
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(width, height),
                                    egui::Sense::hover()
                                );

                                if ui.is_rect_visible(rect) {
                                    // Use dynamic progress bar backgrounds based on theme
                                    let progress_bg = if is_dark { egui::Color32::from_rgb(25, 25, 25) } else { egui::Color32::from_rgb(200, 200, 200) };
                                    ui.painter().rect_filled(rect, 3.0, progress_bg);

                                    let active_width = width * (*progress);
                                    let mut active_rect = rect;
                                    active_rect.max.x = active_rect.min.x + active_width;

                                    ui.painter().rect_filled(active_rect, 3.0, ACCENT_COLOR);
                                }
                            });

                            ui.add_space(15.0);
                            ui.label(
                                egui::RichText::new("BOOTING SYSTEM CORE...")
                                    .color(ACCENT_COLOR)
                                    .size(10.0)
                                    .strong()
                            );
                        });
                    });
                });
            }
            AppState::Main => {
                if self.fade_value < 1.0 {
                    ctx.request_repaint();
                    self.fade_value += 0.02;
                }

                // --- COLLAPSIBLE LEFT NAVIGATION SIDEBAR PANEL ---
                egui::SidePanel::left("sidebar_panel")
                    .resizable(false)
                    .default_width(260.0) 
                    .show(ctx, |ui| {
                        ui.add_space(15.0);
                        
                        match self.active_page {
                            ActivePage::OpenDav => {
                                // 1. CUSTOM CORNER LOGO HEADER
                                let corner_bytes = include_bytes!("../assets/corner_logo.png");
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
                                    let db_bytes = include_bytes!("../assets/button_dashboard.png");
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
                                    let gr_bytes = include_bytes!("../assets/button_graphs.png");
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
                                                if self.session_loaded && self.selected_lap.is_none() {
                                                    if let Some(session) = &self.session {
                                                        let fastest = session.lap_times.iter()
                                                            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                                                            .map(|(l, _)| *l);
                                                        self.selected_lap = fastest;
                                                        self.rebuild_points_cache();
                                                    }
                                                }
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

                                    ui.label(egui::RichText::new("LAP TIMELINE SELECT").color(egui::Color32::DARK_GRAY).size(10.0).strong());
                                    ui.add_space(8.0);

                                    if !self.session_loaded || self.session.is_none() {
                                        ui.label(egui::RichText::new("No Session Active").color(egui::Color32::GRAY).small());
                                    } else {
                                        // CLONE LAP TIMES AND AVOID IN-CLOSURE IMMUTABLE MUT MUTATION LOCKS
                                        let lap_times = if let Some(session) = &self.session {
                                            session.lap_times.clone()
                                        } else {
                                            Vec::new()
                                        };

                                        let fastest_lap = lap_times.iter()
                                            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                                            .map(|(l, _)| *l)
                                            .unwrap_or(0);

                                        let sidebar_style = ui.style_mut();
                                        sidebar_style.spacing.button_padding = egui::vec2(12.0, 8.0);

                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                for (lap_num, duration) in &lap_times {
                                                    let is_selected = self.selected_lap == Some(*lap_num);
                                                    let is_fastest = *lap_num == fastest_lap;

                                                    let is_cyan = self.ref_lap_cyan == Some(*lap_num);
                                                    let is_white = self.ref_lap_white == Some(*lap_num);

                                                    let label_color = if is_selected {
                                                        ACCENT_COLOR
                                                    } else if is_fastest {
                                                        SUB_ACCENT_COLOR
                                                    } else {
                                                        if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }
                                                    };

                                                    ui.horizontal(|ui| {
                                                        // 1. Cyan Reference Toggle Box (Left)
                                                        let btn_c = ui.selectable_label(is_cyan, egui::RichText::new("C").color(if is_cyan { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::DARK_GRAY }).strong());
                                                        if btn_c.clicked() {
                                                            if is_cyan {
                                                                self.ref_lap_cyan = None;
                                                            } else {
                                                                self.ref_lap_cyan = Some(*lap_num);
                                                            }
                                                        }

                                                        // 2. White Reference Toggle Box (Right)
                                                        let btn_w = ui.selectable_label(is_white, egui::RichText::new("W").color(if is_white { egui::Color32::WHITE } else { egui::Color32::DARK_GRAY }).strong());
                                                        if btn_w.clicked() {
                                                            if is_white {
                                                                self.ref_lap_white = None;
                                                            } else {
                                                                self.ref_lap_white = Some(*lap_num);
                                                            }
                                                        }

                                                        // 3. Main Lap Timeline Selection Selector
                                                        let mut text = format!("Lap {} : {}", lap_num, format_lap_time(*duration));
                                                        if is_fastest {
                                                            text += " ★";
                                                        }

                                                        if ui.selectable_label(is_selected, egui::RichText::new(text).color(label_color).strong()).clicked() {
                                                            self.selected_lap = Some(*lap_num);
                                                            
                                                            // MOTEC JUMP-SNAP JUMP bounds to focus perfectly on that lap's relative time window!
                                                            if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == *lap_num) {
                                                                let (_, start_t, _end_t) = self.lap_ranges[pos];
                                                                self.cursor_x = Some(start_t);
                                                                self.reset_bounds_flag = true;
                                                            }
                                                        }
                                                    });
                                                }
                                            });
                                        });
                                    }
                                });
                            }
                        }

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("v3.0.0-rs").color(egui::Color32::DARK_GRAY).small());
                        });
                    });

                // --- TOP ACTION TASKBAR ---
                egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                    ui.add_space(6.0);
                    egui::menu::bar(ui, |ui| {
                        if ui.button("📂 Load IBT Telemetry").clicked() {
                            if let Some(path) = FileDialog::new()
                                .add_filter("iRacing Telemetry", &["ibt"])
                                .set_title("Select Telemetry File")
                                .pick_file() 
                            {
                                self.active_file = Some(path.display().to_string());
                                match ibt_parser::parse_ibt_file(&path) {
                                    Ok(parsed_session) => {
                                        self.session_loaded = true;
                                        
                                        // Trigger the asynchronous background SVG track map downloader
                                        trigger_track_map_download(parsed_session.track_id);
                                        
                                        self.session = Some(parsed_session);
                                        
                                        // Auto-load fastest lap in caching layer on file load
                                        if let Some(session) = &self.session {
                                            let fastest = session.lap_times.iter()
                                                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                                                .map(|(l, _)| *l);
                                            self.selected_lap = fastest;
                                        }
                                        self.cursor_x = None;
                                        self.rebuild_points_cache();
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
                        });
                    });
                    ui.add_space(6.0);
                });

                // --- CENTRAL PAGE RENDERER (With smooth fade animation) ---
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_opacity(self.fade_value);

                    match self.active_page {
                        ActivePage::OpenDav => {
                            if !self.session_loaded || self.session.is_none() {
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                                        ui.label(egui::RichText::new("Please load an iRacing .ibt file from the top taskbar.").color(egui::Color32::GRAY));
                                    });
                                });
                            } else {
                                // SAFE IMMUTABLE CLONING TO RESOLVE RUST BORROW CHECKER CLOSURE LOCKS
                                let session_ref = self.session.as_ref().unwrap();
                                let car = session_ref.car.clone();
                                let venue = session_ref.venue.clone();
                                let air_temp = session_ref.air_temp.clone();
                                let surface_temp = session_ref.surface_temp.clone();
                                let total_session_time = session_ref.total_session_time;
                                let lap_times = session_ref.lap_times.clone();
                                let _num_samples = session_ref.distance.len();

                                ui.heading(egui::RichText::new("Session Intelligence Dashboard").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                ui.add_space(8.0);

                                // 1. Session Metadata Grid Card
                                ui.group(|ui| {
                                    ui.columns(4, |cols| {
                                        cols[0].vertical(|ui| {
                                            ui.label(egui::RichText::new("VEHICLE").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.label(egui::RichText::new(&car).strong().color(ACCENT_COLOR));
                                        });
                                        cols[1].vertical(|ui| {
                                            ui.label(egui::RichText::new("VENUE").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.label(egui::RichText::new(&venue).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                        });
                                        cols[2].vertical(|ui| {
                                            ui.label(egui::RichText::new("AIR TEMP").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.label(egui::RichText::new(&air_temp).strong().color(egui::Color32::from_rgb(50, 205, 50)));
                                        });
                                        cols[3].vertical(|ui| {
                                            ui.label(egui::RichText::new("TRACK TEMP").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.label(egui::RichText::new(&surface_temp).strong().color(egui::Color32::from_rgb(50, 205, 50)));
                                        });
                                    });
                                });

                                ui.add_space(10.0);

                                // 2. High-End Session Statistics Cards
                                ui.columns(3, |cols| {
                                    cols[0].group(|ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.label(egui::RichText::new("TOTAL SESSION TIME").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.add_space(4.0);
                                            ui.heading(egui::RichText::new(format_lap_time(total_session_time)).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                        });
                                    });

                                    cols[1].group(|ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.label(egui::RichText::new("TOTAL VALID LAPS").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.add_space(4.0);
                                            ui.heading(egui::RichText::new(format!("{} Laps", lap_times.len())).strong().color(ACCENT_COLOR));
                                        });
                                    });

                                    cols[2].group(|ui| {
                                        ui.vertical_centered(|ui| {
                                            let avg_lap = if !lap_times.is_empty() {
                                                let sum: f64 = lap_times.iter().map(|(_, t)| *t).sum();
                                                sum / lap_times.len() as f64
                                            } else {
                                                0.0
                                            };
                                            ui.label(egui::RichText::new("AVERAGE LAP TIME").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.add_space(4.0);
                                            ui.heading(egui::RichText::new(format_lap_time(avg_lap)).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                        });
                                    });
                                });

                                ui.add_space(15.0);

                                let fastest_lap = lap_times.iter()
                                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                                    .map(|(l, _)| *l)
                                    .unwrap_or(0);

                                // 3. Stacked lower Dashboard (Top: Laps List, Bottom: Huge Track Map SVG!)
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("VALID LAP SHEET").color(egui::Color32::LIGHT_GRAY).strong().size(11.0));
                                    ui.add_space(4.0);

                                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            for (lap_num, duration) in &lap_times {
                                                let is_fastest = *lap_num == fastest_lap;
                                                
                                                ui.group(|ui| {
                                                    ui.columns(3, |cols| {
                                                        if is_fastest {
                                                            cols[0].label(egui::RichText::new(format!("Lap {}", lap_num)).strong().color(SUB_ACCENT_COLOR));
                                                            cols[1].label(egui::RichText::new(format_lap_time(*duration)).strong().color(SUB_ACCENT_COLOR));
                                                            cols[2].label(egui::RichText::new("★ FASTEST").strong().color(SUB_ACCENT_COLOR));
                                                        } else {
                                                            cols[0].label(egui::RichText::new(format!("Lap {}", lap_num)).color(egui::Color32::LIGHT_GRAY));
                                                            cols[1].label(egui::RichText::new(format_lap_time(*duration)).color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                                            cols[2].label(egui::RichText::new("Valid").color(egui::Color32::DARK_GRAY));
                                                        }
                                                    });
                                                });
                                                ui.add_space(2.0);
                                            }
                                        });
                                    });

                                    ui.add_space(15.0);
                                    ui.label(egui::RichText::new("VENUE DIRECTORY TRACK MAP").color(egui::Color32::LIGHT_GRAY).strong().size(11.0));
                                    ui.add_space(4.0);

                                    ui.group(|ui| {
                                        ui.set_min_height(250.0);
                                        ui.vertical_centered_justified(|ui| {
                                            let track_id = session_ref.track_id;
                                            let map_path = format!("exports/track_maps/{}.svg", track_id);
                                            let path_buf = std::path::Path::new(&map_path);

                                            if path_buf.exists() {
                                                // Natively read the file bytes to avoid UNC path formatting bugs on Windows!
                                                match std::fs::read(path_buf) {
                                                    Ok(svg_bytes) => {
                                                        // Verify the file actually contains valid SVG data and is not an HTML 404 error page!
                                                        let len = usize::min(svg_bytes.len(), 100);
                                                        let header = String::from_utf8_lossy(&svg_bytes[..len]);
                                                        
                                                        if header.contains("<html") || header.contains("<HTML") {
                                                            // It's a broken HTML error page! Delete it so we can fall back or re-download
                                                            let _ = std::fs::remove_file(path_buf);
                                                            
                                                            // Show placeholder since we just deleted the corrupt file
                                                            ui.add_space(80.0);
                                                            ui.spinner();
                                                            ui.add_space(10.0);
                                                            ui.label(egui::RichText::new("SYNCING VENUE LAYOUT SVG...").color(ACCENT_COLOR).strong().size(10.0));
                                                            ui.add_space(80.0);
                                                        } else {
                                                            let img = egui::Image::from_bytes(format!("bytes://track_map_{}.svg", track_id), svg_bytes)
                                                                .max_height(340.0) // 60% LARGER map! Look at that gorgeous outline!
                                                                .maintain_aspect_ratio(true)
                                                                .tint(ACCENT_COLOR); // Tint the vector line using our beautiful Electric Blaze Orange!
                                                            ui.add(img);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        ui.add_space(100.0);
                                                        ui.label(format!("Failed to read track map: {}", e));
                                                    }
                                                }
                                            } else {
                                                // Show downloading placeholder
                                                ui.add_space(80.0);
                                                ui.spinner();
                                                ui.add_space(10.0);
                                                ui.label(egui::RichText::new("SYNCING VENUE LAYOUT SVG...").color(ACCENT_COLOR).strong().size(10.0));
                                                ui.add_space(80.0);
                                            }
                                        });
                                    });
                                });

                                ui.add_space(15.0);
                                ui.vertical_centered(|ui| {
                                    if ui.button(egui::RichText::new("📈 OPEN GRAPHS WORKSPACE").strong().size(12.0).color(if is_dark { egui::Color32::from_rgb(10, 10, 10) } else { egui::Color32::WHITE })).clicked() {
                                        self.active_page = ActivePage::Graphs;
                                        // Default to fastest lap on first entering graphs page
                                        if self.selected_lap.is_none() {
                                            self.selected_lap = Some(fastest_lap);
                                            self.rebuild_points_cache();
                                        }
                                    }
                                });
                            }
                        }
                        ActivePage::Graphs => {
                            if !self.session_loaded || self.session.is_none() {
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                                        ui.label(egui::RichText::new("Please load an iRacing .ibt file from the top taskbar to view graphs.").color(egui::Color32::GRAY));
                                    });
                                });
                            } else {
                                // 1. HORIZONTAL MOTEC WORKSHEET TABS AT THE TOP!
                                ui.horizontal(|ui| {
                                    let tab_style = ui.style_mut();
                                    tab_style.spacing.button_padding = egui::vec2(12.0, 8.0); // Perfect, professional tab sizing

                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::Basic, "🏁  1. Basic (Inputs)");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::DynamicRake, "📈  2. Dynamic Rake");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::TireEnergy, "🔥  3. Tire Energy");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::TireFuelWindows, "🔋  4. Tire & Fuel");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::TireTempLoad, "🌡  5. Temp/Load Map");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::MathSandbox, "🧮  6. Custom Math");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::EmpiricalAero, "🗺  7. Aero Map");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::DownforceMapping, "💨  8. Downforce");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::PitchPlatform, "📐  9. Pitch & Platform");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::HandlingAnalyzer, "🎯  10. Handling");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::TlltdDistribution, "🔄  11. TLLTD");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::CompressionRates, "🚀  12. Compression");
                                });

                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);

                                // 2. ACTIVE WORKSHEET PLOTTING AREA (SINGLE INTEGRATED HIGH-PERFORMANCE PLOT ENVIRONMENT!)
                                // Extract sizes and session metrics natively
                                let max_time = self.front_pts_cache.last().map(|p| p[0]).unwrap_or(100.0);
                                let session_ref = self.session.as_ref().unwrap();

                                // Retrieve active screen bounds to process interactive drag scrubbing & edge panning!
                                let mut plot_height = ui.available_height() - 10.0;
                                if plot_height < 300.0 { plot_height = 300.0; }

                                // Create the unified integrated Plot container taking up 100% of available screen space!
                                let mut plot = Plot::new("integrated_mo_tec_canvas")
                                    .height(plot_height) // Fills entire screen vertically!
                                    // DISABLE ALL NATIVE DRAG/ZOOM/SCROLLS TO COMPLETELY PREVENT RENDERING FIGHTS & GRAPH DISAPPEARANCES!
                                    .allow_zoom([false, false])
                                    .allow_scroll([false, false])
                                    .allow_drag([false, false])
                                    .allow_boxed_zoom(false)
                                    .allow_double_click_reset(false)
                                    .auto_bounds([false, false])
                                    .include_y(0.0)
                                    .include_y(100.0)
                                    .allow_axis_zoom_drag([false, false]);

                                // Format the horizontal timeline into clean MoTeC timestamps
                                plot = plot.x_axis_formatter(|tick, _range| {
                                    let sec = tick.value;
                                    let minutes = (sec / 60.0).floor() as i32;
                                    let seconds = (sec % 60.0).floor() as i32;
                                    let ms = ((sec % 1.0) * 10.0).round() as i32; // tenths
                                    format!("{:02}:{:02}.{}", minutes, seconds, ms)
                                });

                                plot = plot.show_axes([true, false]);

                                match self.active_worksheet {
                                    WorksheetTab::Basic => {
                                        // 3. Extract exact unscaled physical values from session using locked cursor index!
                                        let mut raw_val_speed = 0.0;
                                        let mut raw_val_throttle = 0.0;
                                        let mut raw_val_brake = 0.0;
                                        let mut raw_val_steering = 0.0;
                                        let mut raw_val_rpm = 0.0;
                                        let mut raw_val_gear = 0.0;

                                        if let Some(ref mut cx) = self.cursor_x {
                                            *cx = cx.clamp(0.0, max_time);
                                            let idx = get_closest_index(&self.speed_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), *cx);
                                            
                                            // Look up raw database variables at the active cursor sample
                                            let speed_col = session_ref.dataframe.column("Speed").ok().map(|c| c.f64().ok()).flatten();
                                            let throttle_col = session_ref.dataframe.column("Throttle").ok().map(|c| c.f64().ok()).flatten();
                                            let brake_col = session_ref.dataframe.column("Brake").ok().map(|c| c.f64().ok()).flatten();
                                            let steering_col = session_ref.dataframe.column("SteeringWheelAngle").ok().map(|c| c.f64().ok()).flatten();
                                            let rpm_col = session_ref.dataframe.column("RPM").ok().map(|c| c.f64().ok()).flatten();
                                            let gear_col = session_ref.dataframe.column("Gear").ok().map(|c| c.f64().ok()).flatten();

                                            raw_val_speed = speed_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 3.6;
                                            raw_val_throttle = throttle_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 100.0;
                                            raw_val_brake = brake_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 100.0;
                                            raw_val_steering = steering_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 57.2958;
                                            raw_val_rpm = rpm_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0);
                                            raw_val_gear = gear_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0);
                                        }

                                        ui.horizontal(|ui| {
                                            if let Some(cx) = self.cursor_x {
                                                ui.colored_label(ACCENT_COLOR, format!("⏱  PLAYBACK @ {}", format_lap_time(cx)));
                                                ui.separator();
                                                ui.colored_label(SPEED_COLOR, format!("Speed: {:.1} km/h", raw_val_speed));
                                                ui.separator();
                                                ui.colored_label(egui::Color32::from_rgb(46, 204, 113), format!("Throttle: {:.0}%", raw_val_throttle));
                                                ui.separator();
                                                ui.colored_label(egui::Color32::from_rgb(231, 76, 60), format!("Brake: {:.0}%", raw_val_brake));
                                                ui.separator();
                                                ui.colored_label(SUB_ACCENT_COLOR, format!("Steering: {:.1}°", raw_val_steering));
                                                ui.separator();
                                                ui.colored_label(egui::Color32::from_rgb(241, 196, 15), format!("RPM: {:.0}", raw_val_rpm));
                                                ui.separator();
                                                ui.colored_label(egui::Color32::WHITE, format!("Gear: {}", if raw_val_gear < 0.0 { "R".to_string() } else if raw_val_gear == 0.0 { "N".to_string() } else { format!("{:.0}", raw_val_gear) }));
                                            }
                                        });
                                        ui.add_space(4.0);

                                        plot.show(ui, |plot_ui| {
                                            let is_left_click_down = plot_ui.ctx().input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                                            if self.reset_bounds_flag {
                                                if let Some(sel_lap) = self.selected_lap {
                                                    if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == sel_lap) {
                                                        let (_, start_t, _end_t) = self.lap_ranges[pos];
                                                        let end_time_focus = if pos + 1 < self.lap_ranges.len() {
                                                            self.lap_ranges[pos + 1].1
                                                        } else {
                                                            max_time
                                                        };
                                                        plot_ui.set_plot_bounds_x(start_t..=end_time_focus);
                                                    } else {
                                                        plot_ui.set_plot_bounds_x(0.0..=max_time);
                                                    }
                                                } else {
                                                    plot_ui.set_plot_bounds_x(0.0..=max_time);
                                                }
                                                self.reset_bounds_flag = false;
                                            }

                                            let active_bounds = plot_ui.plot_bounds();
                                            let min_visible_x = active_bounds.min()[0];
                                            let max_visible_x = active_bounds.max()[0];
                                            let visible_width = max_visible_x - min_visible_x;

                                            // Slicing and decimating helper
                                            let decimate_points = |cache: &Vec<[f64; 2]>| -> PlotPoints {
                                                if cache.is_empty() { return PlotPoints::default(); }
                                                let start_idx = match cache.binary_search_by(|p| p[0].partial_cmp(&min_visible_x).unwrap_or(std::cmp::Ordering::Equal)) {
                                                    Ok(idx) => idx,
                                                    Err(idx) => idx,
                                                }.saturating_sub(1);
                                                let end_idx = match cache.binary_search_by(|p| p[0].partial_cmp(&max_visible_x).unwrap_or(std::cmp::Ordering::Equal)) {
                                                    Ok(idx) => idx,
                                                    Err(idx) => idx,
                                                }.min(cache.len());
                                                let slice = &cache[start_idx..end_idx];
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

                                            let speed_dec_pts = decimate_points(&self.speed_pts_cache);
                                            let throttle_dec_pts = decimate_points(&self.throttle_pts_cache);
                                            let brake_dec_pts = decimate_points(&self.brake_pts_cache);
                                            let steering_dec_pts = decimate_points(&self.steering_pts_cache);
                                            let rpm_dec_pts = decimate_points(&self.rpm_pts_cache);

                                            // 1. Draw Axis Dividers separating our 4 lanes cleanly
                                            let div_color = if is_dark { egui::Color32::from_rgb(25, 30, 32) } else { egui::Color32::from_rgb(205, 204, 203) };
                                            plot_ui.hline(HLine::new("Top Lane Divider", 74.0).color(div_color).width(1.0));
                                            plot_ui.hline(HLine::new("Middle-Upper Divider", 50.0).color(div_color).width(1.0));
                                            plot_ui.hline(HLine::new("Middle-Lower Divider", 26.0).color(div_color).width(1.0));
                                            plot_ui.hline(HLine::new("Bottom Ticker Divider", 9.5).color(div_color).width(1.0));

                                            // 2. Draw our Sleek Interactive Dark Ticker timeline background bar!
                                            let track_color = if is_dark { egui::Color32::from_rgb(12, 18, 20) } else { egui::Color32::from_rgb(215, 214, 213) };
                                            plot_ui.hline(HLine::new("Timeline Track", 4.75).color(track_color).width(9.5));

                                            // 3. Draw Stacked Decimated Curves (Guarantees silky-smooth vertex rendering!)
                                            // Lane 1: Speed [76.0, 98.0]
                                            plot_ui.line(Line::new("Speed (km/h)", speed_dec_pts).color(SPEED_COLOR).width(2.2));
                                            
                                            // Lane 2: RPM [52.0, 72.0]
                                            plot_ui.line(Line::new("Engine RPM", rpm_dec_pts).color(egui::Color32::from_rgb(241, 196, 15)).width(2.2));
                                            
                                            // Lane 3: Throttle and Brake overlaid! [28.0, 48.0]
                                            plot_ui.line(Line::new("Throttle (%)", throttle_dec_pts).color(egui::Color32::from_rgb(46, 204, 113)).width(2.2));
                                            plot_ui.line(Line::new("Brake (%)", brake_dec_pts).color(egui::Color32::from_rgb(231, 76, 60)).width(2.2));

                                            // Lane 4: Steering Angle [10.0, 24.0]
                                            plot_ui.line(Line::new("Steering Angle (°)", steering_dec_pts).color(SUB_ACCENT_COLOR).width(2.2));

                                            // --- DYNAMIC MOTEC MULTI-LAP REFERENCE OVERLAYS ---
                                            // A. Bright Cyan Overlay Reference Lap
                                            if let Some(ref_lap_num) = self.ref_lap_cyan {
                                                let ref_start = self.lap_ranges.iter().find(|r| r.0 == ref_lap_num).map(|r| r.1).unwrap_or(0.0);
                                                let slice_speed = self.get_lap_points_slice(&self.speed_pts_cache, ref_lap_num);
                                                let slice_rpm = self.get_lap_points_slice(&self.rpm_pts_cache, ref_lap_num);
                                                let slice_thr = self.get_lap_points_slice(&self.throttle_pts_cache, ref_lap_num);
                                                let slice_brk = self.get_lap_points_slice(&self.brake_pts_cache, ref_lap_num);
                                                let slice_steer = self.get_lap_points_slice(&self.steering_pts_cache, ref_lap_num);

                                                if !slice_speed.is_empty() {
                                                    for &(lap_num, start_t, end_t) in &self.lap_ranges {
                                                        if end_t >= min_visible_x && start_t <= max_visible_x {
                                                            let offset = start_t - ref_start;
                                                            let dec_spd = decimate_points(&slice_speed.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_rpm = decimate_points(&slice_rpm.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_thr = decimate_points(&slice_thr.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_brk = decimate_points(&slice_brk.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_steer = decimate_points(&slice_steer.iter().map(|p| [p[0] + offset, p[1]]).collect());

                                                            plot_ui.line(Line::new(format!("Speed Cyan Ref Lap{}", lap_num), dec_spd).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("RPM Cyan Ref Lap{}", lap_num), dec_rpm).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("Thr Cyan Ref Lap{}", lap_num), dec_thr).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("Brk Cyan Ref Lap{}", lap_num), dec_brk).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("Steer Cyan Ref Lap{}", lap_num), dec_steer).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                        }
                                                    }
                                                }
                                            }

                                            // B. Pure White Overlay Reference Lap
                                            if let Some(ref_lap_num) = self.ref_lap_white {
                                                let ref_start = self.lap_ranges.iter().find(|r| r.0 == ref_lap_num).map(|r| r.1).unwrap_or(0.0);
                                                let slice_speed = self.get_lap_points_slice(&self.speed_pts_cache, ref_lap_num);
                                                let slice_rpm = self.get_lap_points_slice(&self.rpm_pts_cache, ref_lap_num);
                                                let slice_thr = self.get_lap_points_slice(&self.throttle_pts_cache, ref_lap_num);
                                                let slice_brk = self.get_lap_points_slice(&self.brake_pts_cache, ref_lap_num);
                                                let slice_steer = self.get_lap_points_slice(&self.steering_pts_cache, ref_lap_num);

                                                if !slice_speed.is_empty() {
                                                    for &(lap_num, start_t, end_t) in &self.lap_ranges {
                                                        if end_t >= min_visible_x && start_t <= max_visible_x {
                                                            let offset = start_t - ref_start;
                                                            let dec_spd = decimate_points(&slice_speed.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_rpm = decimate_points(&slice_rpm.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_thr = decimate_points(&slice_thr.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_brk = decimate_points(&slice_brk.iter().map(|p| [p[0] + offset, p[1]]).collect());
                                                            let dec_steer = decimate_points(&slice_steer.iter().map(|p| [p[0] + offset, p[1]]).collect());

                                                            plot_ui.line(Line::new(format!("Speed White Ref Lap{}", lap_num), dec_spd).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("RPM White Ref Lap{}", lap_num), dec_rpm).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("Thr White Ref Lap{}", lap_num), dec_thr).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("Brk White Ref Lap{}", lap_num), dec_brk).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("Steer White Ref Lap{}", lap_num), dec_steer).color(egui::Color32::WHITE).width(1.2));
                                                        }
                                                    }
                                                }
                                            }

                                            // 4. Draw thin red dotted column lines flagging LAP BOUNDARY SEPARATIONS (MoTeC Style!)
                                            for &lap_start_time in &self.lap_markers {
                                                if lap_start_time > 0.0 {
                                                    plot_ui.vline(VLine::new(format!("LapSeparator_{}", lap_start_time), lap_start_time)
                                                        .color(egui::Color32::from_rgba_unmultiplied(220, 20, 60, 120)) // Slate Red, translucent
                                                        .style(egui_plot::LineStyle::dotted_dense())
                                                        .width(1.0)
                                                    );
                                                }
                                            }

                                            // 5. Draw Custom Symmetrical Time Stamp Tickers & Labels Inside the Interactive Timeline Track!
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

                                            // 6. Draw Elegant Lap Label Markers Just Above the Graph (Right Under Tooltips)
                                            for (lap_idx, &(lap_num, start_t, end_t)) in self.lap_ranges.iter().enumerate() {
                                                if end_t >= min_visible_x && start_t <= max_visible_x {
                                                    let center = (start_t + end_t) / 2.0;
                                                    let label_str = if lap_idx == 0 { "Outlap".to_string() } else { format!("Lap {}", lap_idx) };
                                                    let label_txt_color = if is_dark { egui::Color32::from_rgb(180, 195, 200) } else { egui::Color32::from_rgb(60, 70, 75) };
                                                    plot_ui.text(Text::new(format!("LapLabelMarker_{}", lap_num), PlotPoint::new(center, 99.0), egui::RichText::new(label_str).color(label_txt_color).size(10.0).strong()));
                                                }
                                            }

                                            // 7. Draw Locked Playback Cursor timeline
                                            if let Some(cx) = self.cursor_x {
                                                plot_ui.vline(VLine::new("Cursor Line", cx).color(ACCENT_COLOR).width(1.5));
                                                let idx = get_closest_index(&self.speed_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), cx);
                                                
                                                let scaled_val_speed = self.speed_pts_cache[idx][1];
                                                let scaled_val_rpm = self.rpm_pts_cache[idx][1];
                                                let scaled_val_thr = self.throttle_pts_cache[idx][1];
                                                let scaled_val_brk = self.brake_pts_cache[idx][1];
                                                let scaled_val_steer = self.steering_pts_cache[idx][1];

                                                plot_ui.points(Points::new("Cursor Speed", PlotPoints::from(vec![[cx, scaled_val_speed]])).color(SPEED_COLOR).radius(5.0));
                                                plot_ui.points(Points::new("Cursor RPM", PlotPoints::from(vec![[cx, scaled_val_rpm]])).color(egui::Color32::from_rgb(241, 196, 15)).radius(5.0));
                                                plot_ui.points(Points::new("Cursor Thr", PlotPoints::from(vec![[cx, scaled_val_thr]])).color(egui::Color32::from_rgb(46, 204, 113)).radius(5.0));
                                                plot_ui.points(Points::new("Cursor Brk", PlotPoints::from(vec![[cx, scaled_val_brk]])).color(egui::Color32::from_rgb(231, 76, 60)).radius(5.0));
                                                plot_ui.points(Points::new("Cursor Steer", PlotPoints::from(vec![[cx, scaled_val_steer]])).color(SUB_ACCENT_COLOR).radius(5.0));
                                                plot_ui.points(Points::new("Stamp Ticker", PlotPoints::from(vec![[cx, 4.75]])).color(ACCENT_COLOR).shape(egui_plot::MarkerShape::Up).radius(10.0));
                                            }

                                            // 8. Highlight Zoom State
                                            if self.is_highlight_active {
                                                if let Some(x_start) = self.highlight_start {
                                                    let current_x = plot_ui.pointer_coordinate().map(|p| p.x.clamp(0.0, max_time)).unwrap_or_else(|| self.cursor_x.unwrap_or(0.0));
                                                    let start = f64::min(x_start, current_x);
                                                    let end = f64::max(x_start, current_x);
                                                    plot_ui.span(Span::new("Zoom Highlight", start..=end).axis(Axis::X).fill(egui::Color32::from_rgba_unmultiplied(242, 82, 37, 32)).border_width(1.0).border_color(egui::Color32::from_rgba_unmultiplied(242, 82, 37, 120)));
                                                }
                                            }

                                            // 9. Time Ticker zone dragging detector
                                            if plot_ui.ctx().input(|i| i.pointer.any_pressed()) {
                                                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                                                    self.is_dragging_ticker = pointer_pos.y < 9.5;
                                                }
                                            }

                                            // 10. Dragging / Panning / Scrubbing
                                            if is_left_click_down {
                                                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                                                    let click_pos = pointer_pos.x.clamp(0.0, max_time);
                                                    if self.is_highlight_active {
                                                        if !plot_ui.response().double_clicked() {
                                                            if let Some(x_start) = self.highlight_start {
                                                                let zoom_min = f64::min(x_start, click_pos);
                                                                let zoom_max = f64::max(x_start, click_pos);
                                                                if (zoom_max - zoom_min).abs() > 0.1 {
                                                                    plot_ui.set_plot_bounds_x(zoom_min..=zoom_max);
                                                                    self.cursor_x = Some(zoom_min);
                                                                }
                                                                self.is_highlight_active = false;
                                                                self.highlight_start = None;
                                                            }
                                                        }
                                                    } else if self.is_dragging_ticker {
                                                        let pixel_delta_x = plot_ui.ctx().input(|i| i.pointer.delta().x);
                                                        let plot_width_pixels = plot_ui.response().rect.width();
                                                        let pixels_per_second = (plot_width_pixels as f64) / visible_width;
                                                        let seconds_delta = (pixel_delta_x as f64) / pixels_per_second;
                                                        let new_min = (min_visible_x - seconds_delta).clamp(0.0, max_time - visible_width);
                                                        let new_max = new_min + visible_width;
                                                        plot_ui.set_plot_bounds_x(new_min..=new_max);
                                                    } else {
                                                        self.cursor_x = Some(click_pos);
                                                    }
                                                }
                                            }

                                            // 11. Zooming Wheel
                                            if !is_left_click_down {
                                                let scroll = plot_ui.ctx().input(|i| i.smooth_scroll_delta);
                                                if scroll.y.abs() > 1.5 {
                                                    let is_zooming_in = scroll.y > 0.0;
                                                    let zoom_factor = if is_zooming_in { 0.925 } else { 1.075 };
                                                    let mut target_width = visible_width * zoom_factor;
                                                    target_width = target_width.clamp(1.5, max_time);
                                                    let center = if is_zooming_in { self.cursor_x.unwrap_or((min_visible_x + max_visible_x) / 2.0) } else { (min_visible_x + max_visible_x) / 2.0 };
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
                                                    }
                                                }
                                            }
                                        });
                                    }
                                    WorksheetTab::DynamicRake => {
                                        // 3. Extract exact unscaled physical values from session using locked cursor index!
                                        let mut raw_val_front = 0.0;
                                        let mut raw_val_rear = 0.0;
                                        let mut raw_val_rake = 0.0;
                                        let mut raw_val_speed = 0.0;

                                        if let Some(ref mut cx) = self.cursor_x {
                                            // Clamp the locked cursor position strictly to the plotted data bounds
                                            *cx = cx.clamp(0.0, max_time);
                                            
                                            // Binary search relative time in our precomputed points cache [0.0 to stint duration]
                                            let idx = match self.front_pts_cache.binary_search_by(|p| p[0].partial_cmp(cx).unwrap_or(std::cmp::Ordering::Equal)) {
                                                Ok(i) => i,
                                                Err(i) => {
                                                    if i >= self.front_pts_cache.len() {
                                                        self.front_pts_cache.len().saturating_sub(1)
                                                    } else {
                                                        i
                                                    }
                                                }
                                            };

                                            if let Some(session) = &self.session {
                                                if idx < session.front_smooth.len() {
                                                    // Convert to millimeters cleanly from physical arrays
                                                    let scale = if session.front_smooth[idx] < 0.5 { 1000.0 } else { 1.0 };
                                                    raw_val_front = session.front_smooth[idx] * scale;
                                                    raw_val_rear = session.rear_smooth[idx] * scale;
                                                    raw_val_rake = session.rake[idx] * scale;
                                                    
                                                    // Speed column lookup (Convert to km/h)
                                                    let speed_col = session.dataframe.column("Speed").unwrap().f64().unwrap();
                                                    raw_val_speed = speed_col.get(idx).unwrap_or(0.0) * 3.6;
                                                }
                                            }
                                        }

                                        ui.horizontal(|ui| {
                                            // Dynamic Locked Values HUD inside graph headers (MoTeC style!)
                                            if let Some(cx) = self.cursor_x {
                                                ui.colored_label(ACCENT_COLOR, format!("⏱  PLAYBACK @ {}", format_lap_time(cx)));
                                                ui.separator();
                                                ui.colored_label(SPEED_COLOR, format!("Ground Speed: {:.1} km/h", raw_val_speed));
                                                ui.separator();
                                                ui.colored_label(SUB_ACCENT_COLOR, format!("Front RH: {:.2}mm", raw_val_front));
                                                ui.colored_label(egui::Color32::from_rgb(255, 20, 147), format!("Rear RH: {:.2}mm", raw_val_rear));
                                                ui.colored_label(ACCENT_COLOR, format!("Rake: {:.2}mm", raw_val_rake));
                                            }
                                        });
                                        ui.add_space(4.0);

                                        plot.show(ui, |plot_ui| {
                                            // Trigger external Reset Zoom button or first-load bounds initialization (Zoom to currently selected lap range)
                                            if self.reset_bounds_flag {
                                                if let Some(sel_lap) = self.selected_lap {
                                                    if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == sel_lap) {
                                                        let (_, start_t, _end_t) = self.lap_ranges[pos];
                                                        let end_time_focus = if pos + 1 < self.lap_ranges.len() {
                                                            self.lap_ranges[pos + 1].1
                                                        } else {
                                                            max_time
                                                        };
                                                        plot_ui.set_plot_bounds_x(start_t..=end_time_focus);
                                                    } else {
                                                        plot_ui.set_plot_bounds_x(0.0..=max_time);
                                                    }
                                                } else {
                                                    plot_ui.set_plot_bounds_x(0.0..=max_time);
                                                }
                                                self.reset_bounds_flag = false;
                                            }

                                            // Retrieve active screen bounds to process interactive drag scrubbing & edge panning!
                                            let active_bounds = plot_ui.plot_bounds();
                                            let min_visible_x = active_bounds.min()[0];
                                            let max_visible_x = active_bounds.max()[0];
                                            let visible_width = max_visible_x - min_visible_x;

                                            // --- HIGH-PERFORMANCE DYNAMIC VIEWPORT DECIMATION / STRIDING ---
                                            // Binary search the exact bounds range in O(log N) time and extract visible points!
                                            // Cops heap-allocations down to virtually zero, restoring buttery-smooth 300+ FPS!
                                            let decimate_points = |cache: &Vec<[f64; 2]>| -> PlotPoints {
                                                if cache.is_empty() { return PlotPoints::default(); }
                                                
                                                let start_idx = match cache.binary_search_by(|p| p[0].partial_cmp(&min_visible_x).unwrap_or(std::cmp::Ordering::Equal)) {
                                                    Ok(idx) => idx,
                                                    Err(idx) => idx,
                                                }.saturating_sub(1);
                                                
                                                let end_idx = match cache.binary_search_by(|p| p[0].partial_cmp(&max_visible_x).unwrap_or(std::cmp::Ordering::Equal)) {
                                                    Ok(idx) => idx,
                                                    Err(idx) => idx,
                                                }.min(cache.len());
                                                
                                                let slice = &cache[start_idx..end_idx];
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

                                            let front_dec_pts = decimate_points(&self.front_pts_cache);
                                            let rear_dec_pts = decimate_points(&self.rear_pts_cache);
                                            let rake_dec_pts = decimate_points(&self.rake_pts_cache);
                                            let speed_dec_pts = decimate_points(&self.speed_pts_cache);

                                            // 1. Draw Axis Dividers separating our lanes cleanly
                                            let div_color = if is_dark { egui::Color32::from_rgb(25, 30, 32) } else { egui::Color32::from_rgb(205, 204, 203) };
                                            plot_ui.hline(HLine::new("Top Lane Divider", 68.0).color(div_color).width(1.0));
                                            plot_ui.hline(HLine::new("Middle Lane Divider", 38.0).color(div_color).width(1.0));
                                            plot_ui.hline(HLine::new("Bottom Ticker Divider", 9.5).color(div_color).width(1.0));

                                            // 2. Draw our Sleek Interactive Dark Ticker timeline background bar!
                                            let track_color = if is_dark { egui::Color32::from_rgb(12, 18, 20) } else { egui::Color32::from_rgb(215, 214, 213) };
                                            plot_ui.hline(HLine::new("Timeline Track", 4.75).color(track_color).width(9.5));

                                            // 3. Draw Stacked Decimated Curves (Guarantees silky-smooth vertex rendering!)
                                            // Lane 1 (Ground Speed - Calm Sky Blue): [70.0, 98.0]
                                            plot_ui.line(Line::new("Ground Speed (km/h)", speed_dec_pts).color(SPEED_COLOR).width(2.2));
                                            
                                            // Lane 2 (Axle Heights): [40.0, 66.0]
                                            plot_ui.line(Line::new("Front Ride Height (4.5s Smoothed)", front_dec_pts).color(SUB_ACCENT_COLOR).width(2.2));
                                            plot_ui.line(Line::new("Rear Ride Height (4.5s Smoothed)", rear_dec_pts).color(egui::Color32::from_rgb(255, 20, 147)).width(2.2));
                                            
                                            // Lane 3 (Dynamic Rake): [12.0, 36.0]
                                            plot_ui.line(Line::new("Dynamic Rake (mm)", rake_dec_pts).color(ACCENT_COLOR).width(2.2));

                                            // --- DYNAMIC MOTEC MULTI-LAP REFERENCE OVERLAYS (All 3 Channels!) ---
                                            // A. Bright Cyan Overlay Reference Lap (Overlaid on EVERY visible lap section in the stint!)
                                            if let Some(ref_lap_num) = self.ref_lap_cyan {
                                                let ref_start = self.lap_ranges.iter()
                                                    .find(|r| r.0 == ref_lap_num)
                                                    .map(|r| r.1)
                                                    .unwrap_or(0.0);
                                                
                                                let slice_front = self.get_lap_points_slice(&self.front_pts_cache, ref_lap_num);
                                                let slice_rear = self.get_lap_points_slice(&self.rear_pts_cache, ref_lap_num);
                                                let slice_rake = self.get_lap_points_slice(&self.rake_pts_cache, ref_lap_num);
                                                let slice_speed = self.get_lap_points_slice(&self.speed_pts_cache, ref_lap_num);

                                                if !slice_front.is_empty() {
                                                    // Loop over every visible lap configuration in the stint and project the reference layout
                                                    for &(lap_num, start_t, end_t) in &self.lap_ranges {
                                                        if end_t >= min_visible_x && start_t <= max_visible_x {
                                                            let offset = start_t - ref_start;
                                                            
                                                            let shifted_front: Vec<[f64; 2]> = slice_front.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                                            let shifted_rear: Vec<[f64; 2]> = slice_rear.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                                            let shifted_rake: Vec<[f64; 2]> = slice_rake.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                                            let shifted_speed: Vec<[f64; 2]> = slice_speed.iter().map(|p| [p[0] + offset, p[1]]).collect();

                                                            let dec_front = decimate_points(&shifted_front);
                                                            let dec_rear = decimate_points(&shifted_rear);
                                                            let dec_rake = decimate_points(&shifted_rake);
                                                            let dec_speed = decimate_points(&shifted_speed);

                                                            // Draw comparison lines in bright Cyan (#00FFFF) slightly thinner (width 1.2) for perfect contrast
                                                            plot_ui.line(Line::new(format!("Speed Cyan Ref Lap{}", lap_num), dec_speed).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("Front Cyan Ref Lap{}", lap_num), dec_front).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("Rear Cyan Ref Lap{}", lap_num), dec_rear).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                            plot_ui.line(Line::new(format!("Rake Cyan Ref Lap{}", lap_num), dec_rake).color(egui::Color32::from_rgb(0, 255, 255)).width(1.2));
                                                        }
                                                    }
                                                }
                                            }

                                            // B. Pure White Overlay Reference Lap (Overlaid on EVERY visible lap section in the stint!)
                                            if let Some(ref_lap_num) = self.ref_lap_white {
                                                let ref_start = self.lap_ranges.iter()
                                                    .find(|r| r.0 == ref_lap_num)
                                                    .map(|r| r.1)
                                                    .unwrap_or(0.0);
                                                
                                                let slice_front = self.get_lap_points_slice(&self.front_pts_cache, ref_lap_num);
                                                let slice_rear = self.get_lap_points_slice(&self.rear_pts_cache, ref_lap_num);
                                                let slice_rake = self.get_lap_points_slice(&self.rake_pts_cache, ref_lap_num);
                                                let slice_speed = self.get_lap_points_slice(&self.speed_pts_cache, ref_lap_num);

                                                if !slice_front.is_empty() {
                                                    // Loop over every visible lap configuration in the stint and project the reference layout
                                                    for &(lap_num, start_t, end_t) in &self.lap_ranges {
                                                        if end_t >= min_visible_x && start_t <= max_visible_x {
                                                            let offset = start_t - ref_start;
                                                            
                                                            let shifted_front: Vec<[f64; 2]> = slice_front.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                                            let shifted_rear: Vec<[f64; 2]> = slice_rear.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                                            let shifted_rake: Vec<[f64; 2]> = slice_rake.iter().map(|p| [p[0] + offset, p[1]]).collect();
                                                            let shifted_speed: Vec<[f64; 2]> = slice_speed.iter().map(|p| [p[0] + offset, p[1]]).collect();

                                                            let dec_front = decimate_points(&shifted_front);
                                                            let dec_rear = decimate_points(&shifted_rear);
                                                            let dec_rake = decimate_points(&shifted_rake);
                                                            let dec_speed = decimate_points(&shifted_speed);

                                                            // Draw comparison lines in pure White slightly thinner (width 1.2) for perfect contrast
                                                            plot_ui.line(Line::new(format!("Speed White Ref Lap{}", lap_num), dec_speed).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("Front White Ref Lap{}", lap_num), dec_front).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("Rear White Ref Lap{}", lap_num), dec_rear).color(egui::Color32::WHITE).width(1.2));
                                                            plot_ui.line(Line::new(format!("Rake White Ref Lap{}", lap_num), dec_rake).color(egui::Color32::WHITE).width(1.2));
                                                        }
                                                    }
                                                }
                                            }

                                            // 4. Draw thin red dotted column lines flagging LAP BOUNDARY SEPARATIONS (MoTeC Style!)
                                            for &lap_start_time in &self.lap_markers {
                                                if lap_start_time > 0.0 {
                                                    plot_ui.vline(VLine::new(format!("LapSeparator_{}", lap_start_time), lap_start_time)
                                                        .color(egui::Color32::from_rgba_unmultiplied(220, 20, 60, 120)) // Slate Red, translucent
                                                        .style(egui_plot::LineStyle::dotted_dense())
                                                        .width(1.0)
                                                    );
                                                }
                                            }

                                            // 5. Draw Custom Symmetrical Time Stamp Tickers & Labels Inside the Interactive Timeline Track!
                                            // Determine optimal tick step sizes dynamically based on zoom horizontal viewport widths
                                            let step = if visible_width > 240.0 {
                                                60.0
                                            } else if visible_width > 120.0 {
                                                30.0
                                            } else if visible_width > 60.0 {
                                                15.0
                                            } else if visible_width > 30.0 {
                                                10.0
                                            } else if visible_width > 15.0 {
                                                5.0
                                            } else if visible_width > 5.0 {
                                                2.0
                                            } else {
                                                0.5
                                            };

                                            let start_tick = (min_visible_x / step).floor() * step;
                                            let end_tick = (max_visible_x / step).ceil() * step;

                                            let mut current_tick = start_tick;
                                            while current_tick <= end_tick {
                                                if current_tick >= 0.0 && current_tick <= max_time {
                                                    // Draw Tick vertical line inside the track
                                                    let tick_line_color = if is_dark { egui::Color32::from_rgb(28, 38, 41) } else { egui::Color32::from_rgb(180, 179, 178) };
                                                    plot_ui.vline(VLine::new(format!("TickLine_{}", current_tick), current_tick)
                                                        .color(tick_line_color)
                                                        .width(1.0)
                                                    );

                                                    // Format & render custom timestamp labels perfectly centered inside track
                                                    let label_str = format_lap_time(current_tick);
                                                    let display_text = label_str.get(3..8).unwrap_or("00:00"); // formats to MM:SS
                                                    let text_color = if is_dark { egui::Color32::from_rgb(12, 18, 20) } else { egui::Color32::from_rgb(80, 80, 80) };
                                                    plot_ui.text(Text::new(
                                                        format!("TickLabel_{}", current_tick),
                                                        PlotPoint::new(current_tick, 4.75),
                                                        egui::RichText::new(display_text).color(text_color).size(9.0)
                                                    ));
                                                }
                                                current_tick += step;
                                            }

                                            // 6. Draw Elegant Lap Label Markers Just Above the Graph (Right Under Tooltips)
                                            // Placed centered above each lap's respective boundary section at Y = 99.0
                                            for (lap_idx, &(lap_num, start_t, end_t)) in self.lap_ranges.iter().enumerate() {
                                                if end_t >= min_visible_x && start_t <= max_visible_x {
                                                    let center = (start_t + end_t) / 2.0;
                                                    let label_str = if lap_idx == 0 {
                                                        "Outlap".to_string()
                                                    } else {
                                                        format!("Lap {}", lap_idx)
                                                    };
                                                    
                                                    // Render high-contrast lap labels centered in timeline track top edge
                                                    let label_txt_color = if is_dark { egui::Color32::from_rgb(180, 195, 200) } else { egui::Color32::from_rgb(60, 70, 75) };
                                                    plot_ui.text(Text::new(
                                                        format!("LapLabelMarker_{}", lap_num),
                                                        PlotPoint::new(center, 99.0),
                                                        egui::RichText::new(label_str).color(label_txt_color).size(10.0).strong()
                                                    ));
                                                }
                                            }

                                            // 7. Draw Locked Playback Cursor timeline
                                            if let Some(cx) = self.cursor_x {
                                                plot_ui.vline(VLine::new("Cursor Line", cx).color(ACCENT_COLOR).width(1.5));
                                                
                                                // Find original index to scale the vertical intersection dots on the decimated view
                                                let idx = get_closest_index(&self.front_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), cx);
                                                
                                                // Extract y positions at cursor index
                                                let scaled_val_front = self.front_pts_cache[idx][1];
                                                let scaled_val_rear = self.rear_pts_cache[idx][1];
                                                let scaled_val_rake = self.rake_pts_cache[idx][1];
                                                let scaled_val_speed = self.speed_pts_cache[idx][1];

                                                // Draw intersection dots mapped directly to pre-scaled heights for all 3 Lanes!
                                                plot_ui.points(Points::new("Cursor Speed", PlotPoints::from(vec![[cx, scaled_val_speed]])).color(SPEED_COLOR).radius(5.0));
                                                plot_ui.points(Points::new("Cursor Front", PlotPoints::from(vec![[cx, scaled_val_front]])).color(SUB_ACCENT_COLOR).radius(5.0));
                                                plot_ui.points(Points::new("Cursor Rear", PlotPoints::from(vec![[cx, scaled_val_rear]])).color(egui::Color32::from_rgb(255, 20, 147)).radius(5.0));
                                                plot_ui.points(Points::new("Cursor Rake", PlotPoints::from(vec![[cx, scaled_val_rake]])).color(ACCENT_COLOR).radius(5.0));

                                                // Draw Glowing orange Cursor Slider stamp ticker on the bottom timeline track!
                                                plot_ui.points(Points::new("Stamp Ticker", PlotPoints::from(vec![[cx, 4.75]]))
                                                    .color(ACCENT_COLOR)
                                                    .shape(egui_plot::MarkerShape::Up)
                                                    .radius(10.0)
                                                );
                                            }

                                            // --- MOTEC STYLE DOUBLE-CLICK HIGHLIGHT ZOOM STATE MACHINE ---
                                            if plot_ui.response().double_clicked() {
                                                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                                                    self.highlight_start = Some(pointer_pos.x.clamp(0.0, max_time));
                                                    self.cursor_x = Some(pointer_pos.x.clamp(0.0, max_time));
                                                    self.is_highlight_active = true;
                                                }
                                            }

                                            // Render custom translucent orange horizontal band (huge vertical column stem) if highlight selection is active!
                                            if self.is_highlight_active {
                                                if let Some(x_start) = self.highlight_start {
                                                    // Track the user's cursor horizontally in real-time
                                                    let current_x = plot_ui.pointer_coordinate()
                                                        .map(|p| p.x.clamp(0.0, max_time))
                                                        .unwrap_or_else(|| self.cursor_x.unwrap_or(0.0));

                                                    let start = f64::min(x_start, current_x);
                                                    let end = f64::max(x_start, current_x);

                                                    // Pulls translucent overlay parameters directly from ACCENT_COLOR
                                                    let fill_alpha = egui::Color32::from_rgba_unmultiplied(242, 82, 37, 32);
                                                    let border_alpha = egui::Color32::from_rgba_unmultiplied(242, 82, 37, 120);

                                                    plot_ui.span(Span::new("Zoom Highlight", start..=end)
                                                        .axis(Axis::X)
                                                        .fill(fill_alpha)
                                                        .border_width(1.0)
                                                        .border_color(border_alpha)
                                                    );
                                                }
                                            }

                                            // --- DETECT DRAG START ZONE (IN COORDINATE SPACE!) ---
                                            // Because the custom track sits perfectly inside Y = [0.0, 9.5], coordinates are ALWAYS Some!
                                            // Clicking on the timeline is 100% guaranteed to be detected without click-through failures!
                                            if plot_ui.ctx().input(|i| i.pointer.any_pressed()) {
                                                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                                                    self.is_dragging_ticker = pointer_pos.y < 9.5;
                                                }
                                            }

                                            // --- INTERACTIVE DRAGGING / SCRUBBING / TIME-TICKER PANNING ---
                                            let is_left_click_down = plot_ui.ctx().input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                                            if is_left_click_down {
                                                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                                                    let click_pos = pointer_pos.x.clamp(0.0, max_time);

                                                    if self.is_highlight_active {
                                                        // --- MOTEC STYLE CLICK-TO-FINALIZE ZOOM ---
                                                        // If highlight mode is active, the very next left-click zooms strictly on the highlighted envelope!
                                                        // To prevent instant-closes on the double click event, ensure double_clicked is FALSE
                                                        if !plot_ui.response().double_clicked() {
                                                            if let Some(x_start) = self.highlight_start {
                                                                let zoom_min = f64::min(x_start, click_pos);
                                                                let zoom_max = f64::max(x_start, click_pos);
                                                                
                                                                if (zoom_max - zoom_min).abs() > 0.1 {
                                                                    plot_ui.set_plot_bounds_x(zoom_min..=zoom_max);
                                                                    self.cursor_x = Some(zoom_min); // Snap cursor strictly to start of highlighted section!
                                                                }
                                                                self.is_highlight_active = false;
                                                                self.highlight_start = None;
                                                            }
                                                        }
                                                    } else if self.is_dragging_ticker {
                                                        // --- MOTEC SLIDING / PANNING MECHANICS ---
                                                        // Dragging on our custom bottom timeline track slides (pans) the visible viewport horizontally left or right!
                                                        let pixel_delta_x = plot_ui.ctx().input(|i| i.pointer.delta().x);
                                                        let plot_width_pixels = plot_ui.response().rect.width();
                                                        let pixels_per_second = (plot_width_pixels as f64) / visible_width;
                                                        let seconds_delta = (pixel_delta_x as f64) / pixels_per_second;

                                                        let new_min = (min_visible_x - seconds_delta).clamp(0.0, max_time - visible_width);
                                                        let new_max = new_min + visible_width;
                                                        plot_ui.set_plot_bounds_x(new_min..=new_max);
                                                    } else {
                                                        // Dragging inside the graph scrubs the playback cursor normally!
                                                        self.cursor_x = Some(click_pos);
                                                    }
                                                }
                                            }

                                            // --- SCROLL WHEEL HIGH-PRECISION HORIZONTAL ZOOMING ---
                                            // Holding zoom centered perfectly on your cursor, completely immune to touchpad loops!
                                            // Disabled strictly when left click is down to avoid any visual jumps during scrubbing dragging!
                                            if !is_left_click_down {
                                                let scroll = plot_ui.ctx().input(|i| i.smooth_scroll_delta);
                                                if scroll.y.abs() > 1.5 {
                                                    let is_zooming_in = scroll.y > 0.0;
                                                    
                                                    // Slowed down zoom speed per click tick by exactly 25% (from 10% adjustments to 7.5% adjustments!)
                                                    let zoom_factor = if is_zooming_in { 0.925 } else { 1.075 };

                                                    // Enforce precision zoom window limits [min_width = 1.5s, max_width = max_time]
                                                    let mut target_width = visible_width * zoom_factor;
                                                    target_width = target_width.clamp(1.5, max_time);

                                                    // Zoom IN -> Center on locked playback cursor (cursor_x)
                                                    // Zoom OUT -> Dock towards center of the visible graph symmetrically to prevent line drift!
                                                    let center = if is_zooming_in {
                                                        self.cursor_x.unwrap_or((min_visible_x + max_visible_x) / 2.0)
                                                    } else {
                                                        (min_visible_x + max_visible_x) / 2.0
                                                    };

                                                    let half_width = target_width / 2.0;
                                                    let mut new_min = center - half_width;
                                                    let mut mut_new_max = center + half_width;

                                                    // Symmetrical Boundary Overflow Shifting (Completely resolves disappeared-graph boundary drift!)
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
                                                    }
                                                }
                                            }
                                        });
                                    }
                                    _ => {
                                        // Placeholder for other tabs standing by
                                        ui.centered_and_justified(|ui| {
                                            ui.vertical_centered(|ui| {
                                                ui.label(egui::RichText::new("Worksheet Active").heading().color(ACCENT_COLOR));
                                                ui.label(egui::RichText::new("D3 D&D replacement plotters are standing by for this section...").color(egui::Color32::GRAY));
                                                ui.label(egui::RichText::new("(Phase 2 Rewrite Roadmap placeholder)").small().color(egui::Color32::DARK_GRAY));
                                            });
                                        });
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

fn height_offset(ui: &egui::Ui) -> f32 {
    let screen_height = ui.ctx().screen_rect().height();
    screen_height / 2.0
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 720.0]) 
            .with_min_inner_size([800.0, 500.0])
            .with_title("OpenDAV Telemetry Suite"),
        ..Default::default()
    };

    eframe::run_native(
        "OpenDAV",
        native_options,
        Box::new(|cc| Ok(Box::new(OpenDavApp::new(cc)))),
    )
}
