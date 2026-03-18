//! Vidhana AI — Natural language settings management
//!
//! Parses NL commands like "make the screen brighter" or "turn off bluetooth"
//! into structured settings operations.

use serde::{Deserialize, Serialize};
use vidhana_core::SettingsCategory;

/// A parsed settings intent from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsIntent {
    pub category: SettingsCategory,
    pub action: SettingsAction,
    pub key: String,
    pub value: Option<String>,
    pub confidence: f32,
}

/// Actions that can be performed on settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsAction {
    Get,
    Set,
    Toggle,
    Increase,
    Decrease,
    Reset,
}

/// Parse a natural language settings command
pub fn parse_settings_command(input: &str) -> Option<SettingsIntent> {
    let lower = input.to_lowercase();
    let lower = lower.trim();

    // Brightness
    if matches_any(&lower, &["brightness", "screen bright", "display bright"]) {
        let action = detect_action(&lower);
        let value = extract_number(&lower).map(|n| n.to_string());
        return Some(SettingsIntent {
            category: SettingsCategory::Display,
            action,
            key: "brightness".to_string(),
            value,
            confidence: 0.9,
        });
    }

    // Theme
    if matches_any(&lower, &["dark mode", "dark theme", "light mode", "light theme", "theme"]) {
        let value = if lower.contains("dark") {
            Some("dark".to_string())
        } else if lower.contains("light") {
            Some("light".to_string())
        } else {
            None
        };
        return Some(SettingsIntent {
            category: SettingsCategory::Display,
            action: if value.is_some() { SettingsAction::Set } else { SettingsAction::Get },
            key: "theme".to_string(),
            value,
            confidence: 0.95,
        });
    }

    // Mute (check before volume — "mute the sound" should match mute, not volume)
    if matches_any(&lower, &["mute", "unmute", "silence"]) {
        let value = if lower.contains("unmute") { "false" } else { "true" };
        return Some(SettingsIntent {
            category: SettingsCategory::Audio,
            action: SettingsAction::Set,
            key: "muted".to_string(),
            value: Some(value.to_string()),
            confidence: 0.95,
        });
    }

    // Volume
    if matches_any(&lower, &["volume", "sound", "audio level"]) {
        let action = detect_action(&lower);
        let value = extract_number(&lower).map(|n| n.to_string());
        return Some(SettingsIntent {
            category: SettingsCategory::Audio,
            action,
            key: "master_volume".to_string(),
            value,
            confidence: 0.9,
        });
    }

    // WiFi
    if matches_any(&lower, &["wifi", "wi-fi", "wireless"]) {
        return Some(SettingsIntent {
            category: SettingsCategory::Network,
            action: detect_toggle_action(&lower),
            key: "wifi_enabled".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.9,
        });
    }

    // Bluetooth
    if lower.contains("bluetooth") {
        return Some(SettingsIntent {
            category: SettingsCategory::Network,
            action: detect_toggle_action(&lower),
            key: "bluetooth_enabled".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.9,
        });
    }

    // Firewall
    if lower.contains("firewall") {
        return Some(SettingsIntent {
            category: SettingsCategory::Network,
            action: detect_toggle_action(&lower),
            key: "firewall_enabled".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.85,
        });
    }

    // Night light / blue light
    if matches_any(&lower, &["night light", "blue light", "night mode", "night shift"]) {
        return Some(SettingsIntent {
            category: SettingsCategory::Display,
            action: detect_toggle_action(&lower),
            key: "night_light".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.9,
        });
    }

    // Screen lock
    if matches_any(&lower, &["screen lock", "lock screen", "auto lock"]) {
        return Some(SettingsIntent {
            category: SettingsCategory::Privacy,
            action: detect_toggle_action(&lower),
            key: "screen_lock_enabled".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.85,
        });
    }

    // Timezone
    if matches_any(&lower, &["timezone", "time zone"]) {
        let value = extract_timezone(&lower);
        return Some(SettingsIntent {
            category: SettingsCategory::Locale,
            action: if value.is_some() { SettingsAction::Set } else { SettingsAction::Get },
            key: "timezone".to_string(),
            value,
            confidence: 0.85,
        });
    }

    // Language
    if matches_any(&lower, &["language", "lang "]) && !lower.contains("keyboard") {
        return Some(SettingsIntent {
            category: SettingsCategory::Locale,
            action: SettingsAction::Get,
            key: "language".to_string(),
            value: None,
            confidence: 0.7,
        });
    }

    // Power profile
    if matches_any(&lower, &["power saver", "power save", "battery saver", "performance mode", "balanced mode", "power profile"]) {
        let value = if lower.contains("performance") {
            Some("performance".to_string())
        } else if lower.contains("saver") || lower.contains("save") {
            Some("power-saver".to_string())
        } else if lower.contains("balanced") {
            Some("balanced".to_string())
        } else {
            None
        };
        return Some(SettingsIntent {
            category: SettingsCategory::Power,
            action: if value.is_some() { SettingsAction::Set } else { SettingsAction::Get },
            key: "power_profile".to_string(),
            value,
            confidence: 0.9,
        });
    }

    // Large text / accessibility
    if matches_any(&lower, &["large text", "big text", "larger font", "bigger font"]) {
        return Some(SettingsIntent {
            category: SettingsCategory::Accessibility,
            action: detect_toggle_action(&lower),
            key: "large_text".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.85,
        });
    }

    // Screen reader
    if matches_any(&lower, &["screen reader"]) {
        return Some(SettingsIntent {
            category: SettingsCategory::Accessibility,
            action: detect_toggle_action(&lower),
            key: "screen_reader".to_string(),
            value: detect_toggle_value(&lower),
            confidence: 0.9,
        });
    }

    None
}

