use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use super::setup_parser::SetupData;
use super::diff::SetupDiff;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub file_name: String,
    pub timestamp: u64,
    pub diff_summary: String,
    #[serde(default)]
    pub track_id: Option<i32>,
}

pub fn get_history(proj_dir: &Path) -> Vec<HistoryEntry> {
    let history_file = proj_dir.join("setuphistory.json");
    if let Ok(data) = fs::read_to_string(&history_file) {
        if let Ok(history) = serde_json::from_str::<Vec<HistoryEntry>>(&data) {
            return history;
        }
    }
    Vec::new()
}

pub fn commit_files(proj_dir: &Path, files: &[PathBuf]) {
    let mut history = get_history(proj_dir);
    let setups_dir = proj_dir.join("setups");
    
    for file in files {
        if let Some(file_name) = file.file_name() {
            let file_name_str = file_name.to_string_lossy().to_string();
            
            // Check if the file is already in history to prevent duplicates
            if history.iter().any(|entry| entry.file_name == file_name_str) {
                continue;
            }

            let dest = setups_dir.join(file_name);
            // Only proceed if copy succeeds
            if fs::copy(file, &dest).is_ok() {
                // Parse the new setup
                if let Ok(new_data) = SetupData::from_ibt_file(&dest) {
                    let summary = if let Some(last_entry) = history.last() {
                        let last_path = setups_dir.join(&last_entry.file_name);
                        if let Ok(prev_data) = SetupData::from_ibt_file(&last_path) {
                            let diff = SetupDiff::compare(&prev_data, &new_data);
                            diff.summarize()
                        } else {
                            "<< Baseline Setup".to_string()
                        }
                    } else {
                        "<< Baseline Setup".to_string()
                    };
                    
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    let track_id = new_data.parameters.get("TrackID").and_then(|s| s.parse::<i32>().ok());
                    if let Some(tid) = track_id {
                        let dest_path = dest.clone();
                        std::thread::spawn(move || {
                            let dest_dir = std::path::Path::new("exports/track_maps");
                            if !dest_dir.exists() {
                                let _ = std::fs::create_dir_all(dest_dir);
                            }
                            let dest_file = dest_dir.join(format!("{}.json", tid));
                            if !dest_file.exists() {
                                println!("Generating track map JSON from telemetry: {}", dest_path.display());
                                if let Err(e) = crate::signals::track_map_generator::generate_track_map_json(&dest_path, &dest_file) {
                                    eprintln!("Failed to generate JSON track map: {}", e);
                                }
                            }
                        });
                    }
                        
                    history.push(HistoryEntry {
                        file_name: file_name_str,
                        timestamp,
                        diff_summary: summary,
                        track_id,
                    });
                }
            }
        }
    }
    
    // Save history
    let history_file = proj_dir.join("setuphistory.json");
    if let Ok(json) = serde_json::to_string_pretty(&history) {
        let _ = fs::write(history_file, json);
    }
}

pub fn remove_file(proj_dir: &Path, file_name: &str) {
    let mut history = get_history(proj_dir);
    history.retain(|e| e.file_name != file_name);
    
    let history_file = proj_dir.join("setuphistory.json");
    if let Ok(json) = serde_json::to_string_pretty(&history) {
        let _ = fs::write(history_file, json);
    }
    
    let file_path = proj_dir.join("setups").join(file_name);
    if file_path.exists() {
        let _ = fs::remove_file(file_path);
    }
}

