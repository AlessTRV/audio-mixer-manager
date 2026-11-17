//! Every user-visible text of the application, in Italian.

/// Application name, used as window title.
pub const APP_TITLE: &str = "Audio Mixer Manager";

// Tray.
pub const TRAY_TOOLTIP: &str = "Audio Mixer Manager";
pub const TRAY_OPEN: &str = "Apri mixer";
pub const TRAY_DEFAULT_OUTPUT: &str = DEFAULT_OUTPUT;
pub const TRAY_QUIT: &str = "Esci";

/// Display name of the system sounds entry.
pub const SYSTEM_SOUNDS: &str = "Suoni di sistema";

// Header.
pub const DEFAULT_OUTPUT: &str = "Uscita predefinita";
pub const NO_OUTPUTS: &str = "Nessuna uscita attiva";
pub const DEFAULT_OUTPUT_TOOLTIP: &str = "Uscita audio predefinita di Windows";
pub const OPEN_SETTINGS: &str = "Impostazioni";

// Application cards.
/// Caption of an application that follows the system default; the default
/// output name is appended in parentheses.
pub const FOLLOWS_DEFAULT: &str = "Predefinita";
/// First entry of the per-application output menu.
pub const SYSTEM_DEFAULT_OUTPUT: &str = "Predefinita di sistema";
/// Shown when an application is routed to a device that is no longer active.
pub const UNKNOWN_OUTPUT: &str = "Uscita non disponibile";
pub const ROUTING_UNAVAILABLE: &str =
    "Questa versione di Windows non permette di scegliere l'uscita per singola app";
pub const VOLUME_TOOLTIP: &str = "Volume";
pub const PERCENT_SUFFIX: &str = "%";
pub const MUTE: &str = "Silenzia";
pub const UNMUTE: &str = "Riattiva audio";
pub const CARD_MENU: &str = "Altre opzioni";
pub const CREATE_MUTE_SHORTCUT: &str = "Crea shortcut mute";
pub const OTHER_OUTPUTS: &str = "Altre uscite";
/// Appended to an output tooltip when the app follows the system default there.
pub const OUTPUT_FOLLOWS_DEFAULT: &str = "segue la predefinita";
/// Appended to an output tooltip when clicking it returns to the default.
pub const OUTPUT_CLICK_TO_RESET: &str = "clic per tornare alla predefinita";
pub const OUTPUT_DISCONNECTED: &str = "non connessa";

// Empty state.
pub const EMPTY_TITLE: &str = "Nessuna app sta usando l'audio";
pub const EMPTY_HINT: &str = "Le app compaiono qui appena aprono una sessione audio.";

// Settings view.
pub const SETTINGS_TITLE: &str = "Impostazioni";
pub const FAVORITES_TITLE: &str = "Uscite preferite";
pub const FAVORITES_HINT: &str = "Compaiono come icone in ogni app: un clic sposta l'app lì.";
pub const FAVORITES_EMPTY: &str = "Nessuna preferita: le app mostrano le prime uscite attive.";
/// Shown when the maximum number of favorites is reached.
pub const FAVORITES_FULL: &str = "Hai raggiunto il massimo di uscite preferite";
pub const FAVORITE_TOGGLE: &str = "Mostra nelle app";
pub const FAVORITE_KIND: &str = "Icona";
pub const MOVE_UP: &str = "Sposta su";
pub const MOVE_DOWN: &str = "Sposta giù";
pub const KIND_HEADPHONES: &str = "Cuffie";
pub const KIND_SPEAKERS: &str = "Altoparlanti";
pub const KIND_DISPLAY: &str = "Monitor";
pub const KIND_GENERIC: &str = "Generico";
pub const SHORTCUTS_TITLE: &str = "Shortcut";
pub const BACK: &str = "Torna al mixer";
pub const ADD_SHORTCUT: &str = "Aggiungi";
pub const NO_SHORTCUTS_TITLE: &str = "Nessuno shortcut";
pub const NO_SHORTCUTS_HINT: &str =
    "Aggiungi una combinazione di tasti per silenziare un'app o spostare l'audio.";
pub const EDIT: &str = "Modifica";
pub const DELETE: &str = "Elimina";
pub const NEW_SHORTCUT: &str = "Nuovo shortcut";
pub const EDIT_SHORTCUT: &str = "Modifica shortcut";
pub const SAVE: &str = "Salva";
pub const CANCEL: &str = "Annulla";

// Action descriptions.
pub const ACTION_TOGGLE_WINDOW: &str = "Mostra/nascondi finestra";
/// Followed by the application name.
pub const ACTION_TOGGLE_MUTE: &str = "Muta/smuta";
/// Followed by the output name and the mode.
pub const ACTION_MOVE_ALL: &str = "Tutto su";

