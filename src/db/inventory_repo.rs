// src/db/inventory_repo.rs

use sqlx::{Executor, PgPool, Postgres};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use uuid::Uuid;
use crate::{
    common::error::AppError,
    models::inventory::{
        Category, Item, InventoryLevel, StockMovement, StockMovementReason, UnitOfMeasure,
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

    // ---
    // Funções de "Leitura" (Getters)
    // ---
    // Funções de leitura são simples e podem usar a pool principal.

    pub async fn get_all_items<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid
    ) -> Result<Vec<Item>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let items = sqlx::query_as!(
            Item,
            "SELECT * FROM items WHERE tenant_id = $1 ORDER BY name ASC",
            tenant_id
        )
            .fetch_all(executor)
            .await?;
        Ok(items)
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
            .fetch_all(executor) // <-- MUDANÇA: Usa executor
            .await?;
        Ok(categories)
    }

    pub async fn get_all_units<'e, E>(
        &self,
        executor: E, // <-- MUDANÇA: Aceita executor
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
            .fetch_all(executor) // <-- MUDANÇA: Usa executor
            .await?;
        Ok(units)
    }

    // ---
    // Funções de "Escrita" (Transacionais)
    // ---
    // Estas usam o padrão genérico 'Executor' para rodar dentro de uma transação.

    /// Cria uma nova unidade (kg, un, L) para um tenant.
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

    /// Cria uma nova categoria (raiz ou subcategoria) para um tenant.
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

    /// Cria um item de "catálogo" (sem estoque) para um tenant.
    pub async fn create_item<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        category_id: Uuid,
        base_unit_id: Uuid,
        sku: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Item, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as!(
            Item,
            r#"
            INSERT INTO items (tenant_id, category_id, base_unit_id, sku, name, description)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            tenant_id,
            category_id,
            base_unit_id,
            sku,
            name,
            description
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

    /// [NOVO] Atualiza o saldo de estoque (quantity) de um item em um local.
    /// Esta é a função mais robusta: ela cria ou atualiza o saldo de forma atômica.
    pub async fn update_inventory_level<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        quantity_changed: Decimal, // ex: +50.0 ou -1.0
        low_stock_threshold: Option<Decimal>, // Opcional: só define na criação
    ) -> Result<InventoryLevel, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Esta query é um "UPSERT".
        // Tenta INSERIR. Se já existir (ON CONFLICT), ele ATUALIZA.
        // Isso é atômico e previne "race conditions".
        let level = sqlx::query_as!(
            InventoryLevel,
            r#"
            INSERT INTO inventory_levels (tenant_id, item_id, location_id, quantity, low_stock_threshold)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (tenant_id, item_id, location_id)
            DO UPDATE SET
                -- Adiciona (ou subtrai) a quantidade da existente
                quantity = inventory_levels.quantity + $4
            RETURNING *
            "#,
            tenant_id,
            item_id,
            location_id,
            quantity_changed,
            // 'COALESCE' usa o 'low_stock_threshold' passado ($5) apenas se for a primeira vez (INSERT).
            // Se for um UPDATE, ele mantém o valor que já estava lá (inventory_levels.low_stock_threshold).
            low_stock_threshold.unwrap_or_else(Decimal::zero)
        )
            .fetch_one(executor)
            .await?;

        Ok(level)
    }

    /// [ATUALIZADO] Registra uma movimentação no livro-razão (auditoria).
    pub async fn record_stock_movement<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid, // <-- Agora sabe o local
        quantity_changed: Decimal,
        reason: StockMovementReason,
        notes: Option<&str>,
    ) -> Result<StockMovement, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let movement = sqlx::query_as!(
            StockMovement,
            r#"
            INSERT INTO stock_movements (tenant_id, item_id, location_id, quantity_changed, reason, notes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, tenant_id, item_id, location_id, quantity_changed, reason as "reason: _", notes, created_at
            "#,
            tenant_id,
            item_id,
            location_id,
            quantity_changed,
            reason as StockMovementReason,
            notes
        )
            .fetch_one(executor)
            .await?;

        Ok(movement)
    }
}