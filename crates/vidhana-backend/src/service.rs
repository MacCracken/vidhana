//! SettingsService — centralized mutation flow for all settings changes.
//!
//! Every mutation (from API, MCP, GUI) goes through the service which:
//! 1. Validates the new settings
//! 2. Applies to the OS via the system backend
//! 3. Updates in-memory shared state
//! 4. Persists to TOML
//! 5. Records the change in the SQLite audit log

use std::sync::Arc;
use vidhana_core::*;
use vidhana_settings::{ChangeSource, SettingsChange, SettingsStore};

use crate::backends::SystemBackend;

/// Centralized settings mutation service.
pub struct SettingsService {
    pub state: SharedState,
    pub store: Arc<SettingsStore>,
    pub backend: Arc<dyn SystemBackend>,
}

/// Errors from the settings service.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("persistence error: {0}")]
    Persistence(String),

    #[error("deserialization error: {0}")]
    Deserialize(String),

    #[error("lock poisoned")]
    LockPoisoned,
}

impl SettingsService {
    pub fn new(
        state: SharedState,
        store: Arc<SettingsStore>,
        backend: Arc<dyn SystemBackend>,
    ) -> Self {
        Self {
            state,
            store,
            backend,
        }
    }

    /// Read current OS state and merge into in-memory settings.
    pub fn sync_from_os(&self) {
        let snap = self.backend.read_system_state();
        let mut guard = self.state.write().unwrap();

        if let Some(b) = snap.brightness {
            guard.display.brightness = b;
        }
        if let Some(v) = snap.master_volume {
            guard.audio.master_volume = v;
        }
        if let Some(m) = snap.muted {
            guard.audio.muted = m;
        }
        if let Some(w) = snap.wifi_enabled {
            guard.network.wifi_enabled = w;
        }
        if let Some(bt) = snap.bluetooth_enabled {
            guard.network.bluetooth_enabled = bt;
        }
        if let Some(pp) = snap.power_profile {
            guard.power.power_profile = pp;
        }
        if let Some(tz) = snap.timezone {
            guard.locale.timezone = tz;
        }

        tracing::info!("Synced settings from OS state");
    }

    // --- Display -----------------------------------------------------------

    pub fn update_display(
        &self,
        mut settings: DisplaySettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.display);

        if let Err(e) = self.backend.apply_display(&settings) {
            tracing::warn!("Backend apply_display: {e}");
        }

