// src/models/inventory.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal; // Certifique-se de ter essa lib no Cargo.toml

// --- Enums (Mapeamento do Postgres) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "item_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemKind {
    Product,  // Físico
    Service,  // Intangível
    Resource, // Alocável (Mesa, Sala)
    Bundle,   // Virtual/Combo
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "composition_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompositionType {
    Component,  // Faz parte (baixa estoque)
    Accessory,  // Acompanha (não essencial)
    Substitute, // Opção de troca
}

// --- Structs Principais ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: Uuid,
    pub tenant_id: Uuid,

    pub sku: String,
    pub name: String,
    pub description: Option<String>,

    // [NOVO] Tipo e Configurações
    pub kind: ItemKind,
    pub settings: Option<serde_json::Value>, // JSON Flexível

    pub unit_id: Uuid,
    pub category_id: Option<Uuid>,

    // Mantemos o preço base (venda) e custo
    pub cost_price: Option<Decimal>,
    pub sale_price: Decimal,

    // Estoque atual (apenas informativo, calculado via transactions/inventory_levels)
    pub current_stock: Decimal,
    pub min_stock: Option<Decimal>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- Struct de Composição (A Ficha Técnica) ---
// Usada para retornar os ingredientes de um produto na API
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CompositionEntry {
    pub id: Uuid, // ID da relação

    // Dados do Item Filho (Join)
    pub child_item_id: Uuid,
    pub child_sku: String,
    pub child_name: String,
    pub child_unit: String, // Símbolo da unidade (Ex: "kg", "un")

    pub quantity: Decimal,
    pub comp_type: CompositionType,
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