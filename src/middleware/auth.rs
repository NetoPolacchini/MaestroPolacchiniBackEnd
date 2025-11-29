// src/middleware/auth.rs

use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use axum::http::StatusCode;
use axum_extra::{
    extract::TypedHeader,
    headers::{authorization::Bearer, Authorization},
};

use crate::{
    common::error::ApiError,
    config::AppState,
    middleware::{
        i18n::Locale,
        tenancy::TenantContext,
    },
    models::auth::User,
};

// ---
// 1. Middleware LEVE (Apenas Autenticação)
// ---
// Usa-se para rotas que NÃO precisam de tenant (ex: criar um tenant, listar meus tenants)
pub async fn auth_guard(
    State(app_state): State<AppState>,
    locale: Locale,
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {

    let token = auth_header.token();

    // Apenas valida quem é o utilizador
    let user = app_state.auth_service.validate_token(token).await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    // Injeta APENAS o user
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

// ---
// 2. Middleware PESADO (Autenticação + Autorização de Tenant)
// ---
// Usa-se para rotas de negócio (Inventário, Vendas, etc.)
pub async fn tenant_guard(
    State(app_state): State<AppState>,
    locale: Locale,
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    // Este extrator OBRIGA a presença do cabeçalho X-Tenant-ID
    tenant_ctx: TenantContext,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {

    // 1. Valida o token
    let token = auth_header.token();
    let user = app_state.auth_service.validate_token(token).await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    let tenant_id = tenant_ctx.0;

    // 2. Valida se o user pertence ao tenant
    // (Aqui usamos o tenant_repo que criámos anteriormente)
    let has_access = app_state.tenant_repo
        .check_user_tenancy(user.id, tenant_id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    if !has_access {
        // O erro ForbiddenAccess que adicionámos ao error.rs
        return Err(crate::common::error::AppError::ForbiddenAccess.to_api_error(&locale, &app_state.i18n_store));
    }

    // Sucesso: Injeta AMBOS (User e Tenant)
    request.extensions_mut().insert(user);
    request.extensions_mut().insert(tenant_ctx);

    Ok(next.run(request).await)
}

// ---
// Extrator (Permanece igual)
// ---
pub struct AuthenticatedUser(pub User);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<User>()
            .cloned()
            .map(AuthenticatedUser)
            .ok_or(ApiError{
                status: StatusCode::UNAUTHORIZED,
                error: "Authentication token required or invalid.".to_string(),
                details: None
            })
    }
}