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
use crate::middleware::auth::{auth_guard, tenant_guard};

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
        .route("/me/companies", get(handlers::auth::get_my_companies))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_guard,
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
        .route("/sell"
               ,post(handlers::inventory::sell_item)
        )
        .route("/stock-entry"
               ,post(handlers::inventory::add_stock)
        )

        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            tenant_guard,
        ));

    let tenancy_routes = Router::new()
        .route("/"
               ,post(handlers::tenancy::create_tenant)
               .get(handlers::tenancy::list_my_tenants)
        )

        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_guard,
        ));

    let tenant_setup_routes = Router::new()
        .route("/pools"
               ,post(handlers::tenancy::create_stock_pool))
        .route("/locations"
               ,post(handlers::tenancy::create_location))

        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            tenant_guard,
        ));

    let crm_routes = Router::new()
        // Configuração de Campos
        .route("/fields"
               ,post(handlers::crm::create_field_definition)
               .get(handlers::crm::list_field_definitions)
        )
        // Gestão de Clientes
        .route("/customers"
               ,post(handlers::crm::create_customer)
               .get(handlers::crm::list_customers)
        )
        // Aplica o middleware de Auth + Tenancy em tudo
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            tenant_guard,
        ));

    let rbac_routes = Router::new()
        .route("/roles", post(handlers::rbac::create_role));

    // Combina tudo no router principal
    let app = Router::new()
        .route("/api/health", get(|| async { "OK" }))
        .route("/api/permissions", get(handlers::rbac::list_permissions))
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .nest("/api/inventory", inventory_routes)
        .nest("/api/tenants", tenancy_routes)
        .nest("/api/tenants/setup", tenant_setup_routes)
        .nest("/api/crm", crm_routes)
        .nest("/api/tenants", rbac_routes.layer(
            axum_middleware::from_fn_with_state(app_state.clone(), tenant_guard)
        ))
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