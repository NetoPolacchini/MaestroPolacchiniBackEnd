// src/common/error.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize; // Importante para serializar a resposta customizada
use thiserror::Error;
use std::collections::HashMap; // Necessário para o mapa de erros
use crate::config::I18nStore;
use crate::middleware::i18n::Locale;

// Nosso tipo de erro principal (Enum do Backend)
#[derive(Debug, Error)]
pub enum AppError {



    #[error("Erro de validação")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("Erros de validação nos campos personalizados")]
    CustomDataValidationError(HashMap<String, String>),

    #[error("E-mail já existe")]
    EmailAlreadyExists,

    #[error("Custom data must be a JSON object")]
    CustomDataJson,

    #[error("Credenciais inválidas")]
    InvalidCredentials,

    #[error("Token inválido")]
    InvalidToken,

    #[error("Usuário não encontrado")]
    UserNotFound,

    #[error("SKU já existe")]
    SkuAlreadyExists,
    
    #[error("Documento já está cadastrado")]
    DocumentAlreadyExists,

    #[error("Pool '{0}' já existe")]
    PoolAlreadyExists(String),

    #[error("Location '{0}' já existe")]
    LocationAlreadyExists(String),

    #[error("O nome da unidade já existe: {0}")]
    UnitNameAlreadyExists(String),

    #[error("O símbolo da unidade já existe: {0}")]
    UnitSymbolAlreadyExists(String),

    #[error("Violação de restrição única: {0}")]
    UniqueConstraintViolation(String),

    #[error("Não foi encontrado recursos")]
    ResourceNotFound(String),

    #[error("Uma categoria com este nome já existe (neste nível): {0}")]
    CategoryNameAlreadyExists(String),

    #[error("Você já possui um estabelecimento com o nome: {0}")]
    TenantNameAlreadyExists(String),

    #[error("O usuário já é membro desta equipe")]
    MemberAlreadyExists,

    #[error("O cargo informado '{0}' não existe")]
    RoleDoesNotExist(String),

    #[error("O cargo informado '{0}' existe")]
    RoleAlreadyExist(String),

    // Erros técnicos (wrappers)
    #[error("Erro de banco de dados")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Erro interno do servidor")]
    InternalServerError(#[from] anyhow::Error),

    #[error("Erro de Bcrypt: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),

    #[error("Erro de JWT: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Acesso Negado")]
    ForbiddenAccess,

    #[error("Já existe um campo com esta chave")]
    CrmFieldKeyAlreadyExists(String), // Recebe a chave duplicada

    #[error("Já existe um tipo com este nome")]
    CrmEntityTypeAlreadyExists(String),

    #[error("Cliente com documento duplicado")]
    CustomerDocumentAlreadyExists(String), // Recebe o número do doc

}

// --- Estrutura de Resposta da API (JSON) ---
#[derive(Serialize)]
pub struct ApiError {
    #[serde(skip)] // Não queremos o status code numérico dentro do JSON, ele vai no Header HTTP
    pub status: StatusCode,

    pub error: String, // Mensagem amigável

    // Só aparece no JSON se tiver conteúdo (Ex: erros de validação)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, Vec<String>>>,
}

// Transforma nossa struct ApiError numa Resposta HTTP do Axum
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        // O axum::Json serializa automaticamente a struct graças ao #[derive(Serialize)]
        (status, Json(self)).into_response()
    }
}

// --- Helper para simplificar erros do Validator ---
// Transforma: ValidationErrors { field: "sku", kind: "length", ... }
// Em: { "sku": ["O tamanho deve ser maior que 3"] }
fn transform_validation_errors<F>(
    err: validator::ValidationErrors,
    resolve_key: F // <--- O "Tradutor" entra aqui
) -> HashMap<String, Vec<String>>
where
    F: Fn(&str) -> String // Diz que 'resolve_key' é uma função que troca String por String
{
    let mut map = HashMap::new();

    for (field, error_kind) in err.field_errors() {
        let messages: Vec<String> = error_kind
            .iter()
            .map(|e| {
                // LÓGICA DE TRADUÇÃO INTELIGENTE:
                // 1. Se o erro já tem mensagem (hardcoded), usa ela.
                // 2. Se não, pega o código (ex: "LocationRequiredForStock") e pede para traduzir.
                e.message
                    .as_ref()
                    .map(|cow| cow.to_string())
                    .unwrap_or_else(|| resolve_key(&e.code)) // <--- Aqui acontece a mágica!
            })
            .collect();

        map.insert(field.to_string(), messages);
    }

    map
}

