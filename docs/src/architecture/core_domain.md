# Modelos do Core (Domínio)

Esta secção documenta as estruturas fundamentais que sustentam a lógica da aplicação, divididas em três grandes pilares: **Autenticação**, **Multi-Inquilinato** e **Inventário**.

## Diagrama de Classes Unificado

```mermaid
classDiagram
    namespace Auth {
        class User {
            +Uuid id
            +String email
            -String password_hash
            +String country_code
            +DocumentType document_type
            +Option~String~ document_number
        }

        class DocumentType {
            <<Enumeration>>
            TAX_ID
            ID_CARD
            PASSPORT
            DRIVER_LICENSE
            OTHER
        }
    }

    namespace Tenancy {
        class Tenant {
            +Uuid id
            +String name
            +Option~String~ description
        }

        class StockPool {
            +Uuid id
            +Uuid tenant_id
            +String name
            +Option~String~ description
        }

        class Location {
            +Uuid id
            +Uuid stock_pool_id
            +String name
            +bool is_warehouse
        }
    }

    namespace Inventory {
        class Item {
            +Uuid id
            +String sku
            +String name
            +Option~Decimal~ default_price
            +Uuid category_id
            +Uuid base_unit_id
        }

        class InventoryLevel {
            +Uuid id
            +Uuid item_id
            +Uuid location_id
            +Decimal quantity
            +Decimal reserved_quantity
            +Decimal average_cost
            +Option~Decimal~ sale_price
        }

        class InventoryBatch {
            +Uuid id
            +String batch_number
            +String position
            +Option~NaiveDate~ expiration_date
            +Decimal quantity
            +Decimal unit_cost
        }

        class StockMovement {
            +Uuid id
            +Decimal quantity_changed
            +StockMovementReason reason
            +Option~String~ position
            +Option~Decimal~ unit_cost
        }

        class StockMovementReason {
            <<Enumeration>>
            INITIAL_STOCK
            PURCHASE
            SALE
            TRANSFER_OUT
            TRANSFER_IN
            ...
        }
    }

    %% Relacionamentos Principais
    User ..> DocumentType : "Possui"
    
    Tenant "1" *-- "*" StockPool : "Contém"
    StockPool "1" *-- "*" Location : "Agrupa"
    
    Location "1" *-- "*" InventoryLevel : "Armazena"
    Item "1" *-- "*" InventoryLevel : "Tem Saldo"
    
    Item "1" *-- "*" InventoryBatch : "Rastreado por"
    InventoryLevel ..> InventoryBatch : "Sumariza"
    
    StockMovement ..> StockMovementReason : "Tipificado por"