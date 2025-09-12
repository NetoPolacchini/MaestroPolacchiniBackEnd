use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;

// Declaração dos nossos módulos
mod common;
mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod services;

// Importações principais
use crate::config::AppState;
use crate::middleware::auth::auth_middleware;

#[tokio::main]
async fn main() {
    // Carrega a configuração e inicializa o estado
    let app_state = AppState::new().await;

    // Define as rotas de autenticação (públicas)
    let auth_routes = Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login));

    // Define as rotas de usuário (protegidas pelo middleware)
    let user_routes = Router::new()
        .route("/me", get(handlers::auth::get_me))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ));

    // Combina tudo no router principal
    let app = Router::new()
        .route("/api/health", get(|| async { "OK" }))
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .with_state(app_state);

    // Inicia o servidor
    let addr = "0.0.0.0:8000";
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("🚀 Servidor escutando em {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service()).await.unwrap();
}