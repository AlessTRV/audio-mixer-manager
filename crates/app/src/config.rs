//! User configuration, stored as TOML in `%APPDATA%\AudioMixerManager\config.toml`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::actions::Action;

/// Current schema version.
pub const CONFIG_VERSION: u32 = 1;

/// Folder under `%APPDATA%`.
const APP_DIR: &str = "AudioMixerManager";
/// File name inside [`APP_DIR`].
const FILE_NAME: &str = "config.toml";

/// Whole configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    /// Global hotkeys, in the order shown in the editor.
    pub bindings: Vec<Binding>,
    /// Outputs shown as one-click buttons on every app card, in order (at most
    /// [`MAX_FAVORITE_OUTPUTS`]). Empty: the cards show the active outputs.
    pub favorite_outputs: Vec<FavoriteOutput>,
}

/// Maximum number of favorite outputs.
pub const MAX_FAVORITE_OUTPUTS: usize = 4;

/// An output pinned to the app cards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteOutput {
    /// Endpoint id.
    pub id: String,
    /// Name shown when the device is not connected.
    pub name: String,
    #[serde(default)]
    pub kind: DeviceKind,
}

/// Icon drawn for an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Headphones,
    Speakers,
    Display,
    #[default]
    Generic,
}

impl DeviceKind {
    /// Best guess from a device name (Italian and English Windows names).
    #[must_use]
    pub fn guess(name: &str) -> Self {
        let name = name.to_lowercase();
        let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
        if has(&[
            "cuffi",
            "headphone",
            "headset",
            "auricolar",
            "earbud",
            "airpods",
        ]) {
            Self::Headphones
        } else if has(&[
            "monitor",
            "display",
            "hdmi",
            "displayport",
            "tv",
            "nvidia",
            "samsung",
            "lg ",
            "acer",
            "dell",
            "asus",
        ]) {
            Self::Display
        } else if has(&[
            "altoparlant",
            "speaker",
            "casse",
            "realtek",
            "digital",
            "spdif",
            "s/pdif",
        ]) {
            Self::Speakers
        } else {
            Self::Generic
        }
    }
}

/// One global hotkey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    /// Key combination in `global-hotkey` syntax, e.g. `"Ctrl+Alt+KeyM"`.
    pub keys: String,
    pub action: Action,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            bindings: vec![Binding {
                keys: "Ctrl+Alt+KeyM".to_owned(),
                action: Action::ToggleWindow,
            }],
            favorite_outputs: Vec::new(),
        }
    }
}

/// `%APPDATA%\AudioMixerManager\config.toml`; `None` when `APPDATA` is unset.
#[must_use]
pub fn config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join(APP_DIR).join(FILE_NAME))
}

impl Config {
    /// Reads the user configuration.
    ///
    /// A missing file gives the default silently. An unreadable or invalid
    /// file is renamed to `config.toml.broken-<unix secs>` and the default is
    /// returned with an Italian message for the user. A file written by a
    /// newer version is left untouched and the default is returned with a
    /// message.
    pub fn load() -> (Self, Option<String>) {
        match config_path() {
            Some(path) => Self::load_from(&path),
            None => (
                Self::default(),
                Some("Cartella APPDATA non trovata: uso le impostazioni predefinite.".to_owned()),
            ),
        }
    }

    /// Writes the user configuration atomically.
    ///
    /// # Errors
    /// When `APPDATA` is unset or the file cannot be written.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path().context("APPDATA is not set")?;
        self.save_to(&path)
    }

    /// [`Config::load`] on an explicit path.
    pub fn load_from(path: &Path) -> (Self, Option<String>) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return (Self::default(), None),
            Err(err) => {
                tracing::warn!("cannot read {}: {err}", path.display());
                return (Self::default(), Some(set_aside(path, "illeggibile")));
            }
        };

        // Syntax first, then the version, then the schema: a newer file may
        // have a shape this version does not understand, and must not be
        // treated as broken.
        let table: toml::Table = match toml::from_str(&text) {
            Ok(table) => table,
            Err(err) => {
                tracing::warn!("invalid TOML in {}: {err}", path.display());
                return (Self::default(), Some(set_aside(path, "non valido")));
            }
        };
        let version = table
            .get("version")
            .and_then(toml::Value::as_integer)
            .unwrap_or(i64::from(CONFIG_VERSION));
        if version > i64::from(CONFIG_VERSION) {
            tracing::warn!(version, "config written by a newer version; ignoring it");
            return (
                Self::default(),
                Some(format!(
                    "Il file delle impostazioni è di una versione più recente dell'app \
                     (versione {version}): uso le impostazioni predefinite e non lo modifico."
                )),
            );
        }

        match toml::from_str::<Self>(&text) {
            Ok(mut config) => {
                config.version = CONFIG_VERSION;
                (config, None)
            }
            Err(err) => {
                tracing::warn!("invalid config in {}: {err}", path.display());
                (Self::default(), Some(set_aside(path, "non valido")))
            }
        }
    }

    /// [`Config::save`] on an explicit path: creates the folder, writes
    /// `<name>.tmp` next to the file, then renames it over the file.
    ///
    /// # Errors
    /// When serialisation or any file operation fails.
    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serialising config")?;
        let tmp = with_suffix(path, ".tmp");
        fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(())
    }
}

