// src/middleware/auth.rs

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};

use axum_extra::{
    extract::TypedHeader,
    headers::{authorization::Bearer, Authorization},
};

use crate::{common::error::AppError, config::AppState, models::auth::User};

// O middleware, agora usando o serviço compartilhado e TypedHeader
pub async fn auth_middleware(
    State(app_state): State<AppState>,
    // Extrai o header "Authorization: Bearer <token>" automaticamente.
    // Se o header não existir ou estiver mal formatado, o Axum já rejeita a requisição.
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    // A lógica manual de extrair o header e o prefixo "Bearer " foi removida!
    let token = auth_header.token();
    
    // REMOVEMOS: let auth_service = AuthService::new(app_state);
    // USAMOS DIRETAMENTE O SERVIÇO DO ESTADO:
    let user = app_state.auth_service.validate_token(token).await?;
    
    // Insere o usuário nos "extensions" da requisição
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}

// O extrator AuthenticatedUser já estava perfeito e não precisa de mudanças.
pub struct AuthenticatedUser(pub User);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<User>()
            .cloned()
            .map(AuthenticatedUser)
            .ok_or(AppError::InvalidToken)
    }
}