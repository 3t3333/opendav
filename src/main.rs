#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod data;
pub mod signals;
pub mod config;
pub mod rendering;
pub mod ui;
pub mod simgit;

#[cfg(feature = "dev_tools")]
pub mod dev_tools;

use crate::config::worksheet::{WorksheetTab, WorksheetConfig, ACCENT_COLOR, DARK_BG_COLOR, LIGHT_BG_COLOR};
use crate::signals::processing::{
    LapData, TrackSector, detect_track_sectors, get_lap_time_at_distance, get_fastest_lap
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ActivePage {
    OpenDav,  // Main Dashboard
    Graphs,   // Telemetry Plots
    Reports,  // Sector Reports
    SimGit,   // Version Control & Workspaces
    Settings, // Application Settings
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SimGitTab {
    Dashboard,
    Setups,
    Cloud,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ReportsTab {
    SectorAnalysis,
    TimingGraphs,
}

#[derive(Clone, Debug)]
pub enum AppState {
    Splash { progress: f32 },
    Main,
}

pub struct LoadedSession {
    pub file_name: String,
    pub session: crate::data::ibt_parser::IbtSession,
    pub track_bounds: Option<(f64, f64, f64, f64)>,
    
    // Caching compiled, scaled PlotPoints for 300+ FPS zero-allocation rendering!
    pub front_pts_cache: Vec<[f64; 2]>,
    pub rear_pts_cache: Vec<[f64; 2]>,
    pub rake_pts_cache: Vec<[f64; 2]>,
    pub speed_pts_cache: Vec<[f64; 2]>,
    pub lat_g_pts_cache: Vec<[f64; 2]>,
    pub long_g_pts_cache: Vec<[f64; 2]>,

    // Basic Driver Inputs Caches
    pub throttle_pts_cache: Vec<[f64; 2]>,
    pub brake_pts_cache: Vec<[f64; 2]>,
    pub steering_pts_cache: Vec<[f64; 2]>,
    pub rpm_pts_cache: Vec<[f64; 2]>,
    pub gear_pts_cache: Vec<[f64; 2]>,

    // Precomputed Lap boundary start/end time markers (relative to stint start in seconds)
    pub lap_ranges: Vec<(i32, f64, f64)>,

    // Map cache index to raw dataframe index to maintain correct time alignment for tooltips
    pub cache_to_df_index: Vec<usize>,

    // Precomputed Lap start timestamps (relative to stint start in seconds) for dotted red line dividers
    pub lap_markers: Vec<f64>,
    
    // Sector reports caches
    pub sectors: Vec<TrackSector>,
    pub sector_bests: Vec<f64>,
    pub lap_data_cache: Vec<LapData>,
    pub bg_image_bytes: Option<Vec<u8>>,
    pub bg_bounds: Option<[f64; 4]>,
    pub bg_texture: Option<egui::TextureHandle>,
    pub fg_image_bytes: Option<Vec<u8>>,
    pub fg_bounds: Option<[f64; 4]>,
    pub fg_texture: Option<egui::TextureHandle>,
    pub map_origin: Option<[f64; 2]>,
}

pub struct OpenDavApp {
    pub app_state: AppState,
    pub active_page: ActivePage,
    pub active_worksheet: WorksheetTab,
    pub active_reports_tab: ReportsTab,
    pub session_loaded: bool,
    pub active_file: Option<String>,
    pub fade_value: f32, // Smooth UI fade-in animation tracker

    pub sessions: Vec<LoadedSession>,
    pub primary_session_idx: usize,

    // Selected active Lap (Session Index, Lap Number)
    pub selected_lap: Option<(usize, i32)>,

    // MoTeC Style Locked Playback Cursor Tracker (Time in seconds)
    pub cursor_x: Option<f64>,

    // Trigger flag to reset native plot boundaries
    pub reset_bounds_flag: bool,
    pub reset_bounds_next_frame: u8,

    // Track if click drag initiated inside the bottom time-stamp ticker zone
    pub is_dragging_ticker: bool,

    // --- MOTEC DUAL-CLICK HIGHLIGHT ZOOM STATE ---
    pub is_highlight_active: bool,
    pub highlight_start: Option<f64>,

    // --- MOTEC MULTI-LAP REFERENCE OVERLAY STATE (Session Index, Lap Number) ---
    pub ref_lap_white: Option<(usize, i32)>,
    pub ref_lap_cyan: Option<(usize, i32)>,

    // Shared horizontal view bounds to perfectly synchronize Zoom/Pan/Scroll across different tabs!
    pub visible_x_range: Option<(f64, f64)>,

    // Tracks worksheet changes to execute tab-sync bounds on switch frames cleanly!
    pub previous_worksheet: Option<WorksheetTab>,

    pub show_graphs_track_map: bool,
    pub previous_page: Option<ActivePage>,
    pub previous_show_graphs_track_map: Option<bool>,
    pub show_sector_deltas: bool,
    pub show_chart_deltas: bool,
    pub sector_deltas: Vec<Option<f64>>,
    
    pub show_all_splits: bool,
    
    // Track Map Customization
    pub auto_follow_track_map: bool,
    pub auto_rotate_track_map: bool,
    pub track_map_rotation: f64,
    pub enable_satellite_map: bool,
    pub magnify_line_deltas: bool,
    pub magnifier_multiplier: f64,
    pub hidden_splits: std::collections::HashSet<String>,
    
    // Timing Graphs state
    pub filter_large_sectors: bool,
    
    pub is_playing: bool,
    pub playback_speed: f64,

    // SimGit State
    pub simgit_manager: crate::simgit::manager::SimGitManager,
    pub simgit_prev_setup: Option<std::path::PathBuf>,
    pub simgit_new_setup: Option<std::path::PathBuf>,
    pub simgit_diff: Option<crate::simgit::diff::SetupDiff>,
    pub simgit_active_tab: SimGitTab,
    pub simgit_new_ws_name: String,
    pub show_new_ws_popup: bool,
    
    // Cached JSON track map segments for SimGit Dashboard Cards
    pub simgit_track_maps: std::collections::HashMap<i32, Vec<Vec<[f64; 2]>>>,

    // Application Settings
    pub settings: crate::config::settings::AppSettings,

    #[cfg(feature = "dev_tools")]
    pub dev_metrics: crate::dev_tools::DebugMetrics,
}

impl Default for OpenDavApp {
    fn default() -> Self {
        Self {
            app_state: AppState::Splash { progress: 0.0 },
            active_page: ActivePage::OpenDav,
            active_worksheet: WorksheetTab::Basic,
            active_reports_tab: ReportsTab::SectorAnalysis,
            session_loaded: false,
            active_file: None,
            fade_value: 0.0,
            sessions: Vec::new(),
            primary_session_idx: 0,
            selected_lap: None,
            cursor_x: None,
            reset_bounds_flag: false,
            reset_bounds_next_frame: 0,
            is_dragging_ticker: false,
            is_highlight_active: false,
            highlight_start: None,
            ref_lap_white: None,
            ref_lap_cyan: None,
            visible_x_range: None,
            previous_worksheet: None,
            show_graphs_track_map: false,
            previous_page: None,
            previous_show_graphs_track_map: None,
            show_sector_deltas: false,
            show_chart_deltas: false,
            sector_deltas: Vec::new(),
            show_all_splits: true,
            auto_follow_track_map: false,
            auto_rotate_track_map: false,
            track_map_rotation: 0.0,
            enable_satellite_map: false,
            magnify_line_deltas: false,
            magnifier_multiplier: 10.0,
            hidden_splits: std::collections::HashSet::new(),
            filter_large_sectors: true,
            is_playing: false,
            playback_speed: 1.0,
            simgit_manager: crate::simgit::manager::SimGitManager::new(std::path::PathBuf::from("workspace")),
            simgit_prev_setup: None,
            simgit_new_setup: None,
            simgit_diff: None,
            simgit_active_tab: SimGitTab::Dashboard,
            simgit_new_ws_name: String::new(),
            show_new_ws_popup: false,
            simgit_track_maps: std::collections::HashMap::new(),
            settings: crate::config::settings::AppSettings::default(),
            
            #[cfg(feature = "dev_tools")]
            dev_metrics: crate::dev_tools::DebugMetrics::default(),
        }
    }
}

impl OpenDavApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders
        egui_extras::install_image_loaders(&_cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "DIN1451".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../assets/fonts/din-1451/DINMittelschriftStd.otf"))),
        );
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "DIN1451".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "DIN1451".to_owned());
        _cc.egui_ctx.set_fonts(fonts);

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

        let mut app = Self::default();
        app.settings = crate::config::settings::AppSettings::load();
        
        // Ensure egui matches our loaded dark mode preference
        if !app.settings.dark_mode {
            _cc.egui_ctx.set_visuals(egui::Visuals::light());
        }
        
        app
    }
}

