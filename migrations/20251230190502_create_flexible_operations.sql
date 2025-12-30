-- migrations/20251230190502_create_flexible_operations.sql

-- 1. Categorias do Sistema (O Backend precisa disso para saber se está aberto ou fechado)
CREATE TYPE pipeline_category AS ENUM ('DRAFT', 'ACTIVE', 'DONE', 'CANCELLED');

-- 2. Tabela de Pipelines (Workflows)
CREATE TABLE pipelines (
                           id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                           tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                           name VARCHAR(100) NOT NULL, -- Ex: "Delivery", "Balcão", "Manutenção"
                           is_default BOOLEAN DEFAULT FALSE,
                           created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Etapas do Pipeline (As colunas do Kanban)
CREATE TABLE pipeline_stages (
                                 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                 tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                                 pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,

                                 name VARCHAR(100) NOT NULL, -- Ex: "Na Cozinha"
                                 category pipeline_category NOT NULL, -- Ex: ACTIVE

                                 position INTEGER NOT NULL DEFAULT 0,
                                 color VARCHAR(20),

    -- === GATILHOS (A Mágica) ===
    -- Se TRUE, ao mover o card para cá, o sistema executa a ação.

    -- Baixa o estoque físico? (Útil para "Em Produção" antes de "Entregue")
                                 stock_action VARCHAR(20) DEFAULT 'NONE', -- 'NONE', 'RESERVE', 'DEDUCT'

    -- Gera contas a receber? (Útil para cobrar antecipado)
                                 generates_receivable BOOLEAN DEFAULT FALSE,

    -- Trava o pedido para edição? (Ninguém mexe no que está no forno)
                                 is_locked BOOLEAN DEFAULT FALSE,

                                 created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 4. Pedidos (Orders)
CREATE TABLE orders (
                        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                        tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                        customer_id UUID REFERENCES customers(id) ON DELETE SET NULL,

    -- Onde ele está no fluxo?
                        pipeline_id UUID NOT NULL REFERENCES pipelines(id),
                        stage_id UUID NOT NULL REFERENCES pipeline_stages(id),

    -- Cache Financeiro
                        total_amount NUMERIC(15, 4) NOT NULL DEFAULT 0,
                        total_discount NUMERIC(15, 4) NOT NULL DEFAULT 0,

                        display_id SERIAL,
                        tags TEXT[],
                        notes TEXT,

    -- Auditoria
                        opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        closed_at TIMESTAMPTZ, -- Preenchido automaticamente quando category = DONE/CANCELLED

                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Itens do Pedido (Snapshot)
CREATE TABLE order_items (
                             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                             tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                             order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
                             item_id UUID NOT NULL REFERENCES items(id) ON DELETE RESTRICT,

                             quantity NUMERIC(15, 4) NOT NULL DEFAULT 1,
                             unit_price NUMERIC(15, 4) NOT NULL,
                             unit_cost NUMERIC(15, 4) NOT NULL DEFAULT 0,
                             discount NUMERIC(15, 4) NOT NULL DEFAULT 0,
                             notes TEXT,

                             created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Índices e Segurança
CREATE INDEX idx_pipeline_stages_order ON pipeline_stages(pipeline_id, position);
CREATE INDEX idx_orders_stage ON orders(tenant_id, stage_id);

ALTER TABLE pipelines ENABLE ROW LEVEL SECURITY;
ALTER TABLE pipeline_stages ENABLE ROW LEVEL SECURITY;
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE order_items ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_pipelines ON pipelines FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);
CREATE POLICY tenant_isolation_stages ON pipeline_stages FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);
CREATE POLICY tenant_isolation_orders ON orders FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);
CREATE POLICY tenant_isolation_order_items ON order_items FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

GRANT ALL ON pipelines, pipeline_stages, orders, order_items TO "user";