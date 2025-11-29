// src/db/inventory_repo.rs

use sqlx::{Executor, PgPool, Postgres};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use uuid::Uuid;
use crate::{
    common::error::AppError,
    models::inventory::{
        Category, Item, InventoryLevel, StockMovement, StockMovementReason, UnitOfMeasure, InventoryBatch
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
    /// [ATUALIZADO] Agora suporta custo médio e preço de venda
    pub async fn update_inventory_level<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        item_id: Uuid,
        location_id: Uuid,
        quantity_delta: Decimal,
        // Novos campos opcionais para atualização:
        reserved_delta: Option<Decimal>, // Pode ser +1, -1 ou 0
        new_average_cost: Option<Decimal>, // Se mudar o custo, passamos o novo valor
        new_sale_price: Option<Decimal>,   // Se mudar o preço, passamos
        low_stock_threshold: Option<Decimal>,
    ) -> Result<InventoryLevel, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Prepara os valores para o SQL
        let reserved_change = reserved_delta.unwrap_or(Decimal::ZERO);

        // Atenção: Custo e Preço são ATUALIZAÇÕES ABSOLUTAS, não deltas (somas).
        // Se eu passar None, quero manter o valor que já está no banco.
        // Para isso, usamos a função COALESCE do SQL no UPDATE, mas no INSERT precisamos de valor.

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
                -- Se passou novo custo ($6), usa. Se não, mantém o antigo.
                -- No INSERT, se for NULL vira 0 (tratado antes).
                average_cost = COALESCE($6, inventory_levels.average_cost),
                sale_price   = COALESCE($7, inventory_levels.sale_price),
                low_stock_threshold = COALESCE($8, inventory_levels.low_stock_threshold),
                updated_at = NOW()
            RETURNING *
            "#,
            tenant_id,
            item_id,
            location_id,
            quantity_delta,    // $4
            reserved_change,   // $5
            new_average_cost.unwrap_or(Decimal::ZERO), // $6 (Só usa 0 se for INSERT novo)
            new_sale_price,    // $7
            low_stock_threshold.unwrap_or(Decimal::ZERO) // $8
        )
            .fetch_one(executor)
            .await?;

        Ok(level)
    }

    /// [ATUALIZADO] Registra uma movimentação no livro-razão (auditoria).
    /// [ATUALIZADO] Registra movimentação com valores financeiros
    /// [ATUALIZADO] Grava a posição no histórico
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
        position: Option<&str>, // <--- NOVO ARGUMENTO
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
                position -- Retorna a nova coluna
            "#,
            tenant_id,
            item_id,
            location_id,
            quantity_changed,
            reason as StockMovementReason,
            unit_cost,
            unit_price,
            notes,
            position // $9
        )
            .fetch_one(executor)
            .await?;

        Ok(movement)
    }

    // [NOVO] Busca o saldo atual de um item em uma loja
    // Retorna Option, pois pode ser que o item nunca tenha entrado nessa loja.
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

    // [NOVO] Leitura com TRAVAMENTO (Row Locking)
    // Use isso dentro de uma transação para garantir que ninguém mude o saldo
    // enquanto você decide se pode vender.
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
        position: &str, // <--- NOVO ARGUMENTO
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
                position, -- Nova Coluna
                expiration_date, quantity, unit_cost
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (tenant_id, item_id, location_id, batch_number, position) -- Nova Constraint
            DO UPDATE SET
                quantity = inventory_batches.quantity + $7,
                updated_at = NOW()
            RETURNING *
            "#,
            tenant_id,
            item_id,
            location_id,
            batch_number,
            position, // $5
            expiration_date, // $6
            quantity_delta,  // $7
            unit_cost        // $8
        )
            .fetch_one(executor)
            .await?;

        Ok(batch)
    }

    // [NOVO] Busca lotes para consumo (FIFO)
    // Ordena por Data de Validade ASC (Vence antes = Primeiro da lista)
    // Se não tiver validade (NULL), ordena por Data de Criação (Mais velho = Primeiro)
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
              AND quantity > 0 -- Só queremos lotes com saldo
            ORDER BY
                expiration_date ASC NULLS LAST, -- Primeiro os que vencem
                created_at ASC                  -- Desempate: os mais antigos
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