//! Minimal dark theme: soft charcoal surfaces, one vivid accent, rounded shapes.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Theme, vec2,
};

/// Window background.
pub const BACKGROUND: Color32 = Color32::from_rgb(0x11, 0x13, 0x18);
/// Cards.
pub const CARD: Color32 = Color32::from_rgb(0x1a, 0x1d, 0x24);
/// Card under the pointer.
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x20, 0x24, 0x2c);
pub const CARD_STROKE: Color32 = Color32::from_rgb(0x26, 0x2a, 0x33);
/// Popups and menus, slightly above the cards.
pub const ELEVATED: Color32 = Color32::from_rgb(0x1f, 0x22, 0x2a);
/// Inputs, slider tracks, keycaps.
pub const SUNKEN: Color32 = Color32::from_rgb(0x14, 0x16, 0x1b);
/// Background of flat buttons and chips under the pointer.
pub const HOVER_FILL: Color32 = Color32::from_rgb(0x2a, 0x2e, 0x38);
/// Track of the volume slider.
pub const TRACK: Color32 = Color32::from_rgb(0x2c, 0x30, 0x3a);

pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xea, 0xed);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x8b, 0x91, 0x9c);
/// Even quieter text: hints, disabled values.
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x5c, 0x62, 0x6d);

/// The only vivid color, used sparingly.
pub const ACCENT: Color32 = Color32::from_rgb(0x7c, 0x6c, 0xff);
/// Accent under the pointer.
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x8f, 0x81, 0xff);
/// Translucent accent for selected backgrounds.
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(0x1f, 0x1b, 0x40, 0x40);
/// Text on accent fills.
pub const ON_ACCENT: Color32 = Color32::WHITE;

/// Soft coral for mute and errors.
pub const CORAL: Color32 = Color32::from_rgb(0xff, 0x7a, 0x7a);
/// Coral background of the active mute button.
pub const CORAL_SOFT: Color32 = Color32::from_rgb(0x3a, 0x22, 0x27);

/// Slider fill while muted.
pub const MUTED_FILL: Color32 = Color32::from_rgb(0x4a, 0x4f, 0x5a);
/// Badge behind application icons.
pub const BADGE: Color32 = Color32::from_rgb(0x24, 0x28, 0x31);

pub const CARD_RADIUS: u8 = 14;
pub const CONTROL_RADIUS: u8 = 10;
/// Fully rounded ends for pills.
pub const PILL_RADIUS: u8 = 255;

pub const CARD_MARGIN: Margin = Margin::symmetric(16, 14);

/// Very light card shadow.
pub const CARD_SHADOW: Shadow = Shadow {
    offset: [0, 2],
    blur: 10,
    spread: 0,
    color: Color32::from_black_alpha(45),
};

/// Duration of hover and state transitions, in seconds.
pub const FADE: f32 = 0.12;

/// Forces the dark theme and applies the palette, spacing and text sizes.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.text_styles = [
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(13.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Heading,
                FontId::new(18.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Monospace),
            ),
        ]
        .into();

        let spacing = &mut style.spacing;
        spacing.item_spacing = vec2(8.0, 8.0);
        spacing.button_padding = vec2(12.0, 6.0);
        spacing.interact_size.y = 30.0;
        spacing.menu_margin = Margin::same(6);
        spacing.window_margin = Margin::same(12);
        spacing.scroll.floating = true;
        spacing.scroll.bar_width = 6.0;

        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();
        visuals.panel_fill = BACKGROUND;
        visuals.window_fill = ELEVATED;
        visuals.window_stroke = Stroke::new(1.0, CARD_STROKE);
        visuals.window_corner_radius = CornerRadius::same(CARD_RADIUS);
        visuals.menu_corner_radius = CornerRadius::same(12);
        visuals.window_shadow = Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(90),
        };
        visuals.popup_shadow = Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(90),
        };
        visuals.extreme_bg_color = SUNKEN;
        visuals.text_edit_bg_color = Some(SUNKEN);
        visuals.faint_bg_color = CARD;
        visuals.override_text_color = None;
        visuals.weak_text_color = Some(TEXT_WEAK);
        visuals.hyperlink_color = ACCENT;
        visuals.error_fg_color = CORAL;
        visuals.warn_fg_color = CORAL;
        visuals.selection.bg_fill = ACCENT_SOFT;
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);

        let radius = CornerRadius::same(CONTROL_RADIUS);
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_STROKE);
        widgets.noninteractive.bg_fill = CARD;
        widgets.noninteractive.weak_bg_fill = CARD;
        widgets.noninteractive.corner_radius = radius;
        for (state, fill, stroke) in [
            (&mut widgets.inactive, SUNKEN, CARD_STROKE),
            (
                &mut widgets.hovered,
                HOVER_FILL,
                Color32::from_rgb(0x34, 0x39, 0x45),
            ),
            (&mut widgets.active, HOVER_FILL, ACCENT),
            (&mut widgets.open, HOVER_FILL, CARD_STROKE),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.fg_stroke = Stroke::new(1.0, TEXT);
            state.corner_radius = radius;
            state.expansion = 0.0;
        }
    });
}
