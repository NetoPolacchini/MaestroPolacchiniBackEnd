// src/models/crm.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc, NaiveDate};
use serde_json::json; // Para exemplos
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// [CORREÇÃO 1] Importamos o DocumentType do módulo auth para evitar duplicação
use crate::models::auth::DocumentType;

// --- Enums ---

// FieldType é específico do CRM, mantemos aqui.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, ToSchema)] // <--- ToSchema
#[sqlx(type_name = "crm_field_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldType {
    Text,
    Number,
    Date,
    Boolean,
    Select,
    Multiselect,
}

// --- Structs de Configuração (O Molde) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct EntityType {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    #[schema(example = "Cliente VIP")]
    pub name: String,
    #[schema(example = "cliente-vip")]
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct FieldDefinition {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub entity_type_id: Option<Uuid>,
    #[schema(example = "Tamanho da Camiseta")]
    pub name: String,
    #[schema(example = "tamanho_camiseta")]
    pub key_name: String,
    pub field_type: FieldType,

    // JSONB mapeia direto para Option<serde_json::Value>
    #[schema(example = json!(["P", "M", "G", "GG"]))]
    pub options: Option<serde_json::Value>,

    pub is_required: bool,
}

// --- Customer (O Dado) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct Customer {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440002")]
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub user_id: Option<Uuid>,

    #[schema(example = "João da Silva")]
    pub full_name: String,
    #[schema(example = "joao@email.com")]
    pub email: Option<String>,

    #[schema(example = "(11) 3333-4444")]
    pub phone: Option<String>,
    #[schema(example = "(11) 99999-8888")]
    pub mobile: Option<String>,

    #[schema(example = "BR")]
    pub country_code: Option<String>,

    // Usa o Enum unificado de Auth (que já tem ToSchema)
    pub document_type: Option<DocumentType>,

    #[schema(example = "12345678900")]
    pub document_number: Option<String>,

    #[schema(value_type = Option<String>, format = Date, example = "1990-01-01")]
    pub birth_date: Option<NaiveDate>,

    // Endereço como JSON
    #[schema(example = json!({
        "rua": "Rua das Flores",
        "numero": "123",
        "cidade": "São Paulo",
        "estado": "SP",
        "cep": "01000-000"
    }))]
    pub address: Option<serde_json::Value>,

    #[schema(example = json!(["novo", "potencial"]))]
    pub tags: Option<Vec<String>>,

    pub entity_types: Option<Vec<Uuid>>,

    // Campos Customizados (A força do CRM)
    #[schema(example = json!({
        "tamanho_camiseta": "M",
        "preferencia_cor": "Azul"
    }))]
    pub custom_data: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}