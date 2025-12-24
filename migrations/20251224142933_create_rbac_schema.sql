-- migrations/20251224142933_create_rbac_schema.sql

-- 1. TABELA DE PERMISSÕES (Globais do Sistema)
-- O dono da loja NÃO cria permissões, ele apenas as seleciona.
CREATE TABLE permissions (
                             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                             slug VARCHAR(100) UNIQUE NOT NULL, -- A chave que usamos no código Rust (ex: 'inventory:write')
                             description VARCHAR(255) NOT NULL, -- Texto bonitinho para o Frontend mostrar
                             module VARCHAR(50) NOT NULL,       -- Para agrupar na tela (ex: 'ESTOQUE', 'FINANCEIRO')
                             created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. TABELA DE CARGOS (Por Tenant)
-- O dono da loja cria: "Caixa da Noite", "Gerente Geral"
CREATE TABLE roles (
                       id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                       tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,

                       name VARCHAR(100) NOT NULL,
                       description TEXT,

                       created_at TIMESTAMPTZ DEFAULT NOW(),
                       updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Não faz sentido ter dois cargos com mesmo nome na mesma loja
                       UNIQUE(tenant_id, name)
);

-- 3. VÍNCULO CARGO <-> PERMISSÕES (Tabela Pivô)
CREATE TABLE role_permissions (
                                  role_id UUID REFERENCES roles(id) ON DELETE CASCADE NOT NULL,
                                  permission_id UUID REFERENCES permissions(id) ON DELETE CASCADE NOT NULL,
                                  PRIMARY KEY(role_id, permission_id)
);

-- 4. MEMBROS DO TENANT (Quem trabalha lá?)
-- Esta tabela vincula o Usuário (Global) à Loja (Tenant) com um Cargo (Role).
CREATE TABLE tenant_members (
                                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE NOT NULL,
                                user_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,

    -- Se deletar o cargo, não pode deletar o membro (tem que reatribuir cargo antes)
                                role_id UUID REFERENCES roles(id) ON DELETE RESTRICT NOT NULL,

                                is_active BOOLEAN DEFAULT TRUE, -- Dono pode "bloquear" o acesso sem apagar o histórico
                                joined_at TIMESTAMPTZ DEFAULT NOW(),

    -- Um usuário só pode ter UM cargo por loja (simplifica a lógica)
                                UNIQUE(tenant_id, user_id)
);

-- ---
-- SEED INICIAL (Permissões básicas)
-- ---
INSERT INTO permissions (slug, description, module) VALUES
-- Módulo de Estoque
('inventory:read',  'Visualizar itens e saldo', 'INVENTORY'),
('inventory:write', 'Criar itens e ajustar estoque', 'INVENTORY'),
('inventory:sell',  'Realizar vendas (baixa de estoque)', 'INVENTORY'),

-- Módulo CRM
('crm:read',   'Visualizar lista de clientes', 'CRM'),
('crm:write',  'Cadastrar e editar clientes', 'CRM'),

-- Módulo Administrativo (Tenant)
('tenant:admin', 'Gerenciar configurações da loja e cargos', 'ADMIN');