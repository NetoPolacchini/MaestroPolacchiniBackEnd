use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

// Nosso tipo de erro customizado para a aplicação
// Ele será usado em todas as camadas (DB, Services, Handlers)
#[derive(Debug)]
pub enum AppError {
    ValidationError(validator::ValidationErrors),
    EmailAlreadyExists,
    InvalidCredentials,
    InternalServerError(String),
    InvalidToken,
    UserNotFound,
}

// Implementação para converter nosso AppError em uma resposta HTTP
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ValidationError(errors) => {
                let first_error = errors.field_errors().into_iter().next().unwrap().1[0].message.as_ref().unwrap();
                (StatusCode::BAD_REQUEST, first_error.to_string())
            }
            AppError::EmailAlreadyExists => (
                StatusCode::CONFLICT,
                "Este e-mail já está em uso.".to_string(),
            ),
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "E-mail ou senha inválidos.".to_string(),
            ),
            AppError::InternalServerError(msg) => {
                tracing::error!("Erro Interno do Servidor: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Ocorreu um erro inesperado.".to_string(),
                )
            }
            AppError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "Token de autenticação inválido ou ausente.".to_string(),
            ),
            AppError::UserNotFound => (
                StatusCode::NOT_FOUND,
                "Usuário não encontrado.".to_string(),
            ),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}