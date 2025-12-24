## 2. Esquema Completo do Banco de Dados (Atualizado)

Este diagrama reflete o estado acumulado de todas as migrations, incluindo Autenticação, Multi-tenancy, Inventário, Precificação e **Controle de Lotes (Batches)**.

```mermaid
erDiagram
    %% --- ACESSO E SEGURANÇA ---
    users ||--o{ user_tenants : "possui acesso via"
    tenants ||--o{ user_tenants : "tem membros"
    
    users {
        uuid id PK
        varchar email UK
        varchar password_hash "Obrigatório"
        timestamptz created_at
        timestamptz updated_at
    }

    tenants {
        uuid id PK
        varchar name
        text description
        timestamptz created_at
        timestamptz updated_at
    }

    user_tenants {
        uuid user_id PK, FK
        uuid tenant_id PK, FK
        timestamptz created_at
    }

    %% --- ZONA PROTEGIDA POR RLS (DADOS DO INQUILINO) ---
    %% Tudo abaixo desta linha pertence a um tenant_id específico e requer 'app.tenant_id'

    %% CATÁLOGO DE PRODUTOS
    tenants ||--o{ items : "catálogo"
    categories ||--o{ items : "classifica"
    units_of_measure ||--o{ items : "medido em"
    categories |o--o{ categories : "sub-categoria"

    items {
        uuid id PK
        uuid tenant_id FK
        varchar sku UK
        varchar name
        text description
        numeric default_price "Preço Base"
        uuid category_id FK
        uuid base_unit_id FK
        timestamptz created_at
        timestamptz updated_at
    }

    categories {
        uuid id PK
        uuid parent_id FK
        varchar name
        text description
    }

    units_of_measure {
        uuid id PK
        varchar name
        varchar symbol
    }

    %% ESTRUTURA FÍSICA (LOCAIS)
    tenants ||--o{ stock_pools : "gere"
    stock_pools ||--o{ locations : "agrupa"

    stock_pools {
        uuid id PK
        uuid tenant_id FK
        varchar name
        text description
    }

    locations {
        uuid id PK
        uuid stock_pool_id FK
        varchar name
        boolean is_warehouse
    }

    %% SALDOS, LOTES E PRECIFICAÇÃO
    items ||--o{ inventory_levels : "possui saldo macro em"
    locations ||--o{ inventory_levels : "armazena macro"
    
    items ||--o{ inventory_batches : "rastreamento detalhado"
    locations ||--o{ inventory_batches : "armazena lotes"

    inventory_levels {
        uuid id PK
        uuid item_id FK
        uuid location_id FK
        numeric quantity "Físico Total (Soma dos Lotes)"
        numeric reserved_quantity "Comprometido"
        numeric low_stock_threshold
        numeric sale_price "Preço Venda"
        numeric average_cost "Custo Médio Ponderado"
        timestamptz updated_at
    }

    inventory_batches {
        uuid id PK
        uuid item_id FK
        uuid location_id FK
        varchar batch_number "Lote/Série"
        varchar position "Posição (Bin/Prateleira) - Novo"
        date expiration_date
        numeric quantity
        numeric unit_cost
        timestamptz created_at
    }

    %% HISTÓRICO E AUDITORIA
    items ||--o{ stock_movements : "audit trail"
    locations ||--o{ stock_movements : "audit trail"

    stock_movements {
        uuid id PK
        uuid item_id FK
        uuid location_id FK
        numeric quantity_changed
        enum reason
        numeric unit_cost
        numeric unit_price
        varchar position "Rastreio de Posição - Novo"
        text notes
        timestamptz created_at
    }
    
    %% --- CRM E CLIENTES (Híbrido) ---
    tenants ||--o{ customers : "gere carteira"
    tenants ||--o{ crm_field_definitions : "configura campos extras"

    customers {
        uuid id PK
        uuid tenant_id FK
        varchar full_name
        varchar document_number
        varchar email
        jsonb address "Endereço Flexível"
        text_array tags "Segmentação (Array)"
        jsonb custom_data "Dados Dinâmicos (Schema-less)"
        timestamptz created_at
    }

    crm_field_definitions {
        uuid id PK
        uuid tenant_id FK
        varchar name "Label Visual"
        varchar key_name "Chave no JSON"
        enum field_type "TEXT, NUMBER, SELECT..."
        jsonb options "Opções p/ Select"
        boolean is_required
    }

 