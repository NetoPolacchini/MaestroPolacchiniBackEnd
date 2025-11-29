// src/handlers/tenancy.rs

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

// Importa os nossos extratores e erros
use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{
        auth::AuthenticatedUser, // O extrator de Utilizador
        i18n::Locale,           // O extrator de Idioma
        tenancy::TenantContext
    },
    models::tenancy::Tenant,
};

// ---
// 1. "Payload" (O "Formulário" da API)
// ---
// O que o cliente precisa de enviar para criar um estabelecimento
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTenantPayload {
    #[validate(length(min = 1, message = "O nome do estabelecimento é obrigatório."))]
    pub name: String,
    pub description: Option<String>, // <-- ADICIONE ESTA LINHA
}

// ---
// 2. O "Handler" (A Rota)
// ---
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

    // 2. Chamar o Serviço (Lógica de Negócio)
    // Esta é uma operação transacional (criar o tenant E ligar o utilizador)
    // Por isso, chamamos um "Serviço", que ainda não criámos.
    let new_tenant = app_state
        .tenant_service
        .create_tenant_and_assign_owner(
            &payload.name,
            payload.description.as_deref(),
            user.0.id,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    // 3. Responder com Sucesso
    Ok((StatusCode::CREATED, Json(new_tenant)))
}

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateStockPoolPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,
    pub description: Option<String>,
}

pub async fn create_stock_pool(
    State(app_state): State<AppState>,
    locale: Locale,
    // Precisamos de TenantContext aqui, pois Pools pertencem a um Tenant!
    tenant: TenantContext,
    // Precisamos de user para o auth_middleware (tenant_guard), mas não usamos aqui diretamente
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

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateLocationPayload {
    // REMOVIDO: pub stock_pool_id: Option<Uuid>,
    // Agora o sistema cria o pool automaticamente!

    #[validate(length(min = 3, message = "O nome deve ter no mínimo 3 caracteres"))]
    pub name: String,

    pub is_warehouse: bool,
}

pub async fn create_location(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateLocationPayload>,
) -> Result<impl IntoResponse, ApiError> {

    // 1. Valida
    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // 2. Chama o Serviço "Inteligente" (Transacional)
    // Note que passamos &app_state.db_pool como executor
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

// --- NOVO ENDPOINT: Listar Lojas ---
pub async fn list_locations(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    _user: AuthenticatedUser,
) -> Result<impl IntoResponse, ApiError> {

    // Precisamos adicionar um método simples 'list_locations' no TenantService/Repo
    // Vou assumir que você tem ou vai criar: tenant_repo.list_locations(tenant_id)

    let locations = app_state.tenant_service
        .list_locations(tenant.0) // Você precisará criar esse método simples no Service
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(locations)))
}