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
    Reports, // Sector/Corner Reports
}

#[derive(Clone, Debug)]
pub struct TrackSector {
    pub name: String,
    pub start_dist: f64,
    pub end_dist: f64,
}

#[derive(Clone, Debug)]
pub struct LapData {
    pub lap_num: i32,
    pub dist: Vec<f64>,
    pub time: Vec<f64>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(PartialEq, Clone, Copy)]
#[allow(dead_code)]
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

    // Shared horizontal view bounds to perfectly synchronize Zoom/Pan/Scroll across different tabs!
    visible_x_range: Option<(f64, f64)>,

    // Tracks worksheet changes to execute tab-sync bounds on switch frames cleanly!
    previous_worksheet: Option<WorksheetTab>,

    // Sector reports caches
    sectors: Vec<TrackSector>,
    sector_bests: Vec<f64>,
    lap_data_cache: Vec<LapData>,
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

// Slice only the cache points that correspond to a specific lap number (Standalone static helper!)
fn get_lap_points_slice<'a>(lap_ranges: &[(i32, f64, f64)], cache: &'a [[f64; 2]], lap_num: i32) -> &'a [[f64; 2]] {
    if let Some(pos) = lap_ranges.iter().position(|r| r.0 == lap_num) {
        let (_, start_t, end_t) = lap_ranges[pos];
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

// Determine which lap the cursor is currently inside (Standalone static helper!)
fn get_active_lap(lap_ranges: &[(i32, f64, f64)], cursor_x: Option<f64>, selected_lap: Option<i32>) -> i32 {
    let cx = cursor_x.unwrap_or(0.0);
    for &(lap_num, start_t, end_t) in lap_ranges {
        if cx >= start_t && cx <= end_t {
            return lap_num;
        }
    }
    selected_lap.unwrap_or(1)
}

// Calculates the fastest lap by ignoring the first 3 laps if there are more than 3 laps in the session.
fn get_fastest_lap(lap_times: &[(i32, f64)]) -> i32 {
    let filtered: Vec<&(i32, f64)> = lap_times.iter()
        .filter(|(lap_num, _)| *lap_num > 3)
        .collect();
    if !filtered.is_empty() {
        filtered.iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|val| val.0)
            .unwrap_or(0)
    } else {
        lap_times.iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|val| val.0)
            .unwrap_or(0)
    }
}

// Finds the exact time elapsed at a target distance on a lap by linear interpolation.
fn get_lap_time_at_distance(lap_dist: &[f64], lap_time: &[f64], target_dist: f64) -> f64 {
    if lap_dist.is_empty() {
        return 0.0;
    }
    if target_dist <= lap_dist[0] {
        return lap_time[0];
    }
    if target_dist >= lap_dist[lap_dist.len() - 1] {
        return lap_time[lap_time.len() - 1];
    }
    match lap_dist.binary_search_by(|val| val.partial_cmp(&target_dist).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => lap_time[idx],
        Err(idx) => {
            if idx == 0 {
                lap_time[0]
            } else if idx >= lap_dist.len() {
                lap_time[lap_time.len() - 1]
            } else {
                let d0 = lap_dist[idx - 1];
                let d1 = lap_dist[idx];
                let t0 = lap_time[idx - 1];
                let t1 = lap_time[idx];
                t0 + (t1 - t0) * ((target_dist - d0) / (d1 - d0))
            }
        }
    }
}

// Finds the coordinate (x, y) at a target distance on a lap by linear interpolation.
fn get_lap_coord_at_distance(lap: &LapData, target_dist: f64) -> (f64, f64) {
    if lap.dist.is_empty() {
        return (0.0, 0.0);
    }
    if target_dist <= lap.dist[0] {
        return (lap.x[0], lap.y[0]);
    }
    if target_dist >= lap.dist[lap.dist.len() - 1] {
        return (lap.x[lap.x.len() - 1], lap.y[lap.y.len() - 1]);
    }
    match lap.dist.binary_search_by(|val| val.partial_cmp(&target_dist).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => (lap.x[idx], lap.y[idx]),
        Err(idx) => {
            if idx == 0 {
                (lap.x[0], lap.y[0])
            } else if idx >= lap.dist.len() {
                (lap.x[lap.x.len() - 1], lap.y[lap.y.len() - 1])
            } else {
                let d0 = lap.dist[idx - 1];
                let d1 = lap.dist[idx];
                let x0 = lap.x[idx - 1];
                let x1 = lap.x[idx];
                let y0 = lap.y[idx - 1];
                let y1 = lap.y[idx];
                let pct = (target_dist - d0) / (d1 - d0);
                (x0 + (x1 - x0) * pct, y0 + (y1 - y0) * pct)
            }
        }
    }
}

