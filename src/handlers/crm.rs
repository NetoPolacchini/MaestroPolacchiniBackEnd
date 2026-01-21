// src/handlers/crm.rs

use axum::{
    extract::{State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use validator::Validate;
use chrono::NaiveDate;
use uuid::Uuid;
use utoipa::ToSchema; 

use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{tenancy::TenantContext, i18n::Locale},
    models::crm::{FieldType, EntityType, FieldDefinition, Customer}, // Importe os models de resposta
};
use crate::models::auth::DocumentType;

// =============================================================================
//  ÁREA 1: TIPOS DE ENTIDADE
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreateEntityTypePayload {
    #[validate(length(min = 2, message = "O nome deve ter no mínimo 2 caracteres"))]
    #[schema(example = "Paciente")]
    pub name: String,

    #[validate(length(min = 2, message = "O slug deve ter no mínimo 2 caracteres"))]
    #[schema(example = "paciente")]
    pub slug: String,
}

// POST /api/crm/types
#[utoipa::path(
    post,
    path = "/api/crm/types",
    tag = "CRM",
    request_body = CreateEntityTypePayload,
    responses(
        (status = 201, description = "Tipo de Entidade criado", body = EntityType),
        (status = 400, description = "Dados inválidos")
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn create_entity_type(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    Json(payload): Json<CreateEntityTypePayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let entity_type = app_state.crm_service
        .create_entity_type(
            &app_state.db_pool,
            tenant.0,
            &payload.name,
            &payload.slug
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(entity_type)))
}

// GET /api/crm/types
#[utoipa::path(
    get,
    path = "/api/crm/types",
    tag = "CRM",
    responses(
        (status = 200, description = "Lista de Tipos de Entidade", body = Vec<EntityType>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn list_entity_types(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let types = app_state.crm_service
        .list_entity_types(&app_state.db_pool, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(types)))
}

// =============================================================================
//  ÁREA 2: CONFIGURAÇÃO (DEFINIÇÕES DE CAMPO)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreateFieldPayload {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub entity_type_id: Option<Uuid>,

    #[validate(length(min = 1, message = "required"))]
    #[schema(example = "Tamanho da Camiseta")]
    pub name: String,

    #[validate(length(min = 1, message = "required"))]
    #[schema(example = "tamanho_camiseta")]
    pub key_name: String,

    #[schema(example = "Select")]
    pub field_type: FieldType,

    #[schema(example = json!(["P", "M", "G"]))]
    pub options: Option<Value>,

    #[serde(default)]
    #[schema(example = true)]
    pub is_required: bool,
}

// POST /api/crm/fields
#[utoipa::path(
    post,
    path = "/api/crm/fields",
    tag = "CRM",
    request_body = CreateFieldPayload,
    responses(
        (status = 201, description = "Campo customizado criado", body = FieldDefinition)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn create_field_definition(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    Json(payload): Json<CreateFieldPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let field = app_state.crm_service
        .create_field_definition(
            &app_state.db_pool,
            tenant.0,
            payload.entity_type_id,
            &payload.name,
            &payload.key_name,
            payload.field_type,
            payload.options,
            payload.is_required,
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(field)))
}

// GET /api/crm/fields
#[utoipa::path(
    get,
    path = "/api/crm/fields",
    tag = "CRM",
    responses(
        (status = 200, description = "Lista de campos customizados", body = Vec<FieldDefinition>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn list_field_definitions(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let fields = app_state.crm_service
        .list_field_definitions(&app_state.db_pool, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(fields)))
}

// =============================================================================
//  ÁREA 3: OPERAÇÃO (CLIENTES)
// =============================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerPayload {
    #[validate(length(min = 1, message = "required"))]
    #[schema(example = "Maria da Silva")]
    pub full_name: String,

    #[validate(length(equal = 2, message = "invalid_country_code"))]
    #[schema(example = "BR")]
    pub country_code: Option<String>,

    pub document_type: Option<DocumentType>,
    #[schema(example = "12345678900")]
    pub document_number: Option<String>,

    #[schema(value_type = Option<String>, format = Date, example = "1990-05-20")]
    pub birth_date: Option<NaiveDate>,

    #[validate(email(message = "invalid_email"))]
    #[schema(example = "maria@email.com")]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,

    pub address: Option<Value>,
    #[schema(example = json!(["vip", "2024"]))]
    pub tags: Option<Vec<String>>,

    #[schema(example = json!(["550e8400-e29b-41d4-a716-446655440000"]))]
    pub entity_types: Option<Vec<Uuid>>,

    #[serde(default)]
    #[schema(example = json!({"tamanho_camiseta": "P"}))]
    pub custom_data: Value,
}

// POST /api/crm/customers
#[utoipa::path(
    post,
    path = "/api/crm/customers",
    tag = "CRM",
    request_body = CreateCustomerPayload,
    responses(
        (status = 201, description = "Cliente criado", body = Customer),
        (status = 400, description = "Dados inválidos")
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn create_customer(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
    Json(payload): Json<CreateCustomerPayload>,
) -> Result<impl IntoResponse, ApiError> {

    payload.validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let customer = app_state.crm_service
        .create_customer(
            &app_state.db_pool,
            tenant.0,
            &payload.full_name,
            payload.country_code.as_deref(),
            payload.document_type,
            payload.document_number.as_deref(),
            payload.birth_date,
            payload.email.as_deref(),
            payload.phone.as_deref(),
            payload.mobile.as_deref(),
            payload.address,
            payload.tags,
            payload.entity_types,
            payload.custom_data
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(customer)))
}

// GET /api/crm/customers
#[utoipa::path(
    get,
    path = "/api/crm/customers",
    tag = "CRM",
    responses(
        (status = 200, description = "Lista de clientes", body = Vec<Customer>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(("api_jwt" = []))
)]
pub async fn list_customers(
    State(app_state): State<AppState>,
    locale: Locale,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let customers = app_state.crm_service
        .list_customers(&app_state.db_pool, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(customers)))
}