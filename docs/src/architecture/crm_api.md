# Interface da API (Handlers & DTOs)

A camada de Handlers atua como a porta de entrada HTTP. Ela não contém regras de negócio complexas; sua responsabilidade é:
1.  **Extrair dados:** Converter JSON, Query Params e Contexto (Tenant) em tipos Rust.
2.  **Validar formato:** Verificar campos obrigatórios, tamanho de strings e formato de email (usando `validator`).
3.  **Despachar:** Chamar o Serviço ou Repositório apropriado.

## Diagrama de Classes (Payloads)

Este diagrama mostra a estrutura dos dados recebidos (Input DTOs).

```mermaid
classDiagram
    direction RL
    
    class CrmHandler {
        <<Module>>
        +create_field_definition()
        +create_customer()
        +list_customers()
    }

    class CreateCustomerPayload {
        <<DTO>>
        +String full_name
        +Option~String~ email
        +Option~String~ document_number
        +Value custom_data
        --
        +validate() Result
    }

    class CreateFieldPayload {
        <<DTO>>
        +String name
        +String key_name
        +CrmFieldType field_type
        +bool is_required
        --
        +validate() Result
    }

    %% Relação de Dependência
    CrmHandler ..> CreateCustomerPayload : "Receives JSON"
    CrmHandler ..> CreateFieldPayload : "Receives JSON"