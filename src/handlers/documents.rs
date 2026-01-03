// src/handlers/documents.rs

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    common::{error::{ApiError, AppError}, db_utils::get_rls_connection},
    config::AppState,
    middleware::{auth::AuthenticatedUser, i18n::Locale, tenancy::TenantContext},
};

pub async fn generate_order_pdf(
    State(app_state): State<AppState>,
    locale: Locale,
    user: AuthenticatedUser,
    tenant: TenantContext,
    Path(order_id): Path<Uuid>,
) -> Result<Response, ApiError> {

    let mut rls_conn = get_rls_connection(&app_state, &tenant, &user)
        .await
        .map_err(|e| e.to_api_error(&locale, &app_state.i18n_store))?;

    let pdf_bytes = app_state.document_service
        .generate_order_pdf(&mut *rls_conn, tenant.0, order_id)
        .await
        .map_err(|app_err| app_err.to_api_error(&locale, &app_state.i18n_store))?;

    // Configura os Headers para o navegador baixar ou mostrar o PDF
    let headers = [
        (header::CONTENT_TYPE, "application/pdf"),
        (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"pedido_{}.pdf\"", order_id)),
    ];

    Ok((headers, pdf_bytes).into_response())
}