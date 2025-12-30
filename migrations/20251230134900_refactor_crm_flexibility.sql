-- migrations/20251230134900_refactor_crm_flexibility.sql

-- 1. Cria a Tabela de Tipos de Entidade (Essa é nova, então CREATE TABLE)
CREATE TABLE crm_entity_types (
                                  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

                                  name VARCHAR(100) NOT NULL, -- Ex: "Paciente"
                                  slug VARCHAR(100) NOT NULL, -- Ex: "paciente"

                                  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

                                  UNIQUE(tenant_id, slug)
);

-- 2. Atualiza a Tabela de Definições de Campos (Já existe)
ALTER TABLE crm_field_definitions
    ADD COLUMN entity_type_id UUID REFERENCES crm_entity_types(id) ON DELETE CASCADE;

-- Ajuste de Unicidade: Precisamos remover a regra antiga que impedia chaves duplicadas no tenant todo
ALTER TABLE crm_field_definitions
DROP CONSTRAINT IF EXISTS crm_field_definitions_tenant_id_key_name_key;

-- Criamos novas regras de unicidade:
-- A: Campos GLOBAIS (entity_type_id IS NULL) não podem repetir nome
CREATE UNIQUE INDEX idx_crm_fields_global_unique
    ON crm_field_definitions (tenant_id, key_name)
    WHERE entity_type_id IS NULL;

-- B: Campos ESPECÍFICOS (entity_type_id IS NOT NULL) não podem repetir nome dentro do mesmo tipo
CREATE UNIQUE INDEX idx_crm_fields_type_unique
    ON crm_field_definitions (tenant_id, entity_type_id, key_name)
    WHERE entity_type_id IS NOT NULL;


-- 3. Atualiza a Tabela de Clientes (Já existe)
ALTER TABLE customers
    ADD COLUMN entity_types UUID[] DEFAULT '{}'; -- Array de IDs dos tipos

-- Nota: 'custom_data' já existe na migração 20251129, não precisamos adicionar.

-- 4. Cria o índice para busca rápida nos tipos (ex: "Buscar todos os Pacientes")
CREATE INDEX idx_customers_entity_types ON customers USING GIN (entity_types);


-- 5. Configurações de Segurança (RLS) para a tabela nova
ALTER TABLE crm_entity_types ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_entity_types ON crm_entity_types
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

GRANT SELECT, INSERT, UPDATE, DELETE ON crm_entity_types TO "user";