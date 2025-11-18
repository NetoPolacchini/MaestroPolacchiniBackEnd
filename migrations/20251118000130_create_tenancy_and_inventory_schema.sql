-- Garante que a extensão pgcrypto, que provê gen_random_uuid(), esteja disponível
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ---
-- 1. FUNÇÃO DE TRIGGER (Para o 'updated_at')
-- ---
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;


-- ---
-- 2. ARQUITETURA DE TENANCY (Estabelecimentos e Usuários)
-- ---
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON tenants FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

CREATE TABLE user_tenants (
    user_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(user_id, tenant_id)
);

-- ---
-- 3. ARQUITETURA DE LOCAIS (Pools e Locais)
-- ---

-- [NOVO] A "Piscina de Estoque" (Estoque Lojas Bike, Estoque Restaurante)
CREATE TABLE stock_pools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON stock_pools FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();


CREATE TABLE locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,

    -- [NOVO] Cada local pertence a um "Pool" de estoque
    stock_pool_id UUID REFERENCES stock_pools(id) ON DELETE RESTRICT NOT NULL,

    name VARCHAR(255) NOT NULL,
    is_warehouse BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON locations FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();


-- ---
-- 4. ESQUEMA DE ESTOQUE (Refatorado)
-- ---
-- (Units, Categories, Items agora pertencem a um Tenant)

CREATE TABLE units_of_measure (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    name VARCHAR(255) NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name),
    UNIQUE(tenant_id, symbol)
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON units_of_measure FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    parent_id UUID REFERENCES categories(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, parent_id, name)
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON categories FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

CREATE TABLE items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    category_id UUID REFERENCES categories(id) ON DELETE RESTRICT NOT NULL,
    base_unit_id UUID REFERENCES units_of_measure(id) ON DELETE RESTRICT NOT NULL,
    sku VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, sku)
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON items FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();


-- Tabela de Saldos (Nível do Estoque)
CREATE TABLE inventory_levels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    item_id UUID REFERENCES items(id) ON DELETE CASCADE NOT NULL,
    location_id UUID REFERENCES locations(id) ON DELETE CASCADE NOT NULL,
    quantity NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    low_stock_threshold NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, item_id, location_id)
);
CREATE TRIGGER set_timestamp BEFORE UPDATE ON inventory_levels FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();


-- Tabela de Auditoria (Movimentações)
CREATE TYPE stock_movement_reason AS ENUM (
    'INITIAL_STOCK',
    'SALE',
    'RETURN',
    'DELIVERY',
    'SPOILAGE',
    'CORRECTION',
    'TRANSFER_OUT',
    'TRANSFER_IN'
);

CREATE TABLE stock_movements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    item_id UUID REFERENCES items(id) ON DELETE CASCADE NOT NULL,
    location_id UUID REFERENCES locations(id) ON DELETE CASCADE NOT NULL,
    quantity_changed NUMERIC(10, 2) NOT NULL,
    reason stock_movement_reason NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Para acelerar buscas de histórico por item ou por local
CREATE INDEX idx_movements_item ON stock_movements (tenant_id, item_id);
CREATE INDEX idx_movements_location ON stock_movements (tenant_id, location_id);

-- Para acelerar relatórios por data
CREATE INDEX idx_movements_created_at ON stock_movements (tenant_id, created_at DESC);

-- Para acelerar a busca de itens por categoria
CREATE INDEX idx_items_category ON items (tenant_id, category_id);

-- Para acelerar a busca de níveis de estoque de um local específico
-- (O índice unique principal já cobre (tenant, item, location),
-- mas não cobre bem (tenant, location) )
CREATE INDEX idx_levels_location ON inventory_levels (tenant_id, location_id);

-- Para acelerar buscas de locais dentro de um "pool"
CREATE INDEX idx_locations_pool ON locations (tenant_id, stock_pool_id);