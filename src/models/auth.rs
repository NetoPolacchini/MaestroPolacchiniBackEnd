// src/models/auth.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// --- ENUM (Compartilhado) ---
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, ToSchema)] // <--- 2. Adicione ToSchema
#[sqlx(type_name = "document_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentType {
    TaxId,
    IdCard,
    Passport,
    DriverLicense,
    Other,
}

// Representa um usuário vindo do banco de dados
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct User {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,
    #[schema(example = "usuario@email.com")]
    pub email: String,

    // Campo que estava faltando
    #[serde(skip_serializing)]
    #[schema(ignore)] // <--- IMPORTANTE: Ignorar na doc do Swagger também
    pub password_hash: String,

    // Novos Campos Globais
    #[schema(example = "BR")]
    pub country_code: String,
    pub document_type: DocumentType,
    #[schema(example = "123.456.789-00")]
    pub document_number: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct UserCompany {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub id: Uuid,
    #[schema(example = "Minha Loja Ltda")]
    pub name: String,
    #[schema(example = "minha-loja")]
    pub slug: String,
}

// Dados para registro de um novo usuário
#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct RegisterUserPayload {
    #[validate(email(message = "O e-mail fornecido é inválido."))]
    #[schema(example = "novo_usuario@email.com")]
    pub email: String,

    #[validate(length(min = 6, message = "A senha deve ter no mínimo 6 caracteres."))]
    #[schema(example = "senhaSegura123", min_length = 6)] 
    pub password: String,

    #[validate(length(equal = 2, message = "O código do país deve ter 2 letras (Ex: BR)."))]
    #[schema(example = "BR", min_length = 2, max_length = 2)]
    pub country_code: Option<String>,

    pub document_type: Option<DocumentType>,

    #[schema(example = "12345678900")]
    pub document_number: Option<String>,
}

// Dados para login
#[derive(Debug, Deserialize, Validate, ToSchema)] // <--- Adicione ToSchema
pub struct LoginUserPayload {
    #[validate(email)]
    #[schema(example = "usuario@email.com")]
    pub email: String,

    #[validate(length(min = 6, code = "password_too_short"))]
    #[schema(example = "senha123")]
    pub password: String,
}

// Resposta de autenticação com o token
#[derive(Debug, Serialize, ToSchema)] // <--- Adicione ToSchema
pub struct AuthResponse {
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub token: String,
}

// Claims não precisa de ToSchema pois é interno do JWT, o Swagger não vê.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    pub iat: usize,
}