// Finds the coordinate (x, y) at a target relative time on a lap by linear interpolation.
fn get_lap_coord_at_time(lap: &LapData, target_time: f64) -> (f64, f64) {
    if lap.time.is_empty() {
        return (0.0, 0.0);
    }
    if target_time <= lap.time[0] {
        return (lap.x[0], lap.y[0]);
    }
    if target_time >= lap.time[lap.time.len() - 1] {
        return (lap.x[lap.x.len() - 1], lap.y[lap.y.len() - 1]);
    }
    match lap.time.binary_search_by(|val| val.partial_cmp(&target_time).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => (lap.x[idx], lap.y[idx]),
        Err(idx) => {
            if idx == 0 {
                (lap.x[0], lap.y[0])
            } else if idx >= lap.time.len() {
                (lap.x[lap.x.len() - 1], lap.y[lap.y.len() - 1])
            } else {
                let t0 = lap.time[idx - 1];
                let t1 = lap.time[idx];
                let x0 = lap.x[idx - 1];
                let x1 = lap.x[idx];
                let y0 = lap.y[idx - 1];
                let y1 = lap.y[idx];
                let pct = (target_time - t0) / (t1 - t0);
                (x0 + (x1 - x0) * pct, y0 + (y1 - y0) * pct)
            }
        }
    }
}

// Splits a lap's coordinates into multiple continuous segments to prevent drawing straight lines across teleportations/resets
fn get_lap_segments(lap: &LapData) -> Vec<PlotPoints<'_>> {
    let mut segments = Vec::new();
    let n = lap.x.len();
    if n == 0 {
        return segments;
    }
    
    let max_jump = 50.0; // 50 meters jump threshold
    let mut current_segment = Vec::new();
    
    current_segment.push([lap.x[0], lap.y[0]]);
    
    for i in 1..n {
        let x0 = lap.x[i - 1];
        let y0 = lap.y[i - 1];
        let x1 = lap.x[i];
        let y1 = lap.y[i];
        
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        
        if dist > max_jump {
            if !current_segment.is_empty() {
                segments.push(PlotPoints::from(current_segment));
                current_segment = Vec::new();
            }
        }
        current_segment.push([x1, y1]);
    }
    
    if !current_segment.is_empty() {
        segments.push(PlotPoints::from(current_segment));
    }
    
    segments
}

