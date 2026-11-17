//! Main window, built with `egui`: the mixer view and the settings view
//! (favorite outputs, shortcut editor).
//!
//! [`WindowUi`] only reads the [`Snapshot`] and [`Config`] it is given. Audio
//! changes go to the controller through [`Shared`]; configuration changes and
//! key capture are reported to the caller as [`UiEvent`]s.

mod keys;
mod mixer;
mod overlay;
mod settings;
mod shortcuts;
pub mod strings;
mod theme;
mod widgets;

use std::collections::VecDeque;

use eframe::egui::{self, Frame, Margin};

use crate::config::Config;
use crate::controller::Shared;
use crate::model::{BindingError, Snapshot};
use mixer::{MixerRequest, MixerUi};
use settings::SettingsUi;

pub use overlay::{Overlay, OverlayMessage};

/// Something the caller has to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    None,
    /// The user confirmed a change to the configuration (bindings or
    /// favorite outputs): save and re-register.
    ConfigChanged(Config),
    /// Key capture started: suspend the global hotkeys until `CaptureEnded`.
    CaptureStarted,
    /// Key capture finished or was cancelled.
    CaptureEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Mixer,
    Settings,
}

/// State of the window that survives between frames.
pub struct WindowUi {
    view: View,
    mixer: MixerUi,
    settings: SettingsUi,
    /// Events not returned yet: `show` hands out one per frame.
    pending: VecDeque<UiEvent>,
}

impl WindowUi {
    /// Applies theme and style to `ctx`.
    #[must_use]
    pub fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        Self {
            view: View::Mixer,
            mixer: MixerUi::new(),
            settings: SettingsUi::new(),
            pending: VecDeque::new(),
        }
    }

    /// Draws the whole window (mixer or settings view) in `ui`, which should
    /// be the root ui of the viewport. Returns at most one event per frame;
    /// when more are queued it requests another frame.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        shared: &Shared,
        config: &Config,
        binding_errors: &[BindingError],
    ) -> UiEvent {
        if let Some(error) = &snapshot.error {
            status_bar(ui, error);
        }

        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(theme::BACKGROUND).inner_margin(Margin {
                left: 16,
                right: 16,
                top: 14,
                bottom: 4,
            }))
            .show(ui, |ui| match self.view {
                View::Mixer => match self.mixer.show(ui, snapshot, shared, config) {
                    Some(MixerRequest::OpenSettings) => {
                        self.settings.on_open();
                        self.view = View::Settings;
                    }
                    Some(MixerRequest::MuteShortcut(app)) => {
                        self.settings.on_open();
                        self.settings.shortcuts.start_mute(app, &mut self.pending);
                        self.view = View::Settings;
                    }
                    None => {}
                },
                View::Settings => {
                    let back =
                        self.settings
                            .show(ui, snapshot, config, binding_errors, &mut self.pending);
                    if back {
                        self.settings.shortcuts.stop_capture(&mut self.pending);
                        self.view = View::Mixer;
                    }
                }
            });

        let event = self.pending.pop_front().unwrap_or(UiEvent::None);
        if !self.pending.is_empty() {
            ui.ctx().request_repaint();
        }
        event
    }
}

fn status_bar(ui: &mut egui::Ui, error: &str) {
    egui::Panel::bottom("window-status")
        .resizable(false)
        .frame(
            Frame::new()
                .fill(theme::BACKGROUND)
                .inner_margin(Margin::symmetric(16, 8)),
        )
        .show(ui, |ui| {
            // Two rows at most; the full text is in the tooltip.
            widgets::message(ui, error, widgets::Tone::Error);
        });
}
