//src/common/error.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use crate::config::I18nStore;
use crate::middleware::i18n::Locale;

// Nosso tipo de erro, agora com `thiserror` para melhor ergonomia.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Erro de validação")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("E-mail já existe")]
    EmailAlreadyExists,

    #[error("Credenciais inválidas")]
    InvalidCredentials,

    #[error("Token inválido")]
    InvalidToken,

    #[error("Usuário não encontrado")]
    UserNotFound,

    #[error("SKU já existe")]
    SkuAlreadyExists,

    // --- NOSSAS NOVAS VARIANTES ---
    #[error("O nome da unidade já existe: {0}")]
    UnitNameAlreadyExists(String), // Armazena o 'name'

    #[error("O símbolo da unidade já existe: {0}")]
    UnitSymbolAlreadyExists(String), // Armazena o 'symbol'

    #[error("Violação de restrição única: {0}")]
    UniqueConstraintViolation(String),

    // --- NOVA LINHA ---
    #[error("Uma categoria com este nome já existe (neste nível): {0}")]
    CategoryNameAlreadyExists(String),


    // Variante para erros de banco de dados (exemplo com sqlx)
    #[error("Erro de banco de dados")]
    DatabaseError(#[from] sqlx::Error),
    
    // Variante genérica para qualquer outro erro inesperado
    // `anyhow::Error` é ótimo para capturar o contexto do erro.
    #[error("Erro interno do servidor")]
    InternalServerError(#[from] anyhow::Error),

    #[error("Erro de Bcrypt: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),

    #[error("Erro de JWT: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Acesso Negado")]
    ForbiddenAccess,
    
}

pub struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

// src/common/error.rs
// ... (imports)

// ... (struct ApiError e impl IntoResponse) ...

impl AppError {
    pub fn to_api_error(self, locale: &Locale, i18n: &I18nStore) -> ApiError {
        let lang_code = &locale.0;

        // --- Helper Interno ---
        // Esta função pega o template (ex: "O nome é '{value}'")
        let get_template = |key: &str| {
            i18n.get(lang_code)
                .and_then(|translations| translations.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    // Fallback para "en" se o idioma ou a chave não existirem
                    i18n.get("en")
                        .and_then(|t| t.get(key))
                        .cloned()
                        .unwrap_or_else(|| key.to_string())
                })
        };
        // --- Fim do Helper ---

        // MUDANÇA: O match agora lida com os placeholders
        let (status, message) = match self {
            // --- Erros Estáticos (sem variáveis) ---
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, get_template("ValidationError")),
            AppError::EmailAlreadyExists => (StatusCode::CONFLICT, get_template("EmailAlreadyExists")),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, get_template("InvalidCredentials")),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, get_template("InvalidToken")),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, get_template("UserNotFound")),
            AppError::SkuAlreadyExists => (StatusCode::CONFLICT, get_template("SkuAlreadyExists")),
            AppError::ForbiddenAccess => (StatusCode::FORBIDDEN, get_template("ForbiddenAccess")),

            // --- Erros Dinâmicos (com variáveis) ---
            AppError::UnitNameAlreadyExists(name) => {
                let template = get_template("UnitNameAlreadyExists");
                // Aqui fazemos a substituição!
                (StatusCode::CONFLICT, template.replace("{value}", &name))
            }
            AppError::UnitSymbolAlreadyExists(symbol) => {
                let template = get_template("UnitSymbolAlreadyExists");
                // Aqui fazemos a substituição!
                (StatusCode::CONFLICT, template.replace("{value}", &symbol))
            }
            AppError::UniqueConstraintViolation(constraint_name) => {
                let template = get_template("UniqueConstraintViolation");
                (StatusCode::CONFLICT, template.replace("{value}", &constraint_name)) // Mantemos o {0} para o fallback
            }

            AppError::CategoryNameAlreadyExists(name) => {
                let template = get_template("CategoryNameAlreadyExists");
                (StatusCode::CONFLICT, template.replace("{value}", &name))
            }

            // --- Erros Internos ---
            ref e => {
                tracing::error!("Erro Interno do Servidor: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, get_template("InternalServerError"))
            }
        };

        ApiError { status, message }
    }
}