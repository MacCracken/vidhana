//! Vidhana MCP — Model Context Protocol tool server
//!
//! Provides 6 MCP tools for agent-driven system settings management.
//! All mutations go through `SettingsService` for consistent
//! validation, OS backend application, persistence, and auditing.

use std::collections::HashMap;
use std::sync::Arc;

use bote::{ToolDef, ToolSchema};
use serde::{Deserialize, Serialize};
use vidhana_backend::SettingsService;
use vidhana_core::*;
use vidhana_settings::ChangeSource;

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
pub fn list_tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "vidhana_display",
            "Get or set AGNOS display settings (brightness, theme, scaling, night light)",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("action".into(), serde_json::json!({ "type": "string", "enum": ["get", "set"], "default": "get" })),
                    ("brightness".into(), serde_json::json!({ "type": "integer", "minimum": 0, "maximum": 100 })),
                    ("theme".into(), serde_json::json!({ "type": "string", "enum": ["light", "dark", "system"] })),
                    ("night_light".into(), serde_json::json!({ "type": "boolean" })),
                ]),
                vec![],
            ),
        ),
        ToolDef::new(
            "vidhana_audio",
            "Get or set AGNOS audio settings (volume, mute, output device)",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("action".into(), serde_json::json!({ "type": "string", "enum": ["get", "set"], "default": "get" })),
                    ("volume".into(), serde_json::json!({ "type": "integer", "minimum": 0, "maximum": 100 })),
                    ("muted".into(), serde_json::json!({ "type": "boolean" })),
                ]),
                vec![],
            ),
        ),
        ToolDef::new(
            "vidhana_network",
            "Get or set AGNOS network settings (WiFi, Bluetooth, firewall, DNS)",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("action".into(), serde_json::json!({ "type": "string", "enum": ["get", "set"], "default": "get" })),
                    ("wifi_enabled".into(), serde_json::json!({ "type": "boolean" })),
                    ("bluetooth_enabled".into(), serde_json::json!({ "type": "boolean" })),
                    ("firewall_enabled".into(), serde_json::json!({ "type": "boolean" })),
                ]),
                vec![],
            ),
        ),
        ToolDef::new(
            "vidhana_privacy",
            "Get or set AGNOS privacy settings (screen lock, telemetry, camera, mic)",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("action".into(), serde_json::json!({ "type": "string", "enum": ["get", "set"], "default": "get" })),
                    ("screen_lock_enabled".into(), serde_json::json!({ "type": "boolean" })),
                    ("telemetry_enabled".into(), serde_json::json!({ "type": "boolean" })),
                    ("camera_enabled".into(), serde_json::json!({ "type": "boolean" })),
                    ("microphone_enabled".into(), serde_json::json!({ "type": "boolean" })),
                ]),
                vec![],
            ),
        ),
        ToolDef::new(
            "vidhana_system",
            "Get or set AGNOS system settings (power profile, locale, timezone, accessibility)",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("action".into(), serde_json::json!({ "type": "string", "enum": ["get", "set"], "default": "get" })),
                    ("category".into(), serde_json::json!({ "type": "string", "enum": ["power", "locale", "accessibility"] })),
                    ("power_profile".into(), serde_json::json!({ "type": "string", "enum": ["performance", "balanced", "power-saver"] })),
                    ("timezone".into(), serde_json::json!({ "type": "string" })),
                    ("language".into(), serde_json::json!({ "type": "string" })),
                ]),
                vec![],
            ),
        ),
        ToolDef::new(
            "vidhana_history",
            "Query recent settings change history with optional category filter",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("category".into(), serde_json::json!({ "type": "string", "enum": ["display", "audio", "network", "privacy", "power", "locale", "accessibility"] })),
                    ("limit".into(), serde_json::json!({ "type": "integer", "minimum": 1, "maximum": 100, "default": 20 })),
                ]),
                vec![],
            ),
        ),
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

/// Handle an MCP tool call via SettingsService
pub fn handle_tool_call(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    match call.name.as_str() {
        "vidhana_display" => handle_display(call, service),
        "vidhana_audio" => handle_audio(call, service),
        "vidhana_network" => handle_network(call, service),
        "vidhana_privacy" => handle_privacy(call, service),
        "vidhana_system" => handle_system(call, service),
        "vidhana_history" => handle_history(call, service),
        _ => McpToolResult::error(format!("Unknown tool: {}", call.name)),
    }
}