        self.write_state(|s| s.display = settings)?;
        let new = self.read_json(|s| &s.display);
        self.persist_and_record("display", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_display(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<DisplaySettings, ServiceError> {
        let old = self.read_json(|s| &s.display);
        let current_val = self.read_value(|s| &s.display)?;
        let mut updated: DisplaySettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_display(&updated) {
            tracing::warn!("Backend apply_display: {e}");
        }

        self.write_state(|s| s.display = updated.clone())?;
        let new = self.read_json(|s| &s.display);
        self.persist_and_record("display", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Audio -------------------------------------------------------------

    pub fn update_audio(
        &self,
        mut settings: AudioSettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.audio);

        if let Err(e) = self.backend.apply_audio(&settings) {
            tracing::warn!("Backend apply_audio: {e}");
        }

        self.write_state(|s| s.audio = settings)?;
        let new = self.read_json(|s| &s.audio);
        self.persist_and_record("audio", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_audio(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<AudioSettings, ServiceError> {
        let old = self.read_json(|s| &s.audio);
        let current_val = self.read_value(|s| &s.audio)?;
        let mut updated: AudioSettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_audio(&updated) {
            tracing::warn!("Backend apply_audio: {e}");
        }

        self.write_state(|s| s.audio = updated.clone())?;
        let new = self.read_json(|s| &s.audio);
        self.persist_and_record("audio", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Network -----------------------------------------------------------

    pub fn update_network(
        &self,
        mut settings: NetworkSettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.network);

        if let Err(e) = self.backend.apply_network(&settings) {
            tracing::warn!("Backend apply_network: {e}");
        }

        self.write_state(|s| s.network = settings)?;
        let new = self.read_json(|s| &s.network);
        self.persist_and_record("network", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_network(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<NetworkSettings, ServiceError> {
        let old = self.read_json(|s| &s.network);
        let current_val = self.read_value(|s| &s.network)?;
        let mut updated: NetworkSettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_network(&updated) {
            tracing::warn!("Backend apply_network: {e}");
        }

        self.write_state(|s| s.network = updated.clone())?;
        let new = self.read_json(|s| &s.network);
        self.persist_and_record("network", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Privacy -----------------------------------------------------------

    pub fn update_privacy(
        &self,
        mut settings: PrivacySettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.privacy);

        if let Err(e) = self.backend.apply_privacy(&settings) {
            tracing::warn!("Backend apply_privacy: {e}");
        }

        self.write_state(|s| s.privacy = settings)?;
        let new = self.read_json(|s| &s.privacy);
        self.persist_and_record("privacy", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_privacy(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<PrivacySettings, ServiceError> {
        let old = self.read_json(|s| &s.privacy);
        let current_val = self.read_value(|s| &s.privacy)?;
        let mut updated: PrivacySettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_privacy(&updated) {
            tracing::warn!("Backend apply_privacy: {e}");
        }

        self.write_state(|s| s.privacy = updated.clone())?;
        let new = self.read_json(|s| &s.privacy);
        self.persist_and_record("privacy", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Locale ------------------------------------------------------------

    pub fn update_locale(
        &self,
        mut settings: LocaleSettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.locale);

        if let Err(e) = self.backend.apply_locale(&settings) {
            tracing::warn!("Backend apply_locale: {e}");
        }

        self.write_state(|s| s.locale = settings)?;
        let new = self.read_json(|s| &s.locale);
        self.persist_and_record("locale", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_locale(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<LocaleSettings, ServiceError> {
        let old = self.read_json(|s| &s.locale);
        let current_val = self.read_value(|s| &s.locale)?;
        let mut updated: LocaleSettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_locale(&updated) {
            tracing::warn!("Backend apply_locale: {e}");
        }

        self.write_state(|s| s.locale = updated.clone())?;
        let new = self.read_json(|s| &s.locale);
        self.persist_and_record("locale", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Power -------------------------------------------------------------

    pub fn update_power(
        &self,
        mut settings: PowerSettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.power);

        if let Err(e) = self.backend.apply_power(&settings) {
            tracing::warn!("Backend apply_power: {e}");
        }

        self.write_state(|s| s.power = settings)?;
        let new = self.read_json(|s| &s.power);
        self.persist_and_record("power", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_power(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<PowerSettings, ServiceError> {
        let old = self.read_json(|s| &s.power);
        let current_val = self.read_value(|s| &s.power)?;
        let mut updated: PowerSettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_power(&updated) {
            tracing::warn!("Backend apply_power: {e}");
        }

        self.write_state(|s| s.power = updated.clone())?;
        let new = self.read_json(|s| &s.power);
        self.persist_and_record("power", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Accessibility -----------------------------------------------------

    pub fn update_accessibility(
        &self,
        mut settings: AccessibilitySettings,
        source: ChangeSource,
    ) -> Result<(), ServiceError> {
        settings.validate();
        let old = self.read_json(|s| &s.accessibility);

        if let Err(e) = self.backend.apply_accessibility(&settings) {
            tracing::warn!("Backend apply_accessibility: {e}");
        }

        self.write_state(|s| s.accessibility = settings)?;
        let new = self.read_json(|s| &s.accessibility);
        self.persist_and_record("accessibility", "*", &old, &new, source);
        Ok(())
    }

    pub fn patch_accessibility(
        &self,
        patch: serde_json::Value,
        source: ChangeSource,
    ) -> Result<AccessibilitySettings, ServiceError> {
        let old = self.read_json(|s| &s.accessibility);
        let current_val = self.read_value(|s| &s.accessibility)?;
        let mut updated: AccessibilitySettings = self.merge_and_parse(current_val, &patch)?;
        updated.validate();

        if let Err(e) = self.backend.apply_accessibility(&updated) {
            tracing::warn!("Backend apply_accessibility: {e}");
        }

        self.write_state(|s| s.accessibility = updated.clone())?;
        let new = self.read_json(|s| &s.accessibility);
        self.persist_and_record("accessibility", "*", &old, &new, source);
        Ok(updated)
    }

    // --- Internals ---------------------------------------------------------

    fn read_json<T: serde::Serialize>(&self, f: impl Fn(&VidhanaState) -> &T) -> String {
        let guard = self.state.read().unwrap();
        serde_json::to_string(f(&guard)).unwrap_or_default()
    }

    fn read_value<T: serde::Serialize>(
        &self,
        f: impl Fn(&VidhanaState) -> &T,
    ) -> Result<serde_json::Value, ServiceError> {
        let guard = self.state.read().unwrap();
        serde_json::to_value(f(&guard)).map_err(|e| ServiceError::Deserialize(e.to_string()))
    }

    fn write_state(&self, f: impl FnOnce(&mut VidhanaState)) -> Result<(), ServiceError> {
        let mut guard = self.state.write().map_err(|_| ServiceError::LockPoisoned)?;
        f(&mut guard);
        Ok(())
    }

    fn merge_and_parse<T: serde::de::DeserializeOwned>(
        &self,
        mut current: serde_json::Value,
        patch: &serde_json::Value,
    ) -> Result<T, ServiceError> {
        if let (Some(obj), Some(p)) = (current.as_object_mut(), patch.as_object()) {
            for (k, v) in p {
                obj.insert(k.clone(), v.clone());
            }
        }
        serde_json::from_value(current).map_err(|e| ServiceError::Deserialize(e.to_string()))
    }

    fn persist_and_record(
        &self,
        category: &str,
        key: &str,
        old: &str,
        new: &str,
        source: ChangeSource,
    ) {
        let guard = self.state.read().unwrap();
        if let Err(e) = self.store.save_state(&guard) {
            tracing::error!("Failed to persist settings: {e}");
        }
        drop(guard);

        let change = SettingsChange {
            timestamp: chrono::Utc::now(),
            category: category.to_string(),
            key: key.to_string(),
            old_value: old.to_string(),
            new_value: new.to_string(),
            source,
        };
        if let Err(e) = self.store.record_change(&change) {
            tracing::error!("Failed to record change: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::NoopBackend;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_service() -> Arc<SettingsService> {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("vidhana-svc-test-{}-{}", std::process::id(), id));
        let store = Arc::new(SettingsStore::new(dir.to_str().unwrap()).unwrap());
        let state = new_shared_state(VidhanaConfig::default());
        let backend = Arc::new(NoopBackend);
        Arc::new(SettingsService::new(state, store, backend))
    }

    #[test]
    fn test_update_display() {
        let svc = test_service();
        let settings = DisplaySettings {
            brightness: 42,
            ..DisplaySettings::default()
        };
        svc.update_display(settings, ChangeSource::Api).unwrap();
        assert_eq!(svc.state.read().unwrap().display.brightness, 42);

        // Verify persisted
        let loaded = svc.store.load_state().unwrap().unwrap();
        assert_eq!(loaded.display.brightness, 42);

        // Verify history
        let changes = svc.store.recent_changes(10).unwrap();
        assert_eq!(changes[0].category, "display");
    }

    #[test]
    fn test_patch_display() {
        let svc = test_service();
        let result = svc
            .patch_display(serde_json::json!({"brightness": 55}), ChangeSource::Api)
            .unwrap();
        assert_eq!(result.brightness, 55);
        assert_eq!(result.theme, Theme::Dark); // unchanged
    }

    #[test]
    fn test_patch_validates() {
        let svc = test_service();
        let result = svc
            .patch_display(serde_json::json!({"brightness": 255}), ChangeSource::Api)
            .unwrap();
        assert_eq!(result.brightness, 100); // clamped
    }

    #[test]
    fn test_patch_invalid_field() {
        let svc = test_service();
        let result = svc.patch_display(serde_json::json!({"theme": "invalid"}), ChangeSource::Api);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_audio() {
        let svc = test_service();
        let settings = AudioSettings {
            master_volume: 50,
            muted: true,
            ..AudioSettings::default()
        };
        svc.update_audio(settings, ChangeSource::Mcp).unwrap();
        let guard = svc.state.read().unwrap();
        assert_eq!(guard.audio.master_volume, 50);
        assert!(guard.audio.muted);
    }

    #[test]
    fn test_update_network() {
        let svc = test_service();
        let mut settings = NetworkSettings::default();
        settings.wifi_enabled = false;
        svc.update_network(settings, ChangeSource::Gui).unwrap();
        assert!(!svc.state.read().unwrap().network.wifi_enabled);
    }

    #[test]
    fn test_update_power() {
        let svc = test_service();
        let settings = PowerSettings {
            power_profile: PowerProfile::Performance,
            ..PowerSettings::default()
        };
        svc.update_power(settings, ChangeSource::Api).unwrap();
        assert_eq!(
            svc.state.read().unwrap().power.power_profile,
            PowerProfile::Performance
        );
    }

    #[test]
    fn test_update_locale() {
        let svc = test_service();
        let mut settings = LocaleSettings::default();
        settings.timezone = "America/New_York".to_string();
        svc.update_locale(settings, ChangeSource::Api).unwrap();
        assert_eq!(
            svc.state.read().unwrap().locale.timezone,
            "America/New_York"
        );
    }

    #[test]
    fn test_sync_from_os() {
        // NoopBackend returns all None, so this is a no-op — just ensure no panic
        let svc = test_service();
        svc.sync_from_os();
    }

    #[test]
    fn test_change_sources() {
        let svc = test_service();
        svc.update_display(DisplaySettings::default(), ChangeSource::Api)
            .unwrap();
        svc.update_audio(AudioSettings::default(), ChangeSource::Mcp)
            .unwrap();
        svc.update_network(NetworkSettings::default(), ChangeSource::Gui)
            .unwrap();

        let changes = svc.store.recent_changes(10).unwrap();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn test_update_privacy() {
        let svc = test_service();
        let settings = PrivacySettings {
            telemetry_enabled: true,
            camera_enabled: false,
            ..PrivacySettings::default()
        };
        svc.update_privacy(settings, ChangeSource::Api).unwrap();
        let g = svc.state.read().unwrap();
        assert!(g.privacy.telemetry_enabled);
        assert!(!g.privacy.camera_enabled);
    }

    #[test]
    fn test_update_accessibility() {
        let svc = test_service();
        let settings = AccessibilitySettings {
            large_text: true,
            screen_reader: true,
            ..AccessibilitySettings::default()
        };
        svc.update_accessibility(settings, ChangeSource::Gui)
            .unwrap();
        let g = svc.state.read().unwrap();
        assert!(g.accessibility.large_text);
        assert!(g.accessibility.screen_reader);
    }

    #[test]
    fn test_patch_audio() {
        let svc = test_service();
        let result = svc
            .patch_audio(serde_json::json!({"master_volume": 33}), ChangeSource::Api)
            .unwrap();
        assert_eq!(result.master_volume, 33);
        assert!(!result.muted); // unchanged default
    }

    #[test]
    fn test_patch_network() {
        let svc = test_service();
        let result = svc
            .patch_network(
                serde_json::json!({"wifi_enabled": false}),
                ChangeSource::Api,
            )
            .unwrap();
        assert!(!result.wifi_enabled);
        assert!(result.bluetooth_enabled); // unchanged
    }

    #[test]
    fn test_patch_privacy() {
        let svc = test_service();
        let result = svc
            .patch_privacy(
                serde_json::json!({"telemetry_enabled": true}),
                ChangeSource::Api,
            )
            .unwrap();
        assert!(result.telemetry_enabled);
        assert!(result.screen_lock_enabled); // unchanged
    }

    #[test]
    fn test_patch_locale() {
        let svc = test_service();
        let result = svc
            .patch_locale(
                serde_json::json!({"timezone": "Europe/London"}),
                ChangeSource::Api,
            )
            .unwrap();
        assert_eq!(result.timezone, "Europe/London");
        assert_eq!(result.language, "en"); // unchanged
    }

    #[test]
    fn test_patch_power() {
        let svc = test_service();
        let result = svc
            .patch_power(
                serde_json::json!({"suspend_timeout_minutes": 15}),
                ChangeSource::Api,
            )
            .unwrap();
        assert_eq!(result.suspend_timeout_minutes, 15);
        assert_eq!(result.power_profile, PowerProfile::Balanced); // unchanged
    }

    #[test]
    fn test_patch_accessibility() {
        let svc = test_service();
        let result = svc
            .patch_accessibility(serde_json::json!({"large_text": true}), ChangeSource::Api)
            .unwrap();
        assert!(result.large_text);
        assert!(!result.screen_reader); // unchanged
    }
}
