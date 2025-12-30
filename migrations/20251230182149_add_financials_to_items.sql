-- migrations/20251230182149_add_financials_to_items.sql

ALTER TABLE items
    -- Financeiro
    ADD COLUMN sale_price NUMERIC(15, 4) NOT NULL DEFAULT 0,
    ADD COLUMN cost_price NUMERIC(15, 4), -- Pode ser nulo se for serviço ou desconhecido

    -- Estoque Global (Denormalizado/Cache)
    -- Útil para listar itens rapidamente sem somar todas as InventoryLevels
    ADD COLUMN current_stock NUMERIC(15, 4) NOT NULL DEFAULT 0,
    ADD COLUMN min_stock NUMERIC(15, 4);