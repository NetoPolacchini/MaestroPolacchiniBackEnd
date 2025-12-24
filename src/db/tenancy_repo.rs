// src/db/tenancy_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use crate::common::error::AppError;
 use crate::models::tenancy::{Tenant, UserTenant, StockPool, Location};

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }


    /// Verifica se um utilizador já possui um tenant com um nome específico.
    pub async fn user_has_tenant_with_name(&self, user_id: Uuid, name: &str) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM tenants t
                JOIN user_tenants ut ON t.id = ut.tenant_id
                WHERE ut.user_id = $1 AND t.name = $2
            )
            "#,
            user_id,
            name
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// Retorna todos os tenants aos quais o utilizador tem acesso.
    pub async fn get_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<Tenant>, AppError> {
        let tenants = sqlx::query_as!(
            Tenant,
            r#"
            SELECT t.* FROM tenants t
            JOIN user_tenants ut ON t.id = ut.tenant_id
            WHERE ut.user_id = $1
            ORDER BY t.name ASC
            "#,
            user_id
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(tenants)
    }


    /// Verifica se um utilizador tem permissão para aceder a um tenant.
    /// Esta é a verificação de segurança de autorização mais importante.
    pub async fn check_user_tenancy(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<bool, AppError> {

        // Usamos SELECT EXISTS para a consulta mais rápida possível.
        // Ele apenas retorna 'true' ou 'false' se a linha for encontrada.
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM user_tenants
                WHERE user_id = $1 AND tenant_id = $2
            )
            "#,
            user_id,
            tenant_id
        )
            .fetch_one(&self.pool)
            .await?;

        // Se 'exists' não for nulo e for 'true', o acesso é permitido.
        Ok(result.exists.unwrap_or(false))
    }

    /// [NOVO] Cria um novo tenant (Estabelecimento) na base de dados.
    pub async fn create_tenant<'e, E>(
        &self,
        executor: E,
        name: &str,
        description: Option<&str>,
    ) -> Result<Tenant, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // 1. Gera um Slug simples (Ex: "padaria-do-ze-a1b2")
        // Usamos uuid parcial para garantir unicidade sem complicar muito agora
        let clean_name = name.to_lowercase().replace(" ", "-");
        let random_suffix = Uuid::new_v4().to_string(); // Pega um UUID novo
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
                // Tratamento básico de erro (caso dê azar de gerar slug igual)
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation("Já existe uma loja com este link.".into());
                    }
                }
                e.into()
            })?;

        Ok(tenant)
    }

    /// [NOVO] Atribui um utilizador a um tenant (na tabela-ponte).
    pub async fn assign_user_to_tenant<'e, E>(
        &self,
        executor: E, // Aceita um executor (pool ou transação)
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<UserTenant, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as!(
            UserTenant,
            r#"
            INSERT INTO user_tenants (user_id, tenant_id)
            VALUES ($1, $2)
            RETURNING *
            "#,
            user_id,
            tenant_id
        )
            .fetch_one(executor)
            .await
            .map_err(|e| e.into()) // Erros de chave duplicada são tratados pelo serviço
    }

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

    // [NOVO] Método para buscar todas as lojas de um tenant
    pub async fn find_all_locations<'e, E>(
        &self,
        executor: E, // Aceita conexão ou transação (flexibilidade)
        tenant_id: Uuid
    ) -> Result<Vec<Location>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let locations = sqlx::query_as!(
            Location,
            r#"
            SELECT * FROM locations
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?; // Se der erro de banco, o '?' propaga

        Ok(locations)
    }

}