// src/db/crm_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use serde_json::Value;
use chrono::NaiveDate;

use crate::{
    common::error::AppError,
    // Note que atualizei os imports para bater com o models/crm.rs novo
    models::crm::{FieldDefinition, FieldType, Customer, EntityType},
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
    //  1. TIPOS DE ENTIDADE (NOVO)
    //  Ex: "Paciente", "Aluno", "Veículo"
    // =========================================================================

    pub async fn create_entity_type<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        slug: &str,
    ) -> Result<EntityType, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let entity_type = sqlx::query_as!(
            EntityType,
            r#"
            INSERT INTO crm_entity_types (tenant_id, name, slug)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, name, slug, created_at
            "#,
            tenant_id,
            name,
            slug
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::CrmEntityTypeAlreadyExists(slug.to_string());
                    }
                }
                e.into()
            })?;

        Ok(entity_type)
    }

    pub async fn list_entity_types<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<EntityType>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let types = sqlx::query_as!(
            EntityType,
            r#"
            SELECT id, tenant_id, name, slug, created_at
            FROM crm_entity_types
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(types)
    }

    // =========================================================================
    //  2. DEFINIÇÕES DE CAMPOS (O Molde)
    // =========================================================================

    /// Cria uma nova definição de campo (Ex: "Peso", "Alergias")
    pub async fn create_field_definition<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        entity_type_id: Option<Uuid>, // [NOVO] Vincula a um tipo específico
        name: &str,
        key_name: &str,
        field_type: FieldType,
        options: Option<&Value>,
        is_required: bool,
    ) -> Result<FieldDefinition, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let definition = sqlx::query_as!(
            FieldDefinition,
            r#"
            INSERT INTO crm_field_definitions (
                tenant_id, entity_type_id, name, key_name, field_type, options, is_required
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, tenant_id, entity_type_id, name, key_name,
                field_type as "field_type: FieldType",
                options, is_required
            "#,
            tenant_id,
            entity_type_id,
            name,
            key_name,
            field_type as FieldType,
            options,
            is_required
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::CrmFieldKeyAlreadyExists(key_name.to_string());
                    }
                }
                e.into()
            })?;

        Ok(definition)
    }

    /// Lista TODAS as definições do tenant (Globais + Específicas)
    pub async fn list_all_field_definitions<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<FieldDefinition>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let fields = sqlx::query_as!(
            FieldDefinition,
            r#"
            SELECT
                id, tenant_id, entity_type_id, name, key_name,
                field_type as "field_type: FieldType",
                options, is_required
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
    //  3. CLIENTES (O Dado)
    // =========================================================================

    pub async fn create_customer<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        full_name: &str,
        country_code: Option<&str>,
        document_type: Option<DocumentType>,
        document_number: Option<&str>,
        birth_date: Option<NaiveDate>,
        email: Option<&str>,
        phone: Option<&str>,
        mobile: Option<&str>,
        address: Option<&Value>,
        tags: Option<&[String]>,

        // [NOVO] Array de tipos (ex: [ID_PACIENTE, ID_ALUNO])
        entity_types: Option<&[Uuid]>,
        custom_data: &Value,
    ) -> Result<Customer, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let final_country = country_code.unwrap_or("BR");
        let final_doc_type = document_type.unwrap_or(DocumentType::TaxId);

        // Garante vetor vazio se vier None para o SQL não reclamar
        let final_entity_types = entity_types.unwrap_or(&[]);

        let customer = sqlx::query_as!(
            Customer,
            r#"
            INSERT INTO customers (
                tenant_id, full_name,
                country_code, document_type, document_number,
                birth_date, email, phone, mobile, address, tags,
                entity_types, custom_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING
                id, tenant_id, user_id, full_name, birth_date,
                email, phone, mobile, address, tags,
                entity_types, custom_data,
                country_code,
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
            final_entity_types,          // $12 [NOVO]
            custom_data                  // $13
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::CustomerDocumentAlreadyExists(document_number.unwrap_or("?").to_string());
                    }
                }
                e.into()
            })?;

        Ok(customer)
    }

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
                email, phone, mobile, address, tags,
                entity_types, custom_data,
                country_code,
                document_type as "document_type: DocumentType",
                document_number,
                created_at, updated_at
            FROM customers
            WHERE tenant_id = $1
            ORDER BY full_name ASC
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(customers)
    }

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
            SELECT
                id, tenant_id, user_id, full_name, birth_date,
                email, phone, mobile, address, tags,
                entity_types, custom_data,
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

    // O método link_user_to_existing_customers e find_companies_by_user
    // permanecem iguais, pois não dependem dos campos novos.

    pub async fn link_user_to_existing_customers<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        country_code: &str,
        document_type: DocumentType,
        document_number: &str,
    ) -> Result<u64, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query!(
            r#"
            UPDATE customers
            SET user_id = $1, updated_at = NOW()
            WHERE
                country_code = $2
                AND document_type = $3::document_type
                AND document_number = $4
                AND user_id IS NULL
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

    pub async fn find_companies_by_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<UserCompany>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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