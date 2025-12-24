# Estado Global e Injeção de Dependência

Em Rust, a gestão de dependências é explícita. O arquivo `config.rs` atua como a **Raiz de Composição**, inicializando recursos caros (como o Pool de conexões do Banco) e injetando-os nas camadas superiores.

## O Objeto `AppState`

O `AppState` é a estrutura Singleton que vive durante todo o ciclo de vida da aplicação. Ele é clonado atomicamente (usando `Arc` internamente pelo Axum) e passado para cada *Thread* que atende uma requisição HTTP.

```mermaid
classDiagram
    class AppState {
        +PgPool db_pool
        +String jwt_secret
        +I18nStore i18n_store
        +AuthService auth_service
        +InventoryService inventory_service
        +TenantService tenant_service
        +CrmService crm_service
    }

    class Services {
        <<Logic Layer>>
        AuthService
        InventoryService
        TenantService
        CrmService
    }

    class Repositories {
        <<Data Layer>>
        UserRepository
        InventoryRepository
        TenantRepository
        CrmRepository
    }

    AppState *-- Services : "Contém"
    AppState *-- Repositories : "Contém (Opcional/Legacy)"
    Services o-- Repositories : "Usa"
    Repositories o-- PgPool : "Usa Conexão"