//! Key combinations: capture from `egui` input in `global-hotkey` syntax, and
//! readable labels for the editor.

use eframe::egui::{Key, Modifiers};

use super::strings;

/// Outcome of a key press while capturing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Captured {
    /// A valid combination, e.g. `"Ctrl+Alt+KeyM"`.
    Combo(String),
    /// A key without modifiers that needs one.
    NeedsModifier,
    /// A key `global-hotkey` cannot register.
    Unsupported,
}

/// Converts a key press into `global-hotkey` syntax. `key` is the logical key,
/// `physical` the key at that position: the logical key wins for letters (the
/// hotkey is registered by virtual key, which follows the layout), the
/// physical one covers shifted symbols such as `Shift+1` → `!`.
pub fn capture(key: Key, physical: Option<Key>, modifiers: Modifiers) -> Captured {
    let Some(code) = key_code(key).or_else(|| physical.and_then(key_code)) else {
        return Captured::Unsupported;
    };
    let mut parts: Vec<&str> = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if parts.is_empty() && !is_standalone(&code) {
        return Captured::NeedsModifier;
    }
    let mut combo = parts.join("+");
    if !combo.is_empty() {
        combo.push('+');
    }
    combo.push_str(&code);
    Captured::Combo(combo)
}

/// F13–F24 exist on few keyboards and are safe to bind alone; so are media keys.
fn is_standalone(code: &str) -> bool {
    if let Some(number) = code.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
        return (13..=24).contains(&number);
    }
    code.starts_with("Media") || code.starts_with("AudioVolume")
}

/// `global-hotkey` code name of a key, `None` for keys it cannot parse or
/// that make no sense as hotkeys.
///
/// Punctuation keys are left out on purpose: `global-hotkey` maps them to
/// virtual keys of the US layout, so on other layouts (Italian included) the
/// registered key is not the one the user pressed. Shifted symbols on the
/// digit row fall back to the physical digit in [`capture`].
fn key_code(key: Key) -> Option<String> {
    let name = match key {
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::ArrowUp => "ArrowUp",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::MediaPlayPause => "MediaPlayPause",
        Key::MediaTrackNext => "MediaTrackNext",
        Key::MediaTrackPrevious => "MediaTrackPrevious",
        Key::MediaStop => "MediaStop",
        Key::AudioVolumeMute => "AudioVolumeMute",
        Key::AudioVolumeDown => "AudioVolumeDown",
        Key::AudioVolumeUp => "AudioVolumeUp",
        _ => {
            let name = key.name();
            // Letters: "A" → "KeyA"; function keys up to F24.
            if name.len() == 1 && name.chars().all(|c| c.is_ascii_uppercase()) {
                return Some(format!("Key{name}"));
            }
            if let Some(number) = name.strip_prefix('F').and_then(|n| n.parse::<u8>().ok())
                && (1..=24).contains(&number)
            {
                return Some(name.to_owned());
            }
            return None;
        }
    };
    Some(name.to_owned())
}

/// Splits a `global-hotkey` combination into readable key labels:
/// `"Ctrl+Alt+KeyM"` → `["Ctrl", "Alt", "M"]`.
pub fn readable(combo: &str) -> Vec<String> {
    combo
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(readable_token)
        .collect()
}