impl LoadedSession {
    pub fn new(file_name: String, mut session: crate::data::ibt_parser::IbtSession, corner_merge_threshold: f64, mapbox_api_key: &str) -> Result<Self, String> {
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
            let mut latg = Vec::with_capacity(n);
            let mut longg = Vec::with_capacity(n);
            let mut cache_to_df_index = Vec::with_capacity(n);

            let time_col_opt = session.dataframe.column("SessionTime").ok().and_then(|c| c.f64().ok());
            if time_col_opt.is_none() { return Err("SessionTime missing".into()); }
            let time_col = time_col_opt.unwrap();
            let session_start = time_col.get(0).unwrap_or(0.0);

            // Fetch columns safely with fallback defaults to prevent schema panics!
            let speed_col = session.dataframe.column("Speed").ok().map(|c| c.f64().ok()).flatten();
            let throttle_col = session.dataframe.column("Throttle").ok().map(|c| c.f64().ok()).flatten();
            let brake_col = session.dataframe.column("Brake").ok().map(|c| c.f64().ok()).flatten();
            let steering_col = session.dataframe.column("SteeringWheelAngle").ok().map(|c| c.f64().ok()).flatten();
            let rpm_col = session.dataframe.column("RPM").ok().map(|c| c.f64().ok()).flatten();
            let gear_col = session.dataframe.column("Gear").ok().map(|c| c.f64().ok()).flatten();
            let latg_col = session.dataframe.column("LatAccel").ok().map(|c| c.f64().ok()).flatten();
            let longg_col = session.dataframe.column("LongAccel").ok().map(|c| c.f64().ok()).flatten();

            let is_on_track_col = session.dataframe.column("IsOnTrack").ok().map(|c| c.f64().ok()).flatten();
            let in_pit_stall_col = session.dataframe.column("PlayerCarInPitStall").ok().map(|c| c.f64().ok()).flatten();

            // 1. Compile entire session relative seconds [0.0 to stint duration]
            for i in 0..n {
                let is_on_track = is_on_track_col.as_ref().map(|c| c.get(i).unwrap_or(1.0)).unwrap_or(1.0);
                let in_pit_stall = in_pit_stall_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0);

                if is_on_track < 1.0 || in_pit_stall > 0.0 {
                    continue; // Skip off-track or pit stall samples cleanly!
                }
                
                cache_to_df_index.push(i);

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
                
                let raw_latg = latg_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) / 9.80665; // convert m/s2 to G
                latg.push([rel_time, raw_latg]);
                
