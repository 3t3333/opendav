use std::fs;
use std::path::Path;
use image::{GenericImage, RgbaImage};
use std::io::Cursor;
use std::io::Read;

fn lon_to_x(lon: f64, zoom: u32) -> f64 {
    (lon + 180.0) / 360.0 * ((1 << zoom) as f64)
}

fn lat_to_y(lat: f64, zoom: u32) -> f64 {
    let lat_rad = lat.to_radians();
    (1.0 - ((lat_rad.tan() + 1.0 / lat_rad.cos()).ln()) / std::f64::consts::PI) / 2.0 * ((1 << zoom) as f64)
}

fn x_to_lon(x: f64, zoom: u32) -> f64 {
    x / ((1 << zoom) as f64) * 360.0 - 180.0
}

fn y_to_lat(y: f64, zoom: u32) -> f64 {
    let n = std::f64::consts::PI - 2.0 * std::f64::consts::PI * y / ((1 << zoom) as f64);
    (0.5 * (n.exp() - (-n).exp())).atan().to_degrees()
}

pub fn wgs84_to_web_mercator(lon_deg: f64, lat_deg: f64) -> (f64, f64) {
    let r_earth = 6378137.0;
    let x = r_earth * lon_deg.to_radians();
    let y = r_earth * ((std::f64::consts::PI / 4.0 + lat_deg.to_radians() / 2.0).tan()).ln();
    (x, y)
}

pub fn web_mercator_to_wgs84(x: f64, y: f64) -> (f64, f64) {
    let r_earth = 6378137.0;
    let lon_deg = (x / r_earth).to_degrees();
    let lat_deg = (2.0 * (y / r_earth).exp().atan() - std::f64::consts::PI / 2.0).to_degrees();
    (lon_deg, lat_deg)
}

fn fetch_layer(
    track_id: i32,
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    padding: f64,
    max_grid_size: u32,
    suffix: &str,
    maps_dir: &Path,
) -> Result<(Vec<u8>, [f64; 4]), Box<dyn std::error::Error>> {
    let cache_path = maps_dir.join(format!("{}_{}.png", track_id, suffix));
    let bounds_path = maps_dir.join(format!("{}_{}_bounds.json", track_id, suffix));

    let (wm_min_x, wm_min_y) = wgs84_to_web_mercator(min_lon, min_lat);
    let (wm_max_x, wm_max_y) = wgs84_to_web_mercator(max_lon, max_lat);
    let cx = (wm_min_x + wm_max_x) / 2.0;
    let cy = (wm_min_y + wm_max_y) / 2.0;
    let width = wm_max_x - wm_min_x;
    let height = wm_max_y - wm_min_y;
    let mut size = f64::max(width, height);
    size *= padding; // dynamically padded
    
    let pad_min_lon = web_mercator_to_wgs84(cx - size / 2.0, cy - size / 2.0).0;
    let pad_min_lat = web_mercator_to_wgs84(cx - size / 2.0, cy - size / 2.0).1;
    let pad_max_lon = web_mercator_to_wgs84(cx + size / 2.0, cy + size / 2.0).0;
    let pad_max_lat = web_mercator_to_wgs84(cx + size / 2.0, cy + size / 2.0).1;

    let mut zoom = 20;
    let mut min_tx = 0;
    let mut max_tx = 0;
    let mut min_ty = 0;
    let mut max_ty = 0;

    loop {
        min_tx = lon_to_x(pad_min_lon, zoom).floor() as u32;
        max_tx = lon_to_x(pad_max_lon, zoom).floor() as u32;
        min_ty = lat_to_y(pad_max_lat, zoom).floor() as u32; 
        max_ty = lat_to_y(pad_min_lat, zoom).floor() as u32;

        let tiles_x = max_tx - min_tx + 1;
        let tiles_y = max_ty - min_ty + 1;

        if tiles_x <= max_grid_size && tiles_y <= max_grid_size {
            break;
        }
        zoom -= 1;
    }

    let final_min_lon = x_to_lon(min_tx as f64, zoom);
    let final_max_lon = x_to_lon((max_tx + 1) as f64, zoom);
    let final_max_lat = y_to_lat(min_ty as f64, zoom);
    let final_min_lat = y_to_lat((max_ty + 1) as f64, zoom);
    let final_bounds = [final_min_lon, final_min_lat, final_max_lon, final_max_lat];

    if cache_path.exists() {
        println!("Loading {} map from cache: {:?}", suffix, cache_path);
        let mut loaded_bounds = final_bounds;
        if bounds_path.exists() {
            if let Ok(json_str) = fs::read_to_string(&bounds_path) {
                if let Ok(bounds) = serde_json::from_str::<[f64; 4]>(&json_str) {
                    loaded_bounds = bounds;
                }
            }
        }
        return Ok((fs::read(&cache_path)?, loaded_bounds));
    }

    let tiles_x = max_tx - min_tx + 1;
    let tiles_y = max_ty - min_ty + 1;
    let tile_res = 256;
    println!("Fetching Google {} tiles at zoom {} ({}x{} grid)...", suffix, zoom, tiles_x, tiles_y);

    let mut stitched_img = RgbaImage::new(tiles_x * tile_res, tiles_y * tile_res);

    use rayon::prelude::*;
    let mut tile_coords = Vec::new();
    for ty in min_ty..=max_ty {
        for tx in min_tx..=max_tx {
            tile_coords.push((tx, ty));
        }
    }

    let fetched_tiles: Vec<Result<((u32, u32), Vec<u8>), String>> = tile_coords.par_iter().map(|&(tx, ty)| {
        let url = format!("https://mt1.google.com/vt/lyrs=s&x={}&y={}&z={}", tx, ty, zoom);
        let response = ureq::get(&url).call().map_err(|e| format!("Network error: {}", e))?;
        let mut bytes = Vec::new();
        response.into_body().into_reader().read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(((tx, ty), bytes))
    }).collect();

    for res in fetched_tiles {
        if let Ok(((tx, ty), bytes)) = res {
            if let Ok(img) = image::load_from_memory(&bytes) {
                stitched_img.copy_from(&img.to_rgba8(), (tx - min_tx) * tile_res, (ty - min_ty) * tile_res).ok();
            }
        }
    }

    let mut final_bytes = Vec::new();
    stitched_img.write_to(&mut Cursor::new(&mut final_bytes), image::ImageFormat::Png)?;
    
    fs::write(&cache_path, &final_bytes)?;
    if let Ok(bounds_json) = serde_json::to_string(&final_bounds) {
        let _ = fs::write(&bounds_path, bounds_json);
    }
    
    Ok((final_bytes, final_bounds))
}

pub fn fetch_google_map_image(
    track_id: i32,
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    max_grid_size: u32,
) -> Result<(Vec<u8>, [f64; 4], Option<Vec<u8>>, Option<[f64; 4]>), Box<dyn std::error::Error>> {
    let maps_dir = Path::new("assets/maps");
    if !maps_dir.exists() {
        fs::create_dir_all(maps_dir)?;
    }

    let (bg_bytes, bg_bounds) = match fetch_layer(track_id, min_lon, min_lat, max_lon, max_lat, 10.0, 4, "google_bg", maps_dir) {
        Ok((b, bounds)) => (Some(b), Some(bounds)),
        Err(e) => {
            eprintln!("Failed to fetch BG layer: {}", e);
            (None, None)
        }
    };

    let (fg_bytes, fg_bounds) = fetch_layer(track_id, min_lon, min_lat, max_lon, max_lat, 1.4, max_grid_size, "google", maps_dir)?;

    Ok((fg_bytes, fg_bounds, bg_bytes, bg_bounds))
}