/// Renames a broken config out of the way and returns the user message.
fn set_aside(path: &Path, what: &str) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let broken = with_suffix(path, &format!(".broken-{secs}"));
    let name = broken
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    match fs::rename(path, &broken) {
        Ok(()) => format!(
            "Il file delle impostazioni era {what}: l'ho rinominato in {name} e uso le \
             impostazioni predefinite."
        ),
        Err(err) => {
            tracing::warn!("cannot rename {}: {err}", path.display());
            format!("Il file delle impostazioni era {what}: uso le impostazioni predefinite.")
        }
    }
}

/// `path` with `suffix` appended to the file name (`config.toml.tmp`).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::actions::MoveMode;

    /// A fresh, empty directory under the system temp dir.
    fn temp_dir(tag: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "audiomixermanager-test-{}-{tag}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn full_config() -> Config {
        Config {
            version: CONFIG_VERSION,
            bindings: vec![
                Binding {
                    keys: "Ctrl+Alt+KeyM".into(),
                    action: Action::ToggleWindow,
                },
                Binding {
                    keys: "Ctrl+Alt+KeyS".into(),
                    action: Action::ToggleMute {
                        app: "spotify.exe".into(),
                    },
                },
                Binding {
                    keys: "Ctrl+Shift+Digit1".into(),
                    action: Action::MoveAll {
                        output: "{0.0.0.00000000}.{a}".into(),
                        output_name: "Cuffie".into(),
                        mode: MoveMode::Reset,
                    },
                },
                Binding {
                    keys: "Ctrl+Shift+Digit2".into(),
                    action: Action::MoveAll {
                        output: "{0.0.0.00000000}.{b}".into(),
                        output_name: "Altoparlanti".into(),
                        mode: MoveMode::ActiveOnly,
                    },
                },
                Binding {
                    keys: "F13".into(),
                    action: Action::MoveAll {
                        output: "{0.0.0.00000000}.{c}".into(),
                        output_name: String::new(),
                        mode: MoveMode::KeepRules,
                    },
                },
            ],
            favorite_outputs: vec![FavoriteOutput {
                id: "{0.0.0.00000000}.{b}".into(),
                name: "Cuffie (Razer USB Sound Card)".into(),
                kind: DeviceKind::Headphones,
            }],
        }
    }

    #[test]
    fn round_trip_all_actions() {
        let config = full_config();
        let text = toml::to_string_pretty(&config).expect("serialise");
        let back: Config = toml::from_str(&text).expect("parse");
        assert_eq!(back, config);
    }

    #[test]
    fn format_is_readable() {
        let text = toml::to_string_pretty(&full_config()).expect("serialise");
        assert!(text.contains(r#"type = "move_all""#), "{text}");
        assert!(text.contains(r#"mode = "active_only""#), "{text}");
        assert!(text.contains(r#"type = "toggle_window""#), "{text}");
    }

    #[test]
    fn save_then_load() {
        let dir = temp_dir("save");
        let path = dir.join("sub").join(FILE_NAME);
        full_config().save_to(&path).expect("save");
        assert!(!with_suffix(&path, ".tmp").exists());
        let (loaded, warning) = Config::load_from(&path);
        assert_eq!(loaded, full_config());
        assert_eq!(warning, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_gives_default() {
        let dir = temp_dir("missing");
        let (config, warning) = Config::load_from(&dir.join(FILE_NAME));
        assert_eq!(config, Config::default());
        assert_eq!(warning, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn broken_file_is_set_aside() {
        let dir = temp_dir("broken");
        let path = dir.join(FILE_NAME);
        fs::write(&path, "version = [unterminated").expect("write");
        let (config, warning) = Config::load_from(&path);
        assert_eq!(config, Config::default());
        assert!(warning.is_some());
        assert!(!path.exists());
        let renamed = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("config.toml.broken-")
            });
        assert!(renamed);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn wrong_schema_is_set_aside() {
        let dir = temp_dir("schema");
        let path = dir.join(FILE_NAME);
        fs::write(&path, "version = 1\nbindings = \"nope\"\n").expect("write");
        let (config, warning) = Config::load_from(&path);
        assert_eq!(config, Config::default());
        assert!(warning.is_some());
        assert!(!path.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn future_version_is_left_alone() {
        let dir = temp_dir("future");
        let path = dir.join(FILE_NAME);
        let text = format!("version = {}\nsomething_new = true\n", CONFIG_VERSION + 1);
        fs::write(&path, &text).expect("write");
        let (config, warning) = Config::load_from(&path);
        assert_eq!(config, Config::default());
        assert!(warning.is_some());
        assert_eq!(fs::read_to_string(&path).expect("still there"), text);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn guesses_device_kind_from_name() {
        assert_eq!(
            DeviceKind::guess("Cuffie (Razer USB Sound Card)"),
            DeviceKind::Headphones
        );
        assert_eq!(
            DeviceKind::guess("Acer VG240Y (NVIDIA High Definition Audio)"),
            DeviceKind::Display
        );
        assert_eq!(
            DeviceKind::guess("Realtek Digital Output (Realtek USB Audio)"),
            DeviceKind::Speakers
        );
        assert_eq!(DeviceKind::guess("Virtual Cable"), DeviceKind::Generic);
    }
}
