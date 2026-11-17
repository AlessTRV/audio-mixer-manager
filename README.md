# Audio Mixer Manager

Audio Mixer Manager is a small Windows tray app, written in Rust, that lists the processes currently playing audio and lets you choose an output device, mute state and volume for each one. Global hotkeys let you move all audio to a chosen output without opening any Windows settings page.

## Features

- Per-app output device, with one-click favorite outputs on each app card.
- Per-app mute and volume.
- Apps ordered by recent use.
- Global hotkeys: toggle the window, mute an app, and move all audio to an output. Moving audio has three modes:
  - reset: set the default output and clear per-app assignments;
  - active only: move only the apps currently playing;
  - keep assignments: move audio but preserve existing per-app assignments.
- Short on-screen overlay when a hotkey acts.
- Tray icon with a default-output menu.
- Start with Windows (starts hidden in the tray).
- Single instance.
- Settings in `%APPDATA%\AudioMixerManager\config.toml`, logs in `%LOCALAPPDATA%\AudioMixerManager\logs`.

## Install

1. Download `AudioMixerManager-Setup-<version>.exe` from the GitHub release page.
2. Run it. The install is per-user and needs no admin rights. The installer has a "Start with Windows" option.
3. To uninstall, use Settings -> Apps.

The build is unsigned, so SmartScreen may show an "unknown publisher" warning. Click More info, then Run anyway.

## Usage

Open the window from the tray icon or with the default hotkey Ctrl+Alt+M. The Settings page (gear icon) holds favorite outputs, hotkeys and the autostart option.

Hotkeys accept letters, digits, F1-F24, arrows, numpad and media keys. Punctuation keys depend on the keyboard layout and are not supported.

## Requirements

Windows 10 1803+ or Windows 11. Per-app routing uses undocumented Windows interfaces (the same ones EarTrumpet uses). If they are unavailable on your Windows build, that feature is disabled.

## Build from source

Requirements: Rust stable (MSVC toolchain) and the Visual Studio Build Tools with the C++ workload.

```
cargo build --release
```

To build the installer, install Inno Setup 6 and run, after the release build:

```
iscc installer\audiomixermanager.iss
```

Development checks:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p audio-core --example dump
```

The `dump` example is read-only when run without arguments.

## Project layout

- `crates/audio-core`: safe library over WASAPI/COM (devices, sessions, per-app routing).
- `crates/app`: the tray app (controller, hotkeys, config, UI).
- `installer/`: Inno Setup script.
- `.github/workflows/`: CI and release workflows.

## Credits

Interface definitions for per-app routing are adapted from [EarTrumpet](https://github.com/File-New-Project/EarTrumpet) (MIT).
