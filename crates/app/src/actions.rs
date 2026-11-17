//! User-bindable actions, shared by hotkeys and (later) the tray.

use audio_core::device::DeviceId;
use serde::{Deserialize, Serialize};

use crate::model::Command;

/// How "move everything to output X" treats the per-application assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveMode {
    /// X becomes the system default and every per-app assignment is removed:
    /// everything, including apps started later, plays on X.
    #[default]
    Reset,
    /// Only apps playing right now are assigned to X; the system default stays.
    ActiveOnly,
    /// X becomes the system default; apps with their own assignment stay put.
    KeepRules,
}

/// Something a hotkey can trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Shows the mixer window, or hides it when it is already in front.
    ToggleWindow,
    /// Mutes or unmutes every session of an application.
    ToggleMute {
        /// Lowercase executable name, as in `AppEntry::key`.
        app: String,
    },
    /// Moves all audio to one output.
    MoveAll {
        /// Endpoint id of the target output.
        output: String,
        /// Name shown when the device is not connected.
        #[serde(default)]
        output_name: String,
        #[serde(default)]
        mode: MoveMode,
    },
}

impl Action {
    /// The controller command for this action; `None` for actions the UI
    /// thread handles itself.
    #[must_use]
    pub fn to_command(&self) -> Option<Command> {
        match self {
            Self::ToggleWindow => None,
            Self::ToggleMute { app } => Some(Command::ToggleMuted { app: app.clone() }),
            Self::MoveAll { output, mode, .. } => Some(Command::MoveAll {
                output: DeviceId(output.clone()),
                mode: *mode,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_window_is_handled_by_the_ui() {
        assert_eq!(Action::ToggleWindow.to_command(), None);
    }

    #[test]
    fn move_all_maps_to_command() {
        let action = Action::MoveAll {
            output: "{0.0.0.00000000}.{x}".into(),
            output_name: "Cuffie".into(),
            mode: MoveMode::ActiveOnly,
        };
        assert_eq!(
            action.to_command(),
            Some(Command::MoveAll {
                output: DeviceId("{0.0.0.00000000}.{x}".into()),
                mode: MoveMode::ActiveOnly,
            })
        );
    }
}
