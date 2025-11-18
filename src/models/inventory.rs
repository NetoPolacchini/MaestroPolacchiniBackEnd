// src/models/inventory.rs

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

// --- 1. Unidades de Medida (ATUALIZADO) ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UnitOfMeasure {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub symbol: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- 2. Categorias (ATUALIZADO) ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- 3. Itens / Produtos (ATUALIZADO) ---
// Esta struct é agora apenas o "catálogo" de produtos.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub category_id: Uuid,
    pub base_unit_id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- 4. Nível de Estoque (NOVO) ---
// Esta é a nova struct "Saldo". Ela liga um Item a um Local.
// Ela representa a tabela 'inventory_levels'.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InventoryLevel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub item_id: Uuid,
    pub location_id: Uuid,
    pub quantity: Decimal,
    pub low_stock_threshold: Decimal,
    pub updated_at: DateTime<Utc>,
}


// --- 5. Movimentações de Estoque (ATUALIZADO) ---

// MUDANÇA: Adicionado TRANSFER_OUT e TRANSFER_IN
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "stock_movement_reason", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum StockMovementReason {
    InitialStock,
    Sale,
    Return,
    Delivery,
    Spoilage,
    Correction,
    TransferOut,
    TransferIn,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub item_id: Uuid,
    pub location_id: Uuid,
    pub quantity_changed: Decimal,
    pub reason: StockMovementReason,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}