//! Global hotkeys: parsing, display and registration of [`Binding`]s.
//!
//! `global-hotkey` keeps its event handler in a `OnceCell`: it can be set
//! once per process and never replaced or cleared. The handler installed
//! here therefore forwards to [`ROUTE`], which each [`Hotkeys`] fills on
//! creation and empties on drop.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once, PoisonError};

use global_hotkey::hotkey::{Code, HotKey, HotKeyParseError};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::actions::Action;
use crate::config::Binding;
use crate::model::BindingError;

type ActionMap = Arc<Mutex<HashMap<u32, Action>>>;
type Callback = Arc<dyn Fn(Action) + Send + Sync>;

/// Where the process-wide event handler delivers presses.
static ROUTE: Mutex<Option<(ActionMap, Callback)>> = Mutex::new(None);
static HANDLER: Once = Once::new();

/// Registered global hotkeys and the actions they trigger.
pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    actions: ActionMap,
    /// Hotkeys registered by the last [`Hotkeys::apply`].
    registered: Vec<HotKey>,
    suspended: bool,
}

impl Hotkeys {
    /// Creates the hotkey manager. Must be called on the UI thread (the one
    /// running eframe's Win32 event loop): Windows delivers `WM_HOTKEY` to a
    /// hidden window owned by that thread.
    ///
    /// `on_action` runs on that thread for every press (not on release).
    ///
    /// # Errors
    /// When the hidden window cannot be created.
    pub fn new(on_action: impl Fn(Action) + Send + Sync + 'static) -> anyhow::Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|err| anyhow::anyhow!("cannot create the hotkey manager: {err}"))?;
        let actions = ActionMap::default();
        *ROUTE.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((Arc::clone(&actions), Arc::new(on_action)));
        HANDLER.call_once(|| GlobalHotKeyEvent::set_event_handler(Some(dispatch)));
        Ok(Self {
            manager,
            actions,
            registered: Vec::new(),
            suspended: false,
        })
    }

    /// Unregisters everything and registers `bindings`, ending any
    /// suspension. Returns one error per binding that could not be
    /// registered; the others are registered anyway.
    pub fn apply(&mut self, bindings: &[Binding]) -> Vec<BindingError> {
        self.unregister_all();
        self.suspended = false;

        let mut errors = Vec::new();
        let mut map = HashMap::new();
        // id -> index of the first binding using it.
        let mut seen: HashMap<u32, usize> = HashMap::new();
        for (index, binding) in bindings.iter().enumerate() {
            let hotkey = match parse(&binding.keys) {
                Ok(hotkey) => hotkey,
                Err(message) => {
                    errors.push(BindingError { index, message });
                    continue;
                }
            };
            if let Some(first) = seen.get(&hotkey.id()) {
                errors.push(BindingError {
                    index,
                    message: format!("Combinazione già usata dalla scorciatoia {}.", first + 1),
                });
                continue;
            }
            seen.insert(hotkey.id(), index);
            match self.manager.register(hotkey) {
                Ok(()) => {
                    self.registered.push(hotkey);
                    map.insert(hotkey.id(), binding.action.clone());
                }
                Err(err) => {
                    tracing::warn!(keys = %binding.keys, "hotkey registration failed: {err}");
                    errors.push(BindingError {
                        index,
                        message: registration_message(&err),
                    });
                }
            }
        }
        *self.actions.lock().unwrap_or_else(PoisonError::into_inner) = map;
        errors
    }

    /// Unregisters every hotkey, keeping the list, so a new combination can
    /// be captured in the editor without triggering the current ones.
    pub fn suspend(&mut self) {
        if self.suspended {
            return;
        }
        self.suspended = true;
        for hotkey in &self.registered {
            if let Err(err) = self.manager.unregister(*hotkey) {
                tracing::warn!("cannot unregister {hotkey}: {err}");
            }
        }
    }

    /// Registers again what [`Hotkeys::suspend`] removed. A combination taken
    /// by another application in the meantime is dropped and logged.
    pub fn resume(&mut self) {
        if !self.suspended {
            return;
        }
        self.suspended = false;
        let mut actions = self.actions.lock().unwrap_or_else(PoisonError::into_inner);
        self.registered
            .retain(|hotkey| match self.manager.register(*hotkey) {
                Ok(()) => true,
                Err(err) => {
                    tracing::warn!("cannot register {hotkey} again: {err}");
                    actions.remove(&hotkey.id());
                    false
                }
            });
    }

