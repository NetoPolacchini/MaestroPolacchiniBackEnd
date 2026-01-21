// src/handlers/auth.rs

use axum::{extract::State, Json};
use validator::Validate;

use crate::middleware::i18n::Locale;
use crate::common::error::ApiError;

use crate::{
    common::error::AppError,
    config::AppState,
    middleware::auth::AuthenticatedUser,
    models::auth::{AuthResponse, LoginUserPayload, RegisterUserPayload, User, UserCompany},
};

// -----------------------------------------------------------------------------
// POST /api/auth/register
// -----------------------------------------------------------------------------
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterUserPayload,
    responses(
        (status = 200, description = "Usuário registrado com sucesso", body = AuthResponse),
        (status = 400, description = "Dados inválidos (Email mal formatado, senha curta)"),
        (status = 409, description = "Conflito: E-mail já existe")
    )
)]
pub async fn register(
    State(app_state): State<AppState>,
    locale: Locale,
    Json(payload): Json<RegisterUserPayload>,
) -> Result<Json<AuthResponse>, ApiError> {
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let token = app_state
        .auth_service
        .register_user(
            &payload.email,
            &payload.password,
            payload.country_code,
            payload.document_type,
            payload.document_number
        )
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(Json(AuthResponse { token }))
}

// -----------------------------------------------------------------------------
// POST /api/auth/login
// -----------------------------------------------------------------------------
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginUserPayload,
    responses(
        (status = 200, description = "Login realizado com sucesso", body = AuthResponse),
        (status = 400, description = "Dados inválidos"),
        (status = 401, description = "Credenciais inválidas (Email ou senha incorretos)")
    )
)]
pub async fn login(
    State(app_state): State<AppState>,
    locale: Locale,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<AuthResponse>, ApiError> {
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e).to_api_error(&locale, &app_state.i18n_store))?;

    let token = app_state
        .auth_service
        .login_user(&payload.email, &payload.password)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(Json(AuthResponse { token }))
}

// -----------------------------------------------------------------------------
// GET /api/users/me
// -----------------------------------------------------------------------------
#[utoipa::path(
    get,
    path = "/api/users/me",
    tag = "Users",
    responses(
        (status = 200, description = "Dados do usuário logado", body = User),
        (status = 401, description = "Não autorizado (Token inválido ou ausente)")
    ),
    security(
        ("api_jwt" = []) // Requer apenas o Token, sem Tenant ID
    )
)]
pub async fn get_me(AuthenticatedUser(user): AuthenticatedUser) -> Json<User> {
    Json(user)
}

// -----------------------------------------------------------------------------
// GET /api/users/me/companies
// -----------------------------------------------------------------------------
#[utoipa::path(
    get,
    path = "/api/users/me/companies",
    tag = "Users",
    responses(
        (status = 200, description = "Lista de empresas vinculadas ao usuário", body = Vec<UserCompany>),
        (status = 401, description = "Não autorizado")
    ),
    security(
        ("api_jwt" = []) // Requer apenas o Token
    )
)]
pub async fn get_my_companies(
    State(app_state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    locale: Locale,
) -> Result<Json<Vec<UserCompany>>, ApiError> {

    let companies = app_state.crm_service
        .find_companies_by_user(&app_state.db_pool, user.id)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    Ok(Json(companies))
}