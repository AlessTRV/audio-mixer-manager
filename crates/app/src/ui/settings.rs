//! Settings view: favorite outputs and keyboard shortcuts.

use std::collections::VecDeque;

use audio_core::device::DeviceId;
use eframe::egui::{self, Align, Label, Layout, RichText, ScrollArea, Ui, vec2};

use super::shortcuts::{ShortcutsUi, card_frame, section_title};
use super::widgets::{self, Icon, Tone};
use super::{UiEvent, strings, theme};
use crate::config::{Config, DeviceKind, FavoriteOutput, MAX_FAVORITE_OUTPUTS};
use crate::model::{BindingError, Snapshot};

const ROW_BUTTON_SIZE: f32 = 28.0;
const KIND_BUTTON_SIZE: f32 = 34.0;
const SECTION_GAP: f32 = 22.0;
/// Opacity of favorites that are not connected.
const DISCONNECTED_OPACITY: f32 = 0.5;

const KINDS: [(DeviceKind, &str); 4] = [
    (DeviceKind::Headphones, strings::KIND_HEADPHONES),
    (DeviceKind::Speakers, strings::KIND_SPEAKERS),
    (DeviceKind::Display, strings::KIND_DISPLAY),
    (DeviceKind::Generic, strings::KIND_GENERIC),
];

/// State of the settings view.
pub struct SettingsUi {
    pub shortcuts: ShortcutsUi,
}

impl SettingsUi {
    pub fn new() -> Self {
        Self {
            shortcuts: ShortcutsUi::new(),
        }
    }

    /// Draws the view. Returns true when the user asked to go back.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &Snapshot,
        config: &Config,
        binding_errors: &[BindingError],
        events: &mut VecDeque<UiEvent>,
    ) -> bool {
        self.shortcuts.begin_frame(ui, events);

        let mut back = false;
        ui.horizontal(|ui| {
            back = widgets::icon_button(ui, Icon::ChevronLeft, 34.0, strings::BACK).clicked();
            ui.label(
                RichText::new(strings::SETTINGS_TITLE)
                    .size(17.0)
                    .color(theme::TEXT),
            );
        });
        ui.add_space(6.0);

        ScrollArea::vertical()
            .id_salt("settings")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(new_config) = favorites_section(ui, snapshot, config) {
                    events.push_back(UiEvent::ConfigChanged(new_config));
                }
                ui.add_space(SECTION_GAP);
                self.shortcuts
                    .section(ui, snapshot, config, binding_errors, events);
            });

        if !back && self.shortcuts.is_idle() {
            back = ui.input(|input| input.key_pressed(egui::Key::Escape));
        }
        back
    }
}

/// Change requested on a favorites row.
enum FavoriteChange {
    Add(FavoriteOutput),
    Remove(usize),
    MoveUp(usize),
    MoveDown(usize),
    Kind(usize, DeviceKind),
}

/// "Uscite preferite": favorites in order, then the other active outputs.
/// Returns the new configuration when something changed.
fn favorites_section(ui: &mut Ui, snapshot: &Snapshot, config: &Config) -> Option<Config> {
    section_title(ui, strings::FAVORITES_TITLE);
    widgets::message(ui, strings::FAVORITES_HINT, Tone::Info);
    ui.add_space(2.0);

    let favorites = &config.favorite_outputs;
    let full = favorites.len() >= MAX_FAVORITE_OUTPUTS;
    let mut change = None;
    card_frame(theme::CARD).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 6.0;
        if favorites.is_empty() {
            widgets::message(ui, strings::FAVORITES_EMPTY, Tone::Info);
        }
        for (index, favorite) in favorites.iter().enumerate() {
            ui.push_id(("favorite", &favorite.id), |ui| {
                if let Some(c) = favorite_row(ui, snapshot, favorites.len(), index, favorite) {
                    change = Some(c);
                }
            });
        }
        for output in &snapshot.outputs {
            if favorites.iter().any(|favorite| favorite.id == output.id.0) {
                continue;
            }
            ui.push_id(("output", &output.id.0), |ui| {
                if other_output_row(ui, &output.name, full) {
                    change = Some(FavoriteChange::Add(FavoriteOutput {
                        id: output.id.0.clone(),
                        name: output.name.clone(),
                        kind: DeviceKind::guess(&output.name),
                    }));
                }
            });
        }
        if full {
            widgets::message(ui, strings::FAVORITES_FULL, Tone::Info);
        }
    });

    let change = change?;
    let mut new_config = config.clone();
    let list = &mut new_config.favorite_outputs;
    match change {
        FavoriteChange::Add(favorite) => list.push(favorite),
        FavoriteChange::Remove(index) => {
            list.remove(index);
        }
        FavoriteChange::MoveUp(index) => list.swap(index - 1, index),
        FavoriteChange::MoveDown(index) => list.swap(index, index + 1),
        FavoriteChange::Kind(index, kind) => list[index].kind = kind,
    }
    Some(new_config)
}

