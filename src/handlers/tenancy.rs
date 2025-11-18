// src/handlers/tenancy.rs

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use validator::Validate;

// Importa os nossos extratores e erros
use crate::{
    common::error::{ApiError, AppError},
    config::AppState,
    middleware::{
        auth::AuthenticatedUser, // O extrator de Utilizador
        i18n::Locale,            // O extrator de Idioma
    },
    // O modelo que este handler irá retornar
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
    // Precisamos do utilizador autenticado para o podermos tornar "dono"
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