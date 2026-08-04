use crate::config::theme::AppTheme;
use crate::simgit::data::note_math::LiveInputSample;
use egui::{Color32, Pos2, Rect, Vec2};
use usvg::{Options, Tree};

/// Cache manager for the Steering Wheel SVG texture
#[derive(Default)]
pub struct SteeringWheelSvgCache {
    texture: Option<egui::TextureHandle>,
}

impl SteeringWheelSvgCache {
    pub fn get_or_load(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        if let Some(ref tex) = self.texture {
            return Some(tex.clone());
        }

        // Try primary path then fallback path
        let paths = [
            "assets/wheel_custom.svg",
            r"C:\Users\bukar\opendav_overlay_poc\assets\wheel_custom.svg",
        ];

        let mut svg_bytes = None;
        for path in &paths {
            if let Ok(bytes) = std::fs::read(path) {
                svg_bytes = Some(bytes);
                break;
            }
        }

        let bytes = svg_bytes?;
        let opt = Options::default();
        let tree = Tree::from_data(&bytes, &opt).ok()?;

        let size = 160;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
        let scale = size as f32 / tree.size().width();
        resvg::render(
            &tree,
            usvg::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [size as usize, size as usize],
            pixmap.data(),
        );

        let handle = ctx.load_texture(
            "wheel_custom_svg",
            color_image,
            egui::TextureOptions::LINEAR,
        );
        self.texture = Some(handle.clone());
        Some(handle)
    }
}

/// Renders a complete driver input gauge overlay card (Vertical Pedals: Left=Brake, Right=Throttle; Vector Steering Wheel)
pub fn draw_input_gauge_card(
    ui: &mut egui::Ui,
    theme: &AppTheme,
    title: &str,
    subtitle: Option<&str>,
    sample: &LiveInputSample,
    wheel_cache: &mut SteeringWheelSvgCache,
    top_delta: Option<(f64, bool)>, // (delta, is_dark)
) {
    egui::Frame::NONE
        .fill(theme.surface_card)
        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            // Optional Top Delta Badge
            if let Some((delta, is_dark)) = top_delta {
                let delta_str = crate::simgit::data::note_math::format_section_delta(delta);
                let delta_clr = crate::simgit::data::note_math::section_delta_color(delta, is_dark);
                let bg_clr = delta_clr.gamma_multiply(0.18);

                ui.vertical_centered(|ui| {
                    egui::Frame::NONE
                        .fill(bg_clr)
                        .stroke(egui::Stroke::new(1.0, delta_clr))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("TIME DELTA: {delta_str}"))
                                    .strong()
                                    .size(13.0)
                                    .color(delta_clr),
                            );
                        });
                });
                ui.add_space(8.0);
            }

            // Title & Subtitle
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .size(14.0)
                        .color(theme.text_primary),
                );
                if let Some(sub) = subtitle {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(sub)
                                .small()
                                .color(theme.text_tertiary),
                        );
                    });
                }
            });
            ui.add_space(8.0);

            // Gauge Layout: Vertical Pedal Box (Brake Left, Throttle Right) + Vector Steering Wheel
            ui.horizontal(|ui| {
                // Vertical Pedal Box: BRAKE (Left), THROTTLE (Right)
                ui.vertical(|ui| {
                    ui.set_width(120.0);
                    draw_vertical_pedal_box(ui, sample.brake, sample.throttle, theme);
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Vector Steering Wheel Overlay
                ui.vertical_centered(|ui| {
                    draw_vector_steering_wheel(
                        ui,
                        theme,
                        sample.steering_angle_rad as f32,
                        sample.steering_angle_deg,
                        wheel_cache,
                    );
                });
            });
        });
}

/// Renders a vertical pedal box with Brake on the Left (Red) and Throttle on the Right (Green)
fn draw_vertical_pedal_box(
    ui: &mut egui::Ui,
    brake_pct: f64,
    throttle_pct: f64,
    theme: &AppTheme,
) {
    let clamp_brk = brake_pct.clamp(0.0, 100.0);
    let clamp_thr = throttle_pct.clamp(0.0, 100.0);

    let pedal_w = 26.0;
    let pedal_h = 76.0;

    // Header labels above pedals
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(pedal_w + 10.0, 16.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("{:.0}%", clamp_brk))
                        .strong()
                        .small()
                        .color(theme.brake),
                );
            });
        });
        ui.add_space(8.0);
        ui.allocate_ui(Vec2::new(pedal_w + 10.0, 16.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("{:.0}%", clamp_thr))
                        .strong()
                        .small()
                        .color(theme.throttle),
                );
            });
        });
    });

    ui.add_space(2.0);

    // Vertical Bar Rectangles
    ui.horizontal(|ui| {
        // BRAKE (LEFT)
        draw_single_vertical_pedal(ui, clamp_brk, theme.brake, theme, Vec2::new(pedal_w, pedal_h));
        ui.add_space(18.0);
        // THROTTLE (RIGHT)
        draw_single_vertical_pedal(ui, clamp_thr, theme.throttle, theme, Vec2::new(pedal_w, pedal_h));
    });

    ui.add_space(4.0);

    // Footer labels under pedals
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(pedal_w + 10.0, 14.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("BRAKE")
                        .strong()
                        .small()
                        .color(theme.text_secondary),
                );
            });
        });
        ui.add_space(8.0);
        ui.allocate_ui(Vec2::new(pedal_w + 10.0, 14.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("THROTTLE")
                        .strong()
                        .small()
                        .color(theme.text_secondary),
                );
            });
        });
    });
}

