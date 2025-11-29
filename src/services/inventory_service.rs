// src/services/inventory_service.rs

use crate::{
    common::error::AppError,
    db::InventoryRepository,
    models::inventory::{Item, InventoryLevel, StockMovementReason},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use sqlx::{PgPool, Postgres, Executor};
use uuid::Uuid;
use chrono::NaiveDate; // Importante

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

    // --- CREATE ITEM ---
    pub async fn create_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        location_id: Option<Uuid>,
        category_id: Uuid,
        base_unit_id: Uuid,
        sku: &str,
        name: &str,
        description: Option<&str>,
        initial_stock: Decimal,
        initial_cost: Decimal,
        low_stock_threshold: Decimal,
    ) -> Result<Item, AppError>
    where
        E: Executor<'e, Database = Postgres> + sqlx::Acquire<'e, Database = Postgres>,
    {
        let mut tx = executor.begin().await?;

        let new_item = self.inventory_repo
            .create_item(&mut *tx, tenant_id, category_id, base_unit_id, sku, name, description)
            .await?;

        if let Some(loc_id) = location_id {
            if initial_stock > Decimal::ZERO {
                // 1. Atualiza Lote Padrão ("Geral")
                self.inventory_repo.update_batch_quantity(
                    &mut *tx,
                    tenant_id,
                    new_item.id,
                    loc_id,
                    "DEFAULT", // Lote padrão
                    "Geral",   // Posição padrão
                    None,      // Sem validade
                    initial_stock,
                    initial_cost
                ).await?;

                // 2. Atualiza Nível Geral
                self.inventory_repo.update_inventory_level(
                    &mut *tx, tenant_id, new_item.id, loc_id, initial_stock,
                    None, Some(initial_cost), None, Some(low_stock_threshold)
                ).await?;

                // 3. Grava Histórico (CORRIGIDO: Passando Some("Geral"))
                self.inventory_repo.record_stock_movement(
                    &mut *tx, tenant_id, new_item.id, loc_id, initial_stock,
                    StockMovementReason::InitialStock, Some(initial_cost), None,
                    Some("Criação de item"),
                    Some("Geral") // <--- Argumento que faltava!
                ).await?;
            }
        }

        tx.commit().await?;
        Ok(new_item)
    }

    // --- ADD STOCK (ENTRADA) ---
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

    // --- SELL ITEM (VENDA / SAÍDA) ---
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
        specific_position: Option<String>, // <--- Adicionado para suportar posição específica
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres> + sqlx::Acquire<'e, Database = Postgres>,
    {
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

        // Posição usada para o histórico (se for FIFO, pode ser misto, então gravamos "Vários" ou o primeiro)
        let mut position_for_history = String::from("Vários/FIFO");

        if let Some(target_batch) = specific_batch_number {
            // CASO A: Lote Específico
            let target_pos = specific_position.unwrap_or_else(|| "Geral".to_string());
            position_for_history = target_pos.clone();

            self.inventory_repo.update_batch_quantity(
                &mut *tx, tenant_id, item_id, location_id,
                &target_batch,
                &target_pos, // <--- Argumento que faltava (Posição Específica)
                None,
                -remaining_to_deduct,
                Decimal::ZERO
            ).await?;
        } else {
            // CASO B: FIFO
            let batches = self.inventory_repo
                .get_batches_for_consumption(&mut *tx, tenant_id, item_id, location_id)
                .await?;

            for batch in batches {
                if remaining_to_deduct <= Decimal::ZERO { break; }

                let available = batch.quantity;
                if available <= Decimal::ZERO { continue; }

                let to_take = if available >= remaining_to_deduct { remaining_to_deduct } else { available };

                // Baixa deste lote específico
                self.inventory_repo.update_batch_quantity(
                    &mut *tx, tenant_id, item_id, location_id,
                    &batch.batch_number,
                    &batch.position, // <--- Argumento que faltava (Posição do lote do loop)
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
            Some(&position_for_history) // <--- Argumento que faltava (Posição)
        ).await?;

        tx.commit().await?;
        Ok(())
    }
}