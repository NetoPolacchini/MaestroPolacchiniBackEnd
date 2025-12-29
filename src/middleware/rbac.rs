// src/middleware/rbac.rs

use axum::{
    extract::{FromRequestParts, FromRef},
    http::{request::Parts, StatusCode},
};
use std::marker::PhantomData;

use crate::{
    common::error::ApiError,
    config::AppState,
    middleware::{auth::AuthenticatedUser, tenancy::TenantContext},
};

/// 1. O Trait que define o que é uma Permissão
pub trait PermissionDef: Send + Sync + 'static {
    fn slug() -> &'static str;
}

/// 2. O Extractor (Guardião)
pub struct RequirePermission<T>(pub PhantomData<T>);

// 3. Implementação do FromRequestParts

impl<T, S> FromRequestParts<S> for RequirePermission<T>
where
    T: PermissionDef,
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // A. Extrai Usuário
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or(ApiError {
                status: StatusCode::UNAUTHORIZED,
                error: "Usuário não autenticado".into(),
                details: None,
            })?;

        // B. Extrai Tenant
        let tenant = parts
            .extensions
            .get::<TenantContext>()
            .ok_or(ApiError {
                status: StatusCode::BAD_REQUEST,
                error: "Contexto da loja não encontrado".into(),
                details: None,
            })?;

        // C. Pega o slug da permissão
        let required_perm = T::slug();

        // D. Verifica no Banco
        let has_permission = app_state
            .rbac_repo
            .user_has_permission(user.0.id, tenant.0, required_perm)
            .await
            .map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                error: "Falha ao verificar permissões".into(),
                details: None,
            })?;

        if !has_permission {
            return Err(ApiError {
                status: StatusCode::FORBIDDEN,
                error: format!("Você precisa da permissão '{}' para realizar esta ação.", required_perm),
                details: None,
            });
        }

        Ok(RequirePermission(PhantomData))
    }
}

// ---
// DEFINIÇÃO DAS PERMISSÕES (TIPOS)
// ---

pub struct PermInventoryWrite;
impl PermissionDef for PermInventoryWrite {
    fn slug() -> &'static str { "inventory:write" }
}

pub struct PermInventoryRead;
impl PermissionDef for PermInventoryRead {
    fn slug() -> &'static str { "inventory:read" }
}

pub struct PermCrmRead;
impl PermissionDef for PermCrmRead {
    fn slug() -> &'static str { "crm:read" }
}