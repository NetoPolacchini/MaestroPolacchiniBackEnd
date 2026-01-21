// src/handlers/tenancy.rs

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use validator::Validate;
use utoipa::ToSchema; // <--- Importe ToSchema
use uuid::Uuid;       // <--- Importe Uuid para o params do Swagger

// Importa os nossos extratores, erros e models
use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{
        auth::AuthenticatedUser,
        i18n::Locale,
        tenancy::TenantContext
    },
    // Importamos os models de resposta para o Swagger
    models::tenancy::{Tenant, StockPool, Location},
};

// =============================================================================
//  GLOBAL USER ROUTES (Não precisa de x-tenant-id)
// =============================================================================

// ---
// 1. "Payload" (O "Formulário" da API)
// ---
#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
pub struct CreateTenantPayload {
    #[validate(length(min = 1, message = "O nome do estabelecimento é obrigatório."))]
    #[schema(example = "Minha Loja Matriz")]
    pub name: String,

    #[schema(example = "Loja principal localizada no centro")]
    pub description: Option<String>,
}

// POST /api/tenants
#[utoipa::path(
    post,
    path = "/api/tenants",
    tag = "Tenancy",
    request_body = CreateTenantPayload,
    responses(
        (status = 201, description = "Estabelecimento criado com sucesso", body = Tenant),
        (status = 400, description = "Dados inválidos")
    ),
    security(
        ("api_jwt" = []) // Apenas Token
    )
)]
pub async fn create_tenant(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    Json(payload): Json<CreateTenantPayload>,
) -> Result<impl IntoResponse, ApiError> {

    // 1. Validar o payload
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // 2. Chamar o Serviço
    let new_tenant = app_state
        .tenant_service
        .create_tenant_with_owner(
            &payload.name,
            payload.description.as_deref(),
            user.0.id,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(new_tenant)))
}

// GET /api/tenants
#[utoipa::path(
    get,
    path = "/api/tenants",
    tag = "Tenancy",
    responses(
        (status = 200, description = "Lista de lojas que o usuário tem acesso", body = Vec<Tenant>)
    ),
    security(
        ("api_jwt" = []) // Apenas Token
    )
)]
pub async fn list_my_tenants(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
) -> Result<impl IntoResponse, ApiError> {

    let tenants = app_state
        .tenant_service
        .list_user_tenants(user.0.id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(tenants)))
}

// =============================================================================
//  TENANT SETUP ROUTES (Precisa de x-tenant-id)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
pub struct CreateStockPoolPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    #[schema(example = "Estoque Central")]
    pub name: String,

    #[schema(example = "Armazém principal")]
    pub description: Option<String>,
}

// POST /api/tenants/setup/pools
#[utoipa::path(
    post,
    path = "/api/tenants/setup/pools",
    tag = "Tenancy Setup",
    request_body = CreateStockPoolPayload,
    responses(
        (status = 201, description = "Pool de estoque criado", body = StockPool)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn create_stock_pool(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateStockPoolPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let pool = app_state.tenant_service
        .create_stock_pool(tenant.0, &payload.name, payload.description.as_deref(),)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(pool)))
}


// ---
// Gestão de Locations
// ---

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreateLocationPayload {
    #[validate(length(min = 3, message = "O nome deve ter no mínimo 3 caracteres"))]
    #[schema(example = "Prateleira A1")]
    pub name: String,

    #[schema(example = true)]
    pub is_warehouse: bool,
}

// POST /api/tenants/setup/locations
#[utoipa::path(
    post,
    path = "/api/tenants/setup/locations",
    tag = "Tenancy Setup",
    request_body = CreateLocationPayload,
    responses(
        (status = 201, description = "Localização criada", body = Location)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn create_location(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateLocationPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let location = app_state.tenant_service
        .create_location_standalone(
            &app_state.db_pool,
            tenant.0,
            &payload.name,
            payload.is_warehouse
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(location)))
}

// GET /api/tenants/setup/locations
#[utoipa::path(
    get,
    path = "/api/tenants/setup/locations",
    tag = "Tenancy Setup",
    responses(
        (status = 200, description = "Lista de localizações", body = Vec<Location>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn list_locations(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    _user: AuthenticatedUser,
) -> Result<impl IntoResponse, ApiError> {

    let locations = app_state.tenant_service
        .list_locations(tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(locations)))
}