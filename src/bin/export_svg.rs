use std::fs::File;
use std::io::Write;
use std::path::Path;

#[path = "../data/ibt_parser.rs"]
mod ibt_parser;

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

fn detect_track_sectors(session: &ibt_parser::IbtSession, fastest_lap: i32) -> Vec<TrackSector> {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let ibt_path_str = if args.len() > 1 {
        &args[1]
    } else {
        r"D:\Oldtelems\25s4Sebring\porsche992cup_sebring international 2025-11-28 11-57-29.ibt"
    };
    let ibt_path = Path::new(ibt_path_str);
    if !ibt_path.exists() {
        return Err(format!("Telemetry file not found: {}", ibt_path.display()).into());
    }


    println!("Parsing telemetry file: {} ...", ibt_path.display());
    let session = ibt_parser::parse_ibt_file(ibt_path)?;
    let fastest_lap = get_fastest_lap(&session.lap_times);
    println!("Fastest Lap detected: Lap {}", fastest_lap);

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

    // Determine bounding box
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for seg in &segments {
        for &(x, y) in seg {
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
        }
    }

    let padding = 100.0;
    min_x -= padding;
    max_x += padding;
    min_y -= padding;
    max_y += padding;

    let width = max_x - min_x;
    let height = max_y - min_y;

    let aspect_ratio = height / width;
    let new_width = 1000.0;
    let new_height = 1000.0 * aspect_ratio;

    // We flip Y coordinates when generating SVG so that up on the track map remains up in the layout!
    let mut svg_content = String::new();
    svg_content.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\" style=\"background-color: #0A0A0A;\">\n",
        min_x, -max_y, width, height, new_width, new_height
    ));

    // Draw active driven track path segments
    for seg in &segments {
        if seg.len() < 2 { continue; }
        svg_content.push_str("  <path d=\"");
        let start_y = -seg[0].1;
        svg_content.push_str(&format!("M {} {}", seg[0].0, start_y));
        for i in 1..seg.len() {
            let py = -seg[i].1;
            svg_content.push_str(&format!(" L {} {}", seg[i].0, py));
        }
        // Glowing brand electric orange path
        svg_content.push_str("\" fill=\"none\" stroke=\"#F25225\" stroke-width=\"16\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />\n");
    }

    // Draw Start/Finish Line
    if x_coords.len() > 1 {
        let x0 = x_coords[0];
        let y0 = y_coords[0];
        let x1 = x_coords[1];
        let y1 = y_coords[1];
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx*dx + dy*dy).sqrt();
        if len > 0.0 {
            let nx = -dy / len;
            let ny = dx / len;
            let sf_width = 25.0;
            let sf_x0 = x0 - nx * sf_width;
            let sf_y0 = -y0 - ny * sf_width;
            let sf_x1 = x0 + nx * sf_width;
            let sf_y1 = -y0 + ny * sf_width;
            svg_content.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#E3E2E1\" stroke-width=\"12\" stroke-linecap=\"round\" />\n",
                sf_x0, sf_y0, sf_x1, sf_y1
            ));
        }
    }

    // Detect track corners to place Turn Label text elements
    let sectors = detect_track_sectors(&session, fastest_lap);
    for sector in &sectors {
        if sector.name.starts_with("Turn") {
            let mid_dist = (sector.start_dist + sector.end_dist) / 2.0;
            let (tx, ty) = get_lap_coord_at_distance(&dists, &x_coords, &y_coords, mid_dist);

            // Find normal vector at this midpoint to offset the label outwards
            let mid_idx = match dists.binary_search_by(|val| val.partial_cmp(&mid_dist).unwrap_or(std::cmp::Ordering::Equal)) {
                Ok(i) => i,
                Err(i) => i.clamp(0, dists.len() - 1),
            };

            let mut nx = 0.0;
            let mut ny = 0.0;
            if mid_idx > 0 && mid_idx < x_coords.len() - 1 {
                let dx = x_coords[mid_idx + 1] - x_coords[mid_idx - 1];
                let dy = y_coords[mid_idx + 1] - y_coords[mid_idx - 1];
                let len = (dx*dx + dy*dy).sqrt();
                if len > 0.0 {
                    nx = -dy / len;
                    ny = dx / len;
                }
            }

            let turn_num = sector.name.replace("Turn ", "");
            let offset_dist = 45.0;
            let label_x = tx + nx * offset_dist;
            let label_y = -ty - ny * offset_dist; // Flip Y coordinates to match SVG space

            svg_content.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"#E3E2E1\" font-family=\"Arial, sans-serif\" font-size=\"42\" font-weight=\"bold\" text-anchor=\"middle\" alignment-baseline=\"middle\">{}</text>\n",
                label_x, label_y, turn_num
            ));
        }
    }

    svg_content.push_str("</svg>\n");

    let dest_path_str = if args.len() > 2 {
        &args[2]
    } else {
        "sebring_vector_branding.svg"
    };
    let dest_path = Path::new(dest_path_str);
    let mut file = File::create(&dest_path)?;
    file.write_all(svg_content.as_bytes())?;
    println!("Successfully exported branding vector SVG to: {}", dest_path.display());

    Ok(())
}
