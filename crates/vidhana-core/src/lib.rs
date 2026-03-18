//! Vidhana Core — Types and state for AGNOS system settings
//!
//! Named after Sanskrit: विधान (regulation, constitution, arrangement)

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Shared application state
pub type SharedState = Arc<RwLock<VidhanaState>>;

/// Create a new shared state from config
pub fn new_shared_state(config: VidhanaConfig) -> SharedState {
    Arc::new(RwLock::new(VidhanaState {
        config,
        display: DisplaySettings::default(),
        audio: AudioSettings::default(),
        network: NetworkSettings::default(),
        privacy: PrivacySettings::default(),
        locale: LocaleSettings::default(),
        power: PowerSettings::default(),
        accessibility: AccessibilitySettings::default(),
    }))
}

/// Top-level application state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VidhanaState {
    pub config: VidhanaConfig,
    pub display: DisplaySettings,
    pub audio: AudioSettings,
    pub network: NetworkSettings,
    pub privacy: PrivacySettings,
    pub locale: LocaleSettings,
    pub power: PowerSettings,
    pub accessibility: AccessibilitySettings,
}

impl VidhanaState {
    /// Validate and clamp all settings to their valid ranges.
    pub fn validate(&mut self) {
        self.display.validate();
        self.audio.validate();
        self.privacy.validate();
        self.power.validate();
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VidhanaConfig {
    pub daimon_url: String,
    pub hoosh_url: String,
    pub bind_address: String,
    pub port: u16,
    pub data_dir: String,
}

impl Default for VidhanaConfig {
    fn default() -> Self {
        Self {
            daimon_url: "http://127.0.0.1:8090".to_string(),
            hoosh_url: "http://127.0.0.1:8088".to_string(),
            bind_address: "127.0.0.1".to_string(),
            port: 8099,
            data_dir: dirs(),
        }
    }
}

impl VidhanaConfig {
    pub fn load() -> Self {
        let config_path = format!("{}/config.toml", dirs());
        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml_from_str(&contents) {
                return config;
            }
        }
        Self::default()
    }
}

fn dirs() -> String {
    std::env::var("VIDHANA_DATA_DIR")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{home}/.local/share/vidhana")
        })
}

fn toml_from_str(s: &str) -> Result<VidhanaConfig, toml::de::Error> {
    toml::from_str(s)
}

/// Display and appearance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub theme: Theme,
    pub brightness: u8,
    pub scaling_factor: f64,
    pub high_contrast: bool,
    pub night_light: bool,
    pub night_light_temperature: u32,
    pub refresh_rate: u32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            brightness: 80,
            scaling_factor: 1.0,
            high_contrast: false,
            night_light: false,
            night_light_temperature: 4500,
            refresh_rate: 60,
        }
    }
}

impl DisplaySettings {
    /// Clamp all numeric fields to valid ranges.
    pub fn validate(&mut self) {
        self.brightness = self.brightness.min(100);
        self.scaling_factor = self.scaling_factor.clamp(0.5, 3.0);
        self.night_light_temperature = self.night_light_temperature.clamp(1000, 10000);
        self.refresh_rate = self.refresh_rate.clamp(30, 360);
    }
}

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Light => write!(f, "light"),
            Self::Dark => write!(f, "dark"),
            Self::System => write!(f, "system"),
        }
    }
}

/// Audio and sound settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: u8,
    pub muted: bool,
    pub output_device: String,
    pub input_device: String,
    pub input_volume: u8,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 75,
            muted: false,
            output_device: "default".to_string(),
            input_device: "default".to_string(),
            input_volume: 80,
        }
    }
}

