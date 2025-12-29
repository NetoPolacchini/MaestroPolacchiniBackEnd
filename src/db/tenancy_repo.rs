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
        E: Executor<'e, Database = Postgres>, // <--- 1. REMOVIDO O "+ Copy"
    {
        let clean_name = name.to_lowercase().replace(" ", "-");
        let mut final_slug = String::new();

        // 2. FASE DE GERAÇÃO (Usamos self.pool)
        // Usamos a pool principal para verificar duplicidade.
        // Isso é seguro, rápido e não consome a sua transação.
        for _ in 0..8 {
            let random_suffix = Uuid::new_v4().to_string();
            // Use 6 chars para garantir entropia suficiente
            let candidate_slug = format!("{}-{}", clean_name, &random_suffix[0..6]);

            let exists = sqlx::query!(
            "SELECT count(*) as count FROM tenants WHERE slug = $1",
            candidate_slug
        )
                .fetch_one(&self.pool) // <--- TRUQUE: Usamos &self.pool aqui!
                .await?
                .count
                .unwrap_or(0) > 0;

            if !exists {
                final_slug = candidate_slug;
                break;
            }

            tracing::warn!("⚠️ Colisão de slug detectada: {}. Tentando novo...", candidate_slug);
        }

        if final_slug.is_empty() {
            return Err(AppError::InternalServerError(anyhow::anyhow!("Falha ao gerar slug único após múltiplas tentativas")));
        }

        // 3. FASE DE INSERÇÃO (Usamos o Executor/Transação)
        // Agora que temos um slug seguro, usamos a transação apenas uma vez.
        let tenant = sqlx::query_as!(
        Tenant,
        r#"
        INSERT INTO tenants (name, description, slug)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        name,
        description,
        final_slug
    )
            .fetch_one(executor) // <--- O executor é movido aqui (tudo bem, é a última linha)
            .await
            .map_err(|e| {
                // Se ainda assim der erro de Unique (chance de 0.000001%),
                // infelizmente a transação terá que ser abortada.
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        if let Some(constraint) = db_err.constraint() {
                            if constraint == "idx_tenants_slug" {
                                return AppError::UniqueConstraintViolation("Erro raríssimo de colisão simultânea. Tente novamente.".into());
                            }
                        }
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
        role_id: Uuid,
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
                    // 1. Checagem de Duplicidade (Já está na loja)
                    if db_err.is_unique_violation() {
                        if let Some(constraint) = db_err.constraint() {
                            // Confirme se o nome é este mesmo no seu banco
                            if constraint == "tenant_members_pkey" {
                                return AppError::MemberAlreadyExists;
                            }
                        }
                    }

                    // 2. Checagem de Integridade (Role ou Tenant não existem)
                    // O código 23503 é Foreign Key Violation
                    if db_err.is_foreign_key_violation() {
                        if let Some(constraint) = db_err.constraint() {
                            if constraint == "tenant_members_role_id_fkey" {
                                return AppError::RoleDoesNotExist(String::from(role_id));
                            }
                        }
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
                        return AppError::PoolAlreadyExists(name.to_string());
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
                        return AppError::LocationAlreadyExists(name.to_string());
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