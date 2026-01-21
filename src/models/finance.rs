// src/models/finance.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// --- Enums (Mapeando o Postgres) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)] // <--- ToSchema
#[sqlx(type_name = "title_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TitleKind {
    Receivable, // A Receber
    Payable,    // A Pagar
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)] // <--- ToSchema
#[sqlx(type_name = "title_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TitleStatus {
    Pending,   // Aberto
    Partial,   // Pago Parcialmente
    Paid,      // Quitado
    Cancelled, // Cancelado
    Overdue,   // Vencido
}

// --- Structs ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct FinancialAccount {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    #[schema(example = "Conta Principal")]
    pub name: String,

    #[schema(example = "Banco do Brasil")]
    pub bank_name: Option<String>,

    #[schema(example = "1500.50")]
    pub current_balance: Decimal,

    #[schema(example = true)]
    pub is_active: Option<bool>,

    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct FinancialCategory {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    #[schema(example = "Venda de Produtos")]
    pub name: String,

    pub kind: TitleKind,

    #[schema(example = true)]
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct FinancialTitle {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    #[schema(example = "Pagamento Fornecedor XYZ")]
    pub description: String,

    // Vínculos
    pub customer_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub category_id: Option<Uuid>,

    pub kind: TitleKind,
    pub status: TitleStatus,

    // Valores
    #[schema(example = "500.00")]
    pub amount_original: Decimal,
    #[schema(example = "500.00")]
    pub amount_balance: Decimal, // Quanto falta pagar

    // Datas
    #[schema(value_type = String, format = Date, example = "2023-12-31")]
    pub due_date: NaiveDate,
    #[schema(value_type = String, format = Date, example = "2023-12-01")]
    pub competence_date: NaiveDate,

    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct FinancialMovement {
    pub id: Uuid,

    #[schema(ignore)]
    pub tenant_id: Uuid,

    pub account_id: Uuid,
    pub title_id: Option<Uuid>,

    //ToDo: Verificar esse outro example here
    #[schema(example = "-150.00", example = "Positivo = Entrada, Negativo = Saída")]
    pub amount: Decimal,

    #[schema(value_type = String, format = Date, example = "2023-12-20")]
    pub movement_date: NaiveDate,

    pub created_at: Option<DateTime<Utc>>,
}