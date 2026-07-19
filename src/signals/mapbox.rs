use std::fs;
use std::path::Path;
use std::io::Read;

fn fetch_layer(
    api_key: &str,
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
    use image::{GenericImage, RgbaImage};
    use std::io::Cursor;

    let cache_path = maps_dir.join(format!("{}_{}.png", track_id, suffix));
    let bounds_path = maps_dir.join(format!("{}_{}_bounds.json", track_id, suffix));

    let (wm_min_x, wm_min_y) = wgs84_to_web_mercator(min_lon, min_lat);
    let (wm_max_x, wm_max_y) = wgs84_to_web_mercator(max_lon, max_lat);
    let cx = (wm_min_x + wm_max_x) / 2.0;
    let cy = (wm_min_y + wm_max_y) / 2.0;
    let width = wm_max_x - wm_min_x;
    let height = wm_max_y - wm_min_y;
    let mut size = f64::max(width, height);
    size *= padding;
    
    let pad_min_lon = web_mercator_to_wgs84(cx - size / 2.0, cy - size / 2.0).0;
    let pad_min_lat = web_mercator_to_wgs84(cx - size / 2.0, cy - size / 2.0).1;
    let pad_max_lon = web_mercator_to_wgs84(cx + size / 2.0, cy + size / 2.0).0;
    let pad_max_lat = web_mercator_to_wgs84(cx + size / 2.0, cy + size / 2.0).1;

    let final_bounds = [pad_min_lon, pad_min_lat, pad_max_lon, pad_max_lat];

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

    let grid_size = std::cmp::max(1, max_grid_size / 4);
    let tile_res = 1280;
    println!("Fetching Mapbox {} map ({}x{} grid)...", suffix, grid_size, grid_size);

    let mut stitched_img = RgbaImage::new(grid_size * tile_res, grid_size * tile_res);

    use rayon::prelude::*;

    let mut tile_coords = Vec::new();
    for r in 0..grid_size {
        for c in 0..grid_size {
            tile_coords.push((c, r));
        }
    }

    let square_min_x = cx - size / 2.0;
    let square_max_y = cy + size / 2.0;

    let fetched_tiles: Vec<Result<((u32, u32), Vec<u8>), String>> = tile_coords.par_iter().map(|&(c, r)| {
        let tile_min_x = square_min_x + (c as f64 * (size / grid_size as f64));
        let tile_max_x = tile_min_x + (size / grid_size as f64);
        let tile_max_y = square_max_y - (r as f64 * (size / grid_size as f64));
        let tile_min_y = tile_max_y - (size / grid_size as f64);

        let t_min_lon = web_mercator_to_wgs84(tile_min_x, tile_min_y).0;
        let t_min_lat = web_mercator_to_wgs84(tile_min_x, tile_min_y).1;
        let t_max_lon = web_mercator_to_wgs84(tile_max_x, tile_max_y).0;
        let t_max_lat = web_mercator_to_wgs84(tile_max_x, tile_max_y).1;

        let url = format!(
            "https://api.mapbox.com/styles/v1/mapbox/satellite-v9/static/[{},{},{},{}]/640x640@2x?access_token={}&logo=false&attribution=false",
            t_min_lon, t_min_lat, t_max_lon, t_max_lat, api_key
        );
        
        let response = ureq::get(&url).call().map_err(|e| format!("Network error: {}", e))?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut response.into_body().into_reader(), &mut bytes).map_err(|e| e.to_string())?;
        Ok(((c, r), bytes))
    }).collect();

    for res in fetched_tiles {
        if let Ok(((c, r), bytes)) = res {
            if let Ok(img) = image::load_from_memory(&bytes) {
                stitched_img.copy_from(&img.to_rgba8(), c * tile_res, r * tile_res).ok();
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

pub fn fetch_mapbox_image(
    api_key: &str,
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

    let (bg_bytes, bg_bounds) = match fetch_layer(api_key, track_id, min_lon, min_lat, max_lon, max_lat, 10.0, 4, "dark_bg", maps_dir) {
        Ok((b, bounds)) => (Some(b), Some(bounds)),
        Err(e) => {
            eprintln!("Failed to fetch BG layer: {}", e);
            (None, None)
        }
    };

    let (fg_bytes, fg_bounds) = fetch_layer(api_key, track_id, min_lon, min_lat, max_lon, max_lat, 1.4, max_grid_size, "dark", maps_dir)?;

    Ok((fg_bytes, fg_bounds, bg_bytes, bg_bounds))
}

pub fn web_mercator_to_wgs84(x: f64, y: f64) -> (f64, f64) {
    let r_earth = 6378137.0;
    let lon_deg = (x / r_earth).to_degrees();
    let lat_deg = (2.0 * (y / r_earth).exp().atan() - std::f64::consts::PI / 2.0).to_degrees();
    (lon_deg, lat_deg)
}

pub fn wgs84_to_web_mercator(lon_deg: f64, lat_deg: f64) -> (f64, f64) {
    let r_earth = 6378137.0;
    let x = lon_deg.to_radians() * r_earth;
    let lat_rad = lat_deg.to_radians();
    let y = (std::f64::consts::PI / 4.0 + lat_rad / 2.0).tan().ln() * r_earth;
    (x, y)
}
