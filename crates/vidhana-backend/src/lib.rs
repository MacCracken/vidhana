//! Vidhana Backend — System backends and settings service
//!
//! Provides OS-level integration for applying settings (brightness, volume,
//! WiFi, power profiles, etc.) and a `SettingsService` mediator that
//! centralizes the validate → apply → persist → audit flow.

mod backends;
pub mod service;

pub use backends::{BackendError, LinuxBackend, NoopBackend, SystemBackend, SystemSnapshot};
pub use service::SettingsService;
