// src/services/inventory_service.rs

use crate::{
    common::error::AppError,
    db::InventoryRepository,
    models::inventory::{Item, StockMovementReason},
};
use rust_decimal::Decimal;
// MUDANÇA: Removemos a PgPool, pois o serviço já não inicia transações
use sqlx::{PgPool, Postgres, Executor}; // Importamos Executor e Postgres
use uuid::Uuid;

#[derive(Clone)]
pub struct InventoryService {
    inventory_repo: InventoryRepository,
}

impl InventoryService {
    pub fn new(inventory_repo: InventoryRepository, _pool: PgPool) -> Self {
        Self { inventory_repo }
    }

    /// LÓGICA DE NEGÓCIO (ATUALIZADA PARA RLS):
    /// Recebe um executor (conexão ou transação) que JÁ TEM O RLS ATIVO.
    pub async fn create_item_with_initial_stock<'e, E>( // <--- 1. Adiciona o lifetime 'e
        &self,
        executor: E, // <-- MUDANÇA: Recebe o executor
        tenant_id: Uuid,
        location_id: Uuid,
        category_id: Uuid,
        base_unit_id: Uuid,
        sku: &str,
        name: &str,
        description: Option<&str>,
        initial_stock: Decimal,
        low_stock_threshold: Decimal,
    ) -> Result<Item, AppError>
    where
        E: Executor<'e, Database = Postgres> + sqlx::Acquire<'e, Database = Postgres>, // <--- 2. Usa 'e e adiciona Acquire
    {
        // 1. Inicia uma transação a partir do executor RLS
        // Precisamos de um tipo explícito para a transação
        let mut tx = executor.begin().await?; // <--- 3. Remove a anotação 'static
        // 2. Cria o item de "catálogo"
        let new_item = self.inventory_repo
            .create_item(
                &mut *tx, // Passa a conexão da transação
                tenant_id,
                category_id,
                base_unit_id,
                sku,
                name,
                description,
            )
            .await?;

        // 3. Define o saldo de estoque (na tabela inventory_levels)
        if initial_stock > Decimal::ZERO || low_stock_threshold > Decimal::ZERO {

            // 3a. Atualiza o saldo (a função UPSERT)
            self.inventory_repo
                .update_inventory_level(
                    &mut *tx, // Passa a mesma transação
                    tenant_id,
                    new_item.id,
                    location_id,
                    initial_stock,
                    Some(low_stock_threshold),
                )
                .await?;

            // 3b. Grava o log de auditoria
            if initial_stock > Decimal::ZERO {
                self.inventory_repo
                    .record_stock_movement(
                        &mut *tx, // Passa a mesma transação
                        tenant_id,
                        new_item.id,
                        location_id,
                        initial_stock,
                        StockMovementReason::InitialStock,
                        Some("Estoque inicial de criação do item"),
                    )
                    .await?;
            }
        }

        // 4. Commita a transação RLS
        tx.commit().await?;

        Ok(new_item)
    }
}