// src/models/crm.rs

use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use serde_json::Value; // <--- A chave para o JSONB
use crate::models::auth::DocumentType; // Ajuste conforme seu caminho de arquivo

// --- ENUMS ---

// Mapeia o CREATE TYPE crm_field_type do banco
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "crm_field_type", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum CrmFieldType {
    Text,
    Number,
    Date,
    Boolean,
    Select,
    Multiselect,
}

// --- DEFINIÇÕES (O Molde) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CrmFieldDefinition {
    pub id: Uuid,
    pub tenant_id: Uuid,

    pub name: String,      // Ex: "Peso"
    pub key_name: String,  // Ex: "weight"

    pub field_type: CrmFieldType,

    // Opções para Selects (Ex: ["A", "B"]).
    // Usamos 'Value' porque pode ser um array de strings ou objetos.
    pub options: Option<Value>,

    pub is_required: bool,
    pub created_at: DateTime<Utc>,
}

// --- CLIENTE (O Dado) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: Uuid,
    pub tenant_id: Uuid,

    pub user_id: Option<Uuid>,

    pub full_name: String,
    pub birth_date: Option<NaiveDate>,

    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,

    // Endereço flexível.
    // O Frontend manda um JSON, o Rust guarda como JSONB.
    pub address: Option<Value>,

    // Tags simples (Array de Strings)
    // No Postgres é TEXT[], no Rust é Vec<String>
    pub tags: Option<Vec<String>>,

    // CAMPOS PERSONALIZADOS
    // Aqui vai o { "weight": 80, "team": "Flamengo" }
    pub custom_data: Value,

    // [ATUALIZADO] Identidade Global
    pub country_code: Option<String>,
    pub document_type: Option<DocumentType>,
    pub document_number: Option<String>, // O antigo era só document_number

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}