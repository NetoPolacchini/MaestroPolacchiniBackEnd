// src/handlers/operations.rs

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

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
    models::operations::PipelineCategory,
};

// =============================================================================
//  1. CONFIGURAÇÃO (PIPELINES & STAGES)
//  Usado principalmente pelos Seeds/Templates
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePipelinePayload {
    #[validate(length(min = 1, message = "required"))]
    pub name: String,

    #[serde(default)]
    pub is_default: bool,
}

pub async fn create_pipeline(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<CreatePipelinePayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let pipeline = app_state.operations_service
        .create_pipeline(&mut *rls_conn, tenant.0, &payload.name, payload.is_default)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(pipeline)))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddStagePayload {
    #[validate(length(min = 1, message = "required"))]
    pub name: String,

    pub category: PipelineCategory,

    pub position: i32,

    // Configurações Avançadas (Gatilhos)
    pub stock_action: Option<String>, // "NONE", "RESERVE", "DEDUCT"
}

pub async fn add_stage(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(pipeline_id): Path<Uuid>,
    Json(payload): Json<AddStagePayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let stage = app_state.operations_service
        .add_stage(
            &mut *rls_conn,
            tenant.0,
            pipeline_id,
            &payload.name,
            payload.category,
            payload.position,
            payload.stock_action.as_deref()
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(stage)))
}

// =============================================================================
//  2. OPERAÇÃO (PEDIDOS)
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderPayload {
    pub pipeline_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub notes: Option<String>,
}

pub async fn create_order(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<CreateOrderPayload>,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // O Service automaticamente coloca o pedido na primeira etapa do pipeline
    let order = app_state.operations_service
        .create_order(
            &mut *rls_conn,
            tenant.0,
            payload.customer_id,
            payload.pipeline_id,
            payload.notes.as_deref()
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(order)))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderItemPayload {
    pub item_id: Uuid,

    // O usuário diz quanto quer vender e a que preço (pode ter desconto manual)
    pub quantity: Decimal,
    pub unit_price: Decimal,

    // NOTA: unit_cost não vem do frontend por segurança.
    // Deveria ser buscado do cadastro do item. Por enquanto passaremos 0.
}

pub async fn add_order_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(order_id): Path<Uuid>,
    Json(payload): Json<AddOrderItemPayload>,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // 1. Busca o Item para saber o Custo (Cost Price)
    // Isso evita que a gente passe 0 ou confie no frontend para dados sensíveis
    let item_data = app_state.inventory_service
        .get_item(&mut *rls_conn, tenant.0, payload.item_id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    let cost = match item_data {
        Some(i) => i.cost_price.unwrap_or(Decimal::ZERO),
        None => return Err(AppError::ResourceNotFound(item_data.unwrap().name.to_string()).to_api_error(&locale, &app_state.i18n_store)),    };

    // 2. Adiciona ao pedido usando o custo real do banco
    let item = app_state.operations_service
        .add_item_to_order(
            &mut *rls_conn,
            tenant.0,
            order_id,
            payload.item_id,
            payload.quantity,
            payload.unit_price,
            cost // <--- Agora sim! Custo real.
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(item)))
}

// =============================================================================
//  3. TRANSIÇÃO (A MÁGICA)
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TransitionOrderPayload {
    pub new_stage_id: Uuid,
}

pub async fn transition_order(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(order_id): Path<Uuid>,
    Json(payload): Json<TransitionOrderPayload>,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    app_state.operations_service
        .transition_order(
            &mut *rls_conn,
            tenant.0,
            order_id,
            payload.new_stage_id
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(StatusCode::OK)
}