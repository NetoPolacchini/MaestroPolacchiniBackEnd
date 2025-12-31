// src/models/finance.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;

// --- Enums (Mapeando o Postgres) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "title_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TitleKind {
    Receivable, // A Receber
    Payable,    // A Pagar
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "title_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TitleStatus {
    Pending,   // Aberto
    Partial,   // Pago Parcialmente
    Paid,      // Quitado
    Cancelled, // Cancelado
    Overdue,   // Vencido (Status lógico, no banco as vezes fica Pending mas a data já passou)
}

// --- Structs ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FinancialAccount {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub bank_name: Option<String>,
    pub current_balance: Decimal,
    pub is_active: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FinancialCategory {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub kind: TitleKind,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FinancialTitle {
    pub id: Uuid,
    pub tenant_id: Uuid,

    pub description: String,

    // Vínculos
    pub customer_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub category_id: Option<Uuid>,

    pub kind: TitleKind,
    pub status: TitleStatus,

    // Valores
    pub amount_original: Decimal,
    pub amount_balance: Decimal, // Quanto falta pagar

    // Datas (NaiveDate pq boleto não tem Timezone)
    pub due_date: NaiveDate,
    pub competence_date: NaiveDate,

    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FinancialMovement {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub title_id: Option<Uuid>,

    pub amount: Decimal, // Positivo = Entrada, Negativo = Saída

    pub movement_date: NaiveDate,
    pub created_at: Option<DateTime<Utc>>,
}