#[derive(Clone, Debug, Default)]
pub struct DebugMetrics {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub graph_render_time_ms: f32,
    pub points_rendered: usize,
    pub points_culled: usize,
    pub show_overlay: bool,
    pub show_simulator: bool,
    pub history_dt: Vec<f32>,
}

pub fn draw_overlay(ctx: &egui::Context, metrics: &mut DebugMetrics) {
    if metrics.show_overlay {
        egui::Window::new("Engine Diagnostics")
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .frame(egui::Frame::window(&ctx.global_style()).fill(egui::Color32::from_black_alpha(200)))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.1} FPS | {:.2} ms", metrics.fps, metrics.frame_time_ms))
                        .color(egui::Color32::YELLOW)
                        .strong()
                        .size(16.0)
                );
                ui.separator();
                ui.label(format!("Graph Render Time: {:.2} ms", metrics.graph_render_time_ms));
                ui.label(format!("Points Rendered: {}", metrics.points_rendered));
                ui.label(format!("Points Culled: {}", metrics.points_culled));
                
                let ratio = if metrics.frame_time_ms > 0.0 {
                    (metrics.points_rendered as f32 / metrics.frame_time_ms).round() as u32
                } else {
                    0
                };
                ui.label(format!("Efficiency: {} pts/ms", ratio));
                
                ui.add_space(4.0);
                let avg_dt = if !metrics.history_dt.is_empty() {
                    metrics.history_dt.iter().sum::<f32>() / metrics.history_dt.len() as f32
                } else {
                    0.0
                };
                let avg_fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
                let avg_ft = avg_dt * 1000.0;
                
                ui.label(
                    egui::RichText::new(format!("Avg (25f): {:.1} FPS | {:.2} ms", avg_fps, avg_ft))
                        .color(egui::Color32::from_rgb(200, 200, 200))
                        .size(12.0)
                );
            });
    }

    if metrics.show_simulator {
        egui::Window::new("Synthetic Benchmarker (Payload Injector)")
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.label("Configure test payload injection parameters here.");
                ui.add_space(8.0);
                ui.label("Work in progress...");
                if ui.button("Close").clicked() {
                    metrics.show_simulator = false;
                }
            });
    }
}
