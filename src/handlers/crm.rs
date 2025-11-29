// src/handlers/crm.rs

use axum::{
    extract::{State, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
    Extension,
};
use serde::Deserialize;
use serde_json::Value;
use validator::Validate;
use chrono::NaiveDate;

use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{tenancy::TenantContext, i18n::Locale},
    models::crm::{CrmFieldDefinition, CrmFieldType, Customer},
};

// =============================================================================
//  ÁREA 1: CONFIGURAÇÃO (DEFINIÇÕES DE CAMPO)
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateFieldPayload {
    #[validate(length(min = 1, message = "required"))]
    pub name: String,     // Ex: "Peso"

    #[validate(length(min = 1, message = "required"))]
    pub key_name: String, // Ex: "weight"

    pub field_type: CrmFieldType, // TEXT, NUMBER, etc.

    pub options: Option<Value>,   // Ex: ["A", "B"] (para Selects)

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

    let field = app_state.crm_repo // Podemos chamar o repo direto aqui, ou criar um método no service
        .create_field_definition(
            &app_state.db_pool,
            tenant.0,
            &payload.name,
            &payload.key_name,
            payload.field_type,
            payload.options.as_ref(),
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

    let fields = app_state.crm_repo
        .list_field_definitions(&app_state.db_pool, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(fields)))
}

// =============================================================================
//  ÁREA 2: OPERAÇÃO (CLIENTES)
// =============================================================================

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerPayload {
    #[validate(length(min = 1, message = "required"))]
    pub full_name: String,

    pub document_number: Option<String>,
    pub birth_date: Option<NaiveDate>,

    #[validate(email(message = "invalid_email"))]
    pub email: Option<String>,

    pub phone: Option<String>,
    pub mobile: Option<String>,

    pub address: Option<Value>,
    pub tags: Option<Vec<String>>,

    // O JSONMágico!
    #[serde(default)] // Se não vier, assume null/vazio
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

    // Aqui chamamos o SERVICE, não o Repo.
    // O Service é quem vai validar se o 'customData' bate com as definições.
    let customer = app_state.crm_service // Você precisará adicionar crm_service no AppState se ainda não fez!
        .create_customer(
            &app_state.db_pool,
            tenant.0,
            &payload.full_name,
            payload.document_number.as_deref(),
            payload.birth_date,
            payload.email.as_deref(),
            payload.phone.as_deref(),
            payload.mobile.as_deref(),
            payload.address,
            payload.tags,
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

    let customers = app_state.crm_repo
        .list_customers(&app_state.db_pool, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(customers)))
}