// src/handlers/inventory.rs

use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;
use validator::{Validate, ValidationError};
use utoipa::ToSchema;

use crate::{
    common::{
        error::{ApiError, AppError},
        db_utils::get_rls_connection
    },
    config::AppState,
    middleware::{
        auth::AuthenticatedUser,
        i18n::Locale,
        tenancy::TenantContext,
        rbac::{RequirePermission, PermInventoryWrite},
    },
    models::inventory::{
        StockMovementReason, ItemKind, CompositionType,
        Item, CompositionEntry, UnitOfMeasure, Category, InventoryLevel
    },
};
use chrono::NaiveDate;

// --- Validações Auxiliares ---
fn validate_not_negative(val: &Decimal) -> Result<(), ValidationError> {
    if val.is_sign_negative() {
        let mut err = ValidationError::new("range");
        err.message = Some("O valor não pode ser negativo.".into());
        return Err(err);
    }
    Ok(())
}

// =============================================================================
//  CREATE ITEM
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemPayload {
    #[validate(length(min = 1, message = "O SKU é obrigatório."))]
    #[schema(example = "PROD-ABC-001")]
    pub sku: String,

    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    #[schema(example = "Produto Exemplo")]
    pub name: String,

    #[schema(example = "Descrição detalhada do produto")]
    pub description: Option<String>,

    #[validate(required(message = "O campo 'categoryId' é obrigatório."))]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub category_id: Option<Uuid>,

    #[validate(required(message = "O campo 'baseUnitId' é obrigatório."))]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440002")]
    pub base_unit_id: Option<Uuid>,

    #[schema(example = "Product")]
    pub kind: ItemKind,

    // Configurações Flexíveis (JSON)
    pub settings: Option<Value>,

    #[validate(custom(function = "validate_not_negative"))]
    #[schema(example = "50.00")]
    pub sale_price: Decimal,

    // Estoque Inicial
    pub location_id: Option<Uuid>,

    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)]
    #[schema(example = "100.0")]
    pub initial_stock: Decimal,

    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)]
    #[schema(example = "25.00")]
    pub initial_cost: Decimal,

    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)]
    #[schema(example = "10.0")]
    pub low_stock_threshold: Decimal,
}

impl CreateItemPayload {
    fn validate_consistency(&self) -> Result<(), ValidationError> {
        if self.kind == ItemKind::Product && self.initial_stock > Decimal::ZERO && self.location_id.is_none() {
            return Err(ValidationError::new("LocationRequiredForStock"));
        }
        Ok(())
    }
}

// POST /api/inventory/items
#[utoipa::path(
    post,
    path = "/api/inventory/items",
    tag = "Inventory",
    request_body = CreateItemPayload,
    responses(
        (status = 201, description = "Item criado com sucesso", body = Item),
        (status = 400, description = "Dados inválidos"),
        (status = 403, description = "Sem permissão")
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn create_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    _guard: RequirePermission<PermInventoryWrite>,
    Json(payload): Json<CreateItemPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    payload.validate_consistency()
        .map_err(|e| {
            let mut errors = validator::ValidationErrors::new();
            errors.add("locationId", e);
            AppError::ValidationError(errors).to_api_error(&locale, &app_state.i18n_store)
        })?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let new_item = app_state.inventory_service
        .create_item(
            &mut *rls_conn,
            tenant.0,
            payload.location_id,
            payload.category_id,
            payload.base_unit_id.unwrap(),
            &payload.sku,
            &payload.name,
            payload.description.as_deref(),
            payload.kind,
            payload.settings,
            payload.initial_stock,
            payload.initial_cost,
            payload.sale_price,
            None,
            payload.low_stock_threshold,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(new_item)))
}

// =============================================================================
//  COMPOSIÇÃO / FICHA TÉCNICA
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddCompositionPayload {
    pub child_item_id: Uuid,

    #[validate(custom(function = "validate_not_negative"))]
    #[schema(example = "0.5")]
    pub quantity: Decimal,

    #[schema(example = "Component")]
    pub comp_type: CompositionType,
}

// POST /api/inventory/items/{id}/composition
#[utoipa::path(
    post,
    path = "/api/inventory/items/{parent_id}/composition",
    tag = "Inventory",
    params(
        ("parent_id" = Uuid, Path, description = "ID do Item Pai (Produto Final)"),
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    request_body = AddCompositionPayload,
    responses(
        (status = 201, description = "Item adicionado à composição"),
        (status = 404, description = "Item não encontrado")
    ),
    security(("api_jwt" = []))
)]
pub async fn add_composition_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(parent_id): Path<Uuid>,
    Json(payload): Json<AddCompositionPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    app_state.inventory_service
        .add_composition_item(
            &mut *rls_conn,
            tenant.0,
            parent_id,
            payload.child_item_id,
            payload.quantity,
            payload.comp_type
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(StatusCode::CREATED)
}

// GET /api/inventory/items/{id}/composition
#[utoipa::path(
    get,
    path = "/api/inventory/items/{parent_id}/composition",
    tag = "Inventory",
    params(
        ("parent_id" = Uuid, Path, description = "ID do Item Pai"),
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    responses(
        (status = 200, description = "Lista de componentes", body = Vec<CompositionEntry>)
    ),
    security(("api_jwt" = []))
)]
pub async fn get_item_composition(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(parent_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let composition = app_state.inventory_service
        .get_item_composition(&mut *rls_conn, tenant.0, parent_id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(composition)))
}