/// Row of a favorite: icon kind picker, name, order arrows and switch.
fn favorite_row(
    ui: &mut Ui,
    snapshot: &Snapshot,
    count: usize,
    index: usize,
    favorite: &FavoriteOutput,
) -> Option<FavoriteChange> {
    let live = snapshot.output_name(&DeviceId(favorite.id.clone()));
    let mut change = None;
    ui.horizontal(|ui| {
        if let Some(kind) = kind_picker(ui, favorite.kind) {
            change = Some(FavoriteChange::Kind(index, kind));
        }
        let buttons = ROW_BUTTON_SIZE * 2.0 + 36.0 + ui.spacing().item_spacing.x * 3.0;
        let text_width = (ui.available_width() - buttons).max(40.0);
        ui.allocate_ui_with_layout(
            vec2(text_width, KIND_BUTTON_SIZE),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(text_width);
                ui.spacing_mut().item_spacing.y = 0.0;
                if live.is_none() {
                    ui.multiply_opacity(DISCONNECTED_OPACITY);
                }
                ui.add(
                    Label::new(RichText::new(live.unwrap_or(&favorite.name)).color(theme::TEXT))
                        .truncate(),
                );
                if live.is_none() {
                    ui.add(
                        Label::new(
                            RichText::new(strings::OUTPUT_DISCONNECTED)
                                .small()
                                .color(theme::TEXT_WEAK),
                        )
                        .truncate(),
                    );
                }
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::toggle(ui, true)
                .on_hover_text(strings::FAVORITE_TOGGLE)
                .clicked()
            {
                change = Some(FavoriteChange::Remove(index));
            }
            ui.spacing_mut().item_spacing.x = 2.0;
            let down = ui
                .add_enabled_ui(index + 1 < count, |ui| {
                    widgets::icon_button(ui, Icon::ChevronDown, ROW_BUTTON_SIZE, strings::MOVE_DOWN)
                })
                .inner;
            if down.clicked() {
                change = Some(FavoriteChange::MoveDown(index));
            }
            let up = ui
                .add_enabled_ui(index > 0, |ui| {
                    widgets::icon_button(ui, Icon::ChevronUp, ROW_BUTTON_SIZE, strings::MOVE_UP)
                })
                .inner;
            if up.clicked() {
                change = Some(FavoriteChange::MoveUp(index));
            }
        });
    });
    change
}

/// Row of an active output that is not a favorite. Returns true when the
/// switch was turned on.
fn other_output_row(ui: &mut Ui, name: &str, full: bool) -> bool {
    let mut add = false;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            vec2(KIND_BUTTON_SIZE, KIND_BUTTON_SIZE),
            egui::Sense::hover(),
        );
        widgets::paint_icon(
            ui.painter(),
            rect.shrink(KIND_BUTTON_SIZE * 0.12),
            Icon::device(DeviceKind::guess(name)),
            theme::TEXT_FAINT,
        );
        let text_width = (ui.available_width() - 36.0 - ui.spacing().item_spacing.x).max(40.0);
        ui.allocate_ui_with_layout(
            vec2(text_width, KIND_BUTTON_SIZE),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add(Label::new(RichText::new(name).color(theme::TEXT_WEAK)).truncate());
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let response = ui
                .add_enabled_ui(!full, |ui| widgets::toggle(ui, false))
                .inner;
            if full {
                response.on_disabled_hover_text(strings::FAVORITES_FULL);
            } else if response.on_hover_text(strings::FAVORITE_TOGGLE).clicked() {
                add = true;
            }
        });
    });
    add
}

/// Button with the current device icon; opens a menu of the four kinds.
fn kind_picker(ui: &mut Ui, current: DeviceKind) -> Option<DeviceKind> {
    let response = widgets::icon_button(
        ui,
        Icon::device(current),
        KIND_BUTTON_SIZE,
        strings::FAVORITE_KIND,
    );
    let mut choice = None;
    widgets::chip_menu(&response, |ui| {
        for (kind, label) in KINDS {
            if widgets::menu_item_icon(ui, kind == current, Some(Icon::device(kind)), label)
                .clicked()
                && kind != current
            {
                choice = Some(kind);
            }
        }
    });
    choice
}