                let raw_longg = longg_col.as_ref().map(|c| c.get(i).unwrap_or(0.0)).unwrap_or(0.0) / 9.80665; // convert m/s2 to G
                longg.push([rel_time, raw_longg]);
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

            let min_latg = latg.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_latg = latg.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let min_longg = longg.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            let max_longg = longg.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let min_g = f64::min(min_latg, min_longg);
            let max_g = f64::max(max_latg, max_longg);
            let pad_g = (max_g - min_g) * 0.1;
            
            let scale_g = |val: f64| -> f64 {
                if max_g == min_g { return 25.0; }
                let pct = ((val - (min_g - pad_g)) / ((max_g + pad_g) - (min_g - pad_g))).clamp(0.0, 1.0);
                10.0 + pct * (40.0 - 10.0)
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
                latg[i][1] = scale_g(latg[i][1]);
                longg[i][1] = scale_g(longg[i][1]);
            }

            // 3. Precompute Lap Start/End Boundaries based on actual parsed lap numbers
            let mut markers = Vec::new();
            let mut ranges = Vec::new();

            let df = &session.dataframe;
            let lap_col_opt = df.column("Lap").ok().and_then(|c| c.f64().ok());
            let time_col_opt = df.column("SessionTime").ok().and_then(|c| c.f64().ok());
            if lap_col_opt.is_none() || time_col_opt.is_none() { return Err("Lap/Time missing".into()); }
            let lap_col = lap_col_opt.unwrap();
            let time_col = time_col_opt.unwrap();
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

