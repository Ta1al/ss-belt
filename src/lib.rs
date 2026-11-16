//! ss-belt — one toolbelt for Linux capture, annotate, record, OCR, QR and upload.
//!
//! The design is an orchestrator, not a re-implementation: external programs are
//! wrapped behind probed adapters, while the parts that are cheap and reliable in
//! Rust (QR decode/encode, image transforms, hashing, history, task chain) are
//! built in.

pub mod action;
pub mod adapters;
pub mod capabilities;
pub mod config;
pub mod daemon;
pub mod doctor;
pub mod error;
pub mod exec;
pub mod history;
pub mod hotkeys;
pub mod notify_click;
pub mod paths;
pub mod pipeline;
pub mod steps;
pub mod sxcu;
pub mod tray;
pub mod util;

pub const APP_NAME: &str = "ss-belt";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SOCKET_NAME: &str = "daemon.sock";
