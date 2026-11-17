//! Shortcut view: list of global hotkeys, inline editor and key capture.

use std::collections::VecDeque;

use eframe::egui::{
    self, Align, CornerRadius, Event, Frame, Label, Layout, Rect, RichText, Stroke, TextEdit, Ui,
    vec2,
};

use super::keys::{self, Captured};
use super::mixer::menu_caption;
use super::widgets::{self, Icon, PillKind, Tone};
use super::{UiEvent, strings, theme};
use crate::actions::{Action, MoveMode};
use crate::config::{Binding, Config};
use crate::model::{BindingError, SYSTEM_SOUNDS_KEY, Snapshot};

const ROW_BUTTON_SIZE: f32 = 30.0;
const CARD_GAP: f32 = 10.0;

/// Kind of action picked in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Window,
    Mute,
    Move,
}

impl Kind {
    const ALL: [Self; 3] = [Self::Window, Self::Mute, Self::Move];

    fn label(self) -> &'static str {
        match self {
            Self::Window => strings::KIND_WINDOW,
            Self::Mute => strings::KIND_MUTE,
            Self::Move => strings::KIND_MOVE,
        }
    }
}

/// Binding being created or edited.
struct Draft {
    /// Index in `Config::bindings`; `None` for a new binding.
    editing: Option<usize>,
    kind: Kind,
    /// Application key or executable typed by the user.
    app: String,
    /// Output id and name.
    output: Option<(String, String)>,
    mode: MoveMode,
    keys: Option<String>,
    /// Scroll the editor into view on the next frame.
    reveal: bool,
}

impl Draft {
    fn new(kind: Kind) -> Self {
        Self {
            editing: None,
            kind,
            app: String::new(),
            output: None,
            mode: MoveMode::default(),
            keys: None,
            reveal: true,
        }
    }

    fn from_binding(index: usize, binding: &Binding) -> Self {
        let mut draft = Self::new(Kind::Window);
        draft.editing = Some(index);
        draft.keys = Some(binding.keys.clone());
        match &binding.action {
            Action::ToggleWindow => {}
            Action::ToggleMute { app } => {
                draft.kind = Kind::Mute;
                draft.app.clone_from(app);
            }
            Action::MoveAll {
                output,
                output_name,
                mode,
            } => {
                draft.kind = Kind::Move;
                draft.output = Some((output.clone(), output_name.clone()));
                draft.mode = *mode;
            }
        }
        draft
    }

    /// Normalized application key: lowercase, `.exe` added when missing.
    fn app_key(&self) -> Option<String> {
        let app = self.app.trim().to_lowercase();
        if app.is_empty() {
            None
        } else if app == SYSTEM_SOUNDS_KEY || app.contains('.') {
            Some(app)
        } else {
            Some(format!("{app}.exe"))
        }
    }

    /// The binding, or the reason it cannot be saved yet.
    fn build(&self, config: &Config) -> Result<Binding, &'static str> {
        let action = match self.kind {
            Kind::Window => Action::ToggleWindow,
            Kind::Mute => Action::ToggleMute {
                app: self.app_key().ok_or(strings::MISSING_APP)?,
            },
            Kind::Move => {
                let (output, output_name) = self.output.clone().ok_or(strings::MISSING_OUTPUT)?;
                Action::MoveAll {
                    output,
                    output_name,
                    mode: self.mode,
                }
            }
        };
        let keys = self.keys.clone().ok_or(strings::MISSING_KEYS)?;
        let duplicate = config.bindings.iter().enumerate().any(|(index, other)| {
            Some(index) != self.editing && keys::same_combo(&other.keys, &keys)
        });
        if duplicate {
            return Err(strings::DUPLICATE_KEYS);
        }
        Ok(Binding { keys, action })
    }
}

/// Key capture state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Capture {
    Idle,
    /// Waiting for a key; the message explains why the last press was refused.
    Waiting(Option<&'static str>),
}

/// State of the shortcut view.
pub struct ShortcutsUi {
    draft: Option<Draft>,
    capture: Capture,
}

