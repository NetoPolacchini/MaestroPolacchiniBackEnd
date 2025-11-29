// src/middleware/tenancy.rs

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;
use crate::common::error::ApiError; // Usamos o nosso ApiError para rejeição

// O nome do nosso cabeçalho HTTP customizado
const TENANT_ID_HEADER: &str = "x-tenant-id";

// O nosso novo extrator.
// Ele armazena o UUID do tenant que o utilizador quer aceder.
#[derive(Debug, Clone)]
pub struct TenantContext(pub Uuid);

// Sempre que uma função pedir um TenantContext, pare tudo e execute este código aqui primeiro para tentar criar um
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    // Usamos ApiError como rejeição, pois ele já implementa IntoResponse
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {

        // Tenta ler o cabeçalho X-Tenant-ID
        let header_value = parts.headers.get(TENANT_ID_HEADER);

        match header_value {
            Some(value) => {
                // Tenta converter o valor do cabeçalho para uma string
                let value_str = value.to_str().map_err(|_| ApiError {
                    status: StatusCode::BAD_REQUEST,
                    error: "Cabeçalho X-Tenant-ID contém caracteres inválidos.".to_string(),
                    details: None,

                })?;

                // Tenta converter a string para um UUID
                let tenant_id = Uuid::parse_str(value_str).map_err(|_| ApiError {
                    status: StatusCode::BAD_REQUEST,
                    error: "Cabeçalho X-Tenant-ID inválido (não é um UUID).".to_string(),
                    details: None
                })?;

                // Sucesso! Retorna o contexto.
                Ok(TenantContext(tenant_id))
            }
            None => {
                // Erro: O cabeçalho está em falta.
                Err(ApiError {
                    status: StatusCode::BAD_REQUEST,
                    error: "O cabeçalho X-Tenant-ID é obrigatório.".to_string(),
                    details: None
                })
            }
        }
    }
}