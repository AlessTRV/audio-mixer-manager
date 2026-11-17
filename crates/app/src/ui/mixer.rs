//! Mixer view: header with the default output, one card per application.

use std::collections::HashMap;
use std::sync::Arc;

use audio_core::device::DeviceId;
use audio_core::process::IconRgba;
use eframe::egui::{
    self, Align, Align2, ColorImage, CornerRadius, FontId, Frame, Image, Label, Layout, Rect,
    RichText, ScrollArea, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2, emath, vec2,
};

use super::widgets::{self, Icon, OutputMark};
use super::{strings, theme};
use crate::config::{Config, DeviceKind, MAX_FAVORITE_OUTPUTS};
use crate::controller::Shared;
use crate::model::{AppEntry, Command, Snapshot};

const BADGE_SIZE: f32 = 40.0;
const ICON_SIZE: f32 = 24.0;
/// Opacity of cards of applications that are not playing, and under the pointer.
const INACTIVE_OPACITY: f32 = 0.55;
const INACTIVE_HOVER_OPACITY: f32 = 0.85;
/// Width reserved for the volume percentage next to the slider.
const PERCENT_WIDTH: f32 = 40.0;
const MUTE_BUTTON_SIZE: f32 = 34.0;
const SMALL_BUTTON_SIZE: f32 = 28.0;
const OUTPUT_BUTTON_SIZE: f32 = 30.0;
const OUTPUT_BUTTON_GAP: f32 = 2.0;
/// Opacity of the card menu button while the card is not hovered.
const MENU_REST_OPACITY: f32 = 0.25;
const MIN_SLIDER_WIDTH: f32 = 60.0;
const CARD_GAP: f32 = 10.0;
/// Seconds over which the peak meter follows a new value.
const PEAK_SMOOTHING: f32 = 0.08;

/// What the mixer asks the window to do.
pub enum MixerRequest {
    OpenSettings,
    /// Open the shortcut editor with a mute shortcut for this application key.
    MuteShortcut(String),
}

/// Texture of an application icon, kept while the snapshot carries the same
/// [`IconRgba`].
struct CachedIcon {
    source: Arc<IconRgba>,
    /// `None` when the pixels are malformed: never retried for this source.
    texture: Option<TextureHandle>,
}

/// Volume the user is changing; shown instead of the snapshot value, which
/// lags behind the commands.
struct VolumeEdit {
    app: String,
    percent: f32,
    dragging: bool,
    /// Snapshot generation when the last command was sent: once a newer
    /// snapshot arrives (and the slider is released) it reflects the change.
    sent_generation: u64,
}

/// State of the mixer view that survives between frames.
pub struct MixerUi {
    icons: HashMap<String, CachedIcon>,
    volume_edit: Option<VolumeEdit>,
}