impl ShortcutsUi {
    pub fn new() -> Self {
        Self {
            draft: None,
            capture: Capture::Idle,
        }
    }

    /// Opens the editor on a new "toggle mute" binding for `app`.
    pub fn start_mute(&mut self, app: String, events: &mut VecDeque<UiEvent>) {
        self.stop_capture(events);
        let mut draft = Draft::new(Kind::Mute);
        draft.app = app;
        self.draft = Some(draft);
    }

    /// Ends a running capture, telling the app to resume the hotkeys.
    pub fn stop_capture(&mut self, events: &mut VecDeque<UiEvent>) {
        if self.capture != Capture::Idle {
            self.capture = Capture::Idle;
            events.push_back(UiEvent::CaptureEnded);
        }
    }

    /// True when neither the editor nor a capture is open.
    pub fn is_idle(&self) -> bool {
        self.capture == Capture::Idle && self.draft.is_none()
    }

    /// Must run first in the frame, before any widget reads the keyboard.
    pub fn begin_frame(&mut self, ui: &mut Ui, events: &mut VecDeque<UiEvent>) {
        self.process_capture(ui, events);
    }

    /// Draws the "Shortcut" section: title, editor and one card per binding.
    pub fn section(
        &mut self,
        ui: &mut Ui,
        snapshot: &Snapshot,
        config: &Config,
        binding_errors: &[BindingError],
        events: &mut VecDeque<UiEvent>,
    ) {
        ui.horizontal(|ui| {
            section_title(ui, strings::SHORTCUTS_TITLE);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.draft.is_none()
                    && widgets::pill_button(
                        ui,
                        strings::ADD_SHORTCUT,
                        Some(Icon::Plus),
                        PillKind::Ghost,
                    )
                    .clicked()
                {
                    self.draft = Some(Draft::new(Kind::Window));
                }
            });
        });

        if self.draft.is_some() {
            self.editor(ui, snapshot, config, events);
            ui.add_space(CARD_GAP);
        }
        if config.bindings.is_empty() && self.draft.is_none() {
            empty_state(ui);
        }
        for (index, binding) in config.bindings.iter().enumerate() {
            let error = binding_errors
                .iter()
                .find(|error| error.index == index)
                .map(|error| error.message.as_str());
            ui.push_id(index, |ui| {
                match binding_card(ui, binding, snapshot, error) {
                    Some(RowAction::Edit) => {
                        self.stop_capture(events);
                        self.draft = Some(Draft::from_binding(index, binding));
                    }
                    Some(RowAction::Delete) => {
                        self.stop_capture(events);
                        if self.draft.as_ref().is_some_and(|d| d.editing.is_some()) {
                            self.draft = None;
                        }
                        let mut new_config = config.clone();
                        new_config.bindings.remove(index);
                        events.push_back(UiEvent::ConfigChanged(new_config));
                    }
                    None => {}
                }
            });
            ui.add_space(CARD_GAP);
        }
    }

    /// Reads the first key press while capturing and swallows keyboard input
    /// so no widget reacts to it.
    fn process_capture(&mut self, ui: &mut Ui, events: &mut VecDeque<UiEvent>) {
        if !matches!(self.capture, Capture::Waiting(_)) {
            return;
        }
        if !ui.input(|input| input.focused) {
            self.stop_capture(events);
            return;
        }
        let pressed = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                } => Some((*key, *physical_key, *modifiers)),
                // egui-winit turns Ctrl+C/X/V into these instead of key events.
                Event::Copy => Some((egui::Key::C, None, input.modifiers)),
                Event::Cut => Some((egui::Key::X, None, input.modifiers)),
                Event::Paste(_) => Some((egui::Key::V, None, input.modifiers)),
                _ => None,
            })
        });
        ui.input_mut(|input| {
            input.events.retain(|event| {
                !matches!(
                    event,
                    Event::Key { .. } | Event::Text(_) | Event::Copy | Event::Cut | Event::Paste(_)
                )
            });
        });
        let Some((key, physical, modifiers)) = pressed else {
            return;
        };
        if key == egui::Key::Escape {
            self.stop_capture(events);
            return;
        }
        match keys::capture(key, physical, modifiers) {
            Captured::Combo(combo) => {
                if let Some(draft) = &mut self.draft {
                    draft.keys = Some(combo);
                }
                self.stop_capture(events);
            }
            Captured::NeedsModifier => {
                self.capture = Capture::Waiting(Some(strings::NEEDS_MODIFIER));
            }
            Captured::Unsupported => {
                self.capture = Capture::Waiting(Some(strings::UNSUPPORTED_KEY));
            }
        }
    }

    fn editor(
        &mut self,
        ui: &mut Ui,
        snapshot: &Snapshot,
        config: &Config,
        events: &mut VecDeque<UiEvent>,
    ) {
        let Some(draft) = &mut self.draft else { return };
        let mut close = false;
        let mut save = None;
        let mut toggle_capture = false;
        let capturing = matches!(self.capture, Capture::Waiting(_));

        let response = card_frame(theme::CARD).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            let title = if draft.editing.is_some() {
                strings::EDIT_SHORTCUT
            } else {
                strings::NEW_SHORTCUT
            };
            ui.label(RichText::new(title).size(15.0).color(theme::TEXT));

            field_label(ui, strings::FIELD_ACTION);
            let selected = Kind::ALL.iter().position(|k| *k == draft.kind).unwrap_or(0);
            let labels = Kind::ALL.map(Kind::label);
            if let Some(index) = widgets::segmented(ui, &labels, selected) {
                draft.kind = Kind::ALL[index];
            }

            match draft.kind {
                Kind::Window => {}
                Kind::Mute => app_fields(ui, draft, snapshot),
                Kind::Move => output_fields(ui, draft, snapshot),
            }

            field_label(ui, strings::FIELD_KEYS);
            toggle_capture = keys_row(ui, self.capture, draft.keys.as_deref());
            ui.add_space(4.0);
            (save, close) = footer(ui, draft.build(config), capturing, draft.keys.is_some());
        });

        if draft.reveal {
            draft.reveal = false;
            ui.scroll_to_rect(response.response.rect, Some(Align::TOP));
        }

        if let Some(binding) = save {
            let mut new_config = config.clone();
            match draft.editing {
                Some(index) if index < new_config.bindings.len() => {
                    new_config.bindings[index] = binding;
                }
                _ => new_config.bindings.push(binding),
            }
            events.push_back(UiEvent::ConfigChanged(new_config));
            close = true;
        }
        if close {
            self.draft = None;
            self.stop_capture(events);
        } else if toggle_capture {
            if capturing {
                self.stop_capture(events);
            } else {
                self.capture = Capture::Waiting(None);
                events.push_back(UiEvent::CaptureStarted);
            }
        }
    }
}

