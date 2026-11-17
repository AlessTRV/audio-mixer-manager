//! Plain data shared between the controller thread and the UI.
//!
//! Nothing here holds a COM object: the snapshot is rebuilt by the controller
//! and read by the UI on another thread.

use std::sync::Arc;

use audio_core::device::DeviceId;
use audio_core::process::IconRgba;

use crate::actions::MoveMode;

/// Key of the system sounds entry in [`AppEntry::key`].
pub const SYSTEM_SOUNDS_KEY: &str = "system sounds";

/// An active output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub id: DeviceId,
    pub name: String,
}

/// One application with audio sessions, all its processes and devices merged.
#[derive(Debug, Clone)]
pub struct AppEntry {
    /// Stable identifier used in commands and rules: lowercase executable
    /// name (`"spotify.exe"`) or [`SYSTEM_SOUNDS_KEY`].
    pub key: String,
    /// Human readable name (file description of the executable).
    pub display_name: String,
    pub icon: Option<Arc<IconRgba>>,
    pub pids: Vec<u32>,
    /// True only when every session of the application is muted.
    pub muted: bool,
    /// Highest session volume, `0.0..=1.0`.
    pub volume: f32,
    /// Highest current peak, `0.0..=1.0`. Updated only while metering is on.
    pub peak: f32,
    /// At least one session is playing.
    pub active: bool,
    /// Devices the application currently has sessions on.
    pub devices: Vec<DeviceId>,
    /// Per-application output assignment; `None` follows the system default.
    pub routed_to: Option<DeviceId>,
    pub is_system: bool,
}

/// Everything the UI shows. Rebuilt by the controller after every change.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub outputs: Vec<Output>,
    pub default_output: Option<DeviceId>,
    /// Active applications first, then by name; system sounds last.
    pub apps: Vec<AppEntry>,
    /// False when per-application routing is not available on this Windows build.
    pub routing_available: bool,
    /// Last error, for a status line. Cleared by the next successful command.
    pub error: Option<String>,
    /// Incremented at every publish.
    pub generation: u64,
}

impl Snapshot {
    /// Name of an output, if it is still active.
    #[must_use]
    pub fn output_name(&self, id: &DeviceId) -> Option<&str> {
        self.outputs
            .iter()
            .find(|output| &output.id == id)
            .map(|output| output.name.as_str())
    }
}

/// Requests the UI, tray and hotkeys send to the controller.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SetMuted {
        app: String,
        muted: bool,
    },
    ToggleMuted {
        app: String,
    },
    SetVolume {
        app: String,
        volume: f32,
    },
    /// `None` makes the application follow the system default again.
    SetAppOutput {
        app: String,
        output: Option<DeviceId>,
    },
    SetDefaultOutput(DeviceId),
    /// Moves all audio to `output`; see [`MoveMode`] for the variants.
    MoveAll {
        output: DeviceId,
        mode: MoveMode,
    },
    /// Turns peak metering on while the window is visible, off when hidden.
    SetMetering(bool),
    #[expect(dead_code, reason = "sent by the tray and hotkeys (M3)")]
    Refresh,
    Shutdown,
}

/// A hotkey binding that could not be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingError {
    /// Index in `Config::bindings`.
    pub index: usize,
    /// Italian message for the settings editor.
    pub message: String,
}