// Action picker.
pub const FIELD_ACTION: &str = "Azione";
pub const KIND_WINDOW: &str = "Finestra";
pub const KIND_MUTE: &str = "Muta app";
pub const KIND_MOVE: &str = "Sposta audio";
pub const FIELD_APP: &str = "App";
pub const PICK_APP: &str = "Scegli un'app";
pub const APP_EXE_HINT: &str = "oppure scrivi l'eseguibile, es. discord.exe";
pub const FIELD_OUTPUT: &str = "Uscita";
pub const PICK_OUTPUT: &str = "Scegli un'uscita";
pub const FIELD_MODE: &str = "Modalità";
pub const MODE_RESET: &str = "Reset";
pub const MODE_RESET_DETAIL: &str = "Predefinita + azzera assegnazioni per app";
pub const MODE_ACTIVE_ONLY: &str = "Solo attive";
pub const MODE_ACTIVE_ONLY_DETAIL: &str = "Sposta solo le app che stanno suonando";
pub const MODE_KEEP_RULES: &str = "Rispetta assegnazioni";
pub const MODE_KEEP_RULES_DETAIL: &str =
    "Cambia la predefinita, le app assegnate restano dove sono";

// Key capture.
pub const FIELD_KEYS: &str = "Combinazione";
pub const CAPTURE: &str = "Registra combinazione";
pub const RECAPTURE: &str = "Cambia";
pub const CAPTURING: &str = "Premi la combinazione…";
pub const CAPTURE_HINT: &str = "Esc per annullare";
pub const NEEDS_MODIFIER: &str = "Serve almeno un tasto tra Ctrl, Alt e Shift (tranne F13–F24)";
pub const UNSUPPORTED_KEY: &str = "Tasto non valido: usa lettere, numeri, F1–F24 o frecce";
pub const DUPLICATE_KEYS: &str = "Combinazione già usata da un altro shortcut";
pub const MISSING_KEYS: &str = "Registra una combinazione";
pub const MISSING_APP: &str = "Scegli un'app";
pub const MISSING_OUTPUT: &str = "Scegli un'uscita";

// Readable key names.
pub const KEY_WIN: &str = "Win";
pub const KEY_SPACE: &str = "Spazio";
pub const KEY_ENTER: &str = "Invio";
pub const KEY_BACKSPACE: &str = "Backspace";
pub const KEY_DELETE: &str = "Canc";
pub const KEY_INSERT: &str = "Ins";
pub const KEY_HOME: &str = "Home";
pub const KEY_END: &str = "Fine";
pub const KEY_PAGE_UP: &str = "PagSu";
pub const KEY_PAGE_DOWN: &str = "PagGiù";
pub const KEY_UP: &str = "Freccia su";
pub const KEY_DOWN: &str = "Freccia giù";
pub const KEY_LEFT: &str = "Freccia sinistra";
pub const KEY_RIGHT: &str = "Freccia destra";
pub const KEY_PLAY_PAUSE: &str = "Play/Pausa";
pub const KEY_NEXT: &str = "Traccia successiva";
pub const KEY_PREVIOUS: &str = "Traccia precedente";
pub const KEY_STOP: &str = "Stop";
pub const KEY_VOLUME_MUTE: &str = "Muto";
pub const KEY_VOLUME_UP: &str = "Volume +";
pub const KEY_VOLUME_DOWN: &str = "Volume -";
pub const KEY_NUMPAD: &str = "Num";

// General settings.
pub const GENERAL_TITLE: &str = "Generale";
pub const AUTOSTART: &str = "Avvia con Windows";
pub const AUTOSTART_DETAIL: &str = "Parte nascosta nell'area di notifica";
pub const AUTOSTART_ERROR: &str = "Impossibile modificare l'avvio automatico";

// Overlay shown when a global shortcut runs an action.
pub const OVERLAY_TITLE: &str = "Audio Mixer Manager";
/// Followed by the output name.
pub const OVERLAY_AUDIO_ON: &str = "Audio su";
pub const OVERLAY_MODE_RESET: &str = "Predefinita + azzera assegnazioni";
pub const OVERLAY_MODE_ACTIVE_ONLY: &str = "Solo app attive";
pub const OVERLAY_MODE_KEEP_RULES: &str = "Rispetta assegnazioni";
pub const OVERLAY_MUTED: &str = "Audio disattivato";
pub const OVERLAY_UNMUTED: &str = "Audio riattivato";
pub const OVERLAY_NO_SESSION: &str = "Nessuna sessione audio";
