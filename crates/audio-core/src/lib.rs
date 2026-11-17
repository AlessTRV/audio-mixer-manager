//! Safe wrapper over the Windows audio APIs used by Audio Mixer Manager.
//!
//! Covers render endpoints, per-process audio sessions and per-application
//! output routing. This crate has no dependency on any UI toolkit.
//!
//! Every thread that uses these types must hold a [`com::ComGuard`]. The COM
//! wrappers are neither `Send` nor `Sync`: keep them on the thread that created
//! them and hand plain data to other threads.

#![cfg(windows)]

pub mod com;
pub mod device;
pub mod error;
pub mod events;

pub use error::{Error, Result};
