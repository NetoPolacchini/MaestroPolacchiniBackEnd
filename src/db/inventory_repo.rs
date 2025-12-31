// src/db/inventory_repo.rs

use sqlx::{Executor, PgPool, Postgres};
use rust_decimal::Decimal;
use uuid::Uuid;
use serde_json::Value;

use crate::{
    common::error::AppError,
    models::inventory::{
        Category, Item, InventoryLevel, StockMovement, StockMovementReason,
        UnitOfMeasure, InventoryBatch, ItemKind, CompositionEntry, CompositionType
    },
};

#[derive(Clone)]
pub struct InventoryRepository {
    pool: PgPool,
}

impl InventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    //  LEITURA (Getters)
    // =========================================================================

    pub async fn get_all_items<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid
    ) -> Result<Vec<Item>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // [CORREÇÃO]: Mapeamos 'base_unit_id' (banco) para 'unit_id' (struct)
        let items = sqlx::query_as!(
            Item,
            r#"
            SELECT
                id, tenant_id, sku, name, description,
                base_unit_id as unit_id,
                category_id,
                kind as "kind: ItemKind",
                settings,
                cost_price, sale_price,
                current_stock, min_stock,
                created_at, updated_at
            FROM items
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#,
            tenant_id
        )
            .fetch_all(executor)
            .await?;
        Ok(items)
    }

    pub async fn get_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
    ) -> Result<Option<Item>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let item = sqlx::query_as!(
            Item,
            r#"
            SELECT
                id, tenant_id, sku, name, description,
                base_unit_id as unit_id,
                category_id,
                kind as "kind: ItemKind",
                settings,
                cost_price, sale_price,
                current_stock, min_stock,
                created_at, updated_at
            FROM items
            WHERE tenant_id = $1 AND id = $2
            "#,
            tenant_id,
            item_id
        )
            .fetch_optional(executor)
            .await?;

        Ok(item)
    }

    pub async fn get_all_categories<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid
    ) -> Result<Vec<Category>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let categories = sqlx::query_as!(
            Category,
            "SELECT * FROM categories WHERE tenant_id = $1 ORDER BY name ASC",
            tenant_id
        )
            .fetch_all(executor)
            .await?;
        Ok(categories)
    }

    pub async fn get_all_units<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid
    ) -> Result<Vec<UnitOfMeasure>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let units = sqlx::query_as!(
            UnitOfMeasure,
            "SELECT * FROM units_of_measure WHERE tenant_id = $1 ORDER BY name ASC",
            tenant_id
        )
            .fetch_all(executor)
            .await?;
        Ok(units)
    }

    // =========================================================================
    //  ESCRITA BÁSICA (Units, Categories)
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
        sqlx::query_as!(
            UnitOfMeasure,
            r#"
            INSERT INTO units_of_measure (tenant_id, name, symbol)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            tenant_id,
            name,
            symbol
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        let constraint = db_err.constraint().unwrap_or_default();
                        if constraint.contains("name") {
                            return AppError::UnitNameAlreadyExists(name.to_string());
                        }
                        if constraint.contains("symbol") {
                            return AppError::UnitSymbolAlreadyExists(symbol.to_string());
                        }
                    }
                }
                e.into()
            })
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
        sqlx::query_as!(
            Category,
            r#"
            INSERT INTO categories (tenant_id, name, description, parent_id)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
            tenant_id,
            name,
            description,
            parent_id
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::CategoryNameAlreadyExists(name.to_string());
                    }
                }
                e.into()
            })
    }

    // =========================================================================
    //  CATÁLOGO AVANÇADO (Items + Composição)
    // =========================================================================

    pub async fn create_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        sku: &str,
        name: &str,
        description: Option<&str>,
        unit_id: Uuid,
        category_id: Option<Uuid>,

        kind: ItemKind,
        settings: Option<Value>,
        cost_price: Option<Decimal>,
        sale_price: Decimal,
        min_stock: Option<Decimal>,
    ) -> Result<Item, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let final_settings = settings.unwrap_or(serde_json::json!({}));

        // [CORREÇÃO]: Usando 'base_unit_id' no INSERT e no RETURNING (mapeado)
        sqlx::query_as!(
            Item,
            r#"
            INSERT INTO items (
                tenant_id, sku, name, description, base_unit_id, category_id,
                kind, settings,
                cost_price, sale_price, min_stock, current_stock
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 0)
            RETURNING
                id, tenant_id, sku, name, description,
                kind as "kind: ItemKind",
                settings,
                base_unit_id as unit_id,
                category_id, cost_price, sale_price,
                current_stock, min_stock, created_at, updated_at
            "#,
            tenant_id,
            sku,
            name,
            description,
            unit_id,
            category_id,
            kind as ItemKind,
            final_settings,
            cost_price,
            sale_price,
            min_stock
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return AppError::SkuAlreadyExists;
                    }
                }
                e.into()
            })
    }

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
        sqlx::query!(
            r#"
            INSERT INTO item_compositions (tenant_id, parent_item_id, child_item_id, quantity, comp_type)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (parent_item_id, child_item_id)
            DO UPDATE SET
                quantity = EXCLUDED.quantity,
                comp_type = EXCLUDED.comp_type
            "#,
            tenant_id,
            parent_id,
            child_id,
            quantity,
            comp_type as CompositionType
        )
            .execute(executor)
            .await?;

        Ok(())
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
        sqlx::query!(
            r#"
            DELETE FROM item_compositions
            WHERE tenant_id = $1 AND parent_item_id = $2 AND child_item_id = $3
            "#,
            tenant_id,
            parent_id,
            child_id
        )
            .execute(executor)
            .await?;

        Ok(())
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
        // [CORREÇÃO]: Usando 'base_unit_id' e tabela 'units_of_measure' correta
        let composition = sqlx::query_as!(
            CompositionEntry,
            r#"
            SELECT
                ic.id,
                ic.child_item_id,
                i.sku as child_sku,
                i.name as child_name,
                u.symbol as child_unit,
                ic.quantity,
                ic.comp_type as "comp_type: CompositionType"
            FROM item_compositions ic
            JOIN items i ON ic.child_item_id = i.id
            JOIN units_of_measure u ON i.base_unit_id = u.id
            WHERE ic.parent_item_id = $1
              AND ic.tenant_id = $2
            ORDER BY i.name ASC
            "#,
            parent_id,
            tenant_id
        )
            .fetch_all(executor)
            .await?;

        Ok(composition)
    }

    // =========================================================================
    //  ESTOQUE & MOVIMENTAÇÃO (Mantido igual)
    // =========================================================================

    pub async fn update_inventory_level<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        quantity_delta: Decimal,
        reserved_delta: Option<Decimal>,
        new_average_cost: Option<Decimal>,
        new_sale_price: Option<Decimal>,
        low_stock_threshold: Option<Decimal>,
    ) -> Result<InventoryLevel, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let reserved_change = reserved_delta.unwrap_or(Decimal::ZERO);

        let level = sqlx::query_as!(
            InventoryLevel,
            r#"
            INSERT INTO inventory_levels (
                tenant_id, item_id, location_id,
                quantity, reserved_quantity,
                average_cost, sale_price, low_stock_threshold
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (tenant_id, item_id, location_id)
            DO UPDATE SET
                quantity = inventory_levels.quantity + $4,
                reserved_quantity = inventory_levels.reserved_quantity + $5,
                average_cost = COALESCE($6, inventory_levels.average_cost),
                sale_price   = COALESCE($7, inventory_levels.sale_price),
                low_stock_threshold = COALESCE($8, inventory_levels.low_stock_threshold),
                updated_at = NOW()
            RETURNING *
            "#,
            tenant_id,
            item_id,
            location_id,
            quantity_delta,
            reserved_change,
            new_average_cost.unwrap_or(Decimal::ZERO),
            new_sale_price,
            low_stock_threshold.unwrap_or(Decimal::ZERO)
        )
            .fetch_one(executor)
            .await?;

        Ok(level)
    }

    pub async fn record_stock_movement<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        quantity_changed: Decimal,
        reason: StockMovementReason,
        unit_cost: Option<Decimal>,
        unit_price: Option<Decimal>,
        notes: Option<&str>,
        position: Option<&str>,
    ) -> Result<StockMovement, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let movement = sqlx::query_as!(
            StockMovement,
            r#"
            INSERT INTO stock_movements (
                tenant_id, item_id, location_id,
                quantity_changed, reason,
                unit_cost, unit_price, notes, position
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, tenant_id, item_id, location_id,
                quantity_changed,
                reason as "reason: StockMovementReason",
                unit_cost, unit_price, notes, created_at,
                position
            "#,
            tenant_id,
            item_id,
            location_id,
            quantity_changed,
            reason as StockMovementReason,
            unit_cost,
            unit_price,
            notes,
            position
        )
            .fetch_one(executor)
            .await?;

        Ok(movement)
    }

    pub async fn get_inventory_level<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<InventoryLevel>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let level = sqlx::query_as!(
            InventoryLevel,
            r#"
            SELECT * FROM inventory_levels
            WHERE tenant_id = $1 AND item_id = $2 AND location_id = $3
            "#,
            tenant_id,
            item_id,
            location_id
        )
            .fetch_optional(executor)
            .await?;

        Ok(level)
    }

    pub async fn get_inventory_level_for_update<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<InventoryLevel>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let level = sqlx::query_as!(
            InventoryLevel,
            r#"
            SELECT * FROM inventory_levels
            WHERE tenant_id = $1 AND item_id = $2 AND location_id = $3
            FOR UPDATE
            "#,
            tenant_id,
            item_id,
            location_id
        )
            .fetch_optional(executor)
            .await?;

        Ok(level)
    }

    pub async fn update_batch_quantity<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        batch_number: &str,
        position: &str,
        expiration_date: Option<chrono::NaiveDate>,
        quantity_delta: Decimal,
        unit_cost: Decimal,
    ) -> Result<InventoryBatch, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let batch = sqlx::query_as!(
            InventoryBatch,
            r#"
            INSERT INTO inventory_batches (
                tenant_id, item_id, location_id, batch_number,
                position,
                expiration_date, quantity, unit_cost
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (tenant_id, item_id, location_id, batch_number, position)
            DO UPDATE SET
                quantity = inventory_batches.quantity + $7,
                updated_at = NOW()
            RETURNING *
            "#,
            tenant_id,
            item_id,
            location_id,
            batch_number,
            position,
            expiration_date,
            quantity_delta,
            unit_cost
        )
            .fetch_one(executor)
            .await?;

        Ok(batch)
    }

    pub async fn get_batches_for_consumption<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<InventoryBatch>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let batches = sqlx::query_as!(
            InventoryBatch,
            r#"
            SELECT * FROM inventory_batches
            WHERE tenant_id = $1
              AND item_id = $2
              AND location_id = $3
              AND quantity > 0
            ORDER BY
                expiration_date ASC NULLS LAST,
                created_at ASC
            "#,
            tenant_id,
            item_id,
            location_id
        )
            .fetch_all(executor)
            .await?;

        Ok(batches)
    }
}