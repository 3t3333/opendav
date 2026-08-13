use egui::{Color32, Stroke, Style, Visuals};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppTheme {
    pub is_dark: bool,
    pub surface_root: Color32,
    pub surface_panel: Color32,
    pub surface_elevated: Color32,
    pub surface_card: Color32,
    pub surface_input: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub text_disabled: Color32,
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub on_accent: Color32,
    pub brand_secondary: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub reference_primary: Color32,
    pub reference_primary_faint: Color32,
    pub reference_secondary: Color32,
    pub reference_secondary_faint: Color32,
    pub plot_grid: Color32,
    pub plot_divider: Color32,
    pub speed: Color32,
    pub rpm: Color32,
    pub gear: Color32,
    pub throttle: Color32,
    pub brake: Color32,
    pub clutch: Color32,
    pub steering: Color32,
    pub tyre_lf: [Color32; 3],
    pub tyre_rf: [Color32; 3],
    pub tyre_lr: [Color32; 3],
    pub tyre_rr: [Color32; 3],
    pub shock_corners: [Color32; 4],
}

impl AppTheme {
    pub const fn for_mode(is_dark: bool) -> Self {
        if is_dark {
            Self {
                is_dark,
                surface_root: Color32::from_rgb(10, 10, 10),
                surface_panel: Color32::from_rgb(15, 16, 18),
                surface_elevated: Color32::from_rgb(24, 27, 31),
                surface_card: Color32::from_rgb(29, 32, 37),
                surface_input: Color32::from_rgb(20, 22, 26),
                text_primary: Color32::from_rgb(242, 244, 247),
                text_secondary: Color32::from_rgb(185, 192, 202),
                text_tertiary: Color32::from_rgb(139, 149, 163),
                text_disabled: Color32::from_rgb(99, 108, 120),
                border_subtle: Color32::from_rgb(45, 50, 58),
                border_strong: Color32::from_rgb(76, 85, 98),
                accent: Color32::from_rgb(242, 82, 37),
                accent_text: Color32::from_rgb(255, 117, 77),
                on_accent: Color32::from_rgb(15, 15, 15),
                brand_secondary: Color32::from_rgb(156, 132, 255),
                success: Color32::from_rgb(76, 214, 139),
                warning: Color32::from_rgb(255, 183, 77),
                danger: Color32::from_rgb(255, 104, 104),
                reference_primary: Color32::from_rgb(245, 247, 250),
                reference_primary_faint: Color32::from_rgba_premultiplied(245, 247, 250, 110),
                reference_secondary: Color32::from_rgb(65, 233, 255),
                reference_secondary_faint: Color32::from_rgba_premultiplied(65, 233, 255, 110),
                plot_grid: Color32::from_rgb(36, 41, 48),
                plot_divider: Color32::from_rgb(49, 56, 65),
                speed: Color32::from_rgb(94, 177, 255),
                rpm: Color32::from_rgb(255, 210, 56),
                gear: Color32::from_rgb(255, 124, 190),
                throttle: Color32::from_rgb(68, 219, 135),
                brake: Color32::from_rgb(255, 105, 91),
                clutch: Color32::from_rgb(82, 174, 235),
                steering: Color32::from_rgb(156, 132, 255),
                tyre_lf: [
                    Color32::from_rgb(72, 202, 255),
                    Color32::from_rgb(37, 133, 255),
                    Color32::from_rgb(118, 228, 255),
                ],
                tyre_rf: [
                    Color32::from_rgb(255, 184, 77),
                    Color32::from_rgb(255, 105, 70),
                    Color32::from_rgb(255, 220, 120),
                ],
                tyre_lr: [
                    Color32::from_rgb(84, 220, 145),
                    Color32::from_rgb(19, 150, 100),
                    Color32::from_rgb(170, 235, 90),
                ],
                tyre_rr: [
                    Color32::from_rgb(190, 130, 255),
                    Color32::from_rgb(128, 90, 230),
                    Color32::from_rgb(238, 140, 220),
                ],
                shock_corners: [
                    Color32::from_rgb(72, 202, 255),
                    Color32::from_rgb(255, 154, 72),
                    Color32::from_rgb(84, 220, 145),
                    Color32::from_rgb(190, 130, 255),
                ],
            }
        } else {
            Self {
                is_dark,
                surface_root: Color32::from_rgb(239, 237, 233),
                surface_panel: Color32::from_rgb(247, 246, 243),
                surface_elevated: Color32::from_rgb(255, 255, 253),
                surface_card: Color32::from_rgb(250, 249, 246),
                surface_input: Color32::from_rgb(255, 255, 255),
                text_primary: Color32::from_rgb(30, 34, 40),
                text_secondary: Color32::from_rgb(72, 80, 91),
                text_tertiary: Color32::from_rgb(101, 111, 124),
                text_disabled: Color32::from_rgb(145, 152, 162),
                border_subtle: Color32::from_rgb(207, 204, 198),
                border_strong: Color32::from_rgb(159, 164, 171),
                accent: Color32::from_rgb(242, 82, 37),
                accent_text: Color32::from_rgb(164, 48, 16),
                on_accent: Color32::from_rgb(20, 20, 20),
                brand_secondary: Color32::from_rgb(79, 55, 171),
                success: Color32::from_rgb(22, 101, 52),
                warning: Color32::from_rgb(146, 76, 0),
                danger: Color32::from_rgb(176, 35, 35),
                reference_primary: Color32::from_rgb(55, 65, 81),
                reference_primary_faint: Color32::from_rgba_premultiplied(55, 65, 81, 105),
                reference_secondary: Color32::from_rgb(0, 102, 119),
                reference_secondary_faint: Color32::from_rgba_premultiplied(0, 102, 119, 105),
                plot_grid: Color32::from_rgb(218, 216, 211),
                plot_divider: Color32::from_rgb(190, 188, 182),
                speed: Color32::from_rgb(22, 91, 156),
                rpm: Color32::from_rgb(128, 92, 0),
                gear: Color32::from_rgb(151, 35, 94),
                throttle: Color32::from_rgb(16, 112, 61),
                brake: Color32::from_rgb(177, 48, 39),
                clutch: Color32::from_rgb(23, 96, 145),
                steering: Color32::from_rgb(79, 55, 171),
                tyre_lf: [
                    Color32::from_rgb(0, 103, 153),
                    Color32::from_rgb(29, 78, 180),
                    Color32::from_rgb(0, 130, 145),
                ],
                tyre_rf: [
                    Color32::from_rgb(157, 80, 0),
                    Color32::from_rgb(180, 48, 30),
                    Color32::from_rgb(126, 99, 0),
                ],
                tyre_lr: [
                    Color32::from_rgb(0, 110, 58),
                    Color32::from_rgb(0, 122, 90),
                    Color32::from_rgb(67, 105, 0),
                ],
                tyre_rr: [
                    Color32::from_rgb(100, 50, 170),
                    Color32::from_rgb(76, 58, 180),
                    Color32::from_rgb(143, 43, 119),
                ],
                shock_corners: [
                    Color32::from_rgb(0, 103, 153),
                    Color32::from_rgb(157, 80, 0),
                    Color32::from_rgb(0, 110, 58),
                    Color32::from_rgb(100, 50, 170),
                ],
            }
        }
    }