impl MixerUi {
    #[must_use]
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            volume_edit: None,
        }
    }

    /// Draws header and cards in `ui` (the central panel of the window).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &Snapshot,
        shared: &Shared,
        config: &Config,
    ) -> Option<MixerRequest> {
        self.sync_icons(ui.ctx(), snapshot);
        self.expire_volume_edit(snapshot);
        let outputs = card_outputs(config, snapshot);

        let mut request = None;
        if header(ui, snapshot, shared) {
            request = Some(MixerRequest::OpenSettings);
        }
        ui.add_space(6.0);
        if snapshot.apps.is_empty() {
            empty_state(ui);
            return request;
        }
        ScrollArea::vertical()
            .id_salt("apps")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in &snapshot.apps {
                    ui.push_id(&entry.key, |ui| {
                        if self.app_card(ui, entry, snapshot, shared, &outputs) {
                            request = Some(MixerRequest::MuteShortcut(entry.key.clone()));
                        }
                    });
                    ui.add_space(CARD_GAP);
                }
            });
        request
    }

    /// Creates textures for new icons and drops those of applications that
    /// are gone (or whose icon changed).
    fn sync_icons(&mut self, ctx: &egui::Context, snapshot: &Snapshot) {
        self.icons.retain(|key, cached| {
            snapshot.apps.iter().any(|app| {
                &app.key == key
                    && app
                        .icon
                        .as_ref()
                        .is_some_and(|icon| Arc::ptr_eq(icon, &cached.source))
            })
        });
        for app in &snapshot.apps {
            let Some(icon) = &app.icon else { continue };
            if self.icons.contains_key(&app.key) {
                continue;
            }
            let texture = load_icon(ctx, &app.key, icon);
            if texture.is_none() {
                tracing::warn!(
                    "malformed icon for {}: {}x{}, {} bytes",
                    app.key,
                    icon.width,
                    icon.height,
                    icon.rgba.len()
                );
            }
            self.icons.insert(
                app.key.clone(),
                CachedIcon {
                    source: Arc::clone(icon),
                    texture,
                },
            );
        }
    }

    fn expire_volume_edit(&mut self, snapshot: &Snapshot) {
        let expired = self.volume_edit.as_ref().is_some_and(|edit| {
            let gone = !snapshot.apps.iter().any(|app| app.key == edit.app);
            gone || (!edit.dragging && snapshot.generation > edit.sent_generation)
        });
        if expired {
            self.volume_edit = None;
        }
    }

    /// Returns true when "create mute shortcut" was chosen.
    fn app_card(
        &mut self,
        ui: &mut Ui,
        entry: &AppEntry,
        snapshot: &Snapshot,
        shared: &Shared,
        outputs: &[CardOutput],
    ) -> bool {
        // Hover is known only after the card is laid out: use last frame's rect.
        let card_id = ui.scope_id().with("card");
        let last_rect: Option<Rect> = ui.data(|data| data.get_temp(card_id));
        let hovered = last_rect.is_some_and(|rect| ui.rect_contains_pointer(rect));
        let t = ui
            .ctx()
            .animate_bool_with_time(card_id, hovered, theme::FADE);

        let mut shortcut_clicked = false;
        let rect = ui
            .scope(|ui| {
                if !entry.active {
                    ui.multiply_opacity(emath::lerp(INACTIVE_OPACITY..=INACTIVE_HOVER_OPACITY, t));
                }
                Frame::new()
                    .fill(theme::CARD.lerp_to_gamma(theme::CARD_HOVER, t))
                    .stroke(Stroke::new(1.0, theme::CARD_STROKE))
                    .corner_radius(CornerRadius::same(theme::CARD_RADIUS))
                    .inner_margin(theme::CARD_MARGIN)
                    .shadow(theme::CARD_SHADOW)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing = vec2(12.0, 10.0);
                        shortcut_clicked = self.card_title(ui, entry, snapshot, shared, outputs, t);
                        ui.horizontal(|ui| {
                            let muted = entry.muted;
                            let tooltip = if muted {
                                strings::UNMUTE
                            } else {
                                strings::MUTE
                            };
                            if widgets::mute_button(ui, muted, MUTE_BUTTON_SIZE, tooltip).clicked()
                            {
                                shared.send(Command::SetMuted {
                                    app: entry.key.clone(),
                                    muted: !muted,
                                });
                            }
                            self.volume_slider(ui, entry, snapshot, shared);
                        });
                    })
                    .response
                    .rect
            })
            .inner;
        ui.data_mut(|data| data.insert_temp(card_id, rect));
        shortcut_clicked
    }

    /// Icon badge, name and output, favorite output buttons and the card
    /// menu. `hover` is the animated hover factor of the card. Returns true
    /// when "create mute shortcut" was chosen.
    fn card_title(
        &self,
        ui: &mut Ui,
        entry: &AppEntry,
        snapshot: &Snapshot,
        shared: &Shared,
        outputs: &[CardOutput],
        hover: f32,
    ) -> bool {
        let mut clicked = false;
        let outputs = if entry.is_system { &[][..] } else { outputs };
        ui.horizontal(|ui| {
            self.icon(ui, entry);
            let spacing = ui.spacing().item_spacing.x;
            #[expect(clippy::cast_precision_loss, reason = "at most a few buttons")]
            let buttons_width = outputs.len() as f32 * (OUTPUT_BUTTON_SIZE + OUTPUT_BUTTON_GAP);
            let text_width =
                (ui.available_width() - SMALL_BUTTON_SIZE - buttons_width - spacing * 2.0)
                    .max(40.0);
            ui.allocate_ui_with_layout(
                vec2(text_width, BADGE_SIZE),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(text_width);
                    ui.spacing_mut().item_spacing.y = 1.0;
                    let name = if entry.is_system {
                        strings::SYSTEM_SOUNDS
                    } else {
                        entry.display_name.as_str()
                    };
                    ui.add(
                        Label::new(RichText::new(name).size(14.5).color(theme::TEXT)).truncate(),
                    );
                    ui.add(
                        Label::new(
                            RichText::new(output_caption(entry, snapshot))
                                .small()
                                .color(theme::TEXT_WEAK),
                        )
                        .truncate(),
                    );
                },
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = card_menu(ui, entry, snapshot, shared, hover);
                ui.spacing_mut().item_spacing.x = OUTPUT_BUTTON_GAP;
                output_buttons(ui, entry, snapshot, shared, outputs);
            });
        });
        clicked
    }

    fn icon(&self, ui: &mut Ui, entry: &AppEntry) {
        let badge = widgets::badge_rect(ui, BADGE_SIZE);
        let inner = Rect::from_center_size(badge.center(), Vec2::splat(ICON_SIZE));
        if let Some(texture) = self.icons.get(&entry.key).and_then(|c| c.texture.as_ref()) {
            Image::from_texture(texture)
                .fit_to_exact_size(inner.size())
                .paint_at(ui, inner);
        } else if entry.is_system {
            widgets::paint_icon(ui.painter(), inner, Icon::Speaker, theme::TEXT_WEAK);
        } else {
            let initial: String = entry
                .display_name
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_default();
            widgets::centered_text(ui, badge, &initial, 16.0, theme::TEXT_WEAK);
        }
    }

    fn volume_slider(
        &mut self,
        ui: &mut Ui,
        entry: &AppEntry,
        snapshot: &Snapshot,
        shared: &Shared,
    ) {
        let percent = self
            .volume_edit
            .as_ref()
            .filter(|edit| edit.app == entry.key)
            .map_or(entry.volume * 100.0, |edit| edit.percent);
        let peak = ui.ctx().animate_value_with_time(
            ui.scope_id().with("peak"),
            entry.peak,
            PEAK_SMOOTHING,
        );

        let width = (ui.available_width() - PERCENT_WIDTH - ui.spacing().item_spacing.x)
            .max(MIN_SLIDER_WIDTH);
        let output = widgets::volume_slider(
            ui,
            ui.scope_id().with("volume"),
            width,
            percent,
            peak,
            entry.muted,
        );
        output.response.on_hover_text(strings::VOLUME_TOOLTIP);

        let shown = output.changed.unwrap_or(percent);
        if let Some(value) = output.changed {
            shared.send(Command::SetVolume {
                app: entry.key.clone(),
                volume: value / 100.0,
            });
            self.volume_edit = Some(VolumeEdit {
                app: entry.key.clone(),
                percent: value,
                dragging: output.dragging,
                sent_generation: snapshot.generation,
            });
        } else if let Some(edit) = self
            .volume_edit
            .as_mut()
            .filter(|edit| edit.app == entry.key)
        {
            edit.dragging = output.dragging;
        }

        let (rect, _) = ui.allocate_exact_size(vec2(PERCENT_WIDTH, 24.0), Sense::hover());
        ui.painter().text(
            rect.right_center(),
            Align2::RIGHT_CENTER,
            format!("{shown:.0}{}", strings::PERCENT_SUFFIX),
            FontId::proportional(13.0),
            theme::TEXT_WEAK,
        );
    }
}

