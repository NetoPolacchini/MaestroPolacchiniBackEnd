// src/db/operations_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use rust_decimal::Decimal;
use crate::{
    common::error::AppError,
    models::operations::{Pipeline, PipelineStage, Order, OrderItem, PipelineCategory},
};

#[derive(Clone)]
pub struct OperationsRepository {
    pool: PgPool,
}

impl OperationsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    //  PIPELINES & STAGES (Configuração)
    // =========================================================================

    pub async fn create_pipeline<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        is_default: bool,
    ) -> Result<Pipeline, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let pipeline = sqlx::query_as!(
            Pipeline,
            r#"
            INSERT INTO pipelines (tenant_id, name, is_default)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, name, is_default, created_at
            "#,
            tenant_id,
            name,
            is_default
        )
            .fetch_one(executor)
            .await?;

        Ok(pipeline)
    }

    pub async fn add_stage<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        pipeline_id: Uuid,
        name: &str,
        category: PipelineCategory,
        position: i32,
        stock_action: Option<&str>, // 'NONE', 'RESERVE', 'DEDUCT'
    ) -> Result<PipelineStage, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let final_stock_action = stock_action.unwrap_or("NONE");

        let stage = sqlx::query_as!(
            PipelineStage,
            r#"
            INSERT INTO pipeline_stages (
                tenant_id, pipeline_id, name, category, position, stock_action
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, tenant_id, pipeline_id, name,
                category as "category: PipelineCategory",
                position, color, stock_action, generates_receivable, is_locked
            "#,
            tenant_id,
            pipeline_id,
            name,
            category as PipelineCategory,
            position,
            final_stock_action
        )
            .fetch_one(executor)
            .await?;

        Ok(stage)
    }

    // Busca a etapa inicial de um pipeline (para criar pedidos novos)
    pub async fn get_default_stage<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        pipeline_id: Uuid,
    ) -> Result<PipelineStage, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let stage = sqlx::query_as!(
            PipelineStage,
            r#"
            SELECT
                id, tenant_id, pipeline_id, name,
                category as "category: PipelineCategory",
                position, color, stock_action, generates_receivable, is_locked
            FROM pipeline_stages
            WHERE tenant_id = $1 AND pipeline_id = $2
            ORDER BY position ASC
            LIMIT 1
            "#,
            tenant_id,
            pipeline_id
        )
            .fetch_one(executor)
            .await?;

        Ok(stage)
    }

    pub async fn get_stage_by_id<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        stage_id: Uuid,
    ) -> Result<PipelineStage, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let stage = sqlx::query_as!(
            PipelineStage,
            r#"
            SELECT
                id, tenant_id, pipeline_id, name,
                category as "category: PipelineCategory",
                position, color, stock_action, generates_receivable, is_locked
            FROM pipeline_stages
            WHERE tenant_id = $1 AND id = $2
            "#,
            tenant_id,
            stage_id
        )
            .fetch_one(executor)
            .await?;

        Ok(stage)
    }

    // =========================================================================
    //  ORDERS (Operação)
    // =========================================================================

    pub async fn create_order_header<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        customer_id: Option<Uuid>,
        pipeline_id: Uuid,
        stage_id: Uuid,
        notes: Option<&str>,
    ) -> Result<Order, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let order = sqlx::query_as!(
            Order,
            r#"
            INSERT INTO orders (
                tenant_id, customer_id, pipeline_id, stage_id, notes
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, tenant_id, customer_id, pipeline_id, stage_id,
                display_id, total_amount, total_discount, tags, notes,
                opened_at, closed_at, created_at, updated_at
            "#,
            tenant_id,
            customer_id,
            pipeline_id,
            stage_id,
            notes
        )
            .fetch_one(executor)
            .await?;

        Ok(order)
    }

    pub async fn add_order_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
        item_id: Uuid,
        quantity: Decimal,
        unit_price: Decimal,
        unit_cost: Decimal,
    ) -> Result<OrderItem, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let item = sqlx::query_as!(
            OrderItem,
            r#"
            INSERT INTO order_items (
                tenant_id, order_id, item_id, quantity, unit_price, unit_cost
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            tenant_id,
            order_id,
            item_id,
            quantity,
            unit_price,
            unit_cost
        )
            .fetch_one(executor)
            .await?;

        Ok(item)
    }

    pub async fn update_order_stage<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
        new_stage_id: Uuid,
        closed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query!(
            r#"
            UPDATE orders
            SET stage_id = $1, closed_at = $2, updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            "#,
            new_stage_id,
            closed_at,
            order_id,
            tenant_id
        )
            .execute(executor)
            .await?;

        Ok(())
    }

    // Atualiza o total do pedido somando os itens
    pub async fn recalculate_order_total<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
    ) -> Result<Decimal, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // 1. Soma os itens
        let row = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(quantity * unit_price - discount), 0) as total
            FROM order_items
            WHERE order_id = $1 AND tenant_id = $2
            "#,
            order_id,
            tenant_id
        )
            .fetch_one(executor)
            .await?;

        let new_total: Decimal = row.total.unwrap_or(Decimal::ZERO);

        // 2. Atualiza o header
        sqlx::query!(
            r#"
            UPDATE orders SET total_amount = $1 WHERE id = $2
            "#,
            new_total,
            order_id
        )
            // Note que aqui não podemos usar o mesmo executor se ele já estiver "gasto" (ex: fetch_one anterior).
            // Mas como estamos passando 'executor' genérico, se for transaction funciona.
            // Porém, sqlx exige re-borrow ou clone se for pool.
            // O ideal é quem chamar essa função garantir a ordem.
            .execute(executor) // Atenção aqui em rust puro, pode dar erro de borrow.
            // No Service resolveremos isso usando Transaction reference.
            .await?;

        Ok(new_total)
    }
}