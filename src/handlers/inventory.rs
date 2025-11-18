// src/handlers/inventory.rs

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;
use validator::{Validate, ValidationError};
use sqlx::Acquire;

// Importa os nossos extratores e erros
use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{
        auth::AuthenticatedUser, // O extrator de Utilizador
        i18n::Locale,            // O extrator de Idioma
        tenancy::TenantContext,  // O extrator de Tenant (do X-Tenant-ID)
    },
};

// ---
// Validação Customizada (Corrigida)
// ---
fn validate_not_negative(val: &Decimal) -> Result<(), ValidationError> {
    if val.is_sign_negative() {
        let mut err = ValidationError::new("range");
        err.add_param("min".into(), &0.0);
        err.message = Some("O valor não pode ser negativo.".into());
        return Err(err);
    }
    Ok(())
}

// ---
// Helper RLS: A "Chave" para o Banco de Dados
// ---
/// Adquire uma conexão da pool e define as variáveis RLS (a "chave").
async fn get_rls_connection(
    app_state: &AppState,
    tenant_ctx: &TenantContext,
    user: &AuthenticatedUser,
    locale: &Locale,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, ApiError> {

    // 1. Adquire uma conexão única da pool
    let mut conn = app_state.db_pool.acquire().await.map_err(|e| {
        tracing::error!("Falha ao adquirir conexão da pool: {}", e);
        AppError::DatabaseError(e).to_api_error(locale, &app_state.i18n_store)
    })?;

    // 2. Define o tenant_id nesta conexão específica.
    // O RLS no PostgreSQL irá agora usar este valor.
    sqlx::query("SET LOCAL app.tenant_id = $1")
        .bind(tenant_ctx.0)
        .execute(&mut *conn) // Usa &mut *conn (deref)
        .await
        .map_err(|e| {
            tracing::error!("Falha ao definir RLS app.tenant_id: {}", e);
            AppError::DatabaseError(e).to_api_error(locale, &app_state.i18n_store)
        })?;

    // 3. Define o user_id (para futura auditoria a nível de banco)
    sqlx::query("SET LOCAL app.user_id = $1")
        .bind(user.0.id) // user.0 dá acesso ao 'User' dentro do 'AuthenticatedUser'
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Falha ao definir RLS app.user_id: {}", e);
            AppError::DatabaseError(e).to_api_error(locale, &app_state.i18n_store)
        })?;

    // 4. Retorna a conexão "pronta" e segura
    Ok(conn)
}


// ---
// Payload: CreateItem (O mesmo)
// ---
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemPayload {
    #[validate(required(message = "O campo 'categoryId' é obrigatório."))]
    pub category_id: Option<Uuid>,
    #[validate(required(message = "O campo 'baseUnitId' é obrigatório."))]
    pub base_unit_id: Option<Uuid>,
    #[validate(required(message = "O campo 'locationId' é obrigatório."))]
    pub location_id: Option<Uuid>,
    #[validate(length(min = 1, message = "O SKU é obrigatório."))]
    pub sku: String,
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,
    pub description: Option<String>,
    #[validate(custom(function = "validate_not_negative"))]
    pub initial_stock: Decimal,
    #[validate(custom(function = "validate_not_negative"))]
    pub low_stock_threshold: Decimal,
}

// ---
// Handler: create_item (Refatorado para RLS)
// ---
pub async fn create_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser, // <-- Mudança: obtemos o 'user'
    tenant: TenantContext,
    Json(payload): Json<CreateItemPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // 1. Adquire uma conexão segura com RLS
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user, &locale).await?;

    // 2. O Serviço de Inventário agora é chamado com a conexão RLS
    // (O serviço irá iniciar a sua própria transação a partir desta conexão)
    let new_item = app_state
        .inventory_service
        .create_item_with_initial_stock(
            &mut *rls_conn, // <-- MUDANÇA: Passa a conexão RLS
            tenant.0,
            payload.location_id.unwrap(),
            payload.category_id.unwrap(),
            payload.base_unit_id.unwrap(),
            &payload.sku,
            &payload.name,
            payload.description.as_deref(),
            payload.initial_stock,
            payload.low_stock_threshold,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    // A conexão 'rls_conn' é libertada e volta para a pool aqui.

    Ok((StatusCode::CREATED, Json(new_item)))
}

// ---
// Handler: get_all_items (Refatorado para RLS)
// ---
pub async fn get_all_items(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser, // <-- Mudança: obtemos o 'user'
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    // 1. Adquire uma conexão segura com RLS
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user, &locale).await?;

    // 2. Chama o repositório com a conexão RLS
    let items = app_state
        .inventory_repo
        .get_all_items(&mut *rls_conn, tenant.0) // <-- MUDANÇA: Passa a conexão RLS
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(items)))
}

// ---
// Payload: CreateUnitPayload (O mesmo)
// ---
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUnitPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,
    #[validate(length(min = 1, message = "O símbolo é obrigatório."))]
    pub symbol: String,
}

// ---
// Handler: create_unit_of_measure (Refatorado para RLS)
// ---
pub async fn create_unit_of_measure(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser, // <-- Mudança: obtemos o 'user'
    tenant: TenantContext,
    Json(payload): Json<CreateUnitPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // 1. Adquire uma conexão segura com RLS
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user, &locale).await?;

    // 2. Inicia uma transação a partir da conexão RLS
    let mut tx = rls_conn.begin().await.map_err(|e| {
        AppError::DatabaseError(e).to_api_error(&locale, &app_state.i18n_store)
    })?;

    // 3. Chama o repositório com a transação
    let unit = app_state
        .inventory_repo
        .create_unit(
            &mut *tx, // Passa a transação
            tenant.0,
            &payload.name,
            &payload.symbol
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    tx.commit().await.map_err(|e| {
        AppError::DatabaseError(e).to_api_error(&locale, &app_state.i18n_store)
    })?;

    Ok((StatusCode::CREATED, Json(unit)))
}

// ---
// Handler: get_all_units (Refatorado para RLS)
// ---
pub async fn get_all_units(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser, // <-- Mudança: obtemos o 'user'
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user, &locale).await?;

    let units = app_state
        .inventory_repo
        .get_all_units(&mut *rls_conn, tenant.0) // <-- MUDANÇA: Passa a conexão RLS
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(units)))
}


// ---
// Payload: CreateCategoryPayload (O mesmo)
// ---
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

// ---
// Handler: create_category (Refatorado para RLS)
// ---
pub async fn create_category(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser, // <-- Mudança: obtemos o 'user'
    tenant: TenantContext,
    Json(payload): Json<CreateCategoryPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user, &locale).await?;

    let mut tx = rls_conn.begin().await.map_err(|e| {
        AppError::DatabaseError(e).to_api_error(&locale, &app_state.i18n_store)
    })?;

    let category = app_state
        .inventory_repo
        .create_category(
            &mut *tx,
            tenant.0,
            &payload.name,
            payload.description.as_deref(),
            payload.parent_id,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    tx.commit().await.map_err(|e| {
        AppError::DatabaseError(e).to_api_error(&locale, &app_state.i18n_store)
    })?;

    Ok((StatusCode::CREATED, Json(category)))
}

// ---
// Handler: get_all_categories (Refatorado para RLS)
// ---
pub async fn get_all_categories(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser, // <-- Mudança: obtemos o 'user'
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user, &locale).await?;

    let categories = app_state
        .inventory_repo
        .get_all_categories(&mut *rls_conn, tenant.0) // <-- MUDANÇA: Passa a conexão RLS
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(categories)))
}