// src/db/crm_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use serde_json::Value;
use chrono::NaiveDate;


use crate::{
    common::error::AppError,
    models::crm::{CrmFieldDefinition, CrmFieldType, Customer},
};
use crate::models::auth::{DocumentType, UserCompany};

#[derive(Clone)]
pub struct CrmRepository {
    pool: PgPool,
}

impl CrmRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    //  DEFINIÇÕES DE CAMPOS (O Molde)
    // =========================================================================

    /// Cria uma nova definição de campo (Ex: "Peso", "Alergias")
    pub async fn create_field_definition<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        key_name: &str,
        field_type: CrmFieldType,
        options: Option<&Value>, // JSON para opções de Select
        is_required: bool,
    ) -> Result<CrmFieldDefinition, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let definition = sqlx::query_as!(
        CrmFieldDefinition,
        r#"
        INSERT INTO crm_field_definitions (
            tenant_id, name, key_name, field_type, options, is_required
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id, tenant_id, name, key_name,
            -- AQUI ESTÁ A CORREÇÃO:
            field_type as "field_type: CrmFieldType",
            options, is_required, created_at
        "#,
        tenant_id,
        name,
        key_name,
        field_type as CrmFieldType,
        options,
        is_required
    )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                // Tratamento de erro de chave duplicada
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation(format!("A chave '{}' já existe.", key_name));
                    }
                }
                e.into()
            })?;

        Ok(definition)
    }

    /// Lista todas as definições para montar o formulário no Frontend
    pub async fn list_field_definitions<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<CrmFieldDefinition>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let fields = sqlx::query_as!(
        CrmFieldDefinition,
        r#"
        SELECT
            id, tenant_id, name, key_name,
            -- AQUI TAMBÉM:
            field_type as "field_type: CrmFieldType",
            options, is_required, created_at
        FROM crm_field_definitions
        WHERE tenant_id = $1
        ORDER BY created_at ASC
        "#,
        tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(fields)
    }

    // =========================================================================
    //  CLIENTES (O Dado)
    // =========================================================================

    /// Cria um cliente com dados flexíveis
    /// Cria um cliente com dados flexíveis
    pub async fn create_customer<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        full_name: &str,
        // [ATUALIZADO] Novos argumentos para identidade global
        country_code: Option<&str>,
        document_type: Option<DocumentType>,
        document_number: Option<&str>,
        // ---------------------------------------------------
        birth_date: Option<NaiveDate>,
        email: Option<&str>,
        phone: Option<&str>,
        mobile: Option<&str>,
        address: Option<&Value>,
        tags: Option<&[String]>,
        custom_data: &Value,
    ) -> Result<Customer, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Define padrões caso venha nulo (igual fizemos no UserRepo)
        let final_country = country_code.unwrap_or("BR");
        let final_doc_type = document_type.unwrap_or(DocumentType::TaxId);

        let customer = sqlx::query_as!(
            Customer,
            r#"
            INSERT INTO customers (
                tenant_id, full_name,
                country_code, document_type, document_number,
                birth_date, email, phone, mobile, address, tags, custom_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING
                id, tenant_id, user_id, full_name, birth_date,
                email, phone, mobile, address, tags, custom_data,
                country_code,
                -- CAST EXPLÍCITO OBRIGATÓRIO:
                document_type as "document_type: DocumentType",
                document_number,
                created_at, updated_at
            "#,
            tenant_id,
            full_name,
            final_country,               // $3
            final_doc_type as DocumentType, // $4
            document_number,             // $5
            birth_date,                  // $6
            email,                       // $7
            phone,                       // $8
            mobile,                      // $9
            address,                     // $10
            tags as Option<&[String]>,   // $11
            custom_data                  // $12
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::UniqueConstraintViolation(format!("Documento '{}' já cadastrado.", document_number.unwrap_or("?")));
                    }
                }
                e.into()
            })?;

        Ok(customer)
    }

    /// Busca simples de todos os clientes
    pub async fn list_customers<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<Customer>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let customers = sqlx::query_as!(
            Customer,
            r#"
            SELECT
                id, tenant_id, user_id, full_name, birth_date,
                email, phone, mobile, address, tags, custom_data,
                country_code,
                document_type as "document_type: DocumentType",
                document_number,
                created_at, updated_at
            FROM customers
            WHERE tenant_id = $1
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(customers)
    }

    /// Exemplo de Poder: Busca por CPF OU por campo dentro do JSON
    /// Ex: Buscar quem tem "weight" > 80 ou quem tem a tag "VIP"
    /// (Para simplificar, vamos fazer busca por nome ou documento por enquanto)
    pub async fn search_customers<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        query: &str,
    ) -> Result<Vec<Customer>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let search_term = format!("%{}%", query);

        // [CORREÇÃO] Removemos SELECT * e colocamos os campos explícitos com o CAST
        let customers = sqlx::query_as!(
            Customer,
            r#"
            SELECT
                id, tenant_id, user_id, full_name, birth_date,
                email, phone, mobile, address, tags, custom_data,
                country_code,
                document_type as "document_type: DocumentType",
                document_number,
                created_at, updated_at
            FROM customers
            WHERE tenant_id = $1
            AND (
                full_name ILIKE $2
                OR document_number ILIKE $2
                OR email ILIKE $2
            )
            ORDER BY full_name ASC
            LIMIT 50
            "#,
            tenant_id,
            search_term
        )
            .fetch_all(executor)
            .await?;

        Ok(customers)
    }

    /// O LINK MÁGICO: Vincula clientes órfãos ao usuário recém-criado
    pub async fn link_user_to_existing_customers<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        country_code: &str,
        document_type: DocumentType,
        document_number: &str,
    ) -> Result<u64, AppError> // Retorna quantos registros foram atualizados
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query!(
            r#"
            UPDATE customers
            SET user_id = $1, updated_at = NOW()
            WHERE
                country_code = $2
                AND document_type = $3::document_type -- Cast importante!
                AND document_number = $4
                AND user_id IS NULL -- Só pega se não tiver dono ainda
            "#,
            user_id,
            country_code,
            document_type as DocumentType,
            document_number
        )
            .execute(executor)
            .await?;

        Ok(result.rows_affected())
    }

    /// Encontra as companies que o usuário possui um registro
    pub async fn find_companies_by_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<UserCompany>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Fazemos um JOIN para pegar o nome da loja (tenants)
        // Baseado no vínculo que existe na tabela customers
        let companies = sqlx::query_as!(
            UserCompany,
            r#"
            SELECT DISTINCT t.id, t.name, t.slug
            FROM tenants t
            INNER JOIN customers c ON c.tenant_id = t.id
            WHERE c.user_id = $1
            ORDER BY t.name ASC
            "#,
            user_id
        )
            .fetch_all(executor)
            .await?;

        Ok(companies)
    }

}