// src/models/operations.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// --- Enums ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "pipeline_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PipelineCategory {
    Draft,
    Active,
    Done,
    Cancelled,
}

// --- Structs de Configuração ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub is_default: Option<bool>,
    // [CORREÇÃO] O banco pode retornar null ou o sqlx infere nullable
    pub created_at: Option<DateTime<Utc>>,
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
    pub stock_action: Option<String>,
    pub generates_receivable: Option<bool>,
    pub is_locked: Option<bool>,
}

// --- Structs de Operação ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,
    pub display_id: i32,
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
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub unit_cost: Decimal,
    pub discount: Decimal,
    pub notes: Option<String>,

    // [CORREÇÃO] Faltava este campo que o SELECT * trazia
    pub created_at: DateTime<Utc>,
}