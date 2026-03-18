//! Vidhana API — REST API and system integration
//!
//! HTTP endpoints for reading/writing AGNOS system settings.
//! Connects to daimon (8090) for service queries and hoosh (8088) for NL.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vidhana_core::*;
use vidhana_settings::{ChangeSource, SettingsChange, SettingsStore};

/// Shared application context for all API handlers.
#[derive(Clone)]
pub struct AppState {
    pub settings: SharedState,
    pub store: Arc<SettingsStore>,
}

/// API error type with JSON response support.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("{0}")]
    BadRequest(String),

    #[error("Category not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            ApiError::Http(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            ApiError::BadRequest(e) => (StatusCode::BAD_REQUEST, e.clone()),
            ApiError::NotFound(e) => (StatusCode::NOT_FOUND, e.clone()),
            ApiError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };
        (status, Json(ErrorResponse { error: msg })).into_response()
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub categories: Vec<String>,
}

/// History query parameters
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
}

/// Build the Vidhana API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/settings", get(get_all_settings))
        .route(
            "/v1/settings/display",
            get(get_display).post(update_display).patch(patch_display),
        )
        .route(
            "/v1/settings/audio",
            get(get_audio).post(update_audio).patch(patch_audio),
        )
        .route(
            "/v1/settings/network",
            get(get_network).post(update_network).patch(patch_network),
        )
        .route(
            "/v1/settings/privacy",
            get(get_privacy).post(update_privacy).patch(patch_privacy),
        )
        .route(
            "/v1/settings/locale",
            get(get_locale).post(update_locale).patch(patch_locale),
        )
        .route(
            "/v1/settings/power",
            get(get_power).post(update_power).patch(patch_power),
        )
        .route(
            "/v1/settings/accessibility",
            get(get_accessibility)
                .post(update_accessibility)
                .patch(patch_accessibility),
        )
        .route("/v1/settings/history", get(get_history))
        .route("/v1/settings/{category}/history", get(get_category_history))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Persist current state and record a change in the audit log.
fn persist_and_record(
    app: &AppState,
    category: &str,
    key: &str,
    old_value: &str,
    new_value: &str,
) {
    let guard = app.settings.read().unwrap();
    if let Err(e) = app.store.save_state(&guard) {
        tracing::error!("Failed to persist settings: {e}");
    }
    drop(guard);

    let change = SettingsChange {
        timestamp: chrono::Utc::now(),
        category: category.to_string(),
        key: key.to_string(),
        old_value: old_value.to_string(),
        new_value: new_value.to_string(),
        source: ChangeSource::Api,
    };
    if let Err(e) = app.store.record_change(&change) {
        tracing::error!("Failed to record change: {e}");
    }
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        categories: vec![
            "display".into(),
            "audio".into(),
            "network".into(),
            "privacy".into(),
            "locale".into(),
            "power".into(),
            "accessibility".into(),
        ],
    })
}

async fn get_all_settings(State(app): State<AppState>) -> Json<serde_json::Value> {
    let guard = app.settings.read().unwrap();
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

// --- Display ---------------------------------------------------------------

async fn get_display(State(app): State<AppState>) -> Json<DisplaySettings> {
    Json(app.settings.read().unwrap().display.clone())
}

async fn update_display(
    State(app): State<AppState>,
    Json(mut update): Json<DisplaySettings>,
) -> Result<StatusCode, ApiError> {
    update.validate();
    let old = serde_json::to_string(&app.settings.read().unwrap().display).unwrap_or_default();
    app.settings.write().unwrap().display = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "display", "*", &old, &new);
    tracing::info!("Display settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_display(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<DisplaySettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.display).unwrap_or_default();
    let mut current = serde_json::to_value(&guard.display).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let mut updated: DisplaySettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    updated.validate();
    guard.display = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "display", "*", &old, &new);
    tracing::info!("Display settings patched via API");
    Ok(Json(updated))
}

// --- Audio -----------------------------------------------------------------

async fn get_audio(State(app): State<AppState>) -> Json<AudioSettings> {
    Json(app.settings.read().unwrap().audio.clone())
}

async fn update_audio(
    State(app): State<AppState>,
    Json(mut update): Json<AudioSettings>,
) -> Result<StatusCode, ApiError> {
    update.validate();
    let old = serde_json::to_string(&app.settings.read().unwrap().audio).unwrap_or_default();
    app.settings.write().unwrap().audio = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "audio", "*", &old, &new);
    tracing::info!("Audio settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_audio(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<AudioSettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.audio).unwrap_or_default();
    let mut current = serde_json::to_value(&guard.audio).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let mut updated: AudioSettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    updated.validate();
    guard.audio = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "audio", "*", &old, &new);
    tracing::info!("Audio settings patched via API");
    Ok(Json(updated))
}

// --- Network ---------------------------------------------------------------

async fn get_network(State(app): State<AppState>) -> Json<NetworkSettings> {
    Json(app.settings.read().unwrap().network.clone())
}

