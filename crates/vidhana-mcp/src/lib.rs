//! Vidhana MCP — Model Context Protocol tool server
//!
//! Provides 5 MCP tools for agent-driven system settings queries:
//! - vidhana_display: Get/set display settings
//! - vidhana_audio: Get/set audio settings
//! - vidhana_network: Get/set network settings
//! - vidhana_privacy: Get/set privacy settings
//! - vidhana_system: Get system info, power, locale, accessibility

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vidhana_core::*;
use vidhana_settings::{ChangeSource, SettingsChange, SettingsStore};

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tool call request
#[derive(Debug, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP tool call result
#[derive(Debug, Serialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

/// MCP content block
#[derive(Debug, Serialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl McpToolResult {
    pub fn success(text: String) -> Self {
        Self {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: false,
        }
    }

    pub fn error(text: String) -> Self {
        Self {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: true,
        }
    }
}

/// List all available Vidhana MCP tools
pub fn list_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "vidhana_display".to_string(),
            description:
                "Get or set AGNOS display settings (brightness, theme, scaling, night light)"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["get", "set"], "default": "get" },
                    "brightness": { "type": "integer", "minimum": 0, "maximum": 100 },
                    "theme": { "type": "string", "enum": ["light", "dark", "system"] },
                    "night_light": { "type": "boolean" }
                }
            }),
        },
        McpTool {
            name: "vidhana_audio".to_string(),
            description: "Get or set AGNOS audio settings (volume, mute, output device)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["get", "set"], "default": "get" },
                    "volume": { "type": "integer", "minimum": 0, "maximum": 100 },
                    "muted": { "type": "boolean" }
                }
            }),
        },
        McpTool {
            name: "vidhana_network".to_string(),
            description: "Get or set AGNOS network settings (WiFi, Bluetooth, firewall, DNS)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["get", "set"], "default": "get" },
                    "wifi_enabled": { "type": "boolean" },
                    "bluetooth_enabled": { "type": "boolean" },
                    "firewall_enabled": { "type": "boolean" }
                }
            }),
        },
        McpTool {
            name: "vidhana_privacy".to_string(),
            description: "Get or set AGNOS privacy settings (screen lock, telemetry, camera, mic)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["get", "set"], "default": "get" },
                    "screen_lock_enabled": { "type": "boolean" },
                    "telemetry_enabled": { "type": "boolean" },
                    "camera_enabled": { "type": "boolean" },
                    "microphone_enabled": { "type": "boolean" }
                }
            }),
        },
        McpTool {
            name: "vidhana_system".to_string(),
            description:
                "Get or set AGNOS system settings (power profile, locale, timezone, accessibility)"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["get", "set"], "default": "get" },
                    "category": { "type": "string", "enum": ["power", "locale", "accessibility"] },
                    "power_profile": { "type": "string", "enum": ["performance", "balanced", "power-saver"] },
                    "timezone": { "type": "string" },
                    "language": { "type": "string" }
                }
            }),
        },
        McpTool {
            name: "vidhana_history".to_string(),
            description: "Query recent settings change history with optional category filter"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "category": { "type": "string", "enum": ["display", "audio", "network", "privacy", "power", "locale", "accessibility"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
                }
            }),
        },
    ]
}

/// Build the MCP initialize response
pub fn initialize_response() -> serde_json::Value {
    serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "vidhana",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

/// Handle an MCP tool call
pub fn handle_tool_call(
    call: &McpToolCall,
    state: &SharedState,
    store: &Arc<SettingsStore>,
) -> McpToolResult {
    match call.name.as_str() {
        "vidhana_display" => handle_display(call, state, store),
        "vidhana_audio" => handle_audio(call, state, store),
        "vidhana_network" => handle_network(call, state, store),
        "vidhana_privacy" => handle_privacy(call, state, store),
        "vidhana_system" => handle_system(call, state, store),
        "vidhana_history" => handle_history(call, store),
        _ => McpToolResult::error(format!("Unknown tool: {}", call.name)),
    }
}

fn persist_and_record(
    state: &SharedState,
    store: &Arc<SettingsStore>,
    category: &str,
    key: &str,
    old_value: &str,
    new_value: &str,
) {
    let guard = state.read().unwrap();
    if let Err(e) = store.save_state(&guard) {
        tracing::error!("MCP: failed to persist settings: {e}");
    }
    drop(guard);
    let change = SettingsChange {
        timestamp: chrono::Utc::now(),
        category: category.to_string(),
        key: key.to_string(),
        old_value: old_value.to_string(),
        new_value: new_value.to_string(),
        source: ChangeSource::Mcp,
    };
    if let Err(e) = store.record_change(&change) {
        tracing::error!("MCP: failed to record change: {e}");
    }
}

fn handle_display(
    call: &McpToolCall,
    state: &SharedState,
    store: &Arc<SettingsStore>,
) -> McpToolResult {
    let action = call
        .arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("get");

    if action == "set" {
        let mut guard = state.write().unwrap();
        let old = serde_json::to_string(&guard.display).unwrap_or_default();
        if let Some(b) = call.arguments.get("brightness").and_then(|v| v.as_u64()) {
            guard.display.brightness = b as u8;
        }
        if let Some(t) = call.arguments.get("theme").and_then(|v| v.as_str()) {
            guard.display.theme = match t {
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                _ => Theme::System,
            };
        }
        if let Some(nl) = call.arguments.get("night_light").and_then(|v| v.as_bool()) {
            guard.display.night_light = nl;
        }
        guard.display.validate();
        let new = serde_json::to_string(&guard.display).unwrap_or_default();
        drop(guard);
        persist_and_record(state, store, "display", "*", &old, &new);
        McpToolResult::success("Display settings updated".to_string())
    } else {
        let guard = state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.display).unwrap())
    }
}

