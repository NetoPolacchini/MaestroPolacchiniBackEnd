-- Add migration script here
-- migrations/xxxxxxxx_enable_row_level_security.sql

-- ---
-- 1. Ativa o RLS em todas as tabelas de dados do Tenant
-- ---
-- O "FOR ALL" aplica-se a SELECT, INSERT, UPDATE, DELETE
-- A política "USING" significa que qualquer consulta SÓ pode ver/modificar
-- linhas onde o tenant_id corresponde à variável 'app.tenant_id'
-- que a nossa API Rust irá definir.

-- Tabelas de Locais
ALTER TABLE stock_pools ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON stock_pools
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

ALTER TABLE locations ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON locations
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

-- Tabelas de Definição de Inventário
ALTER TABLE units_of_measure ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON units_of_measure
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

ALTER TABLE categories ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON categories
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

ALTER TABLE items ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON items
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

-- Tabelas de Dados de Inventário
ALTER TABLE inventory_levels ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON inventory_levels
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

ALTER TABLE stock_movements ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON stock_movements
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

-- ---
-- 2. Concede permissões ao utilizador da aplicação
-- ---
-- O RLS aplica-se por defeito ao "dono" da tabela (o utilizador que
-- correu as migrações). Temos de garantir que o utilizador que a nossa
-- APLICAÇÃO usa (definido no DATABASE_URL) tenha permissão para
-- aceder a estas tabelas (ele será sujeito às políticas acima).

-- Assumindo que o utilizador da app no seu .env é 'user' (do docker-compose.yml)
GRANT SELECT, INSERT, UPDATE, DELETE ON
    stock_pools,
    locations,
    units_of_measure,
    categories,
    items,
    inventory_levels,
    stock_movements
    TO "user";