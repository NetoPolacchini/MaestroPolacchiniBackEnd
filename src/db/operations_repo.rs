// src/db/operations_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use rust_decimal::Decimal;
use crate::{
    common::error::AppError,
    models::operations::{Pipeline, PipelineStage, Order, OrderItem, PipelineCategory, OrderDetail},
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
    //  PIPELINES & STAGES
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
        stock_action: Option<&str>,
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

    // [OTIMIZAÇÃO] Não precisamos mais do get_default_stage separado
    // se usarmos uma subquery na criação do pedido.
    // Mas mantemos aqui para consultas de frontend se necessário.
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
    //  ORDERS
    // =========================================================================

    // [CORREÇÃO] Combinamos "Buscar Stage Default" + "Criar Pedido" em uma única query.
    // Isso evita o erro de "moved value executor" no Service.
    pub async fn create_order_initial<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        customer_id: Option<Uuid>,
        pipeline_id: Uuid,
        notes: Option<&str>,
    ) -> Result<Order, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // A subquery (SELECT id FROM pipeline_stages ...) pega a primeira etapa automaticamente
        let order = sqlx::query_as!(
            Order,
            r#"
            INSERT INTO orders (
                tenant_id, customer_id, pipeline_id, stage_id, notes
            )
            VALUES (
                $1, $2, $3,
                (
                    SELECT id FROM pipeline_stages
                    WHERE pipeline_id = $3 AND tenant_id = $1
                    ORDER BY position ASC LIMIT 1
                ),
                $4
            )
            RETURNING
                id, tenant_id, customer_id, pipeline_id, stage_id,
                display_id, total_amount, total_discount, tags, notes,
                opened_at, closed_at, created_at, updated_at
            "#,
            tenant_id,
            customer_id,
            pipeline_id,
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

    // [CORREÇÃO] Recalcula e Atualiza em UMA única query.
    // Resolve o problema de "borrow executor twice".
    pub async fn recalculate_order_total<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
    ) -> Result<Decimal, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UPDATE com FROM/Subquery é super eficiente no Postgres
        // Retorna o novo total calculado
        let result = sqlx::query!(
            r#"
            UPDATE orders
            SET total_amount = (
                SELECT COALESCE(SUM(quantity * unit_price - discount), 0)
                FROM order_items
                WHERE order_items.order_id = orders.id
            )
            WHERE id = $1 AND tenant_id = $2
            RETURNING total_amount
            "#,
            order_id,
            tenant_id
        )
            .fetch_one(executor)
            .await?;

        Ok(result.total_amount)
    }

    pub async fn list_order_items<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
    ) -> Result<Vec<OrderItem>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let items = sqlx::query_as!(
            OrderItem,
            r#"
            SELECT * FROM order_items
            WHERE tenant_id = $1 AND order_id = $2
            "#,
            tenant_id,
            order_id
        )
            .fetch_all(executor)
            .await?;

        Ok(items)
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

    pub async fn get_order_detail<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
    ) -> Result<OrderDetail, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Vamos buscar o Order + Nome do Cliente + Nome da Etapa
        // Note que OrderDetail foi definido no models/operations.rs mas talvez precisemos ajustar a query
        // para bater com os campos da struct.

        // Vamos fazer uma query manual para preencher a struct OrderDetail
        // Se a struct OrderDetail for complexa, vamos simplificar e criar uma struct local ou
        // usar um JOIN simples.

        // Vamos focar no essencial para o PDF: Header + Customer Name.
        // O OrderDetail pede 'items' também, mas vamos preencher os items separadamente no service.

        let header = sqlx::query_as!(
            Order,
            "SELECT * FROM orders WHERE id = $1 AND tenant_id = $2",
            order_id, tenant_id
        ).fetch_one(executor).await?;

        // Buscamos nomes auxiliares
        let aux = sqlx::query!(
            r#"
            SELECT
                c.full_name as "customer_name?",
                s.name as stage_name,
                s.category as "category: PipelineCategory"
            FROM orders o
            JOIN pipeline_stages s ON o.stage_id = s.id
            LEFT JOIN customers c ON o.customer_id = c.id
            WHERE o.id = $1
            "#,
            order_id
        ).fetch_one(&self.pool.clone()).await?; // Usamos pool aqui pra facilitar, pois o executor foi movido acima

        // Retornamos um objeto montado.
        // Nota: O ideal seria uma query única com JOIN, mas para manter didático:
        Ok(OrderDetail {
            header,
            customer_name: aux.customer_name,
            stage_name: aux.stage_name,
            stage_category: aux.category,
            items: vec![], // Preencheremos no Service
        })
    }
}