// Automatically detects track corners and straights based on the fastest lap's lateral G-force and steering angle.
fn detect_track_sectors(session: &ibt_parser::IbtSession) -> Vec<TrackSector> {
    let mut sectors = Vec::new();
    let fastest_lap = get_fastest_lap(&session.lap_times);
    if fastest_lap <= 0 {
        return sectors;
    }
    let df = &session.dataframe;
    let lap_col = match df.column("Lap").ok().and_then(|c| c.f64().ok()) {
        Some(col) => col,
        None => return sectors,
    };
    let dist_col = match df.column("Distance_Derived").ok().and_then(|c| c.f64().ok()) {
        Some(col) => col,
        None => return sectors,
    };
    let lat_accel_col = match df.column("LatAccel").ok().and_then(|c| c.f64().ok()) {
        Some(col) => col,
        None => return sectors,
    };
    let steer_col = match df.column("SteeringWheelAngle").ok().and_then(|c| c.f64().ok()) {
        Some(col) => col,
        None => return sectors,
    };
    let mut lap_dist = Vec::new();
    let mut lap_lat_accel = Vec::new();
    let mut lap_steer = Vec::new();
    let n = lap_col.len();
    for i in 0..n {
        if lap_col.get(i).unwrap_or(0.0) as i32 == fastest_lap {
            lap_dist.push(dist_col.get(i).unwrap_or(0.0));
            lap_lat_accel.push(lat_accel_col.get(i).unwrap_or(0.0));
            lap_steer.push(steer_col.get(i).unwrap_or(0.0));
        }
    }
    if lap_dist.is_empty() {
        return sectors;
    }
    let start_dist = lap_dist[0];
    let lap_len = lap_dist[lap_dist.len() - 1] - start_dist;
    for d in &mut lap_dist {
        *d -= start_dist;
    }
    let w_size = 20;
    let smoothed_lat = ibt_parser::moving_average(&lap_lat_accel, w_size);
    let smoothed_steer = ibt_parser::moving_average(&lap_steer, w_size);
    let mut is_corner = vec![false; lap_dist.len()];
    for i in 0..lap_dist.len() {
        let abs_lat = smoothed_lat[i].abs();
        let abs_steer = smoothed_steer[i].abs();
        if abs_lat > 3.0 || abs_steer > 0.08 {
            is_corner[i] = true;
        }
    }
    let mut raw_corners = Vec::new();
    let mut in_corner = false;
    let mut start_idx = 0;
    for i in 0..is_corner.len() {
        if is_corner[i] {
            if !in_corner {
                in_corner = true;
                start_idx = i;
            }
        } else {
            if in_corner {
                in_corner = false;
                raw_corners.push((start_idx, i - 1));
            }
        }
    }
    if in_corner {
        raw_corners.push((start_idx, is_corner.len() - 1));
    }
    let mut merged_corners = Vec::new();
    if !raw_corners.is_empty() {
        let mut curr = raw_corners[0];
        for next in raw_corners.iter().skip(1) {
            let gap_dist = lap_dist[next.0] - lap_dist[curr.1];
            if gap_dist < 25.0 {
                curr.1 = next.1;
            } else {
                merged_corners.push(curr);
                curr = *next;
            }
        }
        merged_corners.push(curr);
    }
    let mut final_corners = Vec::new();
    for corner in merged_corners {
        let length = lap_dist[corner.1] - lap_dist[corner.0];
        if length >= 20.0 {
            final_corners.push(corner);
        }
    }
    if final_corners.is_empty() {
        let s1 = lap_len / 3.0;
        let s2 = 2.0 * lap_len / 3.0;
        sectors.push(TrackSector { name: "Sector 1".to_string(), start_dist: 0.0, end_dist: s1 });
        sectors.push(TrackSector { name: "Sector 2".to_string(), start_dist: s1, end_dist: s2 });
        sectors.push(TrackSector { name: "Sector 3".to_string(), start_dist: s2, end_dist: lap_len });
        return sectors;
    }
    let first_corner_start = lap_dist[final_corners[0].0];
    if first_corner_start > 10.0 {
        sectors.push(TrackSector {
            name: "Str 0-1 (Start)".to_string(),
            start_dist: 0.0,
            end_dist: first_corner_start,
        });
    }
    for i in 0..final_corners.len() {
        let t_start = lap_dist[final_corners[i].0];
        let t_end = lap_dist[final_corners[i].1];
        let turn_num = i + 1;
        sectors.push(TrackSector {
            name: format!("Turn {}", turn_num),
            start_dist: t_start,
            end_dist: t_end,
        });
        if i + 1 < final_corners.len() {
            let next_start = lap_dist[final_corners[i + 1].0];
            sectors.push(TrackSector {
                name: format!("Str {}-{}", turn_num, turn_num + 1),
                start_dist: t_end,
                end_dist: next_start,
            });
        }
    }
    let last_corner_end = lap_dist[final_corners[final_corners.len() - 1].1];
    if lap_len - last_corner_end > 10.0 {
        sectors.push(TrackSector {
            name: "Str 0-1 (End)".to_string(),
            start_dist: last_corner_end,
            end_dist: lap_len,
        });
    }
    sectors
}

// Formats sector split duration in seconds into SS.SSS or MM:SS.SSS
fn format_sector_time(sec: f64) -> String {
    if sec <= 0.0 || sec == f64::MAX {
        return "-".to_string();
    }
    if sec >= 60.0 {
        let minutes = (sec / 60.0).floor() as i32;
        let seconds = (sec % 60.0).floor() as i32;
        let ms = ((sec % 1.0) * 1000.0).round() as i32;
        format!("{:02}:{:02}.{:03}", minutes, seconds, ms)
    } else {
        let seconds = sec.floor() as i32;
        let ms = ((sec % 1.0) * 1000.0).round() as i32;
        format!("{:02}.{:03}", seconds, ms)
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

            // Rebuild track sectors and sector bests cache
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
        }
    }
}

// --- REUSABLE MOTEC WORKSPACE PLOTTER ENGINE ---
// Zero-allocation, multi-lane, double-click zoom capable plot drawer!
// Ensures 100% consistent interactions across ALL current and future tabs!
impl OpenDavApp {
    fn draw_motec_plot(&mut self, ui: &mut egui::Ui, plot_id: &str, worksheet: WorksheetTab, is_tab_switch: bool) {
        if self.front_pts_cache.is_empty() { return; }
        
        let max_time = self.front_pts_cache.last().unwrap()[0];
        let is_dark = ui.style().visuals.dark_mode;

        // 1. EXTRACT RAW HUD METRICS AT PLAYBACK CURSOR INDEX (EXCLUSIVE ZERO-CONFLICT SCOPE!)
        let mut raw_val_speed = 0.0;
        let mut raw_val_throttle = 0.0;
        let mut raw_val_brake = 0.0;
        let mut raw_val_steering = 0.0;
        let mut raw_val_rpm = 0.0;
        let mut raw_val_gear = 0.0;
        let mut raw_val_front = 0.0;
        let mut raw_val_rear = 0.0;
        let mut raw_val_rake = 0.0;

        if let Some(cx) = self.cursor_x {
            let idx = get_closest_index(&self.speed_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), cx);
            if let Some(session) = &self.session {
                let speed_col = session.dataframe.column("Speed").ok().map(|c| c.f64().ok()).flatten();
                let throttle_col = session.dataframe.column("Throttle").ok().map(|c| c.f64().ok()).flatten();
                let brake_col = session.dataframe.column("Brake").ok().map(|c| c.f64().ok()).flatten();
                let steering_col = session.dataframe.column("SteeringWheelAngle").ok().map(|c| c.f64().ok()).flatten();
                let rpm_col = session.dataframe.column("RPM").ok().map(|c| c.f64().ok()).flatten();
                let gear_col = session.dataframe.column("Gear").ok().map(|c| c.f64().ok()).flatten();

                raw_val_speed = speed_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 3.6;
                raw_val_throttle = throttle_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 100.0;
                raw_val_brake = brake_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 100.0;
                raw_val_steering = steering_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0) * 57.2958;
                raw_val_rpm = rpm_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0);
                raw_val_gear = gear_col.map(|c| c.get(idx).unwrap_or(0.0)).unwrap_or(0.0);

