// src/models/operations.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use utoipa::ToSchema; // <--- 1. IMPORTANTE: Adicione este import

// --- Enums ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)] // <--- 2. Adicione ToSchema
#[sqlx(type_name = "pipeline_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PipelineCategory {
    Draft,
    Active,
    Done,
    Cancelled,
}

// --- Structs de Configuração ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")] // Exemplo de UUID
    pub id: Uuid,
    #[schema(ignore)] // Geralmente ocultamos o tenant_id da doc pública, pois vem do token
    pub tenant_id: Uuid,
    #[schema(example = "Funil de Vendas Padrão")]
    pub name: String,
    #[schema(example = true)]
    pub is_default: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub id: Uuid,
    #[schema(ignore)]
    pub tenant_id: Uuid,
    pub pipeline_id: Uuid,
    #[schema(example = "Em Negociação")]
    pub name: String,
    pub category: PipelineCategory,
    #[schema(example = 1)]
    pub position: i32,
    #[schema(example = "#FF5733")]
    pub color: Option<String>,
    #[schema(example = "RESERVE")]
    pub stock_action: Option<String>,
    pub generates_receivable: Option<bool>,
    pub is_locked: Option<bool>,
}

// --- Structs de Operação ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: Uuid,
    #[schema(ignore)]
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,
    #[schema(example = 1024)]
    pub display_id: i32,
    #[schema(example = "150.50")]
    pub total_amount: Decimal,
    #[schema(example = "10.00")]
    pub total_discount: Decimal,
    #[schema(example = json!(["urgente", "vip"]))]
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub id: Uuid,
    #[schema(ignore)]
    pub tenant_id: Uuid,
    pub order_id: Uuid,
    pub item_id: Uuid,
    #[schema(example = "2.0")]
    pub quantity: Decimal,
    #[schema(example = "50.00")]
    pub unit_price: Decimal,
    #[schema(example = "30.00")]
    pub unit_cost: Decimal,
    #[schema(example = "0.0")]
    pub discount: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Como você tinha a OrderDetail no operations.rs antes (conforme conversas anteriores),
// se ela ainda existir aí, adicione ToSchema nela também. Se não, ignore.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetail {
    #[serde(flatten)]
    pub header: Order,
    pub customer_name: Option<String>,
    pub stage_name: String,
    pub stage_category: PipelineCategory,
    pub items: Vec<OrderItem>,
}