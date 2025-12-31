// src/services/finance_service.rs

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Postgres, Executor};
use uuid::Uuid;

use crate::{
    common::error::AppError,
    db::FinanceRepository,
    models::finance::{FinancialTitle, TitleKind},
};

#[derive(Clone)]
pub struct FinanceService {
    repo: FinanceRepository,
}

impl FinanceService {
    pub fn new(repo: FinanceRepository) -> Self {
        Self { repo }
    }

    /// Cria um Título a Receber automaticamente a partir de um pedido finalizado
    pub async fn create_receivable_for_order<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
        display_id: i32,
        amount: Decimal,
        customer_id: Option<Uuid>,
    ) -> Result<FinancialTitle, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // [CORREÇÃO] Removemos a busca de categorias que causava o erro de "moved value".
        // Como decidido, por enquanto passamos None no category_id.
        // Futuramente, passaremos o ID da categoria como argumento.

        let description = format!("Venda Pedido #{}", display_id);
        let due_date = Utc::now().date_naive(); // Vence hoje

        // Agora o 'executor' é usado apenas uma vez aqui, resolvendo o erro E0382
        let title = self.repo.create_title(
            executor,
            tenant_id,
            &description,
            TitleKind::Receivable,
            amount,
            due_date,
            None, // category_id (Futuro: Buscar "Vendas")
            customer_id,
            Some(order_id)
        ).await?;

        Ok(title)
    }
}