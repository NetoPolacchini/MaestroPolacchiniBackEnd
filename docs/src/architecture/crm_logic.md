# Lógica de Negócio do CRM

A camada de serviço (`CrmService`) atua como guardiã da integridade dos dados. A sua principal responsabilidade é garantir que o JSON flexível (`custom_data`) obedeça às regras definidas pelo inquilino antes de ser persistido.

## Fluxo de Criação de Cliente

Este fluxo ilustra o padrão **"Fetch-Validate-Persist"** (Buscar-Validar-Salvar). Note que utilizamos uma transação para garantir atomicidade.

```mermaid
sequenceDiagram
    autonumber
    participant H as Handler (API)
    participant S as CrmService
    participant V as Validator (Internal)
    participant DB as Repository / Database

    H->>S: create_customer(tenant_id, json_data)
    
    Note over S, DB: Início da Transação
    S->>DB: BEGIN Transaction
    
    %% Passo 1: Buscar o Molde
    S->>DB: list_field_definitions(tenant_id)
    DB-->>S: Retorna [Definição "Peso", Definição "Time"]

    %% Passo 2: Validar
    S->>V: validate_custom_data(definitions, json_data)
    
    alt Dados Inválidos
        V-->>S: Erro (Tipo Incorreto ou Campo Faltante)
        S->>DB: ROLLBACK
        S-->>H: Retorna 400 Bad Request
    else Dados Válidos
        V-->>S: Ok
        %% Passo 3: Persistir
        S->>DB: create_customer(dados_validados)
        DB-->>S: Customer Criado
        S->>DB: COMMIT
        S-->>H: Retorna Customer (201 Created)
    end