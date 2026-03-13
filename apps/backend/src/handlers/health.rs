use crate::AppState;
use axum::{extract::State, http::header, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub ai_precheck: crate::StartupAiPrecheck,
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        ai_precheck: state.startup_ai_precheck.clone(),
    })
}

pub async fn openapi_yaml() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/yaml; charset=utf-8")],
        include_str!("../../openapi/openapi.yaml"),
    )
}
