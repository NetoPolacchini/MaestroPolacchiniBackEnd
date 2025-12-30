// src/models/operations.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// --- Enums ---

// Categoria do Sistema (O Backend só entende isso)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "pipeline_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PipelineCategory {
    Draft,     // Orçamento / Rascunho
    Active,    // Em Andamento / Produção
    Done,      // Concluído / Entregue
    Cancelled, // Cancelado
}

// --- Structs de Configuração (O Workflow) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub is_default: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub pipeline_id: Uuid,

    pub name: String,
    pub category: PipelineCategory,

    pub position: i32,
    pub color: Option<String>,

    // === GATILHOS ===
    // 'NONE', 'RESERVE', 'DEDUCT'
    pub stock_action: Option<String>,
    pub generates_receivable: Option<bool>,
    pub is_locked: Option<bool>,
}

// --- Structs de Operação (O Pedido) ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,

    // Onde ele está?
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,

    pub display_id: i32, // Serial (Ex: 1045)

    // Cache Financeiro
    pub total_amount: Decimal,
    pub total_discount: Decimal,

    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,

    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub order_id: Uuid,
    pub item_id: Uuid,

    // Snapshot do momento da venda
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub unit_cost: Decimal,
    pub discount: Decimal,

    pub notes: Option<String>,
}

// DTO para retornar o Pedido Completo (Com Itens e Cliente)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetail {
    #[serde(flatten)]
    pub header: Order,

    pub customer_name: Option<String>, // Join com customer
    pub stage_name: String,            // Join com stage
    pub stage_category: PipelineCategory,

    pub items: Vec<OrderItemDetail>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemDetail {
    pub id: Uuid,
    pub item_id: Uuid,
    pub item_name: String, // Join com items
    pub sku: String,

    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total: Decimal, // Calculado (qtd * preço - desc)
}