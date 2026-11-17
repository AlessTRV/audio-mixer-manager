//! Small notification shown when a global shortcut runs an action, also while
//! the main window is hidden.
//!
//! It is an immediate viewport: a borderless, always-on-top window without
//! taskbar entry that is never activated and lets the mouse through. The
//! caller must keep egui running while [`Overlay::is_active`] (see
//! [`Overlay::keep_alive`]), because a hidden root window gets no ui pass.

use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Rect, Stroke, StrokeKind, ViewportBuilder,
    ViewportCommand, ViewportId, pos2, vec2,
};

use super::widgets::{self, Icon};
use super::{strings, theme};
use crate::actions::{Action, MoveMode};
use crate::config::{Config, DeviceKind};
use crate::model::Snapshot;

/// Time the message stays fully visible, fades included.
const VISIBLE: Duration = Duration::from_millis(1600);
/// Fade in and fade out.
const FADE: f32 = 0.15;
const SIZE: egui::Vec2 = vec2(300.0, 64.0);
/// Distance from the bottom of the monitor, above a standard taskbar.
const BOTTOM_OFFSET: f32 = 96.0;
const BADGE: f32 = 40.0;

/// Content of the overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayMessage {
    icon: Icon,
    /// Coral icon (muted, no session) instead of the accent.
    warning: bool,
    title: String,
    detail: String,
}

/// What to do for an action triggered by a hotkey.
pub struct ActionFeedback {
    pub message: OverlayMessage,
    /// False when the action has nothing to act on (app without sessions):
    /// the command should not be sent.
    pub run: bool,
}

impl OverlayMessage {
    /// "Audio su <output>", with the move mode as detail.
    #[must_use]
    pub fn output(name: &str, mode: MoveMode, kind: DeviceKind) -> Self {
        let detail = match mode {
            MoveMode::Reset => strings::OVERLAY_MODE_RESET,
            MoveMode::ActiveOnly => strings::OVERLAY_MODE_ACTIVE_ONLY,
            MoveMode::KeepRules => strings::OVERLAY_MODE_KEEP_RULES,
        };
        Self {
            icon: Icon::device(kind),
            warning: false,
            title: format!("{} {name}", strings::OVERLAY_AUDIO_ON),
            detail: detail.to_owned(),
        }
    }

    /// Application name, "audio off/on" as detail. `muted` is the new state.
    #[must_use]
    pub fn mute(app_name: &str, muted: bool) -> Self {
        Self {
            icon: if muted {
                Icon::SpeakerMuted
            } else {
                Icon::Speaker
            },
            warning: muted,
            title: app_name.to_owned(),
            detail: if muted {
                strings::OVERLAY_MUTED
            } else {
                strings::OVERLAY_UNMUTED
            }
            .to_owned(),
        }
    }

    /// The application of a mute shortcut has no audio session.
    #[must_use]
    pub fn no_session(app: &str) -> Self {
        Self {
            icon: Icon::SpeakerMuted,
            warning: true,
            title: app.to_owned(),
            detail: strings::OVERLAY_NO_SESSION.to_owned(),
        }
    }

    /// Feedback for an action run by a hotkey; `None` for actions without an
    /// overlay (showing or hiding the window).
    #[must_use]
    pub fn for_action(
        action: &Action,
        snapshot: &Snapshot,
        config: &Config,
    ) -> Option<ActionFeedback> {
        match action {
            Action::ToggleWindow => None,
            Action::ToggleMute { app } => {
                let feedback = match snapshot.apps.iter().find(|entry| &entry.key == app) {
                    Some(entry) => {
                        let name = if entry.is_system {
                            strings::SYSTEM_SOUNDS
                        } else {
                            entry.display_name.as_str()
                        };
                        ActionFeedback {
                            message: Self::mute(name, !entry.muted),
                            run: true,
                        }
                    }
                    None => ActionFeedback {
                        message: Self::no_session(app),
                        run: false,
                    },
                };
                Some(feedback)
            }
            Action::MoveAll {
                output,
                output_name,
                mode,
            } => {
                let live = snapshot
                    .outputs
                    .iter()
                    .find(|o| &o.id.0 == output)
                    .map(|o| o.name.as_str());
                let name = live
                    .or_else(|| (!output_name.is_empty()).then_some(output_name.as_str()))
                    .unwrap_or(output.as_str());
                let kind = config
                    .favorite_outputs
                    .iter()
                    .find(|favorite| &favorite.id == output)
                    .map_or_else(|| DeviceKind::guess(name), |favorite| favorite.kind);
                Some(ActionFeedback {
                    message: Self::output(name, *mode, kind),
                    run: true,
                })
            }
        }
    }
}

