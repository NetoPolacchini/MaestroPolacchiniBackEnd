// src/services/tenancy_service.rs

use crate::{
    common::error::AppError,
    db::TenantRepository, // O repositório que acabámos de atualizar
    models::tenancy::Tenant,
};
use sqlx::{Acquire, Executor, PgPool, Postgres};
use uuid::Uuid;
use crate::models::tenancy::{Location, StockPool};

#[derive(Clone)]
pub struct TenantService {
    tenant_repo: TenantRepository,
    pool: PgPool, // Usamos a pool para iniciar transações
}

impl TenantService {
    /// Cria uma nova instância do serviço de tenancy.
    pub fn new(tenant_repo: TenantRepository, pool: PgPool) -> Self {
        Self { tenant_repo, pool }
    }

    /// LÓGICA DE NEGÓCIO: Cria um novo Estabelecimento e, atomicamente,
    /// atribui o utilizador que o criou como o seu primeiro membro (dono).
    pub async fn create_tenant_and_assign_owner(
        &self,
        name: &str,
        description: Option<&str>,
        owner_id: Uuid,
    ) -> Result<Tenant, AppError> {

        let already_exists = self.tenant_repo
            .user_has_tenant_with_name(owner_id, name)
            .await?;

        if already_exists {
            return Err(AppError::TenantNameAlreadyExists(name.to_string()));
        }

        // 1. Inicia a transação
        let mut tx = self.pool.begin().await?;

        // 2. Cria o Estabelecimento (Tenant)
        let new_tenant = self.tenant_repo
            .create_tenant(&mut *tx, name, description) // Passa a transação
            .await?;

        // 3. Atribui o utilizador (dono) ao novo tenant
        self.tenant_repo
            .assign_user_to_tenant(
                &mut *tx, // Passa a MESMA transação
                owner_id,
                new_tenant.id,
            )
            .await?;

        // 4. Se ambas as etapas (2 e 3) funcionarem, commita.
        tx.commit().await?;

        // Se algo falhar, o 'tx' é descartado e o Rust dá ROLLBACK.

        Ok(new_tenant)
    }

    /// Cria uma nova loja e automaticamente cria um "Pool Exclusivo" para ela.
    /// Isso implementa a regra: "Toda loja nasce com seu próprio estoque".
    pub async fn create_location_standalone<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        is_warehouse: bool,
    ) -> Result<Location, AppError>
    where
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        // 1. Inicia a Transação (Atomicidade)
        // Se criar o pool funcionar, mas criar a loja falhar, desfaz tudo.
        let mut tx = executor.begin().await?;

        // 2. Define o nome do Pool Automático
        // Ex: "Estoque - Loja Centro"
        let pool_name = format!("Estoque - {}", name);
        let pool_description = format!("Pool automático criado para a loja {}", name);

        // 3. Cria o Pool
        let new_pool = self.tenant_repo
            .create_stock_pool(
                &mut *tx, // Passa a transação
                tenant_id,
                &pool_name,
                Some(&pool_description)
            )
            .await?;

        // 4. Cria a Loja vinculada a esse novo Pool
        let new_location = self.tenant_repo
            .create_location(
                &mut *tx, // Passa a mesma transação
                tenant_id,
                new_pool.id, // <--- O elo de ligação
                name,
                is_warehouse
            )
            .await?;

        // 5. Confirma (Commit)
        tx.commit().await?;

        Ok(new_location)
    }



    /// [NOVO] Serviço simples para listar tenants
    pub async fn list_user_tenants(&self, user_id: Uuid) -> Result<Vec<Tenant>, AppError> {
        self.tenant_repo.get_tenants_for_user(user_id).await
    }

    pub async fn create_stock_pool(
        &self,
        tenant_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<StockPool, AppError> {
        let mut tx = self.pool.begin().await?;

        let pool = self.tenant_repo
            .create_stock_pool(&mut *tx, tenant_id, name, description)
            .await?;

        tx.commit().await?;
        Ok(pool)
    }

    pub async fn create_location(
        &self,
        tenant_id: Uuid,
        stock_pool_id: Uuid,
        name: &str,
        is_warehouse: bool,
    ) -> Result<Location, AppError> {
        let mut tx = self.pool.begin().await?;

        let location = self.tenant_repo
            .create_location(&mut *tx, tenant_id, stock_pool_id, name, is_warehouse)
            .await?;

        tx.commit().await?;
        Ok(location)
    }

    // [NOVO] Listar lojas
    pub async fn list_locations(
        &self,
        tenant_id: Uuid
    ) -> Result<Vec<Location>, AppError> {

        // Aqui não precisamos de transação (begin/commit),
        // pois é apenas uma leitura. Passamos a pool direto.
        let locations = self.tenant_repo
            .find_all_locations(&self.pool, tenant_id)
            .await?;

        Ok(locations)
    }

}