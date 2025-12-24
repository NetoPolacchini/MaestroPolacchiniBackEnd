# Domínio do CRM (Models)

O módulo de CRM utiliza uma abordagem de **Esquema Híbrido**. Dados essenciais são colunas SQL tradicionais, enquanto dados flexíveis (endereço e campos personalizados) são armazenados como JSONB (`serde_json::Value`).

## Diagrama de Estruturas (Structs)

```mermaid
classDiagram
    class Customer {
        +Uuid id
        +Uuid tenant_id
        +String full_name
        +Option~DocumentType~ document_type
        +Option~String~ document_number
        +Option~String~ email
        +Option~String~ mobile
        +Option~Value~ address
        +Option~Vec_String~ tags
        +Value custom_data
    }

    class CrmFieldDefinition {
        +Uuid id
        +String name
        +String key_name
        +CrmFieldType field_type
        +Option~Value~ options
        +bool is_required
    }

    class CrmFieldType {
        <<enumeration>>
        TEXT
        NUMBER
        DATE
        BOOLEAN
        SELECT
        MULTISELECT
    }

    %% Relação Lógica
    CrmFieldDefinition ..> Customer : "Define a validação do custom_data"