                if idx < session.front_smooth.len() {
                    let scale = if session.front_smooth[idx] < 0.5 { 1000.0 } else { 1.0 };
                    raw_val_front = session.front_smooth[idx] * scale;
                    raw_val_rear = session.rear_smooth[idx] * scale;
                    raw_val_rake = session.rake[idx] * scale;
                }
            }
        }

        // 2. CONSTRUCT LANES ACCORDING TO WORKSHEET TYPE
        let lanes = match worksheet {
            WorksheetTab::Basic => vec![
                ChartLane {
                    title: "Ground Speed",
                    y_min: 76.0,
                    y_max: 98.0,
                    traces: vec![
                        ChartTrace { name: "Speed", scaled_pts: &self.speed_pts_cache, color: SPEED_COLOR, width: 2.2, raw_val: raw_val_speed, unit: " km/h" },
                    ],
                },
                ChartLane {
                    title: "Engine RPM",
                    y_min: 52.0,
                    y_max: 72.0,
                    traces: vec![
                        ChartTrace { name: "RPM", scaled_pts: &self.rpm_pts_cache, color: egui::Color32::from_rgb(241, 196, 15), width: 2.2, raw_val: raw_val_rpm, unit: "" },
                    ],
                },
                ChartLane {
                    title: "Pedal Inputs",
                    y_min: 28.0,
                    y_max: 48.0,
                    traces: vec![
                        ChartTrace { name: "Throttle", scaled_pts: &self.throttle_pts_cache, color: egui::Color32::from_rgb(46, 204, 113), width: 2.2, raw_val: raw_val_throttle, unit: "%" },
                        ChartTrace { name: "Brake", scaled_pts: &self.brake_pts_cache, color: egui::Color32::from_rgb(231, 76, 60), width: 2.2, raw_val: raw_val_brake, unit: "%" },
                    ],
                },
                ChartLane {
                    title: "Steering",
                    y_min: 10.0,
                    y_max: 24.0,
                    traces: vec![
                        ChartTrace { name: "Steering Angle", scaled_pts: &self.steering_pts_cache, color: SUB_ACCENT_COLOR, width: 2.2, raw_val: raw_val_steering, unit: "°" },
                    ],
                },
            ],
            WorksheetTab::DynamicRake => vec![
                ChartLane {
                    title: "Ground Speed",
                    y_min: 70.0,
                    y_max: 98.0,
                    traces: vec![
                        ChartTrace { name: "Speed", scaled_pts: &self.speed_pts_cache, color: SPEED_COLOR, width: 2.2, raw_val: raw_val_speed, unit: " km/h" },
                    ],
                },
                ChartLane {
                    title: "Axle Heights",
                    y_min: 40.0,
                    y_max: 66.0,
                    traces: vec![
                        ChartTrace { name: "Front RH", scaled_pts: &self.front_pts_cache, color: SUB_ACCENT_COLOR, width: 2.2, raw_val: raw_val_front, unit: "mm" },
                        ChartTrace { name: "Rear RH", scaled_pts: &self.rear_pts_cache, color: egui::Color32::from_rgb(255, 20, 147), width: 2.2, raw_val: raw_val_rear, unit: "mm" },
                    ],
                },
                ChartLane {
                    title: "Chassis Attitude",
                    y_min: 12.0,
                    y_max: 36.0,
                    traces: vec![
                        ChartTrace { name: "Dynamic Rake", scaled_pts: &self.rake_pts_cache, color: ACCENT_COLOR, width: 2.2, raw_val: raw_val_rake, unit: "mm" },
                    ],
                },
            ],
            _ => vec![],
        };

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
        if plot_height < 300.0 { plot_height = 300.0; }

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
        let lap_ranges = &self.lap_ranges;
        let ref_lap_cyan = self.ref_lap_cyan;
        let ref_lap_white = self.ref_lap_white;
        let lap_markers = &self.lap_markers;

        plot.show(ui, |plot_ui| {
            let is_left_click_down = plot_ui.ctx().input(|i| i.pointer.button_down(egui::PointerButton::Primary));

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
                    if let Some(pos) = lap_ranges.iter().position(|r| r.0 == sel_lap) {
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
            if let Some(ref_lap_num) = ref_lap_cyan {
                if let Some(pos) = lap_ranges.iter().position(|r| r.0 == ref_lap_num) {
                    let ref_start = lap_ranges[pos].1;
                    
                    for &(lap_num, start_t, end_t) in lap_ranges {
                        if end_t >= min_visible_x && start_t <= max_visible_x {
                            let offset = start_t - ref_start;
                            
                            for lane in &lanes {
                                for trace in &lane.traces {
                                    let ref_slice = get_lap_points_slice(lap_ranges, trace.scaled_pts, ref_lap_num);
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

            // White reference overlays
            if let Some(ref_lap_num) = ref_lap_white {
                if let Some(pos) = lap_ranges.iter().position(|r| r.0 == ref_lap_num) {
                    let ref_start = lap_ranges[pos].1;
                    
                    for &(lap_num, start_t, end_t) in lap_ranges {
                        if end_t >= min_visible_x && start_t <= max_visible_x {
                            let offset = start_t - ref_start;
                            
                            for lane in &lanes {
                                for trace in &lane.traces {
                                    let ref_slice = get_lap_points_slice(lap_ranges, trace.scaled_pts, ref_lap_num);
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
                    
                    // FIXED: Text label color contrast bug! Uses high-contrast light grey-blue (DARK mode) or charcoal grey (LIGHT mode)
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
                    plot_ui.text(Text::new(format!("LapLabelMarker_{}", lap_num), PlotPoint::new(center, 99.0), egui::RichText::new(label_str).color(label_txt_color).size(10.0).strong()));
                }
            }

            // J. DRAW PLAYBACK CURSOR DOTS
            if let Some(cx) = cursor_x {
                plot_ui.vline(VLine::new("Cursor Line", cx).color(ACCENT_COLOR).width(1.5));
                let idx = get_closest_index(&self.front_pts_cache.iter().map(|p| p[0]).collect::<Vec<f64>>(), cx);
                
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
            if plot_ui.ctx().input(|i| i.pointer.any_pressed()) {
                if let Some(pointer_pos) = plot_ui.pointer_coordinate() {
                    is_dragging_ticker = pointer_pos.y < 9.5;
                }
            }

            if is_left_click_down {
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

            // M. SILKY-SMOOTH HIGH-PRECISION ZOOM WHEEL
            if !is_left_click_down {
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

// Formats seconds into MM:SS.SSS
fn height_offset(ui: &egui::Ui) -> f32 {
    let screen_height = ui.ctx().screen_rect().height();
    screen_height / 2.0
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
                            ActivePage::OpenDav | ActivePage::Reports => {
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
                                                         let fastest_lap = get_fastest_lap(&session.lap_times);
                                                         self.selected_lap = if fastest_lap > 0 { Some(fastest_lap) } else { None };
                                                        self.rebuild_points_cache();
                                                    }
                                                }
                                            }
                                        });

                                    ui.add_space(15.0);

                                    // 3. Reports Image Button
                                    let rep_bytes = include_bytes!("../assets/button_reports.png");
                                    let is_rep_selected = self.active_page == ActivePage::Reports;
                                    let border_color_rep = if is_rep_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };

                                    egui::Frame::none()
                                        .stroke(egui::Stroke::new(2.0, border_color_rep))
                                        .rounding(8.0)
                                        .inner_margin(1.0)
                                        .show(ui, |ui| {
                                            let img_rep = egui::Image::from_bytes("bytes://button_reports.png", rep_bytes.to_vec())
                                                .max_width(240.0)
                                                .rounding(8.0)
                                                .sense(egui::Sense::click());
                                            let resp = ui.add(img_rep);
                                            if resp.clicked() {
                                                self.active_page = ActivePage::Reports;
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

                                    let select_hdr_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
                                    ui.label(egui::RichText::new("LAP TIMELINE SELECT").color(select_hdr_color).size(10.0).strong());
                                    ui.add_space(8.0);

                                    if !self.session_loaded || self.session.is_none() {
                                        ui.label(egui::RichText::new("No Session Active").color(egui::Color32::GRAY).small());
                                    } else {
                                        let lap_times = if let Some(session) = &self.session {
                                            session.lap_times.clone()
                                        } else {
                                            Vec::new()
                                        };

                                        let fastest_lap = get_fastest_lap(&lap_times);

                                        let sidebar_style = ui.style_mut();
                                        sidebar_style.spacing.button_padding = egui::vec2(12.0, 8.0);

                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                for (lap_num, duration) in &lap_times {
                                                    let is_selected = self.selected_lap == Some(*lap_num);
                                                    let is_fastest = *lap_num == fastest_lap && *lap_num > 0;

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
                                                        let active_cyan = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 136, 170) };
                                                        let border_color_c = if is_cyan { active_cyan } else { egui::Color32::TRANSPARENT };
                                                        
                                                        let btn_c = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_c))
                                                            .rounding(4.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                                            .show(ui, |ui| {
                                                                ui.selectable_label(false, egui::RichText::new("C").color(if is_cyan { active_cyan } else { egui::Color32::DARK_GRAY }).strong())
                                                            }).inner;
                                                        
                                                        if btn_c.clicked() {
                                                            if is_cyan {
                                                                self.ref_lap_cyan = None;
                                                            } else {
                                                                self.ref_lap_cyan = Some(*lap_num);
                                                            }
                                                        }

                                                        // 2. White Reference Toggle Box (Right)
                                                        let active_white = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(40, 40, 40) };
                                                        let border_color_w = if is_white { active_white } else { egui::Color32::TRANSPARENT };
                                                        
                                                        let btn_w = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_w))
                                                            .rounding(4.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                                            .show(ui, |ui| {
                                                                ui.selectable_label(false, egui::RichText::new("W").color(if is_white { active_white } else { egui::Color32::DARK_GRAY }).strong())
                                                            }).inner;
                                                        
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

                                                        let border_color_l = if is_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };
                                                        
                                                        let btn_l = egui::Frame::none()
                                                            .stroke(egui::Stroke::new(1.0, border_color_l))
                                                            .rounding(4.0)
                                                            .inner_margin(egui::Margin::symmetric(6, 3))
                                                            .show(ui, |ui| {
                                                                ui.selectable_label(false, egui::RichText::new(text).color(label_color).strong())
                                                            }).inner;

                                                        if btn_l.clicked() {
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
                            ui.label(egui::RichText::new("v0.1.0-rs").color(egui::Color32::DARK_GRAY).small());
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
                                             let fastest = get_fastest_lap(&session.lap_times);
                                             self.selected_lap = if fastest > 0 { Some(fastest) } else { None };
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

                // --- CENTRAL PAGE RENDERER ---
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

                                ui.heading(egui::RichText::new("Dashboard").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
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
                                            let avg_lap = {
                                                let filtered: Vec<&(i32, f64)> = lap_times.iter()
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
                                            ui.label(egui::RichText::new("AVERAGE LAP TIME").color(egui::Color32::DARK_GRAY).small().strong());
                                            ui.add_space(4.0);
                                            ui.heading(egui::RichText::new(format_lap_time(avg_lap)).strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                        });
                                    });
                                });

                                ui.add_space(15.0);

                                let fastest_lap = get_fastest_lap(&lap_times);

                                // 3. Stacked lower Dashboard (Top: Laps List, Bottom: Huge Track Map SVG!)
                                ui.vertical(|ui| {
                                    let sheet_hdr_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
                                    ui.label(egui::RichText::new("VALID LAP SHEET").color(sheet_hdr_color).strong().size(11.0));
                                    ui.add_space(4.0);

                                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                        egui::Grid::new("valid_laps_grid")
                                            .striped(true)
                                            .min_col_width(200.0)
                                            .spacing([24.0, 10.0])
                                            .show(ui, |ui| {
                                                let cols = 4;
                                                let mut col_count = 0;
                                                for (lap_num, duration) in &lap_times {
                                                    let is_fastest = *lap_num == fastest_lap;
                                                    let is_selected = self.selected_lap == Some(*lap_num);
                                                    
                                                    let mut row_text = format!("Lap {} : {}", lap_num, format_lap_time(*duration));
                                                    if is_fastest {
                                                        row_text += " ★ FASTEST";
                                                    }
                                                    
                                                    let row_color = if is_selected {
                                                        ACCENT_COLOR
                                                    } else if is_fastest {
                                                        SUB_ACCENT_COLOR
                                                    } else {
                                                        if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }
                                                    };
                                                    
                                                     let border_color = if is_selected { ACCENT_COLOR } else { egui::Color32::TRANSPARENT };
                                                     let btn_resp = egui::Frame::none()
                                                         .stroke(egui::Stroke::new(1.0, border_color))
                                                         .rounding(4.0)
                                                         .inner_margin(egui::Margin::symmetric(6, 3))
                                                         .show(ui, |ui| {
                                                             ui.selectable_label(false, egui::RichText::new(row_text).color(row_color).strong())
                                                         }).inner;

                                                     if btn_resp.clicked() {
                                                         self.selected_lap = Some(*lap_num);
                                                         if let Some(pos) = self.lap_ranges.iter().position(|r| r.0 == *lap_num) {
                                                             let (_, start_t, _end_t) = self.lap_ranges[pos];
                                                             self.cursor_x = Some(start_t);
                                                             self.reset_bounds_flag = true;
                                                         }
                                                     }
                                                    
                                                    col_count += 1;
                                                    if col_count >= cols {
                                                        ui.end_row();
                                                        col_count = 0;
                                                    }
                                                }
                                            });
                                    });
                                });

                                ui.add_space(15.0);
                                let map_hdr_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
                                ui.label(egui::RichText::new(venue.to_uppercase()).color(map_hdr_color).strong().size(11.0));
                                ui.add_space(4.0);

                                ui.group(|ui| {
                                    self.draw_interactive_track_map(ui, 340.0);
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

                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::Basic, "1. Basic (Inputs)");
                                    ui.selectable_value(&mut self.active_worksheet, WorksheetTab::DynamicRake, "2. Dynamic Rake");
                                });

                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);

                                // Calculate if tab was switched this frame to trigger shared viewport boundary syncing!
                                let mut is_tab_switch = false;
                                if Some(self.active_worksheet) != self.previous_worksheet {
                                    is_tab_switch = true;
                                    self.previous_worksheet = Some(self.active_worksheet);
                                }

                                // 2. ACTIVE WORKSHEET PLOTTING AREA (SINGLE INTEGRATED HIGH-PERFORMANCE PLOT ENVIRONMENT!)
                                match self.active_worksheet {
                                    WorksheetTab::Basic => {
                                        self.draw_motec_plot(ui, "basic_worksheet_canvas", WorksheetTab::Basic, is_tab_switch);
                                    }
                                    WorksheetTab::DynamicRake => {
                                        self.draw_motec_plot(ui, "rake_worksheet_canvas", WorksheetTab::DynamicRake, is_tab_switch);
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
                        ActivePage::Reports => {
                            if !self.session_loaded || self.session.is_none() {
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                                        ui.label(egui::RichText::new("Please load an iRacing .ibt file from the top taskbar to view reports.").color(egui::Color32::GRAY));
                                    });
                                });
                            } else {
                                let session_ref = self.session.as_ref().unwrap();
                                let venue = session_ref.venue.clone();

                                ui.horizontal(|ui| {
                                    ui.heading(egui::RichText::new("Sector Analysis Report").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(venue.to_uppercase()).strong().color(ACCENT_COLOR));
                                    });
                                });
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);

                                if self.lap_data_cache.is_empty() || self.sectors.is_empty() {
                                    ui.label("No sector or lap data available for report.");
                                } else {
                                    ui.columns(2, |cols| {
                                        cols[0].vertical(|ui| {
                                            let mut visible_laps: Vec<&LapData> = self.lap_data_cache.iter()
                                                .filter(|lap| lap.lap_num > 3)
                                                .collect();
                                            if visible_laps.is_empty() {
                                                visible_laps = self.lap_data_cache.iter().collect();
                                            }

                                            let best_total_time = visible_laps.iter()
                                                .map(|lap| lap.time.last().copied().unwrap_or(0.0))
                                                .filter(|&t| t > 0.0)
                                                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                                                .unwrap_or(0.0);

                                            egui::ScrollArea::both()
                                                .id_source("sector_report_scroll")
                                                .show(ui, |ui| {
                                                    egui::Grid::new("sector_report_grid")
                                                        .striped(true)
                                                        .min_col_width(85.0)
                                                        .spacing([20.0, 12.0])
                                                        .show(ui, |ui| {
                                                            // Header Row
                                                            ui.label(egui::RichText::new("Sector / Corner").strong());
                                                            for lap in &visible_laps {
                                                                ui.label(egui::RichText::new(format!("Lap {}", lap.lap_num)).strong());
                                                            }
                                                            ui.label(egui::RichText::new("Optimal").strong().color(ACCENT_COLOR));
                                                            ui.end_row();

                                                            // Sector split rows
                                                            for (s_idx, sector) in self.sectors.iter().enumerate() {
                                                                ui.label(egui::RichText::new(&sector.name).strong());

                                                                let best_s_time = self.sector_bests.get(s_idx).copied().unwrap_or(0.0);

                                                                for lap in &visible_laps {
                                                                    let t_start = get_lap_time_at_distance(&lap.dist, &lap.time, sector.start_dist);
                                                                    let t_end = get_lap_time_at_distance(&lap.dist, &lap.time, sector.end_dist);
                                                                    let s_time = t_end - t_start;

                                                                    let is_session_best = s_time > 0.0 && (s_time - best_s_time).abs() < 1e-4;
                                                                    let is_near_best = s_time > 0.0 && s_time <= best_s_time * 1.015;

                                                                    let cell_color = if is_session_best {
                                                                        if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) }
                                                                    } else if is_near_best {
                                                                        if is_dark { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(34, 112, 34) }
                                                                    } else {
                                                                        if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY }
                                                                    };

                                                                    let mut text = egui::RichText::new(format_sector_time(s_time)).color(cell_color);
                                                                    if is_session_best || is_near_best {
                                                                        text = text.strong();
                                                                    }
                                                                    ui.label(text);
                                                                }

                                                                let opt_color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                                                                ui.label(egui::RichText::new(format_sector_time(best_s_time)).color(opt_color).strong());
                                                                ui.end_row();
                                                            }

                                                            // Totals Row
                                                            ui.label(egui::RichText::new("TOTAL").strong().color(ACCENT_COLOR));
                                                            for lap in &visible_laps {
                                                                let total_time = lap.time.last().copied().unwrap_or(0.0);

                                                                let is_total_best = total_time > 0.0 && (total_time - best_total_time).abs() < 1e-4;
                                                                let is_total_near_best = total_time > 0.0 && total_time <= best_total_time * 1.015;

                                                                let total_color = if is_total_best {
                                                                    if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) }
                                                                } else if is_total_near_best {
                                                                    if is_dark { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(34, 112, 34) }
                                                                } else {
                                                                    if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }
                                                                };

                                                                let mut text = egui::RichText::new(format_lap_time(total_time)).color(total_color);
                                                                if is_total_best || is_total_near_best {
                                                                    text = text.strong();
                                                                }
                                                                ui.label(text);
                                                            }

                                                            let optimal_total = self.sector_bests.iter().sum::<f64>();
                                                            let opt_total_color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                                                            ui.label(egui::RichText::new(format_lap_time(optimal_total)).color(opt_total_color).strong());
                                                            ui.end_row();
                                                        });
                                                });
                                        });

                                        cols[1].vertical(|ui| {
                                            ui.heading(egui::RichText::new("Track Layout Map").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                                            ui.add_space(8.0);
                                            ui.group(|ui| {
                                                self.draw_interactive_track_map(ui, 450.0);
                                            });
                                        });
                                    });
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

