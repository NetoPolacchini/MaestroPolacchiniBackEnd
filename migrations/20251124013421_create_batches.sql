-- migrations/20251124013421_create_batches.sql

CREATE TABLE inventory_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
    item_id UUID REFERENCES items(id) ON DELETE CASCADE NOT NULL,
    location_id UUID REFERENCES locations(id) ON DELETE CASCADE NOT NULL,

    batch_number VARCHAR(255) NOT NULL, -- Ex: "LOTE-123" ou "DEFAULT"
    expiration_date DATE, -- Pode ser nulo (Roupas não vencem rápido)

    quantity NUMERIC(10, 2) NOT NULL DEFAULT 0.00,
    unit_cost NUMERIC(10, 2) NOT NULL DEFAULT 0.00, -- Custo deste lote específico

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Garante que não haja dois lotes com mesmo número para o mesmo item no mesmo local
    CONSTRAINT uq_batches_location_batch UNIQUE(tenant_id, item_id, location_id, batch_number)

);

-- Trigger de Timestamp
CREATE TRIGGER set_timestamp BEFORE UPDATE ON inventory_batches FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- --- SEGURANÇA (RLS) ---
-- Não esqueça disso!
ALTER TABLE inventory_batches ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_policy ON inventory_batches
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

GRANT SELECT, INSERT, UPDATE, DELETE ON inventory_batches TO "user";