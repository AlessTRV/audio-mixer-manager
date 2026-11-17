# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-17

### Added

- Per-app output device selection, with one-click favorite outputs on each app card.
- Per-app mute and volume.
- Apps ordered by recent use.
- Global hotkeys to toggle the window, mute an app, and move all audio to an output.
- Three move modes: reset (default output and clear assignments), active only, keep assignments.
- Short on-screen overlay when a hotkey acts.
- Tray icon with a default-output menu.
- Start with Windows, launching hidden in the tray.
- Single-instance enforcement.
- Settings in `%APPDATA%\AudioMixerManager\config.toml` and logs in `%LOCALAPPDATA%\AudioMixerManager\logs`.
- Inno Setup per-user installer, built by the release workflow.
