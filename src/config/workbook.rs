use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Workbook {
    pub name: String,
    pub worksheets: Vec<Worksheet>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Worksheet {
    pub name: String,
    pub tree: egui_tiles::Tree<Pane>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Pane {
    TimeSeries {
        id: String,
        config: crate::config::worksheet::WorksheetConfig,
    },
}


