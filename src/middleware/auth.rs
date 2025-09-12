use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, HeaderMap},
    middleware::Next,
    response::Response,
};

use crate::{
    common::error::AppError,
    config::AppState,
    models::auth::User,
    services::auth::AuthService,
};

// O middleware em si
pub async fn auth_middleware(
    State(app_state): State<AppState>,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let headers = request.headers();
    let auth_header = headers.get("Authorization").and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let auth_service = AuthService::new(app_state);
            let user = auth_service.validate_token(token).await?;
            
            // Insere o usuário nos "extensions" da requisição
            request.extensions_mut().insert(user);
            return Ok(next.run(request).await);
        }
    }

    Err(AppError::InvalidToken)
}

// Extrator para obter o usuário autenticado diretamente nos handlers
pub struct AuthenticatedUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<User>()
            .cloned()
            .map(AuthenticatedUser)
            .ok_or(AppError::InvalidToken)
    }
}