/// The overlay and its timer.
#[derive(Default)]
pub struct Overlay {
    current: Option<(OverlayMessage, Instant)>,
}

impl Overlay {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows `message`, replacing the current one and restarting the timer.
    pub fn show_message(&mut self, message: OverlayMessage) {
        self.current = Some((message, Instant::now()));
    }

    /// True while a message is on screen, and for the one pass after it that
    /// closes the viewport.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.current.is_some()
    }

    /// Call from `App::logic`: while active, asks eframe for a ui pass even
    /// with the main window hidden, and for the next frame of the animation.
    pub fn keep_alive(&self, ctx: &egui::Context) {
        if self.is_active() {
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }

    /// Call from `App::ui` every frame. Not showing the viewport in a pass
    /// closes its window.
    pub fn show(&mut self, ctx: &egui::Context) {
        let Some((message, started)) = &self.current else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed >= VISIBLE {
            self.current = None;
            return;
        }
        let builder = ViewportBuilder::default()
            .with_title(strings::OVERLAY_TITLE)
            .with_inner_size(SIZE)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_mouse_passthrough(true)
            .with_active(false);
        let builder = match position(ctx) {
            Some(pos) => builder.with_position(pos),
            None => builder,
        };
        let (message, started) = (message.clone(), *started);
        // Deferred, not immediate: while the main window is hidden eframe has no
        // event loop at hand to create an immediate viewport and panics.
        // The callback runs on the overlay's own repaints, so it computes the
        // fade itself and keeps repainting until the message expires.
        ctx.show_viewport_deferred(
            ViewportId::from_hash_of("hotkey-overlay"),
            builder,
            move |ui, _class| {
                let elapsed = started.elapsed().as_secs_f32();
                let remaining = VISIBLE.as_secs_f32() - elapsed;
                let opacity = (elapsed / FADE).min(remaining / FADE).clamp(0.0, 1.0);
                paint(ui, &message, opacity);
                if remaining > 0.0 {
                    ui.ctx().request_repaint();
                }
            },
        );
    }
}

/// Bottom center of the monitor of the main window, in points. The monitor
/// origin is not known: this assumes it is the primary one.
fn position(ctx: &egui::Context) -> Option<egui::Pos2> {
    let monitor = ctx.input(|input| input.viewport().monitor_size)?;
    Some(pos2(
        ((monitor.x - SIZE.x) / 2.0).round(),
        (monitor.y - SIZE.y - BOTTOM_OFFSET).round(),
    ))
}

fn paint(ui: &mut egui::Ui, message: &OverlayMessage, opacity: f32) {
    ui.multiply_opacity(opacity);
    let rect = ui.max_rect();
    let painter = ui.painter();
    // The card fills the whole window: if the window cannot be transparent,
    // only the rounded corners show the clear color.
    let radius = CornerRadius::same(theme::CARD_RADIUS);
    painter.rect_filled(rect, radius, theme::ELEVATED);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, theme::CARD_STROKE),
        StrokeKind::Inside,
    );

    let badge = Rect::from_center_size(
        pos2(rect.left() + 14.0 + BADGE / 2.0, rect.center().y),
        vec2(BADGE, BADGE),
    );
    let (badge_fill, icon_color) = if message.warning {
        (theme::CORAL_SOFT, theme::CORAL)
    } else {
        (theme::ACCENT_SOFT, theme::ACCENT_HOVER)
    };
    painter.rect_filled(badge, CornerRadius::same(10), badge_fill);
    widgets::paint_icon(painter, badge.shrink(9.0), message.icon, icon_color);

    let x = badge.right() + 12.0;
    let width = (rect.right() - 14.0 - x).max(20.0);
    let title = single_line(ui, &message.title, FontId::proportional(14.0), width);
    let detail = single_line(ui, &message.detail, FontId::proportional(12.0), width);
    let total = title.size().y + 2.0 + detail.size().y;
    let top = rect.center().y - total / 2.0;
    let title_height = title.size().y;
    let painter = ui.painter();
    painter.galley(pos2(x, top), title, theme::TEXT);
    painter.galley(pos2(x, top + title_height + 2.0), detail, theme::TEXT_WEAK);
}

fn single_line(
    ui: &egui::Ui,
    text: &str,
    font: FontId,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job =
        egui::text::LayoutJob::simple_singleline(text.to_owned(), font, Color32::PLACEHOLDER);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    ui.painter().layout_job(job)
}
