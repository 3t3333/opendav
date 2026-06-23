use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use polars::prelude::*;
use std::path::Path;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VarHeader {
    pub var_type: i32,
    pub offset: i32,
    pub name: String,
    pub unit: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IbtSession {
    pub source_file: String,
    pub car: String,
    pub venue: String,
    pub air_temp: String,
    pub surface_temp: String,
    pub track_id: i32,
    pub timestamp: String,
    pub dataframe: DataFrame,
    pub lap_times: Vec<(i32, f64)>, // (Lap Number, Duration in Seconds)
    pub total_session_time: f64,
    
    // Highly optimized raw caching vectors for real-time 300FPS GUI lookups
    pub distance: Vec<f64>,
    pub front_smooth: Vec<f64>,
    pub rear_smooth: Vec<f64>,
    pub rake: Vec<f64>,
    pub laps: Vec<i32>,
}

// Highly-optimized O(N) sliding window causal moving average filter 
pub fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![0.0; n];
    if n == 0 { return out; }
    let w = window.clamp(1, n);
    let mut running_sum = 0.0;
    for i in 0..n {
        running_sum += data[i];
        if i >= w {
            running_sum -= data[i - w];
            out[i] = running_sum / w as f64;
        } else {
            out[i] = running_sum / (i + 1) as f64;
        }
    }
    out
}

