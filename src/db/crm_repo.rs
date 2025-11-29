// src/db/crm_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use serde_json::Value;
use chrono::NaiveDate;

use crate::{
    common::error::AppError,
    models::crm::{CrmFieldDefinition, CrmFieldType, Customer},
};

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
    pub async fn create_customer<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        full_name: &str,
        document_number: Option<&str>,
        birth_date: Option<NaiveDate>,
        email: Option<&str>,
        phone: Option<&str>,
        mobile: Option<&str>,
        address: Option<&Value>,    // JSON do endereço
        tags: Option<&[String]>,    // Array de strings (tags)
        custom_data: &Value,        // O JSON mágico com os campos personalizados
    ) -> Result<Customer, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let customer = sqlx::query_as!(
            Customer,
            r#"
            INSERT INTO customers (
                tenant_id, full_name, document_number, birth_date,
                email, phone, mobile, address, tags, custom_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
            tenant_id,
            full_name,
            document_number,
            birth_date,
            email,
            phone,
            mobile,
            address,
            tags as Option<&[String]>, // Cast importante para array de texto
            custom_data
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
            SELECT * FROM customers
            WHERE tenant_id = $1
            ORDER BY full_name ASC
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

        let customers = sqlx::query_as!(
            Customer,
            r#"
            SELECT * FROM customers
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
}