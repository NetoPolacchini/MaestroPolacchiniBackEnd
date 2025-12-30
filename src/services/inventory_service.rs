// src/services/inventory_service.rs

use crate::{
    common::error::AppError,
    db::InventoryRepository,
    // Importamos os novos enums e structs
    models::inventory::{
        Category, InventoryLevel, Item, ItemKind, StockMovementReason,
        UnitOfMeasure, CompositionType, CompositionEntry
    },
};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use chrono::NaiveDate;
use serde_json::Value;

#[derive(Clone)]
pub struct InventoryService {
    inventory_repo: InventoryRepository,
}

impl InventoryService {
    pub fn new(inventory_repo: InventoryRepository, _pool: PgPool) -> Self {
        Self { inventory_repo }
    }

    fn calculate_new_average_cost(
        &self,
        current_qty: Decimal,
        current_avg: Decimal,
        incoming_qty: Decimal,
        incoming_cost: Decimal,
    ) -> Decimal {
        let total_current_value = current_qty * current_avg;
        let total_incoming_value = incoming_qty * incoming_cost;
        let new_total_qty = current_qty + incoming_qty;

        if new_total_qty <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        (total_current_value + total_incoming_value) / new_total_qty
    }

    // =========================================================================
    //  CREATE ITEM (ATUALIZADO COM LÓGICA DE TIPO)
    // =========================================================================

    pub async fn create_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        location_id: Option<Uuid>, // Obrigatório apenas se for criar estoque inicial
        category_id: Option<Uuid>,
        base_unit_id: Uuid,
        sku: &str,
        name: &str,
        description: Option<&str>,

        // [NOVOS CAMPOS]
        kind: ItemKind,
        settings: Option<Value>,

