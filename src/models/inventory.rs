// src/models/inventory.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json; // Necessário para exemplos de JSON
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// --- Enums (Mapeamento do Postgres) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)] // <--- ToSchema
#[sqlx(type_name = "item_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemKind {
    Product,  // Físico
    Service,  // Intangível
    Resource, // Alocável (Mesa, Sala)
    Bundle,   // Virtual/Combo
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)] // <--- ToSchema
#[sqlx(type_name = "composition_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompositionType {
    Component,  // Faz parte (baixa estoque)
    Accessory,  // Acompanha (não essencial)
    Substitute, // Opção de troca
}

// --- Structs Principais ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    #[schema(ignore)] // Ignorar tenant_id
    pub tenant_id: Uuid,

    #[schema(example = "PROD-001")]
    pub sku: String,
    #[schema(example = "Coca-Cola 350ml")]
    pub name: String,
    #[schema(example = "Refrigerante de cola em lata")]
    pub description: Option<String>,

    // [NOVO] Tipo e Configurações
    pub kind: ItemKind,

    #[schema(example = json!({"cor": "vermelho", "peso": "350g"}))]
    pub settings: Option<serde_json::Value>, // JSON Flexível

    pub unit_id: Uuid,
    pub category_id: Option<Uuid>,

    // Mantemos o preço base (venda) e custo
    #[schema(example = "2.50")]
    pub cost_price: Option<Decimal>,
    #[schema(example = "5.00")]
    pub sale_price: Decimal,

    // Estoque atual
    #[schema(example = "150")]
    pub current_stock: Decimal,
    #[schema(example = "10")]
    pub min_stock: Option<Decimal>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- Struct de Composição (A Ficha Técnica) ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct CompositionEntry {
    pub id: Uuid,

    // Dados do Item Filho (Join)
    pub child_item_id: Uuid,
    #[schema(example = "ING-SUGAR")]
    pub child_sku: String,
    #[schema(example = "Açúcar Refinado")]
    pub child_name: String,
    #[schema(example = "kg")]
    pub child_unit: String,

    #[schema(example = "0.100")]
    pub quantity: Decimal,
    pub comp_type: CompositionType,
}

// --- 2. Categorias ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub parent_id: Option<Uuid>,
    #[schema(example = "Bebidas")]
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct InventoryLevel {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub item_id: Uuid,
    pub location_id: Uuid,

    #[schema(example = "100.0")]
    pub quantity: Decimal, // Quantidade FÍSICA total

    // [NOVO] Quantidade Reservada
    #[schema(example = "5.0")]
    pub reserved_quantity: Decimal,

    // [NOVO] Financeiro
    #[schema(example = "10.00")]
    pub sale_price: Option<Decimal>,
    #[schema(example = "4.50")]
    pub average_cost: Decimal,

    #[schema(example = "10.0")]
    pub low_stock_threshold: Decimal,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, ToSchema)] // <--- ToSchema
#[sqlx(type_name = "stock_movement_reason", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StockMovementReason {
    InitialStock,
    Purchase,
    Sale,
    Return,
    Delivery,
    Spoilage,
    Correction,
    TransferOut,
    TransferIn,
}

// --- STOCK MOVEMENT (Histórico) ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub item_id: Uuid,
    pub location_id: Uuid,
    #[schema(example = "10.0")]
    pub quantity_changed: Decimal,
    pub reason: StockMovementReason,
    #[schema(example = "Prateleira A2")]
    pub position: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

// --- 1. Unidades de Medida ---
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct UnitOfMeasure {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    #[schema(example = "Quilograma")]
    pub name: String,
    #[schema(example = "kg")]
    pub symbol: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct InventoryBatch {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub item_id: Uuid,
    pub location_id: Uuid,
    #[schema(example = "LOTE-2023-A")]
    pub batch_number: String,
    #[schema(example = "A1")]
    pub position: String,
    pub expiration_date: Option<NaiveDate>,
    #[schema(example = "50.0")]
    pub quantity: Decimal,
    pub unit_cost: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}