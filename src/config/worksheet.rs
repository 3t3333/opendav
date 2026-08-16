pub const DARK_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(10, 10, 10); // #0A0A0A Obsidian
pub const LIGHT_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(227, 226, 225); // #E3E2E1 Slate White
pub const ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(242, 82, 37); // #F25225 Electric Blaze Orange
pub const SUB_ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(102, 72, 212); // #6648D4 Electric Indigo Purple
pub const SPEED_COLOR: egui::Color32 = egui::Color32::from_rgb(78, 159, 245); // Calm Sky Blue for Ground Speed

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum WorksheetTab {
    Driver,  // 1. Basic (Driver Inputs: Speed, Throttle, Brake, Steering, RPM, Gear)
    Vehicle, // 2. Basic Vehicle (Ground Speed, Ride Heights, Rake, Lat G, Long G)
    Tyre,
    Shocks,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum CacheSelector {
    Speed,
    RPM,
    Throttle,
    Brake,
    Steering,
    FrontHeight,
    RearHeight,
    Rake,
    LatG,
    LongG,
    Gear,
    Clutch,
    DistanceDelta,
    TimeDelta,
    LfTempOuter,
    LfTempCenter,
    LfTempInner,
    RfTempInner,
    RfTempCenter,
    RfTempOuter,
    LrTempOuter,
    LrTempCenter,
    LrTempInner,
    RrTempInner,
    RrTempCenter,
    RrTempOuter,
    LfShockDeflection,
    RfShockDeflection,
    LrShockDeflection,
    RrShockDeflection,
}

pub const ALL_CACHE_SELECTORS: [CacheSelector; 30] = [
    CacheSelector::Speed,
    CacheSelector::RPM,
    CacheSelector::Throttle,
    CacheSelector::Brake,
    CacheSelector::Steering,
    CacheSelector::FrontHeight,
    CacheSelector::RearHeight,
    CacheSelector::Rake,
    CacheSelector::LatG,
    CacheSelector::LongG,
    CacheSelector::Gear,
    CacheSelector::Clutch,
    CacheSelector::DistanceDelta,
    CacheSelector::TimeDelta,
    CacheSelector::LfTempOuter,
    CacheSelector::LfTempCenter,
    CacheSelector::LfTempInner,
    CacheSelector::RfTempInner,
    CacheSelector::RfTempCenter,
    CacheSelector::RfTempOuter,
    CacheSelector::LrTempOuter,
    CacheSelector::LrTempCenter,
    CacheSelector::LrTempInner,
    CacheSelector::RrTempInner,
    CacheSelector::RrTempCenter,
    CacheSelector::RrTempOuter,
    CacheSelector::LfShockDeflection,
    CacheSelector::RfShockDeflection,
    CacheSelector::LrShockDeflection,
    CacheSelector::RrShockDeflection,
];

pub const TYRE_CHANNEL_GROUPS: [[CacheSelector; 3]; 4] = [
    [
        CacheSelector::LfTempCenter,
        CacheSelector::LfTempInner,
        CacheSelector::LfTempOuter,
    ],
    [
        CacheSelector::RfTempCenter,
        CacheSelector::RfTempInner,
        CacheSelector::RfTempOuter,
    ],
    [
        CacheSelector::LrTempCenter,
        CacheSelector::LrTempInner,
        CacheSelector::LrTempOuter,
    ],
    [
        CacheSelector::RrTempCenter,
        CacheSelector::RrTempInner,
        CacheSelector::RrTempOuter,
    ],
];

pub const SHOCK_CHANNEL_GROUPS: [[CacheSelector; 1]; 4] = [
    [CacheSelector::LfShockDeflection],
    [CacheSelector::RfShockDeflection],
    [CacheSelector::LrShockDeflection],
    [CacheSelector::RrShockDeflection],
];

pub const STACKED_LANE_RANGES: [(f64, f64); 4] =
    [(76.0, 98.0), (52.0, 72.0), (28.0, 48.0), (10.0, 24.0)];

pub fn worksheet_channel_group(
    selector: CacheSelector,
) -> Option<(&'static [CacheSelector], (f64, f64))> {
    for (group, range) in TYRE_CHANNEL_GROUPS.iter().zip(STACKED_LANE_RANGES) {
        if group.contains(&selector) {
            return Some((group, range));
        }
    }
    for (group, range) in SHOCK_CHANNEL_GROUPS.iter().zip(STACKED_LANE_RANGES) {
        if group.contains(&selector) {
            return Some((group, range));
        }
    }
    None
}

impl CacheSelector {
    pub const fn telemetry_source(self) -> Option<(&'static str, f64)> {
        match self {
            Self::LfTempOuter => Some(("LFtempL", 1.0)),
            Self::LfTempCenter => Some(("LFtempM", 1.0)),
            Self::LfTempInner => Some(("LFtempR", 1.0)),
            Self::RfTempInner => Some(("RFtempL", 1.0)),
            Self::RfTempCenter => Some(("RFtempM", 1.0)),
            Self::RfTempOuter => Some(("RFtempR", 1.0)),
            Self::LrTempOuter => Some(("LRtempL", 1.0)),
            Self::LrTempCenter => Some(("LRtempM", 1.0)),
            Self::LrTempInner => Some(("LRtempR", 1.0)),
            Self::RrTempInner => Some(("RRtempL", 1.0)),
            Self::RrTempCenter => Some(("RRtempM", 1.0)),
            Self::RrTempOuter => Some(("RRtempR", 1.0)),
            Self::LfShockDeflection => Some(("LFshockDefl", 1000.0)),
            Self::RfShockDeflection => Some(("RFshockDefl", 1000.0)),
            Self::LrShockDeflection => Some(("LRshockDefl", 1000.0)),
            Self::RrShockDeflection => Some(("RRshockDefl", 1000.0)),
            _ => None,
        }
    }

    pub const fn is_temperature(self) -> bool {
        matches!(
            self,
            Self::LfTempOuter
                | Self::LfTempCenter
                | Self::LfTempInner
                | Self::RfTempInner
                | Self::RfTempCenter
                | Self::RfTempOuter
                | Self::LrTempOuter
                | Self::LrTempCenter
                | Self::LrTempInner
                | Self::RrTempInner
                | Self::RrTempCenter
                | Self::RrTempOuter
        )
    }

    pub const fn is_shock_deflection(self) -> bool {
        matches!(
            self,
            Self::LfShockDeflection
                | Self::RfShockDeflection
                | Self::LrShockDeflection
                | Self::RrShockDeflection
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LaneScaling {
    Mono,
    Poly,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceSpec {
    pub name: String,
    pub cache: CacheSelector,
    pub custom_channel: Option<String>,
    pub color: egui::Color32,
    pub width: f32,
    pub unit: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct LaneSpec {
    pub title: String,
    pub y_min: f64,
    pub y_max: f64,
    pub scaling: LaneScaling,
    pub traces: Vec<TraceSpec>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum WorksheetClipboard {
    Lanes(Vec<LaneSpec>),
    Traces(Vec<TraceSpec>),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WorksheetConfig {
    pub lanes: Vec<LaneSpec>,
}

impl WorksheetConfig {
    pub fn driver(theme: AppTheme) -> Self {
        Self {
            lanes: vec![
                LaneSpec {
                    title: "Ground Speed".to_string(),
                    y_min: 76.0,
                    y_max: 98.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![TraceSpec {
                        name: "Speed".to_string(),
                        cache: CacheSelector::Speed, custom_channel: None,
                        color: theme.speed,
                        width: 2.2,
                        unit: " km/h".to_string(),
                    }],
                },
                LaneSpec {
                    title: "Engine RPM".to_string(),
                    y_min: 52.0,
                    y_max: 72.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![
                        TraceSpec {
                            name: "RPM".to_string(),
                            cache: CacheSelector::RPM, custom_channel: None,
                            color: theme.rpm,
                            width: 2.2,
                            unit: "".to_string(),
                        },
                        TraceSpec {
                            name: "Gear".to_string(),
                            cache: CacheSelector::Gear, custom_channel: None,
                            color: theme.gear,
                            width: 1.8,
                            unit: "".to_string(),
                        },
                    ],
                },
                LaneSpec {
                    title: "Pedal Inputs".to_string(),
                    y_min: 28.0,
                    y_max: 48.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![
                        TraceSpec {
                            name: "Throttle".to_string(),
                            cache: CacheSelector::Throttle, custom_channel: None,
                            color: theme.throttle,
                            width: 2.2,
                            unit: "%".to_string(),
                        },
                        TraceSpec {
                            name: "Brake".to_string(),
                            cache: CacheSelector::Brake, custom_channel: None,
                            color: theme.brake,
                            width: 2.2,
                            unit: "%".to_string(),
                        },
                        TraceSpec {
                            name: "ClutchRaw".to_string(),
                            cache: CacheSelector::Clutch, custom_channel: None,
                            color: theme.clutch,
                            width: 2.2,
                            unit: "%".to_string(),
                        },
                    ],
                },
                LaneSpec {
                    title: "Steering".to_string(),
                    y_min: 10.0,
                    y_max: 24.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![TraceSpec {
                        name: "Steering Angle".to_string(),
                        cache: CacheSelector::Steering, custom_channel: None,
                        color: theme.steering,
                        width: 2.2,
                        unit: "°".to_string(),
                    }],
                },
            ],
        }
    }

    pub fn vehicle(theme: AppTheme) -> Self {
        Self {
            lanes: vec![
                LaneSpec {
                    title: "Ground Speed".to_string(),
                    y_min: 76.0,
                    y_max: 98.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![TraceSpec {
                        name: "Speed".to_string(),
                        cache: CacheSelector::Speed, custom_channel: None,
                        color: theme.speed,
                        width: 2.2,
                        unit: " km/h".to_string(),
                    }],
                },
                LaneSpec {
                    title: "Ride Heights & Rake".to_string(),
                    y_min: 45.0,
                    y_max: 75.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![
                        TraceSpec {
                            name: "Front Height".to_string(),
                            cache: CacheSelector::FrontHeight, custom_channel: None,
                            color: theme.throttle,
                            width: 2.2,
                            unit: " mm".to_string(),
                        },
                        TraceSpec {
                            name: "Rear Height".to_string(),
                            cache: CacheSelector::RearHeight, custom_channel: None,
                            color: theme.rpm,
                            width: 2.2,
                            unit: " mm".to_string(),
                        },
                        TraceSpec {
                            name: "Dynamic Rake".to_string(),
                            cache: CacheSelector::Rake, custom_channel: None,
                            color: theme.steering,
                            width: 2.2,
                            unit: " mm".to_string(),
                        },
                    ],
                },
                LaneSpec {
                    title: "Accelerations".to_string(),
                    y_min: 10.0,
                    y_max: 40.0,
                    scaling: LaneScaling::Mono,
                    traces: vec![
                        TraceSpec {
                            name: "Lateral G".to_string(),
                            cache: CacheSelector::LatG, custom_channel: None,
                            color: theme.brake,
                            width: 2.2,
                            unit: " G".to_string(),
                        },
                        TraceSpec {
                            name: "Longitudinal G".to_string(),
                            cache: CacheSelector::LongG, custom_channel: None,
                            color: theme.clutch,
                            width: 2.2,
                            unit: " G".to_string(),
                        },
                    ],
                },
            ],
        }
    }

    pub fn tyre(theme: AppTheme) -> Self {
        let lane = |title: &str,
                    range: (f64, f64),
                    names: [&'static str; 3],
                    selectors,
                    colors: [egui::Color32; 3]| {
            LaneSpec {
                title: title.to_string(),
                y_min: range.0,
                y_max: range.1,
                scaling: LaneScaling::Mono,
                traces: names
                    .into_iter()
                    .zip(selectors)
                    .zip(colors)
                    .map(|((position, cache), color)| TraceSpec {
                        name: position.to_string(),
                        cache: cache, custom_channel: None,
                        color,
                        width: 2.0,
                        unit: " °C".to_string(),
                    })
                    .collect(),
            }
        };

        Self {
            lanes: vec![
                lane(
                    "Left Front Tyre",
                    STACKED_LANE_RANGES[0],
                    ["LF Center", "LF Inner", "LF Outer"],
                    TYRE_CHANNEL_GROUPS[0],
                    theme.tyre_lf,
                ),
                lane(
                    "Right Front Tyre",
                    STACKED_LANE_RANGES[1],
                    ["RF Center", "RF Inner", "RF Outer"],
                    TYRE_CHANNEL_GROUPS[1],
                    theme.tyre_rf,
                ),
                lane(
                    "Left Rear Tyre",
                    STACKED_LANE_RANGES[2],
                    ["LR Center", "LR Inner", "LR Outer"],
                    TYRE_CHANNEL_GROUPS[2],
                    theme.tyre_lr,
                ),
                lane(
                    "Right Rear Tyre",
                    STACKED_LANE_RANGES[3],
                    ["RR Center", "RR Inner", "RR Outer"],
                    TYRE_CHANNEL_GROUPS[3],
                    theme.tyre_rr,
                ),
            ],
        }
    }

    pub fn shocks(theme: AppTheme) -> Self {
        let corners = [
            (
                "Left Front Shock",
                "LF Deflection",
                CacheSelector::LfShockDeflection,
            ),
            (
                "Right Front Shock",
                "RF Deflection",
                CacheSelector::RfShockDeflection,
            ),
            (
                "Left Rear Shock",
                "LR Deflection",
                CacheSelector::LrShockDeflection,
            ),
            (
                "Right Rear Shock",
                "RR Deflection",
                CacheSelector::RrShockDeflection,
            ),
        ];
        Self {
            lanes: corners
                .into_iter()
                .zip(STACKED_LANE_RANGES)
                .zip(theme.shock_corners)
                .map(|(((title, name, cache), range), color)| LaneSpec {
                    title: title.to_string(),
                    y_min: range.0,
                    y_max: range.1,
                    scaling: LaneScaling::Mono,
                    traces: vec![TraceSpec {
                        name: name.to_string(),
                        cache, custom_channel: None,
                        color,
                        width: 2.2,
                        unit: " mm".to_string(),
                    }],
                })
                .collect(),
        }
    }
}
use crate::config::theme::AppTheme;

#[cfg(test)]
mod tests {
    use super::{CacheSelector, WorksheetConfig};
    use crate::config::theme::AppTheme;

    #[test]
    fn tyre_worksheet_has_four_three_trace_corner_groups() {
        let worksheet = WorksheetConfig::tyre(AppTheme::for_mode(true));

        assert_eq!(worksheet.lanes.len(), 4);
        assert!(worksheet.lanes.iter().all(|lane| lane.traces.len() == 3));
    }

    #[test]
    fn tyre_inner_and_outer_channels_follow_vehicle_side() {
        assert_eq!(
            CacheSelector::LfTempInner.telemetry_source(),
            Some(("LFtempR", 1.0))
        );
        assert_eq!(
            CacheSelector::RfTempInner.telemetry_source(),
            Some(("RFtempL", 1.0))
        );
    }

    #[test]
    fn shocks_worksheet_has_one_group_per_corner() {
        let worksheet = WorksheetConfig::shocks(AppTheme::for_mode(false));

        assert_eq!(worksheet.lanes.len(), 4);
        assert!(worksheet.lanes.iter().all(|lane| lane.traces.len() == 1));
    }
}
