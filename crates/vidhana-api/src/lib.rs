//! Vidhana API — REST API and system integration
//!
//! HTTP endpoints for reading/writing AGNOS system settings.
//! Connects to daimon (8090) for service queries and hoosh (8088) for NL.

use axum::{
    Router, Json,
    extract::State,
    routing::get,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use vidhana_core::*;

/// API error type
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Settings operation failed: {0}")]
    SettingsOp(String),

    #[error("Category not found: {0}")]
    NotFound(String),
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub categories: Vec<String>,
}

/// Settings response wrapper
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub category: String,
    pub settings: serde_json::Value,
}

/// Update request
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub key: String,
    pub value: serde_json::Value,
}

/// Build the Vidhana API router
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/settings", get(get_all_settings))
        .route("/v1/settings/display", get(get_display).post(update_display))
        .route("/v1/settings/audio", get(get_audio).post(update_audio))
        .route("/v1/settings/network", get(get_network).post(update_network))
        .route("/v1/settings/privacy", get(get_privacy).post(update_privacy))
        .route("/v1/settings/locale", get(get_locale).post(update_locale))
        .route("/v1/settings/power", get(get_power).post(update_power))
        .route("/v1/settings/accessibility", get(get_accessibility).post(update_accessibility))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        categories: vec![
            "display".into(), "audio".into(), "network".into(),
            "privacy".into(), "locale".into(), "power".into(),
            "accessibility".into(),
        ],
    })
}

async fn get_all_settings(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let guard = state.read().unwrap();
    Json(serde_json::json!({
        "display": guard.display,
        "audio": guard.audio,
        "network": guard.network,
        "privacy": guard.privacy,
        "locale": guard.locale,
        "power": guard.power,
        "accessibility": guard.accessibility,
    }))
}

async fn get_display(State(state): State<SharedState>) -> Json<DisplaySettings> {
    Json(state.read().unwrap().display.clone())
}

async fn update_display(
    State(state): State<SharedState>,
    Json(update): Json<DisplaySettings>,
) -> StatusCode {
    state.write().unwrap().display = update;
    tracing::info!("Display settings updated");
    StatusCode::OK
}

async fn get_audio(State(state): State<SharedState>) -> Json<AudioSettings> {
    Json(state.read().unwrap().audio.clone())
}

async fn update_audio(
    State(state): State<SharedState>,
    Json(update): Json<AudioSettings>,
) -> StatusCode {
    state.write().unwrap().audio = update;
    tracing::info!("Audio settings updated");
    StatusCode::OK
}

async fn get_network(State(state): State<SharedState>) -> Json<NetworkSettings> {
    Json(state.read().unwrap().network.clone())
}

async fn update_network(
    State(state): State<SharedState>,
    Json(update): Json<NetworkSettings>,
) -> StatusCode {
    state.write().unwrap().network = update;
    tracing::info!("Network settings updated");
    StatusCode::OK
}

async fn get_privacy(State(state): State<SharedState>) -> Json<PrivacySettings> {
    Json(state.read().unwrap().privacy.clone())
}

async fn update_privacy(
    State(state): State<SharedState>,
    Json(update): Json<PrivacySettings>,
) -> StatusCode {
    state.write().unwrap().privacy = update;
    tracing::info!("Privacy settings updated");
    StatusCode::OK
}

async fn get_locale(State(state): State<SharedState>) -> Json<LocaleSettings> {
    Json(state.read().unwrap().locale.clone())
}

async fn update_locale(
    State(state): State<SharedState>,
    Json(update): Json<LocaleSettings>,
) -> StatusCode {
    state.write().unwrap().locale = update;
    tracing::info!("Locale settings updated");
    StatusCode::OK
}

async fn get_power(State(state): State<SharedState>) -> Json<PowerSettings> {
    Json(state.read().unwrap().power.clone())
}

async fn update_power(
    State(state): State<SharedState>,
    Json(update): Json<PowerSettings>,
) -> StatusCode {
    state.write().unwrap().power = update;
    tracing::info!("Power settings updated");
    StatusCode::OK
}

async fn get_accessibility(State(state): State<SharedState>) -> Json<AccessibilitySettings> {
    Json(state.read().unwrap().accessibility.clone())
}

async fn update_accessibility(
    State(state): State<SharedState>,
    Json(update): Json<AccessibilitySettings>,
) -> StatusCode {
    state.write().unwrap().accessibility = update;
    tracing::info!("Accessibility settings updated");
    StatusCode::OK
}

/// Daimon API client for service status queries
pub struct DaimonClient {
    base_url: String,
    client: reqwest::Client,
}

impl DaimonClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Check if daimon is reachable
    pub async fn health(&self) -> Result<bool, ApiError> {
        let resp = self.client
            .get(format!("{}/v1/health", self.base_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    /// Get system metrics from daimon
    pub async fn metrics(&self) -> Result<serde_json::Value, ApiError> {
        let resp = self.client
            .get(format!("{}/v1/metrics", self.base_url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> SharedState {
        new_shared_state(VidhanaConfig::default())
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = router(test_state());
        let req = Request::get("/health").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_all_settings() {
        let app = router(test_state());
        let req = Request::get("/v1/settings").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_display() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/display").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_display() {
        let state = test_state();
        let app = router(state.clone());

        let display = DisplaySettings {
            brightness: 100,
            ..DisplaySettings::default()
        };

        let req = Request::post("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&display).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(state.read().unwrap().display.brightness, 100);
    }

    #[tokio::test]
    async fn test_get_audio() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/audio").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_network() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/network").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_privacy() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/privacy").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_locale() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/locale").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_power() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/power").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_accessibility() {
        let app = router(test_state());
        let req = Request::get("/v1/settings/accessibility").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_privacy() {
        let state = test_state();
        let app = router(state.clone());

        let privacy = PrivacySettings {
            telemetry_enabled: true,
            ..PrivacySettings::default()
        };

        let req = Request::post("/v1/settings/privacy")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&privacy).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.read().unwrap().privacy.telemetry_enabled);
    }

    #[test]
    fn test_daimon_client_creation() {
        let client = DaimonClient::new("http://127.0.0.1:8090");
        assert_eq!(client.base_url, "http://127.0.0.1:8090");
    }
}