            if ranges.is_empty() && !front.is_empty() {
                let end_stint = front.last().unwrap()[0];
                ranges.push((1, 0.0, end_stint));
                markers.push(0.0);
            }

            // Harmonize and write the physical transition lap list back to the session struct to maintain absolute app coherence!
            let mut sync_laps = Vec::new();
            for &(lap_num, start_t, end_t) in &ranges {
                let duration = end_t - start_t;
                if duration > 1.0 {
                    sync_laps.push((lap_num, duration));
                }
            }
            session.lap_times = sync_laps;

            // Rebuild lap data cache
            let mut data_cache = Vec::new();
            let mut unique_laps = Vec::new();
            for &(lap_num, _, _) in &ranges {
                unique_laps.push(lap_num);
            }

            let df = &session.dataframe;
            let lap_col_opt = df.column("Lap").ok().and_then(|c| c.f64().ok());
            let dist_col_opt = df.column("Distance_Derived").ok().and_then(|c| c.f64().ok());
            let time_col_opt = df.column("SessionTime").ok().and_then(|c| c.f64().ok());
            if lap_col_opt.is_none() || dist_col_opt.is_none() || time_col_opt.is_none() { return Err("Lap/Dist/Time missing".into()); }
            let lap_col = lap_col_opt.unwrap();
            let dist_col = dist_col_opt.unwrap();
            let time_col = time_col_opt.unwrap();
            let lat_col = df.column("Lat").ok().and_then(|c| c.f64().ok());
            let lon_col = df.column("Lon").ok().and_then(|c| c.f64().ok());

            let mut lat0 = 0.0;
            let mut lon0 = 0.0;
            let mut wm0 = [0.0, 0.0];
            if let (Some(la), Some(lo)) = (lat_col.as_ref(), lon_col.as_ref()) {
                lat0 = la.get(0).unwrap_or(0.0);
                lon0 = lo.get(0).unwrap_or(0.0);
                let (wx, wy) = crate::signals::mapbox::wgs84_to_web_mercator(lon0, lat0);
                wm0 = [wx, wy];
            }

            // Removed r_earth, lat0_rad, and lon0_rad since we now use wgs84_to_web_mercator
            
            let mut min_lat = f64::MAX;
            let mut max_lat = f64::MIN;
            let mut min_lon = f64::MAX;
            let mut max_lon = f64::MIN;
            
