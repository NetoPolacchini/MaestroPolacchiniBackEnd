
docker compose up --build

para adicionar algo
sqlx migrate add 

rodar localmente sem docker
docker run --name rust-backend-db -e POSTGRES_USER=user -e POSTGRES_PASSWORD=password -e POSTGRES_DB=meu_app -p 5432:5432 -d postgres
sqlx migrate run
cargo run

Gerar a documentacao
mdbook serve docs