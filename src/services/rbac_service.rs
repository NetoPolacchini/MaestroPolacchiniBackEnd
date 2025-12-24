// src/services/rbac_service.rs

use sqlx::{PgPool, Acquire}; // Acquire é necessário para iniciar transação
use uuid::Uuid;
use crate::common::error::AppError;
use crate::db::RbacRepository;
use crate::models::rbac::{Role, RoleResponse};

#[derive(Clone)]
pub struct RbacService {
    repo: RbacRepository,
    pool: PgPool,
}

impl RbacService {
    pub fn new(repo: RbacRepository, pool: PgPool) -> Self {
        Self { repo, pool }
    }

    pub async fn create_role_with_permissions(
        &self,
        tenant_id: Uuid,
        name: String,
        description: Option<String>,
        permission_slugs: Vec<String>,
    ) -> Result<RoleResponse, AppError> {
        // 1. Inicia Transação
        let mut tx = self.pool.begin().await?;

        // 2. Cria o Cargo
        let role = self.repo.create_role(&mut *tx, tenant_id, &name, description.as_deref()).await?;

        // 3. Resolve Slugs ("crm:read") para IDs (UUIDs)
        let permissions = self.repo.find_permissions_by_slugs(&mut *tx, &permission_slugs).await?;

        let permission_ids: Vec<Uuid> = permissions.iter().map(|p| p.id).collect();
        let valid_slugs: Vec<String> = permissions.into_iter().map(|p| p.slug).collect();

        // 4. Salva o Vínculo
        if !permission_ids.is_empty() {
            self.repo.assign_permissions(&mut *tx, role.id, &permission_ids).await?;
        }

        // 5. Commit
        tx.commit().await?;

        Ok(RoleResponse {
            role,
            permissions: valid_slugs,
        })
    }

    pub async fn list_system_permissions(&self) -> Result<Vec<crate::models::rbac::Permission>, AppError> {
        self.repo.list_all_permissions().await
    }
}