            if let (Some(la), Some(lo)) = (lat_col.as_ref(), lon_col.as_ref()) {
                for i in 0..n {
                    let lat_val = la.get(i).unwrap_or(0.0);
                    let lon_val = lo.get(i).unwrap_or(0.0);
                    if lat_val != 0.0 && lon_val != 0.0 {
                        if lat_val < min_lat { min_lat = lat_val; }
                        if lat_val > max_lat { max_lat = lat_val; }
                        if lon_val < min_lon { min_lon = lon_val; }
                        if lon_val > max_lon { max_lon = lon_val; }
                    }
                }
            }

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
                        
                        // Use exact Web Mercator projection so the trace perfectly matches the satellite map!
                        let (wm_x, wm_y) = crate::signals::mapbox::wgs84_to_web_mercator(lon, lat);
                        let x = wm_x - wm0[0];
                        let y = wm_y - wm0[1];
                        xs.push(x);
                        ys.push(y);
                    }
                }

                // Fallback: if the lap was found in the headers but has no matching samples 
                // (e.g. short test files, out-laps, or glitching Lap channels), we populate it with ALL samples
                // to prevent rendering panics from empty arrays!
                if dists.is_empty() {
                    for i in 0..n {
                        dists.push(dist_col.get(i).unwrap_or(0.0));
                        times.push(time_col.get(i).unwrap_or(0.0));
                        let lat = lat_col.as_ref().and_then(|c| c.get(i)).unwrap_or(0.0);
                        let lon = lon_col.as_ref().and_then(|c| c.get(i)).unwrap_or(0.0);
                        
                        // Use exact Web Mercator projection so the trace perfectly matches the satellite map!
                        let (wm_x, wm_y) = crate::signals::mapbox::wgs84_to_web_mercator(lon, lat);
                        let x = wm_x - wm0[0];
                        let y = wm_y - wm0[1];
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
            // removed self.lap_data_cache

            // Rebuild track sectors and sector bests cache using Signals Layer
            let sectors_cache = detect_track_sectors(&session, corner_merge_threshold);
            
            let mut bests = vec![f64::MAX; sectors_cache.len()];
            for (s_idx, sector) in sectors_cache.iter().enumerate() {
                for lap in &data_cache {
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
                    for lap in &data_cache {
                        let t_start = get_lap_time_at_distance(&lap.dist, &lap.time, sector.start_dist);
                        let t_end = get_lap_time_at_distance(&lap.dist, &lap.time, sector.end_dist);
                        let s_time = t_end - t_start;
                        if s_time > 0.0 && s_time < bests[s_idx] {
                            bests[s_idx] = s_time;
                        }
                    }
                }
            }

            let mut track_bounds = None;
            if min_lat != f64::MAX {
                track_bounds = Some((min_lon, min_lat, max_lon, max_lat));
            }

            Ok(LoadedSession {
                track_bounds,
                bg_image_bytes: None,
                bg_bounds: None,
                bg_texture: None,
                fg_image_bytes: None,
                fg_bounds: None,
                fg_texture: None,
                map_origin: Some(wm0),
                file_name,
                session,
                front_pts_cache: front,
                rear_pts_cache: rear,
                rake_pts_cache: rake,
                speed_pts_cache: speed,
                lat_g_pts_cache: latg,
                long_g_pts_cache: longg,
                throttle_pts_cache: throttle,
                brake_pts_cache: brake,
                steering_pts_cache: steering,
                rpm_pts_cache: rpm,
                gear_pts_cache: gear,
                lap_ranges: ranges,
                lap_markers: markers,
                cache_to_df_index,
                sectors: sectors_cache,
                sector_bests: bests,
                lap_data_cache: data_cache,
            })
        }
    pub fn get_cache_slice(&self, name: &str) -> &[[f64; 2]] {
        match name {
            "Speed" => &self.speed_pts_cache,
            "Lat G" | "Lateral G" => &self.lat_g_pts_cache,
            "Long G" | "Longitudinal G" => &self.long_g_pts_cache,
            "Engine RPM" | "RPM" => &self.rpm_pts_cache,
            "Throttle" => &self.throttle_pts_cache,
            "Brake" => &self.brake_pts_cache,
            "Steering Angle" => &self.steering_pts_cache,
            "Ride Height (F)" | "Front Height" | "Front RH" => &self.front_pts_cache,
            "Ride Height (R)" | "Rear Height" | "Rear RH" => &self.rear_pts_cache,
            "Rake Angle" | "Dynamic Rake" => &self.rake_pts_cache,
            _ => &[],
        }
    }

    pub fn recalculate_sectors(&mut self, threshold: f64) {
        self.sectors = detect_track_sectors(&self.session, threshold);
        
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
        }
        self.sector_bests = bests;
    }
}

