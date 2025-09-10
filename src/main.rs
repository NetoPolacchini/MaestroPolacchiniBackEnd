// Adicionamos o TcpListener do Tokio
use tokio::net::TcpListener;
use axum::{http::StatusCode, response::Json, routing::get, Router};
use serde_json::{json, Value};
use std::time::Duration;
use sqlx::postgres::{PgPoolOptions};
use std::env;

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt().with_target(false).compact().init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL deve ser definida");

    let db_pool = match PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("✅ Conexão com o banco de dados estabelecida com sucesso!");
            pool
        }
        Err(e) => {
            tracing::error!("🔥 Falha ao conectar ao banco de dados: {:?}", e);
            std::process::exit(1);
        }
    };
    
    // O aviso sobre `_db_pool` pode ser ignorado por enquanto.
    // Usaremos a variável `db_pool` na Fase 1.

    let app = Router::new()
        .route("/api/health", get(health_check_handler));

    // --- BLOCO DE CÓDIGO CORRIGIDO ---
    // 1. Define o endereço para escutar
    let addr = "0.0.0.0:8000";
    
    // 2. Cria um "ouvinte" TCP usando Tokio
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("🔥 Falha ao iniciar o listener TCP: {:?}", e);
            std::process::exit(1);
        }
    };
    
    tracing::info!("🚀 Servidor escutando em {}", listener.local_addr().unwrap());
    
    // 3. Inicia o servidor Axum com o listener do Tokio
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn health_check_handler() -> (StatusCode, Json<Value>) {
    let response = json!({ "status": "ok" });
    (StatusCode::OK, Json(response))
}