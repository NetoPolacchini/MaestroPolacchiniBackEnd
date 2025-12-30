-- migrations/20251230165257_catalog_polymorphism_and_composition.sql

-- 1. O Enum que define a natureza do item
CREATE TYPE item_kind AS ENUM ('PRODUCT', 'SERVICE', 'RESOURCE', 'BUNDLE');

-- 2. Atualização da Tabela Principal (Items)
ALTER TABLE items
    -- Se não definirmos default, os itens antigos quebram. Assumimos PRODUCT.
    ADD COLUMN kind item_kind NOT NULL DEFAULT 'PRODUCT',

    -- Configurações Flexíveis (JSONB)
    -- Para SERVIÇO: { "duration_minutes": 60, "allows_overbooking": false }
    -- Para RECURSO: { "max_capacity": 4, "maintenance_interval_days": 30 }
    -- Para PRODUTO: { "perishable": true, "weight_kg": 0.5 }
    ADD COLUMN settings JSONB DEFAULT '{}'::jsonb;

-- 3. Índices para performance no JSON
CREATE INDEX idx_items_kind ON items(tenant_id, kind);
CREATE INDEX idx_items_settings ON items USING GIN (settings);


-- 4. Tabela de Composição (A Engenharia do Produto)
-- Define do que um item é feito (Ex: X-Bacon = Pão + Carne + Bacon)
-- Ou o que um pacote inclui (Ex: Pacote Fitness = 10 Aulas + 1 Avaliação)

CREATE TYPE composition_type AS ENUM ('COMPONENT', 'ACCESSORY', 'SUBSTITUTE');

CREATE TABLE item_compositions (
                                   id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                   tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

                                   parent_item_id UUID NOT NULL REFERENCES items(id) ON DELETE CASCADE,
                                   child_item_id UUID NOT NULL REFERENCES items(id) ON DELETE RESTRICT, -- Não deleta ingrediente se tiver em uso

                                   quantity NUMERIC(15, 4) NOT NULL DEFAULT 1.0, -- Quanto consome?

                                   comp_type composition_type NOT NULL DEFAULT 'COMPONENT',

                                   created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Regras de Ouro:
    -- 1. Um item não pode ter o mesmo ingrediente duas vezes na lista (use update quantidade)
                                   UNIQUE(parent_item_id, child_item_id),

    -- 2. Performance: Buscar "Tudo que vai no X-Bacon"
                                   CONSTRAINT fk_parent FOREIGN KEY (parent_item_id) REFERENCES items(id),

    -- 3. Performance: Buscar "Onde o item Bacon é usado?" (Engenharia reversa)
                                   CONSTRAINT fk_child FOREIGN KEY (child_item_id) REFERENCES items(id)
);

-- Índices vitais para performance de grafos
CREATE INDEX idx_composition_parent ON item_compositions(tenant_id, parent_item_id);
CREATE INDEX idx_composition_child ON item_compositions(tenant_id, child_item_id);

-- RLS (Segurança)
ALTER TABLE item_compositions ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_compositions ON item_compositions
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

GRANT SELECT, INSERT, UPDATE, DELETE ON item_compositions TO "user";