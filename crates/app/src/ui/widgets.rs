//! Custom controls painted by hand: icons, icon buttons, pills, chips, the
//! volume slider and option rows.
//!
//! Icons are drawn with the painter instead of emoji, which the default fonts
//! may lack. Every transition goes through `animate_*`, which only requests
//! repaints while the animation runs: a still window stays idle.

use std::f32::consts::PI;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect,
    Response, Sense, Shape, Stroke, StrokeKind, TextStyle, Ui, Vec2, pos2, vec2,
};

use super::theme;
use crate::config::DeviceKind;

/// Icons drawn with the painter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Speaker,
    SpeakerMuted,
    Keyboard,
    ChevronDown,
    ChevronLeft,
    Plus,
    Trash,
    Pencil,
    Check,
    ChevronUp,
    Dots,
    Gear,
    Headphones,
    SpeakerBox,
    Monitor,
}

impl Icon {
    /// Icon of an output device kind.
    pub fn device(kind: DeviceKind) -> Self {
        match kind {
            DeviceKind::Headphones => Self::Headphones,
            DeviceKind::Speakers => Self::SpeakerBox,
            DeviceKind::Display => Self::Monitor,
            DeviceKind::Generic => Self::Speaker,
        }
    }
}

/// Paints `icon` centered in `rect` (square, the icon uses ~60 % of it).
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    // Unit: one tenth of the icon box.
    let u = rect.width().min(rect.height()) * 0.06;
    let stroke = Stroke::new((u * 1.5).max(1.3), color);
    let p = |x: f32, y: f32| pos2(c.x + x * u, c.y + y * u);
    match icon {
        Icon::Speaker | Icon::SpeakerMuted => {
            let dx = if icon == Icon::Speaker { -2.0 } else { 0.0 };
            let body = vec![
                p(dx - 5.0, -2.2),
                p(dx - 2.5, -2.2),
                p(dx + 1.5, -5.5),
                p(dx + 1.5, 5.5),
                p(dx - 2.5, 2.2),
                p(dx - 5.0, 2.2),
            ];
            painter.add(Shape::convex_polygon(body, color, Stroke::NONE));
            if icon == Icon::Speaker {
                for radius in [3.5, 6.5] {
                    painter.add(Shape::line(
                        arc(p(dx + 1.5, 0.0), radius * u, -PI / 4.0, PI / 4.0),
                        stroke,
                    ));
                }
            } else {
                painter.line_segment(
                    [p(-6.0, -6.0), p(6.0, 6.0)],
                    Stroke::new(stroke.width, color),
                );
            }
        }
        Icon::Keyboard => {
            let body = Rect::from_center_size(c, vec2(14.0 * u, 9.5 * u));
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Middle);
            for row in [-2.0, 0.6] {
                for col in [-4.0, -1.3, 1.3, 4.0] {
                    painter.circle_filled(p(col, row), u * 0.75, color);
                }
            }
            painter.line_segment([p(-2.8, 3.0), p(2.8, 3.0)], stroke);
        }
        Icon::ChevronDown => {
            painter.add(Shape::line(
                vec![p(-3.5, -1.5), p(0.0, 2.0), p(3.5, -1.5)],
                stroke,
            ));
        }
        Icon::ChevronLeft => {
            painter.add(Shape::line(
                vec![p(1.5, -4.5), p(-3.0, 0.0), p(1.5, 4.5)],
                stroke,
            ));
        }
        Icon::Plus => {
            painter.line_segment([p(-5.0, 0.0), p(5.0, 0.0)], stroke);
            painter.line_segment([p(0.0, -5.0), p(0.0, 5.0)], stroke);
        }
        Icon::Trash => {
            painter.line_segment([p(-5.5, -3.5), p(5.5, -3.5)], stroke);
            painter.line_segment([p(-1.8, -5.5), p(1.8, -5.5)], stroke);
            painter.add(Shape::line(
                vec![p(-4.0, -3.5), p(-3.2, 5.5), p(3.2, 5.5), p(4.0, -3.5)],
                stroke,
            ));
        }
        Icon::Pencil => {
            painter.line_segment(
                [p(-4.5, 4.5), p(3.5, -3.5)],
                Stroke::new(stroke.width * 1.6, color),
            );
            painter.line_segment([p(-5.5, 5.5), p(-4.5, 4.5)], stroke);
        }
        Icon::Check => {
            painter.add(Shape::line(
                vec![p(-4.5, 0.3), p(-1.3, 3.5), p(4.5, -3.0)],
                stroke,
            ));
        }
        Icon::ChevronUp => {
            painter.add(Shape::line(
                vec![p(-3.5, 1.5), p(0.0, -2.0), p(3.5, 1.5)],
                stroke,
            ));
        }
        Icon::Dots | Icon::Gear | Icon::Headphones | Icon::SpeakerBox | Icon::Monitor => {
            paint_more_icons(painter, c, u, icon, color);
        }
    }
}

