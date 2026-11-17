//! Audio Mixer Manager: per-application output routing, mute and volume from
//! the tray.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod config;
mod controller;
mod hotkeys;
mod icon;
mod instance;
mod model;
mod tray;
mod ui;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;
use tracing_subscriber::EnvFilter;

use crate::app::MixerApp;
use crate::instance::Instance;
use crate::tray::TrayRequest;

/// Command-line flag used by the autostart entry.
const HIDDEN_FLAG: &str = "--hidden";

fn main() -> anyhow::Result<()> {
    let _log_guard = init_logging();

    let primary = match instance::acquire() {
        Ok(Instance::Primary(primary)) => Some(primary),
        Ok(Instance::Secondary) => {
            tracing::info!("already running; asked the other instance to show itself");
            return Ok(());
        }
        Err(err) => {
            tracing::warn!("single-instance check failed, continuing: {err}");
            None
        }
    };

    let start_hidden = std::env::args().skip(1).any(|arg| arg == HIDDEN_FLAG);
    tracing::info!(start_hidden, "starting");

    let icon = egui::IconData {
        rgba: icon::speaker_rgba(64),
        width: 64,
        height: 64,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(ui::strings::APP_TITLE)
            .with_inner_size([440.0, 560.0])
            .with_min_inner_size([360.0, 320.0])
            .with_icon(icon)
            .with_visible(false),
        ..Default::default()
    };

    // The activation thread needs the request sender, which only exists once
    // eframe has built the app; park it here until then.
    let primary = Arc::new(Mutex::new(primary));
    eframe::run_native(
        ui::strings::APP_TITLE,
        options,
        Box::new(move |cc| {
            let (app, requests) = MixerApp::new(cc, start_hidden)?;
            if let Some(primary) = primary.lock().ok().and_then(|mut slot| slot.take()) {
                let ctx = cc.egui_ctx.clone();
                let activate = move || {
                    let _ = requests.send(TrayRequest::Show);
                    ctx.request_repaint();
                };
                if let Err(err) = primary.on_activate(activate) {
                    tracing::warn!("single-instance wakeup unavailable: {err}");
                }
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| anyhow::anyhow!("{err}"))
}

/// Logs to a daily file in `%LOCALAPPDATA%\AudioMixerManager\logs`, and to
/// stderr in debug builds. Level from `RUST_LOG`, default `info`.
fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(|base| PathBuf::from(base).join("AudioMixerManager").join("logs"));
    let Some(dir) = dir else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
        return None;
    };
    let appender = tracing_appender::rolling::daily(dir, "audiomixermanager.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    Some(guard)
}
