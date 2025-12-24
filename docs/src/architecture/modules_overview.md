# Visão Geral da Aplicação (Entry Point)

A aplicação utiliza o framework **Axum** e segue uma arquitetura modular. A `main.rs` atua como o orquestrador, inicializando o `AppState` (conexão com banco) e compondo as rotas.

## Mapa de Rotas e Módulos

O diagrama abaixo ilustra como as rotas são agrupadas e quais middlewares protegem cada grupo.

```mermaid
graph TD
    User([Cliente / Frontend]) --> Server[Axum Server :3000]
    
    subgraph "Camada de Roteamento (main.rs)"
        Server --> API{/api}
        
        %% Rotas Públicas
        API -->|/auth| AuthRoutes[Auth Module]
        AuthRoutes --> R1(/login)
        AuthRoutes --> R2(/register)

        %% Rotas Protegidas (Apenas Login)
        API -->|/users| UserRoutes[User Module]
        UserRoutes -->|auth_guard| R3(/me)
        
        API -->|/tenants| TenantRoutes[Tenancy Module]
        TenantRoutes -->|auth_guard| R4(/ - list/create)

        %% Rotas Protegidas (Login + Tenant Selecionado)
        API -->|/inventory| InvRoutes[Inventory Module]
        InvRoutes -->|tenant_guard| R5(/items, /sell...)

        API -->|/crm| CRMRoutes[CRM Module]
        CRMRoutes -->|tenant_guard| R6(/customers, /fields)
        
        API -->|/tenants/setup| SetupRoutes[Tenant Setup]
        SetupRoutes -->|tenant_guard| R7(/pools, /locations)
    end

    subgraph "Estado Global"
        State[(AppState)]
        State -.->|Injeção| AuthRoutes
        State -.->|Injeção| InvRoutes
        State -.->|Injeção| CRMRoutes
    end
```