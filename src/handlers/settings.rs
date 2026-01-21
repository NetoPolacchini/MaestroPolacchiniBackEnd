// src/handlers/settings.rs

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    common::{
        error::{ApiError, AppError},
        db_utils::get_rls_connection,
    },
    config::AppState,
    middleware::{
        auth::AuthenticatedUser,
        i18n::Locale,
        tenancy::TenantContext,
    },
    models::settings::UpdateSettingsRequest,
};
use crate::models::settings::TenantSettings;

// GET /api/settings
#[utoipa::path(
    get,
    path = "/api/settings",
    tag = "Settings",
    responses(
        (status = 200, description = "Configurações da loja recuperadas", body = TenantSettings),
        (status = 401, description = "Não autorizado"),
    ),
    security(
        ("api_jwt" = []) // Define que precisa do cadeadinho (Token)
    )
)]
pub async fn get_settings(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    // Obtém conexão segura com RLS
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let settings = app_state.settings_repo
        .get_settings(&mut *rls_conn, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(settings)))
}

// PUT /api/settings
#[utoipa::path(
    put,
    path = "/api/settings",
    tag = "Settings",
    request_body = UpdateSettingsRequest, // Define o corpo da requisição
    responses(
        (status = 200, description = "Configurações atualizadas", body = TenantSettings),
        (status = 401, description = "Não autorizado"),
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn update_settings(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let updated = app_state.settings_repo
        .update_settings(&mut *rls_conn, tenant.0, payload)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(updated)))
}