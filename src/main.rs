// src/main.rs

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::docs::ApiDoc;

mod common;
mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod services;
mod docs;

use crate::config::AppState;
use crate::middleware::auth::{auth_guard, tenant_guard};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).compact().init();

    let app_state = AppState::new()
        .await
        .expect("Falha ao inicializar o estado da aplicação.");

    // Migrações Automáticas
    sqlx::migrate!()
        .run(&app_state.db_pool)
        .await
        .expect("Falha ao rodar as migrações do banco de dados.");

    tracing::info!("✅ Migrações do banco de dados executadas com sucesso!");

    // --- ROTAS PÚBLICAS ---
    let auth_routes = Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login));

    // --- ROTAS AUTENTICADAS (USER) ---
    let user_routes = Router::new()
        .route("/me", get(handlers::auth::get_me))
        .route("/me/companies", get(handlers::auth::get_my_companies))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_guard,
        ));

    let tenancy_routes = Router::new()
        .route("/", post(handlers::tenancy::create_tenant).get(handlers::tenancy::list_my_tenants))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            auth_guard,
        ));

    // --- ROTAS DO TENANT (USER + TENANT) ---

    // 1. Setup & Estoque
    let tenant_setup_routes = Router::new()
        .route("/pools", post(handlers::tenancy::create_stock_pool))
        .route("/locations", post(handlers::tenancy::create_location))
        .layer(axum_middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    let inventory_routes = Router::new()
        .route("/items", post(handlers::inventory::create_item).get(handlers::inventory::get_all_items))
        .route("/items/{id}/composition", post(handlers::inventory::add_composition_item).get(handlers::inventory::get_item_composition))
        .route("/units", post(handlers::inventory::create_unit_of_measure).get(handlers::inventory::get_all_units))
        .route("/categories", post(handlers::inventory::create_category).get(handlers::inventory::get_all_categories))
        .route("/sell", post(handlers::inventory::sell_item))
        .route("/stock-entry", post(handlers::inventory::add_stock))
        .layer(axum_middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    // 2. Operações (CRM & Pedidos)
    let crm_routes = Router::new()
        .route("/fields", post(handlers::crm::create_field_definition).get(handlers::crm::list_field_definitions))
        .route("/customers", post(handlers::crm::create_customer).get(handlers::crm::list_customers))
        .route("/types", post(handlers::crm::create_entity_type).get(handlers::crm::list_entity_types))
        .layer(axum_middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    let operations_routes = Router::new()
        .route("/pipelines", post(handlers::operations::create_pipeline))
        .route("/pipelines/{id}/stages", post(handlers::operations::add_stage))
        .route("/orders", post(handlers::operations::create_order))
        .route("/orders/{id}/items", post(handlers::operations::add_order_item))
        .route("/orders/{id}/transition", post(handlers::operations::transition_order))
        // Nota: A rota de PDF saiu daqui e foi para document_routes
        .layer(axum::middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    // 3. Dashboards & Relatórios
    let dashboard_routes = Router::new()
        .route("/summary", get(handlers::dashboard::get_summary))
        .route("/sales-chart", get(handlers::dashboard::get_sales_chart))
        .route("/top-products", get(handlers::dashboard::get_top_products))
        .layer(axum::middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    // 4. Documentos (PDFs)
    let document_routes = Router::new()
        .route("/orders/{id}/pdf", get(handlers::documents::generate_order_pdf))
        .layer(axum::middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    // 5. Configurações da Loja
    let settings_routes = Router::new()
        .route("/", get(handlers::settings::get_settings).put(handlers::settings::update_settings))
        .layer(axum::middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    // 6. RBAC
    let rbac_routes = Router::new()
        .route("/roles", post(handlers::rbac::create_role))
        .layer(axum_middleware::from_fn_with_state(app_state.clone(), tenant_guard));

    // --- JUNÇÃO FINAL ---
    let app = Router::new()
        .route("/api/health", get(|| async { "OK" }))
        .route("/api/permissions", get(handlers::rbac::list_permissions))
        // Auth Global
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .nest("/api/tenants", tenancy_routes)
        // Tenant Scoped
        .nest("/api/tenants/setup", tenant_setup_routes)
        .nest("/api/inventory", inventory_routes)
        .nest("/api/crm", crm_routes)
        .nest("/api/operations", operations_routes)
        .nest("/api/dashboard", dashboard_routes)
        .nest("/api/documents", document_routes) // Agora em /api/documents/orders/...
        .nest("/api/settings", settings_routes)  // Agora em /api/settings
        .nest("/api/rbac", rbac_routes)          // Ajustei para /api/rbac para não conflitar com /api/tenants
        .with_state(app_state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr)
        .await
        .expect("Falha ao iniciar o listener TCP");

    tracing::info!("🚀 Servidor escutando em {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("Erro no servidor Axum");
}