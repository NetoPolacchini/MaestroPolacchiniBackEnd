```mermaid
graph TD
    %% Fontes Externas
    ENV[Variáveis de Ambiente] --> |DATABASE_URL| Builder
    FS[Sistema de Arquivos] --> |./locale/*.json| I18n[I18nStore]

    subgraph "Composition Root (AppState::new)"
        Builder[Inicializador]
        Pool[PgPool (Conexão DB)]
        
        %% Camada de Repositórios
        subgraph Repositories
            UR[UserRepository]
            IR[InventoryRepository]
            TR[TenantRepository]
            CR[CrmRepository]
        end

        %% Camada de Serviços
        subgraph Services
            AS[AuthService]
            IS[InventoryService]
            TS[TenantService]
            CS[CrmService]
        end

        %% Estado Final
        FinalState((AppState))
    end

    %% Fluxo de Dependência
    Builder --> Pool
    Builder --> I18n

    %% Injeção nos Repositórios
    Pool --> UR
    Pool --> IR
    Pool --> TR
    Pool --> CR

    %% Injeção nos Serviços
    UR --> AS
    IR --> IS
    TR --> TS
    CR --> CS
    
    %% Casos onde o serviço precisa do Pool direto (Transações manuais)
    Pool -.-> AS
    Pool -.-> IS
    Pool -.-> TS

    %% Montagem Final
    AS --> FinalState
    IS --> FinalState
    TS --> FinalState
    CS --> FinalState
    I18n --> FinalState
```