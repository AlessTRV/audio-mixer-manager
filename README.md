# Audio Mixer Manager

A Windows tray application that shows which processes are playing audio and lets you control where and how loudly each one plays.

## Features

- Per-process output device selection
- Per-process mute and volume
- Global hotkeys, including "move everything to output X" with three modes
- Persistent per-app rules
- Tray icon
- Start with Windows

## Status

Early development.

## Requirements

- Windows 10 1803+ or Windows 11
- Rust stable with the MSVC toolchain
- Visual Studio Build Tools with the C++ workload

## Build

```
cargo build --release
```

## Development

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Project layout

- `crates/audio-core`: safe library over the Windows audio APIs (devices, per-process sessions, per-app routing), with no UI dependency.
- `crates/app`: the `audiomixermanager` executable (tray, UI, hotkeys, configuration).
