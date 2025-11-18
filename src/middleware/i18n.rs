// src/middleware/i18n.rs

// 1. REMOVA a linha de importação do async_trait
// use async_trait::async_trait;

use axum::extract::FromRequestParts;
use axum::http::{header, request::Parts};

// Nosso novo extrator de idioma
pub struct Locale(pub String);

// 2. REMOVA a macro #[async_trait] daqui
// #[async_trait]
impl<S> FromRequestParts<S> for Locale
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let default_lang = "en".to_string();

        let lang = parts
            .headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|header_value| header_value.to_str().ok())
            .and_then(|header_str| {
                accept_language::parse(header_str)
                    .get(0) // Pega o primeiro idioma (ex: "pt-BR")
                    .map(|tag_string| {
                        // "pt-BR" -> split vira ["pt", "BR"] -> next() pega "pt"
                        // "en"    -> split vira ["en"]       -> next() pega "en"
                        tag_string.split('-').next().unwrap_or(tag_string).to_string()
                    })
            })
            .unwrap_or(default_lang);

        Ok(Locale(lang))
    }
}