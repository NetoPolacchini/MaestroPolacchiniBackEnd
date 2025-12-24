# Tratamento de Erros e Internacionalização

O sistema implementa uma estratégia centralizada de tratamento de erros. Em vez de espalhar `try/catch` ou tratamentos de `Result` pelos controllers, nós convertemos qualquer falha em um `AppError` e deixamos o sistema decidir como apresentá-lo.

## Fluxo de Transformação

O diagrama abaixo ilustra como um erro de banco de dados ou de validação viaja até o cliente, sendo traduzido e higienizado no processo.

```mermaid
flowchart LR
    %% Fontes de Erro
    DB[(PostgreSQL)] -->|sqlx::Error| Service
    Logic[Lógica de Negócio] -->|ValidationErrors| Service
    
    %% Onde o erro é capturado
    Service -->|Result::Err| Handler[API Handler]
    
    %% A Transformação Mágica
    subgraph "Tratamento Centralizado (error.rs)"
        Handler -->|AppError| Transform{to_api_error}
        
        Transform -->|Match| Log[Log no Terminal]
        Transform -->|Locale + I18nStore| Translate[Busca Tradução]
        
        Log -.->|Warn| UserErr[Erro de Cliente]
        Log -.->|Error| SysErr[Erro de Servidor]
        
        Translate --> ApiError[Struct ApiError]
    end
    
    %% Saída
    ApiError -->|IntoResponse| JSON[JSON Response + HTTP Code]