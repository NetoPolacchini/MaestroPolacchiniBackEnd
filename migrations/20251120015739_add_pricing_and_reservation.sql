-- Add migration script here
-- migrations/20251120120000_add_pricing_and_reservation.sql

-- 1. Atualiza a tabela de ITENS (Catálogo Global)
-- Adiciona um preço sugerido padrão. Útil para quando criar uma loja nova já ter um preço base.
ALTER TABLE items
    ADD COLUMN default_price NUMERIC(10, 2) DEFAULT NULL;

-- 2. Atualiza a tabela de NÍVEIS DE ESTOQUE (Saldos por Loja)
ALTER TABLE inventory_levels
    -- Preço de Venda naquela loja específica
    ADD COLUMN sale_price NUMERIC(10, 2) DEFAULT NULL,
    -- Custo Médio Unitário daquele produto naquela loja
    ADD COLUMN average_cost NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    -- Quantidade Reservada (não disponível para venda)
    ADD COLUMN reserved_quantity NUMERIC(10, 2) NOT NULL DEFAULT 0.00;

-- 3. Atualiza a tabela de MOVIMENTAÇÕES (Histórico)
-- Precisamos saber quanto custou CADA entrada para calcular o custo médio depois.
-- E precisamos saber a qual custo vendemos cada item para calcular lucro (Relatórios).
ALTER TABLE stock_movements
    ADD COLUMN unit_cost NUMERIC(10, 2) DEFAULT NULL, -- Custo unitário no momento da movimentação
    ADD COLUMN unit_price NUMERIC(10, 2) DEFAULT NULL; -- Preço de venda no momento da saída (se for venda)