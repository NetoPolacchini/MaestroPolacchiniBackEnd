-- migrations/20260103195311_create_tenant_settings.sql

CREATE TABLE tenant_settings (
                                 tenant_id UUID PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,

    -- Identidade Visual
                                 logo_url VARCHAR(255),
                                 primary_color VARCHAR(7) DEFAULT '#000000', -- Hex code

    -- Dados de Contato (Para o Rodapé do PDF)
                                 company_name VARCHAR(150), -- Razão Social ou Nome Fantasia
                                 document_number VARCHAR(20), -- CNPJ/CPF
                                 address TEXT,
                                 phone VARCHAR(20),
                                 email VARCHAR(100),

    -- Pagamento Pix (Modo Manual)
                                 pix_key VARCHAR(100),
                                 pix_key_type VARCHAR(20), -- CPF, CNPJ, EMAIL, PHONE, RANDOM

                                 updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- RLS (Segurança)
ALTER TABLE tenant_settings ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_policy ON tenant_settings
    USING (tenant_id = current_setting('app.tenant_id')::uuid);

GRANT ALL ON tenant_settings TO "user";