fn get_action(call: &McpToolCall) -> &str {
    call.arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("get")
}

fn handle_display(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    if get_action(call) == "set" {
        let guard = service.state.read().unwrap();
        let mut display = guard.display.clone();
        drop(guard);

        if let Some(b) = call.arguments.get("brightness").and_then(|v| v.as_u64()) {
            display.brightness = b as u8;
        }
        if let Some(t) = call.arguments.get("theme").and_then(|v| v.as_str()) {
            display.theme = match t {
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                _ => Theme::System,
            };
        }
        if let Some(nl) = call.arguments.get("night_light").and_then(|v| v.as_bool()) {
            display.night_light = nl;
        }

        match service.update_display(display, ChangeSource::Mcp) {
            Ok(()) => McpToolResult::success("Display settings updated".to_string()),
            Err(e) => McpToolResult::error(e.to_string()),
        }
    } else {
        let guard = service.state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.display).unwrap())
    }
}

fn handle_audio(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    if get_action(call) == "set" {
        let mut audio = service.state.read().unwrap().audio.clone();

        if let Some(v) = call.arguments.get("volume").and_then(|v| v.as_u64()) {
            audio.master_volume = v as u8;
        }
        if let Some(m) = call.arguments.get("muted").and_then(|v| v.as_bool()) {
            audio.muted = m;
        }

        match service.update_audio(audio, ChangeSource::Mcp) {
            Ok(()) => McpToolResult::success("Audio settings updated".to_string()),
            Err(e) => McpToolResult::error(e.to_string()),
        }
    } else {
        let guard = service.state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.audio).unwrap())
    }
}

fn handle_network(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    if get_action(call) == "set" {
        let mut network = service.state.read().unwrap().network.clone();

        if let Some(w) = call.arguments.get("wifi_enabled").and_then(|v| v.as_bool()) {
            network.wifi_enabled = w;
        }
        if let Some(b) = call
            .arguments
            .get("bluetooth_enabled")
            .and_then(|v| v.as_bool())
        {
            network.bluetooth_enabled = b;
        }
        if let Some(f) = call
            .arguments
            .get("firewall_enabled")
            .and_then(|v| v.as_bool())
        {
            network.firewall_enabled = f;
        }

        match service.update_network(network, ChangeSource::Mcp) {
            Ok(()) => McpToolResult::success("Network settings updated".to_string()),
            Err(e) => McpToolResult::error(e.to_string()),
        }
    } else {
        let guard = service.state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.network).unwrap())
    }
}

fn handle_privacy(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    if get_action(call) == "set" {
        let mut privacy = service.state.read().unwrap().privacy.clone();

        if let Some(v) = call
            .arguments
            .get("screen_lock_enabled")
            .and_then(|v| v.as_bool())
        {
            privacy.screen_lock_enabled = v;
        }
        if let Some(v) = call
            .arguments
            .get("telemetry_enabled")
            .and_then(|v| v.as_bool())
        {
            privacy.telemetry_enabled = v;
        }
        if let Some(v) = call
            .arguments
            .get("camera_enabled")
            .and_then(|v| v.as_bool())
        {
            privacy.camera_enabled = v;
        }
        if let Some(v) = call
            .arguments
            .get("microphone_enabled")
            .and_then(|v| v.as_bool())
        {
            privacy.microphone_enabled = v;
        }

        match service.update_privacy(privacy, ChangeSource::Mcp) {
            Ok(()) => McpToolResult::success("Privacy settings updated".to_string()),
            Err(e) => McpToolResult::error(e.to_string()),
        }
    } else {
        let guard = service.state.read().unwrap();
        McpToolResult::success(serde_json::to_string_pretty(&guard.privacy).unwrap())
    }
}

