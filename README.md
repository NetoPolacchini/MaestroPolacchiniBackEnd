## Executar o projeto
docker-compose up --build

src/
├── bin/          # Se tiver mais de um executável (opcional)
├── common/       # Utilitários, constantes e códigos compartilhados
    ├── error.rs
    ├──mod.rs
├── config/       # Configurações do ambiente (ex: banco de dados, chaves)
├── db/           # Camada de acesso ao banco de dados (Repository pattern)
├── handlers/     # Lógica de controle de requisições
    ├── auth.rs
    ├── mod.rs
├──middleware
    ├── auth.rs
    ├── mod.rs
├── models/       # Estruturas de dados (structs)
    ├── auth.rs
    ├── mod.rs
├── services/     # Lógica de negócio
└── main.rs       # Ponto de entrada da aplicação