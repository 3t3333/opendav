use std::collections::HashMap;
use regex::Regex;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SetupData {
    pub parameters: HashMap<String, String>,
}

impl SetupData {
    pub fn from_yaml(yaml_str: &str) -> Self {
        let mut parameters = HashMap::new();
        
        let mappings = vec![
            ("LFcoldPressure", r"LeftFront:(?:.|\n)*?StartingPressure:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RFcoldPressure", r"RightFront:(?:.|\n)*?StartingPressure:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LRcoldPressure", r"LeftRear:(?:.|\n)*?StartingPressure:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RRcoldPressure", r"RightRear:(?:.|\n)*?StartingPressure:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LFrideHeight", r"LeftFront:(?:.|\n)*?RideHeight:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RFrideHeight", r"RightFront:(?:.|\n)*?RideHeight:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LRrideHeight", r"LeftRear:(?:.|\n)*?RideHeight:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RRrideHeight", r"RightRear:(?:.|\n)*?RideHeight:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LFspringPerch", r"LeftFront:(?:.|\n)*?SpringPerchOffset:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RFspringPerch", r"RightFront:(?:.|\n)*?SpringPerchOffset:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LRspringPerch", r"LeftRear:(?:.|\n)*?SpringPerchOffset:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RRspringPerch", r"RightRear:(?:.|\n)*?SpringPerchOffset:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LFcamber", r"LeftFront:(?:.|\n)*?Camber:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RFcamber", r"RightFront:(?:.|\n)*?Camber:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("LRcamber", r"LeftRear:(?:.|\n)*?Camber:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("RRcamber", r"RightRear:(?:.|\n)*?Camber:\s*([-+]?\d*\.\d+|\d+)\s*(.*)"),
            ("TireType", r"TireType:\s*([^\n]+)"),
            ("FrontBrakePadMu", r"FrontBrakePadMu:\s*([^\n]+)"),
            ("RearBrakePadMu", r"RearBrakePadMu:\s*([^\n]+)"),
            ("BrakePressureBias", r"BrakePressureBias:\s*([-+]?\d*\.\d+|\d+)\s*([^\n]*)"),
            ("ArbSettingF", r"Front:(?:.|\n)*?ArbSetting:\s*([-+]?\d*\.\d+|\d+)\s*([^\n]*)"),
            ("ArbSettingR", r"Rear:(?:.|\n)*?ArbSetting:\s*([-+]?\d*\.\d+|\d+)\s*([^\n]*)"),
            ("WingAngle", r"WingAngle:\s*([-+]?\d*\.\d+|\d+)\s*([^\n]*)"),
            ("TrackID", r"WeekendInfo:(?:.|\n)*?TrackID:\s*(\d+)"),
        ];

        for (key, pattern) in mappings {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(yaml_str) {
                    let val = caps.get(1).map_or("", |m| m.as_str());
                    let mut unit = "";
                    if let Some(u_match) = caps.get(2) {
                        unit = u_match.as_str().trim();
                    }
                    if !unit.is_empty() {
                        parameters.insert(key.to_string(), format!("{} {}", val, unit));
                    } else {
                        parameters.insert(key.to_string(), val.to_string());
                    }
                }
            }
        }
        
        Self { parameters }
    }

    pub fn from_ibt_file(file_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut f = File::open(file_path)?;
        f.seek(SeekFrom::Start(16))?;
        let session_info_len = f.read_i32::<LittleEndian>()? as usize;
        let session_info_offset = f.read_i32::<LittleEndian>()? as u64;

        f.seek(SeekFrom::Start(session_info_offset))?;
        let mut yaml_buf = vec![0u8; session_info_len];
        f.read_exact(&mut yaml_buf)?;
        let yaml_str = String::from_utf8_lossy(&yaml_buf);

        Ok(Self::from_yaml(&yaml_str))
    }
}