fn handle_audio(
    call: &McpToolCall,
    state: &SharedState,
    store: &Arc<SettingsStore>,
) -> McpToolResult {
    let action = call
        .arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("get");

    if action == "set" {
        let mut guard = state.write().unwrap();
        let old = serde_json::to_string(&guard.audio).unwrap_or_default();
        if let Some(v) = call.arguments.get("volume").and_then(|v| v.as_u64()) {
            guard.audio.master_volume = v as u8;
        }
        if let Some(m) = call.arguments.get("muted").and_then(|v| v.as_bool()) {
            guard.audio.muted = m;
        }
        guard.audio.validate();
        let new = serde_json::to_string(&guard.audio).unwrap_or_default();
        drop(guard);
        persist_and_record(state, store, "audio", "*", &old, &new);
        McpToolResult::success("Audio settings updated".to_string())
    } else {
        let guard = state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.audio).unwrap())
    }
}

fn handle_network(
    call: &McpToolCall,
    state: &SharedState,
    store: &Arc<SettingsStore>,
) -> McpToolResult {
    let action = call
        .arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("get");

    if action == "set" {
        let mut guard = state.write().unwrap();
        let old = serde_json::to_string(&guard.network).unwrap_or_default();
        if let Some(w) = call.arguments.get("wifi_enabled").and_then(|v| v.as_bool()) {
            guard.network.wifi_enabled = w;
        }
        if let Some(b) = call
            .arguments
            .get("bluetooth_enabled")
            .and_then(|v| v.as_bool())
        {
            guard.network.bluetooth_enabled = b;
        }
        if let Some(f) = call
            .arguments
            .get("firewall_enabled")
            .and_then(|v| v.as_bool())
        {
            guard.network.firewall_enabled = f;
        }
        let new = serde_json::to_string(&guard.network).unwrap_or_default();
        drop(guard);
        persist_and_record(state, store, "network", "*", &old, &new);
        McpToolResult::success("Network settings updated".to_string())
    } else {
        let guard = state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.network).unwrap())
    }
}

fn handle_privacy(
    call: &McpToolCall,
    state: &SharedState,
    store: &Arc<SettingsStore>,
) -> McpToolResult {
    let action = call
        .arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("get");

    if action == "set" {
        let mut guard = state.write().unwrap();
        let old = serde_json::to_string(&guard.privacy).unwrap_or_default();
        if let Some(sl) = call
            .arguments
            .get("screen_lock_enabled")
            .and_then(|v| v.as_bool())
        {
            guard.privacy.screen_lock_enabled = sl;
        }
        if let Some(t) = call
            .arguments
            .get("telemetry_enabled")
            .and_then(|v| v.as_bool())
        {
            guard.privacy.telemetry_enabled = t;
        }
        if let Some(c) = call
            .arguments
            .get("camera_enabled")
            .and_then(|v| v.as_bool())
        {
            guard.privacy.camera_enabled = c;
        }
        if let Some(m) = call
            .arguments
            .get("microphone_enabled")
            .and_then(|v| v.as_bool())
        {
            guard.privacy.microphone_enabled = m;
        }
        guard.privacy.validate();
        let new = serde_json::to_string(&guard.privacy).unwrap_or_default();
        drop(guard);
        persist_and_record(state, store, "privacy", "*", &old, &new);
        McpToolResult::success("Privacy settings updated".to_string())
    } else {
        let guard = state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.privacy).unwrap())
    }
}