// --- Lógica de Tradução e Conversão ---
impl AppError {
    pub fn to_api_error(self, locale: &Locale, i18n: &I18nStore) -> ApiError {
        let lang_code = &locale.0;

        // 1. Helper de Tradução (Busca no JSON ou usa Fallback)
        let get_template = |key: &str| {
            i18n.get(lang_code)
                .and_then(|translations| translations.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    // Tenta inglês ou devolve a chave bruta
                    i18n.get("en")
                        .and_then(|t| t.get(key))
                        .cloned()
                        .unwrap_or_else(|| key.to_string())
                })
        };

        // 2. Logging Inteligente (Antes de responder, registramos o erro no terminal)
        match &self {
            // Avisos (Cliente errou algo) - Amarelo/Warn
            AppError::ValidationError(e) => tracing::warn!("⚠️ Validação falhou: {:?}", e),
            AppError::InvalidCredentials
            | AppError::EmailAlreadyExists
            | AppError::SkuAlreadyExists
            | AppError::DocumentAlreadyExists => tracing::warn!("⚠️ Regra de negócio: {}", self),

            // Erros Críticos (Servidor quebrou) - Vermelho/Error
            AppError::DatabaseError(e) => tracing::error!("🔥 ERRO DE BANCO: {:?}", e),
            AppError::InternalServerError(e) => tracing::error!("🔥 ERRO INTERNO: {:?}", e),

            // Outros
            _ => tracing::info!("ℹ️ Erro API: {}", self),
        }

        // 3. Mapeamento de Status, Mensagem e Detalhes
        let (status, message, details) = match self {

            // Caso especial: Validação (retorna detalhes)
            AppError::ValidationError(errs) => {
                let details_map = transform_validation_errors(errs, &get_template);
                (
                    StatusCode::BAD_REQUEST,
                    get_template("ValidationError"),
                    Some(details_map) // <--- Aqui preenchemos o details!
                )
            },

            // Erros Estáticos
            AppError::EmailAlreadyExists => (StatusCode::CONFLICT, get_template("EmailAlreadyExists"), None),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, get_template("InvalidCredentials"), None),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, get_template("InvalidToken"), None),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, get_template("UserNotFound"), None),
            AppError::SkuAlreadyExists => (StatusCode::CONFLICT, get_template("SkuAlreadyExists"), None),
            AppError::DocumentAlreadyExists => (StatusCode::CONFLICT, get_template("DocumentAlreadyExists"), None),
            AppError::ForbiddenAccess => (StatusCode::FORBIDDEN, get_template("ForbiddenAccess"), None),
            AppError::MemberAlreadyExists => (StatusCode::CONFLICT, get_template("MemberAlreadyExists"), None),
            AppError::CustomDataJson => (StatusCode::CONFLICT, get_template("CustomDataJson"), None),

            // Erros Dinâmicos (com replace)
            AppError::UnitNameAlreadyExists(name) => {
                let t = get_template("UnitNameAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &name), None)
            }
            AppError::UnitSymbolAlreadyExists(symbol) => {
                let t = get_template("UnitSymbolAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &symbol), None)
            }
            AppError::CategoryNameAlreadyExists(name) => {
                let t = get_template("CategoryNameAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &name), None)
            }
            AppError::TenantNameAlreadyExists(name) => {
                let t = get_template("TenantNameAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &name), None)
            }
            AppError::UniqueConstraintViolation(val) => {
                let t = get_template("UniqueConstraintViolation");
                (StatusCode::CONFLICT, t.replace("{value}", &val), None)
            }
            AppError::ResourceNotFound(val) => {
                let t = get_template("ResourceNotFound");
                (StatusCode::CONFLICT, t.replace("{value}", &val), None)
            }
            AppError::RoleDoesNotExist(val) => {
                let t = get_template("RoleDoesNotExist");
                (StatusCode::CONFLICT, t.replace("{value}", &val), None)
            }
            AppError::RoleAlreadyExist(val) => {
                let t = get_template("RoleAlreadyExist");
                (StatusCode::CONFLICT, t.replace("{value}", &val), None)
            }
            AppError::PoolAlreadyExists(val) => {
                let t = get_template("PoolAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &val), None)
            }
            AppError::LocationAlreadyExists(val) => {
                let t = get_template("LocationAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &val), None)
            }
            AppError::CrmFieldKeyAlreadyExists(key) => {
                let t = get_template("CrmFieldKeyAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &key), None)
            }
            AppError::CrmEntityTypeAlreadyExists(key) => {
                let t = get_template("CrmEntityTypeAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &key), None)
            }
            AppError::CustomerDocumentAlreadyExists(doc) => {
                let t = get_template("CustomerDocumentAlreadyExists");
                (StatusCode::CONFLICT, t.replace("{value}", &doc), None)
            }

            // Erros Internos (escondemos os detalhes técnicos do usuário)
            _ => (StatusCode::INTERNAL_SERVER_ERROR, get_template("InternalServerError"), None),
        };

        // 4. Retorna a struct pronta
        ApiError {
            status,
            error: message,
            details,
        }
    }
}