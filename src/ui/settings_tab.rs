use crate::OpenDavApp;
use egui::Color32;

impl OpenDavApp {
    pub fn draw_settings_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let text_color = if is_dark { Color32::WHITE } else { Color32::BLACK };
        
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(egui::RichText::new("Application Settings").color(text_color).size(32.0));
            ui.add_space(30.0);
        });

        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.set_width(ui.available_width() - 20.0);
                    ui.add_space(10.0);
                    ui.heading(egui::RichText::new("Algorithm Tuning").color(text_color).size(24.0));
                    ui.add_space(15.0);
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Corner Merge Gap Threshold (meters):").color(text_color).size(16.0));
                        ui.add_space(10.0);
                        
                        let drag = egui::DragValue::new(&mut self.settings.corner_merge_threshold)
                            .speed(0.5)
                            .clamp_range(5.0..=100.0)
                            .suffix(" m");
                        
                        let response = ui.add(drag);
                        
                        if response.changed() {
                            let threshold = self.settings.corner_merge_threshold;
                            for session in &mut self.sessions {
                                session.recalculate_sectors(threshold);
                            }
                            self.settings.save();
                        }
                    });
                    
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Adjust this to control how aggressively tight chicanes are merged into single corners.").color(Color32::GRAY).size(14.0));
                    ui.add_space(15.0);
                    
                    ui.horizontal(|ui| {
                        let mut use_metric = self.settings.use_metric;
                        let metric_resp = ui.checkbox(&mut use_metric, egui::RichText::new("Use Metric Units (km/h, mm, kg)").color(text_color).size(16.0));
                        
                        if metric_resp.changed() {
                        self.settings.use_metric = use_metric;
                        self.settings.save();
                    }
                });
                ui.add_space(5.0);
                ui.label(egui::RichText::new("If disabled, displays Imperial Units (mph, in, lbs).").color(Color32::GRAY).size(14.0));
                
                ui.add_space(20.0);
                ui.heading(egui::RichText::new("Map Integration").color(text_color).size(24.0));
                ui.add_space(15.0);
                
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Mapbox API Key (Optional):").color(text_color).size(16.0));
                    ui.add_space(10.0);
                    
                    let mut api_key = self.settings.mapbox_api_key.clone();
                    let response = ui.add(egui::TextEdit::singleline(&mut api_key).password(true).desired_width(300.0));
                    
                    if response.changed() {
                        self.settings.mapbox_api_key = api_key;
                        self.settings.save();
                    }
                });
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Leaves Google Maps and enables ultra high-res Mapbox satellite imagery for custom tracks. ").color(Color32::GRAY).size(14.0));
                    ui.hyperlink_to("Get a free key here.", "https://account.mapbox.com/auth/signup/");
                });
                
                ui.add_space(30.0);
                
                if ui.button(egui::RichText::new("Save Settings").color(text_color).size(16.0)).clicked() {
                        let threshold = self.settings.corner_merge_threshold;
                        for session in &mut self.sessions {
                            session.recalculate_sectors(threshold);
                        }
                        self.settings.save();
                    }
                    ui.add_space(10.0);
                });
            });
        });
    }
}
