// src/models/settings.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantSettings {
    #[schema(ignore)] // Ocultamos o ID interno, pois o contexto (Header) já define a loja
    pub tenant_id: Uuid,

    #[schema(example = "https://minhaloja.com/assets/logo.png")]
    pub logo_url: Option<String>,

    #[schema(example = "#000000")]
    pub primary_color: Option<String>,

    #[schema(example = "Minha Loja Ltda")]
    pub company_name: Option<String>,

    #[schema(example = "12.345.678/0001-99")]
    pub document_number: Option<String>,

    #[schema(example = "Rua das Flores, 123 - Centro")]
    pub address: Option<String>,

    #[schema(example = "(11) 99999-8888")]
    pub phone: Option<String>,

    #[schema(example = "contato@minhaloja.com")]
    pub email: Option<String>,

    #[schema(example = "12.345.678/0001-99")]
    pub pix_key: Option<String>,

    //ToDo: mais um example para ver
    #[schema(example = "CNPJ", example = "Tipo da chave (CPF, CNPJ, EMAIL, PHONE, EVP)")]
    pub pix_key_type: Option<String>,

    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    #[schema(example = "Minha Nova Loja")]
    pub company_name: Option<String>,

    #[schema(example = "12.345.678/0001-99")]
    pub document_number: Option<String>,

    #[schema(example = "chave@pix.com.br")]
    pub pix_key: Option<String>,

    #[schema(example = "Av. Paulista, 1000")]
    pub address: Option<String>,
}