async fn update_network(
    State(app): State<AppState>,
    Json(update): Json<NetworkSettings>,
) -> Result<StatusCode, ApiError> {
    let old = serde_json::to_string(&app.settings.read().unwrap().network).unwrap_or_default();
    app.settings.write().unwrap().network = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "network", "*", &old, &new);
    tracing::info!("Network settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_network(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<NetworkSettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.network).unwrap_or_default();
    let mut current = serde_json::to_value(&guard.network).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let updated: NetworkSettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    guard.network = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "network", "*", &old, &new);
    tracing::info!("Network settings patched via API");
    Ok(Json(updated))
}

// --- Privacy ---------------------------------------------------------------

async fn get_privacy(State(app): State<AppState>) -> Json<PrivacySettings> {
    Json(app.settings.read().unwrap().privacy.clone())
}

async fn update_privacy(
    State(app): State<AppState>,
    Json(mut update): Json<PrivacySettings>,
) -> Result<StatusCode, ApiError> {
    update.validate();
    let old = serde_json::to_string(&app.settings.read().unwrap().privacy).unwrap_or_default();
    app.settings.write().unwrap().privacy = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "privacy", "*", &old, &new);
    tracing::info!("Privacy settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_privacy(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<PrivacySettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.privacy).unwrap_or_default();
    let mut current = serde_json::to_value(&guard.privacy).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let mut updated: PrivacySettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    updated.validate();
    guard.privacy = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "privacy", "*", &old, &new);
    tracing::info!("Privacy settings patched via API");
    Ok(Json(updated))
}

// --- Locale ----------------------------------------------------------------

async fn get_locale(State(app): State<AppState>) -> Json<LocaleSettings> {
    Json(app.settings.read().unwrap().locale.clone())
}

async fn update_locale(
    State(app): State<AppState>,
    Json(update): Json<LocaleSettings>,
) -> Result<StatusCode, ApiError> {
    let old = serde_json::to_string(&app.settings.read().unwrap().locale).unwrap_or_default();
    app.settings.write().unwrap().locale = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "locale", "*", &old, &new);
    tracing::info!("Locale settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_locale(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<LocaleSettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.locale).unwrap_or_default();
    let mut current = serde_json::to_value(&guard.locale).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let updated: LocaleSettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    guard.locale = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "locale", "*", &old, &new);
    tracing::info!("Locale settings patched via API");
    Ok(Json(updated))
}

// --- Power -----------------------------------------------------------------

async fn get_power(State(app): State<AppState>) -> Json<PowerSettings> {
    Json(app.settings.read().unwrap().power.clone())
}

async fn update_power(
    State(app): State<AppState>,
    Json(mut update): Json<PowerSettings>,
) -> Result<StatusCode, ApiError> {
    update.validate();
    let old = serde_json::to_string(&app.settings.read().unwrap().power).unwrap_or_default();
    app.settings.write().unwrap().power = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "power", "*", &old, &new);
    tracing::info!("Power settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_power(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<PowerSettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.power).unwrap_or_default();
    let mut current = serde_json::to_value(&guard.power).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let mut updated: PowerSettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    updated.validate();
    guard.power = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "power", "*", &old, &new);
    tracing::info!("Power settings patched via API");
    Ok(Json(updated))
}

// --- Accessibility ---------------------------------------------------------

async fn get_accessibility(State(app): State<AppState>) -> Json<AccessibilitySettings> {
    Json(app.settings.read().unwrap().accessibility.clone())
}

async fn update_accessibility(
    State(app): State<AppState>,
    Json(update): Json<AccessibilitySettings>,
) -> Result<StatusCode, ApiError> {
    let old = serde_json::to_string(&app.settings.read().unwrap().accessibility).unwrap_or_default();
    app.settings.write().unwrap().accessibility = update.clone();
    let new = serde_json::to_string(&update).unwrap_or_default();
    persist_and_record(&app, "accessibility", "*", &old, &new);
    tracing::info!("Accessibility settings updated via API");
    Ok(StatusCode::OK)
}