/// Second half of [`paint_icon`], same coordinate system.
fn paint_more_icons(painter: &egui::Painter, c: Pos2, u: f32, icon: Icon, color: Color32) {
    let stroke = Stroke::new((u * 1.5).max(1.3), color);
    let p = |x: f32, y: f32| pos2(c.x + x * u, c.y + y * u);
    match icon {
        Icon::Dots => {
            for x in [-4.5, 0.0, 4.5] {
                painter.circle_filled(p(x, 0.0), u * 1.2, color);
            }
        }
        Icon::Gear => {
            painter.circle_stroke(c, 3.6 * u, stroke);
            let tooth = Stroke::new(stroke.width * 1.4, color);
            for i in 0..8u8 {
                let angle = f32::from(i) * PI / 4.0;
                let dir = vec2(angle.cos(), angle.sin());
                painter.line_segment([c + dir * 4.6 * u, c + dir * 6.4 * u], tooth);
            }
        }
        Icon::Headphones => {
            painter.add(Shape::line(arc(p(0.0, 1.0), 5.2 * u, PI, 2.0 * PI), stroke));
            for x in [-4.6, 4.6] {
                let cup = Rect::from_center_size(p(x, 3.3), vec2(3.0 * u, 5.0 * u));
                painter.rect_filled(cup, CornerRadius::same(2), color);
            }
        }
        Icon::SpeakerBox => {
            let body = Rect::from_center_size(c, vec2(9.0 * u, 12.5 * u));
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Middle);
            painter.circle_stroke(p(0.0, 2.0), 2.4 * u, stroke);
            painter.circle_filled(p(0.0, -3.2), u, color);
        }
        Icon::Monitor => {
            let screen = Rect::from_center_size(p(0.0, -1.2), vec2(13.0 * u, 8.5 * u));
            painter.rect_stroke(screen, CornerRadius::same(2), stroke, StrokeKind::Middle);
            painter.line_segment([p(0.0, 3.0), p(0.0, 5.2)], stroke);
            painter.line_segment([p(-3.2, 5.5), p(3.2, 5.5)], stroke);
        }
        _ => {}
    }
}

