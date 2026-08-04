use egui::Color32;

/// Evaluates section time delta from a time_delta cache (`[session_time, cumulative_delta]`).
/// Section delta is `delta(end_t) - delta(start_t)`.
pub fn calculate_section_delta_from_cache(
    time_delta_pts: &[[f64; 2]],
    start_t: f64,
    end_t: f64,
) -> Option<f64> {
    if time_delta_pts.is_empty() || end_t <= start_t {
        return None;
    }
    let delta_start = sample_channel_at_time(time_delta_pts, start_t);
    let delta_end = sample_channel_at_time(time_delta_pts, end_t);
    Some(delta_end - delta_start)
}

/// Formats section delta into string: `+0.142s` (time lost) or `-0.085s` (time gained).
pub fn format_section_delta(delta: f64) -> String {
    if delta >= 0.0 {
        format!("+{:.3}s", delta)
    } else {
        format!("{:.3}s", delta)
    }
}

/// Returns Red for positive delta (time lost) and Green for negative delta (time gained).
pub fn section_delta_color(delta: f64, is_dark: bool) -> Color32 {
    if delta > 0.0001 {
        // Red - Time Lost
        if is_dark {
            Color32::from_rgb(255, 92, 92)
        } else {
            Color32::from_rgb(210, 40, 40)
        }
    } else if delta < -0.0001 {
        // Green - Time Gained
        if is_dark {
            Color32::from_rgb(70, 210, 132)
        } else {
            Color32::from_rgb(20, 140, 60)
        }
    } else {
        // Neutral grey
        if is_dark {
            Color32::from_rgb(180, 180, 180)
        } else {
            Color32::from_rgb(100, 100, 100)
        }
    }
}

/// Samples a 2D cache (`[[time, value]]`) at `time_sec` using linear interpolation.
pub fn sample_channel_at_time(pts_cache: &[[f64; 2]], time_sec: f64) -> f64 {
    if pts_cache.is_empty() {
        return 0.0;
    }
    if time_sec <= pts_cache[0][0] {
        return pts_cache[0][1];
    }
    if time_sec >= pts_cache[pts_cache.len() - 1][0] {
        return pts_cache[pts_cache.len() - 1][1];
    }

    match pts_cache.binary_search_by(|p| p[0].partial_cmp(&time_sec).unwrap()) {
        Ok(idx) => pts_cache[idx][1],
        Err(idx) => {
            if idx == 0 {
                pts_cache[0][1]
            } else if idx >= pts_cache.len() {
                pts_cache[pts_cache.len() - 1][1]
            } else {
                let p0 = pts_cache[idx - 1];
                let p1 = pts_cache[idx];
                let span = p1[0] - p0[0];
                if span <= 1e-6 {
                    p0[1]
                } else {
                    let factor = (time_sec - p0[0]) / span;
                    p0[1] + factor * (p1[1] - p0[1])
                }
            }
        }
    }
}

/// Samples parallel series (`dist_series`, `val_series`) at `target_dist` using linear interpolation.
pub fn sample_channel_at_distance(dist_series: &[f64], val_series: &[f64], target_dist: f64) -> f64 {
    if dist_series.is_empty() || val_series.is_empty() {
        return 0.0;
    }
    let n = dist_series.len().min(val_series.len());
    if target_dist <= dist_series[0] {
        return val_series[0];
    }
    if target_dist >= dist_series[n - 1] {
        return val_series[n - 1];
    }

    match dist_series[..n].binary_search_by(|d| d.partial_cmp(&target_dist).unwrap()) {
        Ok(idx) => val_series[idx],
        Err(idx) => {
            if idx == 0 {
                val_series[0]
            } else if idx >= n {
                val_series[n - 1]
            } else {
                let d0 = dist_series[idx - 1];
                let d1 = dist_series[idx];
                let span = d1 - d0;
                if span <= 1e-6 {
                    val_series[idx - 1]
                } else {
                    let factor = (target_dist - d0) / span;
                    val_series[idx - 1] + factor * (val_series[idx] - val_series[idx - 1])
                }
            }
        }
    }
}

/// Struct representing a live snapshot of driver control inputs for gauge overlays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveInputSample {
    /// Throttle pedal input percentage (0.0 to 100.0)
    pub throttle: f64,
    /// Brake pedal input percentage (0.0 to 100.0)
    pub brake: f64,
    /// Steering wheel angle in degrees (e.g. -180.0 to +180.0)
    pub steering_angle_deg: f64,
    /// Steering wheel angle in radians (e.g. -PI to +PI)
    pub steering_angle_rad: f64,
    /// Optional section delta at this instant
    pub time_delta: Option<f64>,
}

impl Default for LiveInputSample {
    fn default() -> Self {
        Self {
            throttle: 0.0,
            brake: 0.0,
            steering_angle_deg: 0.0,
            steering_angle_rad: 0.0,
            time_delta: None,
        }
    }
}

impl LiveInputSample {
    pub fn new(throttle: f64, brake: f64, steering_deg: f64, time_delta: Option<f64>) -> Self {
        Self {
            throttle: throttle.clamp(0.0, 100.0),
            brake: brake.clamp(0.0, 100.0),
            steering_angle_deg: steering_deg,
            steering_angle_rad: steering_deg.to_radians(),
            time_delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_channel_at_time_interpolates_correctly() {
        let pts = vec![[0.0, 0.0], [10.0, 100.0], [20.0, 50.0]];
        assert_eq!(sample_channel_at_time(&pts, 0.0), 0.0);
        assert_eq!(sample_channel_at_time(&pts, 5.0), 50.0);
        assert_eq!(sample_channel_at_time(&pts, 10.0), 100.0);
        assert_eq!(sample_channel_at_time(&pts, 15.0), 75.0);
        assert_eq!(sample_channel_at_time(&pts, 25.0), 50.0);
    }

    #[test]
    fn sample_channel_at_distance_interpolates_correctly() {
        let dist = vec![100.0, 200.0, 300.0];
        let val = vec![0.0, 1.0, 0.5];
        assert_eq!(sample_channel_at_distance(&dist, &val, 150.0), 0.5);
        assert_eq!(sample_channel_at_distance(&dist, &val, 300.0), 0.5);
    }

    #[test]
    fn section_delta_math_and_colors() {
        let pts = vec![[5.0, 0.10], [15.0, 0.40], [25.0, 0.20]];
        let delta = calculate_section_delta_from_cache(&pts, 5.0, 15.0).unwrap();
        assert!((delta - 0.30).abs() < 1e-5);
        assert_eq!(format_section_delta(delta), "+0.300s");
        assert_eq!(section_delta_color(delta, true), Color32::from_rgb(255, 92, 92));

        let gained = calculate_section_delta_from_cache(&pts, 15.0, 25.0).unwrap();
        assert!((gained - (-0.20)).abs() < 1e-5);
        assert_eq!(format_section_delta(gained), "-0.200s");
        assert_eq!(section_delta_color(gained, true), Color32::from_rgb(70, 210, 132));
    }

    #[test]
    fn live_input_sample_converts_degrees_to_radians() {
        let sample = LiveInputSample::new(85.0, 10.0, 90.0, Some(0.12));
        assert_eq!(sample.throttle, 85.0);
        assert_eq!(sample.brake, 10.0);
        assert_eq!(sample.steering_angle_deg, 90.0);
        assert!((sample.steering_angle_rad - std::f64::consts::FRAC_PI_2).abs() < 1e-5);
    }
}
