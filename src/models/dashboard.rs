// src/models/dashboard.rs

use serde::{Serialize};
use rust_decimal::Decimal;
use sqlx::FromRow;
use utoipa::ToSchema; // <--- 1. Importe o ToSchema

// 1. Resumo do Dia (Os Cards do Topo)
#[derive(Debug, Serialize, ToSchema)] // <--- 2. Adicione ToSchema
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    #[schema(example = "1500.00")]
    pub sales_today: Decimal,       // Vendas finalizadas hoje

    #[schema(example = "500.50")]
    pub receivables_today: Decimal, // Boletos que vencem hoje (A receber)

    #[schema(example = "200.00")]
    pub payables_today: Decimal,    // Boletos que vencem hoje (A pagar)

    #[schema(example = "12500.00")]
    pub current_balance: Decimal,   // Saldo somado de todos os bancos
}

// 2. Gráfico de Vendas (Últimos 30 dias)
#[derive(Debug, Serialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct SalesChartEntry {
    #[schema(example = "2023-10-25")]
    pub date: Option<String>, // O SQL pode retornar data como string (YYYY-MM-DD)

    #[schema(example = "3500.00")]
    pub total: Option<Decimal>,
}

// 3. Curva ABC (Top Produtos)
#[derive(Debug, Serialize, FromRow, ToSchema)] // <--- ToSchema
#[serde(rename_all = "camelCase")]
pub struct TopProductEntry {
    #[schema(example = "Coca-Cola 2L")]
    pub item_name: String,

    #[schema(example = "150.0")]
    pub total_quantity: Option<Decimal>,

    #[schema(example = "1200.00")]
    pub total_revenue: Option<Decimal>,
}