fn matches_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| input.contains(p))
}

fn detect_action(input: &str) -> SettingsAction {
    if matches_any(input, &["increase", "raise", "higher", "up", "more", "brighter", "louder"]) {
        SettingsAction::Increase
    } else if matches_any(input, &["decrease", "lower", "down", "less", "dimmer", "quieter", "reduce"]) {
        SettingsAction::Decrease
    } else if matches_any(input, &["set", "change", "make", "put"]) {
        SettingsAction::Set
    } else if matches_any(input, &["reset", "default"]) {
        SettingsAction::Reset
    } else if matches_any(input, &["what", "show", "get", "current", "check"]) {
        SettingsAction::Get
    } else {
        SettingsAction::Set
    }
}

fn detect_toggle_action(input: &str) -> SettingsAction {
    if matches_any(input, &["toggle", "switch"]) {
        SettingsAction::Toggle
    } else if matches_any(input, &["enable", "turn on", "activate", "start"]) {
        SettingsAction::Set
    } else if matches_any(input, &["disable", "turn off", "deactivate", "stop"]) {
        SettingsAction::Set
    } else if matches_any(input, &["what", "show", "get", "check", "is"]) {
        SettingsAction::Get
    } else {
        SettingsAction::Toggle
    }
}

fn detect_toggle_value(input: &str) -> Option<String> {
    if matches_any(input, &["enable", "turn on", "activate", "start"]) {
        Some("true".to_string())
    } else if matches_any(input, &["disable", "turn off", "deactivate", "stop"]) {
        Some("false".to_string())
    } else {
        None
    }
}

fn extract_number(input: &str) -> Option<u32> {
    input.split_whitespace()
        .find_map(|word| word.trim_end_matches('%').parse::<u32>().ok())
}