fn readable_token(token: &str) -> String {
    let upper = token.to_ascii_uppercase();
    let fixed = match upper.as_str() {
        "CTRL" | "CONTROL" | "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL"
        | "CMDORCONTROL" => "Ctrl",
        "ALT" | "OPTION" => "Alt",
        "SHIFT" => "Shift",
        "SUPER" | "META" | "COMMAND" | "CMD" => strings::KEY_WIN,
        "SPACE" => strings::KEY_SPACE,
        "ENTER" => strings::KEY_ENTER,
        "BACKSPACE" => strings::KEY_BACKSPACE,
        "DELETE" => strings::KEY_DELETE,
        "INSERT" => strings::KEY_INSERT,
        "HOME" => strings::KEY_HOME,
        "END" => strings::KEY_END,
        "PAGEUP" => strings::KEY_PAGE_UP,
        "PAGEDOWN" => strings::KEY_PAGE_DOWN,
        "ARROWUP" | "UP" => strings::KEY_UP,
        "ARROWDOWN" | "DOWN" => strings::KEY_DOWN,
        "ARROWLEFT" | "LEFT" => strings::KEY_LEFT,
        "ARROWRIGHT" | "RIGHT" => strings::KEY_RIGHT,
        "MEDIAPLAYPAUSE" => strings::KEY_PLAY_PAUSE,
        "MEDIATRACKNEXT" => strings::KEY_NEXT,
        "MEDIATRACKPREV" | "MEDIATRACKPREVIOUS" => strings::KEY_PREVIOUS,
        "MEDIASTOP" => strings::KEY_STOP,
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => strings::KEY_VOLUME_MUTE,
        "AUDIOVOLUMEUP" | "VOLUMEUP" => strings::KEY_VOLUME_UP,
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => strings::KEY_VOLUME_DOWN,
        "BACKQUOTE" => "`",
        "BACKSLASH" => "\\",
        "BRACKETLEFT" => "[",
        "BRACKETRIGHT" => "]",
        "COMMA" => ",",
        "EQUAL" => "=",
        "MINUS" => "-",
        "PERIOD" => ".",
        "QUOTE" => "'",
        "SEMICOLON" => ";",
        "SLASH" => "/",
        _ => {
            if let Some(rest) = upper.strip_prefix("KEY")
                && rest.len() == 1
            {
                return rest.to_owned();
            }
            if let Some(rest) = upper.strip_prefix("DIGIT") {
                return rest.to_owned();
            }
            if let Some(rest) = upper
                .strip_prefix("NUMPAD")
                .or_else(|| upper.strip_prefix("NUM"))
            {
                return format!("{} {rest}", strings::KEY_NUMPAD);
            }
            if upper.chars().count() == 1 {
                return upper;
            }
            return token.to_owned();
        }
    };
    fixed.to_owned()
}

/// Case-insensitive comparison of two combinations, ignoring modifier order
/// and aliases.
pub fn same_combo(a: &str, b: &str) -> bool {
    let normalize = |combo: &str| {
        let mut labels = readable(combo);
        labels.sort();
        labels
    };
    normalize(a) == normalize(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(ctrl: bool, alt: bool, shift: bool) -> Modifiers {
        Modifiers {
            ctrl,
            alt,
            shift,
            command: ctrl,
            ..Modifiers::NONE
        }
    }

    #[test]
    fn captures_letters_digits_and_function_keys() {
        assert_eq!(
            capture(Key::M, Some(Key::M), mods(true, true, false)),
            Captured::Combo("Ctrl+Alt+KeyM".into())
        );
        assert_eq!(
            capture(
                Key::Exclamationmark,
                Some(Key::Num1),
                mods(false, false, true)
            ),
            Captured::Combo("Shift+Digit1".into())
        );
        assert_eq!(
            capture(Key::F13, None, Modifiers::NONE),
            Captured::Combo("F13".into())
        );
    }

    #[test]
    fn requires_a_modifier() {
        assert_eq!(
            capture(Key::M, None, Modifiers::NONE),
            Captured::NeedsModifier
        );
        assert_eq!(
            capture(Key::F5, None, Modifiers::NONE),
            Captured::NeedsModifier
        );
    }

    #[test]
    fn readable_labels() {
        assert_eq!(readable("Ctrl+Alt+KeyM"), ["Ctrl", "Alt", "M"]);
        assert_eq!(readable("Shift+Digit1"), ["Shift", "1"]);
        assert_eq!(readable("ctrl+arrowup"), ["Ctrl", strings::KEY_UP]);
        assert!(same_combo("Alt+Ctrl+KeyM", "ctrl+alt+m"));
    }
}