    fn unregister_all(&mut self) {
        if !self.suspended {
            for hotkey in &self.registered {
                if let Err(err) = self.manager.unregister(*hotkey) {
                    tracing::warn!("cannot unregister {hotkey}: {err}");
                }
            }
        }
        self.registered.clear();
        self.actions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

impl Drop for Hotkeys {
    fn drop(&mut self) {
        self.unregister_all();
        // `set_event_handler(None)` would be a no-op (the handler lives in a
        // `OnceCell`); detach the route instead, if it is still ours.
        let mut route = ROUTE.lock().unwrap_or_else(PoisonError::into_inner);
        if route
            .as_ref()
            .is_some_and(|(actions, _)| Arc::ptr_eq(actions, &self.actions))
        {
            *route = None;
        }
    }
}

/// Process-wide event handler: forwards presses to the current route.
fn dispatch(event: GlobalHotKeyEvent) {
    if event.state != HotKeyState::Pressed {
        return;
    }
    // Clone out of the locks so the callback may freely use `Hotkeys`.
    let route = ROUTE.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let Some((actions, on_action)) = route else {
        return;
    };
    let action = actions
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&event.id)
        .cloned();
    if let Some(action) = action {
        on_action(action);
    }
}

fn registration_message(err: &global_hotkey::Error) -> String {
    match err {
        global_hotkey::Error::AlreadyRegistered(_) => {
            "Combinazione già in uso da un'altra applicazione.".to_owned()
        }
        global_hotkey::Error::FailedToRegister(_) => {
            "Tasto non utilizzabile come scorciatoia globale.".to_owned()
        }
        other => format!("Registrazione non riuscita: {other}"),
    }
}

/// Keys that may be bound alone: nothing types them, so a global binding
/// steals nothing.
const STANDALONE: [Code; 12] = [
    Code::F13,
    Code::F14,
    Code::F15,
    Code::F16,
    Code::F17,
    Code::F18,
    Code::F19,
    Code::F20,
    Code::F21,
    Code::F22,
    Code::F23,
    Code::F24,
];

/// Validates a combination in `global-hotkey` syntax (`"Ctrl+Alt+KeyM"`).
///
/// # Errors
/// An Italian message for the editor when the syntax is not recognised, the
/// main key is a modifier, or there is no modifier (except F13–F24).
pub fn parse(keys: &str) -> Result<HotKey, String> {
    let keys = keys.trim();
    if keys.is_empty() {
        return Err("Combinazione vuota.".to_owned());
    }
    let hotkey = parse_raw(keys).map_err(|err| match err {
        HotKeyParseError::UnsupportedKey(key) => format!("Tasto non riconosciuto: «{key}»."),
        HotKeyParseError::EmptyToken(_) | HotKeyParseError::InvalidFormat(_) => {
            "Combinazione non valida: prima i modificatori, poi un solo tasto.".to_owned()
        }
    })?;
    if is_layout_dependent(hotkey.key) {
        return Err("Tasto non valido: usa lettere, numeri, F1–F24 o frecce".to_owned());
    }
    if is_modifier_key(hotkey.key) {
        return Err("Serve un tasto oltre ai modificatori.".to_owned());
    }
    if hotkey.mods.is_empty() && !STANDALONE.contains(&hotkey.key) && !is_media_key(hotkey.key) {
        return Err(
            "Aggiungi Ctrl, Alt, Shift o Win: da solo il tasto verrebbe catturato \
                    in tutto il sistema."
                .to_owned(),
        );
    }
    Ok(hotkey)
}

/// `global-hotkey` parsing, also accepting `Win` (which it does not know)
/// as the Windows key modifier.
fn parse_raw(keys: &str) -> Result<HotKey, HotKeyParseError> {
    let tokens: Vec<&str> = keys
        .split('+')
        .map(|token| {
            if token.trim().eq_ignore_ascii_case("win") {
                "Super"
            } else {
                token
            }
        })
        .collect();
    tokens.join("+").parse()
}

fn is_modifier_key(code: Code) -> bool {
    matches!(
        code,
        Code::ShiftLeft
            | Code::ShiftRight
            | Code::ControlLeft
            | Code::ControlRight
            | Code::AltLeft
            | Code::AltRight
            | Code::MetaLeft
            | Code::MetaRight
    )
}
/// Punctuation keys: `global-hotkey` maps them to US-layout virtual keys, so on
/// other layouts the registered key differs from the one the user pressed.
fn is_layout_dependent(code: Code) -> bool {
    matches!(
        code,
        Code::Backquote
            | Code::Backslash
            | Code::BracketLeft
            | Code::BracketRight
            | Code::Comma
            | Code::Equal
            | Code::IntlBackslash
            | Code::Minus
            | Code::Period
            | Code::Quote
            | Code::Semicolon
            | Code::Slash
    )
}

/// Media and volume keys are safe to bind without modifiers.
fn is_media_key(code: Code) -> bool {
    let name = format!("{code:?}");
    name.starts_with("Media") || name.starts_with("AudioVolume")
}

#[cfg(test)]
mod tests {
    use global_hotkey::hotkey::Modifiers;

    use super::*;

    #[test]
    fn parses_valid_combinations() {
        let hotkey = parse("Ctrl+Alt+KeyM").expect("valid");
        assert_eq!(hotkey.mods, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(hotkey.key, Code::KeyM);

        let hotkey = parse("shift+win+ArrowUp").expect("valid");
        assert_eq!(hotkey.mods, Modifiers::SHIFT | Modifiers::META);
        assert_eq!(hotkey.key, Code::ArrowUp);

        assert_eq!(parse(" Ctrl+Shift+1 ").expect("valid").key, Code::Digit1);
        assert_eq!(parse("Ctrl+F5").expect("valid").key, Code::F5);
    }

    #[test]
    fn same_combination_same_id() {
        let a = parse("Ctrl+Alt+KeyM").expect("valid");
        let b = parse("alt+control+m").expect("valid");
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn rejects_invalid_syntax() {
        assert!(parse("").is_err());
        assert!(parse("Ctrl+").is_err());
        assert!(parse("Ctrl+KeyM+Alt").is_err());
        assert!(parse("Ctrl+Alt").is_err());
        assert!(parse("Ctrl+Pippo").is_err());
        assert!(parse("Ctrl+ShiftLeft").is_err());
    }

    #[test]
    fn requires_a_modifier() {
        let err = parse("KeyM").expect_err("no modifier");
        assert!(err.contains("Ctrl"), "{err}");
        assert!(parse("F5").is_err());
        assert!(parse("Space").is_err());
        assert!(parse("F13").is_ok());
        assert!(parse("F24").is_ok());
        assert!(parse("Ctrl+Slash").is_err());
        assert!(parse("Ctrl+Shift+Digit7").is_ok());
    }
}
