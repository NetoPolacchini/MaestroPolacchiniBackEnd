-- migrations/20251231020852_create_financial_schema.sql

-- 1. Tipos Básicos
CREATE TYPE title_kind AS ENUM ('RECEIVABLE', 'PAYABLE'); -- A Receber vs A Pagar
CREATE TYPE title_status AS ENUM ('PENDING', 'PARTIAL', 'PAID', 'CANCELLED', 'OVERDUE');

-- 2. Contas (Onde o dinheiro mora)
-- Ex: "Caixa Físico", "Itaú", "Nubank"
CREATE TABLE financial_accounts (
                                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

                                    name VARCHAR(100) NOT NULL,
                                    bank_name VARCHAR(100), -- Opcional

    -- Cache do Saldo (Atualizado via triggers ou aplicação)
                                    current_balance NUMERIC(15, 2) NOT NULL DEFAULT 0,

                                    is_active BOOLEAN DEFAULT TRUE,
                                    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Categorias Financeiras (Plano de Contas simplificado)
-- Ex: "Receita de Vendas", "Despesa com Aluguel", "Pró-Labore"
CREATE TABLE financial_categories (
                                      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                      tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

                                      name VARCHAR(100) NOT NULL,
                                      kind title_kind NOT NULL, -- Se é categoria de Entrada ou Saída

                                      is_active BOOLEAN DEFAULT TRUE
);

-- 4. Títulos (A Promessa de Pagamento/Recebimento)
-- Aqui fica a "Conta de Luz de Janeiro" ou a "Venda do Pedido #1050"
CREATE TABLE financial_titles (
                                  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

    -- De onde veio isso?
                                  description VARCHAR(255) NOT NULL,

    -- Vínculos Opcionais
                                  customer_id UUID REFERENCES customers(id) ON DELETE SET NULL, -- Se for venda
                                  order_id UUID REFERENCES orders(id) ON DELETE SET NULL,       -- Se veio de Operações
                                  category_id UUID REFERENCES financial_categories(id) ON DELETE RESTRICT,

                                  kind title_kind NOT NULL,
                                  status title_status NOT NULL DEFAULT 'PENDING',

    -- Valores
                                  amount_original NUMERIC(15, 2) NOT NULL, -- Valor do Boleto
                                  amount_balance NUMERIC(15, 2) NOT NULL,  -- Quanto falta pagar (Suporta pagamento parcial)

    -- Datas
                                  due_date DATE NOT NULL, -- Vencimento
                                  competence_date DATE NOT NULL DEFAULT CURRENT_DATE, -- Para DRE (Regime de Competência)

                                  created_at TIMESTAMPTZ DEFAULT NOW(),
                                  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 5. Movimentações (O Dinheiro de verdade)
-- Quando você paga um boleto, gera uma linha aqui.
CREATE TABLE financial_movements (
                                     id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                     tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

                                     account_id UUID NOT NULL REFERENCES financial_accounts(id), -- Saiu de qual banco?
                                     title_id UUID REFERENCES financial_titles(id),              -- Pagou qual conta?

                                     amount NUMERIC(15, 2) NOT NULL, -- Valor efetivamente pago/recebido

    -- Se amount < 0 é SAÍDA, se > 0 é ENTRADA.
    -- Geralmente PAYABLE gera movimento negativo, RECEIVABLE gera positivo.

                                     movement_date DATE NOT NULL DEFAULT CURRENT_DATE,
                                     created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Índices
CREATE INDEX idx_titles_status ON financial_titles(tenant_id, status);
CREATE INDEX idx_titles_duedate ON financial_titles(tenant_id, due_date);
CREATE INDEX idx_movements_account ON financial_movements(tenant_id, account_id);

-- RLS
ALTER TABLE financial_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE financial_categories ENABLE ROW LEVEL SECURITY;
ALTER TABLE financial_titles ENABLE ROW LEVEL SECURITY;
ALTER TABLE financial_movements ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_iso_accounts ON financial_accounts FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);
CREATE POLICY tenant_iso_categories ON financial_categories FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);
CREATE POLICY tenant_iso_titles ON financial_titles FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);
CREATE POLICY tenant_iso_movements ON financial_movements FOR ALL USING (tenant_id = current_setting('app.tenant_id')::uuid);

GRANT ALL ON financial_accounts, financial_categories, financial_titles, financial_movements TO "user";