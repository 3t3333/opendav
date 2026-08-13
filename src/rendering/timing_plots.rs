use crate::config::theme::AppTheme;
use crate::OpenDavApp;
use eframe::egui;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoint, PlotPoints, Text};

impl OpenDavApp {
    pub fn draw_timing_graphs_page(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let theme = AppTheme::for_mode(is_dark);

        if !self.session_loaded || self.sessions.is_empty() {
            return;
        }

        let loaded = &self.sessions[self.primary_session_idx];
        if loaded.sectors.is_empty() || loaded.lap_data_cache.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No sector data available.").color(theme.text_tertiary),
                );
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.filter_large_sectors,
                "Filter large sectors (anomalies)",
            );
        });
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for (i, sector) in loaded.sectors.iter().enumerate() {
                        egui::Frame::NONE
                            .fill(theme.surface_card)
                            .stroke(egui::Stroke::new(1.0, theme.border_strong))
                            .corner_radius(8.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.heading(
                                    egui::RichText::new(&sector.name)
                                        .strong()
                                        .color(theme.accent_text),
                                );
                                ui.add_space(5.0);

                                let mut sector_times = Vec::new();
                                let mut optimal_time = f64::MAX;

                                for lap in &loaded.lap_data_cache {
                                    if lap.dist.is_empty() {
                                        continue;
                                    }
                                    let t_start =
                                        crate::signals::processing::get_lap_time_at_distance(
                                            &lap.dist,
                                            &lap.time,
                                            sector.start_dist,
                                        );
                                    let t_end =
                                        crate::signals::processing::get_lap_time_at_distance(
                                            &lap.dist,
                                            &lap.time,
                                            sector.end_dist,
                                        );
                                    if t_end > t_start {
                                        let time = t_end - t_start;
                                        if time > 0.0 {
                                            sector_times.push((lap.lap_num, time));
                                            if time < optimal_time {
                                                optimal_time = time;
                                            }
                                        }
                                    }
                                }

                                if sector_times.is_empty() {
                                    ui.label(
                                        egui::RichText::new("No valid lap data for this sector.")
                                            .color(theme.text_tertiary),
                                    );
                                    return;
                                }

                                let mut filtered_times = sector_times.clone();
                                if self.filter_large_sectors && optimal_time < f64::MAX {
                                    let threshold = optimal_time * 1.10;
                                    filtered_times.retain(|&(_, t)| t <= threshold);
                                }

                                if filtered_times.is_empty() {
                                    ui.label(
                                        egui::RichText::new(
                                            "All data points filtered out as anomalies.",
                                        )
                                        .color(theme.text_disabled),
                                    );
                                    return;
                                }

                                let line_points: Vec<[f64; 2]> =
                                    filtered_times.iter().map(|&(l, t)| [l as f64, t]).collect();

                                let min_t = filtered_times
                                    .iter()
                                    .map(|&(_, t)| t)
                                    .fold(f64::INFINITY, f64::min);
                                let max_t = filtered_times
                                    .iter()
                                    .map(|&(_, t)| t)
                                    .fold(f64::NEG_INFINITY, f64::max);

                                let num_bins = 15;
                                let mut bins = vec![0; num_bins];
                                let bin_width = if max_t > min_t {
                                    (max_t - min_t) / (num_bins as f64)
                                } else {
                                    1.0
                                };

                                for &(_, t) in &filtered_times {
                                    let mut bin_idx = ((t - min_t) / bin_width).floor() as usize;
                                    if bin_idx >= num_bins {
                                        bin_idx = num_bins - 1;
                                    }
                                    bins[bin_idx] += 1;
                                }

                                let bars: Vec<Bar> = bins
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, count)| {
                                        let x =
                                            min_t + (idx as f64) * bin_width + (bin_width / 2.0);
                                        Bar::new(x, count as f64)
                                            .width(bin_width * 0.9)
                                            .fill(theme.brand_secondary)
                                    })
                                    .collect();

                                let plot_height = 200.0;
                                let draw_line_plot = |ui: &mut egui::Ui| {
                                    egui::Frame::NONE
                                        .fill(theme.surface_elevated)
                                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                        .corner_radius(5.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new("Lap Consistency")
                                                    .strong()
                                                    .color(theme.text_secondary),
                                            );
                                            let line_plot = Plot::new(format!("timing_line_{}", i))
                                                .height(plot_height)
                                                .allow_drag(false)
                                                .allow_zoom(false)
                                                .allow_scroll(false)
                                                .allow_boxed_zoom(false)
                                                .allow_double_click_reset(false)
                                                .show_x(true)
                                                .show_y(true)
                                                .x_axis_formatter(|tick, _range| {
                                                    format!("Lap {}", tick.value as i32)
                                                })
                                                .y_axis_formatter(|tick, _range| {
                                                    format!("{:.2}s", tick.value)
                                                });

                                            line_plot.show(ui, |plot_ui| {
                                                plot_ui.line(
                                                    Line::new(
                                                        "Sector Time",
                                                        PlotPoints::new(line_points.clone()),
                                                    )
                                                    .color(theme.accent)
                                                    .width(2.0),
                                                );

                                                for point in &line_points {
                                                    plot_ui.text(Text::new(
                                                        format!("LapText_{}_{}", i, point[0]),
                                                        PlotPoint::new(point[0], point[1]),
                                                        egui::RichText::new(point[0].to_string())
                                                            .color(theme.text_primary)
                                                            .size(11.0)
                                                            .strong(),
                                                    ));
                                                }
                                            });
                                        });
                                };
                                let draw_histogram = |ui: &mut egui::Ui| {
                                    egui::Frame::NONE
                                        .fill(theme.surface_elevated)
                                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                        .corner_radius(5.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    "Time Distribution (Histogram)",
                                                )
                                                .strong()
                                                .color(theme.text_secondary),
                                            );
                                            let hist_plot = Plot::new(format!("timing_hist_{}", i))
                                                .height(plot_height)
                                                .allow_drag(false)
                                                .allow_zoom(false)
                                                .allow_scroll(false)
                                                .allow_boxed_zoom(false)
                                                .allow_double_click_reset(false)
                                                .show_x(true)
                                                .show_y(true)
                                                .x_axis_formatter(|tick, _range| {
                                                    format!("{:.2}s", tick.value)
                                                })
                                                .y_axis_formatter(|tick, _range| {
                                                    format!("{} laps", tick.value as i32)
                                                });

                                            hist_plot.show(ui, |plot_ui| {
                                                plot_ui.bar_chart(
                                                    BarChart::new("Frequency", bars.clone())
                                                        .color(theme.brand_secondary),
                                                );
                                            });
                                        });
                                };

                                if ui.available_width() < 720.0 {
                                    draw_line_plot(ui);
                                    ui.add_space(10.0);
                                    draw_histogram(ui);
                                } else {
                                    ui.columns(2, |cols| {
                                        draw_line_plot(&mut cols[0]);
                                        draw_histogram(&mut cols[1]);
                                    });
                                }
                            });
                        ui.add_space(15.0);
                    }
                });
            });
    }
}
