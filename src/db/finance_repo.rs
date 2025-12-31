// src/db/finance_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::NaiveDate;
use crate::{
    common::error::AppError,
    models::finance::{FinancialAccount, FinancialCategory, FinancialTitle, TitleKind, TitleStatus},
};

#[derive(Clone)]
pub struct FinanceRepository {
    pool: PgPool,
}

impl FinanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    //  CONTAS BANCÁRIAS (Caixa)
    // =========================================================================

    pub async fn create_account<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        bank_name: Option<&str>,
    ) -> Result<FinancialAccount, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let account = sqlx::query_as!(
            FinancialAccount,
            r#"
            INSERT INTO financial_accounts (tenant_id, name, bank_name)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, name, bank_name, current_balance, is_active, created_at
            "#,
            tenant_id,
            name,
            bank_name
        )
            .fetch_one(executor)
            .await?;

        Ok(account)
    }

    pub async fn get_all_accounts<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<FinancialAccount>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let accounts = sqlx::query_as!(
            FinancialAccount,
            "SELECT * FROM financial_accounts WHERE tenant_id = $1 ORDER BY name ASC",
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(accounts)
    }

    // =========================================================================
    //  CATEGORIAS (Plano de Contas)
    // =========================================================================

    pub async fn create_category<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        kind: TitleKind,
    ) -> Result<FinancialCategory, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let category = sqlx::query_as!(
            FinancialCategory,
            r#"
            INSERT INTO financial_categories (tenant_id, name, kind)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, name, kind as "kind: TitleKind", is_active
            "#,
            tenant_id,
            name,
            kind as TitleKind
        )
            .fetch_one(executor)
            .await?;

        Ok(category)
    }

    pub async fn get_all_categories<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<FinancialCategory>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let categories = sqlx::query_as!(
            FinancialCategory,
            r#"
            SELECT id, tenant_id, name, kind as "kind: TitleKind", is_active
            FROM financial_categories
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(categories)
    }

    // =========================================================================
    //  TÍTULOS (Contas a Pagar / Receber)
    // =========================================================================

    pub async fn create_title<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        description: &str,
        kind: TitleKind,
        amount: Decimal,
        due_date: NaiveDate,
        category_id: Option<Uuid>,
        customer_id: Option<Uuid>,
        order_id: Option<Uuid>,
    ) -> Result<FinancialTitle, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // No início, amount_balance (o que falta pagar) é igual ao amount_original
        let title = sqlx::query_as!(
            FinancialTitle,
            r#"
            INSERT INTO financial_titles (
                tenant_id, description, kind,
                amount_original, amount_balance,
                due_date, category_id, customer_id, order_id
            )
            VALUES ($1, $2, $3, $4, $4, $5, $6, $7, $8)
            RETURNING
                id, tenant_id, description,
                kind as "kind: TitleKind",
                status as "status: TitleStatus",
                amount_original, amount_balance,
                due_date, competence_date,
                category_id, customer_id, order_id,
                created_at, updated_at
            "#,
            tenant_id,
            description,
            kind as TitleKind,
            amount, // Passamos 2x no values (original e balance)
            due_date,
            category_id,
            customer_id,
            order_id
        )
            .fetch_one(executor)
            .await?;

        Ok(title)
    }
}