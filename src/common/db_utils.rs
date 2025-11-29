use crate::common::error::{ApiError, AppError};
use crate::config::AppState;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::i18n::Locale;
use crate::middleware::tenancy::TenantContext;

// ---
// Helper RLS: A "Chave" para o Banco de Dados
// ---
/// Adquire uma conexão da pool e define as variáveis RLS (a "chave").
// ---
// Helper RLS: A "Chave" para o Banco de Dados
// ---
pub(crate) async fn get_rls_connection(
    app_state: &AppState,
    tenant_ctx: &TenantContext,
    user: &AuthenticatedUser,
    locale: &Locale,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, ApiError> {

    let mut conn = app_state.db_pool.acquire().await.map_err(|e| {
        tracing::error!("Falha ao adquirir conexão da pool: {}", e);
        AppError::DatabaseError(e).to_api_error(locale, &app_state.i18n_store)
    })?;

    // MUDANÇA 1: Usamos set_config em vez de SET LOCAL
    // O 'true' no final significa "is_local" (apenas para esta transação)
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_ctx.0.to_string()) // UUID precisa virar String aqui
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Falha ao definir RLS app.tenant_id: {}", e);
            AppError::DatabaseError(e).to_api_error(locale, &app_state.i18n_store)
        })?;

    // MUDANÇA 2: O mesmo para o user_id
    sqlx::query("SELECT set_config('app.user_id', $1, true)")
        .bind(user.0.id.to_string())
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Falha ao definir RLS app.user_id: {}", e);
            AppError::DatabaseError(e).to_api_error(locale, &app_state.i18n_store)
        })?;

    Ok(conn)
}