impl OpenDavApp {
    pub fn update_sector_deltas(&mut self) {
        if self.sessions.is_empty() { return; }
        let p_idx = self.primary_session_idx;
        let loaded = &self.sessions[p_idx];

        let ref_lap = self.ref_lap_cyan.or(self.ref_lap_white).map(|(_, lap)| lap);
        let active_lap_num = self.selected_lap.map(|(_, lap)| lap).or_else(|| {
            Some(crate::signals::processing::get_fastest_lap(&loaded.session.lap_times))
        });
        
        self.sector_deltas = crate::signals::processing::recalculate_sector_deltas(
            &loaded.lap_data_cache,
            &loaded.sectors,
            active_lap_num,
            ref_lap,
        );
    }
}

impl eframe::App for OpenDavApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "dev_tools")]
        {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            self.dev_metrics.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
            self.dev_metrics.frame_time_ms = dt * 1000.0;
            self.dev_metrics.history_dt.push(dt);
            if self.dev_metrics.history_dt.len() > 25 {
                self.dev_metrics.history_dt.remove(0);
            }
            
            if ctx.input(|i| i.key_pressed(egui::Key::F10)) {
                self.dev_metrics.show_overlay = !self.dev_metrics.show_overlay;
            }
        }

        let prev_page = self.previous_page;
        if prev_page.is_some() && prev_page != Some(self.active_page) {
            self.reset_bounds_flag = true;
            self.reset_bounds_next_frame = 3;
        }
        self.previous_page = Some(self.active_page);

        // --- DYNAMIC BRAND THEMING SWITCHER ---
        let mut style = (*ctx.global_style()).clone();
        
        // Ensure egui's visuals match our settings
        let is_dark = self.settings.dark_mode;
        if style.visuals.dark_mode != is_dark {
            style.visuals = if is_dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        }
        
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
        ctx.set_global_style(style);

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
                    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                } else {
                    self.draw_splash_screen(ctx, current_progress);
                }
            }
            AppState::Main => {
                if self.fade_value < 1.0 {
                    ctx.request_repaint();
                    self.fade_value += 0.02;
                }

                // Calculate smooth delta-time interpolation (if playback is active)
                if self.is_playing {
                    let dt = ctx.input(|i| i.stable_dt) as f64;
                    
                    if let Some((s_idx, lap_num)) = self.selected_lap {
                        if s_idx < self.sessions.len() {
                            let loaded = &self.sessions[s_idx];
                            if let Some(pos) = loaded.lap_ranges.iter().position(|r| r.0 == lap_num) {
                                let (_, start_t, end_t) = loaded.lap_ranges[pos];
                                
                                let mut current_t = self.cursor_x.unwrap_or(start_t);
                                current_t += dt * self.playback_speed;
                                
                                // Seamless looping at the end of the selected lap
                                if current_t > end_t {
                                    current_t = start_t;
                                }
                                
                                self.cursor_x = Some(current_t);
                                ctx.request_repaint(); // Crucial for smooth playback
                            } else {
                                self.is_playing = false; // Failsafe
                            }
                        } else {
                            self.is_playing = false; // Failsafe
                        }
                    } else {
                        self.is_playing = false;
                    }
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
                        ActivePage::SimGit => {
                            self.draw_simgit_page(ui, is_dark);
                        }
                        ActivePage::Settings => {
                            self.draw_settings_page(ui, is_dark);
                        }
                    }
                });
            }
        }

        #[cfg(feature = "dev_tools")]
        {
            crate::dev_tools::draw_overlay(ctx, &mut self.dev_metrics);
            ctx.request_repaint(); // continuously repaint when dev tools are shown for live stats
        }
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

