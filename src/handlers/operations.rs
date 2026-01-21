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
use utoipa::ToSchema; // <--- Importe ToSchema

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
    // Importe os models de resposta para o Swagger
    models::operations::{Pipeline, PipelineStage, PipelineCategory, Order, OrderItem},
};

// =============================================================================
//  1. CONFIGURAÇÃO (PIPELINES & STAGES)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreatePipelinePayload {
    #[validate(length(min = 1, message = "required"))]
    #[schema(example = "Funil de Vendas B2B")]
    pub name: String,

    #[serde(default)]
    #[schema(example = true)]
    pub is_default: bool,
}

// POST /api/operations/pipelines
#[utoipa::path(
    post,
    path = "/api/operations/pipelines",
    tag = "Operations",
    request_body = CreatePipelinePayload,
    responses(
        (status = 201, description = "Funil criado", body = Pipeline)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct AddStagePayload {
    #[validate(length(min = 1, message = "required"))]
    #[schema(example = "Negociação")]
    pub name: String,

    #[schema(example = "Active")]
    pub category: PipelineCategory,

    #[schema(example = 1)]
    pub position: i32,

    //TodO: Trocar segundo example
    #[schema(example = "RESERVE", example = "Ação de estoque: NONE, RESERVE, DEDUCT")]
    pub stock_action: Option<String>,
}

// POST /api/operations/pipelines/{id}/stages
#[utoipa::path(
    post,
    path = "/api/operations/pipelines/{pipeline_id}/stages",
    tag = "Operations",
    request_body = AddStagePayload,
    responses(
        (status = 201, description = "Etapa adicionada ao funil", body = PipelineStage)
    ),
    params(
        ("pipeline_id" = Uuid, Path, description = "ID do Funil"),
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreateOrderPayload {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub pipeline_id: Uuid,

    pub customer_id: Option<Uuid>,

    #[schema(example = "Pedido urgente do cliente VIP")]
    pub notes: Option<String>,
}

// POST /api/operations/orders
#[utoipa::path(
    post,
    path = "/api/operations/orders",
    tag = "Operations",
    request_body = CreateOrderPayload,
    responses(
        (status = 201, description = "Pedido criado (iniciado na 1ª etapa)", body = Order)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct AddOrderItemPayload {
    pub item_id: Uuid,

    #[schema(example = "2.0")]
    pub quantity: Decimal,

    #[schema(example = "50.00")]
    pub unit_price: Decimal,
}

// POST /api/operations/orders/{id}/items
#[utoipa::path(
    post,
    path = "/api/operations/orders/{order_id}/items",
    tag = "Operations",
    request_body = AddOrderItemPayload,
    responses(
        (status = 201, description = "Item adicionado ao pedido", body = OrderItem),
        (status = 404, description = "Pedido ou Item não encontrado")
    ),
    params(
        ("order_id" = Uuid, Path, description = "ID do Pedido"),
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
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

    let item_data = app_state.inventory_service
        .get_item(&mut *rls_conn, tenant.0, payload.item_id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    let cost = match item_data {
        Some(i) => i.cost_price.unwrap_or(Decimal::ZERO),
        None => return Err(AppError::ResourceNotFound(format!("Item {}", payload.item_id)).to_api_error(&locale, &app_state.i18n_store)), // Pequena correção para string
    };

    let item = app_state.operations_service
        .add_item_to_order(
            &mut *rls_conn,
            tenant.0,
            order_id,
            payload.item_id,
            payload.quantity,
            payload.unit_price,
            cost
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(item)))
}

// =============================================================================
//  3. TRANSIÇÃO (A MÁGICA)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct TransitionOrderPayload {
    pub new_stage_id: Uuid,
}

// POST /api/operations/orders/{id}/transition
#[utoipa::path(
    post,
    path = "/api/operations/orders/{order_id}/transition",
    tag = "Operations",
    request_body = TransitionOrderPayload,
    responses(
        (status = 200, description = "Pedido movido para nova etapa (Estoque/Financeiro atualizados)"),
        (status = 400, description = "Transição inválida")
    ),
    params(
        ("order_id" = Uuid, Path, description = "ID do Pedido"),
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
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