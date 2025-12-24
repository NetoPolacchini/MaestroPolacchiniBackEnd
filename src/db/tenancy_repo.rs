// src/db/tenancy_repo.rs

use sqlx::{PgPool, Postgres, Executor}; // Removi PgConnection que não estava a ser usado explicitamente
use uuid::Uuid;
use crate::common::error::AppError;
use crate::models::tenancy::{Tenant, StockPool, Location};
// Nota: Não precisamos importar TenantMember aqui a não ser que o retornemos,
// mas as queries usam Tenant, StockPool, Location.

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Verifica se um utilizador já possui um tenant com um nome específico.
    /// Atualizado para verificar na tabela tenant_members.
    pub async fn user_has_tenant_with_name(&self, user_id: Uuid, name: &str) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM tenants t
                JOIN tenant_members tm ON t.id = tm.tenant_id
                WHERE tm.user_id = $1 AND t.name = $2
            )
            "#,
            user_id,
            name
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// Retorna todos os tenants ativos aos quais o utilizador tem acesso.
    /// Atualizado para usar tenant_members e verificar is_active.
    pub async fn get_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<Tenant>, AppError> {
        let tenants = sqlx::query_as!(
            Tenant,
            r#"
            SELECT t.* FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = $1 AND tm.is_active = true
            ORDER BY t.name ASC
            "#,
            user_id
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(tenants)
    }

    /// Verifica permissão de acesso (Authorization).
    /// Atualizado para tenant_members.
    pub async fn check_user_tenancy(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM tenant_members
                WHERE user_id = $1 AND tenant_id = $2 AND is_active = true
            )
            "#,
            user_id,
            tenant_id
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// Cria um novo tenant com Slug gerado.
    pub async fn create_tenant<'e, E>(
        &self,
        executor: E,
        name: &str,
        description: Option<&str>,
    ) -> Result<Tenant, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Gera slug simples: "nome-loja-uuid"
        let clean_name = name.to_lowercase().replace(" ", "-");
        let random_suffix = Uuid::new_v4().to_string();
        let slug = format!("{}-{}", clean_name, &random_suffix[0..4]);

        let tenant = sqlx::query_as!(
            Tenant,
            r#"
            INSERT INTO tenants (name, description, slug)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            name,
            description,
            slug
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation("Já existe uma loja com este link.".into());
                    }
                }
                e.into()
            })?;

        Ok(tenant)
    }

    /// [NOVO MÉTODO] Adiciona membro com Cargo (Role).
    /// Substitui o antigo assign_user_to_tenant.
    pub async fn add_member_to_tenant<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        user_id: Uuid,
        role_id: Uuid, // <--- Agora exigimos o Cargo
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query!(
            r#"
            INSERT INTO tenant_members (tenant_id, user_id, role_id, is_active)
            VALUES ($1, $2, $3, true)
            "#,
            tenant_id,
            user_id,
            role_id
        )
            .execute(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation("Usuário já é membro desta loja.".into());
                    }
                }
                e.into()
            })?;

        Ok(())
    }

    // --- Métodos de Estoque/Pool (Mantidos do original) ---

    pub async fn create_stock_pool<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<StockPool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as!(
            StockPool,
            r#"
            INSERT INTO stock_pools (tenant_id, name, description)
            VALUES ($1, $2, $3)
            RETURNING *
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
                        return AppError::UniqueConstraintViolation(format!("Pool '{}' já existe", name));
                    }
                }
                e.into()
            })
    }

    pub async fn create_location<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        stock_pool_id: Uuid,
        name: &str,
        is_warehouse: bool,
    ) -> Result<Location, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as!(
            Location,
            r#"
            INSERT INTO locations (tenant_id, stock_pool_id, name, is_warehouse)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
            tenant_id,
            stock_pool_id,
            name,
            is_warehouse
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation(format!("Local '{}' já existe", name));
                    }
                }
                e.into()
            })
    }

    pub async fn find_all_locations<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid
    ) -> Result<Vec<Location>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let locations = sqlx::query_as!(
            Location,
            r#"SELECT * FROM locations WHERE tenant_id = $1 ORDER BY name ASC"#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(locations)
    }
}