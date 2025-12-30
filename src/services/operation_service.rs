// src/services/operations_service.rs

use std::sync::Arc;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Postgres, Executor, Acquire};
use uuid::Uuid;

use crate::{
    common::error::AppError,
    db::OperationsRepository,
    models::operations::{Order, OrderItem, Pipeline, PipelineStage, PipelineCategory},
    services::inventory_service::InventoryService,
};

#[derive(Clone)]
pub struct OperationsService {
    repo: OperationsRepository,
    inventory_service: InventoryService,
}

impl OperationsService {
    pub fn new(repo: OperationsRepository, inventory_service: InventoryService) -> Self {
        Self {
            repo,
            inventory_service
        }
    }

    // --- PIPELINES ---

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
        self.repo.create_pipeline(executor, tenant_id, name, is_default).await
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
        self.repo.add_stage(executor, tenant_id, pipeline_id, name, category, position, stock_action).await
    }

    // --- PEDIDOS ---

    pub async fn create_order<'e, E>(
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
        // [CORREÇÃO] Chamamos o método otimizado do Repo
        self.repo.create_order_initial(executor, tenant_id, customer_id, pipeline_id, notes).await
    }

    pub async fn add_item_to_order<'e, E>(
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
    // REMOVIDO: + Copy
    // ADICIONADO: + Acquire (Permite iniciar uma transação)
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        // 1. Iniciamos uma transação para garantir que a inserção e o recálculo sejam atômicos
        let mut tx = executor.begin().await?;

        // 2. Adiciona o item (passando a referência mutável da transação)
        let item = self.repo.add_order_item(
            &mut *tx, // Re-borrow seguro
            tenant_id,
            order_id,
            item_id,
            quantity,
            unit_price,
            unit_cost
        ).await?;

        // 3. Atualiza o cache de total no pedido (passando a referência de novo)
        self.repo.recalculate_order_total(&mut *tx, tenant_id, order_id).await?;

        // 4. Salva tudo
        tx.commit().await?;

        Ok(item)
    }

    // --- TRANSIÇÃO ---

    pub async fn transition_order<'e, E>(
        &self,
        mut tx: E,
        tenant_id: Uuid,
        order_id: Uuid,
        new_stage_id: Uuid,
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        let mut tx = tx.begin().await?;

        let new_stage = self.repo.get_stage_by_id(&mut *tx, tenant_id, new_stage_id).await?;

        if let Some(action) = &new_stage.stock_action {
            if action == "DEDUCT" {
                let items = self.repo.list_order_items(&mut *tx, tenant_id, order_id).await?;

                // Busca um local padrão para baixar estoque (Ex: o primeiro local criado)
                // TODO: Melhorar isso no futuro para pegar do user context
                let location = sqlx::query!(
                    "SELECT id FROM locations WHERE tenant_id = $1 LIMIT 1",
                    tenant_id
                ).fetch_optional(&mut *tx).await?;

                if let Some(loc) = location {
                    for item in items {
                        self.inventory_service.sell_item(
                            &mut *tx,
                            tenant_id,
                            item.item_id,
                            loc.id,
                            item.quantity,
                            item.unit_price,
                            false,
                            Some(&format!("Pedido {}", order_id)),
                            None,
                            None
                        ).await?;
                    }
                }
            }
        }

        let closed_at = match new_stage.category {
            PipelineCategory::Done | PipelineCategory::Cancelled => Some(Utc::now()),
            _ => None,
        };

        self.repo.update_order_stage(&mut *tx, tenant_id, order_id, new_stage_id, closed_at).await?;

        tx.commit().await?;

        Ok(())
    }
}