/// Uploads an icon as a texture; `None` if the pixel buffer does not match
/// the declared size.
fn load_icon(ctx: &egui::Context, key: &str, icon: &IconRgba) -> Option<TextureHandle> {
    let width = usize::try_from(icon.width).ok()?;
    let height = usize::try_from(icon.height).ok()?;
    let expected = width.checked_mul(height)?.checked_mul(4)?;
    if width == 0 || height == 0 || expected != icon.rgba.len() {
        return None;
    }
    let image = ColorImage::from_rgba_unmultiplied([width, height], &icon.rgba);
    Some(ctx.load_texture(format!("app-icon:{key}"), image, TextureOptions::LINEAR))
}

/// Title, default output chip and the button to the shortcut view. Returns
/// true when that button was clicked.
fn header(ui: &mut Ui, snapshot: &Snapshot, shared: &Shared) -> bool {
    let mut open_shortcuts = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(strings::APP_TITLE)
                .size(17.0)
                .color(theme::TEXT),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            open_shortcuts =
                widgets::icon_button(ui, Icon::Gear, 34.0, strings::OPEN_SETTINGS).clicked();
            default_output_chip(ui, snapshot, shared);
        });
    });
    open_shortcuts
}

fn default_output_chip(ui: &mut Ui, snapshot: &Snapshot, shared: &Shared) {
    let selected = snapshot
        .default_output
        .as_ref()
        .and_then(|id| snapshot.output_name(id))
        .unwrap_or(strings::NO_OUTPUTS);
    let mut choice: Option<DeviceId> = None;
    ui.add_enabled_ui(!snapshot.outputs.is_empty(), |ui| {
        let max_width = ui.available_width().min(260.0);
        let response = widgets::chip(ui, selected, false, max_width);
        widgets::chip_menu(&response, |ui| {
            menu_caption(ui, strings::DEFAULT_OUTPUT);
            for output in &snapshot.outputs {
                let is_selected = snapshot.default_output.as_ref() == Some(&output.id);
                if widgets::menu_item(ui, is_selected, &output.name).clicked() && !is_selected {
                    choice = Some(output.id.clone());
                }
            }
        });
        response.on_hover_text(strings::DEFAULT_OUTPUT_TOOLTIP);
    });
    if let Some(id) = choice {
        shared.send(Command::SetDefaultOutput(id));
    }
}