impl AudioSettings {
    /// Clamp all numeric fields to valid ranges.
    pub fn validate(&mut self) {
        self.master_volume = self.master_volume.min(100);
        self.input_volume = self.input_volume.min(100);
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub hostname: String,
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub vpn_enabled: bool,
    pub firewall_enabled: bool,
    pub dns_servers: Vec<String>,
    pub proxy: Option<ProxyConfig>,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            hostname: "agnos".to_string(),
            wifi_enabled: true,
            bluetooth_enabled: true,
            vpn_enabled: false,
            firewall_enabled: true,
            dns_servers: vec!["1.1.1.1".to_string(), "9.9.9.9".to_string()],
            proxy: None,
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub http: Option<String>,
    pub https: Option<String>,
    pub no_proxy: Vec<String>,
}

/// Privacy and security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub screen_lock_enabled: bool,
    pub screen_lock_timeout_secs: u32,
    pub location_enabled: bool,
    pub telemetry_enabled: bool,
    pub camera_enabled: bool,
    pub microphone_enabled: bool,
    pub agent_approval_required: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            screen_lock_enabled: true,
            screen_lock_timeout_secs: 300,
            location_enabled: false,
            telemetry_enabled: false,
            camera_enabled: true,
            microphone_enabled: true,
            agent_approval_required: true,
        }
    }
}

impl PrivacySettings {
    /// Clamp all numeric fields to valid ranges.
    pub fn validate(&mut self) {
        self.screen_lock_timeout_secs = self.screen_lock_timeout_secs.clamp(30, 3600);
    }
}

/// Locale and language settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSettings {
    pub language: String,
    pub region: String,
    pub timezone: String,
    pub use_24h_clock: bool,
    pub first_day_of_week: Weekday,
    pub keyboard_layout: String,
}

impl Default for LocaleSettings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            region: "US".to_string(),
            timezone: "UTC".to_string(),
            use_24h_clock: false,
            first_day_of_week: Weekday::Sunday,
            keyboard_layout: "us".to_string(),
        }
    }
}

/// Day of week
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Sunday,
    Monday,
    Saturday,
}

/// Power management settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSettings {
    pub suspend_on_lid_close: bool,
    pub suspend_timeout_minutes: u32,
    pub display_off_timeout_minutes: u32,
    pub power_profile: PowerProfile,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            suspend_on_lid_close: true,
            suspend_timeout_minutes: 30,
            display_off_timeout_minutes: 10,
            power_profile: PowerProfile::Balanced,
        }
    }
}

impl PowerSettings {
    /// Clamp all numeric fields to valid ranges.
    pub fn validate(&mut self) {
        self.suspend_timeout_minutes = self.suspend_timeout_minutes.clamp(5, 120);
        self.display_off_timeout_minutes = self.display_off_timeout_minutes.clamp(1, 60);
    }
}

/// Power profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerProfile {
    Performance,
    Balanced,
    PowerSaver,
}

impl std::fmt::Display for PowerProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Performance => write!(f, "performance"),
            Self::Balanced => write!(f, "balanced"),
            Self::PowerSaver => write!(f, "power-saver"),
        }
    }
}

/// Accessibility settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    pub large_text: bool,
    pub reduce_motion: bool,
    pub screen_reader: bool,
    pub sticky_keys: bool,
    pub cursor_size: CursorSize,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            large_text: false,
            reduce_motion: false,
            screen_reader: false,
            sticky_keys: false,
            cursor_size: CursorSize::Default,
        }
    }
}

/// Cursor size option
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorSize {
    Small,
    Default,
    Large,
    ExtraLarge,
}

/// Setting category for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsCategory {
    Display,
    Audio,
    Network,
    Privacy,
    Locale,
    Power,
    Accessibility,
}

impl std::fmt::Display for SettingsCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Display => write!(f, "display"),
            Self::Audio => write!(f, "audio"),
            Self::Network => write!(f, "network"),
            Self::Privacy => write!(f, "privacy"),
            Self::Locale => write!(f, "locale"),
            Self::Power => write!(f, "power"),
            Self::Accessibility => write!(f, "accessibility"),
        }
    }
}

