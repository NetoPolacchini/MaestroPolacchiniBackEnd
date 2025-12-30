// src/models/crm.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
// REMOVIDO: use sqlx::types::Json; -> Não precisamos mais deste wrapper para Value puro
use chrono::{DateTime, Utc, NaiveDate};

// [CORREÇÃO 1] Importamos o DocumentType do módulo auth para evitar duplicação
use crate::models::auth::DocumentType;

// --- Enums ---

// FieldType é específico do CRM, mantemos aqui.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "crm_field_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldType {
    Text,
    Number,
    Date,
    Boolean,
    Select,
    Multiselect,
}

// REMOVIDO: DocumentType (Agora usamos o de auth)

// --- Structs de Configuração (O Molde) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EntityType {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefinition {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub entity_type_id: Option<Uuid>,
    pub name: String,
    pub key_name: String,
    pub field_type: FieldType,

    // [CORREÇÃO 2] JSONB mapeia direto para Option<serde_json::Value>
    pub options: Option<serde_json::Value>,

    pub is_required: bool,
}

// --- Customer (O Dado) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,

    pub full_name: String,
    pub email: Option<String>,

    pub phone: Option<String>,
    pub mobile: Option<String>,

    pub country_code: Option<String>,

    // [CORREÇÃO 1] Usa o Enum unificado
    pub document_type: Option<DocumentType>,

    pub document_number: Option<String>,

    pub birth_date: Option<NaiveDate>,

    // [CORREÇÃO 2] JSONB -> serde_json::Value direto
    pub address: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,

    pub entity_types: Option<Vec<Uuid>>,

    // [CORREÇÃO 2] JSONB -> serde_json::Value (Option pois o SQL infere que pode ser nulo)
    // Usamos 'custom_data' puro, sem Json<...>
    pub custom_data: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}