/// Renders a single vertical pedal bar filling upward from bottom
fn draw_single_vertical_pedal(
    ui: &mut egui::Ui,
    val_pct: f64,
    fill_color: Color32,
    theme: &AppTheme,
    size: Vec2,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    // Background track frame
    ui.painter().rect_filled(rect, 4.0, theme.surface_elevated);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, theme.border_subtle), egui::StrokeKind::Inside);

    // Vertical fill from bottom upward
    let fill_fraction = (val_pct / 100.0).clamp(0.0, 1.0) as f32;
    if fill_fraction > 0.005 {
        let fill_height = rect.height() * fill_fraction;
        let fill_rect = Rect::from_min_max(
            Pos2::new(rect.min.x, rect.max.y - fill_height),
            rect.max,
        );
        ui.painter().rect_filled(fill_rect, 4.0, fill_color);
    }
}

/// Renders the vector steering wheel graphic, rotated accurately by steering angle
fn draw_vector_steering_wheel(
    ui: &mut egui::Ui,
    theme: &AppTheme,
    angle_rad: f32,
    angle_deg: f64,
    wheel_cache: &mut SteeringWheelSvgCache,
) {
    ui.label(
        egui::RichText::new("STEERING")
            .strong()
            .small()
            .color(theme.text_secondary),
    );

    let wheel_size = Vec2::splat(72.0);
    let (rect, _) = ui.allocate_exact_size(wheel_size, egui::Sense::hover());
    let center = rect.center();
    let radius = rect.width() / 2.0 - 4.0;

    // Draw rotated vector steering wheel
    if let Some(tex) = wheel_cache.get_or_load(ui.ctx()) {
        let rot = egui::emath::Rot2::from_angle(angle_rad);
        let corners = [
            center + rot * Vec2::new(-radius, -radius),
            center + rot * Vec2::new(radius, -radius),
            center + rot * Vec2::new(radius, radius),
            center + rot * Vec2::new(-radius, radius),
        ];

        let uvs = [
            Pos2::new(0.0, 0.0),
            Pos2::new(1.0, 0.0),
            Pos2::new(1.0, 1.0),
            Pos2::new(0.0, 1.0),
        ];

        let mut mesh = egui::Mesh::with_texture(tex.id());
        for i in 0..4 {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: corners[i],
                uv: uvs[i],
                color: Color32::WHITE,
            });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);

        ui.painter().add(mesh);
    } else {
        // Fallback: Vector geometry matching wheel_custom.svg
        let rim_color = Color32::from_rgb(192, 192, 192);
        let marker_color = Color32::from_rgb(255, 255, 0);

        ui.painter().circle_stroke(center, radius, egui::Stroke::new(5.0, rim_color));

        let rot = egui::emath::Rot2::from_angle(angle_rad);

        // Horizontal spoke
        let left_spoke = center + rot * Vec2::new(-radius + 3.0, 0.0);
        let right_spoke = center + rot * Vec2::new(radius - 3.0, 0.0);
        ui.painter().line_segment([left_spoke, right_spoke], egui::Stroke::new(5.0, rim_color));

        // Vertical drop spoke
        let bottom_spoke = center + rot * Vec2::new(0.0, radius - 3.0);
        ui.painter().line_segment([center, bottom_spoke], egui::Stroke::new(5.0, rim_color));

        // Center Yellow Marker at Top
        let marker_start = center + rot * Vec2::new(0.0, -radius - 1.0);
        let marker_end = center + rot * Vec2::new(0.0, -radius + 7.0);
        ui.painter().line_segment([marker_start, marker_end], egui::Stroke::new(5.0, marker_color));
    }

    ui.add_space(4.0);
    let dir_str = if angle_deg > 0.5 {
        "R"
    } else if angle_deg < -0.5 {
        "L"
    } else {
        ""
    };
    ui.label(
        egui::RichText::new(format!("{:.0}° {}", angle_deg.abs(), dir_str))
            .strong()
            .small()
            .color(theme.text_primary),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steering_wheel_cache_initializes_empty() {
        let cache = SteeringWheelSvgCache::default();
        assert!(cache.texture.is_none());
    }

    #[test]
    fn vertical_pedal_fill_height_calculation() {
        let size = Vec2::new(26.0, 80.0);
        let fill_pct_100 = 100.0 / 100.0;
        let fill_pct_50 = 50.0 / 100.0;

        assert_eq!(size.y * fill_pct_100 as f32, 80.0);
        assert_eq!(size.y * fill_pct_50 as f32, 40.0);
    }
}