/// Captured keys (or the capture prompt) and the capture button, with the
/// hint or the refusal reason below. Returns true when the button was
/// clicked. The button keeps its size; keys and prompt shrink first.
fn keys_row(ui: &mut Ui, capture: Capture, keys: Option<&str>) -> bool {
    let capturing = matches!(capture, Capture::Waiting(_));
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if capturing {
                strings::CANCEL
            } else if keys.is_some() {
                strings::RECAPTURE
            } else {
                strings::CAPTURE
            };
            let icon = (!capturing).then_some(Icon::Keyboard);
            clicked = widgets::pill_button(ui, label, icon, PillKind::Ghost).clicked();
            let height = ui.spacing().interact_size.y;
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    if capturing {
                        ui.add(
                            Label::new(RichText::new(strings::CAPTURING).color(theme::ACCENT))
                                .truncate(),
                        );
                    } else if let Some(combo) = keys {
                        widgets::keycaps(ui, &keys::readable(combo));
                    }
                },
            );
        });
    });
    if let Capture::Waiting(message) = capture {
        match message {
            Some(message) => widgets::message(ui, message, Tone::Error),
            None => widgets::message(ui, strings::CAPTURE_HINT, Tone::Info),
        };
    }
    clicked
}

/// Validation message, save and cancel. Returns the binding to save and
/// whether the editor should close.
fn footer(
    ui: &mut Ui,
    result: Result<Binding, &'static str>,
    capturing: bool,
    has_keys: bool,
) -> (Option<Binding>, bool) {
    // Before the first capture the missing keys are obvious: no message.
    if let Err(message) = &result
        && has_keys
        && !capturing
    {
        widgets::message(ui, message, Tone::Error);
    }
    let mut save = None;
    let mut close = false;
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let can_save = result.is_ok() && !capturing;
            let response = ui
                .add_enabled_ui(can_save, |ui| {
                    widgets::pill_button(ui, strings::SAVE, None, PillKind::Primary)
                })
                .inner;
            match result {
                Err(message) => {
                    response.on_disabled_hover_text(message);
                }
                Ok(binding) => {
                    if response.clicked() {
                        save = Some(binding);
                    }
                }
            }
            if widgets::pill_button(ui, strings::CANCEL, None, PillKind::Ghost).clicked() {
                close = true;
            }
        })
    });
    (save, close)
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
}

