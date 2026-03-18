//! Vidhana API — REST API and system integration
//!
//! HTTP endpoints for reading/writing AGNOS system settings.
//! All mutations go through `SettingsService` which handles
//! validation, OS backend application, persistence, and auditing.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vidhana_backend::{SettingsService, service::ServiceError};
use vidhana_core::*;
use vidhana_settings::{ChangeSource, SettingsChange};

/// Shared application context for all API handlers.
#[derive(Clone)]
pub struct AppState {
    pub settings: SharedState,
    pub service: Arc<SettingsService>,
    pub hoosh: Option<Arc<vidhana_ai::HooshClient>>,
}

/// API error type with JSON response support.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
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
            ApiError::BadRequest(e) => (StatusCode::BAD_REQUEST, e.clone()),
            ApiError::NotFound(e) => (StatusCode::NOT_FOUND, e.clone()),
            ApiError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };
        (status, Json(ErrorResponse { error: msg })).into_response()
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Deserialize(e) => ApiError::BadRequest(e),
            ServiceError::Validation(e) => ApiError::BadRequest(e),
            e => ApiError::Internal(e.to_string()),
        }
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
        .route("/v1/nl", axum::routing::post(parse_natural_language))
        .with_state(state)
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

fn read_state(app: &AppState) -> Result<std::sync::RwLockReadGuard<'_, VidhanaState>, ApiError> {
    app.settings
        .read()
        .map_err(|_| ApiError::Internal("settings lock poisoned".to_string()))
}

async fn get_all_settings(
    State(app): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let guard = read_state(&app)?;
    Ok(Json(serde_json::json!({
        "display": guard.display,
        "audio": guard.audio,
        "network": guard.network,
        "privacy": guard.privacy,
        "locale": guard.locale,
        "power": guard.power,
        "accessibility": guard.accessibility,
    })))
}

// --- Display ---------------------------------------------------------------

async fn get_display(State(app): State<AppState>) -> Result<Json<DisplaySettings>, ApiError> {
    Ok(Json(read_state(&app)?.display.clone()))
}

