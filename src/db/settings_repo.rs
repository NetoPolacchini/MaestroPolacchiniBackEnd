use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;
use crate::{
    common::error::AppError,
    models::settings::{TenantSettings, UpdateSettingsRequest},
};

#[derive(Clone)]
pub struct SettingsRepository {
    pool: PgPool,
}

impl SettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_settings<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<TenantSettings, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Tenta buscar. Se não existir, retorna um Default (ou cria um vazio).
        // Aqui vamos usar LEFT JOIN ou simplesmente buscar e tratar o erro.
        // Opção robusta: UPSERT na criação do tenant, mas vamos tratar o "Not Found" como "Vazio".

        let settings = sqlx::query_as!(
            TenantSettings,
            "SELECT * FROM tenant_settings WHERE tenant_id = $1",
            tenant_id
        )
            .fetch_optional(executor)
            .await?;

        match settings {
            Some(s) => Ok(s),
            None => Ok(TenantSettings {
                tenant_id,
                logo_url: None, primary_color: None, company_name: None,
                document_number: None, address: None, phone: None,
                email: None, pix_key: None, pix_key_type: None,
                updated_at: None,
            })
        }
    }

    pub async fn update_settings<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        input: UpdateSettingsRequest,
    ) -> Result<TenantSettings, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UPSERT (Insert or Update)
        let settings = sqlx::query_as!(
            TenantSettings,
            r#"
            INSERT INTO tenant_settings (tenant_id, company_name, document_number, pix_key, address)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (tenant_id)
            DO UPDATE SET
                company_name = EXCLUDED.company_name,
                document_number = EXCLUDED.document_number,
                pix_key = EXCLUDED.pix_key,
                address = EXCLUDED.address,
                updated_at = NOW()
            RETURNING *
            "#,
            tenant_id,
            input.company_name,
            input.document_number,
            input.pix_key,
            input.address
        )
            .fetch_one(executor)
            .await?;

        Ok(settings)
    }
}