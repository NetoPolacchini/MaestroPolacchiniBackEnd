// src/handlers/rbac.rs

use axum::{extract::State, Json, response::IntoResponse, http::StatusCode};
use uuid::Uuid; // Necessário para o tipo no parametro do Swagger

use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{tenancy::TenantContext, i18n::Locale},
    models::rbac::{CreateRolePayload, RoleResponse, Permission}, // Importe Permission e RoleResponse
};

// =============================================================================
//  CREATE ROLE
// =============================================================================

// POST /api/rbac/roles
#[utoipa::path(
    post,
    path = "/api/rbac/roles",
    tag = "RBAC",
    request_body = CreateRolePayload,
    responses(
        (status = 201, description = "Cargo criado com sucesso", body = RoleResponse),
        (status = 400, description = "Dados inválidos"),
        (status = 403, description = "Sem permissão (Requer 'access_control:write')")
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
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

// =============================================================================
//  LIST PERMISSIONS
// =============================================================================

// GET /api/permissions
#[utoipa::path(
    get,
    path = "/api/permissions",
    tag = "RBAC",
    responses(
        (status = 200, description = "Lista todas as permissões disponíveis no sistema", body = Vec<Permission>)
    ),
    // Nota: Esta rota não pede TenantContext, então não precisa do Header x-tenant-id
    security(
        ("api_jwt" = [])
    )
)]
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