impl OpenDavApp {
    fn draw_interactive_track_map(&self, ui: &mut egui::Ui, height: f32) {
        if self.lap_data_cache.is_empty() {
            ui.label("No track map coordinates precomputed.");
            return;
        }

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

        plot.show(ui, |plot_ui| {
            // 1. Draw Reference Laps (underneath)
            if let Some(lap) = ref_cyan_lap {
                let color = if is_dark { egui::Color32::from_rgb(0, 255, 255) } else { egui::Color32::from_rgb(0, 120, 136) };
                let segments = get_lap_segments(lap);
                for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                    plot_ui.line(Line::new(format!("Ref Lap {} (Cyan) - Seg {}", self.ref_lap_cyan.unwrap(), seg_idx), seg_pts)
                        .color(color)
                        .width(1.8)
                    );
                }
            }

            if let Some(lap) = ref_white_lap {
                let color = if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 100) };
                let segments = get_lap_segments(lap);
                for (seg_idx, seg_pts) in segments.into_iter().enumerate() {
                    plot_ui.line(Line::new(format!("Ref Lap {} (White) - Seg {}", self.ref_lap_white.unwrap(), seg_idx), seg_pts)
                        .color(color)
                        .width(1.8)
                    );
                }
            }

            // 2. Draw Active Lap
            let active_color = if is_dark { egui::Color32::from_rgb(255, 255, 255) } else { egui::Color32::from_rgb(10, 10, 10) };
            let active_segments = get_lap_segments(active_lap);
            for (seg_idx, seg_pts) in active_segments.into_iter().enumerate() {
                plot_ui.line(Line::new(format!("Lap {} - Seg {}", active_lap_num, seg_idx), seg_pts)
                    .color(active_color)
                    .width(3.0)
                );
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

            // 4. Draw Turn Labels at corner midpoints
            for sector in &self.sectors {
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
                    
                    // Offset the text slightly by 15 meters along normal
                    let offset_dist = 15.0;
                    let label_x = tx + nx * offset_dist;
                    let label_y = ty + ny * offset_dist;

                    let label_color = if is_dark { egui::Color32::LIGHT_GRAY } else { egui::Color32::DARK_GRAY };
                    plot_ui.text(Text::new(
                        &sector.name,
                        PlotPoint::new(label_x, label_y),
                        egui::RichText::new(turn_num).color(label_color)
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
