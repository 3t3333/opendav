use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub dark_mode: bool,
    pub corner_merge_threshold: f64,
    pub use_metric: bool,
    #[serde(default)]
    pub show_graph_grid: bool,
    #[serde(default = "default_graph_grid_opacity")]
    pub graph_grid_opacity: f32,
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
            show_graph_grid: false,
            graph_grid_opacity: default_graph_grid_opacity(),
            mapbox_api_key: String::new(),
            recent_files: Vec::new(),
        }
    }
}

const fn default_graph_grid_opacity() -> f32 {
    0.35
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

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn older_settings_files_receive_graph_grid_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
                "dark_mode": true,
                "corner_merge_threshold": 20.0,
                "use_metric": true
            }"#,
        )
        .expect("legacy settings should remain readable");

        assert!(!settings.show_graph_grid);
        assert!((settings.graph_grid_opacity - 0.35).abs() < f32::EPSILON);
    }
}
