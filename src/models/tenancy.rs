// src/models/tenancy.rs

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// ---
// 1. Tenant (O "Estabelecimento")
// ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    #[schema(example = "Minha Loja de Roupas")]
    pub name: String,

    #[schema(example = "minha-loja-roupas")]
    pub slug: String,

    #[schema(example = "Matriz localizada no centro.")]
    pub description: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---
// 2. TenantMember (A Nova "Ponte" com Cargos)
// ---
// Substitui o antigo UserTenant para suportar RBAC
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct TenantMember {
    pub id: Uuid,

    #[schema(ignore)] // Geralmente oculto pois depende do contexto
    pub tenant_id: Uuid,

    pub user_id: Uuid,

    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub role_id: Uuid,

    #[schema(example = true)]
    pub is_active: bool,

    pub joined_at: Option<DateTime<Utc>>,
}

// ---
// 3. StockPool (Centro de Estoque Lógico)
// ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct StockPool {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    #[schema(example = "Estoque Central")]
    pub name: String,

    #[schema(example = "Armazém principal de produtos acabados")]
    pub description: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---
// 4. Location (Localização Física/Prateleira)
// ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub stock_pool_id: Uuid,

    #[schema(example = "Prateleira A1")]
    pub name: String,

    //ToDo: dar uma olhada nesse segundo example, ele deveria ser um description
    #[schema(example = true, example = "Se é um armazém (true) ou ponto de venda (false)")]
    pub is_warehouse: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}