fn handle_system(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    let category = call
        .arguments
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("power");

    if get_action(call) == "set" {
        let guard = service.state.read().unwrap();
        match category {
            "power" => {
                let mut power = guard.power.clone();
                drop(guard);
                if let Some(pp) = call.arguments.get("power_profile").and_then(|v| v.as_str()) {
                    power.power_profile = match pp {
                        "performance" => PowerProfile::Performance,
                        "power-saver" => PowerProfile::PowerSaver,
                        _ => PowerProfile::Balanced,
                    };
                }
                match service.update_power(power, ChangeSource::Mcp) {
                    Ok(()) => McpToolResult::success("power settings updated".to_string()),
                    Err(e) => McpToolResult::error(e.to_string()),
                }
            }
            "locale" => {
                let mut locale = guard.locale.clone();
                drop(guard);
                if let Some(tz) = call.arguments.get("timezone").and_then(|v| v.as_str()) {
                    locale.timezone = tz.to_string();
                }
                if let Some(lang) = call.arguments.get("language").and_then(|v| v.as_str()) {
                    locale.language = lang.to_string();
                }
                match service.update_locale(locale, ChangeSource::Mcp) {
                    Ok(()) => McpToolResult::success("locale settings updated".to_string()),
                    Err(e) => McpToolResult::error(e.to_string()),
                }
            }
            "accessibility" => {
                let a11y = guard.accessibility.clone();
                drop(guard);
                match service.update_accessibility(a11y, ChangeSource::Mcp) {
                    Ok(()) => McpToolResult::success("accessibility settings updated".to_string()),
                    Err(e) => McpToolResult::error(e.to_string()),
                }
            }
            _ => McpToolResult::error(format!("Unknown system category: {category}")),
        }
    } else {
        let guard = service.state.read().unwrap();
        let result = match category {
            "power" => serde_json::to_string_pretty(&guard.power).unwrap(),
            "locale" => serde_json::to_string_pretty(&guard.locale).unwrap(),
            "accessibility" => serde_json::to_string_pretty(&guard.accessibility).unwrap(),
            _ => return McpToolResult::error(format!("Unknown system category: {category}")),
        };
        McpToolResult::success(result)
    }
}

