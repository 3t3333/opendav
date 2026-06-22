use std::fs::File;
use std::io::{Write, Result};
use byteorder::{LittleEndian, WriteBytesExt};

fn main() -> Result<()> {
    let file_path = "mock_telemetry.ibt";
    let mut f = File::create(file_path)?;

    // 1. Setup metadata
    let session_info = "TrackDisplayName: Sebring International Raceway\nTrackID: 123\nCarPath: porsche992cup\nTrackAirTemp: 22.5 C\nTrackSurfaceTemp: 31.2 C\n";
    let session_info_bytes = session_info.as_bytes();
    let session_info_len = session_info_bytes.len();
    
    // Variables list
    let var_names = vec![
        "SessionTime", "Distance", "Lap", "Speed", "Throttle", "Brake",
        "SteeringWheelAngle", "RPM", "Gear", "IsOnTrack", "PlayerCarInPitStall",
        "Lat", "Lon", "LatAccel",
        "Ride Height FL", "Ride Height FR", "Ride Height RL", "Ride Height RR"
    ];
    let num_vars = var_names.len();
    let buf_len = num_vars * 8; // 18 variables * 8 bytes (all float64) = 144 bytes per record
    
    // Offsets in file
    let header_size = 64;
    let session_info_offset = header_size;
    let var_header_offset = session_info_offset + session_info_len;
    let buf_offset = var_header_offset + (num_vars * 144);

    // 2. Write Header (64 bytes)
    f.write_i32::<LittleEndian>(1)?; // session_num
    f.write_i32::<LittleEndian>(100)?; // session_tick
    f.write_f64::<LittleEndian>(60.0)?; // session_rate (60 Hz)
    f.write_i32::<LittleEndian>(session_info_len as i32)?;
    f.write_i32::<LittleEndian>(session_info_offset as i32)?;
    f.write_i32::<LittleEndian>(num_vars as i32)?;
    f.write_i32::<LittleEndian>(var_header_offset as i32)?;
    f.write_i32::<LittleEndian>(1)?; // num_buf
    f.write_i32::<LittleEndian>(buf_len as i32)?; // buf_len
    
    // Seek to 52 for buf_offset
    let padding_before_52 = 52 - 40;
    f.write_all(&vec![0u8; padding_before_52])?;
    f.write_i32::<LittleEndian>(buf_offset as i32)?; // buf_offset
    
    // Remainder of header (to 64 bytes)
    let padding_after_52 = 64 - 56;
    f.write_all(&vec![0u8; padding_after_52])?;

    // 3. Write Session Info YAML
    f.write_all(session_info_bytes)?;

    // 4. Write Variable Headers (144 bytes each)
    for (idx, name) in var_names.iter().enumerate() {
        let mut var_buf = vec![0u8; 144];
        
        // var_type: 5 (float64)
        let v_type_bytes = 5i32.to_le_bytes();
        var_buf[0..4].copy_from_slice(&v_type_bytes);
        
        // offset: idx * 8
        let offset_bytes = ((idx * 8) as i32).to_le_bytes();
        var_buf[4..8].copy_from_slice(&offset_bytes);
        
        // name: bytes 16..48
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len().min(32);
        var_buf[16..(16 + name_len)].copy_from_slice(&name_bytes[..name_len]);
        
        // unit: bytes 112..144
        let unit = match *name {
            "SessionTime" => "s",
            "Distance" => "m",
            "Speed" => "m/s",
            "Throttle" => "%",
            "Brake" => "%",
            "SteeringWheelAngle" => "rad",
            "RPM" => "rev/min",
            "Gear" => "",
            "Lat" | "Lon" => "rad",
            "LatAccel" => "m/s^2",
            "Ride Height FL" | "Ride Height FR" | "Ride Height RL" | "Ride Height RR" => "m",
            _ => "",
        };
        let unit_bytes = unit.as_bytes();
        let unit_len = unit_bytes.len().min(32);
        var_buf[112..(112 + unit_len)].copy_from_slice(&unit_bytes[..unit_len]);

        f.write_all(&var_buf)?;
    }

    // 5. Write Data Samples (60 Hz, 182.5 seconds -> 10950 samples)
    let hz = 60.0;
    let dt = 1.0 / hz;
    let lap1_len = 61.0;
    let lap2_len = 59.5;
    let lap3_len = 62.0;
    let total_seconds = lap1_len + lap2_len + lap3_len;
    let num_samples = (total_seconds * hz) as usize;
    let r_earth = 6378137.0;
    let lat0 = 27.456;
    let lon0 = -81.348;
    let lat0_rad = lat0 * std::f64::consts::PI / 180.0;
    let lon0_rad = lon0 * std::f64::consts::PI / 180.0;

    let mut distance_cum = 0.0;

    for i in 0..num_samples {
        let t = i as f64 * dt;
        
        // Lap and relative time calculation
        let (lap, t_rel, lap_len) = if t < lap1_len {
            (1.0, t, lap1_len)
        } else if t < lap1_len + lap2_len {
            (2.0, t - lap1_len, lap2_len)
        } else {
            (3.0, t - lap1_len - lap2_len, lap3_len)
        };

        // Track path (loop over active lap duration)
        let theta = (2.0 * std::f64::consts::PI * t_rel) / lap_len;
        let x = 600.0 * theta.cos() + 150.0 * (2.0 * theta).sin();
        let y = 300.0 * theta.sin() + 100.0 * (3.0 * theta).cos();

        // Calculate speed (m/s)
        let dtheta = (2.0 * std::f64::consts::PI) / lap_len;
        let dx_dt = -600.0 * theta.sin() * dtheta + 300.0 * (2.0 * theta).cos() * dtheta;
        let dy_dt = 300.0 * theta.cos() * dtheta - 300.0 * (3.0 * theta).sin() * dtheta;
        let mut speed = (dx_dt * dx_dt + dy_dt * dy_dt).sqrt();
        // Scale speed to Porsche GT3 Cup ranges (30 m/s to 70 m/s)
        speed = 30.0 + speed * 0.06;

        distance_cum += speed * dt;

        // Curvature and Steering angle
        let d2theta = dtheta * dtheta;
        let d2x_dt2 = -600.0 * theta.cos() * d2theta - 600.0 * (2.0 * theta).sin() * d2theta;
        let d2y_dt2 = -300.0 * theta.sin() * d2theta - 900.0 * (3.0 * theta).cos() * d2theta;
        
        let numerator = dx_dt * d2y_dt2 - dy_dt * d2x_dt2;
        let denominator = (dx_dt * dx_dt + dy_dt * dy_dt).powf(1.5);
        let curvature = if denominator > 0.0 { numerator / denominator } else { 0.0 };
        let steering = (curvature * 45.0).clamp(-2.0, 2.0); // Rad steering angle

        // LatAccel (m/s^2)
        let lat_accel = speed * speed * curvature * 0.05;

        // Throttle and Brake lookahead
        let t_lookahead_rel = (t_rel + 0.5) % lap_len;
        let theta_la = (2.0 * std::f64::consts::PI * t_lookahead_rel) / lap_len;
        let dx_dt_la = -600.0 * theta_la.sin() * dtheta + 300.0 * (2.0 * theta_la).cos() * dtheta;
        let dy_dt_la = 300.0 * theta_la.cos() * dtheta - 300.0 * (3.0 * theta_la).sin() * dtheta;
        let speed_la = 30.0 + (dx_dt_la * dx_dt_la + dy_dt_la * dy_dt_la).sqrt() * 0.06;

        let delta_speed = speed_la - speed;
        let mut throttle = 0.0;
        let mut brake = 0.0;

        if delta_speed < -0.8 {
            brake = (-delta_speed * 0.25).clamp(0.0, 1.0);
            throttle = 0.0;
        } else if delta_speed > 0.1 {
            throttle = (delta_speed * 1.5).clamp(0.0, 1.0);
            brake = 0.0;
        } else {
            throttle = 0.35; // Cruising corner-exit control
            brake = 0.0;
        }

        // Gear selection based on speed
        let gear = if speed < 22.0 {
            1.0
        } else if speed < 32.0 {
            2.0
        } else if speed < 43.0 {
            3.0
        } else if speed < 54.0 {
            4.0
        } else if speed < 65.0 {
            5.0
        } else {
            6.0
        };

        // RPM calculation
        let rpm = 3500.0 + (speed / gear) * 350.0 + (throttle * 1200.0);
        let rpm = rpm.clamp(3200.0, 8400.0);

        // Lat/Lon conversion
        let lat = lat0 + (y / r_earth) * (180.0 / std::f64::consts::PI);
        let lon = lon0 + (x / (r_earth * lat0_rad.cos())) * (180.0 / std::f64::consts::PI);

        let lat_rad = lat * std::f64::consts::PI / 180.0;
        let lon_rad = lon * std::f64::consts::PI / 180.0;

        // Suspension Ride Heights FL, FR, RL, RR
        let aero_downforce = 0.018 * (speed / 70.0).powi(2);
        let pitch_effect = (brake * 0.012) - (throttle * 0.006);
        let roll_effect = steering * 0.009;
        let bumps = 0.002 * (25.0 * t).sin() + 0.001 * (65.0 * t).cos();

        let rh_fl = 0.065 - aero_downforce - pitch_effect + roll_effect + bumps;
        let rh_fr = 0.065 - aero_downforce - pitch_effect - roll_effect + bumps;
        let rh_rl = 0.075 - aero_downforce + pitch_effect + roll_effect + bumps;
        let rh_rr = 0.075 - aero_downforce + pitch_effect - roll_effect + bumps;

        // Write to record
        let record = vec![
            t,                // SessionTime
            distance_cum,     // Distance
            lap,              // Lap
            speed,            // Speed
            throttle,         // Throttle
            brake,            // Brake
            steering,         // SteeringWheelAngle
            rpm,              // RPM
            gear,             // Gear
            1.0,              // IsOnTrack
            0.0,              // PlayerCarInPitStall
            lat_rad,          // Lat (radians)
            lon_rad,          // Lon (radians)
            lat_accel,        // LatAccel
            rh_fl,            // Ride Height FL
            rh_fr,            // Ride Height FR
            rh_rl,            // Ride Height RL
            rh_rr             // Ride Height RR
        ];

        for val in record {
            f.write_f64::<LittleEndian>(val)?;
        }
    }

    println!("Successfully generated mock telemetry file '{}'!", file_path);
    println!("Total samples: {}, size: {} bytes", num_samples, buf_offset + (num_samples * buf_len));

    Ok(())
}