        initial_stock: Decimal,
        initial_cost: Decimal,
        sale_price: Decimal,
        min_stock: Option<Decimal>,
        low_stock_threshold: Decimal,
    ) -> Result<Item, AppError>
    where
        E: Executor<'e, Database = Postgres> + sqlx::Acquire<'e, Database = Postgres>,
    {
        let mut tx = executor.begin().await?;

        // 1. Cria o Item (Catálogo)
        let new_item = self.inventory_repo
            .create_item(
                &mut *tx, tenant_id, sku, name, description,
                base_unit_id, category_id,
                kind, settings, Some(initial_cost), sale_price, min_stock
            )
            .await?;

        // 2. Se for PRODUTO Físico e tiver estoque inicial, cria saldo
        // Se for SERVIÇO ou RECURSO, ignoramos estoque inicial (não faz sentido estocar "Consulta Médica")
        if kind == ItemKind::Product {
            if let Some(loc_id) = location_id {
                if initial_stock > Decimal::ZERO {
                    // 2.1. Lote Padrão
                    self.inventory_repo.update_batch_quantity(
                        &mut *tx,
                        tenant_id, new_item.id, loc_id,
                        "DEFAULT", "Geral", None,
                        initial_stock, initial_cost
                    ).await?;

                    // 2.2. Nível Geral
                    self.inventory_repo.update_inventory_level(
                        &mut *tx, tenant_id, new_item.id, loc_id, initial_stock,
                        None, Some(initial_cost), Some(sale_price), Some(low_stock_threshold)
                    ).await?;

                    // 2.3. Histórico
                    self.inventory_repo.record_stock_movement(
                        &mut *tx, tenant_id, new_item.id, loc_id, initial_stock,
                        StockMovementReason::InitialStock, Some(initial_cost), None,
                        Some("Criação de item"), Some("Geral")
                    ).await?;
                }
            }
        }

        tx.commit().await?;
        Ok(new_item)
    }

    // =========================================================================
    //  COMPOSIÇÃO (FICHA TÉCNICA) - NOVO
    // =========================================================================

    pub async fn add_composition_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        parent_id: Uuid,
        child_id: Uuid,
        quantity: Decimal,
        comp_type: CompositionType,
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // VALIDAÇÃO DE SEGURANÇA: Previne ciclo infinito simples (A -> A)
        if parent_id == child_id {
            return Err(AppError::ValidationError(
                validator::ValidationErrors::new() // Erro genérico, ideal criar AppError::CircularDependency
            ));
        }

        self.inventory_repo.add_composition_item(executor, tenant_id, parent_id, child_id, quantity, comp_type).await
    }

    pub async fn get_item_composition<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        parent_id: Uuid,
    ) -> Result<Vec<CompositionEntry>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.get_item_composition(executor, tenant_id, parent_id).await
    }

    pub async fn remove_composition_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        parent_id: Uuid,
        child_id: Uuid,
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.remove_composition_item(executor, tenant_id, parent_id, child_id).await
    }

    // =========================================================================
    //  LEITURAS BÁSICAS (Mantidas)
    // =========================================================================

    pub async fn get_all_items<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<Item>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.get_all_items(executor, tenant_id).await
    }

    pub async fn get_all_units<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<UnitOfMeasure>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.get_all_units(executor, tenant_id).await
    }

    pub async fn get_all_categories<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<Category>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.get_all_categories(executor, tenant_id).await
    }

    // =========================================================================
    //  OPERAÇÕES DE ESCRITA AUXILIARES (Units, Categories)
    // =========================================================================

    pub async fn create_unit<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        symbol: &str,
    ) -> Result<UnitOfMeasure, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.create_unit(executor, tenant_id, name, symbol).await
    }

    pub async fn create_category<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        name: &str,
        description: Option<&str>,
        parent_id: Option<Uuid>,
    ) -> Result<Category, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.inventory_repo.create_category(executor, tenant_id, name, description, parent_id).await
    }

    // =========================================================================
    //  MOVIMENTAÇÃO DE ESTOQUE (ADD / SELL)
    // =========================================================================

    pub async fn add_stock<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        quantity: Decimal,
        unit_cost: Decimal,
        reason: StockMovementReason,
        notes: Option<&str>,
        batch_number: Option<String>,
        expiration_date: Option<NaiveDate>,
        position: Option<String>,
    ) -> Result<InventoryLevel, AppError>
    where
        E: Executor<'e, Database = Postgres> + sqlx::Acquire<'e, Database = Postgres>,
    {
        // Só permite add_stock se o item for PRODUCT
        // (Seria ideal validar item.kind antes, mas por performance, deixamos passar
        // ou você pode fazer um get_item antes para checar)

        let mut tx = executor.begin().await?;

        let final_batch = batch_number.unwrap_or_else(|| "DEFAULT".to_string());
        let final_position = position.unwrap_or_else(|| "Geral".to_string());

        // 1. Atualiza Lote
        self.inventory_repo.update_batch_quantity(
            &mut *tx, tenant_id, item_id, location_id, &final_batch,
            &final_position, expiration_date, quantity, unit_cost
        ).await?;

        // 2. Atualiza Nível Geral
        let current_level = self.inventory_repo
            .get_inventory_level(&mut *tx, tenant_id, item_id, location_id)
            .await?;

        let (current_qty, current_avg) = match &current_level {
            Some(level) => (level.quantity, level.average_cost),
            None => (Decimal::ZERO, Decimal::ZERO),
        };

        let new_avg_cost = self.calculate_new_average_cost(current_qty, current_avg, quantity, unit_cost);

        let updated_level = self.inventory_repo.update_inventory_level(
            &mut *tx, tenant_id, item_id, location_id, quantity,
            None, Some(new_avg_cost), None, None
        ).await?;

        // 3. Grava Histórico
        self.inventory_repo.record_stock_movement(
            &mut *tx, tenant_id, item_id, location_id, quantity, reason,
            Some(unit_cost), None, notes, Some(&final_position)
        ).await?;

        tx.commit().await?;
        Ok(updated_level)
    }

    pub async fn sell_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        quantity: Decimal,
        unit_price: Decimal,
        consume_reservation: bool,
        notes: Option<&str>,
        specific_batch_number: Option<String>,
        specific_position: Option<String>,
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres> + sqlx::Acquire<'e, Database = Postgres>,
    {
        // Aqui também seria ideal checar se item.kind == PRODUCT
        // Se for SERVICE, a gente deveria apenas registrar a venda sem baixar estoque.
        // Mas isso ficará para o módulo de "Sales/Orders". O `sell_item` aqui é puramente movimentação de estoque.

        let mut tx = executor.begin().await?;

        // 1. Valida Saldo Total
        let level_opt = self.inventory_repo
            .get_inventory_level_for_update(&mut *tx, tenant_id, item_id, location_id)
            .await?;
        let level = level_opt.ok_or(AppError::UniqueConstraintViolation("Item não existe no estoque".into()))?;

        let available = level.quantity - level.reserved_quantity;
        if !consume_reservation && available < quantity {
            return Err(AppError::UniqueConstraintViolation("Estoque insuficiente".into()));
        }

        // 2. Atualiza Nível Total
        let quantity_delta = -quantity;
        let reserved_delta = if consume_reservation { Some(-quantity) } else { None };

        self.inventory_repo.update_inventory_level(
            &mut *tx, tenant_id, item_id, location_id, quantity_delta, reserved_delta, None, None, None
        ).await?;

        // 3. Baixa nos Lotes (FIFO ou Específico)
        let mut remaining_to_deduct = quantity;
        let mut position_for_history = String::from("Vários/FIFO");

        if let Some(target_batch) = specific_batch_number {
            let target_pos = specific_position.unwrap_or_else(|| "Geral".to_string());
            position_for_history = target_pos.clone();

            self.inventory_repo.update_batch_quantity(
                &mut *tx, tenant_id, item_id, location_id,
                &target_batch,
                &target_pos,
                None,
                -remaining_to_deduct,
                Decimal::ZERO
            ).await?;
        } else {
            let batches = self.inventory_repo
                .get_batches_for_consumption(&mut *tx, tenant_id, item_id, location_id)
                .await?;

            for batch in batches {
                if remaining_to_deduct <= Decimal::ZERO { break; }
                let available = batch.quantity;
                if available <= Decimal::ZERO { continue; }
                let to_take = if available >= remaining_to_deduct { remaining_to_deduct } else { available };

                self.inventory_repo.update_batch_quantity(
                    &mut *tx, tenant_id, item_id, location_id,
                    &batch.batch_number,
                    &batch.position,
                    None,
                    -to_take,
                    Decimal::ZERO
                ).await?;

                remaining_to_deduct -= to_take;
            }
        }

        // 4. Grava Histórico
        self.inventory_repo.record_stock_movement(
            &mut *tx, tenant_id, item_id, location_id, quantity_delta,
            StockMovementReason::Sale, None, Some(unit_price), notes,
            Some(&position_for_history)
        ).await?;

        tx.commit().await?;
        Ok(())
    }
}