async fn update_display(
    State(app): State<AppState>,
    Json(update): Json<DisplaySettings>,
) -> Result<StatusCode, ApiError> {
    app.service.update_display(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_display(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<DisplaySettings>, ApiError> {
    Ok(Json(app.service.patch_display(patch, ChangeSource::Api)?))
}

// --- Audio -----------------------------------------------------------------

async fn get_audio(State(app): State<AppState>) -> Result<Json<AudioSettings>, ApiError> {
    Ok(Json(read_state(&app)?.audio.clone()))
}

async fn update_audio(
    State(app): State<AppState>,
    Json(update): Json<AudioSettings>,
) -> Result<StatusCode, ApiError> {
    app.service.update_audio(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_audio(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<AudioSettings>, ApiError> {
    Ok(Json(app.service.patch_audio(patch, ChangeSource::Api)?))
}

// --- Network ---------------------------------------------------------------

async fn get_network(State(app): State<AppState>) -> Result<Json<NetworkSettings>, ApiError> {
    Ok(Json(read_state(&app)?.network.clone()))
}

async fn update_network(
    State(app): State<AppState>,
    Json(update): Json<NetworkSettings>,
) -> Result<StatusCode, ApiError> {
    app.service.update_network(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_network(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<NetworkSettings>, ApiError> {
    Ok(Json(app.service.patch_network(patch, ChangeSource::Api)?))
}

// --- Privacy ---------------------------------------------------------------

async fn get_privacy(State(app): State<AppState>) -> Result<Json<PrivacySettings>, ApiError> {
    Ok(Json(read_state(&app)?.privacy.clone()))
}

async fn update_privacy(
    State(app): State<AppState>,
    Json(update): Json<PrivacySettings>,
) -> Result<StatusCode, ApiError> {
    app.service.update_privacy(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_privacy(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<PrivacySettings>, ApiError> {
    Ok(Json(app.service.patch_privacy(patch, ChangeSource::Api)?))
}

// --- Locale ----------------------------------------------------------------

async fn get_locale(State(app): State<AppState>) -> Result<Json<LocaleSettings>, ApiError> {
    Ok(Json(read_state(&app)?.locale.clone()))
}

async fn update_locale(
    State(app): State<AppState>,
    Json(update): Json<LocaleSettings>,
) -> Result<StatusCode, ApiError> {
    app.service.update_locale(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_locale(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<LocaleSettings>, ApiError> {
    Ok(Json(app.service.patch_locale(patch, ChangeSource::Api)?))
}

// --- Power -----------------------------------------------------------------

async fn get_power(State(app): State<AppState>) -> Result<Json<PowerSettings>, ApiError> {
    Ok(Json(read_state(&app)?.power.clone()))
}

async fn update_power(
    State(app): State<AppState>,
    Json(update): Json<PowerSettings>,
) -> Result<StatusCode, ApiError> {
    app.service.update_power(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_power(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<PowerSettings>, ApiError> {
    Ok(Json(app.service.patch_power(patch, ChangeSource::Api)?))
}

// --- Accessibility ---------------------------------------------------------

async fn get_accessibility(
    State(app): State<AppState>,
) -> Result<Json<AccessibilitySettings>, ApiError> {
    Ok(Json(read_state(&app)?.accessibility.clone()))
}

async fn update_accessibility(
    State(app): State<AppState>,
    Json(update): Json<AccessibilitySettings>,
) -> Result<StatusCode, ApiError> {
    app.service
        .update_accessibility(update, ChangeSource::Api)?;
    Ok(StatusCode::OK)
}

async fn patch_accessibility(
    State(app): State<AppState>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<AccessibilitySettings>, ApiError> {
    Ok(Json(
        app.service.patch_accessibility(patch, ChangeSource::Api)?,
    ))
}

// --- History ---------------------------------------------------------------

async fn get_history(
    State(app): State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<SettingsChange>>, ApiError> {
    let limit = params.limit.unwrap_or(50);
    app.service
        .store
        .recent_changes(limit)
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

async fn get_category_history(
    State(app): State<AppState>,
    Path(category): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<SettingsChange>>, ApiError> {
    category
        .parse::<SettingsCategory>()
        .map_err(ApiError::NotFound)?;
    let limit = params.limit.unwrap_or(50);
    app.service
        .store
        .changes_for_category(&category, limit)
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

// --- Natural Language -------------------------------------------------------

#[derive(Debug, Deserialize)]
struct NlRequest {
    text: String,
}

#[derive(Debug, Serialize)]
struct NlResponse {
    intent: Option<vidhana_ai::SettingsIntent>,
    raw_text: String,
}

async fn parse_natural_language(
    State(app): State<AppState>,
    Json(req): Json<NlRequest>,
) -> Result<Json<NlResponse>, ApiError> {
    let hoosh_ref = app.hoosh.as_deref();
    let intent = vidhana_ai::parse_with_hoosh(&req.text, hoosh_ref).await;
    Ok(Json(NlResponse {
        intent,
        raw_text: req.text,
    }))
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
    use vidhana_backend::NoopBackend;
    use vidhana_settings::SettingsStore;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_app() -> AppState {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("vidhana-api-test-{}-{}", std::process::id(), id));
        let store = Arc::new(SettingsStore::new(dir.to_str().unwrap()).unwrap());
        let state = new_shared_state(VidhanaConfig::default());
        let service = Arc::new(SettingsService::new(
            state.clone(),
            store,
            Arc::new(NoopBackend),
        ));
        AppState {
            settings: state,
            service,
            hoosh: None,
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
        assert_eq!(state.settings.read().unwrap().display.brightness, 42);
        assert_eq!(state.settings.read().unwrap().display.theme, Theme::Dark);
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

        let loaded = state.service.store.load_state().unwrap().unwrap();
        assert_eq!(loaded.display.brightness, 50);

        let changes = state.service.store.recent_changes(10).unwrap();
        assert!(!changes.is_empty());
        assert_eq!(changes[0].category, "display");
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

        let display = DisplaySettings {
            brightness: 42,
            ..DisplaySettings::default()
        };
        let req = Request::post("/v1/settings/display")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&display).unwrap()))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::get("/v1/settings/history")
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

    #[tokio::test]
    async fn test_nl_endpoint_recognized() {
        let app = router(test_app());
        let req = Request::post("/v1/nl")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"text": "turn off wifi"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(parsed["intent"].is_object());
        assert_eq!(parsed["intent"]["key"], "wifi_enabled");
    }

    #[tokio::test]
    async fn test_nl_endpoint_unrecognized() {
        let app = router(test_app());
        let req = Request::post("/v1/nl")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"text": "what is the meaning of life"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(parsed["intent"].is_null());
    }
}