/// App picker: menu of running applications plus a free text field.
fn app_fields(ui: &mut Ui, draft: &mut Draft, snapshot: &Snapshot) {
    field_label(ui, strings::FIELD_APP);
    let key = draft.app_key();
    let running = snapshot
        .apps
        .iter()
        .find(|app| Some(&app.key) == key.as_ref());
    let chip_text = match (running, key.as_deref()) {
        (Some(app), _) => app_name(app),
        (None, Some(key)) => key,
        (None, None) => strings::PICK_APP,
    };
    let response = widgets::chip(ui, chip_text, false, ui.available_width());
    widgets::chip_menu(&response, |ui| {
        for app in &snapshot.apps {
            let selected = Some(&app.key) == key.as_ref();
            if widgets::menu_item(ui, selected, app_name(app)).clicked() {
                draft.app.clone_from(&app.key);
            }
        }
    });
    ui.add(
        TextEdit::singleline(&mut draft.app)
            .hint_text(strings::APP_EXE_HINT)
            .desired_width(ui.available_width())
            .margin(vec2(10.0, 6.0)),
    );
}

fn app_name(app: &crate::model::AppEntry) -> &str {
    if app.is_system {
        strings::SYSTEM_SOUNDS
    } else {
        &app.display_name
    }
}

/// Output menu and move mode.
fn output_fields(ui: &mut Ui, draft: &mut Draft, snapshot: &Snapshot) {
    field_label(ui, strings::FIELD_OUTPUT);
    let chip_text = draft
        .output
        .as_ref()
        .map_or(strings::PICK_OUTPUT, |(id, name)| {
            snapshot
                .outputs
                .iter()
                .find(|output| &output.id.0 == id)
                .map_or(name.as_str(), |output| output.name.as_str())
        });
    let response = widgets::chip(ui, chip_text, false, ui.available_width());
    widgets::chip_menu(&response, |ui| {
        menu_caption(ui, strings::FIELD_OUTPUT);
        for output in &snapshot.outputs {
            let selected = draft
                .output
                .as_ref()
                .is_some_and(|(id, _)| id == &output.id.0);
            if widgets::menu_item(ui, selected, &output.name).clicked() {
                draft.output = Some((output.id.0.clone(), output.name.clone()));
            }
        }
    });

    field_label(ui, strings::FIELD_MODE);
    ui.spacing_mut().item_spacing.y = 6.0;
    for (mode, title, detail) in [
        (
            MoveMode::Reset,
            strings::MODE_RESET,
            strings::MODE_RESET_DETAIL,
        ),
        (
            MoveMode::ActiveOnly,
            strings::MODE_ACTIVE_ONLY,
            strings::MODE_ACTIVE_ONLY_DETAIL,
        ),
        (
            MoveMode::KeepRules,
            strings::MODE_KEEP_RULES,
            strings::MODE_KEEP_RULES_DETAIL,
        ),
    ] {
        if widgets::option_row(ui, draft.mode == mode, title, detail).clicked() {
            draft.mode = mode;
        }
    }
}

