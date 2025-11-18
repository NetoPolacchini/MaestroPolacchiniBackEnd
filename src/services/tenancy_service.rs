// src/services/tenancy_service.rs

use crate::{
    common::error::AppError,
    db::TenantRepository, // O repositório que acabámos de atualizar
    models::tenancy::Tenant,
};
use sqlx::PgPool;
use uuid::Uuid;

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
}