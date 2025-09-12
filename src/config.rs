// src/config.rs

use crate::{db::UserRepository, services::auth::AuthService}; // Importe seus serviços e repositórios
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub jwt_secret: String,
    // Adicionamos o serviço ao estado, como discutido
    pub auth_service: AuthService,
}

impl AppState {
    // A assinatura agora retorna um Result!
    pub async fn new() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL deve ser definida");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET deve ser definido");
        
        // Conecta ao banco de dados, usando '?' para propagar erros
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await?; // <-- Se falhar, retorna um Err em vez de dar panic ou exit
        
        tracing::info!("✅ Conexão com o banco de dados estabelecida com sucesso!");

        // --- Monta o gráfico de dependências ---
        let user_repo = UserRepository::new(db_pool.clone());
        let auth_service = AuthService::new(user_repo, jwt_secret.clone());
        
        // Retorna Ok com o estado montado
        Ok(Self {
            db_pool,
            jwt_secret,
            auth_service,
        })
    }
}