async fn patch_accessibility(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<AccessibilitySettings>, ApiError> {
    let mut guard = app.settings.write().unwrap();
    let old = serde_json::to_string(&guard.accessibility).unwrap_or_default();
    let mut current =
        serde_json::to_value(&guard.accessibility).map_err(|e| ApiError::Internal(e.to_string()))?;
    merge_json(&mut current, &patch);
    let updated: AccessibilitySettings =
        serde_json::from_value(current).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    guard.accessibility = updated.clone();
    drop(guard);
    let new = serde_json::to_string(&updated).unwrap_or_default();
    persist_and_record(&app, "accessibility", "*", &old, &new);
    tracing::info!("Accessibility settings patched via API");
    Ok(Json(updated))
}

// --- History ---------------------------------------------------------------

async fn get_history(
    State(app): State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<SettingsChange>>, ApiError> {
    let limit = params.limit.unwrap_or(50);
    app.store
        .recent_changes(limit)
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

async fn get_category_history(
    State(app): State<AppState>,
    Path(category): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<SettingsChange>>, ApiError> {
    // Validate category name
    category
        .parse::<SettingsCategory>()
        .map_err(|e| ApiError::NotFound(e))?;
    let limit = params.limit.unwrap_or(50);
    app.store
        .changes_for_category(&category, limit)
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

// ---------------------------------------------------------------------------
// JSON merge helper
// ---------------------------------------------------------------------------

/// Shallow merge of `patch` into `target`. Only top-level keys are overwritten.
fn merge_json(target: &mut serde_json::Value, patch: &serde_json::Value) {
    if let (Some(target_obj), Some(patch_obj)) = (target.as_object_mut(), patch.as_object()) {
        for (k, v) in patch_obj {
            target_obj.insert(k.clone(), v.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Daimon client
// ---------------------------------------------------------------------------

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
        let resp = self
            .client
            .get(format!("{}/v1/health", self.base_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    /// Get system metrics from daimon
    pub async fn metrics(&self) -> Result<serde_json::Value, ApiError> {
        let resp = self
            .client
            .get(format!("{}/v1/metrics", self.base_url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_app() -> AppState {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "vidhana-api-test-{}-{}",
            std::process::id(),
            id
        ));
        let store = SettingsStore::new(dir.to_str().unwrap()).unwrap();
        AppState {
            settings: new_shared_state(VidhanaConfig::default()),
            store: Arc::new(store),
        }
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = router(test_app());
        let req = Request::get("/health").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_all_settings() {
        let app = router(test_app());
        let req = Request::get("/v1/settings").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_display() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/display")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_display() {
        let state = test_app();
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
        assert_eq!(state.settings.read().unwrap().display.brightness, 100);
    }

    #[tokio::test]
    async fn test_patch_display_brightness() {
        let state = test_app();
        let app = router(state.clone());

        let req = Request::builder()
            .method("PATCH")
            .uri("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"brightness": 42}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let guard = state.settings.read().unwrap();
        assert_eq!(guard.display.brightness, 42);
        // Other fields unchanged
        assert_eq!(guard.display.theme, Theme::Dark);
    }

    #[tokio::test]
    async fn test_patch_validates() {
        let state = test_app();
        let app = router(state.clone());

        let req = Request::builder()
            .method("PATCH")
            .uri("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"brightness": 255}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Should be clamped to 100
        assert_eq!(state.settings.read().unwrap().display.brightness, 100);
    }

    #[tokio::test]
    async fn test_update_persists_and_records_history() {
        let state = test_app();
        let app = router(state.clone());

        let display = DisplaySettings {
            brightness: 50,
            ..DisplaySettings::default()
        };

        let req = Request::post("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&display).unwrap()))
            .unwrap();
        app.oneshot(req).await.unwrap();

        // Verify persisted to disk
        let loaded = state.store.load_state().unwrap().unwrap();
        assert_eq!(loaded.display.brightness, 50);

        // Verify history recorded
        let changes = state.store.recent_changes(10).unwrap();
        assert!(!changes.is_empty());
        assert_eq!(changes[0].category, "display");
    }

    #[tokio::test]
    async fn test_get_audio() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/audio")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_network() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/network")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_privacy() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/privacy")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_locale() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/locale")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_power() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/power")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_accessibility() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/accessibility")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_privacy() {
        let state = test_app();
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
        assert!(state.settings.read().unwrap().privacy.telemetry_enabled);
    }

    #[tokio::test]
    async fn test_history_endpoint() {
        let state = test_app();
        let app = router(state.clone());

        // Make a change first
        let display = DisplaySettings {
            brightness: 42,
            ..DisplaySettings::default()
        };
        let req = Request::post("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&display).unwrap()))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        // Query history
        let req = Request::get("/v1/settings/history")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_category_history_endpoint() {
        let state = test_app();
        let app = router(state.clone());

        let req = Request::get("/v1/settings/display/history")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_category_history_invalid() {
        let app = router(test_app());
        let req = Request::get("/v1/settings/nonsense/history")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_patch_bad_field() {
        let app = router(test_app());
        let req = Request::builder()
            .method("PATCH")
            .uri("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"theme": "invalid_theme"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_daimon_client_creation() {
        let client = DaimonClient::new("http://127.0.0.1:8090");
        assert_eq!(client.base_url, "http://127.0.0.1:8090");
    }

    #[test]
    fn test_merge_json() {
        let mut target = serde_json::json!({"a": 1, "b": 2});
        let patch = serde_json::json!({"b": 99, "c": 3});
        merge_json(&mut target, &patch);
        assert_eq!(target["a"], 1);
        assert_eq!(target["b"], 99);
        assert_eq!(target["c"], 3);
    }
}
