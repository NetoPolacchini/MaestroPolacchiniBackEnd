-- migrations/20251129020907_create_crm_schema.sql

-- ---
-- 1. Definições de Campos Personalizados (O Molde)
-- ---

-- Enum para garantir que o tipo do campo seja controlado
CREATE TYPE crm_field_type AS ENUM ('TEXT', 'NUMBER', 'DATE', 'BOOLEAN', 'SELECT', 'MULTISELECT');

CREATE TABLE crm_field_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,

    -- Ex: "Peso", "Time de Futebol"
    name VARCHAR(255) NOT NULL,

    -- Ex: "weight", "soccer_team" (usado como chave no JSON)
    key_name VARCHAR(255) NOT NULL,

    field_type crm_field_type NOT NULL,

    -- Para campos do tipo SELECT/MULTISELECT. Ex: ["Flamengo", "Vasco"]
    options JSONB DEFAULT NULL,

    is_required BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Garante que não existam duas chaves iguais no mesmo Tenant
    UNIQUE(tenant_id, key_name)
);

-- ---
-- 2. Tabela de Clientes (O Dado)
-- ---

CREATE TABLE customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,

    -- Dados Fixos (Todo negócio precisa)
    full_name VARCHAR(255) NOT NULL,

    -- Documento (CPF/CNPJ/Passaporte).
    -- Pode ser NULL (ex: restaurante que só pega nome), mas se tiver, deve ser único na empresa.
    document_number VARCHAR(50),

    birth_date DATE,

    -- Contatos
    email VARCHAR(255),
    phone VARCHAR(50),  -- Fixo
    mobile VARCHAR(50), -- Celular/WhatsApp

    -- Endereço Flexível (Rua, Número, Bairro, CEP, Cidade, Estado...)
    -- Usamos JSONB para não criar 10 colunas que ficam vazias na maioria das vezes
    address JSONB DEFAULT '{}'::jsonb,

    -- Tags para segmentação (Ex: ["VIP", "Inadimplente", "Vegano"])
    tags TEXT[],

    -- AQUI MORA A MÁGICA: Os dados dos campos personalizados
    -- Ex: { "weight": 80.5, "allergies": ["Glúten"] }
    custom_data JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Regra de negócio: Não duplicar documento dentro da mesma empresa
    UNIQUE(tenant_id, document_number)
);

-- Trigger para atualizar o updated_at automaticamente
CREATE TRIGGER set_timestamp BEFORE UPDATE ON customers FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();


-- ---
-- 3. Índices de Performance (GIN Index para JSONB)
-- ---
-- Isso permite buscar dentro do JSON com velocidade de raio.
-- Ex: Buscar todos os clientes onde custom_data->'weight' > 80
CREATE INDEX idx_customers_custom_data ON customers USING GIN (custom_data);
CREATE INDEX idx_customers_address ON customers USING GIN (address);
CREATE INDEX idx_customers_tags ON customers USING GIN (tags);

-- Índices normais
CREATE INDEX idx_customers_name ON customers (tenant_id, full_name);
CREATE INDEX idx_customers_email ON customers (tenant_id, email);


-- ---
-- 4. SEGURANÇA (RLS - Row Level Security)
-- ---

-- Habilita segurança na tabela de definições
ALTER TABLE crm_field_definitions ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON crm_field_definitions
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

-- Habilita segurança na tabela de clientes
ALTER TABLE customers ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_policy ON customers
    FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);


-- ---
-- 5. Permissões
-- ---
GRANT SELECT, INSERT, UPDATE, DELETE ON crm_field_definitions, customers TO "user";