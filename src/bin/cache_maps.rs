use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use inquire::{Select, Text};
use indicatif::{ProgressBar, ProgressStyle};

// Bring in fetchers
#[path = "../signals/mapbox.rs"]
mod mapbox;

#[path = "../signals/google_maps.rs"]
mod google_maps;

#[path = "../data/ibt_parser.rs"]
mod ibt_parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- OpenDAV Map Cache Manager ---");

    let dir_path_str = Text::new("Path to telemetry directory (.ibt files):")
        .with_default("C:\\Users\\bukar\\Documents\\iRacing\\telemetry")
        .prompt()?;
    
    let dir_path = Path::new(&dir_path_str);
    if !dir_path.exists() || !dir_path.is_dir() {
        eprintln!("Error: Directory does not exist: {:?}", dir_path);
        std::process::exit(1);
    }

    let providers = vec!["Google Satellite (No Watermark)", "Mapbox Satellite"];
    let provider = Select::new("Select Map Provider:", providers).prompt()?;

    let resolutions = vec!["Normal (16x16 / 4096px - Instant)", "Ultra (32x32 / 8192px - 1.6GB Memory)"];
    let res_choice = Select::new("Select Resolution (Ultra might freeze main app):", resolutions).prompt()?;
    
    let is_google = provider.contains("Google");
    let max_grid_size = if res_choice.contains("Ultra") { 32 } else { 16 };
    
    let mut mapbox_api_key = String::new();
    if !is_google {
        mapbox_api_key = Text::new("Enter Mapbox API Key:")
            .prompt()?;
    }

    println!("Scanning directory for .ibt files...");
    let mut files_to_process = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("ibt") {
            files_to_process.push(path);
        }
    }

    if files_to_process.is_empty() {
        println!("No .ibt files found.");
        return Ok(());
    }

    let pb = ProgressBar::new(files_to_process.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}")
        .unwrap()
        .progress_chars("#>-"));

    let mut cached_tracks = HashSet::new();

    for path in files_to_process {
        pb.set_message(format!("Reading {:?}", path.file_name().unwrap()));
        
        match ibt_parser::parse_ibt_file(path.to_str().unwrap()) {
            Ok(session) => {
                let track_id = session.track_id;
                if cached_tracks.contains(&track_id) {
                    pb.inc(1);
                    continue;
                }

                let df = session.dataframe;
                let lat_col = df.column("Lat").ok().and_then(|c| c.f64().ok());
                let lon_col = df.column("Lon").ok().and_then(|c| c.f64().ok());

                if let (Some(la_col), Some(lo_col)) = (lat_col, lon_col) {
                    let mut min_lat = f64::MAX;
                    let mut min_lon = f64::MAX;
                    let mut max_lat = f64::MIN;
                    let mut max_lon = f64::MIN;

                    let n = la_col.len();
                    for i in 0..n {
                        if let (Some(lat), Some(lon)) = (la_col.get(i), lo_col.get(i)) {
                            if lat != 0.0 && lon != 0.0 && lat > -90.0 && lat < 90.0 {
                                min_lat = min_lat.min(lat);
                                max_lat = max_lat.max(lat);
                                min_lon = min_lon.min(lon);
                                max_lon = max_lon.max(lon);
                            }
                        }
                    }

                    if min_lat != f64::MAX {
                        pb.set_message(format!("Fetching Track {}", track_id));
                        if is_google {
                            let _ = google_maps::fetch_google_map_image(track_id, min_lon, min_lat, max_lon, max_lat, max_grid_size);
                        } else {
                            let _ = mapbox::fetch_mapbox_image(&mapbox_api_key, track_id, min_lon, min_lat, max_lon, max_lat, max_grid_size);
                        }
                        cached_tracks.insert(track_id);
                    }
                }
            },
            Err(_) => {} // Ignore parse errors in cache loop
        }
        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    println!("Successfully cached {} unique tracks.", cached_tracks.len());

    Ok(())
}
