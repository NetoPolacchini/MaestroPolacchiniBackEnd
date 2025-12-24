# Middleware de Segurança e Contexto

O sistema utiliza o padrão de **Guards** (Guardiões) para interceptar todas as requisições HTTP antes que elas cheguem aos Handlers.

Existem dois níveis de proteção, dependendo da sensibilidade da rota.

## Estratégia de Proteção

| Middleware | Função | Onde é usado |
| :--- | :--- | :--- |
| **`auth_guard`** | Apenas valida se o JWT é válido e quem é o usuário. | Rotas globais (ex: Listar meus tenants, Criar novo tenant). |
| **`tenant_guard`** | Valida o JWT **E** verifica no banco se o usuário pertence àquele Inquilino. | Rotas de negócio (CRM, Inventário, Vendas). |

## Fluxo de Execução (Tenant Guard)

O diagrama abaixo detalha o funcionamento do `tenant_guard`, que é o mais restritivo. Ele aborta a requisição imediatamente se qualquer verificação falhar.

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant Guard as Tenant Guard
    participant AuthSvc as AuthService
    participant Repo as TenantRepo
    participant Axum as Axum Extensions
    participant Handler as Route Handler

    Client->>Guard: Request (Header: Bearer Token + X-Tenant-ID)

    %% FASE 1: QUEM É VOCÊ?
    Note over Guard: 1. Autenticação (CPU Bound)
    Guard->>AuthSvc: validate_token(jwt)
    alt Token Inválido / Expirado
        AuthSvc-->>Guard: Erro
        Guard-->>Client: 401 Unauthorized
    else Token Válido
        AuthSvc-->>Guard: Retorna Struct User
    end

    %% FASE 2: VOCÊ PODE ENTRAR AQUI?
    Note over Guard: 2. Autorização (I/O Bound)
    Guard->>Repo: check_user_tenancy(user_id, tenant_id)
    alt Sem Acesso ao Tenant
        Repo-->>Guard: False
        Guard-->>Client: 403 Forbidden
    else Acesso Permitido
        Repo-->>Guard: True
        
        %% FASE 3: INJEÇÃO DE DEPENDÊNCIA
        Note over Guard, Handler: 3. Enriquecimento do Contexto
        Guard->>Axum: extensions.insert(User)
        Guard->>Axum: extensions.insert(TenantContext)
        
        Guard->>Handler: next.run(req)
        Handler-->>Client: Response (200 OK)
    end