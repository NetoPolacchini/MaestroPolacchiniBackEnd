```mermaid
sequenceDiagram
    autonumber
    participant App as Rust Backend (Pool)
    participant DB as PostgreSQL
    participant RLS as RLS Policy Engine

    Note over App, DB: Usuário autenticado e Tenant identificado

    App->>DB: BEGIN;
    
    %% O passo crucial do RLS
    rect rgb(255, 240, 240)
        Note right of App: Contexto Injetado
        App->>DB: SET LOCAL app.tenant_id = 'uuid-do-tenant';
    end

    App->>DB: SELECT * FROM items;
    
    %% O que acontece dentro do banco
    DB->>RLS: Intercepta Query
    RLS->>RLS: Verifica: current_setting('app.tenant_id')
    RLS->>DB: Filtra linhas invisivelmente
    
    DB-->>App: Retorna apenas linhas do Tenant
    App->>DB: COMMIT;