## Executar o projeto
docker-compose up --build

src/
├── bin/          # Se tiver mais de um executável (opcional)
├── common/       # Utilitários, constantes e códigos compartilhados
    ├── error.rs
    ├──mod.rs
├── config/       # Configurações do ambiente (ex: banco de dados, chaves)
    ├── mod.rs
├── db/           # Camada de acesso ao banco de dados (Repository pattern)
    ├── mod.rs
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
    ├── auth.rs
    ├── mod.rs
└── main.rs       # Ponto de entrada da aplicação

---

## Executando

### 1. Inicie o Banco de Dados (Apenas o DB)
Vamos iniciar apenas o banco de dados primeiro, para que possamos rodar as migrações nele.

 `docker-compose up -d db`

- `up`: Inicia os serviços.

- `-d`: "Detached mode" (roda em segundo plano).

- `db`: Inicia apenas o serviço db.

Aguarde alguns segundos. O healthcheck no seu docker-compose.yml garantirá que ele esteja pronto.

### 2. Rode as Migrações
 Agora que o banco está rodando em `localhost:5432` (graças ao `ports: "5432:5432"`), rode o comando do `sqlx-cli` da sua máquina:
 
Garanta que esta variável de ambiente aponte para localhost dentro do `.env`

export DATABASE_URL=postgres://user:password@localhost:5432/meu_app

```js
sqlx migrate run
```

Se tudo der certo, você verá o `sqlx-cli` aplicando a migração. Suas tabelas agora existem no volume `postgres_data`!

### 3. Inicie a Aplicação Completa

Agora que o banco está pronto e com as tabelas, suba tudo:

```js
docker-compose up --build
```

- `up`: Inicia todos os serviços no `docker-compose.yml` (vai iniciar o `backend` e ver que o `db` já está rodando).
- `-build`: Força o Docker a reconstruir sua imagem `backend` usando o `Dockerfile`. Isso é bom para garantir que quaisquer mudanças no seu código Rust sejam compiladas.

---
O que vai acontecer
O Docker vai (re)construir sua imagem backend (o estágio builder vai compilar seu Rust, o estágio final vai criar a imagem debian-slim).

O Docker Compose vai ver que o db já está healthy.

O Docker Compose vai iniciar seu contêiner backend.

Seu backend (em main.rs) vai iniciar, ler o .env (com DATABASE_URL=...//@db:5432...), se conectar ao serviço db, e começar a escutar na porta 8000.

Você deve ver o log do tracing no seu terminal: INFO 🚀 Servidor escutando em 0.0.0.0:8000

Se você vir isso, parabéns! Seu servidor está no ar e pronto para receber requisições em http://localhost:8000.

Caso dê algum problema, solucione ele, execute o comento `docker-compose down` e suba tudo novamente


docker compose up --build

rodar localmente sem docker
docker run --name rust-backend-db -e POSTGRES_USER=user -e POSTGRES_PASSWORD=password -e POSTGRES_DB=meu_app -p 5432:5432 -d postgres
sqlx migrate run
cargo run