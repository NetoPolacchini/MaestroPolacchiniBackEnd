use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

// Representa um usuário vindo do banco de dados
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub hashed_password: String,
    pub created_at: DateTime<Utc>,
}

// Dados para registro de um novo usuário
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterUserPayload {
    #[validate(email(message = "O e-mail fornecido é inválido."))]
    pub email: String,
    #[validate(length(min = 6, message = "A senha deve ter no mínimo 6 caracteres."))]
    pub password: String,
}

// Dados para login
#[derive(Debug, Deserialize, Validate)]
pub struct LoginUserPayload {
    #[validate(email(message = "O e-mail fornecido é inválido."))]
    pub email: String,
    #[validate(length(min = 6, message = "A senha deve ter no mínimo 6 caracteres."))]
    pub password: String,
}

// Resposta de autenticação com o token
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}

// Estrutura de dados ("claims") dentro do JWT
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub sub: Uuid,
}