pub fn parse_ibt_file<P: AsRef<Path>>(file_path: P) -> Result<IbtSession, Box<dyn std::error::Error>> {
    let mut f = File::open(&file_path)?;
    let file_len = f.metadata()?.len();

    // 1. Read Header (64 bytes)
    let _session_num = f.read_i32::<LittleEndian>()?;
    let _session_tick = f.read_i32::<LittleEndian>()?;
    let _session_rate = f.read_f64::<LittleEndian>()?;
    
    // Header offsets
    let session_info_len = f.read_i32::<LittleEndian>()? as usize;
    let session_info_offset = f.read_i32::<LittleEndian>()? as u64;
    let num_vars = f.read_i32::<LittleEndian>()? as usize;
    let var_header_offset = f.read_i32::<LittleEndian>()? as u64;
    
    let _num_buf = f.read_i32::<LittleEndian>()?;
    let buf_len = f.read_i32::<LittleEndian>()? as usize;
    
    // Seek to buf offset in header (offset 52)
    f.seek(SeekFrom::Start(52))?;
    let buf_offset = f.read_i32::<LittleEndian>()? as u64;

    // 2. Parse YAML Session Info (Venue & Car details)
    f.seek(SeekFrom::Start(session_info_offset))?;
    let mut yaml_buf = vec![0u8; session_info_len];
    f.read_exact(&mut yaml_buf)?;
    let yaml_str = String::from_utf8_lossy(&yaml_buf);

    let mut venue = "Unknown Venue".to_string();
    let mut car = "Unknown Vehicle".to_string();
    let mut air_temp = "Unknown".to_string();
    let mut surface_temp = "Unknown".to_string();
    let mut track_id = 0;

    for line in yaml_str.lines() {
        if line.contains("TrackDisplayName:") {
            if let Some(val) = line.split(':').nth(1) {
                venue = val.trim().trim_matches('"').to_string();
            }
        }
        if line.contains("TrackID:") {
            if let Some(val) = line.split(':').nth(1) {
                if let Ok(id) = val.trim().trim_matches('"').parse::<i32>() {
                    track_id = id;
                }
            }
        }
        if line.contains("DriverCarName:") || line.contains("CarPath:") {
            if let Some(val) = line.split(':').nth(1) {
                car = val.trim().trim_matches('"').to_string();
            }
        }
        if line.contains("TrackAirTemp:") {
            if let Some(val) = line.split(':').nth(1) {
                air_temp = val.trim().trim_matches('"').to_string();
            }
        }
        if line.contains("TrackSurfaceTemp:") {
            if let Some(val) = line.split(':').nth(1) {
                surface_temp = val.trim().trim_matches('"').to_string();
            }
        }
    }

    // 3. Parse Variable Headers (144 bytes each)
    f.seek(SeekFrom::Start(var_header_offset))?;
    let mut var_headers = Vec::with_capacity(num_vars);
    
    for _ in 0..num_vars {
        let mut var_buf = vec![0u8; 144];
        f.read_exact(&mut var_buf)?;

        let v_type = i32::from_le_bytes([var_buf[0], var_buf[1], var_buf[2], var_buf[3]]);
        let offset = i32::from_le_bytes([var_buf[4], var_buf[5], var_buf[6], var_buf[7]]);
        
        // Extract null-terminated strings for name & unit
        let name_bytes = &var_buf[16..48];
        let name = String::from_utf8_lossy(name_bytes)
            .trim_end_matches('\0')
            .to_string();
            
        let unit_bytes = &var_buf[112..144];
        let unit = String::from_utf8_lossy(unit_bytes)
            .trim_end_matches('\0')
            .to_string();

        var_headers.push(VarHeader {
            var_type: v_type,
            offset,
            name,
            unit,
        });
    }

    // 4. Load Data Samples into memory
    let num_samples = ((file_len - buf_offset) / buf_len as u64) as usize;
    f.seek(SeekFrom::Start(buf_offset))?;
    let mut raw_bytes = vec![0u8; num_samples * buf_len];
    f.read_exact(&mut raw_bytes)?;

    // Prepare vector arrays for each series
    let mut channel_vectors: Vec<Vec<f64>> = vec![Vec::with_capacity(num_samples); num_vars];

    // Read values using safe pointer/slice casting
    for s in 0..num_samples {
        let row_offset = s * buf_len;
        for (v_idx, var) in var_headers.iter().enumerate() {
            let offset = row_offset + var.offset as usize;
            let val = match var.var_type {
                0 => raw_bytes[offset] as i8 as f64, // Char
                1 => (raw_bytes[offset] > 0) as i32 as f64, // Bool
                2 => { // Int32
                    let b = [raw_bytes[offset], raw_bytes[offset+1], raw_bytes[offset+2], raw_bytes[offset+3]];
                    i32::from_le_bytes(b) as f64
                }
                3 => { // Uint32
                    let b = [raw_bytes[offset], raw_bytes[offset+1], raw_bytes[offset+2], raw_bytes[offset+3]];
                    u32::from_le_bytes(b) as f64
                }
                4 => { // Float32
                    let b = [raw_bytes[offset], raw_bytes[offset+1], raw_bytes[offset+2], raw_bytes[offset+3]];
                    f32::from_le_bytes(b) as f64
                }
                5 => { // Float64
                    let b = [
                        raw_bytes[offset], raw_bytes[offset+1], raw_bytes[offset+2], raw_bytes[offset+3],
                        raw_bytes[offset+4], raw_bytes[offset+5], raw_bytes[offset+6], raw_bytes[offset+7]
                    ];
                    f64::from_le_bytes(b)
                }
                _ => 0.0,
            };
            channel_vectors[v_idx].push(val);
        }
    }

    // Identify Lap and SessionTime vector indices for quick math
    let lap_idx = var_headers.iter().position(|v| v.name == "Lap");
    let time_idx = var_headers.iter().position(|v| v.name == "SessionTime");

    let mut lap_ranges: HashMap<i32, (f64, f64)> = HashMap::new();
    let mut total_session_time = 0.0;
    let mut laps_raw = vec![0; num_samples];

    if let (Some(l_idx), Some(t_idx)) = (lap_idx, time_idx) {
        let lap_vec = &channel_vectors[l_idx];
        let time_vec = &channel_vectors[t_idx];
        
        if !time_vec.is_empty() {
            total_session_time = time_vec[time_vec.len() - 1] - time_vec[0];
        }

        for s in 0..num_samples {
            let lap_val = lap_vec[s] as i32;
            laps_raw[s] = lap_val;
            let time_val = time_vec[s];
            if lap_val <= 0 {
                continue; // Skip out-laps / pit entry warmups
            }
            lap_ranges.entry(lap_val)
                .and_modify(|range| {
                    if time_val < range.0 { range.0 = time_val; }
                    if time_val > range.1 { range.1 = time_val; }
                })
                .or_insert((time_val, time_val));
        }
    }

    // Compile lap times
    let mut raw_lap_times: Vec<(i32, f64)> = lap_ranges.into_iter()
        .map(|(lap_num, (start, end))| (lap_num, end - start))
        .filter(|(_, duration)| *duration > 1.0) // Filter out 0-second artifacts
        .collect();

    raw_lap_times.sort_by_key(|(lap_num, _)| *lap_num);

    // Filter out short glitch laps (e.g. less than 10s) and keep valid laps.
    // A lap is discarded as a glitch/reset if it is less than 75% of the median lap time.
    let mut filtered_laps: Vec<(i32, f64)> = raw_lap_times.into_iter()
        .filter(|(_, t)| *t >= 10.0)
        .collect();

    if !filtered_laps.is_empty() {
        let mut durations: Vec<f64> = filtered_laps.iter().map(|(_, t)| *t).collect();
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_lap_time = durations[durations.len() / 2];
        let limit_time = median_lap_time * 0.75;
        filtered_laps.retain(|(_, t)| *t >= limit_time);
    }

    let lap_times = filtered_laps;

    // -------------------------------------------------------------------------
    // PRECOMPUTED DERIVED MATHEMATICS & SIGNAL FILTERS (Dynamic Rake, etc.)
    // -------------------------------------------------------------------------
    
    // Quick closure helper to retrieve channel array with fallbacks
    let get_channel_any = |names: &[&str], var_headers: &[VarHeader], channel_vectors: &Vec<Vec<f64>>| -> Vec<f64> {
        for name in names {
            if let Some(idx) = var_headers.iter().position(|v| v.name == *name) {
                return channel_vectors[idx].clone();
            }
        }
        vec![0.0; num_samples]
    };

    // Retrieve Front & Rear Ride Heights
    let fl_rh = get_channel_any(&["Ride Height FL", "LFrideHeight", "LFshockDefl"], &var_headers, &channel_vectors);
    let fr_rh = get_channel_any(&["Ride Height FR", "RFrideHeight", "RFshockDefl"], &var_headers, &channel_vectors);
    let rl_rh = get_channel_any(&["Ride Height RL", "LRrideHeight", "LRshockDefl"], &var_headers, &channel_vectors);
    let rr_rh = get_channel_any(&["Ride Height RR", "RRrideHeight", "RRshockDefl"], &var_headers, &channel_vectors);

    // Calculate Averaged Front/Rear Axle Ride Heights (multiply deflection by 1000 to convert to mm)
    let mut front_avg = vec![0.0; num_samples];
    let mut rear_avg = vec![0.0; num_samples];
    for s in 0..num_samples {
        let scale = if fl_rh[s] < 0.5 { 1000.0 } else { 1.0 }; // Auto-detect scales (meters vs mm)
        front_avg[s] = ((fl_rh[s] + fr_rh[s]) / 2.0) * scale;
        rear_avg[s] = ((rl_rh[s] + rr_rh[s]) / 2.0) * scale;
    }

    // Determine sample rate and configure 4.5-second smoothing window
    let time_vec = if let Some(t_idx) = time_idx {
        channel_vectors[t_idx].clone()
    } else {
        (0..num_samples).map(|i| i as f64 * 0.016).collect()
    };

    let dt = if num_samples > 1 {
        (time_vec[num_samples - 1] - time_vec[0]) / num_samples as f64
    } else {
        0.016
    };
    let w_size = (4.5 / dt).round() as usize;

    // Apply moving average filter
    let front_smooth = moving_average(&front_avg, w_size);
    let rear_smooth = moving_average(&rear_avg, w_size);

    // Calculate Dynamic Rake Attitude (Rear - Front in mm)
    let mut rake = vec![0.0; num_samples];
    for s in 0..num_samples {
        rake[s] = rear_smooth[s] - front_smooth[s];
    }

    // Derive dynamic Distance vector if none exists
    let dist_vec = if let Some(d_idx) = var_headers.iter().position(|v| v.name == "Distance" || v.name == "LapDist") {
        channel_vectors[d_idx].clone()
    } else {
        let mut d = vec![0.0; num_samples];
        let mut curr_dist = 0.0;
        let speed_vec = get_channel_any(&["Speed"], &var_headers, &channel_vectors);
        for s in 0..num_samples {
            curr_dist += speed_vec[s] * dt;
            d[s] = curr_dist;
        }
        d
    };

    // 5. Build Polars DataFrame from compiled channel Vectors using Polars 0.41.3 API
    let mut series_list = Vec::with_capacity(num_vars + 5);
    for (v_idx, var) in var_headers.iter().enumerate() {
        let vec = std::mem::take(&mut channel_vectors[v_idx]);
        let s = Series::new(&var.name, vec);
        series_list.push(s);
    }

    // Inject our precomputed derived arrays into the master DataFrame
    series_list.push(Series::new("Distance_Derived", dist_vec.clone()));
    series_list.push(Series::new("FrontRideHeightSmooth", front_smooth.clone()));
    series_list.push(Series::new("RearRideHeightSmooth", rear_smooth.clone()));
    series_list.push(Series::new("DynamicRake", rake.clone()));

    let dataframe = DataFrame::new(series_list)?;

    // Extract File Timestamp
    let file_name = file_path.as_ref()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    Ok(IbtSession {
        source_file: file_name,
        car,
        venue,
        air_temp,
        surface_temp,
        track_id,
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        dataframe,
        lap_times,
        total_session_time,
        distance: dist_vec,
        front_smooth,
        rear_smooth,
        rake,
        laps: laps_raw,
    })
}
