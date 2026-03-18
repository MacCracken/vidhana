//! Vidhana — AGNOS System Settings
//!
//! Sanskrit: विधान (regulation, constitution, arrangement)

use clap::Parser;
use std::sync::Arc;
use vidhana_backend::{LinuxBackend, SettingsService};
use vidhana_core::*;

#[derive(Parser)]
#[command(name = "vidhana", about = "Vidhana — AGNOS system settings")]
struct Cli {
    /// Launch the desktop GUI
    #[arg(long)]
    gui: bool,

    /// Run in headless mode (API only, same as default)
    #[arg(long)]
    headless: bool,

    /// Run as MCP server (JSON-RPC over stdin/stdout)
    #[arg(long)]
    mcp: bool,

    /// Bind address for HTTP API
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Port for Vidhana HTTP API
    #[arg(long, default_value = "8099")]
    port: u16,

    /// Data directory
    #[arg(long)]
    data_dir: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("Vidhana system settings v{}", env!("CARGO_PKG_VERSION"));

    let mut config = VidhanaConfig::load();
    config.bind_address = cli.bind.clone();
    config.port = cli.port;
    if let Some(ref dir) = cli.data_dir {
        config.data_dir = dir.clone();
    }

    let store = vidhana_settings::SettingsStore::new(&config.data_dir)
        .expect("Failed to initialize settings store");
    let store = Arc::new(store);

    let state = new_shared_state(config.clone());

    // Load persisted settings if available
    if let Ok(Some(saved)) = store.load_state() {
        let mut guard = state.write().unwrap();
        guard.display = saved.display;
        guard.audio = saved.audio;
        guard.network = saved.network;
        guard.privacy = saved.privacy;
        guard.locale = saved.locale;
        guard.power = saved.power;
        guard.accessibility = saved.accessibility;
        guard.validate();
        tracing::info!("Loaded persisted settings from {}", config.data_dir);
    }

    // Detect system backends and sync OS state
    let backend = Arc::new(LinuxBackend::detect());
    backend.log_capabilities();

    let service = Arc::new(SettingsService::new(state.clone(), store, backend));
    service.sync_from_os();

    if cli.mcp {
        tracing::info!("Starting MCP server on stdin/stdout");
        run_mcp_server(service);
        return;
    }

    if cli.gui {
        tracing::info!("Launching GUI");
        vidhana_ui::run_app(state, service);
        return;
    }

    // Default: run HTTP API
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let addr = format!("{}:{}", cli.bind, cli.port);
        tracing::info!("Starting HTTP API on {addr}");

        let hoosh = Arc::new(vidhana_ai::HooshClient::new(&config.hoosh_url));

        let app_state = vidhana_api::AppState {
            settings: state,
            service,
            hoosh: Some(hoosh),
        };

        let app = vidhana_api::router(app_state)
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .layer(tower_http::cors::CorsLayer::permissive());

        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
    });
}

fn run_mcp_server(service: Arc<SettingsService>) {
    use std::io::{self, BufRead, Write};

    let stdin = io::stdin();
    let stdout = io::stdout();
    let tools = vidhana_mcp::list_tools();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request): Result<serde_json::Value, _> = serde_json::from_str(&line) else {
            continue;
        };

        let response = match request.get("method").and_then(|m| m.as_str()) {
            Some("initialize") => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "result": vidhana_mcp::initialize_response()
                })
            }
            Some("notifications/initialized") => continue,
            Some("tools/list") => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "result": { "tools": tools }
                })
            }
            Some("tools/call") => {
                let params = request.get("params").cloned().unwrap_or_default();
                let call = vidhana_mcp::McpToolCall {
                    name: params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string(),
                    arguments: params.get("arguments").cloned().unwrap_or_default(),
                };
                let result = vidhana_mcp::handle_tool_call(&call, &service);
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "result": result
                })
            }
            _ => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "error": { "code": -32601, "message": "Method not found" }
                })
            }
        };

        let mut out = stdout.lock();
        let _ = serde_json::to_writer(&mut out, &response);
        let _ = out.write_all(b"\n");
        let _ = out.flush();
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");
    tracing::info!("Shutting down Vidhana");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_defaults() {
        let cli = Cli::parse_from(["vidhana"]);
        assert!(!cli.gui);
        assert!(!cli.headless);
        assert!(!cli.mcp);
        assert_eq!(cli.bind, "127.0.0.1");
        assert_eq!(cli.port, 8099);
        assert!(cli.data_dir.is_none());
    }

    #[test]
    fn test_cli_parse_gui() {
        let cli = Cli::parse_from(["vidhana", "--gui"]);
        assert!(cli.gui);
    }

    #[test]
    fn test_cli_parse_mcp() {
        let cli = Cli::parse_from(["vidhana", "--mcp"]);
        assert!(cli.mcp);
    }

    #[test]
    fn test_cli_parse_custom_port() {
        let cli = Cli::parse_from(["vidhana", "--port", "9000"]);
        assert_eq!(cli.port, 9000);
    }

    #[test]
    fn test_cli_parse_data_dir() {
        let cli = Cli::parse_from(["vidhana", "--data-dir", "/tmp/vidhana-test"]);
        assert_eq!(cli.data_dir, Some("/tmp/vidhana-test".to_string()));
    }
}