/// Points of a circular arc from angle `from` to `to` (radians, 0 = right).
fn arc(center: Pos2, radius: f32, from: f32, to: f32) -> Vec<Pos2> {
    const STEPS: u16 = 10;
    (0..=STEPS)
        .map(|i| {
            let angle = from + (to - from) * f32::from(i) / f32::from(STEPS);
            center + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

/// Hover factor `0..=1` for a widget, animated.
pub fn hover_t(ui: &Ui, response: &Response) -> f32 {
    let lit = response.hovered() || response.has_focus() || response.is_pointer_button_down_on();
    ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        lit && ui.is_enabled(),
        theme::FADE,
    )
}

/// Round, flat icon button: no background until hovered.
pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tooltip: &str) -> Response {
    icon_button_dim(ui, icon, size, tooltip, 1.0)
}

/// Like [`icon_button`], but at rest the icon is drawn at `idle_opacity`
/// (`0..=1`); hover, and an open popup anchored to it, light it fully.
pub fn icon_button_dim(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    tooltip: &str,
    idle_opacity: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let t = if open { 1.0 } else { hover_t(ui, &response) };
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.circle_filled(
            rect.center(),
            size / 2.0,
            transparent_to(theme::HOVER_FILL, t),
        );
        let idle = transparent_to(theme::TEXT_WEAK, idle_opacity.clamp(0.0, 1.0));
        let color = idle.lerp_to_gamma(theme::TEXT, t);
        paint_icon(painter, rect, icon, color);
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// How an output button shows that it is the app's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMark {
    None,
    /// The app follows the system default, which is this device.
    Default,
    /// The app is explicitly assigned to this device.
    Assigned,
}

/// Compact round button for one output device. Disabled (faint) when the
/// ui is disabled.
pub fn output_button(ui: &mut Ui, icon: Icon, size: f32, mark: OutputMark) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = hover_t(ui, &response);
    let ctx = ui.ctx().clone();
    let assigned = ctx.animate_bool_with_time(
        response.id.with("assigned"),
        mark == OutputMark::Assigned,
        theme::FADE,
    );
    let default = ctx.animate_bool_with_time(
        response.id.with("default"),
        mark == OutputMark::Default,
        theme::FADE,
    );
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();
        let radius = size / 2.0;
        let fill = transparent_to(theme::HOVER_FILL, t).lerp_to_gamma(theme::ACCENT_SOFT, assigned);
        painter.circle_filled(center, radius, fill);
        if default > 0.0 {
            painter.circle_stroke(
                center,
                radius - 1.0,
                Stroke::new(1.0, transparent_to(theme::ACCENT, 0.45 * default)),
            );
        }
        let base = if ui.is_enabled() {
            theme::TEXT_WEAK.lerp_to_gamma(theme::TEXT, t)
        } else {
            theme::TEXT_FAINT
        };
        let color = base
            .lerp_to_gamma(theme::ACCENT.gamma_multiply(0.8), default * 0.8)
            .lerp_to_gamma(theme::ACCENT_HOVER, assigned);
        paint_icon(painter, rect.shrink(size * 0.12), icon, color);
    }
    if ui.is_enabled() {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// On/off switch.
pub fn toggle(ui: &mut Ui, on: bool) -> Response {
    let size = vec2(36.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let t = hover_t(ui, &response);
    let s = ui
        .ctx()
        .animate_bool_with_time(response.id.with("on"), on, theme::FADE);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let off_fill = theme::TRACK.lerp_to_gamma(theme::HOVER_FILL, t);
        let fill = off_fill.lerp_to_gamma(theme::ACCENT, s);
        let fill = if ui.is_enabled() {
            fill
        } else {
            fill.gamma_multiply(0.5)
        };
        painter.rect_filled(rect, CornerRadius::same(theme::PILL_RADIUS), fill);
        let knob_radius = rect.height() / 2.0 - 3.0;
        let x = egui::lerp(
            (rect.left() + 3.0 + knob_radius)..=(rect.right() - 3.0 - knob_radius),
            s,
        );
        let knob = if ui.is_enabled() {
            theme::TEXT
        } else {
            theme::TEXT_FAINT
        };
        painter.circle_filled(pos2(x, rect.center().y), knob_radius, knob);
    }
    if ui.is_enabled() {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// Round mute toggle: speaker when audible, coral barred speaker when muted.
pub fn mute_button(ui: &mut Ui, muted: bool, size: f32, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = hover_t(ui, &response);
    let m = ui
        .ctx()
        .animate_bool_with_time(response.id.with("muted"), muted, theme::FADE);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let idle = transparent_to(theme::HOVER_FILL, t);
        let fill = idle.lerp_to_gamma(theme::CORAL_SOFT, m);
        painter.circle_filled(rect.center(), size / 2.0, fill);
        let normal = theme::TEXT_WEAK.lerp_to_gamma(theme::TEXT, t);
        let color = normal.lerp_to_gamma(theme::CORAL, m);
        let icon = if m > 0.5 {
            Icon::SpeakerMuted
        } else {
            Icon::Speaker
        };
        paint_icon(painter, rect, icon, color);
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// `color` with its alpha scaled by `t`.
fn transparent_to(color: Color32, t: f32) -> Color32 {
    Color32::TRANSPARENT.lerp_to_gamma(color, t)
}

/// Pill button style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    /// Accent fill: the main action of a view.
    Primary,
    /// Flat, background on hover.
    Ghost,
}

/// Fully rounded text button, with an optional leading icon.
pub fn pill_button(ui: &mut Ui, text: &str, icon: Option<Icon>, kind: PillKind) -> Response {
    let font = TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER);
    let icon_size = 16.0;
    let icon_width = if icon.is_some() { icon_size + 6.0 } else { 0.0 };
    let padding = vec2(14.0, 0.0);
    let size = vec2(
        galley.size().x + icon_width + padding.x * 2.0,
        ui.spacing().interact_size.y,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let t = hover_t(ui, &response);
    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let (fill, text_color) = match kind {
            PillKind::Primary => (
                theme::ACCENT.lerp_to_gamma(theme::ACCENT_HOVER, t),
                theme::ON_ACCENT,
            ),
            PillKind::Ghost => (
                transparent_to(theme::HOVER_FILL, t),
                theme::TEXT_WEAK.lerp_to_gamma(theme::TEXT, t),
            ),
        };
        let (fill, text_color) = if enabled {
            (fill, text_color)
        } else {
            (
                if kind == PillKind::Primary {
                    theme::HOVER_FILL
                } else {
                    fill
                },
                theme::TEXT_FAINT,
            )
        };
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(theme::PILL_RADIUS), fill);
        let mut x = rect.left() + padding.x;
        if let Some(icon) = icon {
            let icon_rect = Rect::from_center_size(
                pos2(x + icon_size / 2.0, rect.center().y),
                Vec2::splat(icon_size),
            );
            paint_icon(painter, icon_rect, icon, text_color);
            x += icon_width;
        }
        painter.galley(
            pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );
    }
    if ui.is_enabled() {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// Compact pill showing a value and a chevron; clicking it is meant to open a
/// popup (see [`chip_menu`]). `max_width` truncates long names.
pub fn chip(ui: &mut Ui, text: &str, small: bool, max_width: f32) -> Response {
    let font = if small {
        TextStyle::Small.resolve(ui.style())
    } else {
        TextStyle::Button.resolve(ui.style())
    };
    let height = if small {
        24.0
    } else {
        ui.spacing().interact_size.y
    };
    let padding = if small { 10.0 } else { 12.0 };
    let chevron = if small { 12.0 } else { 14.0 };
    let max_text = (max_width - padding * 2.0 - chevron - 4.0).max(20.0);
    let galley = truncated_galley(ui, text, font, max_text);
    let elided = galley.elided;
    let size = vec2(galley.size().x + padding * 2.0 + chevron + 4.0, height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let t = hover_t(ui, &response);
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let base = if small {
            Color32::TRANSPARENT
        } else {
            theme::SUNKEN
        };
        let lit = if open { 1.0 } else { t };
        painter.rect_filled(
            rect,
            CornerRadius::same(theme::PILL_RADIUS),
            base.lerp_to_gamma(theme::HOVER_FILL, lit),
        );
        if !small {
            painter.rect_stroke(
                rect,
                CornerRadius::same(theme::PILL_RADIUS),
                Stroke::new(1.0, theme::CARD_STROKE),
                StrokeKind::Inside,
            );
        }
        let color = if ui.is_enabled() {
            theme::TEXT_WEAK.lerp_to_gamma(theme::TEXT, lit)
        } else {
            theme::TEXT_FAINT
        };
        painter.galley(
            pos2(
                rect.left() + padding,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            color,
        );
        let chevron_rect = Rect::from_center_size(
            pos2(
                rect.right() - padding - chevron / 2.0 + 2.0,
                rect.center().y,
            ),
            Vec2::splat(chevron),
        );
        paint_icon(painter, chevron_rect, Icon::ChevronDown, color);
    }
    let response = if elided {
        response.on_hover_text(text)
    } else {
        response
    };
    if ui.is_enabled() {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// Lays out `text` on one line, cut with an ellipsis to fit `max_width`.
fn truncated_galley(
    ui: &Ui,
    text: &str,
    font: FontId,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job =
        egui::text::LayoutJob::simple_singleline(text.to_owned(), font, Color32::PLACEHOLDER);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    ui.painter().layout_job(job)
}

/// Opens a rounded popup below `response` when it is clicked.
pub fn chip_menu<R>(response: &Response, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    egui::Popup::menu(response)
        .width(response.rect.width().max(220.0))
        .gap(6.0)
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            add_contents(ui)
        })
        .map(|inner| inner.inner)
}

/// One entry of a chip menu: full-width row, check mark when selected.
pub fn menu_item(ui: &mut Ui, selected: bool, text: &str) -> Response {
    menu_item_icon(ui, selected, None, text)
}

/// [`menu_item`] with a leading icon.
pub fn menu_item_icon(ui: &mut Ui, selected: bool, icon: Option<Icon>, text: &str) -> Response {
    const ICON: f32 = 18.0;
    let font = TextStyle::Body.resolve(ui.style());
    let width = ui.available_width().max(160.0);
    let indent = if icon.is_some() { ICON + 8.0 } else { 0.0 };
    let galley = truncated_galley(ui, text, font, width - 44.0 - indent);
    let elided = galley.elided;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let t = hover_t(ui, &response);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            CornerRadius::same(8),
            transparent_to(theme::HOVER_FILL, t),
        );
        let color = if selected {
            theme::TEXT
        } else {
            theme::TEXT_WEAK.lerp_to_gamma(theme::TEXT, t)
        };
        if let Some(icon) = icon {
            let icon_rect = Rect::from_center_size(
                pos2(rect.left() + 10.0 + ICON / 2.0, rect.center().y),
                Vec2::splat(ICON),
            );
            paint_icon(painter, icon_rect, icon, color);
        }
        painter.galley(
            pos2(
                rect.left() + 10.0 + indent,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            color,
        );
        if selected {
            let check = Rect::from_center_size(
                pos2(rect.right() - 18.0, rect.center().y),
                Vec2::splat(16.0),
            );
            paint_icon(painter, check, Icon::Check, theme::ACCENT);
        }
    }
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if elided {
        response.on_hover_text(text)
    } else {
        response
    }
}

/// Result of [`volume_slider`].
pub struct SliderOutput {
    pub response: Response,
    /// New value in percent, when the user moved the slider this frame.
    pub changed: Option<f32>,
    /// The pointer is holding the slider.
    pub dragging: bool,
}

/// Thin rounded volume track, accent fill and a knob that grows on hover.
/// `peak` (`0..=1`, already smoothed) lights up the filled part.
pub fn volume_slider(
    ui: &mut Ui,
    id: Id,
    width: f32,
    percent: f32,
    peak: f32,
    muted: bool,
) -> SliderOutput {
    const HEIGHT: f32 = 24.0;
    const TRACK: f32 = 6.0;
    const KNOB: f32 = 7.0;
    const KNOB_HOVER: f32 = 9.0;

    let (rect, _) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::hover());
    let response = ui.interact(rect, id, Sense::click_and_drag());
    // The knob may stick out by its radius on both ends.
    let rail = rect.shrink2(vec2(KNOB_HOVER, 0.0));

    let mut changed = None;
    if let Some(pointer) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked() || response.is_pointer_button_down_on())
    {
        let value = ((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0) * 100.0;
        let value = value.round();
        if (value - percent).abs() >= 0.5 {
            changed = Some(value);
        }
    }
    let dragging = response.dragged() || response.is_pointer_button_down_on();
    let shown = changed.unwrap_or(percent);

    let lit = ui.ctx().animate_bool_with_time(
        id.with("knob"),
        response.hovered() || dragging,
        theme::FADE,
    );
    let m = ui
        .ctx()
        .animate_bool_with_time(id.with("muted"), muted, theme::FADE);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track = Rect::from_center_size(rail.center(), vec2(rail.width(), TRACK));
        let radius = CornerRadius::same(theme::PILL_RADIUS);
        painter.rect_filled(track.expand2(vec2(TRACK / 2.0, 0.0)), radius, theme::TRACK);

        let fill_color = theme::ACCENT.lerp_to_gamma(theme::MUTED_FILL, m);
        let x = rail.left() + rail.width() * shown / 100.0;
        let filled = Rect::from_min_max(
            pos2(track.left() - TRACK / 2.0, track.top()),
            pos2(x, track.bottom()),
        );
        if filled.width() > 0.0 {
            painter.rect_filled(filled, radius, fill_color);
        }

        // Audio level: a brighter band inside the filled part.
        let level = peak.clamp(0.0, 1.0) * (1.0 - m);
        if level > 0.01 && shown > 0.0 {
            let band = Rect::from_min_max(
                filled.min,
                pos2(filled.left() + filled.width() * level, filled.bottom()),
            );
            painter.rect_filled(band, radius, Color32::from_white_alpha(70));
        }

        let knob_radius = KNOB + (KNOB_HOVER - KNOB) * lit;
        let center = pos2(x, rail.center().y);
        if lit > 0.0 {
            painter.circle_filled(
                center,
                knob_radius + 5.0 * lit,
                transparent_to(fill_color, 0.25 * lit),
            );
        }
        painter.circle_filled(center, knob_radius, theme::TEXT);
    }

    SliderOutput {
        response: response.on_hover_cursor(CursorIcon::PointingHand),
        changed,
        dragging,
    }
}

/// Selectable row with a title and an explanation, radio-style. The title
/// is cut to one line, the explanation wraps on at most two.
pub fn option_row(ui: &mut Ui, selected: bool, title: &str, detail: &str) -> Response {
    let width = ui.available_width();
    let text_width = (width - 48.0).max(40.0);
    let title_galley = truncated_galley(ui, title, TextStyle::Body.resolve(ui.style()), text_width);
    let detail_galley =
        wrapped_galley(ui, detail, TextStyle::Small.resolve(ui.style()), text_width);
    let elided = title_galley.elided || detail_galley.elided;
    let height = 14.0 + title_galley.size().y + 2.0 + detail_galley.size().y;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let t = hover_t(ui, &response);
    let s = ui
        .ctx()
        .animate_bool_with_time(response.id.with("selected"), selected, theme::FADE);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same(theme::CONTROL_RADIUS);
        let fill = theme::SUNKEN.lerp_to_gamma(theme::HOVER_FILL, t * 0.6);
        painter.rect_filled(rect, radius, fill);
        let stroke = theme::CARD_STROKE.lerp_to_gamma(theme::ACCENT, s);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, stroke), StrokeKind::Inside);

        let dot = pos2(rect.left() + 18.0, rect.center().y);
        painter.circle_stroke(
            dot,
            7.0,
            Stroke::new(1.5, theme::TEXT_FAINT.lerp_to_gamma(theme::ACCENT, s)),
        );
        if s > 0.0 {
            painter.circle_filled(dot, 3.5 * s, theme::ACCENT);
        }
        let x = rect.left() + 36.0;
        let y = rect.top() + 7.0;
        let title_height = title_galley.size().y;
        painter.galley(pos2(x, y), title_galley, theme::TEXT);
        painter.galley(
            pos2(x, y + title_height + 2.0),
            detail_galley,
            theme::TEXT_WEAK,
        );
    }
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if elided {
        response.on_hover_text(format!("{title}\n{detail}"))
    } else {
        response
    }
}

