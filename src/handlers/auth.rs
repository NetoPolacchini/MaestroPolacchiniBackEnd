// src/handlers/login

use axum::{extract::State, Json};
use validator::Validate;

use crate::middleware::i18n::Locale; // <-- Importe o Locale
use crate::common::error::ApiError; // <-- Importe o novo ApiError

use crate::{
    common::error::AppError,
    config::AppState,
    middleware::auth::AuthenticatedUser,
    models::auth::{AuthResponse, LoginUserPayload, RegisterUserPayload, User},
};

// Handler de registro
pub async fn register(
    State(app_state): State<AppState>, // O AppState já contém o serviço
    locale: Locale, // <-- 1. Adicione o extrator de idioma
    Json(payload): Json<RegisterUserPayload>,
) -> Result<Json<AuthResponse>, ApiError> {
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;


    // REMOVEMOS: let auth_service = AuthService::new(app_state);
    // USAMOS DIRETAMENTE O SERVIÇO DO ESTADO:
    let token = app_state
        .auth_service
        .register_user(&payload.email, &payload.password)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(Json(AuthResponse { token }))
}

// Handler de login
pub async fn login(
    State(app_state): State<AppState>, // O AppState já contém o serviço
    locale: Locale,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<AuthResponse>, ApiError> {
    payload
        .validate()
        //.map_err(ApiError::ValidationError)?;
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;
    
    // REMOVEMOS: let auth_service = AuthService::new(app_state);
    // USAMOS DIRETAMENTE O SERVIÇO DO ESTADO:
    let token = app_state
        .auth_service
        .login_user(&payload.email, &payload.password)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;
    
    Ok(Json(AuthResponse { token }))
}

// Handler da rota protegida /me
// (Este já estava correto e não precisa de mudanças)
pub async fn get_me(AuthenticatedUser(user): AuthenticatedUser) -> Json<User> {
    Json(user)
}