impl std::str::FromStr for SettingsCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "display" | "screen" | "appearance" => Ok(Self::Display),
            "audio" | "sound" | "volume" => Ok(Self::Audio),
            "network" | "wifi" | "internet" | "bluetooth" => Ok(Self::Network),
            "privacy" | "security" | "lock" => Ok(Self::Privacy),
            "locale" | "language" | "timezone" | "keyboard" | "region" => Ok(Self::Locale),
            "power" | "battery" | "suspend" | "sleep" => Ok(Self::Power),
            "accessibility" | "a11y" => Ok(Self::Accessibility),
            other => Err(format!("unknown settings category: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_display_settings() {
        let ds = DisplaySettings::default();
        assert_eq!(ds.brightness, 80);
        assert_eq!(ds.theme, Theme::Dark);
        assert!(!ds.high_contrast);
        assert!(!ds.night_light);
        assert_eq!(ds.scaling_factor, 1.0);
    }

    #[test]
    fn test_default_audio_settings() {
        let audio = AudioSettings::default();
        assert_eq!(audio.master_volume, 75);
        assert!(!audio.muted);
        assert_eq!(audio.output_device, "default");
    }

    #[test]
    fn test_default_network_settings() {
        let net = NetworkSettings::default();
        assert!(net.wifi_enabled);
        assert!(net.firewall_enabled);
        assert!(net.bluetooth_enabled);
        assert!(!net.vpn_enabled);
        assert_eq!(net.dns_servers.len(), 2);
    }

    #[test]
    fn test_default_privacy_settings() {
        let priv_s = PrivacySettings::default();
        assert!(priv_s.screen_lock_enabled);
        assert!(!priv_s.telemetry_enabled);
        assert!(!priv_s.location_enabled);
        assert!(priv_s.agent_approval_required);
    }

    #[test]
    fn test_default_locale_settings() {
        let locale = LocaleSettings::default();
        assert_eq!(locale.language, "en");
        assert_eq!(locale.timezone, "UTC");
        assert_eq!(locale.keyboard_layout, "us");
    }

    #[test]
    fn test_default_power_settings() {
        let power = PowerSettings::default();
        assert!(power.suspend_on_lid_close);
        assert_eq!(power.power_profile, PowerProfile::Balanced);
        assert_eq!(power.suspend_timeout_minutes, 30);
    }

    #[test]
    fn test_default_accessibility_settings() {
        let a11y = AccessibilitySettings::default();
        assert!(!a11y.large_text);
        assert!(!a11y.screen_reader);
        assert_eq!(a11y.cursor_size, CursorSize::Default);
    }

    #[test]
    fn test_theme_display() {
        assert_eq!(Theme::Dark.to_string(), "dark");
        assert_eq!(Theme::Light.to_string(), "light");
        assert_eq!(Theme::System.to_string(), "system");
    }

    #[test]
    fn test_power_profile_display() {
        assert_eq!(PowerProfile::Performance.to_string(), "performance");
        assert_eq!(PowerProfile::Balanced.to_string(), "balanced");
        assert_eq!(PowerProfile::PowerSaver.to_string(), "power-saver");
    }

    #[test]
    fn test_settings_category_from_str() {
        assert_eq!("display".parse::<SettingsCategory>().unwrap(), SettingsCategory::Display);
        assert_eq!("screen".parse::<SettingsCategory>().unwrap(), SettingsCategory::Display);
        assert_eq!("sound".parse::<SettingsCategory>().unwrap(), SettingsCategory::Audio);
        assert_eq!("wifi".parse::<SettingsCategory>().unwrap(), SettingsCategory::Network);
        assert_eq!("security".parse::<SettingsCategory>().unwrap(), SettingsCategory::Privacy);
        assert_eq!("timezone".parse::<SettingsCategory>().unwrap(), SettingsCategory::Locale);
        assert_eq!("battery".parse::<SettingsCategory>().unwrap(), SettingsCategory::Power);
        assert_eq!("a11y".parse::<SettingsCategory>().unwrap(), SettingsCategory::Accessibility);
        assert!("nonsense".parse::<SettingsCategory>().is_err());
    }

    #[test]
    fn test_shared_state_creation() {
        let config = VidhanaConfig::default();
        assert_eq!(config.port, 8099);
        assert_eq!(config.daimon_url, "http://127.0.0.1:8090");
        let state = new_shared_state(config);
        let guard = state.read().unwrap();
        assert_eq!(guard.display.brightness, 80);
        assert_eq!(guard.config.port, 8099);
    }

    #[test]
    fn test_display_serialization() {
        let ds = DisplaySettings::default();
        let json = serde_json::to_string(&ds).unwrap();
        let parsed: DisplaySettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, ds.theme);
        assert_eq!(parsed.brightness, ds.brightness);
    }

    #[test]
    fn test_network_serialization() {
        let net = NetworkSettings::default();
        let json = serde_json::to_string(&net).unwrap();
        let parsed: NetworkSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.hostname, net.hostname);
        assert!(parsed.proxy.is_none());
    }

    #[test]
    fn test_proxy_config() {
        let proxy = ProxyConfig {
            http: Some("http://proxy:8080".to_string()),
            https: Some("https://proxy:8443".to_string()),
            no_proxy: vec!["localhost".to_string(), "127.0.0.1".to_string()],
        };
        let json = serde_json::to_string(&proxy).unwrap();
        let parsed: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.http, proxy.http);
        assert_eq!(parsed.no_proxy.len(), 2);
    }

    #[test]
    fn test_settings_category_display() {
        assert_eq!(SettingsCategory::Display.to_string(), "display");
        assert_eq!(SettingsCategory::Audio.to_string(), "audio");
        assert_eq!(SettingsCategory::Network.to_string(), "network");
        assert_eq!(SettingsCategory::Privacy.to_string(), "privacy");
        assert_eq!(SettingsCategory::Locale.to_string(), "locale");
        assert_eq!(SettingsCategory::Power.to_string(), "power");
        assert_eq!(SettingsCategory::Accessibility.to_string(), "accessibility");
    }

    #[test]
    fn test_display_validation_clamps() {
        let mut ds = DisplaySettings {
            brightness: 200,
            scaling_factor: 5.0,
            night_light_temperature: 500,
            refresh_rate: 0,
            ..DisplaySettings::default()
        };
        ds.validate();
        assert_eq!(ds.brightness, 100);
        assert_eq!(ds.scaling_factor, 3.0);
        assert_eq!(ds.night_light_temperature, 1000);
        assert_eq!(ds.refresh_rate, 30);
    }

    #[test]
    fn test_audio_validation_clamps() {
        let mut audio = AudioSettings {
            master_volume: 255,
            input_volume: 200,
            ..AudioSettings::default()
        };
        audio.validate();
        assert_eq!(audio.master_volume, 100);
        assert_eq!(audio.input_volume, 100);
    }

    #[test]
    fn test_privacy_validation_clamps() {
        let mut priv_s = PrivacySettings {
            screen_lock_timeout_secs: 10,
            ..PrivacySettings::default()
        };
        priv_s.validate();
        assert_eq!(priv_s.screen_lock_timeout_secs, 30);
    }

    #[test]
    fn test_power_validation_clamps() {
        let mut power = PowerSettings {
            suspend_timeout_minutes: 999,
            display_off_timeout_minutes: 0,
            ..PowerSettings::default()
        };
        power.validate();
        assert_eq!(power.suspend_timeout_minutes, 120);
        assert_eq!(power.display_off_timeout_minutes, 1);
    }

    #[test]
    fn test_state_validate() {
        let mut state = VidhanaState {
            config: VidhanaConfig::default(),
            display: DisplaySettings { brightness: 200, ..DisplaySettings::default() },
            audio: AudioSettings { master_volume: 200, ..AudioSettings::default() },
            network: NetworkSettings::default(),
            privacy: PrivacySettings { screen_lock_timeout_secs: 1, ..PrivacySettings::default() },
            locale: LocaleSettings::default(),
            power: PowerSettings { suspend_timeout_minutes: 999, ..PowerSettings::default() },
            accessibility: AccessibilitySettings::default(),
        };
        state.validate();
        assert_eq!(state.display.brightness, 100);
        assert_eq!(state.audio.master_volume, 100);
        assert_eq!(state.privacy.screen_lock_timeout_secs, 30);
        assert_eq!(state.power.suspend_timeout_minutes, 120);
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let config = VidhanaConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: VidhanaConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.port, config.port);
        assert_eq!(parsed.daimon_url, config.daimon_url);
        assert_eq!(parsed.data_dir, config.data_dir);
    }

    #[test]
    fn test_vidhana_state_serialization() {
        let state = VidhanaState {
            config: VidhanaConfig::default(),
            display: DisplaySettings::default(),
            audio: AudioSettings::default(),
            network: NetworkSettings::default(),
            privacy: PrivacySettings::default(),
            locale: LocaleSettings::default(),
            power: PowerSettings::default(),
            accessibility: AccessibilitySettings::default(),
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("display"));
        assert!(json.contains("network"));
        assert!(json.contains("privacy"));
    }
}
