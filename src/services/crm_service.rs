// src/services/crm_service.rs

use serde_json::Value;
use sqlx::{PgPool, Postgres, Executor, Acquire};
use uuid::Uuid;
use chrono::NaiveDate;

use crate::{
    common::error::AppError,
    db::CrmRepository,
    models::crm::{CrmFieldDefinition, CrmFieldType, Customer},
};
use crate::models::auth::DocumentType;
#[derive(Clone)]
pub struct CrmService {
    repo: CrmRepository,
}

impl CrmService {
    pub fn new(repo: CrmRepository) -> Self {
        Self { repo }
    }

    // --- CONFIGURAÇÃO (DEFINIÇÕES DE CAMPO) ---

    pub async fn create_field_definition<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        key_name: &str,
        field_type: CrmFieldType,
        options: Option<Value>,
        is_required: bool,
    ) -> Result<CrmFieldDefinition, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Aqui poderíamos validar se 'options' é válido caso o tipo seja SELECT
        // Por enquanto, apenas repassa.
        self.repo.create_field_definition(
            executor, tenant_id, name, key_name, field_type, options.as_ref(), is_required
        ).await
    }

    pub async fn list_field_definitions<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<CrmFieldDefinition>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.list_field_definitions(executor, tenant_id).await
    }

    // --- CLIENTES (COM VALIDAÇÃO DINÂMICA) ---

    pub async fn create_customer<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        full_name: &str,
        // [NOVOS ARGUMENTOS AQUI]
        country_code: Option<&str>,
        document_type: Option<DocumentType>,
        document_number: Option<&str>,
        // -----------------------
        birth_date: Option<NaiveDate>,
        email: Option<&str>,
        phone: Option<&str>,
        mobile: Option<&str>,
        address: Option<Value>,
        tags: Option<Vec<String>>,
        custom_data: Value,
    ) -> Result<Customer, AppError>
    where
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        let mut tx = executor.begin().await?;

        let definitions = self.repo.list_field_definitions(&mut *tx, tenant_id).await?;
        self.validate_custom_data(&definitions, &custom_data)?;

        let tags_slice = tags.as_deref();

        // [CHAMADA ATUALIZADA AO REPOSITÓRIO]
        let customer = self.repo.create_customer(
            &mut *tx,
            tenant_id,
            full_name,
            country_code,    // Passando o novo arg
            document_type,   // Passando o novo arg
            document_number,
            birth_date,
            email,
            phone,
            mobile,
            address.as_ref(),
            tags_slice,
            &custom_data
        ).await?;

        tx.commit().await?;

        Ok(customer)
    }

    // --- MOTOR DE VALIDAÇÃO (Privado) ---
    // Aqui acontece a mágica de garantir tipagem em dados flexíveis.
    fn validate_custom_data(
        &self,
        definitions: &[CrmFieldDefinition],
        data: &Value
    ) -> Result<(), AppError> {

        let obj = data.as_object().ok_or_else(|| {
            let mut err = validator::ValidationErrors::new();
            err.add("customData", validator::ValidationError::new("Must be a JSON object"));
            AppError::ValidationError(err)
        })?;

        for def in definitions {
            let value = obj.get(&def.key_name);

            if def.is_required && (value.is_none() || value.unwrap().is_null()) {
                return Err(self.validation_error(&def.key_name, &format!("O campo personalizado '{}' é obrigatório.", def.name)));
            }

            if let Some(val) = value {
                if !val.is_null() {
                    match def.field_type {
                        CrmFieldType::Number => {
                            if !val.is_number() {
                                return Err(self.validation_error(&def.key_name, &format!("O campo '{}' deve ser numérico.", def.name)));
                            }
                        },
                        CrmFieldType::Boolean => {
                            if !val.is_boolean() {
                                return Err(self.validation_error(&def.key_name, &format!("O campo '{}' deve ser verdadeiro/falso.", def.name)));
                            }
                        },
                        CrmFieldType::Text | CrmFieldType::Date | CrmFieldType::Select => {
                            if !val.is_string() {
                                return Err(self.validation_error(&def.key_name, &format!("O campo '{}' deve ser um texto.", def.name)));
                            }
                        },
                        CrmFieldType::Multiselect => {
                            if !val.is_array() {
                                return Err(self.validation_error(&def.key_name, &format!("O campo '{}' deve ser uma lista.", def.name)));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Helper para criar erro rápido
    fn validation_error(&self, field: &str, message: &str) -> AppError {
        let mut err = validator::ValidationErrors::new();
        let mut validation_err = validator::ValidationError::new("invalid_type");
        validation_err.message = Some(message.to_string().into());

        // O TRUQUE: Vazamos a string para a memória estática.
        // Como o nome dos campos é limitado e não muda a cada request, o impacto é mínimo.
        let static_field: &'static str = Box::leak(field.to_string().into_boxed_str());

        err.add(static_field, validation_err);

        AppError::ValidationError(err)
    }
}