fn mode_label(mode: MoveMode) -> &'static str {
    match mode {
        MoveMode::Reset => strings::MODE_RESET,
        MoveMode::ActiveOnly => strings::MODE_ACTIVE_ONLY,
        MoveMode::KeepRules => strings::MODE_KEEP_RULES,
    }
}

/// Human description of an action.
fn describe(action: &Action, snapshot: &Snapshot) -> String {
    match action {
        Action::ToggleWindow => strings::ACTION_TOGGLE_WINDOW.to_owned(),
        Action::ToggleMute { app } => {
            let name = snapshot
                .apps
                .iter()
                .find(|entry| &entry.key == app)
                .map_or(app.as_str(), app_name);
            format!("{} {name}", strings::ACTION_TOGGLE_MUTE)
        }
        Action::MoveAll {
            output,
            output_name,
            mode,
        } => {
            let name = snapshot
                .outputs
                .iter()
                .find(|o| &o.id.0 == output)
                .map(|o| o.name.as_str())
                .or_else(|| (!output_name.is_empty()).then_some(output_name.as_str()))
                .unwrap_or(output.as_str());
            format!(
                "{} {name} · {}",
                strings::ACTION_MOVE_ALL,
                mode_label(*mode)
            )
        }
    }
}

enum RowAction {
    Edit,
    Delete,
}

/// Card of one binding: keycaps, description, error, edit and delete.
fn binding_card(
    ui: &mut Ui,
    binding: &Binding,
    snapshot: &Snapshot,
    error: Option<&str>,
) -> Option<RowAction> {
    let card_id = ui.scope_id().with("card");
    let last_rect: Option<Rect> = ui.data(|data| data.get_temp(card_id));
    let hovered = last_rect.is_some_and(|rect| ui.rect_contains_pointer(rect));
    let t = ui
        .ctx()
        .animate_bool_with_time(card_id, hovered, theme::FADE);

    let mut action = None;
    let response = card_frame(theme::CARD.lerp_to_gamma(theme::CARD_HOVER, t)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let buttons = ROW_BUTTON_SIZE * 2.0 + ui.spacing().item_spacing.x * 2.0;
            let text_width = (ui.available_width() - buttons).max(60.0);
            ui.allocate_ui_with_layout(vec2(text_width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(text_width);
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.horizontal(|ui| widgets::keycaps(ui, &keys::readable(&binding.keys)));
                ui.add(
                    Label::new(
                        RichText::new(describe(&binding.action, snapshot)).color(theme::TEXT_WEAK),
                    )
                    .truncate(),
                );
                if let Some(error) = error {
                    widgets::message(ui, error, Tone::Error);
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Trash, ROW_BUTTON_SIZE, strings::DELETE).clicked()
                {
                    action = Some(RowAction::Delete);
                }
                if widgets::icon_button(ui, Icon::Pencil, ROW_BUTTON_SIZE, strings::EDIT).clicked()
                {
                    action = Some(RowAction::Edit);
                }
            });
        });
    });
    ui.data_mut(|data| data.insert_temp(card_id, response.response.rect));
    action
}

fn empty_state(ui: &mut Ui) {
    card_frame(theme::CARD).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(10), theme::BADGE);
            widgets::paint_icon(
                ui.painter(),
                rect.shrink(9.0),
                Icon::Keyboard,
                theme::TEXT_FAINT,
            );
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(strings::NO_SHORTCUTS_TITLE).color(theme::TEXT));
                widgets::message(ui, strings::NO_SHORTCUTS_HINT, Tone::Info);
            });
        });
    });
}

/// Title of a settings section.
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).color(theme::TEXT));
}

/// Card used by the settings sections.
pub fn card_frame(fill: egui::Color32) -> Frame {
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, theme::CARD_STROKE))
        .corner_radius(CornerRadius::same(theme::CARD_RADIUS))
        .inner_margin(theme::CARD_MARGIN)
        .shadow(theme::CARD_SHADOW)
}
