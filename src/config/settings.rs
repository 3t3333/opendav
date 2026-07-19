use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub dark_mode: bool,
    pub corner_merge_threshold: f64,
    pub use_metric: bool,
    #[serde(default)]
    pub mapbox_api_key: String,
    #[serde(default)]
    pub recent_files: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            corner_merge_threshold: 20.0,
            use_metric: true,
            mapbox_api_key: String::new(),
            recent_files: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn add_recent_file(&mut self, file: String) {
        self.recent_files.retain(|f| f != &file);
        self.recent_files.insert(0, file);
        if self.recent_files.len() > 5 {
            self.recent_files.truncate(5);
        }
    }
    pub fn load() -> Self {
        if let Ok(data) = std::fs::read_to_string("opendav_config.json") {
            if let Ok(settings) = serde_json::from_str(&data) {
                return settings;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("opendav_config.json", data);
        }
    }
}
