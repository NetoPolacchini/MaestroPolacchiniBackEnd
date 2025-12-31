// src/models/dashboard.rs

use serde::{Serialize};
use rust_decimal::Decimal;
use sqlx::FromRow;

// 1. Resumo do Dia (Os Cards do Topo)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub sales_today: Decimal,       // Vendas finalizadas hoje
    pub receivables_today: Decimal, // Boletos que vencem hoje (A receber)
    pub payables_today: Decimal,    // Boletos que vencem hoje (A pagar)
    pub current_balance: Decimal,   // Saldo somado de todos os bancos
}

// 2. Gráfico de Vendas (Últimos 30 dias)
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SalesChartEntry {
    pub date: Option<String>, // O SQL pode retornar data como string (YYYY-MM-DD)
    pub total: Option<Decimal>,
}

// 3. Curva ABC (Top Produtos)
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TopProductEntry {
    pub item_name: String,
    pub total_quantity: Option<Decimal>,
    pub total_revenue: Option<Decimal>,
}