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
    // Importamos os novos Enums
    models::inventory::{StockMovementReason, ItemKind, CompositionType},
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
//  CREATE ITEM (ATUALIZADO)
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemPayload {
    // Campos Básicos
    #[validate(length(min = 1, message = "O SKU é obrigatório."))]
    pub sku: String,

    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,

    pub description: Option<String>,

    #[validate(required(message = "O campo 'categoryId' é obrigatório."))]
    pub category_id: Option<Uuid>,

    #[validate(required(message = "O campo 'baseUnitId' é obrigatório."))]
    pub base_unit_id: Option<Uuid>,

    // [NOVO] Tipo do Item (Product, Service, Resource, Bundle)
    // Se não vier, o serde pode falhar ou podemos assumir Product no front
    pub kind: ItemKind,

    // [NOVO] Configurações Flexíveis (JSON)
    pub settings: Option<Value>,

    // Preços
    #[validate(custom(function = "validate_not_negative"))]
    pub sale_price: Decimal,

    // Estoque Inicial (Opcional se for Serviço, Obrigatório se tiver stock > 0)
    pub location_id: Option<Uuid>,

    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)]
    pub initial_stock: Decimal,

    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)]
    pub initial_cost: Decimal,

    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)]
    pub low_stock_threshold: Decimal,
}

// Validação de Consistência
impl CreateItemPayload {
    fn validate_consistency(&self) -> Result<(), ValidationError> {
        // Regra 1: Se for PRODUTO e tiver estoque inicial > 0, precisa de local.
        // Se for SERVIÇO, initial_stock é ignorado no backend, então não cobramos location.
        if self.kind == ItemKind::Product && self.initial_stock > Decimal::ZERO && self.location_id.is_none() {
            return Err(ValidationError::new("LocationRequiredForStock"));
        }
        Ok(())
    }
}

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
            payload.base_unit_id.unwrap(), // Validado pelo required
            &payload.sku,
            &payload.name,
            payload.description.as_deref(),

            // Novos Argumentos
            payload.kind,
            payload.settings,

            payload.initial_stock,
            payload.initial_cost,
            payload.sale_price,
            None, // min_stock (futuro)
            payload.low_stock_threshold,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(new_item)))
}

// =============================================================================
//  COMPOSIÇÃO / FICHA TÉCNICA (NOVO)
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddCompositionPayload {
    pub child_item_id: Uuid,

    #[validate(custom(function = "validate_not_negative"))]
    pub quantity: Decimal,

    pub comp_type: CompositionType,
}

pub async fn add_composition_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(parent_id): Path<Uuid>, // ID do item pai na URL
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
//  GET ITEMS (Mantido e atualizado RLS)
// =============================================================================

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
//  AUXILIARES (Categories, Units) - Mantenha o código existente
//  (Copie e cole aqui os handlers create_unit, get_all_units, create_category, etc.)
//  Eles não mudaram de lógica, apenas precisam estar presentes no arquivo final.
// =============================================================================

// --- Payload: CreateUnitPayload ---
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUnitPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,
    #[validate(length(min = 1, message = "O símbolo é obrigatório."))]
    pub symbol: String,
}

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

// --- Payload: CreateCategoryPayload ---
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryPayload {
    #[validate(length(min = 1, message = "O nome é obrigatório."))]
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

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

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddStockPayload {
    pub location_id: Uuid,
    pub item_id: Uuid,
    #[validate(custom(function = "validate_not_negative"))]
    pub quantity: Decimal,
    #[validate(custom(function = "validate_not_negative"))]
    pub unit_cost: Decimal,
    pub reason: StockMovementReason,
    pub notes: Option<String>,
    pub batch_number: Option<String>,
    pub expiration_date: Option<NaiveDate>,
    pub position: Option<String>,
}

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

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SellItemPayload {
    pub location_id: Uuid,
    pub item_id: Uuid,
    #[validate(custom(function = "validate_not_negative"))]
    pub quantity: Decimal,
    #[validate(custom(function = "validate_not_negative"))]
    pub unit_price: Decimal,
    pub batch_number: Option<String>,
    pub position: Option<String>,
}

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