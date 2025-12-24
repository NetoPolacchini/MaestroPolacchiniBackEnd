// src/models/login

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;


// --- ENUM (Compartilhado) ---
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "document_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")] // JSON usa "TAX_ID"
pub enum DocumentType {
    TaxId,
    IdCard,
    Passport,
    DriverLicense,
    Other,
}

// Representa um usuário vindo do banco de dados
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    
    // Campo que estava faltando
    #[serde(skip_serializing)] // IMPORTANTE para segurança
    pub password_hash: String,

    // Novos Campos Globais
    pub country_code: String,
    pub document_type: DocumentType,
    pub document_number: Option<String>,

    // Campos de data/hora que estavam faltando
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Dados para registro de um novo usuário
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")] // Importante para o JSON ser countryCode e não country_code
pub struct RegisterUserPayload {
    #[validate(email(message = "O e-mail fornecido é inválido."))]
    pub email: String,

    #[validate(length(min = 6, message = "A senha deve ter no mínimo 6 caracteres."))]
    pub password: String,

    // [NOVOS CAMPOS]
    #[validate(length(equal = 2, message = "O código do país deve ter 2 letras (Ex: BR)."))]
    pub country_code: Option<String>,

    pub document_type: Option<DocumentType>,

    // Aqui você pode adicionar validação de CPF futuramente se quiser
    pub document_number: Option<String>,
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
    pub sub: Uuid,  // Subject (ID do usuário)
    pub exp: usize, // Expiration time (quando o token expira)
    pub iat: usize, // Issued At (quando o token foi criado)
}