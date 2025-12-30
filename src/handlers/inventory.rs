// src/handlers/inventory.rs

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;
use validator::{Validate, ValidationError};

// Importa os nossos extratores e erros
use crate::{common::error::{ApiError, AppError}, config::AppState, middleware::{
    auth::AuthenticatedUser, // O extrator de Utilizador
    i18n::Locale,            // O extrator de Idioma
    tenancy::TenantContext,  // O extrator de Tenant (do X-Tenant-ID)
    rbac::{RequirePermission, PermInventoryWrite}, // Guardião e a Permissão específica
}
};
// Importa os nossos extratores e erros
use crate::common::db_utils::get_rls_connection;
use chrono::NaiveDate; // Importante para a validade
use crate::models::inventory::StockMovementReason; // Importe o Enum

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
// Payload: CreateItem (O mesmo)
// ---
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemPayload {
    #[validate(required(message = "O campo 'categoryId' é obrigatório."))]
    pub category_id: Option<Uuid>,

    #[validate(required(message = "O campo 'baseUnitId' é obrigatório."))]
    pub base_unit_id: Option<Uuid>,

    // [NOVO] O usuário precisa informar quanto custou esse estoque inicial
    // Se não tiver estoque inicial, pode mandar 0.
    #[validate(custom(function = "validate_not_negative"))]
    #[serde(default)] // Se o JSON não tiver esse campo, assume 0
    pub initial_cost: Decimal,

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

// MUDANÇA 2: Validação de Consistência
// O Rust permite adicionar lógica ao struct.
impl CreateItemPayload {
    fn validate_consistency(&self) -> Result<(), ValidationError> {
        // Regra: Se o estoque for maior que zero, PRECISAMOS saber onde guardar (location_id).
        if self.initial_stock > Decimal::ZERO && self.location_id.is_none() {
            return Err(ValidationError::new("LocationRequiredForStock"));
        }

        // Regra: Se definir alerta de estoque baixo, precisa de um local.
        if self.low_stock_threshold > Decimal::ZERO && self.location_id.is_none() {
            return Err(ValidationError::new("LocationRequiredForThreshold"));
        }

        Ok(())
    }
}

// ---
// Handler: create_item (Refatorado para RLS)
// ---
pub async fn create_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    _guard: RequirePermission<PermInventoryWrite>,
    Json(payload): Json<CreateItemPayload>,
) -> Result<impl IntoResponse, ApiError> {

    // Validação padrão do Validator
    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // MUDANÇA 3: Nossa validação de consistência manual
    payload.validate_consistency()
        .map_err(|e| {
            // Criamos um ValidationErrors manual para manter o padrão de resposta
            let mut errors = validator::ValidationErrors::new();
            errors.add("locationId", e); // Atribui o erro ao campo locationId
            AppError::ValidationError(errors).to_api_error(&locale, &app_state.i18n_store)
        })?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // MUDANÇA 4: Passamos location_id como Option (sem o unwrap)
    let new_item = app_state
        .inventory_service
        .create_item(
                      &mut *rls_conn,
                      tenant.0,
                      payload.location_id, // Passa Option<Uuid>
                      payload.category_id.unwrap(),
                      payload.base_unit_id.unwrap(),
                      &payload.sku,
                      &payload.name,
                      payload.description.as_deref(),
                      payload.initial_stock,
                      payload.initial_cost,
                      payload.low_stock_threshold,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(new_item)))
}

// ---
// Handler: get_all_items (Refatorado para RLS)
// ---
pub async fn get_all_items(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    // 1. Prepara a infraestrutura (Conexão Segura)
    // Usamos o helper refatorado (que retorna AppError)
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // 2. Chama o Service (Regra de Negócio)
    // Note que passamos &mut *rls_conn como executor
    let items = app_state
        .inventory_service // <--- AGORA SIM: Service
        .get_all_items(&mut *rls_conn, tenant.0)
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
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<CreateUnitPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // 1. Adquire Conexão RLS
    // (Não precisa iniciar Transaction se for apenas 1 operação de banco)
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // 2. Chama o Service (passando a conexão RLS como executor)
    let unit = app_state
        .inventory_service // <--- MUDANÇA: Service
        .create_unit(
            &mut *rls_conn, // A conexão age como executor direto
            tenant.0,
            &payload.name,
            &payload.symbol
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(unit)))
}

// ---
// Handler: get_all_units (Refatorado para RLS)
// ---
pub async fn get_all_units(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let units = app_state
        .inventory_service
        .get_all_units(&mut *rls_conn, tenant.0)
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
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<CreateCategoryPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    // 1. Conexão RLS
    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // 2. Chamada ao Service (Sem begin/commit manual)
    let category = app_state
        .inventory_service // <--- MUDANÇA: Service
        .create_category(
            &mut *rls_conn, // Executor direto
            tenant.0,
            &payload.name,
            payload.description.as_deref(),
            payload.parent_id,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(category)))
}

// ---
// Handler: get_all_categories (Refatorado para RLS)
// ---
pub async fn get_all_categories(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let categories = app_state
        .inventory_service // <--- MUDANÇA: Service
        .get_all_categories(&mut *rls_conn, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(categories)))
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

    // [NOVO] Opcional. Se vier, tenta baixar desse lote. Se não, usa FIFO.
    pub batch_number: Option<String>,

    // [NOVO] Adicione se quiser permitir vender de uma posição específica
    pub position: Option<String>,
}

pub async fn sell_item(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<SellItemPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // Chama o serviço de Venda Direta (sem reserva prévia)
    app_state.inventory_service
        .sell_item(
            &mut *rls_conn,
            tenant.0,
            payload.item_id,
            payload.location_id,
            payload.quantity,
            payload.unit_price,
            false, // consume_reservation = false (Venda Direta)
            Some("Venda via API"),
            payload.batch_number, // Passa o lote (ou None)
            payload.position,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(StatusCode::OK)
}

// --- DTO: Entrada de Estoque ---
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddStockPayload {
    pub location_id: Uuid,
    pub item_id: Uuid,

    #[validate(custom(function = "validate_not_negative"))]
    pub quantity: Decimal,

    #[validate(custom(function = "validate_not_negative"))]
    pub unit_cost: Decimal, // Quanto pagou por unidade (para o Custo Médio)

    pub reason: StockMovementReason, // Ex: "PURCHASE"
    pub notes: Option<String>,

    // [NOVOS CAMPOS DE LOTE]
    // Se for remédio, manda esses dois. Se for roupa, manda null.
    pub batch_number: Option<String>,
    pub expiration_date: Option<NaiveDate>, // Formato YYYY-MM-DD

    pub position: Option<String>,
}

// --- HANDLER ---
pub async fn add_stock(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Json(payload): Json<AddStockPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    // Chama o serviço poderoso que criamos
    let updated_level = app_state.inventory_service
        .add_stock(
            &mut *rls_conn,
            tenant.0,
            payload.item_id,
            payload.location_id,
            payload.quantity,
            payload.unit_cost,
            payload.reason,
            payload.notes.as_deref(),
            // Passa os dados do lote (opcionais)
            payload.batch_number,
            payload.expiration_date,
            payload.position,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    // Retorna o novo saldo total para o frontend atualizar a tela
    Ok((StatusCode::OK, Json(updated_level)))
}