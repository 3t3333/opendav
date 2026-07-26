use crate::config::theme::AppTheme;

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
        let is_dark = ctx.global_style().visuals.dark_mode;
        let theme = AppTheme::for_mode(is_dark);

        egui::Window::new("🛠 Engine Diagnostics")
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 48.0])
            .resizable(false)
            .collapsible(true)
            .title_bar(true)
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(theme.surface_card.gamma_multiply(0.92))
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(6.0),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let fps_color = if metrics.fps >= 55.0 {
                        theme.success
                    } else if metrics.fps >= 30.0 {
                        theme.warning
                    } else {
                        theme.danger
                    };
                    ui.label(
                        egui::RichText::new(format!("{:.1} FPS", metrics.fps))
                            .color(fps_color)
                            .strong()
                            .size(15.0),
                    );
                    ui.label(
                        egui::RichText::new(format!("({:.2} ms)", metrics.frame_time_ms))
                            .color(theme.text_secondary)
                            .size(12.0),
                    );
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(
                    egui::RichText::new(format!("Graph Render: {:.2} ms", metrics.graph_render_time_ms))
                        .color(theme.text_primary)
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("Points Rendered: {}", metrics.points_rendered))
                        .color(theme.text_primary)
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("Points Culled: {}", metrics.points_culled))
                        .color(theme.text_tertiary)
                        .small(),
                );
                
                let ratio = if metrics.frame_time_ms > 0.0 {
                    (metrics.points_rendered as f32 / metrics.frame_time_ms).round() as u32
                } else {
                    0
                };
                ui.label(
                    egui::RichText::new(format!("Efficiency: {} pts/ms", ratio))
                        .color(theme.accent_text)
                        .small()
                        .strong(),
                );
                
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
                        .color(theme.text_tertiary)
                        .small(),
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
