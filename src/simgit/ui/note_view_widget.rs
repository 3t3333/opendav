use crate::config::theme::AppTheme;
use crate::simgit::data::note_math::{sample_channel_at_distance, sample_channel_at_time, LiveInputSample};
use crate::simgit::data::repository::AnalysisNote;
use crate::simgit::ui::input_gauges::{draw_input_gauge_card, SteeringWheelSvgCache};
use egui::Ui;

/// Context state passed into the dedicated Note Viewing Sidebar Widget
pub struct NoteViewContext<'a> {
    pub note: &'a AnalysisNote,
    pub is_dark: bool,
    pub cursor_x: f64,
    pub primary_session: Option<&'a crate::LoadedSession>,
    pub cyan_comparison: Option<&'a crate::signals::comparison::ComparisonCache>,
    pub secondary_comparison: Option<&'a crate::signals::comparison::ComparisonCache>,
    pub wheel_cache: &'a mut SteeringWheelSvgCache,
}

/// Renders the dedicated sidebar widget for viewing an active SimGit note
/// Returns `true` if the user clicked the "Back to Notes List" button.
pub fn draw_dedicated_note_view_widget(ui: &mut Ui, ctx: NoteViewContext<'_>) -> bool {
    let theme = AppTheme::for_mode(ctx.is_dark);
    let mut back_clicked = false;

    egui::ScrollArea::vertical()
        .id_salt("dedicated_note_view_sidebar")
        .show(ui, |ui| {
            // 1. Navigation Top Bar
            ui.horizontal(|ui| {
                if ui
                    .button(
                        egui::RichText::new("← All Notes")
                            .strong()
                            .color(theme.accent),
                    )
                    .clicked()
                {
                    back_clicked = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (tag_rect, _) =
                        ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        tag_rect,
                        4.0,
                        ctx.note.color.display_color(ctx.is_dark),
                    );
                    ui.label(
                        egui::RichText::new("VIEWING NOTE")
                            .strong()
                            .small()
                            .color(theme.accent_text),
                    );
                });
            });
            ui.add_space(8.0);

            // 2. Objective Header (Driving Goal Change)
            ui.heading(
                egui::RichText::new(ctx.note.display_objective())
                    .strong()
                    .size(19.0)
                    .color(theme.text_primary),
            );
            ui.add_space(10.0);

            // 3. Full Explanation Made by the Coach
            egui::Frame::NONE
                .fill(theme.surface_card)
                .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("COACH ANALYSIS & INSTRUCTION")
                            .strong()
                            .small()
                            .color(theme.text_tertiary),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&ctx.note.body)
                                .size(14.0)
                                .color(theme.text_primary),
                        )
                        .wrap(),
                    );
                    ui.add_space(8.0);

                    // Note Context details
                    let lap_str = ctx
                        .note
                        .context
                        .lap_number
                        .map(|l| format!("Lap {l}"))
                        .unwrap_or_else(|| "All Laps".to_owned());
                    ui.label(
                        egui::RichText::new(format!(
                            "Author: {}  •  {}  •  Worksheet: {}",
                            ctx.note.author, lap_str, ctx.note.context.worksheet
                        ))
                        .small()
                        .color(theme.text_secondary),
                    );
                });

            ui.add_space(14.0);

            // 4. Section Input Gauges Header
            ui.heading(
                egui::RichText::new("Section Driver Input Gauges")
                    .strong()
                    .size(16.0)
                    .color(theme.text_primary),
            );
            ui.label(
                egui::RichText::new("Scrub or playback telemetry to view real-time pedal and steering inputs.")
                    .small()
                    .color(theme.text_secondary),
            );
            ui.add_space(8.0);

            // Calculate live input samples for primary and reference laps
            let primary_sample = extract_live_sample(ctx.primary_session, ctx.cursor_x);

            // Determine reference session if available
            let ref_comparison = ctx.cyan_comparison.or(ctx.secondary_comparison);
            let ref_sample = ref_comparison.map(|comp| extract_comparison_live_sample(Some(comp), ctx.cursor_x));

            // Section time delta value to show above the baseline input overlay
            let section_delta = ctx.note.context.section_delta.or_else(|| {
                ctx.primary_session.and_then(|sess| {
                    let (start_t, end_t) = ctx.note.context.viewport?;
                    crate::simgit::data::note_math::calculate_section_delta_from_cache(
                        &sess.time_delta_pts_cache,
                        start_t,
                        end_t,
                    )
                })
            });

            // 5. Baseline Input Gauge Card
            let baseline_lap_text = ctx
                .note
                .context
                .lap_number
                .map(|l| format!("Lap {l}"))
                .unwrap_or_else(|| "Active Lap".to_owned());
            draw_input_gauge_card(
                ui,
                &theme,
                "BASELINE TELEMETRY",
                Some(&baseline_lap_text),
                &primary_sample,
                ctx.wheel_cache,
                section_delta.map(|d| (d, ctx.is_dark)),
            );

            // 6. Reference Input Gauge Card (if reference lap is loaded)
            if let Some(ref_sample) = ref_sample {
                ui.add_space(10.0);
                let ref_label = if ctx.cyan_comparison.is_some() {
                    "CYAN REFERENCE"
                } else {
                    "SECONDARY REFERENCE"
                };
                draw_input_gauge_card(
                    ui,
                    &theme,
                    ref_label,
                    Some("Reference Lap"),
                    &ref_sample,
                    ctx.wheel_cache,
                    None,
                );
            }
        });

    back_clicked
}

/// Helper function to extract live driver input sample (Throttle, Brake, Steering Angle)
/// from a LoadedSession at `cursor_time`.
fn extract_live_sample(session: Option<&crate::LoadedSession>, cursor_time: f64) -> LiveInputSample {
    let Some(sess) = session else {
        return LiveInputSample::default();
    };

    let throttle = sample_channel_at_time(&sess.throttle_raw_pts_cache, cursor_time);
    let brake = sample_channel_at_time(&sess.brake_raw_pts_cache, cursor_time);
    let steering_deg = -sample_channel_at_time(&sess.steering_raw_pts_cache, cursor_time);
    let time_delta = if !sess.time_delta_pts_cache.is_empty() {
        Some(sample_channel_at_time(&sess.time_delta_pts_cache, cursor_time))
    } else {
        None
    };

    LiveInputSample::new(throttle, brake, steering_deg, time_delta)
}

/// Helper function to extract live driver input sample from an aligned ComparisonCache.
fn extract_comparison_live_sample(
    comparison: Option<&crate::signals::comparison::ComparisonCache>,
    cursor_time: f64,
) -> LiveInputSample {
    let Some(comp) = comparison else {
        return LiveInputSample::default();
    };

    let throttle = comp
        .channel(crate::config::worksheet::CacheSelector::Throttle)
        .and_then(|c| c.raw_value_at(cursor_time))
        .unwrap_or(0.0);
        
    let brake = comp
        .channel(crate::config::worksheet::CacheSelector::Brake)
        .and_then(|c| c.raw_value_at(cursor_time))
        .unwrap_or(0.0);
        
    let steering_deg = -comp
        .channel(crate::config::worksheet::CacheSelector::Steering)
        .and_then(|c| c.raw_value_at(cursor_time))
        .unwrap_or(0.0);

    LiveInputSample::new(throttle, brake, steering_deg, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_live_sample_handles_none_session() {
        let sample = extract_live_sample(None, 10.0);
        assert_eq!(sample, LiveInputSample::default());
    }
}