/// Segmented control: one pill per option, the selected one filled. When
/// the labels do not fit, the segments share the width and labels are cut.
/// Returns the index clicked this frame.
pub fn segmented(ui: &mut Ui, options: &[&str], selected: usize) -> Option<usize> {
    const PADDING: f32 = 24.0;
    const GAP: f32 = 2.0;
    let font = TextStyle::Button.resolve(ui.style());
    let natural: Vec<f32> = options
        .iter()
        .map(|text| {
            ui.painter()
                .layout_no_wrap((*text).to_owned(), font.clone(), Color32::PLACEHOLDER)
                .size()
                .x
                + PADDING
        })
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "a handful of options")]
    let count = options.len().max(1) as f32;
    // Frame margin on both sides plus the gaps.
    let room = ui.available_width() - 6.0 - GAP * (count - 1.0);
    let fits = natural.iter().sum::<f32>() <= room;
    let shared = (room / count).max(PADDING + 10.0);

    let mut clicked = None;
    Frame::new()
        .fill(theme::SUNKEN)
        .corner_radius(CornerRadius::same(theme::PILL_RADIUS))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            ui.horizontal(|ui| {
                for (index, label) in options.iter().enumerate() {
                    let width = if fits { natural[index] } else { shared };
                    if segment(ui, label, index == selected, width).clicked() {
                        clicked = Some(index);
                    }
                }
            });
        });
    clicked
}

