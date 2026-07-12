use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub dark_mode: bool,
    pub corner_merge_threshold: f64,
    pub use_metric: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            corner_merge_threshold: 20.0,
            use_metric: true,
        }
    }
}

impl AppSettings {
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