/// Small caption at the top of a menu.
pub fn menu_caption(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(RichText::new(text).small().color(theme::TEXT_FAINT));
    });
}

fn empty_state(ui: &mut Ui) {
    ui.add_space((ui.available_height() * 0.3).max(0.0));
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 28.0, theme::CARD);
        widgets::paint_icon(
            ui.painter(),
            rect.shrink(12.0),
            Icon::Speaker,
            theme::TEXT_FAINT,
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(strings::EMPTY_TITLE)
                .size(15.0)
                .color(theme::TEXT),
        );
        ui.label(RichText::new(strings::EMPTY_HINT).color(theme::TEXT_WEAK));
    });
}

/// Effective output of an application: the routed device, or the system
/// default with its name.
fn output_caption(entry: &AppEntry, snapshot: &Snapshot) -> String {
    if let Some(id) = &entry.routed_to {
        return snapshot
            .output_name(id)
            .unwrap_or(strings::UNKNOWN_OUTPUT)
            .to_owned();
    }
    match snapshot
        .default_output
        .as_ref()
        .and_then(|id| snapshot.output_name(id))
    {
        Some(name) => format!("{} ({name})", strings::FOLLOWS_DEFAULT),
        None => strings::FOLLOWS_DEFAULT.to_owned(),
    }
}

/// One output button of the cards.
pub struct CardOutput {
    id: DeviceId,
    name: String,
    kind: DeviceKind,
    connected: bool,
}

/// Outputs shown on every card: the favorites in their order, or the first
/// active outputs when there are none.
fn card_outputs(config: &Config, snapshot: &Snapshot) -> Vec<CardOutput> {
    if config.favorite_outputs.is_empty() {
        return snapshot
            .outputs
            .iter()
            .take(MAX_FAVORITE_OUTPUTS)
            .map(|output| CardOutput {
                id: output.id.clone(),
                name: output.name.clone(),
                kind: DeviceKind::guess(&output.name),
                connected: true,
            })
            .collect();
    }
    config
        .favorite_outputs
        .iter()
        .take(MAX_FAVORITE_OUTPUTS)
        .map(|favorite| {
            let id = DeviceId(favorite.id.clone());
            let live = snapshot.output_name(&id);
            CardOutput {
                name: live.unwrap_or(&favorite.name).to_owned(),
                connected: live.is_some(),
                id,
                kind: favorite.kind,
            }
        })
        .collect()
}