fn segment(ui: &mut Ui, text: &str, selected: bool, width: f32) -> Response {
    let font = TextStyle::Button.resolve(ui.style());
    let galley = truncated_galley(ui, text, font, width - 16.0);
    let elided = galley.elided;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 26.0), Sense::click());
    let t = hover_t(ui, &response);
    let s = ui
        .ctx()
        .animate_bool_with_time(response.id.with("selected"), selected, theme::FADE);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let base = transparent_to(theme::HOVER_FILL, t * 0.7);
        painter.rect_filled(
            rect,
            CornerRadius::same(theme::PILL_RADIUS),
            base.lerp_to_gamma(theme::ACCENT, s),
        );
        let color = theme::TEXT_WEAK
            .lerp_to_gamma(theme::TEXT, t)
            .lerp_to_gamma(theme::ON_ACCENT, s);
        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    }
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if elided {
        response.on_hover_text(text)
    } else {
        response
    }
}

/// Draws a key combination as a row of keycaps; when they do not fit in the
/// available width, falls back to one line of text cut with an ellipsis.
pub fn keycaps(ui: &mut Ui, keys: &[String]) {
    const SIZE: f32 = 12.5;
    const PADDING: f32 = 16.0;
    const GAP: f32 = 4.0;
    let font = FontId::proportional(SIZE);
    let needed: f32 = keys
        .iter()
        .map(|key| {
            ui.painter()
                .layout_no_wrap(key.clone(), font.clone(), Color32::PLACEHOLDER)
                .size()
                .x
                + PADDING
                + GAP
        })
        .sum();
    if needed > ui.available_width() {
        ui.add(
            egui::Label::new(
                egui::RichText::new(keys.join(" + "))
                    .size(SIZE)
                    .color(theme::TEXT),
            )
            .truncate(),
        );
        return;
    }
    ui.spacing_mut().item_spacing.x = GAP;
    for key in keys {
        Frame::new()
            .fill(theme::SUNKEN)
            .stroke(Stroke::new(1.0, theme::CARD_STROKE))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(key).size(SIZE).color(theme::TEXT));
            });
    }
}