fn handle_system(
    call: &McpToolCall,
    state: &SharedState,
    store: &Arc<SettingsStore>,
) -> McpToolResult {
    let action = call
        .arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("get");
    let category = call
        .arguments
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("power");

    if action == "set" {
        let mut guard = state.write().unwrap();
        match category {
            "power" => {
                let old = serde_json::to_string(&guard.power).unwrap_or_default();
                if let Some(pp) = call.arguments.get("power_profile").and_then(|v| v.as_str()) {
                    guard.power.power_profile = match pp {
                        "performance" => PowerProfile::Performance,
                        "power-saver" => PowerProfile::PowerSaver,
                        _ => PowerProfile::Balanced,
                    };
                }
                guard.power.validate();
                let new = serde_json::to_string(&guard.power).unwrap_or_default();
                drop(guard);
                persist_and_record(state, store, "power", "*", &old, &new);
            }
            "locale" => {
                let old = serde_json::to_string(&guard.locale).unwrap_or_default();
                if let Some(tz) = call.arguments.get("timezone").and_then(|v| v.as_str()) {
                    guard.locale.timezone = tz.to_string();
                }
                if let Some(lang) = call.arguments.get("language").and_then(|v| v.as_str()) {
                    guard.locale.language = lang.to_string();
                }
                let new = serde_json::to_string(&guard.locale).unwrap_or_default();
                drop(guard);
                persist_and_record(state, store, "locale", "*", &old, &new);
            }
            "accessibility" => {
                let old = serde_json::to_string(&guard.accessibility).unwrap_or_default();
                // Accessibility fields are booleans — no extra validation needed
                let new = serde_json::to_string(&guard.accessibility).unwrap_or_default();
                drop(guard);
                persist_and_record(state, store, "accessibility", "*", &old, &new);
            }
            _ => return McpToolResult::error(format!("Unknown system category: {category}")),
        }
        McpToolResult::success(format!("{category} settings updated"))
    } else {
        let guard = state.read().unwrap();
        let result = match category {
            "power" => serde_json::to_string_pretty(&guard.power).unwrap(),
            "locale" => serde_json::to_string_pretty(&guard.locale).unwrap(),
            "accessibility" => serde_json::to_string_pretty(&guard.accessibility).unwrap(),
            _ => return McpToolResult::error(format!("Unknown system category: {category}")),
        };
        McpToolResult::success(result)
    }
}

