// src/handlers/crm.rs

use axum::{
    extract::{State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use validator::Validate;
use chrono::NaiveDate;
use uuid::Uuid; // <--- Não esqueça de importar Uuid

use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{tenancy::TenantContext, i18n::Locale},
    // Importamos os Enums e Structs necessários
    models::crm::{FieldType},
};
use crate::models::auth::DocumentType;

// =============================================================================
//  ÁREA 1: TIPOS DE ENTIDADE (NOVO)
//  Ex: Criar "Paciente", "Aluno", "Veículo"
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntityTypePayload {
    #[validate(length(min = 2, message = "O nome deve ter no mínimo 2 caracteres"))]
    pub name: String, // Ex: "Paciente"

    #[validate(length(min = 2, message = "O slug deve ter no mínimo 2 caracteres"))]
    pub slug: String, // Ex: "paciente"
}

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
//  Atualizado para aceitar 'entityTypeId'
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateFieldPayload {
    // [NOVO] O campo pertence a um tipo? (Ex: "Convênio" pertence a "Paciente")
    // Se nulo, é um campo Global.
    pub entity_type_id: Option<Uuid>,

    #[validate(length(min = 1, message = "required"))]
    pub name: String,

    #[validate(length(min = 1, message = "required"))]
    pub key_name: String,

    pub field_type: FieldType, // Enum FieldType

    pub options: Option<Value>,

    #[serde(default)]
    pub is_required: bool,
}

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
            payload.entity_type_id, // Passando o novo argumento
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
//  Atualizado para aceitar 'entityTypes' (Ex: "Esse cliente é Paciente e Aluno")
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerPayload {
    #[validate(length(min = 1, message = "required"))]
    pub full_name: String,

    #[validate(length(equal = 2, message = "invalid_country_code"))]
    pub country_code: Option<String>,

    pub document_type: Option<DocumentType>,
    pub document_number: Option<String>,

    pub birth_date: Option<NaiveDate>,

    #[validate(email(message = "invalid_email"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,

    pub address: Option<Value>,
    pub tags: Option<Vec<String>>,

    // [NOVO] Array de IDs dos tipos (Ex: [ID_PACIENTE])
    pub entity_types: Option<Vec<Uuid>>,

    #[serde(default)]
    pub custom_data: Value,
}

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
            // [NOVO] Passando os tipos para ativar a validação dinâmica
            payload.entity_types,
            payload.custom_data
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::CREATED, Json(customer)))
}

// Listagem simples
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