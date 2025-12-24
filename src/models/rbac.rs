// src/models/rbac.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc};

// O que sai do banco (Tabela Roles)
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at:Option<DateTime<Utc>>,//Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// O que sai do banco (Tabela Permissions)
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub id: Uuid,
    pub slug: String,       // "inventory:read"
    pub description: String, // "Visualizar itens"
    pub module: String,      // "INVENTORY"
}

// O Payload para criar um cargo
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRolePayload {
    pub name: String,             // "Gerente"
    pub description: Option<String>,
    pub permissions: Vec<String>, // ["inventory:read", "crm:write"]
}

// Resposta completa (Cargo + Lista de Permissões)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleResponse {
    #[serde(flatten)] // Mescla os campos do Role no JSON raiz
    pub role: Role,
    pub permissions: Vec<String>, // Devolvemos os slugs para confirmar
}