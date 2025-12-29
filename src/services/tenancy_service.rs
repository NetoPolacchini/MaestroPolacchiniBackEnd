// src/services/tenancy_service.rs

use crate::{
    common::error::AppError,
    db::{TenantRepository, RbacRepository}, // O repositório que acabámos de atualizar
    models::tenancy::Tenant,
};
use sqlx::{Acquire, Executor, PgPool, Postgres};
use uuid::Uuid;
use crate::models::tenancy::{Location, StockPool};

#[derive(Clone)]
pub struct TenantService {
    tenant_repo: TenantRepository,
    rbac_repo: RbacRepository,
    pool: PgPool, // Usamos a pool para iniciar transações
}

impl TenantService {
    /// Cria uma nova instância do serviço de tenancy.
    pub fn new(
        tenant_repo: TenantRepository,
        rbac_repo: RbacRepository,
        pool: PgPool
    ) -> Self {
        Self { tenant_repo, rbac_repo, pool }
    }

    /// LÓGICA DE NEGÓCIO: Cria um novo Estabelecimento e, atomicamente,
    /// atribui o utilizador que o criou como o seu primeiro membro (dono).
    pub async fn create_tenant_with_owner(
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
            .create_tenant(&mut *tx, name, description)
            .await?;

        // 3. [NOVO] Cria o Cargo "Dono" para esta loja
        let owner_role = self.rbac_repo.create_role(
            &mut *tx,
            new_tenant.id,
            "Dono",
            Some("Acesso total administrativo (Gerado automaticamente)")
        ).await?;

        // 4. [NOVO] Busca TODAS as permissões do sistema
        let all_permissions = self.rbac_repo.list_all_permissions().await?; // Use &self.pool se list_all_permissions não aceitar executor, ou ajuste o repo
        // Nota: Se list_all_permissions usar &self.pool internamente, não participa da transação.
        // Idealmente, ele deveria aceitar executor, mas como é só leitura, não quebra a lógica crítica.

        let all_perm_ids: Vec<Uuid> = all_permissions.iter().map(|p| p.id).collect();

        // 5. [NOVO] Atribui as permissões ao cargo
        if !all_perm_ids.is_empty() {
            self.rbac_repo.assign_permissions(&mut *tx, owner_role.id, &all_perm_ids).await?;
        }

        // 6. [NOVO] Atribui o usuário à loja COM O CARGO CRIADO
        self.tenant_repo
            .add_member_to_tenant(
                &mut *tx,
                new_tenant.id,
                owner_id,
                owner_role.id 
            )
            .await?;

        // 7. Commit
        tx.commit().await?;

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