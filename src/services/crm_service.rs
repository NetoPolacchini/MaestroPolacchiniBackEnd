// src/services/crm_service.rs

use std::collections::HashMap;
use serde_json::Value;
use sqlx::{ Postgres, Executor, Acquire};
use uuid::Uuid;
use chrono::NaiveDate;

use crate::{
    common::error::AppError,
    db::CrmRepository,
    // [ATUALIZADO] Imports corretos do models/crm.rs
    models::crm::{FieldDefinition, FieldType, Customer, EntityType},
};
use crate::models::auth::{DocumentType, UserCompany};

#[derive(Clone)]
pub struct CrmService {
    repo: CrmRepository,
}

impl CrmService {
    pub fn new(repo: CrmRepository) -> Self {
        Self { repo }
    }

    // =========================================================================
    //  1. TIPOS DE ENTIDADE (NOVO)
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
        // Poderíamos validar formato do slug aqui se quisesse
        self.repo.create_entity_type(executor, tenant_id, name, slug).await
    }

    pub async fn list_entity_types<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<EntityType>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.list_entity_types(executor, tenant_id).await
    }

    // =========================================================================
    //  2. CONFIGURAÇÃO (DEFINIÇÕES DE CAMPO)
    // =========================================================================

    pub async fn create_field_definition<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        // [NOVO] Opcional: O campo pertence a um tipo específico?
        entity_type_id: Option<Uuid>,
        name: &str,
        key_name: &str,
        field_type: FieldType,
        options: Option<Value>,
        is_required: bool,
    ) -> Result<FieldDefinition, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Aqui poderíamos validar se 'options' é válido caso o tipo seja SELECT

        self.repo.create_field_definition(
            executor,
            tenant_id,
            entity_type_id, // Passando o novo argumento
            name,
            key_name,
            field_type,
            options.as_ref(),
            is_required
        ).await
    }

    pub async fn list_field_definitions<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<FieldDefinition>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.list_all_field_definitions(executor, tenant_id).await
    }

    // =========================================================================
    //  3. CLIENTES (COM VALIDAÇÃO DINÂMICA INTELIGENTE)
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
        address: Option<Value>,
        tags: Option<Vec<String>>,

        // [NOVO] Quais tipos esse cliente tem?
        entity_types: Option<Vec<Uuid>>,
        custom_data: Value,
    ) -> Result<Customer, AppError>
    where
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        let mut tx = executor.begin().await?;

        // 1. Busca TODAS as definições do tenant
        let definitions = self.repo.list_all_field_definitions(&mut *tx, tenant_id).await?;

        // 2. Valida os dados customizados APENAS para os tipos selecionados
        self.validate_custom_data(
            &definitions,
            &custom_data,
            entity_types.as_deref().unwrap_or(&[])
        )?;

        let tags_slice = tags.as_deref();
        let entity_types_slice = entity_types.as_deref();

        // 3. Salva
        let customer = self.repo.create_customer(
            &mut *tx,
            tenant_id,
            full_name,
            country_code,
            document_type,
            document_number,
            birth_date,
            email,
            phone,
            mobile,
            address.as_ref(),
            tags_slice,
            entity_types_slice, // Passando para o repo
            &custom_data
        ).await?;

        tx.commit().await?;

        Ok(customer)
    }

    // --- MOTOR DE VALIDAÇÃO ---
    // Agora ele sabe filtrar campos irrelevantes
    fn validate_custom_data(
        &self,
        definitions: &[FieldDefinition],
        data: &Value,
        selected_types: &[Uuid],
    ) -> Result<(), AppError> {

        let obj = data.as_object().ok_or_else(|| {
            // Erro genérico de formato
            AppError::CustomDataJson
        })?;

        // Mapa de erros: Chave do campo -> Código do erro
        let mut errors: HashMap<String, String> = HashMap::new();

        for def in definitions {
            // A. FILTRAGEM (Sua lógica estava perfeita aqui)
            if let Some(type_id) = def.entity_type_id {
                if !selected_types.contains(&type_id) {
                    continue;
                }
            }

            let value = obj.get(&def.key_name);

            // B. VALIDAÇÃO DE OBRIGATORIEDADE
            // Se for obrigatório E (não existe OU é null)
            if def.is_required && (value.is_none() || value.map_or(true, |v| v.is_null())) {
                // Usamos CÓDIGO, não frase
                errors.insert(def.key_name.clone(), "required".to_string());
                continue; // Se já falhou aqui, pula pro próximo campo
            }

            // C. VALIDAÇÃO DE TIPO
            if let Some(val) = value {
                if !val.is_null() {
                    let valid = match def.field_type {
                        FieldType::Number => val.is_number(),
                        FieldType::Boolean => val.is_boolean(),
                        FieldType::Multiselect => val.is_array(),
                        FieldType::Text | FieldType::Select => val.is_string(),

                        // Validação REAL de data
                        FieldType::Date => {
                            val.is_string() &&
                                NaiveDate::parse_from_str(val.as_str().unwrap(), "%Y-%m-%d").is_ok()
                        }
                    };

                    if !valid {
                        // Define o código de erro baseado no tipo esperado
                        let error_code = match def.field_type {
                            FieldType::Number => "invalid_number",
                            FieldType::Date => "invalid_date_format", // Espera YYYY-MM-DD
                            FieldType::Boolean => "invalid_boolean",
                            FieldType::Multiselect => "invalid_list",
                            _ => "invalid_text",
                        };
                        errors.insert(def.key_name.clone(), error_code.to_string());
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(AppError::CustomDataValidationError(errors));
        }

        Ok(())
    }

    // Helper para criar erro de validação
    fn validation_error(&self, field: &str, message: &str) -> AppError {
        let mut err = validator::ValidationErrors::new();
        let mut validation_err = validator::ValidationError::new("invalid_type");
        validation_err.message = Some(message.to_string().into());

        // Leak seguro para erro estático
        let static_field: &'static str = Box::leak(field.to_string().into_boxed_str());
        err.add(static_field, validation_err);

        AppError::ValidationError(err)
    }

    pub async fn find_companies_by_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<UserCompany>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.find_companies_by_user(executor, user_id).await
    }

    pub async fn list_customers<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<Customer>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.list_customers(executor, tenant_id).await
    }
}