/// Rows of a message before it is cut.
const MESSAGE_ROWS: usize = 2;

/// Lays out `text` wrapped to `max_width` on at most [`MESSAGE_ROWS`] rows,
/// the last one cut with an ellipsis.
fn wrapped_galley(
    ui: &Ui,
    text: &str,
    font: FontId,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job =
        egui::text::LayoutJob::simple(text.to_owned(), font, Color32::PLACEHOLDER, max_width);
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows: MESSAGE_ROWS,
        break_anywhere: false,
        overflow_character: Some('\u{2026}'),
    };
    ui.painter().layout_job(job)
}

/// Style of a [`message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Small coral dot before the text.
    Error,
    /// Plain, quieter text.
    Info,
}

/// Short secondary text that wraps inside the available width on at most
/// two rows; longer text is cut and shown whole in a tooltip.
pub fn message(ui: &mut Ui, text: &str, tone: Tone) -> Response {
    const DOT: f32 = 6.0;
    let font = TextStyle::Small.resolve(ui.style());
    let row_height = font.size;
    let indent = if tone == Tone::Error { DOT + 6.0 } else { 0.0 };
    let width = (ui.available_width() - indent).max(40.0);
    let galley = wrapped_galley(ui, text, font, width);
    let elided = galley.elided;
    let size = vec2(indent + galley.size().x, galley.size().y.max(row_height));
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if tone == Tone::Error {
            painter.circle_filled(
                pos2(rect.left() + DOT / 2.0, rect.top() + row_height * 0.6),
                DOT / 2.0,
                theme::CORAL,
            );
        }
        let color = match tone {
            Tone::Error => theme::TEXT_WEAK,
            Tone::Info => theme::TEXT_FAINT,
        };
        painter.galley(pos2(rect.left() + indent, rect.top()), galley, color);
    }
    if elided {
        response.on_hover_text(text)
    } else {
        response
    }
}

/// Square badge with rounded corners behind an application icon.
pub fn badge_rect(ui: &mut Ui, size: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), theme::BADGE);
    rect
}

/// Paints centered text, e.g. a fallback initial inside a badge.
pub fn centered_text(ui: &Ui, rect: Rect, text: &str, size: f32, color: Color32) {
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(size),
        color,
    );
}
