use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TenantSettings {
    pub tenant_id: Uuid,
    pub logo_url: Option<String>,
    pub primary_color: Option<String>,
    pub company_name: Option<String>,
    pub document_number: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub pix_key: Option<String>,
    pub pix_key_type: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub company_name: Option<String>,
    pub document_number: Option<String>,
    pub pix_key: Option<String>,
    pub address: Option<String>,
    // ... adicione outros campos se quiser permitir editar tudo
}