fn extract_timezone(input: &str) -> Option<String> {
    // Look for common timezone patterns like UTC, US/Eastern, America/New_York
    let words: Vec<&str> = input.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if *word == "to" || *word == "=" {
            if let Some(tz) = words.get(i + 1) {
                return Some(tz.to_string());
            }
        }
        if word.contains('/') && word.len() > 3 {
            return Some(word.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_brightness_increase() {
        let intent = parse_settings_command("make the screen brighter").unwrap();
        assert_eq!(intent.category, SettingsCategory::Display);
        assert_eq!(intent.action, SettingsAction::Increase);
        assert_eq!(intent.key, "brightness");
    }

    #[test]
    fn test_parse_brightness_set() {
        let intent = parse_settings_command("set brightness to 100").unwrap();
        assert_eq!(intent.category, SettingsCategory::Display);
        assert_eq!(intent.key, "brightness");
        assert_eq!(intent.value, Some("100".to_string()));
    }

    #[test]
    fn test_parse_dark_mode() {
        let intent = parse_settings_command("switch to dark mode").unwrap();
        assert_eq!(intent.category, SettingsCategory::Display);
        assert_eq!(intent.key, "theme");
        assert_eq!(intent.value, Some("dark".to_string()));
    }

    #[test]
    fn test_parse_light_theme() {
        let intent = parse_settings_command("enable light theme").unwrap();
        assert_eq!(intent.category, SettingsCategory::Display);
        assert_eq!(intent.value, Some("light".to_string()));
    }

    #[test]
    fn test_parse_volume_up() {
        let intent = parse_settings_command("increase the volume").unwrap();
        assert_eq!(intent.category, SettingsCategory::Audio);
        assert_eq!(intent.action, SettingsAction::Increase);
        assert_eq!(intent.key, "master_volume");
    }

    #[test]
    fn test_parse_mute() {
        let intent = parse_settings_command("mute the sound").unwrap();
        assert_eq!(intent.category, SettingsCategory::Audio);
        assert_eq!(intent.key, "muted");
        assert_eq!(intent.value, Some("true".to_string()));
    }

    #[test]
    fn test_parse_unmute() {
        let intent = parse_settings_command("unmute").unwrap();
        assert_eq!(intent.key, "muted");
        assert_eq!(intent.value, Some("false".to_string()));
    }

    #[test]
    fn test_parse_wifi_off() {
        let intent = parse_settings_command("turn off wifi").unwrap();
        assert_eq!(intent.category, SettingsCategory::Network);
        assert_eq!(intent.key, "wifi_enabled");
        assert_eq!(intent.value, Some("false".to_string()));
    }

    #[test]
    fn test_parse_bluetooth_on() {
        let intent = parse_settings_command("enable bluetooth").unwrap();
        assert_eq!(intent.category, SettingsCategory::Network);
        assert_eq!(intent.key, "bluetooth_enabled");
        assert_eq!(intent.value, Some("true".to_string()));
    }

    #[test]
    fn test_parse_night_light() {
        let intent = parse_settings_command("turn on night light").unwrap();
        assert_eq!(intent.category, SettingsCategory::Display);
        assert_eq!(intent.key, "night_light");
        assert_eq!(intent.value, Some("true".to_string()));
    }

    #[test]
    fn test_parse_power_saver() {
        let intent = parse_settings_command("switch to power saver").unwrap();
        assert_eq!(intent.category, SettingsCategory::Power);
        assert_eq!(intent.key, "power_profile");
        assert_eq!(intent.value, Some("power-saver".to_string()));
    }

    #[test]
    fn test_parse_performance_mode() {
        let intent = parse_settings_command("enable performance mode").unwrap();
        assert_eq!(intent.category, SettingsCategory::Power);
        assert_eq!(intent.value, Some("performance".to_string()));
    }

    #[test]
    fn test_parse_large_text() {
        let intent = parse_settings_command("enable large text").unwrap();
        assert_eq!(intent.category, SettingsCategory::Accessibility);
        assert_eq!(intent.key, "large_text");
    }

    #[test]
    fn test_parse_screen_reader() {
        let intent = parse_settings_command("turn on screen reader").unwrap();
        assert_eq!(intent.category, SettingsCategory::Accessibility);
        assert_eq!(intent.key, "screen_reader");
        assert_eq!(intent.value, Some("true".to_string()));
    }

    #[test]
    fn test_parse_firewall() {
        let intent = parse_settings_command("disable firewall").unwrap();
        assert_eq!(intent.category, SettingsCategory::Network);
        assert_eq!(intent.key, "firewall_enabled");
        assert_eq!(intent.value, Some("false".to_string()));
    }

    #[test]
    fn test_parse_unknown_returns_none() {
        assert!(parse_settings_command("hello world").is_none());
        assert!(parse_settings_command("what time is it").is_none());
    }

    #[test]
    fn test_extract_number() {
        assert_eq!(extract_number("set to 75%"), Some(75));
        assert_eq!(extract_number("brightness 50"), Some(50));
        assert_eq!(extract_number("no numbers here"), None);
    }

    #[test]
    fn test_settings_action_serialization() {
        let json = serde_json::to_string(&SettingsAction::Toggle).unwrap();
        assert_eq!(json, "\"toggle\"");
    }

    #[test]
    fn test_settings_intent_serialization() {
        let intent = parse_settings_command("set brightness to 80").unwrap();
        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("brightness"));
    }
}