fn handle_history(call: &McpToolCall, service: &Arc<SettingsService>) -> McpToolResult {
    let limit = call
        .arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let category = call.arguments.get("category").and_then(|v| v.as_str());

    let changes = if let Some(cat) = category {
        service.store.changes_for_category(cat, limit)
    } else {
        service.store.recent_changes(limit)
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
    use vidhana_backend::NoopBackend;
    use vidhana_settings::SettingsStore;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_service() -> Arc<SettingsService> {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("vidhana-mcp-test-{}-{}", std::process::id(), id));
        let store = Arc::new(SettingsStore::new(dir.to_str().unwrap()).unwrap());
        let state = new_shared_state(VidhanaConfig::default());
        Arc::new(SettingsService::new(state, store, Arc::new(NoopBackend)))
    }

    #[test]
    fn test_list_tools() {
        let tools = list_tools();
        assert_eq!(tools.len(), 6);
        assert_eq!(tools[5].name, "vidhana_history");
        // Verify all tools have object schemas with properties
        for tool in &tools {
            assert_eq!(tool.input_schema.schema_type, "object");
        }
    }

    #[test]
    fn test_display_get() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "get"}),
        };
        let result = handle_tool_call(&call, &svc);
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("brightness"));
    }

    #[test]
    fn test_display_set_brightness() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_display".to_string(),
            arguments: serde_json::json!({"action": "set", "brightness": 100}),
        };
        let result = handle_tool_call(&call, &svc);
        assert!(!result.is_error);
        assert_eq!(svc.state.read().unwrap().display.brightness, 100);
    }

    #[test]
    fn test_audio_set_volume() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_audio".to_string(),
            arguments: serde_json::json!({"action": "set", "volume": 50}),
        };
        handle_tool_call(&call, &svc);
        assert_eq!(svc.state.read().unwrap().audio.master_volume, 50);
    }

    #[test]
    fn test_network_disable_wifi() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_network".to_string(),
            arguments: serde_json::json!({"action": "set", "wifi_enabled": false}),
        };
        handle_tool_call(&call, &svc);
        assert!(!svc.state.read().unwrap().network.wifi_enabled);
    }

    #[test]
    fn test_system_power_profile() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_system".to_string(),
            arguments: serde_json::json!({"action": "set", "category": "power", "power_profile": "performance"}),
        };
        handle_tool_call(&call, &svc);
        assert_eq!(
            svc.state.read().unwrap().power.power_profile,
            PowerProfile::Performance
        );
    }

    #[test]
    fn test_system_locale_timezone() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_system".to_string(),
            arguments: serde_json::json!({"action": "set", "category": "locale", "timezone": "America/New_York"}),
        };
        handle_tool_call(&call, &svc);
        assert_eq!(
            svc.state.read().unwrap().locale.timezone,
            "America/New_York"
        );
    }

    #[test]
    fn test_unknown_tool() {
        let svc = test_service();
        let call = McpToolCall {
            name: "vidhana_unknown".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = handle_tool_call(&call, &svc);
        assert!(result.is_error);
    }

    #[test]
    fn test_history_tool() {
        let svc = test_service();
        // Make a change
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_display".to_string(),
                arguments: serde_json::json!({"action": "set", "brightness": 42}),
            },
            &svc,
        );
        // Query history
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_history".to_string(),
                arguments: serde_json::json!({}),
            },
            &svc,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("display"));
    }

    #[test]
    fn test_initialize_response() {
        let resp = initialize_response();
        assert_eq!(resp["protocolVersion"], "2024-11-05");
        assert_eq!(resp["serverInfo"]["name"], "vidhana");
    }

    #[test]
    fn test_tool_schemas_valid() {
        for tool in list_tools() {
            assert_eq!(tool.input_schema.schema_type, "object");
            assert!(!tool.input_schema.properties.is_empty() || tool.name == "vidhana_history");
        }
    }

    #[test]
    fn test_audio_get() {
        let svc = test_service();
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_audio".to_string(),
                arguments: serde_json::json!({}),
            },
            &svc,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("master_volume"));
    }

    #[test]
    fn test_network_get() {
        let svc = test_service();
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_network".to_string(),
                arguments: serde_json::json!({"action": "get"}),
            },
            &svc,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("wifi_enabled"));
    }

    #[test]
    fn test_privacy_get() {
        let svc = test_service();
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_privacy".to_string(),
                arguments: serde_json::json!({"action": "get"}),
            },
            &svc,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("screen_lock_enabled"));
    }

    #[test]
    fn test_privacy_set() {
        let svc = test_service();
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_privacy".to_string(),
                arguments: serde_json::json!({"action": "set", "telemetry_enabled": true, "camera_enabled": false}),
            },
            &svc,
        );
        let g = svc.state.read().unwrap();
        assert!(g.privacy.telemetry_enabled);
        assert!(!g.privacy.camera_enabled);
    }

    #[test]
    fn test_system_get_all_categories() {
        let svc = test_service();
        for cat in ["power", "locale", "accessibility"] {
            let result = handle_tool_call(
                &McpToolCall {
                    name: "vidhana_system".to_string(),
                    arguments: serde_json::json!({"action": "get", "category": cat}),
                },
                &svc,
            );
            assert!(!result.is_error, "Failed for category: {cat}");
        }
    }

    #[test]
    fn test_system_invalid_category() {
        let svc = test_service();
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_system".to_string(),
                arguments: serde_json::json!({"action": "get", "category": "nonsense"}),
            },
            &svc,
        );
        assert!(result.is_error);
    }

    #[test]
    fn test_default_action_is_get() {
        let svc = test_service();
        // No action field — should default to "get"
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_display".to_string(),
                arguments: serde_json::json!({}),
            },
            &svc,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("brightness"));
    }

    #[test]
    fn test_display_set_theme() {
        let svc = test_service();
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_display".to_string(),
                arguments: serde_json::json!({"action": "set", "theme": "light"}),
            },
            &svc,
        );
        assert_eq!(svc.state.read().unwrap().display.theme, Theme::Light);
    }

    #[test]
    fn test_audio_mute() {
        let svc = test_service();
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_audio".to_string(),
                arguments: serde_json::json!({"action": "set", "muted": true}),
            },
            &svc,
        );
        assert!(svc.state.read().unwrap().audio.muted);
    }

    #[test]
    fn test_network_set_bluetooth() {
        let svc = test_service();
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_network".to_string(),
                arguments: serde_json::json!({"action": "set", "bluetooth_enabled": false}),
            },
            &svc,
        );
        assert!(!svc.state.read().unwrap().network.bluetooth_enabled);
    }

    #[test]
    fn test_history_with_category_filter() {
        let svc = test_service();
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_display".to_string(),
                arguments: serde_json::json!({"action": "set", "brightness": 50}),
            },
            &svc,
        );
        handle_tool_call(
            &McpToolCall {
                name: "vidhana_audio".to_string(),
                arguments: serde_json::json!({"action": "set", "volume": 30}),
            },
            &svc,
        );
        let result = handle_tool_call(
            &McpToolCall {
                name: "vidhana_history".to_string(),
                arguments: serde_json::json!({"category": "audio"}),
            },
            &svc,
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("audio"));
        assert!(!result.content[0].text.contains("display"));
    }
}
