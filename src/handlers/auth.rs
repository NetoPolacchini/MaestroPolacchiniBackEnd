use axum::{extract::State, Json};
use validator::Validate;

use crate::{
    common::error::AppError,
    config::AppState,
    middleware::auth::AuthenticatedUser,
    models::auth::{AuthResponse, LoginUserPayload, RegisterUserPayload, User},
    services::auth::AuthService,
};

// Handler de registro
pub async fn register(
    State(app_state): State<AppState>,
    Json(payload): Json<RegisterUserPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    payload.validate().map_err(AppError::ValidationError)?;
    
    let auth_service = AuthService::new(app_state);
    let token = auth_service.register_user(&payload.email, &payload.password).await?;

    Ok(Json(AuthResponse { token }))
}

// Handler de login
pub async fn login(
    State(app_state): State<AppState>,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    payload.validate().map_err(AppError::ValidationError)?;
    
    let auth_service = AuthService::new(app_state);
    let token = auth_service.login_user(&payload.email, &payload.password).await?;
    
    Ok(Json(AuthResponse { token }))
}

// Handler da rota protegida /me
pub async fn get_me(AuthenticatedUser(user): AuthenticatedUser) -> Json<User> {
    Json(user)
}