// =============================================================================
//  GET ITEMS
// =============================================================================

// GET /api/inventory/items
#[utoipa::path(
    get,
    path = "/api/inventory/items",
    tag = "Inventory",
    responses(
        (status = 200, description = "Listagem de Itens", body = Vec<Item>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn get_all_items(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let items = app_state.inventory_service
        .get_all_items(&mut *rls_conn, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(items)))
}

// =============================================================================
//  AUXILIARES (Categories, Units)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUnitPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    #[schema(example = "Quilograma")]
    pub name: String,
    #[validate(length(min = 1, message = "O símbolo é obrigatório."))]
    #[schema(example = "kg")]
    pub symbol: String,
}

// POST /api/inventory/units
#[utoipa::path(
    post,
    path = "/api/inventory/units",
    tag = "Inventory",
    request_body = CreateUnitPayload,
    responses(
        (status = 201, description = "Unidade criada", body = UnitOfMeasure)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn create_unit_of_measure(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<CreateUnitPayload>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user).await.map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;
    let unit = app_state.inventory_service.create_unit(&mut *rls_conn, tenant.0, &payload.name, &payload.symbol).await.map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    Ok((StatusCode::CREATED, Json(unit)))
}

// GET /api/inventory/units
#[utoipa::path(
    get,
    path = "/api/inventory/units",
    tag = "Inventory",
    responses(
        (status = 200, description = "Lista de unidades", body = Vec<UnitOfMeasure>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn get_all_units(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user).await.map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;
    let units = app_state.inventory_service.get_all_units(&mut *rls_conn, tenant.0).await.map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    Ok((StatusCode::OK, Json(units)))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    #[schema(example = "Bebidas")]
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

// POST /api/inventory/categories
#[utoipa::path(
    post,
    path = "/api/inventory/categories",
    tag = "Inventory",
    request_body = CreateCategoryPayload,
    responses(
        (status = 201, description = "Categoria criada", body = Category)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn create_category(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<CreateCategoryPayload>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user).await.map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;
    let category = app_state.inventory_service.create_category(&mut *rls_conn, tenant.0, &payload.name, payload.description.as_deref(), payload.parent_id).await.map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    Ok((StatusCode::CREATED, Json(category)))
}

// GET /api/inventory/categories
#[utoipa::path(
    get,
    path = "/api/inventory/categories",
    tag = "Inventory",
    responses(
        (status = 200, description = "Lista de categorias", body = Vec<Category>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn get_all_categories(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user).await.map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;
    let categories = app_state.inventory_service.get_all_categories(&mut *rls_conn, tenant.0).await.map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    Ok((StatusCode::OK, Json(categories)))
}

// =============================================================================
//  MOVIMENTAÇÃO DE ESTOQUE (Add/Sell)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddStockPayload {
    pub location_id: Uuid,
    pub item_id: Uuid,
    #[validate(custom(function = "validate_not_negative"))]
    #[schema(example = "100.0")]
    pub quantity: Decimal,
    #[validate(custom(function = "validate_not_negative"))]
    #[schema(example = "25.50")]
    pub unit_cost: Decimal,
    #[schema(example = "Purchase")]
    pub reason: StockMovementReason,
    pub notes: Option<String>,
    pub batch_number: Option<String>,
    pub expiration_date: Option<NaiveDate>,
    pub position: Option<String>,
}

// POST /api/inventory/stock-entry
#[utoipa::path(
    post,
    path = "/api/inventory/stock-entry",
    tag = "Inventory",
    request_body = AddStockPayload,
    responses(
        (status = 200, description = "Estoque adicionado", body = InventoryLevel)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn add_stock(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<AddStockPayload>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user).await.map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;
    let updated_level = app_state.inventory_service.add_stock(
        &mut *rls_conn, tenant.0, payload.item_id, payload.location_id,
        payload.quantity, payload.unit_cost, payload.reason, payload.notes.as_deref(),
        payload.batch_number, payload.expiration_date, payload.position,
    ).await.map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    Ok((StatusCode::OK, Json(updated_level)))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SellItemPayload {
    pub location_id: Uuid,
    pub item_id: Uuid,
    #[validate(custom(function = "validate_not_negative"))]
    #[schema(example = "1.0")]
    pub quantity: Decimal,
    #[validate(custom(function = "validate_not_negative"))]
    #[schema(example = "50.00")]
    pub unit_price: Decimal,
    pub batch_number: Option<String>,
    pub position: Option<String>,
}

// POST /api/inventory/sell
#[utoipa::path(
    post,
    path = "/api/inventory/sell",
    tag = "Inventory",
    request_body = SellItemPayload,
    responses(
        (status = 200, description = "Item vendido (estoque baixado)")
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn sell_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<SellItemPayload>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate().map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user).await.map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;
    app_state.inventory_service.sell_item(
        &mut *rls_conn, tenant.0, payload.item_id, payload.location_id,
        payload.quantity, payload.unit_price, false, Some("Venda via API"),
        payload.batch_number, payload.position,
    ).await.map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    Ok(StatusCode::OK)
}