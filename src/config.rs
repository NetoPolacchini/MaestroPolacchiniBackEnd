// src/config.rs

// Importe dos serviços
use crate::services::{
    auth::AuthService,
    inventory_service::InventoryService,
    tenancy_service::TenantService,
    crm_service::CrmService,
};

// Importe dos repositórios
use crate::db::{
    UserRepository,
    InventoryRepository,
    TenantRepository,
    CrmRepository
};

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

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
    pub tenant_service: TenantService,
    pub crm_repo: CrmRepository,
    pub crm_service: CrmService,
}

// Uma função helper para carregar os arquivos
fn load_translations() -> anyhow::Result<I18nStore> {
    let mut store = HashMap::new();
    let paths = fs::read_dir("./locale")?;

    for path in paths {
        let path = path?.path();
        if path.extension().map_or(false, |e| e == "json") {
            let lang_code = path.file_stem().unwrap().to_str().unwrap().to_string();
            let file_content = fs::read_to_string(path)?;
            let translations: HashMap<String, String> = serde_json::from_str(&file_content)?;
            store.insert(lang_code, translations);
        }
    }
    Ok(Arc::new(store))
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

        let i18n_store = load_translations().expect("Falha ao carregar as traduções.");
        tracing::info!("✅ Traduções carregadas com sucesso!");

        // --- Monta o gráfico de dependências ---
        let user_repo = UserRepository::new(db_pool.clone());
        let inventory_repo = InventoryRepository::new(db_pool.clone());
        let tenant_repo = TenantRepository::new(db_pool.clone());
        let crm_repo = CrmRepository::new(db_pool.clone());

        // Serviços
        let auth_service = AuthService::new(user_repo, jwt_secret.clone());
        let inventory_service = InventoryService::new(inventory_repo.clone(), db_pool.clone());
        let tenant_service = TenantService::new(tenant_repo.clone(), db_pool.clone());
        let crm_service = CrmService::new(crm_repo.clone());

        // Retorna Ok com o estado montado
        Ok(Self {
            db_pool,
            jwt_secret,
            auth_service,
            i18n_store, // <-- NOSSA ADIÇÃO
            inventory_service,
            inventory_repo,
            tenant_repo,
            tenant_service,
            crm_repo,
            crm_service
        })
    }
}