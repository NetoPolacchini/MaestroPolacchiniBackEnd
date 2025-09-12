use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};

// O estado compartilhado que será acessível em toda a aplicação
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub jwt_secret: String,
}

impl AppState {
    // Função para carregar as configurações e criar o AppState
    pub async fn new() -> Self {
        dotenvy::dotenv().expect("Falha ao carregar o arquivo .env");
        tracing_subscriber::fmt().with_target(false).compact().init();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL deve ser definida");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET deve ser definido");

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

        Self { db_pool, jwt_secret }
    }
}