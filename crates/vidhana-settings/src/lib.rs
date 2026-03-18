//! Vidhana Settings — Persistent settings storage
//!
//! Manages reading/writing settings to TOML config files and SQLite history.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vidhana_core::VidhanaState;

/// Settings storage backend
pub struct SettingsStore {
    config_dir: PathBuf,
    db_path: PathBuf,
}

/// A recorded settings change for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsChange {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub category: String,
    pub key: String,
    pub old_value: String,
    pub new_value: String,
    pub source: ChangeSource,
}

/// Where the change originated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeSource {
    Gui,
    Api,
    Mcp,
    Cli,
    Agent,
}

/// Settings storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Settings not found: {0}")]
    NotFound(String),
}

impl SettingsStore {
    /// Create a new settings store at the given data directory
    pub fn new(data_dir: &str) -> Result<Self, StorageError> {
        let config_dir = PathBuf::from(data_dir);
        let db_path = config_dir.join("history.db");

        std::fs::create_dir_all(&config_dir)?;

        let store = Self {
            config_dir,
            db_path,
        };
        store.init_db()?;
        Ok(store)
    }

    fn init_db(&self) -> Result<(), StorageError> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                old_value TEXT NOT NULL,
                new_value TEXT NOT NULL,
                source TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_category ON settings_history(category);
            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON settings_history(timestamp);",
        )?;
        Ok(())
    }

    /// Save current state to TOML files
    pub fn save_state(&self, state: &VidhanaState) -> Result<(), StorageError> {
        let settings_path = self.config_dir.join("settings.toml");
        let content = toml::to_string_pretty(state)?;
        std::fs::write(&settings_path, content)?;
        tracing::info!(path = %settings_path.display(), "Settings saved");
        Ok(())
    }

    /// Load state from TOML file
    pub fn load_state(&self) -> Result<Option<VidhanaState>, StorageError> {
        let settings_path = self.config_dir.join("settings.toml");
        if !settings_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&settings_path)?;
        let state: VidhanaState = toml::from_str(&content)?;
        Ok(Some(state))
    }

    /// Record a settings change in the audit history
    pub fn record_change(&self, change: &SettingsChange) -> Result<(), StorageError> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO settings_history (timestamp, category, key, old_value, new_value, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                change.timestamp.to_rfc3339(),
                change.category,
                change.key,
                change.old_value,
                change.new_value,
                serde_json::to_string(&change.source).unwrap_or_default(),
            ],
        )?;
        Ok(())
    }

    /// Get recent change history
    pub fn recent_changes(&self, limit: usize) -> Result<Vec<SettingsChange>, StorageError> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT timestamp, category, key, old_value, new_value, source
             FROM settings_history ORDER BY timestamp DESC LIMIT ?1",
        )?;

        let changes = stmt
            .query_map([limit], |row| {
                let ts_str: String = row.get(0)?;
                let source_str: String = row.get(5)?;
                Ok(SettingsChange {
                    timestamp: chrono::DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    category: row.get(1)?,
                    key: row.get(2)?,
                    old_value: row.get(3)?,
                    new_value: row.get(4)?,
                    source: serde_json::from_str(&source_str).unwrap_or(ChangeSource::Cli),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(changes)
    }

    /// Get change history for a specific category
    pub fn changes_for_category(
        &self,
        category: &str,
        limit: usize,
    ) -> Result<Vec<SettingsChange>, StorageError> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT timestamp, category, key, old_value, new_value, source
             FROM settings_history WHERE category = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;

        let changes = stmt
            .query_map(rusqlite::params![category, limit], |row| {
                let ts_str: String = row.get(0)?;
                let source_str: String = row.get(5)?;
                Ok(SettingsChange {
                    timestamp: chrono::DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    category: row.get(1)?,
                    key: row.get(2)?,
                    old_value: row.get(3)?,
                    new_value: row.get(4)?,
                    source: serde_json::from_str(&source_str).unwrap_or(ChangeSource::Cli),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(changes)
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};
    use vidhana_core::*;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> SettingsStore {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("vidhana-test-{}-{}", std::process::id(), id));
        SettingsStore::new(dir.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_store_creation() {
        let store = temp_store();
        assert!(store.config_dir().exists());
        std::fs::remove_dir_all(store.config_dir()).ok();
    }

    #[test]
    fn test_save_and_load_state() {
        let store = temp_store();
        let state = VidhanaState {
            config: vidhana_core::VidhanaConfig::default(),
            display: DisplaySettings::default(),
            audio: AudioSettings::default(),
            network: NetworkSettings::default(),
            privacy: PrivacySettings::default(),
            locale: LocaleSettings::default(),
            power: PowerSettings::default(),
            accessibility: AccessibilitySettings::default(),
        };
        store.save_state(&state).unwrap();
        let loaded = store.load_state().unwrap().unwrap();
        assert_eq!(loaded.display.brightness, 80);
        std::fs::remove_dir_all(store.config_dir()).ok();
    }

    #[test]
    fn test_load_missing_state() {
        let store = temp_store();
        let result = store.load_state().unwrap();
        assert!(result.is_none());
        std::fs::remove_dir_all(store.config_dir()).ok();
    }

    #[test]
    fn test_record_and_query_changes() {
        let store = temp_store();
        let change = SettingsChange {
            timestamp: chrono::Utc::now(),
            category: "display".to_string(),
            key: "brightness".to_string(),
            old_value: "80".to_string(),
            new_value: "100".to_string(),
            source: ChangeSource::Gui,
        };
        store.record_change(&change).unwrap();

        let history = store.recent_changes(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].key, "brightness");
        assert_eq!(history[0].new_value, "100");
        std::fs::remove_dir_all(store.config_dir()).ok();
    }

    #[test]
    fn test_changes_for_category() {
        let store = temp_store();
        for (cat, key) in [
            ("display", "brightness"),
            ("audio", "volume"),
            ("display", "theme"),
        ] {
            store
                .record_change(&SettingsChange {
                    timestamp: chrono::Utc::now(),
                    category: cat.to_string(),
                    key: key.to_string(),
                    old_value: "old".to_string(),
                    new_value: "new".to_string(),
                    source: ChangeSource::Api,
                })
                .unwrap();
        }

        let display_changes = store.changes_for_category("display", 10).unwrap();
        assert_eq!(display_changes.len(), 2);

        let audio_changes = store.changes_for_category("audio", 10).unwrap();
        assert_eq!(audio_changes.len(), 1);
        std::fs::remove_dir_all(store.config_dir()).ok();
    }

    #[test]
    fn test_change_source_serialization() {
        let json = serde_json::to_string(&ChangeSource::Mcp).unwrap();
        assert_eq!(json, "\"mcp\"");
        let parsed: ChangeSource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ChangeSource::Mcp);
    }
}
