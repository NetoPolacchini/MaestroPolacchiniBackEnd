// src/models/rbac.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use serde_json::json;
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// O que sai do banco (Tabela Roles)
#[derive(Debug, Serialize, FromRow, ToSchema)] // <--- 2. Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct Role {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    #[schema(ignore)] // Ocultamos tenant_id da documentação pública
    pub tenant_id: Uuid,

    #[schema(example = "Gerente de Vendas")]
    pub name: String,

    #[schema(example = "Acesso completo ao módulo de vendas e CRM")]
    pub description: Option<String>,

    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// O que sai do banco (Tabela Permissions)
#[derive(Debug, Serialize, FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct Permission {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub id: Uuid,

    #[schema(example = "inventory:read")]
    pub slug: String,

    #[schema(example = "Visualizar itens de estoque")]
    pub description: String,

    #[schema(example = "INVENTORY")]
    pub module: String,
}

// O Payload para criar um cargo
#[derive(Debug, Deserialize, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct CreateRolePayload {
    #[schema(example = "Auxiliar de Estoque")]
    pub name: String,

    #[schema(example = "Pode apenas visualizar produtos e dar entrada em notas")]
    pub description: Option<String>,

    #[schema(example = json!(["inventory:read", "inventory:write"]))]
    pub permissions: Vec<String>, // Slugs das permissões
}

// Resposta completa (Cargo + Lista de Permissões)
#[derive(Debug, Serialize, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct RoleResponse {
    #[serde(flatten)]
    pub role: Role,

    #[schema(example = json!(["inventory:read", "inventory:write"]))]
    pub permissions: Vec<String>,
}