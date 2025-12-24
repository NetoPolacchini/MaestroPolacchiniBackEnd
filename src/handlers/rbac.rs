// src/handlers/rbac.rs

use axum::{extract::State, Json, response::IntoResponse, http::StatusCode};
use crate::{
    common::error::ApiError,
    config::AppState,
    middleware::{tenancy::TenantContext, i18n::Locale},
    models::rbac::CreateRolePayload,
};
use crate::common::error::AppError; // Adicione este import

// POST /api/tenants/roles
pub async fn create_role(
    State(app_state): State<AppState>,
    tenant: TenantContext,
    locale: Locale,
    Json(payload): Json<CreateRolePayload>,
) -> Result<impl IntoResponse, ApiError> {

    let response = app_state.rbac_service
        .create_role_with_permissions(
            tenant.0, // ID do Tenant
            payload.name,
            payload.description,
            payload.permissions
        )
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(response)))
}

// GET /api/permissions (Para o frontend saber o que mostrar na tela de criação)
pub async fn list_permissions(
    State(app_state): State<AppState>,
    locale: Locale,
) -> Result<impl IntoResponse, ApiError> {

    let permissions = app_state.rbac_service
        .list_system_permissions()
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(Json(permissions))
}