fn main() -> eframe::Result<()> {
    // Load window icon from assets
    let icon_bytes = include_bytes!("../assets/logo_transparent_orangetext.png");
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
        .with_inner_size([862.0, 540.0]) 
        .with_min_inner_size([800.0, 500.0])
        .with_decorations(false) // Start frameless for splash screen
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

impl OpenDavApp {
    pub fn load_telemetry_file(&mut self, path: &std::path::Path) {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        self.active_file = Some(file_name.clone());
        match crate::data::ibt_parser::parse_ibt_file(path.to_str().unwrap_or("")) {
            Ok(parsed_session) => {
                self.session_loaded = true;
                crate::signals::processing::trigger_track_map_download(parsed_session.track_id);
                
                match crate::LoadedSession::new(file_name, parsed_session, self.settings.corner_merge_threshold, &self.settings.mapbox_api_key) {
                    Ok(new_session) => {
                        self.sessions.push(new_session);
                        let new_idx = self.sessions.len() - 1;
                        self.primary_session_idx = new_idx;
                        let fastest = crate::signals::processing::get_fastest_lap(&self.sessions[new_idx].session.lap_times);
                        self.selected_lap = if fastest > 0 { Some((new_idx, fastest)) } else { None };
                        self.cursor_x = None;
                        self.update_sector_deltas();
                        self.reset_bounds_flag = true;
                        self.reset_bounds_next_frame = 3;
                    },
                    Err(e) => {
                        eprintln!("Failed to initialize session: {}", e);
                    }
                }
                
                // Automatically switch to Dashboard if loaded successfully
                self.active_page = ActivePage::OpenDav;
            }
            Err(e) => {
                eprintln!("Error parsing .ibt file: {}", e);
            }
        }
    }
}

impl LoadedSession {
    pub fn fetch_satellite_maps(&mut self, mapbox_api_key: &str) {
        if let Some((min_lon, min_lat, max_lon, max_lat)) = self.track_bounds {
            let track_id = self.session.track_id;
            let mapbox_path = std::path::Path::new("assets/maps").join(format!("{}_dark.png", track_id));
            let google_path = std::path::Path::new("assets/maps").join(format!("{}_google.png", track_id));
            
            let fetch_result = if mapbox_path.exists() {
                crate::signals::mapbox::fetch_mapbox_image(mapbox_api_key, track_id, min_lon, min_lat, max_lon, max_lat, 16)
            } else if google_path.exists() {
                crate::signals::google_maps::fetch_google_map_image(track_id, min_lon, min_lat, max_lon, max_lat, 16)
            } else {
                if !mapbox_api_key.trim().is_empty() {
                    let res = crate::signals::mapbox::fetch_mapbox_image(mapbox_api_key, track_id, min_lon, min_lat, max_lon, max_lat, 16);
                    if res.is_ok() {
                        res
                    } else {
                        crate::signals::google_maps::fetch_google_map_image(track_id, min_lon, min_lat, max_lon, max_lat, 16)
                    }
                } else {
                    crate::signals::google_maps::fetch_google_map_image(track_id, min_lon, min_lat, max_lon, max_lat, 16)
                }
            };
            
            if let Ok((fg_b, fg_bnds, bg_b, bg_bnds)) = fetch_result {
                self.fg_image_bytes = Some(fg_b);
                self.fg_bounds = Some(fg_bnds);
                self.bg_image_bytes = bg_b;
                self.bg_bounds = bg_bnds;
            }
        }
    }
}

