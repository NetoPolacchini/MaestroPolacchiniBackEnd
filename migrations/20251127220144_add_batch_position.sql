-- migrations/20251127220144_add_batch_position.sql

-- 1. Alterar Tabela de Lotes (Batches)
ALTER TABLE inventory_batches
    ADD COLUMN position VARCHAR(100) NOT NULL DEFAULT 'Geral';

-- MUDANÇA AQUI: Usamos o nome curto que definimos no passo anterior
ALTER TABLE inventory_batches
DROP CONSTRAINT uq_batches_location_batch;

-- Criar a nova restrição (Também vamos dar um nome curto para evitar problemas futuros)
ALTER TABLE inventory_batches
    ADD CONSTRAINT uq_batches_location_batch_position
        UNIQUE (tenant_id, item_id, location_id, batch_number, position);

-- 2. Alterar Tabela de Movimentações
ALTER TABLE stock_movements
    ADD COLUMN position VARCHAR(100);