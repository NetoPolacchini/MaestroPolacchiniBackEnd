use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

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
    
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            // Sugestão B: Retornar todos os detalhes da validação.
            AppError::ValidationError(errors) => {
                let mut details = std::collections::HashMap::new();
                for (field, field_errors) in errors.field_errors() {
                    let messages: Vec<String> = field_errors.iter()
                        .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                        .collect();
                    details.insert(field.to_string(), messages);
                }
                let body = Json(json!({
                    "error": "Um ou mais campos são inválidos.",
                    "details": details,
                }));
                return (StatusCode::BAD_REQUEST, body).into_response();
            }
            AppError::EmailAlreadyExists => (StatusCode::CONFLICT, "Este e-mail já está em uso."),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "E-mail ou senha inválidos."),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Token de autenticação inválido ou ausente."),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "Usuário não encontrado."),
            
            // Todos os outros erros (DatabaseError, InternalServerError) viram 500.
            // O `#[from]` cuidou da conversão, agora só precisamos tratar o que fazer com eles.
            // O `tracing` vai logar a mensagem detalhada que `thiserror` nos deu.
            ref e => {
                tracing::error!("Erro Interno do Servidor: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Ocorreu um erro inesperado.")
            }
        };

        // Resposta padrão para erros simples que só têm uma mensagem.
        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}