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
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, AppError> { // <--- Retorna AppError

    // 1. Adquire conexão
    // O operador '?' converte automaticamente sqlx::Error -> AppError::DatabaseError
    let mut conn = app_state.db_pool.acquire().await?;

    // 2. Define Tenant ID
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_ctx.0.to_string())
        .execute(&mut *conn)
        .await?; // Se falhar, vira AppError::DatabaseError automaticamente

    // 3. Define User ID
    sqlx::query("SELECT set_config('app.user_id', $1, true)")
        .bind(user.0.id.to_string())
        .execute(&mut *conn)
        .await?;

    Ok(conn)
}
