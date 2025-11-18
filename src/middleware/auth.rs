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
    common::error::{ApiError, AppError}, // <-- Importe o AppError
    config::AppState,
    middleware::{
        i18n::Locale,
        tenancy::TenantContext, // <-- NOVO: Importa o extrator de Tenant
    },
    models::auth::User,
};

/// O middleware de segurança principal.
/// Agora, ele faz DUAS coisas:
/// 1. Autenticação (O token é válido?)
/// 2. Autorização (Este utilizador pode aceder a este tenant?)
pub async fn auth_middleware(
    State(app_state): State<AppState>,
    locale: Locale, // Para tradução de erros

    // --- Novos Extratores de Segurança ---
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>, // 1. O Token
    tenant_ctx: TenantContext, // 2. O Cabeçalho X-Tenant-ID

    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {

    // --- 1. Autenticação (Quem é você?) ---
    let token = auth_header.token();
    let user = app_state.auth_service.validate_token(token)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    let tenant_id = tenant_ctx.0; // O UUID do X-Tenant-ID

    // --- 2. Autorização (Você pode estar aqui?) ---
    let has_access = app_state.tenant_repo
        .check_user_tenancy(user.id, tenant_id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    if !has_access {
        // Se a verificação falhar, retorne 403 Forbidden.
        // O utilizador está autenticado, mas não autorizado para ESTE tenant.
        return Err(AppError::ForbiddenAccess.to_api_error(&locale, &app_state.i18n_store));
    }

    // --- Sucesso ---
    // Injeta o Utilizador E o Tenant nos "extensions" da requisição.
    // Os handlers downstream (como create_item) podem agora confiar
    // que este tenant_id foi verificado.
    request.extensions_mut().insert(user);
    request.extensions_mut().insert(tenant_ctx); // Passa o TenantContext

    Ok(next.run(request).await)
}

// O extrator AuthenticatedUser (para obter o User)
pub struct AuthenticatedUser(pub User);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    // Esta função agora LÊ dos extensions que o middleware injetou
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
                // Esta mensagem só aparece se o auth_middleware falhar (o que não deve acontecer)
                message: "Falha ao extrair o utilizador do contexto.".to_string(),
            })
    }
}