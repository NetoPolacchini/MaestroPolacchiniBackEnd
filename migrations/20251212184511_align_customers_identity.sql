-- 20251212184511_align_customers_identity.sql

-- 1. Link Mágico (Conecta o cadastro da loja ao usuário do App)
ALTER TABLE customers
    ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE SET NULL;

-- 2. Identidade Global no Cliente
ALTER TABLE customers
    ADD COLUMN country_code CHAR(2) DEFAULT 'BR',
    ADD COLUMN document_type document_type DEFAULT 'TAX_ID';

-- 3. Atualizar a Regra de Unicidade
-- Remove a restrição antiga (que olhava só o número)
ALTER TABLE customers
DROP CONSTRAINT IF EXISTS customers_tenant_id_document_number_key;

-- Adiciona a nova restrição (Tenant + País + Tipo + Número)
CREATE UNIQUE INDEX idx_customers_tenant_identity
    ON customers (tenant_id, country_code, document_type, document_number)
    WHERE document_number IS NOT NULL;