fn handle_history(call: &McpToolCall, store: &Arc<SettingsStore>) -> McpToolResult {
    let limit = call
        .arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let category = call.arguments.get("category").and_then(|v| v.as_str());

    let changes = if let Some(cat) = category {
        store.changes_for_category(cat, limit)
    } else {
        store.recent_changes(limit)
    };

    match changes {
        Ok(changes) if changes.is_empty() => {
            McpToolResult::success("No changes recorded yet.".to_string())
        }
        Ok(changes) => {
            let lines: Vec<String> = changes
                .iter()
                .map(|c| {
                    format!(
                        "[{}] {} ({:?}) key={} old={} new={}",
                        c.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        c.category,
                        c.source,
                        c.key,
                        c.old_value,
                        c.new_value
                    )
                })
                .collect();
            McpToolResult::success(lines.join("\n"))
        }
        Err(e) => McpToolResult::error(format!("Failed to query history: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_store() -> Arc<SettingsStore> {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("vidhana-mcp-test-{}-{}", std::process::id(), id));
        Arc::new(SettingsStore::new(dir.to_str().unwrap()).unwrap())
    }

    fn test_state() -> SharedState {
        new_shared_state(VidhanaConfig::default())
    }

    #[test]
    fn test_list_tools() {
        let tools = list_tools();
        assert_eq!(tools.len(), 6);
        assert_eq!(tools[0].name, "vidhana_display");
        assert_eq!(tools[1].name, "vidhana_audio");
        assert_eq!(tools[2].name, "vidhana_network");
        assert_eq!(tools[3].name, "vidhana_privacy");
        assert_eq!(tools[4].name, "vidhana_system");
        assert_eq!(tools[5].name, "vidhana_history");
    }

    #[test]
    fn test_display_get() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "get"}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("brightness"));
    }

    #[test]
    fn test_display_set_brightness() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "set", "brightness": 100}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert_eq!(state.read().unwrap().display.brightness, 100);
        // Verify persisted
        let loaded = store.load_state().unwrap().unwrap();
        assert_eq!(loaded.display.brightness, 100);
    }

    #[test]
    fn test_display_set_theme() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "set", "theme": "light"}),
        };
        handle_tool_call(&call, &state, &store);
        assert_eq!(state.read().unwrap().display.theme, Theme::Light);
    }

    #[test]
    fn test_audio_get() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_audio".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("master_volume"));
    }

    #[test]
    fn test_audio_set_volume() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_audio".to_string(),
            arguments: serde_json::json!({"action": "set", "volume": 50}),
        };
        handle_tool_call(&call, &state, &store);
        assert_eq!(state.read().unwrap().audio.master_volume, 50);
    }

    #[test]
    fn test_audio_mute() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_audio".to_string(),
            arguments: serde_json::json!({"action": "set", "muted": true}),
        };
        handle_tool_call(&call, &state, &store);
        assert!(state.read().unwrap().audio.muted);
    }

    #[test]
    fn test_network_get() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_network".to_string(),
            arguments: serde_json::json!({"action": "get"}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("wifi_enabled"));
    }

    #[test]
    fn test_network_disable_wifi() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_network".to_string(),
            arguments: serde_json::json!({"action": "set", "wifi_enabled": false}),
        };
        handle_tool_call(&call, &state, &store);
        assert!(!state.read().unwrap().network.wifi_enabled);
    }

    #[test]
    fn test_privacy_get() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_privacy".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
    }

    #[test]
    fn test_privacy_disable_telemetry() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_privacy".to_string(),
            arguments: serde_json::json!({"action": "set", "telemetry_enabled": true}),
        };
        handle_tool_call(&call, &state, &store);
        assert!(state.read().unwrap().privacy.telemetry_enabled);
    }

    #[test]
    fn test_system_power_profile() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_system".to_string(),
            arguments: serde_json::json!({"action": "set", "category": "power", "power_profile": "performance"}),
        };
        handle_tool_call(&call, &state, &store);
        assert_eq!(
            state.read().unwrap().power.power_profile,
            PowerProfile::Performance
        );
    }

    #[test]
    fn test_system_locale_timezone() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_system".to_string(),
            arguments: serde_json::json!({"action": "set", "category": "locale", "timezone": "America/New_York"}),
        };
        handle_tool_call(&call, &state, &store);
        assert_eq!(state.read().unwrap().locale.timezone, "America/New_York");
    }

    #[test]
    fn test_system_get_power() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_system".to_string(),
            arguments: serde_json::json!({"action": "get", "category": "power"}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("power_profile"));
    }

    #[test]
    fn test_unknown_tool() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_unknown".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(result.is_error);
    }

    #[test]
    fn test_tool_schemas_valid_json() {
        for tool in list_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema.get("properties").is_some());
        }
    }

    #[test]
    fn test_mcp_set_records_history() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "set", "brightness": 42}),
        };
        handle_tool_call(&call, &state, &store);
        let changes = store.recent_changes(10).unwrap();
        assert!(!changes.is_empty());
        assert_eq!(changes[0].category, "display");
        assert_eq!(changes[0].source, ChangeSource::Mcp);
    }

    #[test]
    fn test_history_tool_empty() {
        let state = test_state();
        let store = test_store();
        let call = McpToolCall {
            name: "vidhana_history".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("No changes"));
    }

    #[test]
    fn test_history_tool_with_changes() {
        let state = test_state();
        let store = test_store();
        // Make a change first
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "set", "brightness": 42}),
        };
        handle_tool_call(&call, &state, &store);
        // Query history
        let call = McpToolCall {
            name: "vidhana_history".to_string(),
            arguments: serde_json::json!({"limit": 10}),
        };
        let result = handle_tool_call(&call, &state, &store);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("display"));
    }

    #[test]
    fn test_history_tool_category_filter() {
        let state = test_state();
        let store = test_store();
        // Make changes in two categories
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_display".to_string(),
                arguments: serde_json::json!({"action": "set", "brightness": 42}),
            },
            &state,
            &store,
        );
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_audio".to_string(),
                arguments: serde_json::json!({"action": "set", "volume": 50}),
            },
            &state,
            &store,
        );
        // Query only audio
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_history".to_string(),
                arguments: serde_json::json!({"category": "audio"}),
            },
            &state,
            &store,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("audio"));
        assert!(!result.content[0].text.contains("display"));
    }

    #[test]
    fn test_initialize_response() {
        let resp = initialize_response();
        assert_eq!(resp["protocolVersion"], "2024-11-05");
        assert!(resp["capabilities"]["tools"].is_object());
        assert_eq!(resp["serverInfo"]["name"], "vidhana");
    }
}
