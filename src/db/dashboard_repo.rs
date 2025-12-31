// src/db/dashboard_repo.rs

use sqlx::{PgPool, Postgres, Executor, Acquire};
use uuid::Uuid;
use rust_decimal::Decimal;
use crate::{
    common::error::AppError,
    models::dashboard::{DashboardSummary, SalesChartEntry, TopProductEntry},
};

#[derive(Clone)]
pub struct DashboardRepository {
    pool: PgPool,
}

impl DashboardRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // 1. Resumo Geral
    pub async fn get_summary<'e, E>(
        &self,
        executor: E, // Removemos o 'mut' do argumento se houver, o begin resolve
        tenant_id: Uuid,
    ) -> Result<DashboardSummary, AppError>
    where
    // REMOVIDO: + Copy
    // ADICIONADO: + Acquire
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        // Iniciamos uma transação (Snapshot consistente dos dados)
        let mut tx = executor.begin().await?;

        // A. Vendas de Hoje
        // Note o uso de &mut *tx em todas as chamadas abaixo
        let sales_today = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(o.total_amount), 0) as total
            FROM orders o
            JOIN pipeline_stages s ON o.stage_id = s.id
            WHERE o.tenant_id = $1
              AND s.category = 'DONE'
              AND o.closed_at::date = CURRENT_DATE
            "#,
            tenant_id
        )
            .fetch_one(&mut *tx) // <--- Use a transação
            .await?
            .total.unwrap_or(Decimal::ZERO);

        // B. A Receber Hoje
        let receivables_today = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(amount_balance), 0) as total
            FROM financial_titles
            WHERE tenant_id = $1
              AND kind = 'RECEIVABLE'
              AND status IN ('PENDING', 'PARTIAL')
              AND due_date = CURRENT_DATE
            "#,
            tenant_id
        )
            .fetch_one(&mut *tx) // <--- Use a transação
            .await?
            .total.unwrap_or(Decimal::ZERO);

        // C. A Pagar Hoje
        let payables_today = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(amount_balance), 0) as total
            FROM financial_titles
            WHERE tenant_id = $1
              AND kind = 'PAYABLE'
              AND status IN ('PENDING', 'PARTIAL')
              AND due_date = CURRENT_DATE
            "#,
            tenant_id
        )
            .fetch_one(&mut *tx) // <--- Use a transação
            .await?
            .total.unwrap_or(Decimal::ZERO);

        // D. Saldo Atual
        let current_balance = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(current_balance), 0) as total
            FROM financial_accounts
            WHERE tenant_id = $1 AND is_active = true
            "#,
            tenant_id
        )
            .fetch_one(&mut *tx) // <--- Use a transação
            .await?
            .total.unwrap_or(Decimal::ZERO);

        // Fecha a transação (Commit ou Rollback tanto faz pra leitura, mas commit é clean)
        tx.commit().await?;

        Ok(DashboardSummary {
            sales_today,
            receivables_today,
            payables_today,
            current_balance,
        })
    }

    // 2. Gráfico de Linha (Últimos 30 dias)
    pub async fn get_sales_last_30_days<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<SalesChartEntry>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let data = sqlx::query_as!(
            SalesChartEntry,
            r#"
            SELECT
                to_char(o.closed_at, 'YYYY-MM-DD') as "date",
                SUM(o.total_amount) as "total"
            FROM orders o
            JOIN pipeline_stages s ON o.stage_id = s.id
            WHERE o.tenant_id = $1
              AND s.category = 'DONE'
              AND o.closed_at >= (CURRENT_DATE - INTERVAL '30 days')
            GROUP BY 1
            ORDER BY 1 ASC
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(data)
    }

    // 3. Curva ABC (Top 5 Produtos mais vendidos em R$)
    pub async fn get_top_products<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<TopProductEntry>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let data = sqlx::query_as!(
            TopProductEntry,
            r#"
            SELECT
                i.name as item_name,
                SUM(oi.quantity) as total_quantity,
                SUM(oi.quantity * oi.unit_price - oi.discount) as total_revenue
            FROM order_items oi
            JOIN orders o ON oi.order_id = o.id
            JOIN pipeline_stages s ON o.stage_id = s.id
            JOIN items i ON oi.item_id = i.id
            WHERE o.tenant_id = $1
              AND s.category = 'DONE'
            GROUP BY i.id, i.name
            ORDER BY total_revenue DESC
            LIMIT 5
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(data)
    }
}