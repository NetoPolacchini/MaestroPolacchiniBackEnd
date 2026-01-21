// src/handlers/dashboard.rs

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid; // Importante para o Swagger params

use crate::{
    common::{
        error::{ApiError, AppError},
        db_utils::get_rls_connection,
    },
    config::AppState,
    middleware::{
        auth::AuthenticatedUser,
        i18n::Locale,
        tenancy::TenantContext,
    },
    // Importamos os models para referenciar no Swagger
    models::dashboard::{DashboardSummary, SalesChartEntry, TopProductEntry},
};

// GET /api/dashboard/summary
#[utoipa::path(
    get,
    path = "/api/dashboard/summary",
    tag = "Dashboard",
    responses(
        (status = 200, description = "Resumo financeiro e operacional do dia", body = DashboardSummary),
        (status = 401, description = "Não autorizado"),
        (status = 403, description = "Sem acesso à loja")
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn get_summary(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let summary = app_state.dashboard_service
        .get_summary(&mut *rls_conn, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(summary)))
}

// GET /api/dashboard/sales-chart
#[utoipa::path(
    get,
    path = "/api/dashboard/sales-chart",
    tag = "Dashboard",
    responses(
        (status = 200, description = "Dados para gráfico de vendas (últimos 30 dias)", body = Vec<SalesChartEntry>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn get_sales_chart(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let chart = app_state.dashboard_service
        .get_sales_chart(&mut *rls_conn, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(chart)))
}

// GET /api/dashboard/top-products
#[utoipa::path(
    get,
    path = "/api/dashboard/top-products",
    tag = "Dashboard",
    responses(
        (status = 200, description = "Ranking dos produtos mais vendidos (Curva ABC)", body = Vec<TopProductEntry>)
    ),
    params(
        ("x-tenant-id" = Uuid, Header, description = "ID da Loja")
    ),
    security(
        ("api_jwt" = [])
    )
)]
pub async fn get_top_products(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
) -> Result<impl IntoResponse, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let products = app_state.dashboard_service
        .get_top_products(&mut *rls_conn, tenant.0)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    Ok((StatusCode::OK, Json(products)))
}