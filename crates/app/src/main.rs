//! Audio Mixer Manager: per-application output routing, mute and volume from
//! the tray.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod config;
mod controller;
mod hotkeys;
mod icon;
mod model;
mod tray;
mod ui;

fn main() {}
