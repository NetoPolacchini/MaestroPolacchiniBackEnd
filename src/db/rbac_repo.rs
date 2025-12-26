// src/db/rbac_repo.rs

use sqlx::{Postgres, Executor, PgPool};
use uuid::Uuid;
use crate::common::error::AppError;
use crate::models::rbac::{Role, Permission};

#[derive(Clone)]
pub struct RbacRepository {
    pool: PgPool,
}

impl RbacRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // 1. Criar o Cargo
    pub async fn create_role<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<Role, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let role = sqlx::query_as!(
            Role,
            r#"
            INSERT INTO roles (tenant_id, name, description)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, name, description, created_at, updated_at
            "#,
            tenant_id,
            name,
            description
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation("Já existe um cargo com esse nome.".into());
                    }
                }
                e.into()
            })?;

        Ok(role)
    }

    // 2. Buscar IDs das permissões baseado nos Slugs ("inventory:write" -> UUID)
    pub async fn find_permissions_by_slugs<'e, E>(
        &self,
        executor: E,
        slugs: &[String],
    ) -> Result<Vec<Permission>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // O SQLx lida bem com arrays usando ANY
        let permissions = sqlx::query_as!(
            Permission,
            r#"
            SELECT id, slug, description, module
            FROM permissions
            WHERE slug = ANY($1)
            "#,
            slugs
        )
            .fetch_all(executor)
            .await?;

        Ok(permissions)
    }

    // 3. Vincular Cargo <-> Permissão
    pub async fn assign_permissions<'e, E>(
        &self,
        executor: E,
        role_id: Uuid,
        permission_ids: &[Uuid],
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Inserção em massa usando UNNEST para performance
        sqlx::query!(
            r#"
            INSERT INTO role_permissions (role_id, permission_id)
            SELECT $1, unnest($2::uuid[])
            ON CONFLICT DO NOTHING
            "#,
            role_id,
            permission_ids
        )
            .execute(executor)
            .await?;

        Ok(())
    }

    // 4. Listar todas as permissões disponíveis (para o Frontend montar a tela)
    pub async fn list_all_permissions(&self) -> Result<Vec<Permission>, AppError> {
        let permissions = sqlx::query_as!(
            Permission,
            "SELECT id, slug, description, module FROM permissions ORDER BY module, slug"
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(permissions)
    }

    pub async fn user_has_permission(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        permission_slug: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM tenant_members tm
                JOIN roles r ON tm.role_id = r.id
                JOIN role_permissions rp ON r.id = rp.role_id
                JOIN permissions p ON rp.permission_id = p.id
                WHERE tm.user_id = $1
                  AND tm.tenant_id = $2
                  AND tm.is_active = true
                  AND p.slug = $3
            )
            "#,
            user_id,
            tenant_id,
            permission_slug
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(result.exists.unwrap_or(false))
    }
}