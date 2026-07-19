use std::fs;
use std::path::Path;
use std::io::Read;

const R_EARTH: f64 = 6378137.0;

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
    
    // Bounds in meters
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

pub fn calculate_bbox(lat_col: &[f64], lon_col: &[f64], lat0: f64, lon0: f64) -> Option<BoundingBox> {
    if lat_col.is_empty() || lon_col.is_empty() { return None; }
    
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;
    
    let mut x_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    
    let lat0_rad = lat0 * std::f64::consts::PI / 180.0;
    let lon0_rad = lon0 * std::f64::consts::PI / 180.0;
    
    for i in 0..lat_col.len() {
        let lat = lat_col[i] * 180.0 / std::f64::consts::PI;
        let lon = lon_col[i] * 180.0 / std::f64::consts::PI;
        if lat < min_lat { min_lat = lat; }
        if lat > max_lat { max_lat = lat; }
        if lon < min_lon { min_lon = lon; }
        if lon > max_lon { max_lon = lon; }
        
        let lat_rad = lat * std::f64::consts::PI / 180.0;
        let lon_rad = lon * std::f64::consts::PI / 180.0;
        
        let x = R_EARTH * (lon_rad - lon0_rad) * lat0_rad.cos();
        let y = R_EARTH * (lat_rad - lat0_rad);
        
        if x < x_min { x_min = x; }
        if x > x_max { x_max = x; }
        if y < y_min { y_min = y; }
        if y > y_max { y_max = y; }
    }
    
    // Add a 10% margin
    let x_range = x_max - x_min;
    let y_range = y_max - y_min;
    
    let lat_margin = (max_lat - min_lat) * 0.1;
    let lon_margin = (max_lon - min_lon) * 0.1;
    
    Some(BoundingBox {
        min_lon: min_lon - lon_margin,
        min_lat: min_lat - lat_margin,
        max_lon: max_lon + lon_margin,
        max_lat: max_lat + lat_margin,
        x_min: x_min - (x_range * 0.1),
        x_max: x_max + (x_range * 0.1),
        y_min: y_min - (y_range * 0.1),
        y_max: y_max + (y_range * 0.1),
    })
}

pub fn get_satellite_image(track_id: i32, bbox: &BoundingBox) -> Option<Vec<u8>> {
    let api_key = std::env::var("MAPBOX_API_KEY").unwrap_or_default();
    let cache_path = format!("assets/tracks/{}.jpg", track_id);
    
    // 1. Try to load from cache
    if Path::new(&cache_path).exists() {
        if let Ok(bytes) = fs::read(&cache_path) {
            return Some(bytes);
        }
    }
    
    // 2. Fetch from Mapbox if key is provided
    if api_key.is_empty() {
        return None;
    }
    
    let width_m = bbox.x_max - bbox.x_min;
    let height_m = bbox.y_max - bbox.y_min;
    
    let mut width_px = 1280.0;
    let mut height_px = 1280.0;
    
    if width_m > height_m {
        height_px = 1280.0 * (height_m / width_m);
    } else {
        width_px = 1280.0 * (width_m / height_m);
    }
    
    let width_px = width_px.round() as i32;
    let height_px = height_px.round() as i32;
    
    let url = format!(
        "https://api.mapbox.com/styles/v1/mapbox/dark-v11/static/[{},{},{},{}]/{}x{}@2x?access_token={}",
        bbox.min_lon, bbox.min_lat, bbox.max_lon, bbox.max_lat,
        width_px, height_px, api_key
    );
    
    if let Ok(mut response) = ureq::get(&url).call() {
        if let Ok(bytes) = response.body_mut().read_to_vec() {
            // Save to cache
            let _ = fs::create_dir_all("assets/tracks");
            let _ = fs::write(&cache_path, &bytes);
            return Some(bytes);
        }
    }
    
    None
}
