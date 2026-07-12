use crate::data::ibt_parser::IbtSession;
use polars::prelude::*;

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

// Highly-optimized O(N) sliding window causal moving average filter 
pub fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![0.0; n];
    if n == 0 { return out; }
    let w = window.clamp(1, n);
    let mut running_sum = 0.0;
    
    // Warm up phase
    for i in 0..w {
        running_sum += data[i];
        out[i] = running_sum / (i + 1) as f64;
    }
    
    // Standard phase
    for i in w..n {
        running_sum = running_sum - data[i - w] + data[i];
        out[i] = running_sum / w as f64;
    }
    out
}

// Determine which lap the cursor is currently inside (Standalone static helper!)
pub fn get_active_lap(lap_ranges: &[(i32, f64, f64)], cursor_x: Option<f64>, selected_lap: Option<i32>) -> i32 {
    let cx = cursor_x.unwrap_or(0.0);
    for &(lap_num, start_t, end_t) in lap_ranges {
        if cx >= start_t && cx <= end_t {
            return lap_num;
        }
    }
    selected_lap.unwrap_or(1)
}

// Calculates the fastest lap by ignoring the first 3 laps if there are more than 3 laps in the session.
pub fn get_fastest_lap(lap_times: &[(i32, f64)]) -> i32 {
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
pub fn get_lap_time_at_distance(lap_dist: &[f64], lap_time: &[f64], target_dist: f64) -> f64 {
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
pub fn get_lap_coord_at_distance(lap: &LapData, target_dist: f64) -> (f64, f64) {
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
pub fn get_lap_coord_at_time(lap: &LapData, target_time: f64) -> (f64, f64) {
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
// Cleaned of egui_plot dependencies: returns pure coordinate points!
pub fn get_lap_segments(lap: &LapData) -> Vec<Vec<[f64; 2]>> {
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
                segments.push(current_segment);
                current_segment = Vec::new();
            }
        }
        current_segment.push([x1, y1]);
    }
    
    if !current_segment.is_empty() {
        segments.push(current_segment);
    }
    segments
}

// Automatically detects track corners and straights based on the fastest lap's lateral G-force and steering angle.
pub fn detect_track_sectors(session: &IbtSession, merge_threshold: f64) -> Vec<TrackSector> {
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
    let smoothed_lat = moving_average(&lap_lat_accel, w_size);
    let smoothed_steer = moving_average(&lap_steer, w_size);
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
            if gap_dist < merge_threshold {
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

// Highly precise binary search bisector to locate closest index in < 1 microsecond!
pub fn get_closest_index(distance: &[f64], target_x: f64) -> usize {
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
pub fn get_lap_points_slice<'a>(lap_ranges: &[(i32, f64, f64)], cache: &'a [[f64; 2]], lap_num: i32) -> &'a [[f64; 2]] {
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

// Formats seconds into MM:SS.SSS
pub fn format_lap_time(sec: f64) -> String {
    let minutes = (sec / 60.0).floor() as i32;
    let seconds = (sec % 60.0).floor() as i32;
    let ms = ((sec % 1.0) * 1000.0).round() as i32;
    format!("{:02}:{:02}.{:03}", minutes, seconds, ms)
}

// Formats sector split duration in seconds into SS.SSS or MM:SS.SSS
pub fn format_sector_time(sec: f64) -> String {
    if sec >= 60.0 {
        format_lap_time(sec)
    } else {
        let seconds = sec.floor() as i32;
        let ms = ((sec % 1.0) * 1000.0).round() as i32;
        format!("{:02}.{:03}", seconds, ms)
    }
}

// Spawns a background thread to download the track map SVG from the public iRacing static assets CDN
pub fn trigger_track_map_download(track_id: i32) {
    let download_enabled = false; // Set to true to re-enable CDN downloading
    if !download_enabled { return; }
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

pub fn get_sector_segments(lap: &LapData, start_dist: f64, end_dist: f64) -> Vec<Vec<[f64; 2]>> {
    let mut segments = Vec::new();
    if lap.dist.is_empty() {
        return segments;
    }
    let start_idx = get_closest_index(&lap.dist, start_dist);
    let end_idx = get_closest_index(&lap.dist, end_dist);
    if start_idx >= end_idx {
        return segments;
    }

    let max_jump = 50.0; // 50 meters jump threshold
    let mut current_segment = Vec::new();
    
    current_segment.push([lap.x[start_idx], lap.y[start_idx]]);
    
    for i in (start_idx + 1)..=end_idx {
        if i >= lap.x.len() { break; }
        let x0 = lap.x[i - 1];
        let y0 = lap.y[i - 1];
        let x1 = lap.x[i];
        let y1 = lap.y[i];
        
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        
        if dist > max_jump {
            if !current_segment.is_empty() {
                segments.push(current_segment);
                current_segment = Vec::new();
            }
        }
        current_segment.push([x1, y1]);
    }
    
    if !current_segment.is_empty() {
        segments.push(current_segment);
    }
    segments
}

pub fn recalculate_sector_deltas(
    lap_data_cache: &[LapData],
    sectors: &[TrackSector],
    active_lap_num: Option<i32>,
    ref_lap_num: Option<i32>,
) -> Vec<Option<f64>> {
    let mut deltas = vec![None; sectors.len()];
    if let (Some(act_num), Some(ref_num)) = (active_lap_num, ref_lap_num) {
        if let (Some(act), Some(re)) = (
            lap_data_cache.iter().find(|l| l.lap_num == act_num),
            lap_data_cache.iter().find(|l| l.lap_num == ref_num),
        ) {
            for (s_idx, sector) in sectors.iter().enumerate() {
                let act_start = get_lap_time_at_distance(&act.dist, &act.time, sector.start_dist);
                let act_end = get_lap_time_at_distance(&act.dist, &act.time, sector.end_dist);
                let act_time = act_end - act_start;

                let ref_start = get_lap_time_at_distance(&re.dist, &re.time, sector.start_dist);
                let ref_end = get_lap_time_at_distance(&re.dist, &re.time, sector.end_dist);
                let ref_time = ref_end - ref_start;

                if act_time > 0.0 && ref_time > 0.0 {
                    deltas[s_idx] = Some(act_time - ref_time);
                }
            }
        }
    }
    deltas
}
