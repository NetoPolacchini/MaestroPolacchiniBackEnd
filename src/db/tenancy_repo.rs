// src/db/tenancy_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use crate::common::error::AppError;
 use crate::models::tenancy::{Tenant, UserTenant};

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        executor: E, // Aceita um executor (pool ou transação)
        name: &str,
        description: Option<&str>,
    ) -> Result<Tenant, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as!(
            Tenant,
            r#"
            INSERT INTO tenants (name, description)
            VALUES ($1, $2)
            RETURNING *
            "#,
            name,
            description
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                // (Futuramente, podemos adicionar verificação de nome de tenant duplicado aqui)
                e.into()
            })
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


}