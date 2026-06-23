pub const DARK_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(10, 10, 10);      // #0A0A0A Obsidian
pub const LIGHT_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(227, 226, 225); // #E3E2E1 Slate White
pub const ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(242, 82, 37);      // #F25225 Electric Blaze Orange
pub const SUB_ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(102, 72, 212); // #6648D4 Electric Indigo Purple
pub const SPEED_COLOR: egui::Color32 = egui::Color32::from_rgb(78, 159, 245);      // Calm Sky Blue for Ground Speed

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum WorksheetTab {
    Basic,             // 1. Basic (Driver Inputs: Speed, Throttle, Brake, Steering, RPM, Gear)
    DynamicRake,       // 2. Dynamic Rake Analyzer
    TireEnergy,        // 3. Tire Energy Profiler
    TireFuelWindows,   // 4. Tire & Fuel Windows
    TireTempLoad,      // 5. Tire Temp/Load Map
    MathSandbox,       // 6. Custom Math Sandbox
    EmpiricalAero,     // 7. Empirical Aero Map
    DownforceMapping,  // 8. Downforce Mapping
    PitchPlatform,     // 9. Pitch & Platform
    HandlingAnalyzer,  // 10. Handling Analyzer (Yaw Error)
    TlltdDistribution, // 11. TLLTD Distribution
    CompressionRates,  // 12. Compression Rates
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CacheSelector {
    Speed,
    RPM,
    Throttle,
    Brake,
    Steering,
    FrontHeight,
    RearHeight,
    Rake, 
}



pub struct TraceSpec {
    pub name: &'static str,
    pub cache: CacheSelector,
    pub color: egui::Color32,
    pub width: f32,
    pub unit: &'static str,
}

pub struct LaneSpec {
    pub title: &'static str,
    pub y_min: f64,
    pub y_max: f64,
    pub traces: Vec<TraceSpec>,
}

pub struct WorksheetConfig {
    pub lanes: Vec<LaneSpec>,
}

impl WorksheetConfig {
    pub fn basic() -> Self {
        Self {
            lanes: vec![
                LaneSpec {
                    title: "Ground Speed",
                    y_min: 76.0,
                    y_max: 98.0,
                    traces: vec![
                        TraceSpec { name: "Speed", cache: CacheSelector::Speed, color: SPEED_COLOR, width: 2.2, unit: " km/h" },
                    ],
                },
                LaneSpec {
                    title: "Engine RPM",
                    y_min: 52.0,
                    y_max: 72.0,
                    traces: vec![
                        TraceSpec { name: "RPM", cache: CacheSelector::RPM, color: egui::Color32::from_rgb(241, 196, 15), width: 2.2, unit: "" },
                    ],
                },
                LaneSpec {
                    title: "Pedal Inputs",
                    y_min: 28.0,
                    y_max: 48.0,
                    traces: vec![
                        TraceSpec { name: "Throttle", cache: CacheSelector::Throttle, color: egui::Color32::from_rgb(46, 204, 113), width: 2.2, unit: "%" },
                        TraceSpec { name: "Brake", cache: CacheSelector::Brake, color: egui::Color32::from_rgb(231, 76, 60), width: 2.2, unit: "%" },
                    ],
                },
                LaneSpec {
                    title: "Steering",
                    y_min: 10.0,
                    y_max: 24.0,
                    traces: vec![
                        TraceSpec { name: "Steering Angle", cache: CacheSelector::Steering, color: SUB_ACCENT_COLOR, width: 2.2, unit: "°" },
                    ],
                },
            ],
        }
    }

    pub fn rake() -> Self {
        Self {
            lanes: vec![
                LaneSpec {
                    title: "Ground Speed",
                    y_min: 70.0,
                    y_max: 98.0,
                    traces: vec![
                        TraceSpec { name: "Speed", cache: CacheSelector::Speed, color: SPEED_COLOR, width: 2.2, unit: " km/h" },
                    ],
                },
                LaneSpec {
                    title: "Axle Heights",
                    y_min: 40.0,
                    y_max: 66.0,
                    traces: vec![
                        TraceSpec { name: "Front RH", cache: CacheSelector::FrontHeight, color: SUB_ACCENT_COLOR, width: 2.2, unit: "mm" },
                        TraceSpec { name: "Rear RH", cache: CacheSelector::RearHeight, color: egui::Color32::from_rgb(255, 20, 147), width: 2.2, unit: "mm" },
                    ],
                },
                LaneSpec {
                    title: "Chassis Attitude",
                    y_min: 12.0,
                    y_max: 36.0,
                    traces: vec![
                        TraceSpec { name: "Dynamic Rake", cache: CacheSelector::Rake, color: ACCENT_COLOR, width: 2.2, unit: "mm" },
                    ],
                },
            ],
        }
    }
}
