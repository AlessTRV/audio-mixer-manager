//! Notification area icon and its menu.
//!
//! Tray and menu callbacks run on the UI thread's message loop but outside
//! egui: they only queue a [`TrayRequest`] and wake egui, which handles it in
//! `App::logic` even while the window is hidden.

use std::sync::mpsc::Sender;

use audio_core::device::DeviceId;
use eframe::egui;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::icon;
use crate::model::{Output, Snapshot};
use crate::ui::strings;

const ID_OPEN: &str = "open";
const ID_QUIT: &str = "quit";
const OUTPUT_PREFIX: &str = "output:";

/// What the user asked for from the tray.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayRequest {
    Show,
    Quit,
    SetDefaultOutput(DeviceId),
}

/// The tray icon. Must live on the thread running the event loop.
pub struct Tray {
    _icon: TrayIcon,
    outputs_menu: Submenu,
    output_items: Vec<(DeviceId, CheckMenuItem)>,
    shown_outputs: Vec<Output>,
    shown_default: Option<DeviceId>,
}

impl Tray {
    /// Creates the icon and routes its events to `requests`, waking `ctx`.
    pub fn new(requests: &Sender<TrayRequest>, ctx: &egui::Context) -> anyhow::Result<Self> {
        let outputs_menu = Submenu::new(strings::TRAY_DEFAULT_OUTPUT, true);
        let menu = Menu::with_items(&[
            &MenuItem::with_id(ID_OPEN, strings::TRAY_OPEN, true, None),
            &PredefinedMenuItem::separator(),
            &outputs_menu,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(ID_QUIT, strings::TRAY_QUIT, true, None),
        ])?;

        let icon = Icon::from_rgba(icon::speaker_rgba(32), 32, 32)?;
        let tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip(strings::TRAY_TOOLTIP)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()?;

        let (tx, wake) = (requests.clone(), ctx.clone());
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id = event.id.0.as_str();
            let request = match id {
                ID_OPEN => Some(TrayRequest::Show),
                ID_QUIT => Some(TrayRequest::Quit),
                _ => id
                    .strip_prefix(OUTPUT_PREFIX)
                    .map(|device| TrayRequest::SetDefaultOutput(DeviceId(device.to_owned()))),
            };
            if let Some(request) = request {
                let _ = tx.send(request);
                wake.request_repaint();
            }
        }));

        let (tx, wake) = (requests.clone(), ctx.clone());
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = tx.send(TrayRequest::Show);
                wake.request_repaint();
            }
        }));

        Ok(Self {
            _icon: tray,
            outputs_menu,
            output_items: Vec::new(),
            shown_outputs: Vec::new(),
            shown_default: None,
        })
    }

    /// Brings the outputs submenu in line with `snapshot`.
    pub fn sync(&mut self, snapshot: &Snapshot) {
        if snapshot.outputs != self.shown_outputs {
            while self.outputs_menu.remove_at(0).is_some() {}
            self.output_items.clear();
            for output in &snapshot.outputs {
                let item = CheckMenuItem::with_id(
                    format!("{OUTPUT_PREFIX}{}", output.id.as_str()),
                    &output.name,
                    true,
                    false,
                    None,
                );
                if let Err(err) = self.outputs_menu.append(&item) {
                    tracing::warn!("cannot add tray menu item: {err}");
                    continue;
                }
                self.output_items.push((output.id.clone(), item));
            }
            self.shown_outputs.clone_from(&snapshot.outputs);
            self.shown_default = None;
        }
        if snapshot.default_output != self.shown_default || self.shown_default.is_none() {
            for (id, item) in &self.output_items {
                item.set_checked(snapshot.default_output.as_ref() == Some(id));
            }
            self.shown_default.clone_from(&snapshot.default_output);
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // The handlers hold clones of the egui context: release them with the tray.
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}
