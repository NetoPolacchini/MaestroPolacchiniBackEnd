// src/config.rs

// Importe dos serviços
use crate::services::{
    auth::AuthService,
    inventory_service::InventoryService,
    tenancy_service::TenantService, // Nome correto da Struct
    crm_service::CrmService,
    rbac_service::RbacService,
    operation_service::OperationsService,
    dashboard_service::DashboardService,
    document_service::DocumentService,
};

// Importe dos repositórios
use crate::db::{UserRepository, InventoryRepository, TenantRepository, CrmRepository, RbacRepository};

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use crate::services::finance_service::FinanceService;

pub type I18nStore = Arc<HashMap<String, HashMap<String, String>>>;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub jwt_secret: String,
    pub auth_service: AuthService,
    pub i18n_store: I18nStore,
    pub inventory_service: InventoryService,
    pub inventory_repo: InventoryRepository,
    pub tenant_repo: TenantRepository,
    pub tenant_service: TenantService, // Nome do campo
    pub crm_repo: CrmRepository,
    pub crm_service: CrmService,
    pub rbac_repo: RbacRepository,
    pub rbac_service: RbacService,
    pub operations_service: OperationsService,
    pub finance_service: FinanceService,
    pub dashboard_service: DashboardService,
    pub document_service: DocumentService
}

// Uma função helper para carregar os arquivos
fn load_translations() -> anyhow::Result<I18nStore> {
    let mut store = HashMap::new();
    // Verifica se a pasta existe antes de ler, para evitar panic em ambiente limpo
    if let Ok(paths) = fs::read_dir("./locale") {
        for path in paths {
            let path = path?.path();
            if path.extension().map_or(false, |e| e == "json") {
                let lang_code = path.file_stem().unwrap().to_str().unwrap().to_string();
                let file_content = fs::read_to_string(path)?;
                let translations: HashMap<String, String> = serde_json::from_str(&file_content)?;
                store.insert(lang_code, translations);
            }
        }
    }
    Ok(Arc::new(store))
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL deve ser definida");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET deve ser definido");

        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await?;

        tracing::info!("✅ Conexão com o banco de dados estabelecida com sucesso!");

        let i18n_store = load_translations().expect("Falha ao carregar as traduções.");
        tracing::info!("✅ Traduções carregadas com sucesso!");

        // --- 1. Inicializa Repositórios (Base) ---
        let user_repo = UserRepository::new(db_pool.clone());
        let inventory_repo = InventoryRepository::new(db_pool.clone());
        let tenant_repo = TenantRepository::new(db_pool.clone());
        let crm_repo = CrmRepository::new(db_pool.clone());
        let operations_repo = crate::db::OperationsRepository::new(db_pool.clone());
        let finance_repo = crate::db::FinanceRepository::new(db_pool.clone());
        let dashboard_repo = crate::db::DashboardRepository::new(db_pool.clone());


        // [CORREÇÃO] RBAC Repo precisa ser criado ANTES de ser usado nos serviços
        let rbac_repo = RbacRepository::new(db_pool.clone());

        // --- 2. Inicializa Serviços (Dependentes) ---

        let auth_service = AuthService::new(
            user_repo.clone(),
            crm_repo.clone(),
            jwt_secret.clone(),
            db_pool.clone()
        );

        let finance_service = FinanceService::new(finance_repo);
        let inventory_service = InventoryService::new(inventory_repo.clone(), db_pool.clone());
        let document_service = DocumentService::new(operations_repo.clone());
        let operations_service = OperationsService::new(operations_repo, inventory_service.clone(), finance_service.clone());
        let dashboard_service = DashboardService::new(dashboard_repo);

        // [CORREÇÃO] TenantService agora recebe rbac_repo que já foi criado acima
        let tenant_service = TenantService::new(
            tenant_repo.clone(),
            rbac_repo.clone(), // Agora esta variável existe!
            db_pool.clone()
        );

        let crm_service = CrmService::new(crm_repo.clone());


        let rbac_service = RbacService::new(rbac_repo.clone(), db_pool.clone());

        // --- 3. Monta o Estado ---
        Ok(Self {
            db_pool,
            jwt_secret,
            auth_service,
            i18n_store,
            inventory_service,
            inventory_repo,
            tenant_repo,
            tenant_service, // Variável local tenant_service -> Campo tenant_service
            crm_repo,
            crm_service,
            rbac_repo,
            rbac_service,
            operations_service,
            finance_service,
            dashboard_service,
            document_service
        })
    }
}