use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::data::ibt_parser;

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

// Finds the coordinate (x, y) at a target distance on a lap by linear interpolation.
fn get_lap_coord_at_distance(dist: &[f64], x_coords: &[f64], y_coords: &[f64], target_dist: f64) -> (f64, f64) {
    if dist.is_empty() {
        return (0.0, 0.0);
    }
    if target_dist <= dist[0] {
        return (x_coords[0], y_coords[0]);
    }
    if target_dist >= dist[dist.len() - 1] {
        return (x_coords[x_coords.len() - 1], y_coords[y_coords.len() - 1]);
    }
    match dist.binary_search_by(|val| val.partial_cmp(&target_dist).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => (x_coords[idx], y_coords[idx]),
        Err(idx) => {
            if idx == 0 {
                (x_coords[0], y_coords[0])
            } else if idx >= dist.len() {
                (x_coords[x_coords.len() - 1], y_coords[y_coords.len() - 1])
            } else {
                let d0 = dist[idx - 1];
                let d1 = dist[idx];
                let x0 = x_coords[idx - 1];
                let x1 = x_coords[idx];
                let y0 = y_coords[idx - 1];
                let y1 = y_coords[idx];
                let pct = (target_dist - d0) / (d1 - d0);
                (x0 + (x1 - x0) * pct, y0 + (y1 - y0) * pct)
            }
        }
    }
}

pub struct TrackSector {
    pub name: String,
    pub start_dist: f64,
    pub end_dist: f64,
}

fn detect_track_sectors(session: &ibt_parser::IbtSession, fastest_lap: i32, merge_threshold: f64) -> Vec<TrackSector> {
    let mut sectors = Vec::new();
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

pub fn generate_track_map_json(ibt_path: &Path, dest_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !ibt_path.exists() {
        return Err(format!("Telemetry file not found: {}", ibt_path.display()).into());
    }

    let session = ibt_parser::parse_ibt_file(ibt_path)?;
    let fastest_lap = get_fastest_lap(&session.lap_times);

    let df = &session.dataframe;
    let n = session.distance.len();

    let lap_col = df.column("Lap")?.f64()?;
    let dist_col = df.column("Distance_Derived")?.f64()?;
    let lat_col = df.column("Lat")?.f64()?;
    let lon_col = df.column("Lon")?.f64()?;

    let mut lat0 = 0.0;
    let mut lon0 = 0.0;
    if lat_col.len() > 0 {
        lat0 = lat_col.get(0).unwrap_or(0.0);
        lon0 = lon_col.get(0).unwrap_or(0.0);
    }

    let r_earth = 6378137.0;
    let lat0_rad = lat0 * std::f64::consts::PI / 180.0;
    let lon0_rad = lon0 * std::f64::consts::PI / 180.0;

    let mut dists = Vec::new();
    let mut x_coords = Vec::new();
    let mut y_coords = Vec::new();

    for i in 0..n {
        if lap_col.get(i).unwrap_or(0.0) as i32 == fastest_lap {
            dists.push(dist_col.get(i).unwrap_or(0.0));
            let lat = lat_col.get(i).unwrap_or(0.0);
            let lon = lon_col.get(i).unwrap_or(0.0);
            let lat_rad = lat * std::f64::consts::PI / 180.0;
            let lon_rad = lon * std::f64::consts::PI / 180.0;
            let x = r_earth * (lon_rad - lon0_rad) * lat0_rad.cos();
            let y = r_earth * (lat_rad - lat0_rad);
            x_coords.push(x);
            y_coords.push(y);
        }
    }

    if dists.is_empty() {
        return Err("No coordinate data found for fastest lap.".into());
    }

    let base_dist = dists[0];
    for d in &mut dists {
        *d -= base_dist;
    }

    // Split into segments to avoid jumps
    let max_jump = 50.0;
    let mut segments = Vec::new();
    let mut curr_seg = Vec::new();
    curr_seg.push((x_coords[0], y_coords[0]));

    for i in 1..x_coords.len() {
        let dx = x_coords[i] - x_coords[i - 1];
        let dy = y_coords[i] - y_coords[i - 1];
        let step_dist = (dx*dx + dy*dy).sqrt();
        if step_dist > max_jump {
            if !curr_seg.is_empty() {
                segments.push(curr_seg);
                curr_seg = Vec::new();
            }
        }
        curr_seg.push((x_coords[i], y_coords[i]));
    }
    if !curr_seg.is_empty() {
        segments.push(curr_seg);
    }

    let json_data = serde_json::to_string(&segments)?;
    let mut file = File::create(dest_path)?;
    file.write_all(json_data.as_bytes())?;
    
    Ok(())
}
