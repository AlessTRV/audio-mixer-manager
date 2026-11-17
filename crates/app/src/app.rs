//! The eframe application: window visibility, tray and hotkey requests,
//! configuration and the window UI.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use eframe::egui;

use crate::actions::Action;
use crate::config::Config;
use crate::controller::{self, Shared};
use crate::hotkeys::Hotkeys;
use crate::model::{BindingError, Command};
use crate::tray::{Tray, TrayRequest};
use crate::ui::{UiEvent, WindowUi};

pub struct MixerApp {
    shared: Shared,
    controller: Option<JoinHandle<()>>,
    ui: WindowUi,
    tray: Option<Tray>,
    requests: Receiver<TrayRequest>,
    hotkeys: Option<Hotkeys>,
    actions: Receiver<Action>,
    config: Config,
    binding_errors: Vec<BindingError>,
    visible: bool,
    quitting: bool,
    synced_generation: u64,
}

impl MixerApp {
    /// Builds the app inside eframe's creator, on the UI thread.
    ///
    /// Returns the sender other threads (single-instance activation) can use to
    /// queue requests.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        start_hidden: bool,
    ) -> anyhow::Result<(Self, Sender<TrayRequest>)> {
        let ctx = cc.egui_ctx.clone();
        let wake = ctx.clone();
        let (shared, controller) = controller::spawn(move || wake.request_repaint())?;

        let (requests_tx, requests) = mpsc::channel();
        let tray = match Tray::new(&requests_tx, &ctx) {
            Ok(tray) => Some(tray),
            Err(err) => {
                tracing::error!("tray icon unavailable: {err:#}");
                None
            }
        };

        let (config, warning) = Config::load();
        if let Some(warning) = warning {
            tracing::warn!("{warning}");
        }

        let (actions_tx, actions) = mpsc::channel();
        let wake = ctx.clone();
        let mut hotkeys = match Hotkeys::new(move |action| {
            let _ = actions_tx.send(action);
            wake.request_repaint();
        }) {
            Ok(hotkeys) => Some(hotkeys),
            Err(err) => {
                tracing::error!("global hotkeys unavailable: {err:#}");
                None
            }
        };
        let binding_errors = hotkeys
            .as_mut()
            .map(|hotkeys| hotkeys.apply(&config.bindings))
            .unwrap_or_default();

        let mut app = Self {
            shared,
            controller: Some(controller),
            ui: WindowUi::new(&ctx),
            tray,
            requests,
            hotkeys,
            actions,
            config,
            binding_errors,
            visible: false,
            quitting: false,
            synced_generation: 0,
        };
        // Without a tray icon a hidden window could never be reopened.
        if start_hidden && app.tray.is_some() {
            app.hide(&ctx);
        } else {
            app.show(&ctx);
        }
        Ok((app, requests_tx))
    }

    fn show(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        if !self.visible {
            self.visible = true;
            self.shared.send(Command::SetMetering(true));
        }
    }

    fn hide(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        // A capture in progress cannot end while hidden: re-enable hotkeys now.
        if let Some(hotkeys) = &mut self.hotkeys {
            hotkeys.resume();
        }
        self.visible = false;
        self.shared.send(Command::SetMetering(false));
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn run_action(&mut self, ctx: &egui::Context, action: &Action) {
        if let Some(command) = action.to_command() {
            self.shared.send(command);
            return;
        }
        // ToggleWindow: hide only when the window is already in front.
        let focused = ctx.input(|i| i.viewport().focused).unwrap_or(false);
        if self.visible && focused {
            self.hide(ctx);
        } else {
            self.show(ctx);
        }
    }

    fn apply_config(&mut self, config: Config) {
        if let Err(err) = config.save() {
            tracing::error!("cannot save configuration: {err:#}");
        }
        self.config = config;
        if let Some(hotkeys) = &mut self.hotkeys {
            self.binding_errors = hotkeys.apply(&self.config.bindings);
        }
    }
}

impl eframe::App for MixerApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(request) = self.requests.try_recv() {
            match request {
                TrayRequest::Show => self.show(ctx),
                TrayRequest::Quit => self.quit(ctx),
                TrayRequest::SetDefaultOutput(id) => {
                    self.shared.send(Command::SetDefaultOutput(id));
                }
            }
        }
        while let Ok(action) = self.actions.try_recv() {
            self.run_action(ctx, &action);
        }

        // Closing the window keeps the app running in the tray.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting && self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide(ctx);
        }

        if let Some(tray) = &mut self.tray {
            let snapshot = self.shared.snapshot();
            if snapshot.generation != self.synced_generation {
                tray.sync(&snapshot);
                self.synced_generation = snapshot.generation;
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let snapshot = self.shared.snapshot();
        let event = self.ui.show(
            ui,
            &snapshot,
            &self.shared,
            &self.config,
            &self.binding_errors,
        );
        match event {
            UiEvent::None => {}
            UiEvent::ConfigChanged(config) => self.apply_config(config),
            UiEvent::CaptureStarted => {
                if let Some(hotkeys) = &mut self.hotkeys {
                    hotkeys.suspend();
                }
            }
            UiEvent::CaptureEnded => {
                if let Some(hotkeys) = &mut self.hotkeys {
                    hotkeys.resume();
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.hotkeys = None;
        self.shared.send(Command::Shutdown);
        if let Some(controller) = self.controller.take() {
            let _ = controller.join();
        }
        self.tray = None;
    }
}