    pub fn apply(self, style: &mut Style) {
        style.visuals = if self.is_dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        style.visuals.window_fill = self.surface_root;
        style.visuals.panel_fill = self.surface_root;
        style.visuals.extreme_bg_color = self.surface_input;
        style.visuals.faint_bg_color = self.surface_panel;
        style.visuals.code_bg_color = self.surface_elevated;
        style.visuals.window_stroke = Stroke::new(1.0, self.border_subtle);
        style.visuals.selection.bg_fill = self.accent;
        style.visuals.selection.stroke = Stroke::new(1.0, self.on_accent);

        style.visuals.widgets.noninteractive.bg_fill = self.surface_panel;
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_primary);
        style.visuals.widgets.inactive.bg_fill = self.surface_elevated;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary);
        style.visuals.widgets.hovered.bg_fill = self.surface_card;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, self.border_strong);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, self.text_primary);
        style.visuals.widgets.active.bg_fill = self.accent;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, self.accent_text);
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.5, self.on_accent);
        style.visuals.widgets.open.bg_fill = self.surface_card;
        style.visuals.widgets.open.bg_stroke = Stroke::new(1.5, self.accent_text);
        style.visuals.widgets.open.fg_stroke = Stroke::new(1.5, self.text_primary);
        style.visuals.hyperlink_color = self.accent_text;
        style.visuals.warn_fg_color = self.warning;
        style.visuals.error_fg_color = self.danger;
    }
}

#[cfg(test)]
mod tests {
    use super::AppTheme;

    fn relative_luminance(color: egui::Color32) -> f64 {
        let convert = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * convert(color.r()) + 0.7152 * convert(color.g()) + 0.0722 * convert(color.b())
    }

    fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
        let (lighter, darker) = {
            let a = relative_luminance(a);
            let b = relative_luminance(b);
            if a > b {
                (a, b)
            } else {
                (b, a)
            }
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn light_theme_text_tokens_meet_normal_text_contrast() {
        let theme = AppTheme::for_mode(false);

        assert!(contrast(theme.text_primary, theme.surface_root) >= 4.5);
        assert!(contrast(theme.text_secondary, theme.surface_root) >= 4.5);
        assert!(contrast(theme.accent_text, theme.surface_root) >= 4.5);
    }

    #[test]
    fn channel_colors_meet_graphical_contrast_in_both_modes() {
        for theme in [AppTheme::for_mode(true), AppTheme::for_mode(false)] {
            for color in [
                theme.speed,
                theme.rpm,
                theme.gear,
                theme.throttle,
                theme.brake,
                theme.clutch,
                theme.steering,
            ] {
                assert!(contrast(color, theme.surface_root) >= 3.0);
            }
            for color in theme
                .tyre_lf
                .into_iter()
                .chain(theme.tyre_rf)
                .chain(theme.tyre_lr)
                .chain(theme.tyre_rr)
                .chain(theme.shock_corners)
            {
                assert!(contrast(color, theme.surface_root) >= 3.0);
            }
        }
    }
}
