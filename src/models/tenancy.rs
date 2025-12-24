// src/models/tenancy.rs

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

// ---
// 1. Tenant (O "Estabelecimento")
// ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String, // Já está correto aqui
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---
// 2. TenantMember (A Nova "Ponte" com Cargos)
// ---
// Substitui o antigo UserTenant para suportar RBAC
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TenantMember {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid, // <--- O campo crucial
    pub is_active: bool,
    pub joined_at: Option<DateTime<Utc>>,
}

// ---
// 3. StockPool
// ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct StockPool {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---
// 4. Location
// ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub stock_pool_id: Uuid,
    pub name: String,
    pub is_warehouse: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}