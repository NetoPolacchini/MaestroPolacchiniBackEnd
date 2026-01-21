// src/docs.rs

use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use crate::handlers;
use crate::models;

#[derive(OpenApi)]
#[openapi(
    paths(
        // --- Settings ---
        handlers::settings::get_settings,
        handlers::settings::update_settings,

        // --- Auth ---
        handlers::auth::register,
        handlers::auth::login,

        // --- Users ---
        handlers::auth::get_me,
        handlers::auth::get_my_companies,

        // --- INVENTORY ---
        handlers::inventory::create_item,
        handlers::inventory::get_all_items,
        handlers::inventory::add_composition_item,
        handlers::inventory::get_item_composition,
        handlers::inventory::create_unit_of_measure,
        handlers::inventory::get_all_units,
        handlers::inventory::create_category,
        handlers::inventory::get_all_categories,
        handlers::inventory::add_stock,
        handlers::inventory::sell_item,

        // --- RBAC ---
        handlers::rbac::create_role,
        handlers::rbac::list_permissions,

        // --- Tenancy ---
        handlers::tenancy::create_tenant,
        handlers::tenancy::list_my_tenants,
        handlers::tenancy::create_stock_pool,
        handlers::tenancy::create_location,
        handlers::tenancy::list_locations,

        // --- Dashboard ---
        handlers::dashboard::get_summary,
        handlers::dashboard::get_sales_chart,
        handlers::dashboard::get_top_products,

        // --- OPERATIONS ---
        handlers::operations::create_pipeline,
        handlers::operations::add_stage,
        handlers::operations::create_order,
        handlers::operations::add_order_item,
        handlers::operations::transition_order,
    ),
    components(
        schemas(

            // --- DASHBOARD ---
            models::dashboard::DashboardSummary,
            models::dashboard::SalesChartEntry,
            models::dashboard::TopProductEntry,

            // --- Settings ---
            models::settings::TenantSettings,
            models::settings::UpdateSettingsRequest,

            // --- Operations ---
            models::operations::PipelineCategory,
            models::operations::Pipeline,
            models::operations::PipelineStage,
            models::operations::Order,
            models::operations::OrderItem,
            models::operations::OrderDetail,

            // --- Auth ---
            models::auth::DocumentType,
            models::auth::User,
            models::auth::UserCompany,
            models::auth::RegisterUserPayload,
            models::auth::LoginUserPayload,
            models::auth::AuthResponse,

            // --- Inventory ---
            models::inventory::ItemKind,
            models::inventory::CompositionType,
            models::inventory::Item,
            models::inventory::CompositionEntry,
            models::inventory::Category,
            models::inventory::InventoryLevel,
            models::inventory::StockMovementReason,
            models::inventory::StockMovement,
            models::inventory::UnitOfMeasure,
            models::inventory::InventoryBatch,

            // --- Payloads ---
            handlers::inventory::CreateItemPayload,
            handlers::inventory::AddCompositionPayload,
            handlers::inventory::CreateUnitPayload,
            handlers::inventory::CreateCategoryPayload,
            handlers::inventory::AddStockPayload,
            handlers::inventory::SellItemPayload,

            // --- RBAC ---
            models::rbac::Role,
            models::rbac::Permission,
            models::rbac::CreateRolePayload,
            models::rbac::RoleResponse,

            // --- CRM ---
            models::crm::FieldType,
            models::crm::EntityType,
            models::crm::FieldDefinition,
            models::crm::Customer,

            // --- TENANCY ---
            models::tenancy::Tenant,
            models::tenancy::TenantMember,
            models::tenancy::StockPool,
            models::tenancy::Location,
            handlers::tenancy::CreateTenantPayload,
            handlers::tenancy::CreateStockPoolPayload,
            handlers::tenancy::CreateLocationPayload,

            // --- FINANCE ---
            models::finance::TitleKind,
            models::finance::TitleStatus,
            models::finance::FinancialAccount,
            models::finance::FinancialCategory,
            models::finance::FinancialTitle,
            models::finance::FinancialMovement,

            // --- OPERATIONS PAYLOADS ---
            handlers::operations::CreatePipelinePayload,
            handlers::operations::AddStagePayload,
            handlers::operations::CreateOrderPayload,
            handlers::operations::AddOrderItemPayload,
            handlers::operations::TransitionOrderPayload,
        )
    ),
    tags(
        (name = "Settings", description = "Configurações da Loja"),
        (name = "Operations", description = "Gestão de Pedidos e Pipelines"),
        (name = "Auth", description = "Autenticação e Registro"),
        (name = "Inventory", description = "Gestão de Estoque e Produtos"),
        (name = "Users", description = "Dados do Usuário e Perfil"),
        (name = "RBAC", description = "Controle de Acesso (Cargos e Permissões)"),
        (name = "Tenancy", description = "Gestão de Lojas e Acesso"),
        (name = "Tenancy Setup", description = "Configuração Física da Loja (Estoques e Locais)"),
        (name = "Dashboard", description = "Indicadores e Gráficos Gerenciais")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_jwt",
            SecurityScheme::Http(
                Http::new(HttpAuthScheme::Bearer)
            ),
        );
    }
}