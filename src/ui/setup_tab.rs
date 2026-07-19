use crate::OpenDavApp;
use crate::config::worksheet::{ACCENT_COLOR, SUB_ACCENT_COLOR};

impl OpenDavApp {
    pub fn draw_setup_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        if !self.session_loaded || self.sessions.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Awaiting Telemetry Stream").heading().color(SUB_ACCENT_COLOR));
                    ui.label(egui::RichText::new("Please load an iRacing .ibt file to view setups.").color(egui::Color32::GRAY));
                });
            });
            return;
        }

        let p_idx = self.primary_session_idx;
        let setup_data_opt = self.sessions[p_idx].setup.clone();

        ui.heading(egui::RichText::new("Racecar Setup").strong().color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
        ui.add_space(8.0);

        if let Some(setup) = setup_data_opt {
            let mut is_visualizer = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("setup_view_mode"), || true));
            
            ui.horizontal(|ui| {
                ui.selectable_value(&mut is_visualizer, true, "🏎 2D Visualizer");
                ui.selectable_value(&mut is_visualizer, false, "📋 Raw Data Sheet");
            });
            
            ui.add_space(15.0);
            
            if is_visualizer {
                self.draw_setup_visualizer(ui, &setup, is_dark);
            } else {
                self.draw_setup_raw_sheet(ui, &setup, is_dark);
            }
            
            ui.data_mut(|d| d.insert_temp(egui::Id::new("setup_view_mode"), is_visualizer));
        } else {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("No Setup Data Found").heading().color(egui::Color32::LIGHT_RED));
                    ui.label(egui::RichText::new("The loaded telemetry file does not contain embedded setup data.").color(egui::Color32::GRAY));
                });
            });
        }
    }

    fn draw_setup_raw_sheet(&self, ui: &mut egui::Ui, setup: &crate::simgit::setup_parser::SetupData, is_dark: bool) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("setup_raw_sheet_grid")
                .striped(true)
                .min_col_width(200.0)
                .spacing([40.0, 10.0])
                .show(ui, |ui| {
                    let mut sorted_keys: Vec<_> = setup.parameters.keys().collect();
                    sorted_keys.sort();
                    
                    for key in sorted_keys {
                        let value = setup.parameters.get(key).unwrap();
                        ui.label(egui::RichText::new(key).strong().color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new(value).color(if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }));
                        ui.end_row();
                    }
                });
        });
    }

    fn draw_setup_visualizer(&self, ui: &mut egui::Ui, setup: &crate::simgit::setup_parser::SetupData, is_dark: bool) {
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(800.0, 600.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Drawing Constants
        let center = rect.center();
        let car_width = 160.0;
        let car_length = 380.0;
        let wheel_width = 40.0;
        let wheel_length = 80.0;
        let track_width = car_width + wheel_width * 1.5;
        let wheelbase = car_length * 0.65;
        
        let chassis_color = if is_dark { egui::Color32::from_rgb(60, 60, 60) } else { egui::Color32::from_rgb(200, 200, 200) };
        let wheel_color = if is_dark { egui::Color32::from_rgb(25, 25, 25) } else { egui::Color32::from_rgb(40, 40, 40) };
        let accent_stroke = egui::Stroke::new(2.0, ACCENT_COLOR);

        // 1. Draw Wheels
        let fl_pos = center + egui::vec2(-track_width / 2.0, -wheelbase / 2.0);
        let fr_pos = center + egui::vec2(track_width / 2.0, -wheelbase / 2.0);
        let rl_pos = center + egui::vec2(-track_width / 2.0, wheelbase / 2.0);
        let rr_pos = center + egui::vec2(track_width / 2.0, wheelbase / 2.0);

        let draw_wheel = |pos: egui::Pos2| {
            painter.rect_filled(
                egui::Rect::from_center_size(pos, egui::vec2(wheel_width, wheel_length)),
                4.0,
                wheel_color,
            );
        };
        
        draw_wheel(fl_pos);
        draw_wheel(fr_pos);
        draw_wheel(rl_pos);
        draw_wheel(rr_pos);

        // 2. Draw Axles
        painter.line_segment([fl_pos, fr_pos], egui::Stroke::new(6.0, chassis_color));
        painter.line_segment([rl_pos, rr_pos], egui::Stroke::new(6.0, chassis_color));

        // 3. Draw Generic Chassis (Procedural Bird's Eye Racing Car)
        // Main body
        let chassis_rect = egui::Rect::from_center_size(center, egui::vec2(car_width, car_length));
        painter.rect(chassis_rect, 20.0, chassis_color, accent_stroke, egui::StrokeKind::Inside);
        
        // Cockpit
        let cockpit_rect = egui::Rect::from_center_size(center + egui::vec2(0.0, 10.0), egui::vec2(car_width * 0.7, car_length * 0.35));
        painter.rect_filled(cockpit_rect, 15.0, if is_dark { egui::Color32::BLACK } else { egui::Color32::DARK_GRAY });

        // Front Splitter
        let splitter_rect = egui::Rect::from_center_size(center + egui::vec2(0.0, -car_length / 2.0), egui::vec2(car_width * 1.1, 15.0));
        painter.rect_filled(splitter_rect, 2.0, wheel_color);

        // Rear Wing
        let wing_rect = egui::Rect::from_center_size(center + egui::vec2(0.0, car_length / 2.0 - 10.0), egui::vec2(car_width * 1.2, 25.0));
        painter.rect_filled(wing_rect, 4.0, wheel_color);

        // 4. Draw Labels and Setup Values
        let draw_param = |x: f32, y: f32, title: &str, key: &str, align: egui::Align| {
            let val = setup.parameters.get(key).map(|s| s.as_str()).unwrap_or("--");
            let pos = center + egui::vec2(x, y);
            
            let align2 = match align {
                egui::Align::Min => egui::Align2::LEFT_CENTER,
                egui::Align::Max => egui::Align2::RIGHT_CENTER,
                egui::Align::Center => egui::Align2::CENTER_CENTER,
            };
            
            painter.text(
                pos,
                align2,
                format!("{}: {}", title, val),
                egui::FontId::proportional(13.0),
                if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK },
            );
        };

        // Front Left
        draw_param(-track_width / 2.0 - 55.0, -wheelbase / 2.0 - 20.0, "Pressure", "LFcoldPressure", egui::Align::Max);
        draw_param(-track_width / 2.0 - 55.0, -wheelbase / 2.0, "Camber", "LFcamber", egui::Align::Max);
        draw_param(-track_width / 2.0 - 55.0, -wheelbase / 2.0 + 20.0, "Ride H", "LFrideHeight", egui::Align::Max);
        draw_param(-track_width / 2.0 - 55.0, -wheelbase / 2.0 + 40.0, "Perch", "LFspringPerch", egui::Align::Max);

        // Front Right
        draw_param(track_width / 2.0 + 55.0, -wheelbase / 2.0 - 20.0, "Pressure", "RFcoldPressure", egui::Align::Min);
        draw_param(track_width / 2.0 + 55.0, -wheelbase / 2.0, "Camber", "RFcamber", egui::Align::Min);
        draw_param(track_width / 2.0 + 55.0, -wheelbase / 2.0 + 20.0, "Ride H", "RFrideHeight", egui::Align::Min);
        draw_param(track_width / 2.0 + 55.0, -wheelbase / 2.0 + 40.0, "Perch", "RFspringPerch", egui::Align::Min);

        // Rear Left
        draw_param(-track_width / 2.0 - 55.0, wheelbase / 2.0 - 20.0, "Pressure", "LRcoldPressure", egui::Align::Max);
        draw_param(-track_width / 2.0 - 55.0, wheelbase / 2.0, "Camber", "LRcamber", egui::Align::Max);
        draw_param(-track_width / 2.0 - 55.0, wheelbase / 2.0 + 20.0, "Ride H", "LRrideHeight", egui::Align::Max);
        draw_param(-track_width / 2.0 - 55.0, wheelbase / 2.0 + 40.0, "Perch", "LRspringPerch", egui::Align::Max);

        // Rear Right
        draw_param(track_width / 2.0 + 55.0, wheelbase / 2.0 - 20.0, "Pressure", "RRcoldPressure", egui::Align::Min);
        draw_param(track_width / 2.0 + 55.0, wheelbase / 2.0, "Camber", "RRcamber", egui::Align::Min);
        draw_param(track_width / 2.0 + 55.0, wheelbase / 2.0 + 20.0, "Ride H", "RRrideHeight", egui::Align::Min);
        draw_param(track_width / 2.0 + 55.0, wheelbase / 2.0 + 40.0, "Perch", "RRspringPerch", egui::Align::Min);

        // Center Components
        draw_param(0.0, -wheelbase / 2.0 + 20.0, "Front ARB", "ArbSettingF", egui::Align::Center);
        draw_param(0.0, -30.0, "Brake Bias", "BrakePressureBias", egui::Align::Center);
        draw_param(0.0, wheelbase / 2.0 - 20.0, "Rear ARB", "ArbSettingR", egui::Align::Center);
        draw_param(0.0, car_length / 2.0 + 25.0, "Wing Angle", "WingAngle", egui::Align::Center);
    }
}
