// src/models/inventory.rs

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, NaiveDate};
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
    pub default_price: Option<Decimal>,
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

    pub quantity: Decimal, // Quantidade FÍSICA total

    // [NOVO] Quantidade Reservada
    pub reserved_quantity: Decimal,

    // [NOVO] Financeiro
    pub sale_price: Option<Decimal>, // Preço de Venda na Loja
    pub average_cost: Decimal,       // Custo Médio Unitário

    pub low_stock_threshold: Decimal,
    pub updated_at: DateTime<Utc>,
}


// --- 5. Movimentações de Estoque (ATUALIZADO) ---

// MUDANÇA: Adicionado TRANSFER_OUT e TRANSFER_IN
// src/models/inventory.rs

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "stock_movement_reason", rename_all = "SCREAMING_SNAKE_CASE")] // Banco
#[serde(rename_all = "SCREAMING_SNAKE_CASE")] // JSON
pub enum StockMovementReason {
    InitialStock, // Vira "INITIAL_STOCK"
    Purchase,     // Vira "PURCHASE"
    Sale,         // Vira "SALE"
    Return,       // Vira "RETURN"
    Delivery,
    Spoilage,
    Correction,
    TransferOut,  // Vira "TRANSFER_OUT"
    TransferIn,   // Vira "TRANSFER_IN"
}

// --- STOCK MOVEMENT (Histórico) ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub item_id: Uuid,
    pub location_id: Uuid,
    pub quantity_changed: Decimal,
    pub reason: StockMovementReason,
    pub position: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InventoryBatch {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub item_id: Uuid,
    pub location_id: Uuid,
    pub batch_number: String,
    pub position: String,
    pub expiration_date: Option<NaiveDate>, // Data simples (Dia/Mês/Ano)
    pub quantity: Decimal,
    pub unit_cost: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}