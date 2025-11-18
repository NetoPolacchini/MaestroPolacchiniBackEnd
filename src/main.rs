//src/main.rs

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;

// Declaração dos nossos módulos (isso está perfeito)
mod common;
mod config;
mod db;
mod handlers;
mod middleware; 
mod models;
mod services;

// Importações principais
use crate::config::AppState;
//use crate::handlers; 
use crate::middleware::auth::auth_middleware; // Esta linha agora vai funcionar!

#[tokio::main]
async fn main() {
    // Inicializa o logger, que movemos para o main.
    tracing_subscriber::fmt().with_target(false).compact().init();

    // Lida com o Result retornado por AppState::new()
    // .expect() é bom aqui: se a configuração falhar, a aplicação não deve iniciar.
    let app_state = AppState::new()
        .await
        .expect("Falha ao inicializar o estado da aplicação.");

    // --- CORREÇÃO AQUI ---
    // Faz o app rodar as migrações do SQLx na inicialização
    sqlx::migrate!()
        .run(&app_state.db_pool)
        .await
        .expect("Falha ao rodar as migrações do banco de dados.");

    tracing::info!("✅ Migrações do banco de dados executadas com sucesso!");
    // --- FIM DA CORREÇÃO ---

    // Define as rotas de autenticação (públicas)
    let auth_routes = Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login));

    // Define as rotas de usuário (protegidas pelo middleware)
    let user_routes = Router::new()
        .route("/me", get(handlers::auth::get_me))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ));

    let inventory_routes = Router::new()
        .route("/items"
               ,post(handlers::inventory::create_item)
               .get(handlers::inventory::get_all_items)
        )

        .route("/units"
               ,post(handlers::inventory::create_unit_of_measure)
               .get(handlers::inventory::get_all_units)
        )

        .route("/categories"
               ,post(handlers::inventory::create_category)
               .get(handlers::inventory::get_all_categories)
        )

        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ));

    let tenancy_routes = Router::new()
        .route("/", post(handlers::tenancy::create_tenant))
        // (Futuramente: .get(handlers::tenancy::get_my_tenants))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ));

    // Combina tudo no router principal
    let app = Router::new()
        .route("/api/health", get(|| async { "OK" }))
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .nest("/api/inventory", inventory_routes)
        .nest("/api/tenants", tenancy_routes)
        .with_state(app_state);

    // Inicia o servidor
    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr)
        .await
        .expect("Falha ao iniciar o listener TCP");
    tracing::info!("🚀 Servidor escutando em {}", listener.local_addr().unwrap());
    axum::serve(listener, app) // .into_make_service() não é mais necessário nas versões recentes de Axum
        .await
        .expect("Erro no servidor Axum");
}