/// One round button per card output, in a right-to-left ui. Clicking moves
/// the app there; clicking its explicit output again returns to the default.
fn output_buttons(
    ui: &mut Ui,
    entry: &AppEntry,
    snapshot: &Snapshot,
    shared: &Shared,
    outputs: &[CardOutput],
) {
    let mut choice: Option<Option<DeviceId>> = None;
    // Right to left: iterate backwards so the order on screen is the
    // configured one.
    for output in outputs.iter().rev() {
        let mark = if entry.routed_to.as_ref() == Some(&output.id) {
            OutputMark::Assigned
        } else if entry.routed_to.is_none() && snapshot.default_output.as_ref() == Some(&output.id)
        {
            OutputMark::Default
        } else {
            OutputMark::None
        };
        let enabled = output.connected && snapshot.routing_available;
        let response = ui
            .push_id(&output.id.0, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    widgets::output_button(ui, Icon::device(output.kind), OUTPUT_BUTTON_SIZE, mark)
                })
                .inner
            })
            .inner;
        let tooltip = match mark {
            OutputMark::Assigned => {
                format!("{} · {}", output.name, strings::OUTPUT_CLICK_TO_RESET)
            }
            OutputMark::Default => {
                format!("{} · {}", output.name, strings::OUTPUT_FOLLOWS_DEFAULT)
            }
            OutputMark::None => output.name.clone(),
        };
        if !snapshot.routing_available {
            response.on_disabled_hover_text(strings::ROUTING_UNAVAILABLE);
        } else if !output.connected {
            response.on_disabled_hover_text(format!(
                "{} · {}",
                output.name,
                strings::OUTPUT_DISCONNECTED
            ));
        } else if response.on_hover_text(tooltip).clicked() {
            choice = Some((mark != OutputMark::Assigned).then(|| output.id.clone()));
        }
    }
    if let Some(output) = choice {
        shared.send(Command::SetAppOutput {
            app: entry.key.clone(),
            output,
        });
    }
}

/// The "⋯" button of a card, barely visible until the card is hovered, and
/// its menu: mute shortcut and the full output list. Returns true when
/// "create mute shortcut" was chosen.
fn card_menu(
    ui: &mut Ui,
    entry: &AppEntry,
    snapshot: &Snapshot,
    shared: &Shared,
    hover: f32,
) -> bool {
    let rest = emath::lerp(MENU_REST_OPACITY..=1.0, hover);
    let response =
        widgets::icon_button_dim(ui, Icon::Dots, SMALL_BUTTON_SIZE, strings::CARD_MENU, rest);
    let mut shortcut = false;
    let mut choice: Option<Option<DeviceId>> = None;
    widgets::chip_menu(&response, |ui| {
        if widgets::menu_item_icon(
            ui,
            false,
            Some(Icon::Keyboard),
            strings::CREATE_MUTE_SHORTCUT,
        )
        .clicked()
        {
            shortcut = true;
        }
        if entry.is_system {
            return;
        }
        ui.add_space(4.0);
        menu_caption(ui, strings::OTHER_OUTPUTS);
        ui.add_enabled_ui(snapshot.routing_available, |ui| {
            let follows_default = entry.routed_to.is_none();
            if widgets::menu_item(ui, follows_default, strings::SYSTEM_DEFAULT_OUTPUT).clicked()
                && !follows_default
            {
                choice = Some(None);
            }
            for output in &snapshot.outputs {
                let selected = entry.routed_to.as_ref() == Some(&output.id);
                let icon = Icon::device(DeviceKind::guess(&output.name));
                if widgets::menu_item_icon(ui, selected, Some(icon), &output.name).clicked()
                    && !selected
                {
                    choice = Some(Some(output.id.clone()));
                }
            }
        })
        .response
        .on_disabled_hover_text(strings::ROUTING_UNAVAILABLE);
    });
    if let Some(output) = choice {
        shared.send(Command::SetAppOutput {
            app: entry.key.clone(),
            output,
        });
    }
    shortcut
}
