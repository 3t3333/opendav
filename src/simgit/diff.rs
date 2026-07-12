use super::setup_parser::SetupData;

#[derive(Debug, Clone)]
pub struct ParameterDiff {
    pub key: String,
    pub previous_value: String,
    pub new_value: String,
    pub numeric_delta: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SetupDiff {
    pub changes: Vec<ParameterDiff>,
}

impl SetupDiff {
    pub fn compare(previous: &SetupData, new: &SetupData) -> Self {
        let mut changes = Vec::new();
        
        let all_keys: std::collections::HashSet<_> = previous.parameters.keys()
            .chain(new.parameters.keys())
            .collect();
            
        let mut sorted_keys: Vec<_> = all_keys.into_iter().collect();
        sorted_keys.sort();

        for key in sorted_keys {
            let prev_val = previous.parameters.get(key).cloned().unwrap_or_else(|| "N/A".to_string());
            let new_val = new.parameters.get(key).cloned().unwrap_or_else(|| "N/A".to_string());
            
            if prev_val != new_val {
                let numeric_delta = Self::calculate_numeric_delta(&prev_val, &new_val);
                changes.push(ParameterDiff {
                    key: key.clone(),
                    previous_value: prev_val,
                    new_value: new_val,
                    numeric_delta,
                });
            }
        }
        
        Self { changes }
    }

    fn calculate_numeric_delta(prev: &str, new: &str) -> Option<f64> {
        let prev_num = prev.split_whitespace().next().and_then(|s| s.parse::<f64>().ok());
        let new_num = new.split_whitespace().next().and_then(|s| s.parse::<f64>().ok());
        
        match (prev_num, new_num) {
            (Some(p), Some(n)) => Some(n - p),
            _ => None,
        }
    }

    pub fn summarize(&self) -> String {
        if self.changes.is_empty() {
            return "<< No Changes".to_string();
        }

        let mut has_aero = false;
        let mut has_susp = false;
        let mut has_chassis = false;

        for change in &self.changes {
            let k = change.key.as_str();
            if k.contains("Wing") || k.contains("Aero") {
                has_aero = true;
            } else if k.contains("RideHeight") || k.contains("Spring") || k.contains("Camber") || k.contains("Arb") {
                has_susp = true;
            } else {
                has_chassis = true;
            }
        }

        let mut tags = Vec::new();
        if has_susp { tags.push("Suspension"); }
        if has_aero { tags.push("Aero"); }
        if has_chassis { tags.push("Chassis"); }

        format!("<< {} Changes", tags.join(" & "))
    }
}
