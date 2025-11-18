// src/models/tenancy.rs

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

// ---
// 1. Tenant (O "Estabelecimento")
// ---
// A conta principal (Loja, Restaurante, Academia)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---
// 2. UserTenant (A "Ponte" Usuário-Tenant)
// ---
// Liga um Usuário a um Estabelecimento
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserTenant {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// ---
// 3. StockPool (A "Piscina de Estoque")
// ---
// O grupo de locais que partilham visibilidade de estoque
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct StockPool {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---
// 4. Location (O "Local")
// ---
// O local físico do estoque (Loja A, Loja B, Barracão)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: Uuid,
    pub tenant_id: Uuid,

    // ATUALIZADO: Agora liga-se ao StockPool
    pub stock_pool_id: Uuid,

    pub name: String,
    // 'true' para o seu "Barracão" (apenas estoque, sem vendas)
    pub is_warehouse: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}