//! Start with Windows through the per-user `Run` registry key.
//!
//! The entry launches the current executable with `--hidden`, so the app
//! starts in the tray. The registry is the source of truth: nothing is stored
//! in the configuration file.

use anyhow::Context as _;
use windows_registry::CURRENT_USER;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "AudioMixerManager";

/// Whether the `Run` entry exists and points at this executable.
#[must_use]
pub fn is_enabled() -> bool {
    let Ok(expected) = command_line() else {
        return false;
    };
    CURRENT_USER
        .open(RUN_KEY)
        .and_then(|key| key.get_string(VALUE_NAME))
        .is_ok_and(|value| value.eq_ignore_ascii_case(&expected))
}

/// Adds or removes the `Run` entry.
pub fn set_enabled(enabled: bool) -> anyhow::Result<()> {
    let key = CURRENT_USER
        .create(RUN_KEY)
        .context("opening the Run key")?;
    if enabled {
        key.set_string(VALUE_NAME, command_line()?)
            .context("writing the Run entry")?;
    } else {
        // Nothing to remove when the entry is already absent.
        if key.get_string(VALUE_NAME).is_ok() {
            key.remove_value(VALUE_NAME)
                .context("removing the Run entry")?;
        }
    }
    Ok(())
}

/// `"<exe path>" --hidden`.
fn command_line() -> anyhow::Result<String> {
    let exe = std::env::current_exe().context("locating the executable")?;
    Ok(format!("\"{}\" {}", exe.display(), crate::HIDDEN_FLAG))
}
